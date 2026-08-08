//! pre-defined variables are available:
//! - url_path: request url path
//! - body: request body json value defined only when exists
//!
//! (ref) The Rhai Book: https://rhai.rs/book/language/statements.html
//!
//! Middleware runs before the rule sets, once per request. A returned
//! value serves the response directly; returning nothing (or falling
//! off the end, as the last line here does) declines and falls
//! through to `rule_sets`.
//!
//! A returned value is interpreted by shape:
//! - a plain string      -> serve that file path
//! - #{ "file_path": _ } -> same, spelled out
//! - #{ "json": _ }      -> a literal JSON response body
//! - #{ "text": _ }      -> a plain text response body

let profile_file = "data/profile.json";

// Three ways to answer the same question, so all three response
// shapes are demonstrated on real, working routes.
if url_path == "/profile/file-path" {
    return #{ "file_path": profile_file };
}
else if url_path == "/profile/json" {
    return #{ "json": "{\"plan\": \"pro\", \"source\": \"middleware-json\"}" };
}
else if url_path == "/profile/text" {
    return #{ "text": "plan: pro (middleware-text)" };
}
else if url_path == "/profile" {
    // A bare string return is shorthand for #{ "file_path": ... }.
    return profile_file;
}

// Middleware can also inspect the request body, when there is one.
if is_def_var("body") {
    switch (url_path) {
        "/orders" if body.priority == "rush" => {
            return #{ "text": "expedited: this order jumps the queue" };
        },
        _ => ()
    }
}

// Anything else falls through to apimock-rule-set.toml.
return;
