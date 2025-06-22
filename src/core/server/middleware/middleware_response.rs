use hyper::HeaderMap;

use std::path::Path;

use crate::core::server::{
    response::{self, error_response::internal_server_error_response, file_response::FileResponse},
    types::BoxBody,
};

pub struct MiddlewareResponse {
    pub file_path: String,
    pub request_headers: HeaderMap,
}

impl MiddlewareResponse {
    pub fn new(file_path: &str, request_headers: &HeaderMap) -> Self {
        MiddlewareResponse {
            file_path: file_path.to_owned(),
            request_headers: request_headers.to_owned(),
        }
    }

    /// file content response specified in rhai script
    pub async fn file_response(
        &self,
        rhai_response_file_path: &str,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        let file_path = if Path::new(rhai_response_file_path).is_absolute() {
            rhai_response_file_path.to_owned()
        } else {
            let middleware_dir_path = Path::new(self.file_path.as_str()).parent();

            let joined_file_path = match middleware_dir_path {
                Some(x) => x.join(rhai_response_file_path),
                None => {
                    return Some(internal_server_error_response(
                        &format!(
                            "failed to get middleware parent dir: {}",
                            self.file_path.as_str(),
                        ),
                        &self.request_headers,
                    ))
                }
            };

            match joined_file_path.to_str() {
                Some(x) => x.to_owned(),
                None => {
                    let msg = format!(
                        "middleware response file path is invalid: {}/{}",
                        self.file_path.as_str(),
                        rhai_response_file_path
                    );
                    return Some(internal_server_error_response(
                        msg.as_str(),
                        &self.request_headers,
                    ));
                }
            }
        };

        Some(
            FileResponse::new(file_path.as_str(), None, &self.request_headers)
                .file_content_response()
                .await,
        )
    }

    /// json response created by rhai script
    pub fn json_response(
        &self,
        json_str: &str,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        Some(response::json_response::json_response(
            json_str,
            None,
            &self.request_headers,
            self.file_path.as_str(),
        ))
    }

    /// text response created by rhai script
    pub fn text_response(
        &self,
        s: &str,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        Some(response::text_response::text_response(
            s,
            None,
            None,
            &self.request_headers,
        ))
    }
}
