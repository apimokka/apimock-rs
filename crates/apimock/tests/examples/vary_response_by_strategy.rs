//! Verifies `crates/apimock/examples/vary-response-by-strategy/README.md`.

use std::collections::HashSet;

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("vary-response-by-strategy")
        .launch()
        .await
}

#[tokio::test]
async fn priority_always_picks_the_higher_priority_rule() {
    let port = setup().await;
    for _ in 0..5 {
        let response = TestRequest::default("/priority", port).send().await;
        assert_eq!(
            response_body_str(response).await,
            "special response (higher priority)"
        );
    }
}

#[tokio::test]
async fn round_robin_cycles_in_order() {
    let port = setup().await;
    let mut seen = Vec::with_capacity(6);
    for _ in 0..6 {
        let response = TestRequest::default("/round-robin", port).send().await;
        seen.push(response_body_str(response).await);
    }
    assert_eq!(
        seen,
        vec![
            "server-a", "server-b", "server-c", "server-a", "server-b", "server-c"
        ]
    );
}

/// Unseeded, so the exact sequence isn't fixed - what's verified is
/// the property the README documents: both weighted variants appear
/// over enough requests. 40 requests at a real 3:1 weighting makes
/// "variant-b never once appears" astronomically unlikely (~1e-5)
/// without being slow.
#[tokio::test]
async fn weighted_random_produces_both_variants() {
    let port = setup().await;
    let mut seen: HashSet<String> = HashSet::new();
    for _ in 0..40 {
        let response = TestRequest::default("/weighted", port).send().await;
        seen.insert(response_body_str(response).await);
    }
    assert!(
        seen.contains("variant-a"),
        "variant-a never appeared in 40 requests"
    );
    assert!(
        seen.contains("variant-b"),
        "variant-b never appeared in 40 requests"
    );
    assert_eq!(seen.len(), 2, "unexpected response(s): {seen:?}");
}
