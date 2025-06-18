use console::style;
use hyper::StatusCode;
use serde::Deserialize;
use util::full_file_path;

use std::{collections::HashMap, path::Path};

mod util;

use crate::core::{
    server::{
        parsed_request::ParsedRequest,
        response::{
            error_response::internal_server_error_response,
            file_response::FileResponse,
            status_code_response::{status_code_response, status_code_response_with_message},
            text_response::text_response,
        },
        types::BoxBody,
    },
    util::http::delay_response,
};

#[derive(Clone, Deserialize, Debug)]
pub struct Respond {
    pub file_path: Option<String>,
    pub csv_records_key: Option<String>,
    pub text: Option<String>,
    pub status: Option<u16>,
    #[serde(skip)]
    pub status_code: Option<StatusCode>,
    pub headers: Option<HashMap<String, Option<String>>>,
    pub delay_response_milliseconds: Option<u16>,
}

impl Respond {
    /// generate response
    pub async fn response(
        &self,
        dir_prefix: &str,
        parsed_request: &ParsedRequest,
    ) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        if let Some(delay_response_milliseconds) = self.delay_response_milliseconds {
            delay_response(delay_response_milliseconds).await;
        }

        if let Some(file_path) = self.file_path.as_ref() {
            let full_file_path = full_file_path(file_path.as_str(), dir_prefix);
            if full_file_path.is_none() {
                log::error!(
                    "{}:\n{} (prefix = {})",
                    style("file not found").red(),
                    self.file_path.clone().unwrap_or_default().as_str(),
                    dir_prefix
                );
                return internal_server_error_response(
                    "failed to get response file",
                    &parsed_request.component_parts.headers,
                );
            }
            FileResponse::new_with_csv_records_jsonpath(
                full_file_path.unwrap().as_str(),
                self.headers.as_ref(),
                self.csv_records_key.clone(),
                &parsed_request.component_parts.headers,
            )
            .file_content_response()
            .await
        } else if let Some(text) = self.text.as_ref() {
            if let Some(status_code) = self.status_code.as_ref() {
                status_code_response_with_message(
                    status_code,
                    text.as_str(),
                    &parsed_request.component_parts.headers,
                )
            } else {
                text_response(
                    text.as_str(),
                    None,
                    self.headers.as_ref(),
                    &parsed_request.component_parts.headers,
                )
            }
        } else if let Some(status_code) = self.status_code.as_ref() {
            status_code_response(status_code, &parsed_request.component_parts.headers)
        } else {
            internal_server_error_response(
                "invalid respond def",
                &parsed_request.component_parts.headers,
            )
        }
    }

    /// validate
    pub fn validate(&self, dir_prefix: &str, rule_idx: usize, rule_set_idx: usize) -> bool {
        let all_missing_of_file_path_text_status =
            self.file_path.is_none() && self.text.is_none() && self.status.is_none();
        if all_missing_of_file_path_text_status {
            log::error!(
                "{} at least either of: file_path, text or status (rule #{} in rule set #{})",
                style("required").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        let duplicate_file_path_text = self.file_path.is_some() && self.text.is_some();
        if duplicate_file_path_text {
            log::error!(
                "{} set at both file_path and text (rule #{} in rule set #{})",
                style("cannot").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        let file_path_with_status = self.file_path.is_some() && self.status.is_some();
        if file_path_with_status {
            log::error!(
                "{} use status with file_path. only with text (rule #{} in rule set #{})",
                style("cannot").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        if let Some(file_path) = self.file_path.as_ref() {
            file_path_validate(file_path.as_str(), dir_prefix, rule_idx, rule_set_idx)
        } else {
            true
        }
    }
}

impl std::fmt::Display for Respond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status_code) = self.status_code {
            let _ = writeln!(f, "status_code = {} ", status_code);
        }
        if let Some(text) = self.text.as_ref() {
            let _ = writeln!(f, "text = `{}` ", text);
        }
        if let Some(file_path) = self.file_path.as_ref() {
            let _ = writeln!(f, "file_path = `{}` ", file_path);
        }

        Ok(())
    }
}

/// validate on content with response type
fn file_path_validate(
    file_path: &str,
    dir_prefix: &str,
    rule_idx: usize,
    rule_set_idx: usize,
) -> bool {
    let p = Path::new(dir_prefix).join(file_path);
    let ret = p.exists();
    if !ret {
        log::error!(
            "{} (rule #{} in rule set #{}):\n`{}`",
            style("file not found").red(),
            rule_idx + 1,
            rule_set_idx + 1,
            p.to_str().unwrap_or_default(),
        );
    }
    ret
}
