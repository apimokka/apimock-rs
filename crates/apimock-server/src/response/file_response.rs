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

        // RFC 077 P-05: read the file's bytes once, then decide
        // text-vs-binary from the bytes already in hand — this used to
        // be two blocking reads (`read_to_string`, and on its failure a
        // second `read` of the same file for the binary fallback).
        // `String::from_utf8` reproduces exactly the same dispatch
        // `read_to_string`'s success/failure did (it fails on the same
        // input `read_to_string` would have failed to decode), so the
        // detection RFC 065's review pinned as load-bearing is
        // unchanged — see the tests below, written before this diff.
        let file_path_to_read = file_path.clone();
        let content = task::spawn_blocking(move || fs::read(file_path_to_read)).await;

        match content {
            Ok(Ok(bytes)) => match String::from_utf8(bytes) {
                Ok(text) => {
                    self.text_content = Some(text);
                    self.text_file_content_response()
                }
                Err(err) => {
                    self.binary_content = Some(err.into_bytes());
                    self.binary_content_type_response()
                }
            },
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
                // RFC 076: `.json` is served exactly as written — no
                // parse/reserialise round-trip. `.json5` still converts
                // (JSON5 is not JSON; converting it is the point, and a
                // user writing JSON5 has already accepted a
                // transformation) via `json_file_content_response` below,
                // unchanged.
                "json" => self.raw_json_file_content_response(),
                "json5" => self.json_file_content_response(),
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

    /// `.json` file response — served byte-for-byte, no parsing.
    ///
    /// # RFC 076: why skipping the parse/reserialise round-trip is safe
    ///
    /// The old path here (still used for `.json5`, see
    /// `json_file_content_response` below) parsed the file into a
    /// `Value` and reserialised it — minifying it and, before RFC 076's
    /// `preserve_order`, reordering keys alphabetically, relative to
    /// what was on disk. Neither is a validity check: RFC 065 already
    /// validates every `.json`/`.json5` `file_path` at config-load time
    /// (`Respond::validate`, the same JSON5 parser this module still
    /// uses for `.json5`), so parsing again here bought nothing but the
    /// two side effects above. Serving the bytes this method already
    /// read (`self.text_content`) is therefore both the byte-identical
    /// fix and the elimination of a redundant parse. If RFC 065's
    /// load-time validation is ever loosened or removed, this comment is
    /// the signal to reconsider whether a content check belongs here.
    fn raw_json_file_content_response(
        &self,
    ) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
        let json_str = self.text_content.clone().unwrap_or_default();
        ResponseHandler::default()
            .with_json_body(json_str)
            .with_custom_headers(self.custom_headers.as_ref())
            .into_response(&self.request_headers, &self.cors_allow_credentials_origins)
    }

    /// `.json5` file response — parsed and reserialised (unchanged by
    /// RFC 076; converting JSON5 to JSON is the point, not a defect).
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

#[cfg(test)]
mod tests {
    //! RFC 077 P-05: pinned *before* the read-twice-into-one-read
    //! refactor, per the tranche handoff. The dispatch between
    //! `text_file_content_response` and `binary_content_type_response`
    //! is decided by whether the file's bytes are valid UTF-8 — RFC
    //! 065's review established this as load-bearing — never by the
    //! file's extension. Both tests below deliberately mismatch
    //! extension against content to make that point unambiguous: a
    //! `.txt` file of invalid-UTF-8 bytes must still be served binary,
    //! and a `.bin` file of valid-UTF-8 bytes must still be served text.
    use hyper::HeaderMap;

    use super::*;
    use crate::response::confine::canonical_dir;

    #[tokio::test]
    async fn invalid_utf8_bytes_are_served_as_binary_regardless_of_a_text_extension() {
        let dir = tempfile::tempdir().unwrap();
        let bytes: &[u8] = &[0xFF, 0xFE, 0x00, 0x01, 0x02];
        let file_path = dir.path().join("weird.txt");
        std::fs::write(&file_path, bytes).unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let mut file_response = FileResponse::new(
            file_path.to_str().unwrap(),
            None,
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        );
        let response = file_response.file_content_response().await.unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/octet-stream",
            "invalid-UTF-8 bytes must take the binary path even though the \
             extension says .txt"
        );
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(body.as_ref(), bytes, "binary bytes must round-trip exactly");
    }

    #[tokio::test]
    async fn valid_utf8_bytes_are_served_as_text_regardless_of_a_binary_extension() {
        let dir = tempfile::tempdir().unwrap();
        let text = "hello, this is plain text";
        let file_path = dir.path().join("weird.bin");
        std::fs::write(&file_path, text).unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let mut file_response = FileResponse::new(
            file_path.to_str().unwrap(),
            None,
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        );
        let response = file_response.file_content_response().await.unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain; charset=utf-8",
            "valid-UTF-8 bytes must take the text path even though the \
             extension says .bin"
        );
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(body.as_ref(), text.as_bytes());
    }

    /// RFC 076: a `.json` file with non-alphabetical keys and pretty
    /// (non-minified) formatting is served byte-for-byte — comparing
    /// bytes, not parsed equality, since a parse-and-recompare would
    /// pass with the old minify-and-reorder behaviour still in place.
    #[tokio::test]
    async fn json_file_is_served_byte_identical_key_order_and_formatting_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let bytes: &[u8] = b"{\n  \"zebra\": 1,\n  \"apple\": 2\n}\n";
        let file_path = dir.path().join("data.json");
        std::fs::write(&file_path, bytes).unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let mut file_response = FileResponse::new(
            file_path.to_str().unwrap(),
            None,
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        );
        let response = file_response.file_content_response().await.unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(
            body.as_ref(),
            bytes,
            "a .json file must be served exactly as written — not \
             minified, not key-reordered"
        );
    }

    /// RFC 076 non-goal: `.json5` still converts (parses and
    /// reserialises) — JSON5 is not JSON, and converting it is the
    /// point. Pinned so the `.json`/`.json5` split above can't
    /// accidentally start treating them the same.
    #[tokio::test]
    async fn json5_file_still_converts_to_minified_json() {
        let dir = tempfile::tempdir().unwrap();
        // Trailing comma and unquoted-friendly spacing: valid JSON5,
        // invalid strict JSON — proves this path still goes through the
        // JSON5 parser rather than being served as raw bytes.
        let source = b"{\n  \"zebra\": 1,\n  \"apple\": 2,\n}\n";
        let file_path = dir.path().join("data.json5");
        std::fs::write(&file_path, source).unwrap();
        let confine_to = canonical_dir(dir.path().to_str().unwrap());

        let mut file_response = FileResponse::new(
            file_path.to_str().unwrap(),
            None,
            &HeaderMap::new(),
            confine_to.as_deref(),
            &[],
        );
        let response = file_response.file_content_response().await.unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        // Converted: minified, trailing comma gone. Key order is a
        // separate question (RFC 076's `preserve_order`) — this test
        // only pins that conversion still happens at all.
        assert_ne!(
            body.as_ref(),
            source,
            ".json5 must still be converted, not served raw"
        );
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["zebra"], 1);
        assert_eq!(parsed["apple"], 2);
    }
}
