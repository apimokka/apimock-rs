//! pre-defined variables are available:
//! - url_path: request url path
//! - body: request body json value defined only when exists
//!
//! (ref) The Rhai Book: https://rhai.rs/book/language/statements.html
//!
//! Runs before the rule sets: a returned value serves the response
//! directly, and anything else falls through to `rule_sets`. For a
//! fuller demonstration - all three return shapes, plus a body-driven
//! branch - see crates/apimock/examples/scripting-with-middleware/.

if url_path == "/hello" {
    return #{ "text": "Hello from middleware!" };
}

return;
