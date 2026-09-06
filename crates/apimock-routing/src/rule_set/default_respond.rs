use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DefaultRespond {
    pub delay_response_milliseconds: Option<u32>,
}

impl DefaultRespond {
    /// Intentionally trivial — always `true` (RFC 079 F-10/M-04e).
    ///
    /// `delay_response_milliseconds` is a plain `Option<u32>` — `serde`
    /// already rejects a value that doesn't fit at deserialise time,
    /// and any `u32` that does fit is a meaningful delay in
    /// milliseconds, so there is no further constraint to check here.
    /// Kept (not removed), same reasoning as `RuleSet::validate`'s own
    /// doc comment: called from `ServiceConfig::validate`'s loop today,
    /// and the natural place a future field on this struct would want
    /// real validation added, rather than a new method wired in from
    /// scratch. See RFC 079 § 2 for the keep-and-document decision this
    /// reflects.
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
