//! Verifies every `curl` call and expected response documented in
//! `crates/apimock/examples/*/README.md` (RFC 036). An example that
//! stops matching its README fails here.

#[path = "examples/common.rs"]
mod common;

#[path = "examples/default.rs"]
mod default;
#[path = "examples/match_headers_and_body.rs"]
mod match_headers_and_body;
#[path = "examples/scripting_with_middleware.rs"]
mod scripting_with_middleware;
#[path = "examples/secure_with_tls.rs"]
mod secure_with_tls;
#[path = "examples/serve_json_resources.rs"]
mod serve_json_resources;
#[path = "examples/simulate_slow_backend.rs"]
mod simulate_slow_backend;
#[path = "examples/status_codes_and_errors.rs"]
mod status_codes_and_errors;
#[path = "examples/validate_in_ci.rs"]
mod validate_in_ci;
#[path = "examples/vary_response_by_strategy.rs"]
mod vary_response_by_strategy;

#[path = "util.rs"]
pub mod util;
