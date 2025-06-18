use console::style;
use serde::Deserialize;
use strategy::Strategy;
use util::canonicalized_fallback_respond_dir_to_print;

use std::path::Path;

pub mod strategy;
mod util;

use super::constant::{PRINT_DELIMITER, SERVICE_DEFAULT_FALLBACK_RESPOND_DIR};
use crate::core::server::{
    middleware::Middleware,
    parsed_request::ParsedRequest,
    response::{error_response::internal_server_error_response, file_response::FileResponse},
    routing::rule_set::RuleSet,
    types::BoxBody,
};

/// verbose logs
#[derive(Clone, Deserialize)]
pub struct ServiceConfig {
    // routing
    pub strategy: Option<Strategy>,
    #[serde(rename = "rule_sets")]
    pub rule_sets_file_paths: Option<Vec<String>>,
    #[serde(skip)]
    pub rule_sets: Vec<RuleSet>,

    #[serde(rename = "middlewares")]
    pub middlewares_file_paths: Option<Vec<String>>,
    #[serde(skip)]
    pub middlewares: Vec<Middleware>,

    pub fallback_respond_dir: String,
}

impl ServiceConfig {
    /// handle middleware(s)
    pub async fn middleware_response(
        &self,
        parsed_request: &ParsedRequest,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        for middleware in self.middlewares.iter() {
            let middleware_response_file_path = match middleware.handle(
                parsed_request.url_path.as_str(),
                parsed_request.body_json.as_ref(),
            ) {
                Some(x) => x,
                None => continue,
            };

            let response_file_path =
                if Path::new(middleware_response_file_path.as_str()).is_absolute() {
                    middleware_response_file_path
                } else {
                    let middleware_dir_path = Path::new(middleware.file_path.as_str()).parent();

                    let joined_file_path = match middleware_dir_path {
                        Some(x) => x.join(middleware_response_file_path.as_str()),
                        None => {
                            return Some(internal_server_error_response(
                                &format!(
                                    "failed to get middleware parent dir: {}",
                                    middleware.file_path.as_str(),
                                ),
                                &parsed_request.component_parts.headers,
                            ))
                        }
                    };

                    match joined_file_path.to_str() {
                        Some(x) => x.to_owned(),
                        None => {
                            return Some(internal_server_error_response(
                                &format!(
                                    "middleware response file path is invalid: {}/{}",
                                    middleware.file_path.as_str(),
                                    middleware_response_file_path
                                ),
                                &parsed_request.component_parts.headers,
                            ))
                        }
                    }
                };

            return Some(
                FileResponse::new(
                    response_file_path.as_str(),
                    None,
                    &parsed_request.component_parts.headers,
                )
                .file_content_response()
                .await,
            );
        }
        None
    }

    /// handle on `rule_sets`
    pub async fn rule_set_response(
        &self,
        parsed_request: &ParsedRequest,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        for (rule_set_idx, rule_set) in self.rule_sets.iter().enumerate() {
            match rule_set.find_matched(parsed_request, self.strategy.as_ref(), rule_set_idx) {
                Some(respond) => {
                    let dir_prefix = rule_set.dir_prefix();
                    let response = respond.response(dir_prefix.as_str(), &parsed_request).await;
                    return Some(response);
                }
                None => (),
            }
        }
        None
    }

    /// validate
    pub fn validate(&self) -> bool {
        let rule_sets_validate =
            self.rule_sets
                .iter()
                .enumerate()
                .all(|(rule_set_idx, rule_set)| {
                    let prefix_validate = rule_set.prefix.is_none()
                        || rule_set.prefix.as_ref().unwrap().validate(rule_set_idx);

                    let default_validate =
                        rule_set.default.is_none() || rule_set.default.as_ref().unwrap().validate();

                    let guard_validate =
                        rule_set.guard.is_none() || rule_set.guard.as_ref().unwrap().validate();

                    let dir_prefix = rule_set.dir_prefix();
                    let rules_validate =
                        rule_set.rules.iter().enumerate().all(|(rule_idx, rule)| {
                            rule.when.validate(rule_idx, rule_set_idx)
                                && rule.respond.validate(
                                    dir_prefix.as_str(),
                                    rule_idx,
                                    rule_set_idx,
                                )
                        });

                    prefix_validate && default_validate && guard_validate && rules_validate
                });
        if !rule_sets_validate {
            log::error!("something wrong in rule sets");
        }

        let fallback_respond_dir_validate = Path::new(self.fallback_respond_dir.as_str()).exists();
        if !fallback_respond_dir_validate {
            log::error!(
                "{} fallback_respond_dir: {}",
                style("invalid").red(),
                self.fallback_respond_dir
            );
        }

        rule_sets_validate && fallback_respond_dir_validate
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            strategy: Some(Strategy::default()),
            rule_sets_file_paths: None,
            rule_sets: vec![],
            middlewares_file_paths: None,
            middlewares: vec![],
            fallback_respond_dir: SERVICE_DEFAULT_FALLBACK_RESPOND_DIR.to_owned(),
        }
    }
}

impl std::fmt::Display for ServiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_rule_sets = !self.rule_sets.is_empty();

        if has_rule_sets {
            let _ = writeln!(
                f,
                "[rule_sets.strategy] {}",
                self.strategy.clone().unwrap_or_default()
            );
            let _ = writeln!(f, "");
        }

        for (idx, rule_set) in self.rule_sets.iter().enumerate() {
            let _ = writeln!(
                f,
                "@ rule_set #{} ({})\n",
                idx + 1,
                style(rule_set.file_path.as_str()).green()
            );
            let _ = write!(f, "{}\n", rule_set);
        }

        if has_rule_sets {
            let _ = writeln!(f, "{}", PRINT_DELIMITER);
        }

        let _ = writeln!(
            f,
            "[fallback_respond_dir] {}",
            canonicalized_fallback_respond_dir_to_print(self.fallback_respond_dir.as_str())
        );

        Ok(())
    }
}
