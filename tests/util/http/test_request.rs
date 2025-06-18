use console::style;
use http_body_util::{BodyExt, Empty, Full};
use hyper::{
    body::{Bytes, Incoming},
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Method, Request, Response, Uri,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

pub struct TestRequest {
    pub port: u16,
    pub url_path: String,
    pub http_method: Option<Method>,
    pub headers: Option<HeaderMap<HeaderValue>>,
    pub body: Option<String>,
}

impl TestRequest {
    /// default
    pub fn default(url_path: &str, port: u16) -> Self {
        Self {
            port,
            url_path: url_path.to_owned(),
            http_method: None,
            headers: None,
            body: None,
        }
    }

    /// default with http method
    pub fn with_http_method(mut self, http_method: &Method) -> Self {
        self.http_method = Some(http_method.to_owned());
        self
    }

    /// default with headers condition
    pub fn with_headers(mut self, headers: &HeaderMap<HeaderValue>) -> Self {
        self.headers = Some(headers.to_owned());
        self
    }

    /// default with body condition
    pub fn with_body(mut self, body: &str) -> Self {
        self.body = Some(body.to_owned());
        self
    }

    /// default with body condition as json
    pub fn with_body_as_json(mut self, body: &str) -> Self {
        let headers: HeaderMap = [(CONTENT_TYPE, "application/json")]
            .into_iter()
            .map(|(k, v)| (k, HeaderValue::from_static(v)))
            .collect();
        self.headers = Some(headers);

        self.body = Some(body.to_owned());

        self
    }

    /// send request to get http response from mock server
    pub async fn send(&self) -> Response<Incoming> {
        let url: Uri = Uri::builder()
            .scheme("http")
            .authority(format!("127.0.0.1:{}", self.port.to_string()))
            .path_and_query(self.url_path.as_str())
            .build()
            .unwrap();

        let host = url.host().expect("url has no host");
        let port = url.port_u16().expect("some problem around port");
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(addr)
            .await
            .expect(&format!("tcp connect failed with {}:{}", host, port));
        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                log::error!("{} to connect to server: {:?}", style("failed").red(), err);
            }
        });

        let authority = url.authority().unwrap().clone();

        let path = url.path();
        let body = if self.body.is_none() {
            Empty::new().boxed()
        } else {
            Full::new(Bytes::from(self.body.as_ref().unwrap().to_owned())).boxed()
        };
        let mut builder = Request::builder()
            .uri(path)
            .header(hyper::header::HOST, authority.as_str());
        if let Some(http_method) = self.http_method.as_ref() {
            builder = builder.method(http_method);
        }
        if let Some(headers) = self.headers.as_ref() {
            for (header_key, header_value) in headers.iter() {
                builder = builder.header(header_key, header_value);
            }
        }
        let req = builder.body(body).expect("failed to create http request");

        sender.send_request(req).await.unwrap()
    }
}
