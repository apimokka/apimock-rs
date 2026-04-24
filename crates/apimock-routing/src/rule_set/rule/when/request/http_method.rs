use hyper::Method;
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    /// is match
    pub fn is_match(&self, http_method: &Method) -> bool {
        self.as_str().to_lowercase() == http_method.as_str().to_lowercase()
    }

    /// as str
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP Method is {}", self.as_str())
    }
}
