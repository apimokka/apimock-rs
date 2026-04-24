//! Routing-layer utilities.
//!
//! These live in the routing crate because they are used by routing
//! primitives (rule-op matching, JSON-body predicates). They are kept
//! `pub` so other crates can reuse them without re-implementing — for
//! instance the server crate can call `normalize_url_path` when
//! preparing a ParsedRequest.

pub mod glob;
pub mod http;
pub mod json;
