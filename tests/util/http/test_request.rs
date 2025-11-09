use hyper::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Method,
};
use reqwest::{Client, Response};

const LOCALHOST: &str = "127.0.0.1";

pub struct TestRequest {
    pub host: String,
    pub port: u16,
    pub url_path: String,
    pub http_method: Option<Method>,
    pub headers: Option<HeaderMap<HeaderValue>>,
    pub body: Option<String>,
    pub https: bool,
}

impl TestRequest {
    /// default
    pub fn default(url_path: &str, port: u16) -> Self {
        Self {
            host: LOCALHOST.to_owned(),
            port,
            url_path: url_path.to_owned(),
            http_method: None,
            headers: None,
            body: None,
            https: false,
        }
    }

    /// default with https protocol
    pub fn with_https(mut self) -> Self {
        self.https = true;
        self
    }

    /// default with server host
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.to_owned();
        self
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

    /// send request to get http(s) response from mock server
    pub async fn send(&self) -> Response {
        let protocol = if self.https { "https" } else { "http" };
        let socket_addrs = format!("{}:{}", self.host, self.port);
        let url_str = format!("{}://{}{}", protocol, socket_addrs, self.url_path);
        let url = match reqwest::Url::parse(url_str.as_str()) {
            Ok(x) => x,
            Err(err) => panic!("failed to parse url: {} ({:?})", url_str, err),
        };

        let http_method = if let Some(http_method) = self.http_method.as_ref() {
            http_method.to_owned()
        } else {
            Method::GET
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            hyper::header::HOST,
            HeaderValue::from_str(socket_addrs.as_str())
                .expect("failed to get header value on socket addrs"),
        );
        if let Some(h) = self.headers.as_ref() {
            for (header_key, header_value) in h.iter() {
                headers.insert(header_key, header_value.to_owned());
            }
        }

        let mut client_builder = Client::builder();
        if self.https {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }
        let client = client_builder.build().expect("failed to build client");

        let mut request_builder = client.request(http_method, url).headers(headers);
        if let Some(body) = self.body.clone() {
            request_builder = request_builder.body(body);
        }
        request_builder
            .send()
            .await
            .expect("failed to get https response")
    }
}
