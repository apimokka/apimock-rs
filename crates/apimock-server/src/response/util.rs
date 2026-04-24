use serde_json::{Map, Value};

use std::path::Path;

/// file extension string from file path
pub fn file_extension(file_path: &str) -> Option<String> {
    match Path::new(file_path)
        .extension()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .to_str()
    {
        Some(x) => Some(x.to_owned()),
        None => None,
    }
}

/// content type from text file extension
pub fn text_file_content_type(ext: impl AsRef<str>) -> String {
    let ret = match ext.as_ref() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "text/plain; charset=utf-8",
    };
    ret.to_owned()
}

/// json value with jsonpath as key
pub fn json_value_with_jsonpath_key(jsonpath_key: &str, value: Value) -> Value {
    let mut keys: Vec<&str> = jsonpath_key.split('.').collect();
    keys.reverse();

    let mut ret = value;

    for key in keys {
        let mut map = Map::new();
        map.insert(key.to_string(), ret);
        ret = Value::Object(map);
    }

    ret
}

/// content-type from file ext
pub fn binary_content_type(file_path: &str) -> String {
    let content_type = match file_extension(file_path).unwrap_or_default().as_str() {
        // - image
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",

        // - audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "weba" => "audio/webm",

        // video
        "mp4" => "video/mp4",
        "m4v" => "video/mp4",
        "mpeg" | "mpg" => "video/mpeg",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "ogv" => "video/ogg",

        // - doc
        "pdf" => "application/pdf",
        // - archive
        "zip" => "application/zip",
        // - (else)
        _ => "application/octet-stream",
    };

    content_type.to_owned()
}
