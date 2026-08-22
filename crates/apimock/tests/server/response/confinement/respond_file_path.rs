//! RFC 063, Finding 2 — a rule's `respond.file_path` resolving outside
//! its rule set's `respond_dir`. Not request-reachable (the value is
//! operator-authored), but confined the same way.

use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn a_file_path_resolving_outside_respond_dir_is_refused() {
    let port = setup().await;

    let response = TestRequest::default("/outside", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// A rule's `file_path` that legitimately stays inside `respond_dir`
/// is unaffected.
#[tokio::test]
async fn a_file_path_resolving_inside_respond_dir_still_serves() {
    let port = setup().await;

    let response = TestRequest::default("/inside", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"key\":\"inside\"}");
}

async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::CONFINEMENT_RESPOND_FILE_PATH);
    test_setup.launch().await
}
