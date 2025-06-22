use console::style;
use hyper::HeaderMap;
use serde_json::{Map, Value};
use tokio::task;

use std::{collections::HashMap, fs};

use crate::core::{
    server::{
        constant::CSV_RECORDS_DEFAULT_KEY,
        response::{error_response::not_found_response, json_response::json_response},
        response_handler::ResponseHandler,
        types::BoxBody,
    },
    util::json::resolve_with_json_compatible_extensions,
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
}

impl FileResponse {
    /// create instance
    pub fn new(
        file_path: &str,
        custom_headers: Option<&HashMap<String, Option<String>>>,
        request_headers: &HeaderMap,
    ) -> Self {
        FileResponse {
            file_path: file_path.to_owned(),
            csv_records_key: None,
            text_content: None,
            binary_content: None,
            custom_headers: custom_headers.cloned(),
            request_headers: request_headers.clone(),
        }
    }

    /// create instance
    pub fn new_with_csv_records_jsonpath(
        file_path: &str,
        custom_headers: Option<&HashMap<String, Option<String>>>,
        csv_records_key: Option<String>,
        request_headers: &HeaderMap,
    ) -> Self {
        let mut ret = FileResponse::new(file_path, custom_headers, request_headers);
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
                return not_found_response(&self.request_headers);
            }
        };
        self.file_path = file_path.clone();

        // read file as text file in non-blocking task
        let file_path_to_read_text_file = file_path.clone();
        let content =
            task::spawn_blocking(move || fs::read_to_string(file_path_to_read_text_file)).await;

        let response = match content {
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
                        return internal_server_error_response(
                            &format!("{}: failed to read file - {}", self.file_path, err),
                            &self.request_headers,
                        )
                    }
                    Err(err) => {
                        return internal_server_error_response(
                            &format!("{}: async task failed - {}", self.file_path, err),
                            &self.request_headers,
                        )
                    }
                }
            }
            Err(err) => {
                return internal_server_error_response(
                    &format!("{}: async task failed - {}", self.file_path, err),
                    &self.request_headers,
                )
            }
        };

        response
    }

    /// text file response
    fn text_file_content_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        match file_extension(self.file_path.as_str()) {
            Some(ext) => match ext.as_str() {
                "json" | "json5" => self.json_file_content_response(),
                "csv" => self.csv_file_content_response(),
                _ => text_response(
                    self.text_content.clone().unwrap_or_default().as_str(),
                    Some(text_file_content_type(ext).as_str()),
                    None,
                    &self.request_headers,
                ),
            },
            None => text_response(
                self.text_content.clone().unwrap_or_default().as_str(),
                None,
                None,
                &self.request_headers,
            ),
        }
    }

    /// json file response
    fn json_file_content_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let json_str = self.text_content.clone().unwrap_or_default();
        json_response(
            json_str.as_str(),
            self.custom_headers.as_ref(),
            &self.request_headers,
            self.file_path.as_str(),
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
            return internal_server_error_response(
                &format!("{}: failed to analyze csv headers", self.file_path.as_str()),
                &self.request_headers,
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
                        self.custom_headers.as_ref(),
                        &self.request_headers,
                        self.file_path.as_str(),
                    ),
                    Err(err) => internal_server_error_response(
                        &format!(
                            "{}: failed to convert csv records to json response - {}",
                            self.file_path.as_str(),
                            err
                        ),
                        &self.request_headers,
                    ),
                }
            }
            Err(err) => internal_server_error_response(
                &format!(
                    "{}: failed to analyze csv records - {}",
                    self.file_path.as_str(),
                    err
                ),
                &self.request_headers,
            ),
        }
    }

    /// binary file response
    fn binary_content_type_response(&self) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let mut response_handler = ResponseHandler::default();

        if let Some(custom_headers) = self.custom_headers.clone() {
            response_handler = response_handler.with_headers(custom_headers);
        }

        let content = self.binary_content.clone().unwrap_or_default().to_owned();
        let content_type = binary_content_type(self.file_path.as_str());
        response_handler
            .with_binary_body(content, Some(content_type))
            .into_response(&self.request_headers)
    }
}
