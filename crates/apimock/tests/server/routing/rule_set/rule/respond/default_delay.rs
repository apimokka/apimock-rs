//! RFC 045 Defect 2: `[default].delay_response_milliseconds` must
//! actually delay when a rule doesn't set its own
//! `respond.delay_response_milliseconds`, and the per-rule value must
//! still override it when both are set.
//!
//! Measured symptom before this fix: 2000 ms configured, ~4 ms observed
//! (`RuleSet.default` was read only for the startup-log `Display` impl).
//! Thresholds below are generous relative to the 300 ms / 10 ms
//! configured values specifically to avoid CI-timing flakiness while
//! still being unambiguous about which behaviour occurred.

use std::time::Instant;

use hyper::StatusCode;

use crate::{
    constant::root_config_dir,
    util::{http::test_request::TestRequest, test_setup::TestSetup},
};

#[tokio::test]
async fn rule_set_default_delay_applies_when_rule_sets_none() {
    let port = setup().await;

    let start = Instant::now();
    let response = TestRequest::default("/respond-default-delay/inherits-default", port)
        .send()
        .await;
    let elapsed = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        elapsed.as_millis() >= 250,
        "expected the rule-set default (300ms) to apply; observed {:?}",
        elapsed
    );
}

#[tokio::test]
async fn per_rule_delay_overrides_rule_set_default() {
    let port = setup().await;

    let start = Instant::now();
    let response = TestRequest::default("/respond-default-delay/overrides-default", port)
        .send()
        .await;
    let elapsed = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        elapsed.as_millis() < 250,
        "expected the per-rule delay (10ms) to override the rule-set \
         default (300ms); observed {:?}, which looks like the default won",
        elapsed
    );
}

/// internal setup fn
async fn setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(root_config_dir::RULE_RESPOND);
    test_setup.launch().await
}
