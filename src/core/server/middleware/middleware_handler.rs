use hyper::HeaderMap;
use rhai::{serde::to_dynamic, Dynamic, Engine, Map, Scope, AST};
use serde_json::Value;

use std::{path::Path, sync::Arc};

use crate::core::server::{middleware::middleware_response::MiddlewareResponse, types::BoxBody};

#[derive(Clone)]
pub struct MiddlewareHandler {
    pub engine: Arc<Engine>,
    pub file_path: String,
    pub ast: AST,
}

impl MiddlewareHandler {
    pub fn new(file_path: &str) -> Result<Self, String> {
        if !Path::new(file_path).exists() {
            return Err(format!("middleware file path must be wrong: {}", file_path));
        }

        let engine = Engine::new();
        // todo: watch source file change - `notify` crate ?
        let ast = engine.compile_file(file_path.into()).expect(
            format!(
                "failed to compile middleware file to get ast: {}",
                file_path
            )
            .as_str(),
        );

        let middleware = MiddlewareHandler {
            engine: Arc::new(engine),
            file_path: file_path.to_owned(),
            ast,
        };

        Ok(middleware)
    }

    /// return response if middleware returns valid value
    pub async fn handle(
        &self,
        request_url_path: &str,
        request_body_json_value: Option<&Value>,
        request_headers: &HeaderMap,
    ) -> Option<Result<hyper::Response<BoxBody>, hyper::http::Error>> {
        let mut scope = Scope::new();
        scope.push("url_path", request_url_path.to_owned());
        if let Some(request_body_json_value) = request_body_json_value {
            scope.push(
                "body",
                to_dynamic(request_body_json_value)
                    .expect("failed to request body to dynamic for middleware"),
            );
        }

        // middleware response
        let rhai_response = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &self.ast)
            .expect("failed to evaluate middleware");

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
