use serde::Deserialize;

mod body;
mod headers;
pub mod rule_op;
pub mod url_path;

use body::Body;
use headers::Headers;
use url_path::{UrlPath, UrlPathConfig};

use crate::core::server::parsed_request::ParsedRequest;

#[derive(Clone, Deserialize, Debug)]
pub struct Request {
    #[serde(rename = "url_path")]
    pub url_path_config: Option<UrlPathConfig>,
    #[serde(skip)]
    pub url_path: Option<UrlPath>,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
}

impl Request {
    /// match with condition
    pub fn is_match(
        &self,
        sent_request: &ParsedRequest,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> bool {
        let url_path_is_match = self.url_path.is_none()
            || self
                .url_path
                .as_ref()
                .unwrap()
                .is_match(sent_request.url_path.as_str());

        let headers_is_match = self.headers.is_none()
            || self.headers.as_ref().unwrap().is_match(
                &sent_request.component_parts.headers,
                rule_idx,
                rule_set_idx,
            );

        let body_is_match =
            self.body.is_none() || self.body.as_ref().unwrap().is_match(&sent_request);

        url_path_is_match && headers_is_match && body_is_match
    }

    /// validate
    pub fn validate(&self, rule_idx: usize, rule_set_idx: usize) -> bool {
        if self.url_path.is_none() && self.headers.is_none() && self.body.is_none() {
            log::error!("either of url_path, headers or body in when.request is required (rule #{} in rule set #{})", rule_idx + 1, rule_set_idx + 1);
            return false;
        }

        let url_path_validate = match self.url_path.as_ref() {
            Some(url_path) => url_path.validate(),
            None => true,
        };

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

        url_path_validate && headers_validate && body_validate
    }
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(x) = self.url_path_config.as_ref() {
            let _ = write!(f, "{} ", x);
        }
        if let Some(x) = self.headers.as_ref() {
            let _ = write!(f, "{} ", x);
        }
        if let Some(x) = self.body.as_ref() {
            let _ = write!(f, "{} ", x);
        }
        Ok(())
    }
}
