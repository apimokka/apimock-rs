use hyper::HeaderMap;
use rhai::{AST, Dynamic, Engine, Map, Scope, serde::to_dynamic};
use serde_json::Value;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    error::{ServerError, ServerResult},
    middleware::middleware_response::MiddlewareResponse,
    response::confine::canonical_dir,
    types::BoxBody,
};

/// RFC 068 S-03: bounds on a middleware script that aren't the
/// operator-configurable `middleware_max_operations` — fixed,
/// generous ceilings on call depth and string/array/map growth so a
/// script's *shape* can't run away even within its operation budget.
/// Not configurable: unlike the operation count, there's no
/// legitimate mock-middleware reason to need deeper recursion or
/// bigger single values than this.
mod limits {
    /// Rhai's own default in release builds; set explicitly here so the
    /// bound doesn't silently change between debug and release builds.
    pub const MAX_CALL_LEVELS: usize = 64;
    /// Characters in a single Rhai string value.
    pub const MAX_STRING_SIZE: usize = 1_000_000;
    /// Elements in a single Rhai array value.
    pub const MAX_ARRAY_SIZE: usize = 100_000;
    /// Entries in a single Rhai object-map value.
    pub const MAX_MAP_SIZE: usize = 10_000;
}

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
    /// The middleware script's own directory, canonicalised once here
    /// at compile time. A file path the script returns is confined to
    /// this directory the same way a rule's `respond.file_path` is
    /// confined to `respond_dir` — see `MiddlewareResponse::file_response`
    /// (private to this crate; not linked here since rustdoc's public
    /// docs can't resolve a private item, and widening its visibility
    /// is a separate, deliberate decision — not this comment's to make).
    pub confine_to: Option<PathBuf>,
}

impl MiddlewareHandler {
    /// Compile a middleware script from disk into a reusable handler.
    ///
    /// Returns an `AppError` on either a missing file or a compile-time
    /// Rhai parse error. Callers treat both as startup-time failures —
    /// we deliberately do not try to recover by, say, skipping the offending
    /// script, because silently ignoring a misconfigured middleware would
    /// produce confusing request-time behaviour.
    ///
    /// # `max_operations` (RFC 068 S-03)
    ///
    /// `Engine::new()` used to set no limits at all — not
    /// `set_max_operations`, not `set_max_call_levels`, not the
    /// string/array size caps — so a non-terminating script (a `while
    /// true` an operator is actively developing is the ordinary case,
    /// not an attack) ran forever. `max_operations` bounds a script by
    /// work done; call-depth and string/array/map growth get fixed,
    /// generous ceilings from `limits` regardless of what's configured
    /// here, since there's no legitimate reason a mock middleware needs
    /// more of either. Neither is the whole fix on its own — see
    /// [`handle`](Self::handle)'s doc comment for the other half.
    pub fn new(file_path: &str, max_operations: u64) -> ServerResult<Self> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(ServerError::MiddlewareMissing {
                path: path.to_path_buf(),
            });
        }

        let mut engine = Engine::new();
        engine.set_max_operations(max_operations);
        engine.set_max_call_levels(limits::MAX_CALL_LEVELS);
        engine.set_max_string_size(limits::MAX_STRING_SIZE);
        engine.set_max_array_size(limits::MAX_ARRAY_SIZE);
        engine.set_max_map_size(limits::MAX_MAP_SIZE);

        // todo: watch source file change - `notify` crate ?
        let ast =
            engine
                .compile_file(file_path.into())
                .map_err(|e| ServerError::MiddlewareCompile {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                })?;

        let confine_to = path
            .parent()
            .and_then(|p| p.to_str())
            .and_then(canonical_dir);

        Ok(MiddlewareHandler {
            engine: Arc::new(engine),
            file_path: file_path.to_owned(),
            ast,
            confine_to,
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
    ///
    /// # Why evaluation runs in `spawn_blocking` (RFC 068 S-03)
    ///
    /// This used to call `eval_ast_with_scope` directly, synchronously,
    /// on the async runtime's own worker thread. `max_operations`
    /// (`Self::new`) bounds a runaway script by work done, but that is
    /// a value an operator can raise, and it does nothing for a script
    /// blocked on something that isn't a counted operation. Moving
    /// evaluation into `spawn_blocking` is what turns the failure mode
    /// from "one fewer tokio worker, permanently" into "one slow
    /// request" — the same reason file reads already go through
    /// `spawn_blocking` elsewhere in this crate. Rhai's `sync` feature
    /// is enabled, so `Engine`/`AST` are `Send` and this is possible
    /// without a dependency change.
    pub async fn handle(
        &self,
        request_url_path: &str,
        request_body_json_value: Option<&Value>,
        request_headers: &HeaderMap,
        cors_allow_credentials_origins: &[String],
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

        // middleware response — see this method's doc comment for why
        // this is a blocking task, not a direct call.
        let engine = Arc::clone(&self.engine);
        let ast = self.ast.clone();
        let eval_result = match tokio::task::spawn_blocking(move || {
            engine.eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
        })
        .await
        {
            Ok(result) => result,
            Err(join_err) => {
                log::warn!(
                    "middleware `{}`: evaluation task panicked or was cancelled: {}",
                    self.file_path,
                    join_err
                );
                return None;
            }
        };
        let rhai_response = match eval_result {
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
        let middleware_response = MiddlewareResponse::new(
            self.file_path.as_str(),
            request_headers,
            self.confine_to.as_deref(),
            cors_allow_credentials_origins,
        );

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
