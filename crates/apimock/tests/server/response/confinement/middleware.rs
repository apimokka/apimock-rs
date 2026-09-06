//! RFC 063, Finding 2's Rhai counterpart — a middleware script returning
//! a `file_path` outside its own directory. Same fix, same base
//! (the middleware script's own directory), applied at
//! `middleware/middleware_response.rs`'s `FileResponse` construction
//! instead of `respond_response.rs`'s.

use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{
            test_request::TestRequest,
            test_response::{platform_eol, response_body_str},
        },
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn a_middleware_returned_path_resolving_outside_its_dir_is_refused() {
    let port = setup().await;

    let response = TestRequest::default("/outside", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// A middleware-returned path that legitimately stays inside the
/// script's own directory is unaffected.
///
/// RFC 076: the expected body is `script/inside.json`'s own bytes (one
/// space after the colon, trailing newline) — a `.json` `file_path` is
/// now served byte-for-byte, not minified. **Updated because the bytes
/// are now correct.**
#[tokio::test]
async fn a_middleware_returned_path_resolving_inside_its_dir_still_serves() {
    let port = setup().await;

    let response = TestRequest::default("/inside", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), platform_eol("{\"key\": \"inside\"}\n"));
}

async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::CONFINEMENT_MIDDLEWARE);
    test_setup.launch().await
}
