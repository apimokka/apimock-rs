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
use response_handler::default_response_headers;
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

pub mod constant;
pub mod middleware;
pub mod parsed_request;
pub mod response;
mod response_handler;
pub mod routing;
mod routing_analysis;
pub mod types;

use crate::core::app::app_state::AppState;
use crate::core::error::{AppError, AppResult};
use crate::core::{
    app::constant::APP_NAME,
    util::tls::{load_certs, load_private_key},
};
use parsed_request::ParsedRequest;
use response::error_response::internal_server_error_response;
use routing::dyn_route::dyn_route_content;
use types::BoxBody;

/// HTTP(S) server.
///
/// Holds the resolved socket addresses for HTTP and HTTPS (each optional:
/// TLS-only, plaintext-only, and dual listeners are all supported) and
/// the shared `AppState` that per-request tasks clone cheaply.
pub struct Server {
    pub app_state: AppState,
    pub http_addr: Option<SocketAddr>,
    pub https_addr: Option<SocketAddr>,
}

impl Server {
    /// Resolve listener addresses and build the server shell.
    ///
    /// Address resolution happens here (once at startup) rather than at
    /// each `start()` call so that any DNS / format error is reported
    /// before we print the "listening on..." message.
    pub async fn new(app_state: AppState) -> AppResult<Self> {
        let http_addr = resolve_listener(app_state.config.listener_http_addr().as_deref())?;
        let https_addr = resolve_listener(app_state.config.listener_https_addr().as_deref())?;

        Ok(Server {
            http_addr,
            https_addr,
            app_state,
        })
    }

    /// Start both the HTTP and HTTPS listeners, whichever are configured.
    ///
    /// # Why `tokio::join!` and not `tokio::select!`
    ///
    /// Each listener runs a never-ending `accept` loop. We want both to
    /// stay up for the life of the process; `join!` waits for both, which
    /// in practice means "run forever unless one panics". `select!` would
    /// abort the other listener the moment one returns, which is the
    /// opposite of what we want.
    pub async fn start(&self) {
        let http = self.http_start();
        let https = self.https_start();
        tokio::join!(http, https);
    }

    /// HTTP listener accept loop.
    async fn http_start(&self) {
        let Some(addr) = self.http_addr else {
            return;
        };

        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(err) => {
                log::error!("failed to bind HTTP listener at {}: {}", addr, err);
                return;
            }
        };

        log::info!(
            "Greetings from {APP_NAME} !!\nListening on {} ...\n",
            style(format!("http://{}", addr)).cyan()
        );

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

    /// HTTPS listener accept loop.
    async fn https_start(&self) {
        let Some(addr) = self.https_addr else {
            return;
        };

        // HTTPS implies TLS is configured — `self.https_addr` is only
        // `Some` when `listener_https_addr()` (which requires
        // `listener.tls`) returned `Some`.
        let tls = match self
            .app_state
            .config
            .listener
            .as_ref()
            .and_then(|l| l.tls.as_ref())
        {
            Some(t) => t.clone(),
            None => {
                log::error!("internal: HTTPS listener scheduled without TLS config");
                return;
            }
        };

        let certs = match load_certs(tls.cert.as_str()) {
            Ok(c) => c,
            Err(err) => {
                log::error!("{}", err);
                return;
            }
        };
        let key = match load_private_key(tls.key.as_str()) {
            Ok(k) => k,
            Err(err) => {
                log::error!("{}", err);
                return;
            }
        };

        let mut config = match ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
        {
            Ok(c) => c,
            Err(err) => {
                log::error!("failed to build rustls ServerConfig: {}", err);
                return;
            }
        };

        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        let acceptor = TlsAcceptor::from(Arc::new(config));

        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(err) => {
                log::error!("failed to bind HTTPS listener at {}: {}", addr, err);
                return;
            }
        };

        log::info!(
            "Greetings from {APP_NAME} !!\nListening on {} ...\n",
            style(format!("https://{}", addr)).cyan()
        );

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
}

/// Resolve an `ip:port` string into a single `SocketAddr`.
///
/// Consolidated out of `Server::new` so both HTTP and HTTPS paths share
/// the same error-to-`AppError` translation.
fn resolve_listener(addr_str: Option<&str>) -> AppResult<Option<SocketAddr>> {
    let Some(addr_str) = addr_str else {
        return Ok(None);
    };

    let mut addrs = addr_str
        .to_socket_addrs()
        .map_err(|e| AppError::ListenerAddress {
            addr: addr_str.to_owned(),
            reason: e.to_string(),
        })?;

    addrs
        .next()
        .map(Some)
        .ok_or_else(|| AppError::ListenerAddress {
            addr: addr_str.to_owned(),
            reason: "address resolved to no socket addresses".to_owned(),
        })
}

/// Entry point for each HTTP request.
///
/// # Why the routing order is: OPTIONS, middleware, rule sets, dyn route
///
/// - `OPTIONS` gets its own short-circuit because browsers' CORS preflight
///   must always return 204 regardless of configured rules.
/// - Middleware runs next so it can handle arbitrary logic (auth, logging,
///   synthetic responses) ahead of declarative rules.
/// - Rule sets come before the directory fallback because they are more
///   specific — if an operator wrote a rule, they want it to win over a
///   generic file on disk with the same URL shape.
/// - `dyn_route_content` is the fallback "just serve the JSON in the
///   folder" behaviour, preserving the zero-config experience.
pub async fn service(
    request: hyper::Request<body::Incoming>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let request_headers = request.headers().clone();

    if request.method() == hyper::Method::OPTIONS {
        return handle_options(&request_headers);
    }

    let parsed_request = match ParsedRequest::from(request).await {
        Ok(x) => x,
        Err(err) => return internal_server_error_response(err.as_str(), &request_headers),
    };

    let shared_app_state = { app_state.lock().await.clone() };

    // app handle driven by config
    let config = shared_app_state.config;

    parsed_request.capture_in_log(config.log.unwrap_or_default().verbose);

    if let Some(response) = config.service.middleware_response(&parsed_request).await {
        return response;
    }

    if let Some(response) = config.service.rule_set_response(&parsed_request).await {
        return response;
    }

    dyn_route_content(
        parsed_request.url_path.as_str(),
        config.service.fallback_respond_dir.as_str(),
        &request_headers,
    )
    .await
}

/// OPTIONS request handler (CORS preflight).
///
/// Always returns 204 No Content with the default response headers; the
/// actual CORS policy lives in `default_response_headers`, which echoes
/// the Origin back for authenticated-style requests and uses `*` otherwise.
fn handle_options(
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let mut response = Response::new(Empty::new().boxed());

    *response.status_mut() = hyper::StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

    // Apply the default header set. Missing keys are quietly dropped
    // because `default_response_headers` already logged each failure at
    // the point it tried to build the header.
    for (header_key, header_value) in default_response_headers(request_headers).into_iter() {
        if let Some(header_key) = header_key {
            response.headers_mut().insert(header_key, header_value);
        }
    }

    Ok(response)
}
