use crate::core::server::{routing::rule_set::prefix::Prefix, util::normalize_url_path};

/// url path merged with prefix and then normalized
pub fn url_path_with_prefix(url_path: &str, rule_set_prefix: Option<&Prefix>) -> String {
    let url_path_prefix = if let Some(prefix) = rule_set_prefix {
        if let Some(url_path_prefix) = prefix.url_path_prefix.as_ref() {
            Some(url_path_prefix.as_str())
        } else {
            None
        }
    } else {
        None
    };

    normalize_url_path(url_path, url_path_prefix)
}
