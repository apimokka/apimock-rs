use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn matches_prefix_url_path_prefix_1() {
    let port = setup().await;

    let response = TestRequest::default("/prefix/equal", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str, "url path prefix if");
}

/// RFC 075 F-02: `/prefix` (normalised from the authored `/prefix/`)
/// must not match `/prefixyz` — the old `starts_with` comparison did,
/// making this rule set claim a sibling path it was never scoped to.
/// `/prefixyz` matches no rule in this rule set and no other rule set
/// exists in this config, so it falls through to the dyn-route fallback
/// and 404s (there is no real file named `prefixyz` to serve).
#[tokio::test]
async fn does_not_match_a_sibling_path_that_merely_shares_the_prefix_as_a_string() {
    let port = setup().await;

    let response = TestRequest::default("/prefixyz", port).send().await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// The companion positive case, named for what it proves rather than
/// merely "the prefix matches": `/prefix` bare (no trailing segment)
/// still reaches the rule set at all, even though it isn't the
/// `/prefix/equal` shape the other two tests exercise. It falls to the
/// rule set's own `not_equal "equal"` rule (the request's relative
/// `url_path` is empty, not `"equal"`) rather than being scoped out
/// before ever reaching rule matching.
#[tokio::test]
async fn matches_the_bare_prefix_itself() {
    let port = setup().await;

    let response = TestRequest::default("/prefix", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    assert_eq!(body_str, "url path prefix else");
}

#[tokio::test]
async fn matches_prefix_url_path_prefix_2() {
    let port = setup().await;

    let response = TestRequest::default("/prefix/equal2", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str, "url path prefix else");
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(root_config_dir::RULE_SET_PREFIX);
    test_setup.launch().await
}
