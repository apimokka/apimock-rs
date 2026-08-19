//! Routing-layer microbenchmarks.
//!
//! # What this measures
//!
//! `RuleSet::find_matched` — the pure-CPU portion of request routing.
//! No I/O, no async runtime, no HTTP parsing. This isolates the cost of:
//!
//! - iterating the rule list,
//! - evaluating `url_path` / `method` / `headers` / `body.json` conditions,
//! - short-circuiting on the first match (current `FirstMatch` strategy).
//!
//! # Why it's the most honest CPU benchmark
//!
//! HTTP-level benches mix routing cost with tokio scheduling, syscalls,
//! and reqwest's client pipeline. To answer "did my rule-matching change
//! get faster or slower?" those other costs are noise. Running the
//! matcher in-process, in a single thread, against an in-memory
//! `ParsedRequest` gives criterion's statistical machinery a clean signal.
//!
//! # Axes covered
//!
//! We parametrize over rule-set size (1, 10, 100 rules). Real operator
//! configs sit somewhere in that range; synthetic 10k-rule decks would
//! just measure iterator overhead and wouldn't reflect anything real.

use std::hint::black_box;

use apimock_routing::ParsedRequest;
use apimock_routing::RuleSet;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hyper::Request;

/// Build a `RuleSet` on disk and load it, so that this bench exercises
/// the real `RuleSet::new` code path (prefix normalization, derived
/// fields, etc.) rather than a hand-built shape that could drift from
/// production.
///
/// Writes a temp file per invocation; caller cleans it up.
fn build_rule_set(rule_count: usize) -> (tempfile::TempDir, RuleSet) {
    let dir = tempfile::tempdir().expect("tempdir");
    let rule_set_path = dir.path().join("rules.toml");

    // A rule-set TOML with `rule_count` simple url_path rules plus a
    // wildcard default at the end. We mix a couple of ops so the matcher
    // exercises more than one `RuleOp` branch.
    let mut toml = String::new();
    toml.push_str("[prefix]\nurl_path = \"/api\"\n\n");
    for i in 0..rule_count {
        match i % 3 {
            0 => toml.push_str(&format!(
                "[[rules]]\nwhen.request.url_path = \"/v1/users/{}\"\nrespond = {{ text = \"user-{}\" }}\n\n",
                i, i,
            )),
            1 => toml.push_str(&format!(
                "[[rules]]\nwhen.request.url_path = {{ value = \"/v1/orders/{}\", op = \"starts_with\" }}\nrespond = {{ text = \"order-{}\" }}\n\n",
                i, i,
            )),
            _ => toml.push_str(&format!(
                "[[rules]]\nwhen.request.url_path = {{ value = \"search-{}\", op = \"contains\" }}\nrespond = {{ text = \"hit-{}\" }}\n\n",
                i, i,
            )),
        }
    }
    // Fallback rule — catches any request via a wildcard.
    toml.push_str(
        "[[rules]]\nwhen.request.url_path = { value = \"*\", op = \"wild_card\" }\nrespond = { status = 404 }\n",
    );
    std::fs::write(&rule_set_path, toml).expect("write rule set");

    let rule_set = RuleSet::new(
        rule_set_path.to_str().expect("utf-8 path"),
        dir.path().to_str().expect("utf-8 path"),
        0,
    )
    .expect("build rule set");

    (dir, rule_set)
}

/// Hand-build a `ParsedRequest` without going through hyper::Request.
///
/// `Parts` exposes a `Default` only via `Request::default()`, so we
/// allocate a trivial request, tear it open, and keep the parts. The
/// body on this sentinel is empty, which matches what the matcher sees
/// for GET requests anyway.
fn parsed_request_for(url_path: &str) -> ParsedRequest {
    let req = Request::builder()
        .method("GET")
        .uri(url_path)
        .body(())
        .expect("build request");
    let (component_parts, _) = req.into_parts();
    ParsedRequest::new(url_path.to_owned(), component_parts)
}

fn bench_find_matched(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_matched");

    // Throughput = 1 match attempt per iteration. This makes the report
    // print "elements/second" which is what operators actually care about
    // when comparing rule-set sizes.
    group.throughput(Throughput::Elements(1));

    for &rule_count in &[1usize, 10, 100] {
        let (_guard, rule_set) = build_rule_set(rule_count);

        // --- First-rule hit: pay minimum matcher cost, for the
        //     best-case in a first_match strategy.
        let first_hit = parsed_request_for("/api/v1/users/0");
        group.bench_with_input(
            BenchmarkId::new("first_rule_hit", rule_count),
            &rule_count,
            |b, _| {
                b.iter(|| {
                    let m = rule_set.find_matched(black_box(&first_hit), None, 0);
                    black_box(m)
                });
            },
        );

        // --- Last-rule hit: pay near-full iteration cost. This is the
        //     more useful measure for operators whose rules get added
        //     to the end of the file over time.
        let last_idx = rule_count.saturating_sub(1);
        let last_path = match last_idx % 3 {
            0 => format!("/api/v1/users/{}", last_idx),
            1 => format!("/api/v1/orders/{}/detail", last_idx),
            _ => format!("/api/v1/search-{}-results", last_idx),
        };
        let last_hit = parsed_request_for(&last_path);
        group.bench_with_input(
            BenchmarkId::new("last_rule_hit", rule_count),
            &rule_count,
            |b, _| {
                b.iter(|| {
                    let m = rule_set.find_matched(black_box(&last_hit), None, 0);
                    black_box(m)
                });
            },
        );

        // --- No-match: walks every rule, returns None. Falls through
        //     to the final wildcard rule in our synthetic set, so this
        //     still returns Some(...) — but via the full iteration.
        let no_match = parsed_request_for("/api/totally/unknown/path");
        group.bench_with_input(
            BenchmarkId::new("miss_all_specific_rules", rule_count),
            &rule_count,
            |b, _| {
                b.iter(|| {
                    let m = rule_set.find_matched(black_box(&no_match), None, 0);
                    black_box(m)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_find_matched);
criterion_main!(benches);
