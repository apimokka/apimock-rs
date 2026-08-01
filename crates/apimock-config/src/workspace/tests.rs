//! Workspace tests, split by concern into submodules.
//!
//! # Module layout
//!
//! | Module | Coverage |
//! |---|---|
//! | `workspace_core` | load, snapshot, apply, validate |
//! | `save` | save, round-trip, atomic write, diff tracking |
//! | `headers_body` | header/body condition preservation (5.5.0) |
//! | `url_path_op` | RFC 013 — url_path / url_path_op validation |
//! | `file_tree_filter` | RFC 012 — config-driven FileTreeFilter |
//! | `conditions` | RFC 016 — per-condition NodeId (Add/Remove Header/Body) |

mod common;

mod workspace_core;
mod save;
mod headers_body;
mod url_path_op;
mod file_tree_filter;
mod conditions;
mod rfc_024_025;
mod rfc_027_029;
