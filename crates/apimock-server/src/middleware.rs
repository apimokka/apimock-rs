//! Compiled Rhai middlewares.
//!
//! # Why this lives here and not in `apimock-config`
//!
//! A `MiddlewareHandler` owns a compiled Rhai `AST` and returns
//! `hyper::Response` values when it handles a request. Both of those
//! are runtime concerns that must not leak into the config crate (the
//! config crate must remain serde/TOML-centric so stage-2 GUI editing
//! can reason about it without linking against a scripting engine).

use std::path::Path;

use crate::error::{ServerError, ServerResult};

pub mod middleware_handler;
mod middleware_response;

pub use middleware_handler::MiddlewareHandler;

/// An ordered list of compiled middleware handlers.
///
/// Compilation happens once at server startup (see [`compile`]) so the
/// per-request path is free of filesystem reads and Rhai parsing cost.
#[derive(Clone, Default)]
pub struct LoadedMiddlewares {
    handlers: Vec<MiddlewareHandler>,
}

impl LoadedMiddlewares {
    /// Compile every Rhai source file listed in `middleware_file_paths`.
    ///
    /// Paths are interpreted relative to `relative_dir_path` — the same
    /// convention used by `Config::new` for rule-set paths.
    pub fn compile(
        middleware_file_paths: &[String],
        relative_dir_path: &str,
    ) -> ServerResult<Self> {
        let mut handlers = Vec::with_capacity(middleware_file_paths.len());
        for (idx, relative_path) in middleware_file_paths.iter().enumerate() {
            let joined = Path::new(relative_dir_path).join(relative_path);
            let path_str = joined.to_str().ok_or_else(|| {
                ServerError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "middleware #{} path contains non-UTF-8 bytes: {}",
                        idx + 1,
                        joined.to_string_lossy(),
                    ),
                ))
            })?;
            handlers.push(MiddlewareHandler::new(path_str)?);
        }
        Ok(Self { handlers })
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MiddlewareHandler> {
        self.handlers.iter()
    }
}
