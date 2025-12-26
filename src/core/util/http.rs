use hyper::{
    header::{HeaderValue, CONTENT_TYPE},
    HeaderMap,
};
use tokio::time;

use std::time::Duration;

/// check if content-type is application/json
/// supporting case when "application/json; charset=utf-8"
pub fn content_type_is_application_json(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    let content_type = match headers.get(CONTENT_TYPE) {
        Some(x) => x,
        None => {
            return None;
        }
    };

    let ret = if let Ok(content_type) = content_type.to_str() {
        content_type
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("application/json")
    } else {
        false
    };
    Some(ret)
}

/// normalize url path
pub fn normalize_url_path(url_path: &str, url_path_prefix: Option<&str>) -> String {
    let url_path_prefix = match url_path_prefix {
        Some(prefix) if !prefix.is_empty() => prefix.strip_suffix("/").unwrap_or_else(|| prefix),
        _ => "",
    };

    let url_path = url_path.strip_prefix("/").unwrap_or_else(|| url_path);

    let merged = format!("{}/{}", url_path_prefix, url_path);

    let mut ret: &str = merged.as_str();
    ret = ret.strip_suffix("/").unwrap_or_else(|| ret);
    ret = ret.strip_prefix("/").unwrap_or_else(|| ret);
    format!("/{}", ret)
}

/// sleep
pub async fn delay_response(milliseconds: u32) {
    time::sleep(Duration::from_millis(milliseconds.into())).await
}
