use std::str::FromStr;

use hyper::{Method, StatusCode};

use crate::{
    constant::root_config_dir,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn match_http_method_1() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("POST").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method POST matched");
}

#[tokio::test]
async fn match_http_method_2() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/get", port)
        .with_http_method(&Method::from_str("GET").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method GET matched");
}

#[tokio::test]
async fn match_http_method_3() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/put", port)
        .with_http_method(&Method::from_str("PUT").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method PUT matched");
}

#[tokio::test]
async fn match_http_method_4() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/delete", port)
        .with_http_method(&Method::from_str("DELETE").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method DELETE matched");
}

#[tokio::test]
async fn not_match_http_method_1() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("GET").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_match_http_method_2() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("PUT").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn not_match_http_method_3() {
    let port = setup().await;
    let response = TestRequest::default("/http-method", port)
        .with_http_method(&Method::from_str("DELETE").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// RFC 082 acceptance: a real PATCH request matches a PATCH rule, asserted
/// on the HTTP response against a running server.
#[tokio::test]
async fn match_http_method_patch() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/patch", port)
        .with_http_method(&Method::PATCH)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method PATCH matched");
}

/// RFC 082: a PATCH request does not match a GET rule on the same path, and a
/// GET request does not match the PATCH rule.
#[tokio::test]
async fn patch_does_not_match_a_get_rule_and_get_does_not_match_patch() {
    let port = setup().await;

    let patch_to_get = TestRequest::default("/http-method/get", port)
        .with_http_method(&Method::PATCH)
        .send()
        .await;
    assert_eq!(patch_to_get.status(), StatusCode::NOT_FOUND);

    let get_to_patch = TestRequest::default("/http-method/patch", port)
        .with_http_method(&Method::GET)
        .send()
        .await;
    assert_eq!(get_to_patch.status(), StatusCode::NOT_FOUND);
}

/// RFC 082 § 4, pinned: the wire is case-insensitive, so a lowercase `patch`
/// on the wire still matches the PATCH rule.
#[tokio::test]
async fn lowercase_patch_on_the_wire_matches() {
    let port = setup().await;
    let response = TestRequest::default("/http-method/patch", port)
        .with_http_method(&Method::from_str("patch").unwrap())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "http-method PATCH matched");
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup =
        TestSetup::default_with_root_config_dir(root_config_dir::RULE_WHEN_REQUEST_HTTP_METHOD);
    test_setup.launch().await
}

/// Write a one-rule config whose rule names `method`, and return the error
/// `App::new` gives when it loads that config.
async fn config_load_error_for_method(method: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("rules.toml"),
        format!(
            "[[rules]]\nwhen.request.url_path = \"/x\"\nwhen.request.method = \"{method}\"\nrespond = {{ text = \"ok\" }}\n"
        ),
    )
    .expect("write rule set");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .expect("write root config");

    let mut env_args = apimock::EnvArgs::empty();
    env_args.config_file_path = Some(
        dir.path()
            .join("apimock.toml")
            .to_string_lossy()
            .into_owned(),
    );
    match apimock::App::new(&env_args, None, true).await {
        Ok(_) => panic!("config naming `{method}` should be refused at load"),
        Err(err) => err.to_string(),
    }
}

/// RFC 082 § 4, pinned: config spelling is case-sensitive, so a lowercase
/// `patch` in TOML is refused — exactly as `get` always was.
#[tokio::test]
async fn lowercase_patch_in_config_is_still_refused() {
    let err = config_load_error_for_method("patch").await;
    assert!(err.contains("unknown variant `patch`"), "{err}");
}

/// RFC 082 Amendment 1, quoted: OPTIONS is refused with the enumerated set and
/// its reason.
#[tokio::test]
async fn options_in_config_is_refused_with_the_set_and_its_reason() {
    let err = config_load_error_for_method("OPTIONS").await;
    assert!(
        err.contains("unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`\n  — OPTIONS is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule"),
        "{err}"
    );
}

/// RFC 082 Amendment 1, quoted: HEAD is refused with the enumerated set and its
/// reason.
#[tokio::test]
async fn head_in_config_is_refused_with_the_set_and_its_reason() {
    let err = config_load_error_for_method("HEAD").await;
    assert!(
        err.contains("unknown variant `HEAD`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`\n  — HEAD is not matchable yet: a rule could answer it with a response body, which HTTP forbids for HEAD"),
        "{err}"
    );
}

/// RFC 082 Amendment 1: a typo gets the enumerated set and no reason clause.
#[tokio::test]
async fn a_typo_in_config_is_refused_with_the_set_and_no_reason() {
    let err = config_load_error_for_method("GTE").await;
    assert!(
        err.contains(
            "unknown variant `GTE`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`"
        ),
        "{err}"
    );
    assert!(
        !err.contains("\n  —"),
        "a typo must carry no reason clause: {err}"
    );
}
