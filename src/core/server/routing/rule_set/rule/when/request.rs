use console::style;
use http_method::HttpMethod;
use serde::Deserialize;

mod body;
mod headers;
mod http_method;
pub mod rule_op;
pub mod url_path;
mod util;

use crate::core::server::parsed_request::ParsedRequest;
use body::Body;
use headers::Headers;
use url_path::{UrlPath, UrlPathConfig};
use util::fmt_condition_connector;

#[derive(Clone, Deserialize, Debug)]
pub struct Request {
    #[serde(rename = "url_path")]
    pub url_path_config: Option<UrlPathConfig>,
    #[serde(skip)]
    pub url_path: Option<UrlPath>,
    #[serde(rename = "method")]
    pub http_method: Option<HttpMethod>,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
}

impl Request {
    /// match with condition
    pub fn is_match(
        &self,
        parsed_request: &ParsedRequest,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> bool {
        let url_path_is_match = self.url_path.is_none()
            || self
                .url_path
                .as_ref()
                .unwrap()
                .is_match(parsed_request.url_path.as_str());

        let http_method_is_match = self.http_method.is_none()
            || self
                .http_method
                .as_ref()
                .unwrap()
                .is_match(&parsed_request.component_parts.method);

        let headers_is_match = self.headers.is_none()
            || self.headers.as_ref().unwrap().is_match(
                &parsed_request.component_parts.headers,
                rule_idx,
                rule_set_idx,
            );

        let body_is_match =
            self.body.is_none() || self.body.as_ref().unwrap().is_match(&parsed_request);

        url_path_is_match && http_method_is_match && headers_is_match && body_is_match
    }

    /// validate
    pub fn validate(&self, rule_idx: usize, rule_set_idx: usize) -> bool {
        if self.url_path.is_none()
            && self.http_method.is_none()
            && self.headers.is_none()
            && self.body.is_none()
        {
            log::error!(
                "{} in when.request at least either of: url_path, method, headers or body (rule #{} in rule set #{})",
                style("required").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        let url_path_validate = match self.url_path.as_ref() {
            Some(url_path) => url_path.validate(),
            None => true,
        };

        let http_method_validate = true;

        let headers_validate = match self.headers.as_ref() {
            Some(headers) => {
                let ret = headers.validate();
                if !ret {
                    log::error!(
                        "something wrong in headers (rule #{} in rule set #{})",
                        rule_idx + 1,
                        rule_set_idx + 1
                    )
                }
                ret
            }
            None => true,
        };

        let body_validate = match self.body.as_ref() {
            Some(body) => {
                let ret = body.validate();
                if !ret {
                    log::error!(
                        "something wrong in body (rule #{} in rule set #{})",
                        rule_idx + 1,
                        rule_set_idx + 1
                    )
                }
                ret
            }
            None => true,
        };

        url_path_validate && http_method_validate && headers_validate && body_validate
    }
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s: Vec<String> = vec![];

        if let Some(x) = self.url_path_config.as_ref() {
            s.push(format!("{}", x));
        }
        if let Some(x) = self.http_method.as_ref() {
            s.push(format!("{}", x));
        }
        if let Some(x) = self.headers.as_ref() {
            s.push(format!("{}", x));
        }
        if let Some(x) = self.body.as_ref() {
            s.push(format!("{}", x));
        }

        let _ = write!(f, "{} ", s.join(fmt_condition_connector().as_str()));
        Ok(())
    }
}
