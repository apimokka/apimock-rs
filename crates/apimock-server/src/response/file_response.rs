use console::style;
use hyper::HeaderMap;
use serde_json::{Map, Value};
use tokio::task;

use std::{collections::HashMap, fs, path::Path};

use crate::{
    constant::CSV_RECORDS_DEFAULT_KEY,
    json_path_util::resolve_with_json_compatible_extensions,
    response::{
        confine::confine, error_response::not_found_response, json_response::json_response,
    },
    response_handler::ResponseHandler,
    types::BoxBody,
};

use super::{
    error_response::internal_server_error_response,
    text_response::text_response,
    util::{
        binary_content_type, file_extension, json_value_with_jsonpath_key, text_file_content_type,
    },
};

pub struct FileResponse {
    file_path: String,
    csv_records_key: Option<String>,
    text_content: Option<String>,
    binary_content: Option<Vec<u8>>,
    custom_headers: Option<HashMap<String, Option<String>>>,
    request_headers: HeaderMap,
    /// The directory `file_path` must resolve inside, already
    /// canonicalised by the caller. `None` means it couldn't be (the
    /// directory doesn't exist) — every candidate is then refused,
    /// never served unchecked.
    confine_to: Option<std::path::PathBuf>,
    /// RFC 067 — see `response_handler::default_response_headers`.
    cors_allow_credentials_origins: Vec<String>,
}

impl FileResponse {
    /// create instance
    pub fn new(
        file_path: &str,
        custom_headers: Option<&HashMap<String, Option<String>>>,
        request_headers: &HeaderMap,
        confine_to: Option<&Path>,
        cors_allow_credentials_origins: &[String],
    ) -> Self {
        FileResponse {
            file_path: file_path.to_owned(),
            csv_records_key: None,
            text_content: None,
            binary_content: None,
            custom_headers: custom_headers.cloned(),
            request_headers: request_headers.clone(),
            confine_to: confine_to.map(Path::to_path_buf),
            cors_allow_credentials_origins: cors_allow_credentials_origins.to_vec(),
        }
    }

    /// create instance
    pub fn new_with_csv_records_jsonpath(
        file_path: &str,
        custom_headers: Option<&HashMap<String, Option<String>>>,
        csv_records_key: Option<String>,
        request_headers: &HeaderMap,
        confine_to: Option<&Path>,
        cors_allow_credentials_origins: &[String],
    ) -> Self {
        let mut ret = FileResponse::new(
            file_path,
            custom_headers,
            request_headers,
            confine_to,
            cors_allow_credentials_origins,
        );
        ret.csv_records_key = csv_records_key;
        ret
    }

    /// response from file path
    pub async fn file_content_response(
        &mut self,
    ) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let file_path = match resolve_with_json_compatible_extensions(self.file_path.as_str()) {
            Some(x) => x,
            None => {
                log::warn!(
                    "{}:\n{} (missing or a directory)",
                    style("file not found").red(),
                    self.file_path
                );
                return not_found_response(
                    &self.request_headers,
                    &self.cors_allow_credentials_origins,
                );
            }
        };

        // Confine the resolved candidate to the directory it was meant
        // to come from. This runs after extension/`index.*` resolution
        // above, so it also catches a path that only escapes at that
        // stage (e.g. a symlinked `index.html`), not only one that
        // arrived already outside.
        let file_path = match confine(file_path.as_str(), self.confine_to.as_deref()) {
            Some(canonical) => match canonical.to_str() {
                Some(x) => x.to_owned(),
                None => {
                    log::error!(
                        "{} to get str from canonicalized file path:\n{}",
                        style("failed").red(),
                        file_path
                    );
                    return not_found_response(
                        &self.request_headers,
                        &self.cors_allow_credentials_origins,
                    );
                }
            },
            None => {
                return not_found_response(
                    &self.request_headers,
                    &self.cors_allow_credentials_origins,
                );
            }
        };
        self.file_path = file_path.clone();

        // read file as text file in non-blocking task
        let file_path_to_read_text_file = file_path.clone();
        let content =
            task::spawn_blocking(move || fs::read_to_string(file_path_to_read_text_file)).await;

        match content {
            Ok(Ok(content)) => {
                self.text_content = Some(content);
                self.text_file_content_response()
            }
            Ok(Err(_)) => {
                // read file as binary in non-blocking task
                let file_path_to_read_binary = file_path.clone();
                let content =
                    task::spawn_blocking(move || fs::read(file_path_to_read_binary)).await;
                match content {
                    Ok(Ok(content)) => {
                        self.binary_content = Some(content);
                        self.binary_content_type_response()
                    }
                    Ok(Err(err)) => {
                        log::error!("failed to read file ({}): {}", self.file_path, err);
                        internal_server_error_response(
                            "failed to read response file",
                            &self.request_headers,
                            &self.cors_allow_credentials_origins,
                        )
                    }
                    Err(err) => {
                        log::error!("async task failed ({}): {}", self.file_path, err);
                        internal_server_error_response(
                            "failed to read response file",
                            &self.request_headers,
                            &self.cors_allow_credentials_origins,
                        )
                    }
                }
            }
            Err(err) => {
                log::error!("async task failed ({}): {}", self.file_path, err);
                internal_server_error_response(
                    "failed to read response file",
                    &self.request_headers,
                    &self.cors_allow_credentials_origins,
                )
            }
        }
    }

    /// text file response
    ///
    /// `self.custom_headers` is threaded through here the same way
    /// `json_file_content_response`/`csv_file_content_response` already
    /// do below - this branch previously hardcoded `None`, silently
    /// dropping every custom header on a plain-text `file_path` response
    /// (RFC 045 Defect 1, extended: this contradicted the RFC's own
    /// "`file_path` | honoured" claim, which held only for the
    /// json/json5/csv sub-cases).
    fn text_file_content_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        match file_extension(self.file_path.as_str()) {
            Some(ext) => match ext.as_str() {
                "json" | "json5" => self.json_file_content_response(),
                "csv" => self.csv_file_content_response(),
                _ => text_response(
                    self.text_content.clone().unwrap_or_default().as_str(),
                    Some(text_file_content_type(ext).as_str()),
                    self.custom_headers.as_ref(),
                    &self.request_headers,
                    &self.cors_allow_credentials_origins,
                ),
            },
            None => text_response(
                self.text_content.clone().unwrap_or_default().as_str(),
                None,
                self.custom_headers.as_ref(),
                &self.request_headers,
                &self.cors_allow_credentials_origins,
            ),
        }
    }

    /// json file response
    fn json_file_content_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let json_str = self.text_content.clone().unwrap_or_default();
        json_response(
            json_str.as_str(),
            None,
            self.custom_headers.as_ref(),
            &self.request_headers,
            Some(self.file_path.as_str()),
            &self.cors_allow_credentials_origins,
        )
    }

    /// csv file response
    fn csv_file_content_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let text_content = self.text_content.clone().unwrap_or_default();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(text_content.as_bytes());

        let csv_headers = if let Ok(csv_headers) = rdr.headers() {
            csv_headers.clone()
        } else {
            log::error!(
                "failed to analyze csv headers ({})",
                self.file_path.as_str()
            );
            return internal_server_error_response(
                "failed to analyze csv headers",
                &self.request_headers,
                &self.cors_allow_credentials_origins,
            );
        };

        let rows = rdr
            .records()
            .map(|result| {
                let record = result?;
                let obj = csv_headers
                    .iter()
                    .zip(record.iter())
                    .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
                    .collect::<Map<_, _>>();
                Ok(Value::Object(obj))
            })
            .collect::<Result<Vec<Value>, csv::Error>>();

        match rows {
            Ok(rows) => {
                let jsonpath_key = if let Some(csv_records_key) = self.csv_records_key.as_ref() {
                    csv_records_key.as_str()
                } else {
                    CSV_RECORDS_DEFAULT_KEY
                };
                let json_value = json_value_with_jsonpath_key(jsonpath_key, Value::from(rows));

                let body = serde_json::to_string(&json_value);
                match body {
                    Ok(body) => json_response(
                        body.as_str(),
                        None,
                        self.custom_headers.as_ref(),
                        &self.request_headers,
                        Some(self.file_path.as_str()),
                        &self.cors_allow_credentials_origins,
                    ),
                    Err(err) => {
                        log::error!(
                            "failed to convert csv records to json response ({}): {}",
                            self.file_path.as_str(),
                            err
                        );
                        internal_server_error_response(
                            "failed to convert csv records to json response",
                            &self.request_headers,
                            &self.cors_allow_credentials_origins,
                        )
                    }
                }
            }
            Err(err) => {
                log::error!(
                    "failed to analyze csv records ({}): {}",
                    self.file_path.as_str(),
                    err
                );
                internal_server_error_response(
                    "failed to analyze csv records",
                    &self.request_headers,
                    &self.cors_allow_credentials_origins,
                )
            }
        }
    }

    /// binary file response
    ///
    /// `with_custom_headers` runs *after* `with_binary_body` (RFC 065)
    /// — previously reversed, the same ordering bug as `json_response`
    /// (D2): `with_binary_body` always sets a derived `content-type`,
    /// so applying custom headers first let that overwrite an explicit
    /// one every time, on every binary `file_path` response (`.png`,
    /// `.pdf`, …).
    fn binary_content_type_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let content = self.binary_content.clone().unwrap_or_default().to_owned();
        let content_type = binary_content_type(self.file_path.as_str());
        ResponseHandler::default()
            .with_binary_body(content, Some(content_type))
            .with_custom_headers(self.custom_headers.as_ref())
            .into_response(&self.request_headers, &self.cors_allow_credentials_origins)
    }
}
