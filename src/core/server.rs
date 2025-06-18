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
use tokio::net::TcpListener;
use tokio::sync::Mutex;

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
use crate::core::app::constant::APP_NAME;
use parsed_request::ParsedRequest;
use response::error_response::internal_server_error_response;
use routing::dyn_route::dyn_route_content;
use types::BoxBody;

/// server
pub struct Server {
    pub addr: SocketAddr,
    pub app_state: AppState,
}

impl Server {
    pub async fn new(app_state: AppState) -> Self {
        let addr = app_state
            .config
            .listener_address()
            .to_socket_addrs()
            .expect("invalid listend address or port")
            .next()
            .expect("failed to resolve address");

        Server { addr, app_state }
    }

    /// app start
    pub async fn start(&self) {
        let listener = TcpListener::bind(self.addr)
            .await
            .expect("tcp listener failed to bind address");

        log::info!(
            "Greetings from {APP_NAME} !!\nListening on {} ...\n",
            style(format!("http://{}", self.addr)).cyan()
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
