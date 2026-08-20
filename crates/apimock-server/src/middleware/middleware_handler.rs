use hyper::HeaderMap;
use rhai::{AST, Dynamic, Engine, Map, Scope, serde::to_dynamic};
use serde_json::Value;

use std::{path::Path, sync::Arc};

use crate::{
    error::{ServerError, ServerResult},
    middleware::middleware_response::MiddlewareResponse,
    types::BoxBody,
};

/// Handler for a single Rhai middleware script.
///
/// # Why the AST is compiled once at startup
///
/// Rhai offers both "compile on every evaluation" and "compile once, re-run
/// the AST" modes. Middleware is invoked on the hot path (every request),
/// so we keep the compiled `AST` alongside the `Engine` and only evaluate
/// at request time. This trades a small amount of memory for a large
/// throughput win and keeps parse errors as startup failures instead of
/// per-request 500s.
///
/// The `Engine` is wrapped in `Arc` so that `MiddlewareHandler` can be
/// cloned cheaply into each request task without deep-cloning the
/// interpreter state.
#[derive(Clone)]
#[non_exhaustive]
pub struct MiddlewareHandler {
    pub engine: Arc<Engine>,
    pub file_path: String,
    pub ast: AST,
}

impl MiddlewareHandler {
    /// Compile a middleware script from disk into a reusable handler.
    ///
    /// Returns an `AppError` on either a missing file or a compile-time
    /// Rhai parse error. Callers treat both as startup-time failures —
    /// we deliberately do not try to recover by, say, skipping the offending
    /// script, because silently ignoring a misconfigured middleware would
    /// produce confusing request-time behaviour.
    pub fn new(file_path: &str) -> ServerResult<Self> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(ServerError::MiddlewareMissing {
                path: path.to_path_buf(),
            });
        }

        let engine = Engine::new();
        // todo: watch source file change - `notify` crate ?
        let ast =
            engine
                .compile_file(file_path.into())
                .map_err(|e| ServerError::MiddlewareCompile {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                })?;

        Ok(MiddlewareHandler {
            engine: Arc::new(engine),
            file_path: file_path.to_owned(),
            ast,
        })
    }

    /// Evaluate the middleware for one request.
    ///
    /// Returns:
    /// - `Some(Ok(response))` — the script decided to handle the request
    ///   and produced a response.
    /// - `Some(Err(_))` — the script tried to handle the request but the
    ///   response could not be built (e.g. invalid header value).
    /// - `None` — the script returned a value that is neither a string nor
    ///   a map, which is the convention for "let the next layer handle it".
    ///
    /// # Why errors here are logged and converted, not propagated
    ///
    /// A Rhai runtime error during per-request evaluation is a script bug,
    /// not a startup config bug. Turning it into an `AppError` would
    /// force the whole process down, which is the opposite of what an
    /// HTTP server should do. We instead log and fall through to the
    /// next handler, producing an HTTP response rather than aborting.
    pub async fn handle(
        &self,
        request_url_path: &str,
        request_body_json_value: Option<&Value>,
        request_headers: &HeaderMap,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        let mut scope = Scope::new();
        scope.push("url_path", request_url_path.to_owned());
        if let Some(request_body_json_value) = request_body_json_value {
            match to_dynamic(request_body_json_value) {
                Ok(body_dynamic) => {
                    scope.push("body", body_dynamic);
                }
                Err(err) => {
                    log::warn!(
                        "middleware `{}`: failed to convert request body to Rhai Dynamic: {}",
                        self.file_path,
                        err
                    );
                    return None;
                }
            }
        }

        // middleware response
        let rhai_response = match self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &self.ast)
        {
            Ok(v) => v,
            Err(err) => {
                log::warn!(
                    "middleware `{}`: script evaluation failed: {}",
                    self.file_path,
                    err
                );
                return None;
            }
        };

        if !rhai_response.is_string() && !rhai_response.is_map() {
            return None;
        }
        let middleware_response = MiddlewareResponse::new(self.file_path.as_str(), request_headers);

        // string is treated as file path
        if let Some(x) = rhai_response.clone().try_cast::<String>() {
            middleware_response.file_response(x.as_str()).await
        // map may be as either of: file path, json response string, text response string
        } else if let Some(x) = rhai_response.try_cast::<Map>() {
            if let Some(x) = x
                .get("file_path")
                .and_then(|x| x.clone().try_cast::<String>())
            {
                middleware_response.file_response(x.as_str()).await
            } else if let Some(x) = x.get("json").and_then(|x| x.clone().try_cast::<String>()) {
                middleware_response.json_response(x.as_str())
            } else if let Some(x) = x.get("text").and_then(|x| x.clone().try_cast::<String>()) {
                middleware_response.text_response(x.as_str())
            } else {
                None
            }
        } else {
            None
        }
    }
}
