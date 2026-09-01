//! Turn a matched `Respond` declaration into an HTTP response.
//!
//! # Why this is a server-side free function and not a `Respond` method
//!
//! Pre-5.0, `Respond::response(...)` lived on the type itself. That
//! method built a `hyper::Response<BoxBody>` and touched the server's
//! file-response / text-response / status-response helpers. To keep
//! `apimock-routing` free of hyper-body construction (so a future GUI
//! can depend on it cheaply), that work moved here.

use apimock_routing::{ParsedRequest, Respond};
use console::style;
use std::path::Path;

use crate::{
    http_util::delay_response,
    respond_util::full_file_path,
    response::{
        error_response::internal_server_error_response,
        file_response::FileResponse,
        json_response::json_response,
        status_code_response::{status_code_response, status_code_response_with_message},
        text_response::text_response,
    },
    types::BoxBody,
};

/// Produce the HTTP response for a matched `Respond` declaration.
///
/// # Why the branches are ordered file → text → json → status → error
///
/// The fields are mutually specialised (RFC 065: `file_path`, `text`
/// and `json` are the three body sources, `Respond::validate` rejects
/// more than one being set):
/// - `file_path` serves a file (possibly with CSV→JSON conversion).
/// - `text` + `status` yields a custom-status text response.
/// - `text` alone yields a plain 200 text response.
/// - `json` (+ an optional `status`) yields an `application/json`
///   response, parsed with the same JSON5 parser `Respond::validate`
///   already checked it against at load time.
/// - `status` alone yields an empty body with that status.
///
/// `Respond::validate` rejects nonsensical combinations at startup, so
/// hitting the final `Err` branch means something slipped past
/// validation — a real bug, not user input.
///
/// `rule_set_default_delay_ms` is the matched rule set's
/// `[default].delay_response_milliseconds`, if any (RFC 045 Defect 2).
/// The per-rule `respond.delay_response_milliseconds` always overrides
/// it when both are set; the rule-set value only applies when the rule
/// itself is silent.
pub async fn respond_response(
    respond: &Respond,
    dir_prefix: &str,
    parsed_request: &ParsedRequest,
    rule_set_default_delay_ms: Option<u32>,
    confine_to: Option<&Path>,
    cors_allow_credentials_origins: &[String],
) -> Result<hyper::Response<BoxBody>, hyper::http::Error> {
    if let Some(delay_ms) = respond
        .delay_response_milliseconds
        .or(rule_set_default_delay_ms)
    {
        delay_response(delay_ms).await;
    }

    let request_headers = &parsed_request.component_parts.headers;

    // file_path → file/CSV/JSON response
    if let Some(file_path) = respond.file_path.as_ref() {
        let Some(full_file_path) = full_file_path(file_path.as_str(), dir_prefix) else {
            log::error!(
                "{}:\n{} (prefix = {})",
                style("file not found").red(),
                file_path,
                dir_prefix,
            );
            return internal_server_error_response(
                "failed to get response file",
                request_headers,
                cors_allow_credentials_origins,
            );
        };

        // dir_prefix is used only for the file-not-found message above;
        // the actual read happens against the resolved full_file_path.
        let _ = Path::new(dir_prefix);

        return FileResponse::new_with_csv_records_jsonpath(
            full_file_path.as_str(),
            respond.headers.as_ref(),
            respond.csv_records_key.clone(),
            request_headers,
            confine_to,
            cors_allow_credentials_origins,
        )
        .file_content_response()
        .await;
    }

    if let Some(text) = respond.text.as_ref() {
        return match respond.status_code.as_ref() {
            Some(status_code) => status_code_response_with_message(
                status_code,
                text.as_str(),
                respond.headers.as_ref(),
                request_headers,
                cors_allow_credentials_origins,
            ),
            None => text_response(
                text.as_str(),
                None,
                respond.headers.as_ref(),
                request_headers,
                cors_allow_credentials_origins,
            ),
        };
    }

    if let Some(json_str) = respond.json.as_ref() {
        return json_response(
            json_str.as_str(),
            respond.status_code.as_ref(),
            respond.headers.as_ref(),
            request_headers,
            None,
            cors_allow_credentials_origins,
        );
    }

    if let Some(status_code) = respond.status_code.as_ref() {
        return status_code_response(
            status_code,
            respond.headers.as_ref(),
            request_headers,
            cors_allow_credentials_origins,
        );
    }

    internal_server_error_response(
        "invalid respond def",
        request_headers,
        cors_allow_credentials_origins,
    )
}
