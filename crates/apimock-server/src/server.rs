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
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use crate::{
    dyn_route::dyn_route_content,
    error::{ServerError, ServerResult},
    middleware::LoadedMiddlewares,
    parsed_request::{capture_in_log, parsed_request_from},
    respond_response::respond_response,
    response::error_response::internal_server_error_response,
    response_handler::default_response_headers,
    tls::{build_server_config_reloadable, load_certs, load_private_key},
    types::BoxBody,
};

pub use crate::control::{ReloadHint, ServerControl, ServerHandle, ServerState};
use crate::trace::{Outcome, RequestSummary, TraceEmitter};

/// Shared state cloned into each per-request task.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub middlewares: LoadedMiddlewares,
    /// Live match-trace channel. Shared across all request handler tasks.
    pub tracer: TraceEmitter,
}

/// HTTP(S) server.
pub struct Server {
    pub app_state: AppState,
    pub http_addr: Option<SocketAddr>,
    pub https_addr: Option<SocketAddr>,
}

impl Server {
    /// Resolve listener addresses and build the server shell.
    ///
    /// Also compiles Rhai middlewares listed in
    /// `config.service.middlewares_file_paths`. Compilation happens here
    /// (not in the config crate) because the compiled artefact is a
    /// runtime object — see the server-level module docstring.
    pub async fn new(config: Config) -> ServerResult<Self> {
        let http_addr = resolve_listener(config.listener_http_addr().as_deref())?;
        let https_addr = resolve_listener(config.listener_https_addr().as_deref())?;

        // Resolve middleware paths against the config file's dir
        let relative_dir_path = config
            .current_dir_to_parent_dir_relative_path()
            .map_err(ServerError::Config)?;

        let middlewares = LoadedMiddlewares::compile(
            config
                .service
                .middlewares_file_paths
                .as_deref()
                .unwrap_or(&[]),
            relative_dir_path.as_str(),
        )?;
        if !middlewares.is_empty() {
            log::info!("middleware is activated: {} file(s)", middlewares.len());
        }

        Ok(Server {
            http_addr,
            https_addr,
            app_state: AppState {
                config,
                middlewares,
                tracer: TraceEmitter::new(),
            },
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

    /// Bind the HTTPS listener (including loading TLS material) without
    /// accepting connections yet. See [`Server::bind_http`] for why this
    /// is split from serving.
    ///
    /// Every failure this used to swallow via `log::error!` + early
    /// return - missing TLS config, unreadable cert/key, a TLS config
    /// that fails to build, or the bind itself - now surfaces as `Err`.
    pub async fn bind_https(&self) -> ServerResult<Option<(TcpListener, TlsAcceptor)>> {
        let Some(addr) = self.https_addr else {
            return Ok(None);
        };

        let tls = self
            .app_state
            .config
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
        let (tls_config, resolver) = build_server_config_reloadable(certs, key).map_err(|err| {
            ServerError::ListenerAddress {
                addr: addr.to_string(),
                reason: format!("failed to build TLS config: {}", err),
            }
        })?;
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        drop(resolver); // Server holds the resolver via the config; expose via ServerHandle if needed

        let listener =
            TcpListener::bind(addr)
                .await
                .map_err(|err| ServerError::ListenerAddress {
                    addr: addr.to_string(),
                    reason: err.to_string(),
                })?;

        Ok(Some((listener, acceptor)))
    }

    /// Accept connections forever on an already-bound HTTPS listener.
    pub async fn serve_https(&self, listener: TcpListener, acceptor: TlsAcceptor) {
        if let Ok(addr) = listener.local_addr() {
            log::info!(
                "Greetings from apimock-rs (API Mock) !!\nListening on {} ...\n",
                style(format!("https://{}", addr)).cyan()
            );
        }

        let app_state = Arc::new(Mutex::new(self.app_state.clone()));
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(err) => {
                    log::error!("HTTPS accept failed: {}", err);
                    continue;
                }
            };
            let acceptor = acceptor.clone();
            let app_state = app_state.clone();

            tokio::spawn(async move {
                let tls_stream = match acceptor.accept(stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("TLS handshake failed: {:?}", e);
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

/// Resolve an `ip:port` string into a single `SocketAddr`.
// clippy: ServerError is a public error type (RFC 030 §6 escalation
// trigger); boxing its large variant would change that type's shape.
// See ESCALATION-002 in the RFC 030 review-request package.
#[allow(clippy::result_large_err)]
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

    if request.method() == hyper::Method::OPTIONS {
        return handle_options(&request_headers);
    }

    let parsed_request = match parsed_request_from(request).await {
        Ok(x) => x,
        Err(err) => return internal_server_error_response(err.as_str(), &request_headers),
    };

    let shared_app_state = { app_state.lock().await.clone() };

    let config = shared_app_state.config;
    let middlewares = shared_app_state.middlewares;
    let tracer = shared_app_state.tracer;

    let received_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let start = std::time::Instant::now();

    capture_in_log(
        &parsed_request,
        config.log.clone().unwrap_or_default().verbose,
    );

    if let Some(response) = middleware_response(&middlewares, &parsed_request).await {
        return response;
    }

    if let Some(response) = rule_set_response(&config, &parsed_request).await {
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
    )
    .await
}

/// Dispatch the request through every loaded middleware in order.
async fn middleware_response(
    middlewares: &LoadedMiddlewares,
    parsed_request: &ParsedRequest,
) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
    for handler in middlewares.iter() {
        match handler
            .handle(
                parsed_request.url_path.as_str(),
                parsed_request.body_json.as_ref(),
                &parsed_request.component_parts.headers,
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
async fn rule_set_response(
    config: &Config,
    parsed_request: &ParsedRequest,
) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
    for (rule_set_idx, rule_set) in config.service.rule_sets.iter().enumerate() {
        if let Some(respond) = rule_set.find_matched(
            parsed_request,
            config.service.strategy.as_ref(),
            rule_set_idx,
        ) {
            let dir_prefix = rule_set.dir_prefix();
            let rule_set_default_delay_ms = rule_set
                .default
                .as_ref()
                .and_then(|default| default.delay_response_milliseconds);
            return Some(
                respond_response(
                    &respond,
                    dir_prefix.as_str(),
                    parsed_request,
                    rule_set_default_delay_ms,
                )
                .await,
            );
        }
    }
    None
}

/// OPTIONS request handler (CORS preflight).
fn handle_options(
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let mut response = Response::new(Empty::new().boxed());
    *response.status_mut() = hyper::StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

    for (header_key, header_value) in default_response_headers(request_headers).into_iter() {
        if let Some(header_key) = header_key {
            response.headers_mut().insert(header_key, header_value);
        }
    }

    Ok(response)
}
