use console::style;
use http_body_util::{BodyExt, Empty};
use hyper::{
    body,
    header::{HeaderValue, CONTENT_LENGTH},
    service::service_fn,
    HeaderMap, Response,
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
use crate::core::{
    app::constant::APP_NAME,
    util::tls::{load_certs, load_private_key},
};
use parsed_request::ParsedRequest;
use response::error_response::internal_server_error_response;
use routing::dyn_route::dyn_route_content;
use types::BoxBody;

/// server
pub struct Server {
    pub app_state: AppState,
    pub http_addr: Option<SocketAddr>,
    pub https_addr: Option<SocketAddr>,
}

impl Server {
    /// new
    pub async fn new(app_state: AppState) -> Self {
        let http_addr_str = app_state.config.listener_http_addr();
        let https_addr_str = app_state.config.listener_https_addr();

        let http_addr = if let Some(http_addr_str) = http_addr_str {
            Some(
                http_addr_str
                    .to_socket_addrs()
                    .expect("invalid listend address or port")
                    .next()
                    .expect("failed to resolve address"),
            )
        } else {
            None
        };

        let https_addr = if let Some(https_addr_str) = https_addr_str {
            Some(
                https_addr_str
                    .to_socket_addrs()
                    .expect("invalid listend address or port")
                    .next()
                    .expect("failed to resolve address"),
            )
        } else {
            None
        };

        Server {
            http_addr,
            https_addr,
            app_state,
        }
    }

    /// app starts
    pub async fn start(&self) {
        let http = self.http_start();
        let https = self.https_start();
        tokio::join!(http, https);
    }

    /// http listener server starts
    async fn http_start(&self) {
        let addr = if let Some(addr) = self.http_addr {
            addr
        } else {
            return;
        };

        let listener = TcpListener::bind(addr)
            .await
            .expect("tcp listener failed to bind address");

        log::info!(
            "Greetings from {APP_NAME} !!\nListening on {} ...\n",
            style(format!("http://{}", addr)).cyan()
        );

        let app_state = Arc::new(Mutex::new(self.app_state.clone()));
        loop {
            let (stream, _) = listener
                .accept()
                .await
                .expect("tcp listener failed to accept");
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

    /// https listener server starts
    async fn https_start(&self) {
        let addr = if let Some(addr) = self.https_addr {
            addr
        } else {
            return;
        };

        let tls = self.app_state.config.listener.clone().unwrap().tls.unwrap();

        let certs = load_certs(tls.cert.as_str());
        let key = load_private_key(tls.key.as_str());

        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| format!("bad certificate/key configuration: {:?}", e))
            .expect("failed ...");

        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        let acceptor = TlsAcceptor::from(Arc::new(config));

        let listener = TcpListener::bind(addr)
            .await
            .expect("tcp listener failed to bind address");

        log::info!(
            "Greetings from {APP_NAME} !!\nListening on {} ...\n",
            style(format!("https://{}", addr)).cyan()
        );

        let app_state = Arc::new(Mutex::new(self.app_state.clone()));

        loop {
            let (stream, _) = listener.accept().await.expect("tcp accept failed");
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

/// entry point of http requests handler service
pub async fn service(
    request: hyper::Request<body::Incoming>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let request_headers = request.headers().clone();

    let _ = match request.method() {
        &hyper::Method::OPTIONS => return handle_options(&request_headers),
        _ => (),
    };

    let parsed_request = match ParsedRequest::from(request).await {
        Ok(x) => x,
        Err(err) => return internal_server_error_response(err.as_str(), &request_headers),
    };

    let shared_app_state = { app_state.lock().await.clone() };

    // app handle driven by config
    let config = shared_app_state.config;

    parsed_request.capture_in_log(config.log.unwrap_or_default().verbose);

    match config.service.middleware_response(&parsed_request).await {
        Some(x) => return x,
        None => (),
    }

    match config.service.rule_set_response(&parsed_request).await {
        Some(x) => return x,
        None => (),
    }

    dyn_route_content(
        parsed_request.url_path.as_str(),
        config.service.fallback_respond_dir.as_str(),
        &request_headers,
    )
    .await
}

/// OPTIONS request handler
fn handle_options(
    request_headers: &HeaderMap,
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    let mut response = Response::new(Empty::new().boxed());

    // empty
    *response.status_mut() = hyper::StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

    // default headers
    response = default_response_headers(request_headers).into_iter().fold(
        response,
        |mut response, (header_key, header_value)| {
            if let Some(header_key) = header_key {
                response.headers_mut().insert(header_key, header_value);
                response
            } else {
                response
            }
        },
    );
    Ok(response)
}
