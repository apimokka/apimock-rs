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
        if self.delay_response_milliseconds.is_some() {
            let _ = write!(
                f,
                "[delay_response_milliseconds] {}",
                self.delay_response_milliseconds.as_ref().unwrap()
            );
        }
        Ok(())
    }
}
