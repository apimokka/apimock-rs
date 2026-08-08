//! Verifies `crates/apimock/examples/simulate-slow-backend/README.md`.

use std::time::{Duration, Instant};

use crate::{
    common::example_test_setup,
    util::http::{test_request::TestRequest, test_response::response_body_str},
};

async fn setup() -> u16 {
    example_test_setup("simulate-slow-backend").launch().await
}

#[tokio::test]
async fn fast_responds_immediately() {
    let port = setup().await;
    let started = Instant::now();
    let response = TestRequest::default("/fast", port).send().await;
    assert_eq!(response_body_str(response).await, "instant response");
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "expected /fast to respond well under 500ms, took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn slow_delays_about_800ms() {
    let port = setup().await;
    let started = Instant::now();
    let response = TestRequest::default("/slow", port).send().await;
    assert_eq!(response_body_str(response).await, "eventually...");
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(750),
        "expected /slow to take at least ~800ms, took {elapsed:?}"
    );
}

#[tokio::test]
async fn very_slow_delays_about_2000ms() {
    let port = setup().await;
    let started = Instant::now();
    let response = TestRequest::default("/very-slow", port).send().await;
    assert_eq!(response_body_str(response).await, "much later...");
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(1950),
        "expected /very-slow to take at least ~2000ms, took {elapsed:?}"
    );
}
