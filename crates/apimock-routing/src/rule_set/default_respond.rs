use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct DefaultRespond {
    pub delay_response_milliseconds: Option<u32>,
}

impl DefaultRespond {
    /// validate
    pub fn validate(&self) -> bool {
        true
    }
}

impl std::fmt::Display for DefaultRespond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(delay_response_milliseconds) = self.delay_response_milliseconds.as_ref() {
            let _ = write!(
                f,
                "[delay_response_milliseconds] {}",
                delay_response_milliseconds
            );
        }
        Ok(())
    }
}
