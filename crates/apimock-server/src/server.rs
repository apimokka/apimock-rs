//! HTTP(S) server runtime.
//!
//! # 5.0 layout
//!
//! The [`Server`] struct holds the listener addresses and the shared
//! application state. [`AppState`] in turn holds a `Config` (editable
//! declarative data from `apimock-config`) alongside [`LoadedMiddlewares`]
//! (compiled Rhai — runtime only, server-owned).
//!
//! Dispatch methods (`middleware_response`, `rule_set_response`) used
//! to hang off `ServiceConfig` but were moved here in 5.0 because they
//! build `hyper::Response` values, which a config crate must not do.
//! They are now free functions in this module that take borrowed config
//! + loaded state and produce an `hyper::Response`.

use apimock_config::Config;
use apimock_config::config::constant::{
    SERVICE_DEFAULT_MAX_REQUEST_BODY_BYTES, SERVICE_DEFAULT_MIDDLEWARE_MAX_OPERATIONS,
    TLS_DEFAULT_HANDSHAKE_TIMEOUT_SECONDS, TLS_DEFAULT_MAX_CONNECTIONS,
};
use apimock_routing::ParsedRequest;
use console::style;
use http_body_util::{BodyExt, Empty};
use hyper::{
    HeaderMap, Response, body,
    header::{CONTENT_LENGTH, HeaderValue},
    service::service_fn,
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Semaphore};
use tokio_rustls::TlsAcceptor;

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    dyn_route::dyn_route_content,
    error::{ServerError, ServerResult},
    middleware::LoadedMiddlewares,
    parsed_request::{ParsedRequestError, capture_in_log_with_trace_config, parsed_request_from},
    respond_response::respond_response,
    response::{
        confine::canonical_dir,
        error_response::{internal_server_error_response, payload_too_large_response},
    },
    response_handler::default_response_headers,
    tls::{build_server_config_reloadable, load_certs, load_private_key},
    types::BoxBody,
};

pub use crate::control::{ReloadHint, ServerControl, ServerHandle, ServerState};
use crate::trace::{Outcome, RequestSummary, TraceEmitter};

/// Shared state cloned into each per-request task.
#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    pub config: Config,
    pub middlewares: LoadedMiddlewares,
    /// Live match-trace channel. Shared across all request handler tasks.
    pub tracer: TraceEmitter,
    /// `config.service.fallback_respond_dir`, canonicalised once here
    /// rather than per request — only the per-request candidate needs
    /// fresh canonicalisation. `None` if the directory doesn't exist.
    canonical_fallback_respond_dir: Option<PathBuf>,
    /// Parallel to `config.service.rule_sets`, same reasoning.
    canonical_rule_set_respond_dirs: Vec<Option<PathBuf>>,
}

impl AppState {
    pub fn new(config: Config, middlewares: LoadedMiddlewares, tracer: TraceEmitter) -> Self {
        let canonical_fallback_respond_dir =
            canonical_dir(config.service.fallback_respond_dir.as_str());
        let canonical_rule_set_respond_dirs = config
            .service
            .rule_sets
            .iter()
            .map(|rule_set| canonical_dir(rule_set.dir_prefix().as_str()))
            .collect();
        Self {
            config,
            middlewares,
            tracer,
            canonical_fallback_respond_dir,
            canonical_rule_set_respond_dirs,
        }
    }
}

/// HTTP(S) server.
#[non_exhaustive]
pub struct Server {
    pub app_state: AppState,
    pub http_addr: Option<SocketAddr>,
    pub https_addr: Option<SocketAddr>,
    /// TLS material, loaded and built once at construction — see
    /// `Server::new`'s doc comment for why this isn't built lazily
    /// inside `bind_https` any more (RFC 074 S-08). `None` iff
    /// `https_addr` is `None`.
    https_tls: Option<HttpsTls>,
}

/// Everything `bind_https`/`serve_https` need once TLS material has
/// been loaded and validated (RFC 074 S-08, S-07).
#[derive(Clone)]
struct HttpsTls {
    acceptor: TlsAcceptor,
    handshake_timeout: std::time::Duration,
    max_connections: usize,
}

impl Server {
    /// Resolve listener addresses and build the server shell.
    ///
    /// Also compiles Rhai middlewares listed in
    /// `config.service.middlewares_file_paths`. Compilation happens here
    /// (not in the config crate) because the compiled artefact is a
    /// runtime object — see the server-level module docstring.
    ///
    /// # TLS material is loaded here, eagerly (RFC 074 S-08)
    ///
    /// If `[listener.tls]` is present, its cert/key are loaded and the
    /// TLS config built as part of this call — not lazily, the first
    /// time `bind_https` runs. A malformed PEM (the file exists —
    /// `apimock_config::Config::new` already rejected a missing one —
    /// but doesn't parse) now fails `Server::new` itself, which
    /// `App::new` propagates with `?`, which `main` propagates with
    /// `?`: the process exits before `main` ever reaches
    /// `app.server.start().await`, so *no* listener binds, HTTP
    /// included. Building this lazily inside `bind_https` — the
    /// previous behaviour — let `https_start` log the error and return
    /// while any separately-configured HTTP listener kept serving,
    /// which is exactly the silent HTTP-only degradation this RFC
    /// exists to close.
    pub async fn new(config: Config) -> ServerResult<Self> {
        let http_addr = resolve_listener(config.listener_http_addr().as_deref())?;
        let https_addr = resolve_listener(config.listener_https_addr().as_deref())?;

        let https_tls = match https_addr {
            Some(addr) => Some(build_https_tls(&config, addr)?),
            None => None,
        };

        // Resolve middleware paths against the config file's dir
        let relative_dir_path = config
            .current_dir_to_parent_dir_relative_path()
            .map_err(ServerError::Config)?;

        let middleware_max_operations = config
            .service
            .middleware_max_operations
            .unwrap_or(SERVICE_DEFAULT_MIDDLEWARE_MAX_OPERATIONS);
        let middlewares = LoadedMiddlewares::compile(
            config
                .service
                .middlewares_file_paths
                .as_deref()
                .unwrap_or(&[]),
            relative_dir_path.as_str(),
            middleware_max_operations,
        )?;
        if !middlewares.is_empty() {
            log::info!("middleware is activated: {} file(s)", middlewares.len());
        }

        Ok(Server {
            http_addr,
            https_addr,
            https_tls,
            app_state: AppState::new(config, middlewares, TraceEmitter::new()),
        })
    }

    /// Start both listeners (whichever are configured) and block.
    pub async fn start(&self) {
        let http = self.http_start();
        let https = self.https_start();
        tokio::join!(http, https);
    }

    /// Bind the HTTP listener without accepting connections yet.
    ///
    /// Returns `Ok(None)` if no HTTP listener is configured, `Ok(Some(_))`
    /// on a successful bind, or `Err` if the bind itself failed — this is
    /// the piece `http_start` used to swallow via `log::error!` + early
    /// return, with no way for a caller to observe it.
    ///
    /// Splitting bind from serve exists for callers (namely the
    /// integration-test harness) that need the two to be separate steps:
    /// bind, read back the real port via `local_addr()` (useful when
    /// `[listener].port` is `0` and the OS assigns one), *then* hand the
    /// same listener to [`Server::serve_http`]. Because it's the same
    /// listener throughout, there is no window between "port known" and
    /// "port held" for another process to take it.
    pub async fn bind_http(&self) -> ServerResult<Option<TcpListener>> {
        let Some(addr) = self.http_addr else {
            return Ok(None);
        };

        let listener =
            TcpListener::bind(addr)
                .await
                .map_err(|err| ServerError::ListenerAddress {
                    addr: addr.to_string(),
                    reason: err.to_string(),
                })?;

        Ok(Some(listener))
    }

    /// Accept connections forever on an already-bound HTTP listener.
    pub async fn serve_http(&self, listener: TcpListener) {
        if let Ok(addr) = listener.local_addr() {
            log::info!(
                "Greetings from apimock-rs (API Mock) !!\nListening on {} ...\n",
                style(format!("http://{}", addr)).cyan()
            );
        }

        let app_state = Arc::new(Mutex::new(self.app_state.clone()));
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(err) => {
                    log::error!("HTTP accept failed: {}", err);
                    continue;
                }
            };
            let io = TokioIo::new(stream);

            let app_state = app_state.clone();
            tokio::task::spawn(async move {
                if let Err(err) = Builder::new(TokioExecutor::new())
                    .serve_connection(
                        io,
                        service_fn(move |request: hyper::Request<body::Incoming>| {
                            service(request, app_state.clone())
                        }),
                    )
                    .await
                {
                    log::error!("{} to build connection: {:?}", style("failed").red(), err);
                }
            });
        }
    }

    async fn http_start(&self) {
        match self.bind_http().await {
            Ok(Some(listener)) => self.serve_http(listener).await,
            Ok(None) => (),
            Err(err) => log::error!("{}", err),
        }
    }

    /// Bind the HTTPS listener without accepting connections yet. See
    /// [`Server::bind_http`] for why this is split from serving.
    ///
    /// TLS material is already loaded and validated by this point —
    /// see [`Server::new`]'s doc comment — so the only failure left
    /// here is the socket bind itself (e.g. the port is in use).
    pub async fn bind_https(&self) -> ServerResult<Option<(TcpListener, TlsAcceptor)>> {
        let (Some(addr), Some(https_tls)) = (self.https_addr, self.https_tls.as_ref()) else {
            return Ok(None);
        };

        let listener =
            TcpListener::bind(addr)
                .await
                .map_err(|err| ServerError::ListenerAddress {
                    addr: addr.to_string(),
                    reason: err.to_string(),
                })?;

        Ok(Some((listener, https_tls.acceptor.clone())))
    }

    /// Accept connections forever on an already-bound HTTPS listener.
    ///
    /// # RFC 074 S-07: handshake timeout and connection cap
    ///
    /// A connection that opens and never completes its TLS handshake is
    /// dropped after `handshake_timeout` — previously nothing bounded
    /// this, so such a connection held its task (and the OS socket)
    /// forever. Concurrency is bounded by a `Semaphore` sized to
    /// `max_connections`: a permit is acquired *before* spawning the
    /// per-connection task, so once `max_connections` connections are
    /// in flight, `listener.accept()` keeps accepting into the kernel
    /// backlog but this loop stops handing new connections to the TLS
    /// handshake until a permit frees — the server recovers as soon as
    /// existing connections close, rather than needing a restart.
    pub async fn serve_https(&self, listener: TcpListener, acceptor: TlsAcceptor) {
        if let Ok(addr) = listener.local_addr() {
            log::info!(
                "Greetings from apimock-rs (API Mock) !!\nListening on {} ...\n",
                style(format!("https://{}", addr)).cyan()
            );
        }

        let (handshake_timeout, max_connections) = self
            .https_tls
            .as_ref()
            .map(|t| (t.handshake_timeout, t.max_connections))
            .unwrap_or((
                std::time::Duration::from_secs(TLS_DEFAULT_HANDSHAKE_TIMEOUT_SECONDS),
                TLS_DEFAULT_MAX_CONNECTIONS,
            ));
        let connection_slots = Arc::new(Semaphore::new(max_connections));

        let app_state = Arc::new(Mutex::new(self.app_state.clone()));
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(err) => {
                    log::error!("HTTPS accept failed: {}", err);
                    continue;
                }
            };
            let Ok(permit) = Arc::clone(&connection_slots).acquire_owned().await else {
                // Semaphore only closes if `close()` is called, which
                // nothing here does — unreachable in practice, but the
                // accept loop should not panic on it.
                continue;
            };
            let acceptor = acceptor.clone();
            let app_state = app_state.clone();

            tokio::spawn(async move {
                let _permit = permit; // held for the connection's lifetime
                let tls_stream =
                    match tokio::time::timeout(handshake_timeout, acceptor.accept(stream)).await {
                        Ok(Ok(s)) => s,
                        Ok(Err(e)) => {
                            log::error!("TLS handshake failed: {:?}", e);
                            return;
                        }
                        Err(_elapsed) => {
                            log::error!(
                                "TLS handshake timed out after {:?}; dropping connection",
                                handshake_timeout
                            );
                            return;
                        }
                    };
                let io = TokioIo::new(tls_stream);
                let app_state = app_state.clone();
                tokio::task::spawn(async move {
                    if let Err(err) = Builder::new(TokioExecutor::new())
                        .serve_connection(
                            io,
                            service_fn(move |request: hyper::Request<body::Incoming>| {
                                service(request, app_state.clone())
                            }),
                        )
                        .await
                    {
                        log::error!("{} to build connection: {:?}", style("failed").red(), err);
                    }
                });
            });
        }
    }

    async fn https_start(&self) {
        match self.bind_https().await {
            Ok(Some((listener, acceptor))) => self.serve_https(listener, acceptor).await,
            Ok(None) => (),
            Err(err) => log::error!("{}", err),
        }
    }
}

/// Load TLS material and build the acceptor + S-07 settings for the
/// HTTPS listener. Called once, eagerly, from [`Server::new`] — see
/// its doc comment for why this doesn't happen lazily in `bind_https`
/// any more.
fn build_https_tls(config: &Config, addr: SocketAddr) -> ServerResult<HttpsTls> {
    let tls = config
        .listener
        .as_ref()
        .and_then(|l| l.tls.as_ref())
        .cloned()
        .ok_or_else(|| ServerError::ListenerAddress {
            addr: addr.to_string(),
            reason: "internal: HTTPS listener scheduled without TLS config".to_owned(),
        })?;

    let certs = load_certs(tls.cert.as_str())?;
    let key = load_private_key(tls.key.as_str())?;

    // RFC 020: use a reloadable resolver so TlsCertFile / TlsKeyFile
    // changes are SoftReload (no listener rebind needed).
    let (tls_config, resolver) =
        build_server_config_reloadable(certs, key).map_err(|err| ServerError::ListenerAddress {
            addr: addr.to_string(),
            reason: format!("failed to build TLS config: {}", err),
        })?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    drop(resolver); // Server holds the resolver via the config; expose via ServerHandle if needed

    let handshake_timeout = std::time::Duration::from_secs(
        tls.handshake_timeout_seconds
            .unwrap_or(TLS_DEFAULT_HANDSHAKE_TIMEOUT_SECONDS),
    );
    let max_connections = tls.max_connections.unwrap_or(TLS_DEFAULT_MAX_CONNECTIONS);

    Ok(HttpsTls {
        acceptor,
        handshake_timeout,
        max_connections,
    })
}

/// Resolve an `ip:port` string into a single `SocketAddr`.
fn resolve_listener(addr_str: Option<&str>) -> ServerResult<Option<SocketAddr>> {
    let Some(addr_str) = addr_str else {
        return Ok(None);
    };

    let mut addrs = addr_str
        .to_socket_addrs()
        .map_err(|e| ServerError::ListenerAddress {
            addr: addr_str.to_owned(),
            reason: e.to_string(),
        })?;

    addrs
        .next()
        .map(Some)
        .ok_or_else(|| ServerError::ListenerAddress {
            addr: addr_str.to_owned(),
            reason: "address resolved to no socket addresses".to_owned(),
        })
}

/// Entry point for each HTTP request.
///
/// # Routing order
///
/// OPTIONS → middleware → rule sets → dyn_route (fallback). See
/// `respond_response` and `dyn_route_content` for each step's details.
pub async fn service(
    request: hyper::Request<body::Incoming>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let request_headers = request.headers().clone();

    // Locked here, before the OPTIONS early return (moved up twice now:
    // first for RFC 068 S-02's body limit, then here for RFC 067 —
    // `cors_allow_credentials_origins` is config too, and an OPTIONS
    // preflight needs the real CORS decision exactly like every other
    // response does, not the empty-list default a caller with no config
    // in scope gets).
    let shared_app_state = { app_state.lock().await.clone() };

    let config = shared_app_state.config;
    let middlewares = shared_app_state.middlewares;
    let tracer = shared_app_state.tracer;
    let canonical_fallback_respond_dir = shared_app_state.canonical_fallback_respond_dir;
    let canonical_rule_set_respond_dirs = shared_app_state.canonical_rule_set_respond_dirs;

    let cors_allow_credentials_origins = config
        .service
        .cors_allow_credentials_origins
        .clone()
        .unwrap_or_default();

    if request.method() == hyper::Method::OPTIONS {
        return handle_options(&request_headers, &cors_allow_credentials_origins);
    }

    let max_request_body_bytes = config
        .service
        .max_request_body_bytes
        .unwrap_or(SERVICE_DEFAULT_MAX_REQUEST_BODY_BYTES);
    let max_request_body_bytes = usize::try_from(max_request_body_bytes).unwrap_or(usize::MAX);

    let parsed_request = match parsed_request_from(request, max_request_body_bytes).await {
        Ok(x) => x,
        Err(ParsedRequestError::BodyTooLarge) => {
            return payload_too_large_response(
                &format!(
                    "request body exceeds the configured limit ({} bytes)",
                    max_request_body_bytes
                ),
                &request_headers,
                &cors_allow_credentials_origins,
            );
        }
        Err(ParsedRequestError::Other(err)) => {
            return internal_server_error_response(
                err.as_str(),
                &request_headers,
                &cors_allow_credentials_origins,
            );
        }
    };

    let received_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let start = std::time::Instant::now();

    capture_in_log_with_trace_config(
        &parsed_request,
        config.log.clone().unwrap_or_default().verbose,
        &tracer.config,
    );

    if let Some(response) = middleware_response(
        &middlewares,
        &parsed_request,
        &cors_allow_credentials_origins,
    )
    .await
    {
        return response;
    }

    if let Some(response) = rule_set_response(
        &config,
        &parsed_request,
        canonical_rule_set_respond_dirs.as_slice(),
        &cors_allow_credentials_origins,
    )
    .await
    {
        // Emit trace event on match.
        if tracer.has_subscribers() {
            let headers = parsed_request
                .component_parts
                .headers
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_owned())))
                .collect();
            let mut summary = RequestSummary::new(
                parsed_request.component_parts.method.to_string(),
                parsed_request.url_path.clone(),
                headers,
                parsed_request.body_len,
                &tracer.config,
            );
            tracer.enrich_with_body(&mut summary, parsed_request.body_json.as_ref());
            tracer.emit(
                received_at_ms,
                start.elapsed().as_millis() as u32,
                summary,
                Outcome::Miss { status: 0 }, // coarse-grained; fine-grained tracing is a future pass
            );
        }
        return response;
    }

    dyn_route_content(
        parsed_request.url_path.as_str(),
        config.service.fallback_respond_dir.as_str(),
        &request_headers,
        canonical_fallback_respond_dir.as_deref(),
        &cors_allow_credentials_origins,
    )
    .await
}

/// Dispatch the request through every loaded middleware in order.
async fn middleware_response(
    middlewares: &LoadedMiddlewares,
    parsed_request: &ParsedRequest,
    cors_allow_credentials_origins: &[String],
) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
    for handler in middlewares.iter() {
        match handler
            .handle(
                parsed_request.url_path.as_str(),
                parsed_request.body_json.as_ref(),
                &parsed_request.component_parts.headers,
                cors_allow_credentials_origins,
            )
            .await
        {
            Some(x) => return Some(x),
            None => continue,
        }
    }
    None
}

/// Dispatch through the configured rule sets.
///
/// `canonical_rule_set_respond_dirs` is parallel to
/// `config.service.rule_sets` — see `AppState::new`.
async fn rule_set_response(
    config: &Config,
    parsed_request: &ParsedRequest,
    canonical_rule_set_respond_dirs: &[Option<PathBuf>],
    cors_allow_credentials_origins: &[String],
) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
    for (rule_set_idx, rule_set) in config.service.rule_sets.iter().enumerate() {
        if let Some((_rule_idx, respond)) = rule_set.find_matched(
            parsed_request,
            config.service.strategy.as_ref(),
            rule_set_idx,
        ) {
            let dir_prefix = rule_set.dir_prefix();
            let rule_set_default_delay_ms = rule_set
                .default
                .as_ref()
                .and_then(|default| default.delay_response_milliseconds);
            let confine_to = canonical_rule_set_respond_dirs
                .get(rule_set_idx)
                .and_then(|dir| dir.as_deref());
            return Some(
                respond_response(
                    &respond,
                    dir_prefix.as_str(),
                    parsed_request,
                    rule_set_default_delay_ms,
                    confine_to,
                    cors_allow_credentials_origins,
                )
                .await,
            );
        }
    }
    None
}

/// OPTIONS request handler (CORS preflight). `pub` so `apimock get`
/// (RFC 055) can answer for an `OPTIONS` request through the exact same
/// function `service` calls, rather than reimplementing it.
pub fn handle_options(
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let mut response = Response::new(Empty::new().boxed());
    *response.status_mut() = hyper::StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

    for (header_key, header_value) in
        default_response_headers(request_headers, cors_allow_credentials_origins).into_iter()
    {
        if let Some(header_key) = header_key {
            response.headers_mut().insert(header_key, header_value);
        }
    }

    Ok(response)
}
