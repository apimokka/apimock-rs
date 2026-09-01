use http_body_util::{BodyExt, Empty, Full};
use hyper::{
    HeaderMap, StatusCode,
    body::{Body, Bytes},
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_LENGTH, HeaderName,
        HeaderValue, ORIGIN, VARY,
    },
    http::response::Builder,
};

use std::{collections::HashMap, str::FromStr};

use super::{
    constant::DEFAULT_RESPONSE_HEADERS, response::error_response::internal_server_error_response,
};
use crate::types::BoxBody;

#[derive(Clone, Default)]
pub enum BodyKind {
    #[default]
    Empty,
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Default)]
pub struct ResponseHandler {
    response_builder: Builder,
    status: Option<StatusCode>,
    headers: HashMap<String, Option<String>>,
    body_kind: BodyKind,
}

impl ResponseHandler {
    /// build response
    pub fn into_response(
        self,
        request_headers: &HeaderMap,
        cors_allow_credentials_origins: &[String],
    ) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        // - body + content-length
        let response = match self.body_kind {
            BodyKind::Text(s) => self
                .response_builder
                .body(Full::new(Bytes::from(s.to_owned())).boxed()),
            BodyKind::Binary(b) => self
                .response_builder
                .body(Full::new(Bytes::from(b)).boxed()),
            BodyKind::Empty => self.response_builder.body(Empty::new().boxed()),
        };

        let mut response = match response {
            Ok(x) => x,
            Err(err) => {
                return internal_server_error_response(
                    &format!("failed to create response: {}", err),
                    request_headers,
                    cors_allow_credentials_origins,
                );
            }
        };

        // - http status code
        *response.status_mut() = if let Some(status) = self.status {
            status
        } else {
            StatusCode::OK
        };

        // - content-length
        let content_length = response.body().size_hint().exact().unwrap_or_default();

        let headers = response.headers_mut();

        headers.insert(CONTENT_LENGTH, HeaderValue::from(content_length));

        // - the other default headers
        for (header_key, header_value) in
            default_response_headers(request_headers, cors_allow_credentials_origins).iter()
        {
            headers.insert(header_key, header_value.to_owned());
        }

        // - additional custom headers passed from caller
        for (header_key, header_value) in self.headers {
            match HeaderName::from_str(header_key.as_str()) {
                Ok(header_key) => {
                    match HeaderValue::from_str(header_value.unwrap_or_default().as_str()) {
                        Ok(header_value) => {
                            headers.insert(header_key, header_value);
                        }
                        Err(err) => {
                            log::warn!(
                                "failed to create header with the header value (header key = {}) ({})",
                                header_key,
                                err
                            );
                            headers.insert(header_key, HeaderValue::from_static(""));
                        }
                    }
                }
                Err(err) => log::warn!(
                    "failed to create header with the header key: {} ({})",
                    header_key,
                    err
                ),
            };
        }

        Ok(response)
    }

    /// set http status code
    pub fn with_status(mut self, status: &StatusCode) -> Self {
        self.status = Some(status.to_owned());
        self
    }

    /// add custom header
    pub fn with_header(mut self, key: impl Into<String>, value: Option<impl Into<String>>) -> Self {
        self.headers.insert(key.into(), value.map(|x| x.into()));
        self
    }

    /// add custom headers
    pub fn with_headers<K, V, I>(mut self, headers: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, Option<V>)>,
    {
        for (key, value) in headers {
            self.headers.insert(key.into(), value.map(|x| x.into()));
        }
        self
    }

    /// Apply an operator's custom `respond.headers`, always **last** —
    /// after whichever `with_text`/`with_json_body`/`with_binary_body`
    /// call already set a default `content-type` (RFC 065's override
    /// rule: an explicit `content-type` always wins over the derived
    /// default, on every body source, uniformly).
    ///
    /// # Why one method instead of each call site's own `if let`
    ///
    /// Before this, every response-building function repeated
    /// `if let Some(custom_headers) = custom_headers { response_handler
    /// = response_handler.with_headers(custom_headers.to_owned()); }`
    /// itself, and the two places that got the ordering wrong
    /// (`json_response`, and `FileResponse`'s own binary-file path) each
    /// silently let the derived content-type win instead — because
    /// `self.headers` is a plain `HashMap`, whichever call happens last
    /// wins for that key, and there was nothing forcing "last" to always
    /// mean "the custom headers." Routing every call site through this
    /// one method, called only after the body is set, makes that
    /// ordering the only way to call it — not a convention that can
    /// drift a third time.
    pub fn with_custom_headers(
        self,
        custom_headers: Option<&HashMap<String, Option<String>>>,
    ) -> Self {
        match custom_headers {
            Some(custom_headers) => self.with_headers(custom_headers.to_owned()),
            None => self,
        }
    }

    /// add text to body
    pub fn with_text(mut self, text: impl Into<String>, content_type: Option<&str>) -> Self {
        let content_type = if let Some(content_type) = content_type {
            content_type.into()
        } else {
            "text/plain; charset=utf-8".to_owned()
        };
        self.headers
            .insert("content-type".into(), Some(content_type));

        self.body_kind = BodyKind::Text(text.into());
        self
    }

    /// treat response as json
    pub fn with_json_body(mut self, body: impl Into<String>) -> Self {
        self.headers
            .insert("content-type".into(), Some("application/json".into()));
        self.body_kind = BodyKind::Text(body.into());
        self
    }

    /// treat response as json
    pub fn with_binary_body(
        mut self,
        body: Vec<u8>,
        content_type: Option<impl Into<String>>,
    ) -> Self {
        let content_type = if let Some(content_type) = content_type {
            content_type.into()
        } else {
            "application/octet-stream".to_owned()
        };
        self.headers
            .insert("content-type".into(), Some(content_type));

        self.body_kind = BodyKind::Binary(body);

        self
    }
}

/// default response headers key-value pairs.
///
/// `cors_allow_credentials_origins` is RFC 067's
/// `[service].cors_allow_credentials_origins` — exact origin strings
/// (beyond the implicitly-allowed loopback ones, see
/// `is_credentialed_reflection_allowed` — private to this crate, not
/// linked here since rustdoc's public docs can't resolve a private
/// item) allowed credentialed reflection. Empty for a caller that has
/// no config in scope (e.g. a fixed 204 preflight built with no
/// request context) — degrading to the safe, non-credentialed path is
/// correct there, never the reverse.
pub fn default_response_headers(
    request_headers: &HeaderMap,
    cors_allow_credentials_origins: &[String],
) -> HeaderMap {
    let mut header_map_src = Vec::with_capacity(DEFAULT_RESPONSE_HEADERS.len() + 1);

    // resource
    // - the other default headers but access-control-allow-origin, vary
    header_map_src.extend(
        DEFAULT_RESPONSE_HEADERS
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string())),
    );

    // - access-control-allow-origin, vary
    //
    // RFC 067: a credentialed request (Cookie/Authorization present)
    // only gets its Origin reflected — and Access-Control-Allow-Credentials:
    // true — when that origin is allowed (see
    // `is_credentialed_reflection_allowed`). An origin the operator
    // never named gets exactly the same `origin = None` path a
    // non-credentialed request takes below: `ACAO: *`, no credentials.
    // The response is still served either way — refusing credentialed
    // cross-origin *reads* of it is the browser's job, not this
    // server's, and erroring here would also break the many requests
    // that carry a `Cookie` incidentally and need no CORS at all.
    let origin = if is_likely_authenticated_request(request_headers) {
        request_headers
            .get(ORIGIN)
            .and_then(|value| value.to_str().ok())
            .filter(|origin| {
                is_credentialed_reflection_allowed(origin, cors_allow_credentials_origins)
            })
            .and_then(|origin| HeaderValue::from_str(origin).ok())
    } else {
        None
    };
    let (origin, vary) = if let Some(origin) = origin {
        header_map_src.push((
            ACCESS_CONTROL_ALLOW_CREDENTIALS.to_string(),
            "true".to_owned(),
        ));

        (origin, HeaderValue::from_static("Origin"))
    } else {
        (HeaderValue::from_static("*"), HeaderValue::from_static("*"))
    };
    header_map_src.push((
        ACCESS_CONTROL_ALLOW_ORIGIN.to_string(),
        origin.to_str().unwrap_or_default().to_owned(),
    ));
    header_map_src.push((
        VARY.to_string(),
        vary.to_str().unwrap_or_default().to_owned(),
    ));

    // header map
    header_map_src
        .iter()
        .fold(HeaderMap::new(), |mut ret, (header_key, header_value)| {
            match HeaderName::from_str(header_key) {
                Ok(header_key) => match HeaderValue::from_str(header_value.as_str()) {
                    Ok(header_value) => {
                        ret.insert(header_key, header_value);
                        ret
                    }
                    Err(err) => {
                        log::warn!(
                            "only header key set because failed to get header value: {} [key = {}] ({})",
                            header_value.as_str(),
                            header_key,
                            err
                        );
                        ret.insert(header_key, HeaderValue::from_static(""));
                        ret
                    }
                },
                Err(err) => {
                    log::warn!("failed to set header key: {} ({})", header_key, err);
                    ret
                }
            }
        })
}

/// guess if the request is likely related to authentication
fn is_likely_authenticated_request(request_headers: &HeaderMap) -> bool {
    request_headers.contains_key("cookie") || request_headers.contains_key("authorization")
}

/// RFC 067: whether `origin` gets credentialed CORS reflection.
/// Loopback origins are allowed regardless of config (see
/// [`is_implicit_loopback_origin`]); every other origin must appear,
/// exactly, in `cors_allow_credentials_origins`.
fn is_credentialed_reflection_allowed(
    origin: &str,
    cors_allow_credentials_origins: &[String],
) -> bool {
    is_implicit_loopback_origin(origin)
        || cors_allow_credentials_origins
            .iter()
            .any(|allowed| allowed == origin)
}

/// RFC 067 § Design, "the convenience question": `http://localhost:*`
/// and `http://127.0.0.1:*` are allowed credentialed reflection without
/// any configuration — a page served from the developer's own machine
/// is already inside the trust boundary the loopback bind assumes, so
/// this keeps "front-end on :5173, mock on :3001" working untouched.
///
/// Deliberately a plain prefix-plus-suffix check, not a URL parser: the
/// two exact hosts this recognises don't need one, and getting this
/// wrong in the permissive direction is the whole class of bug this
/// RFC exists to close, so the check is written to fail closed on
/// anything not exactly `http://localhost[:<port>]` or
/// `http://127.0.0.1[:<port>]` — in particular, `http://localhost.evil.example`
/// does *not* match (the suffix after the prefix is neither empty nor
/// `:<digits>`), and neither does `https://localhost` (a scheme
/// mismatch): only what RFC 067's own examples specify.
fn is_implicit_loopback_origin(origin: &str) -> bool {
    for prefix in ["http://localhost", "http://127.0.0.1"] {
        let Some(rest) = origin.strip_prefix(prefix) else {
            continue;
        };
        if rest.is_empty() {
            return true;
        }
        return rest
            .strip_prefix(':')
            .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()));
    }
    false
}
