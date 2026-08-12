# RFC 046 — Test harness: port race and server readiness

**Status.** Proposed — awaiting owner approval.
**Tracks.** Pipeline trust. The integration test harness picks a port it
does not hold and waits a fixed interval instead of waiting for the
server, so tests fail intermittently for reasons unrelated to what they
test — and one of the gates this flakiness can fail is the release gate.
**Touches.** `crates/apimock/tests/util/test_setup.rs`, and whatever the
server exposes about its bound address. **No crate source change is
expected**, but see Goal 3 — if the server cannot report the address it
actually bound, that is a small library-side addition.

## Summary

`TestSetup::launch` picks a port by binding it, dropping the listener,
and returning the number; the server then binds it some hundreds of
milliseconds later. Between those two moments any other test may take the
port. Separately, readiness is a fixed `sleep(400ms)` rather than a check
that the server is listening, and the spawned task's result — including a
bind failure — is discarded.

Together these produce failures that look like the product is broken when
it is not, and they can fail a release.

## Motivation

### This is a release-gate risk, not only a nuisance

`quality-gate` in `.github/workflows/release-executable.yaml` runs
`cargo test --workspace` on every tag push. A flake there fails the
release, and the documented recovery in `RELEASING.md` is deleting the
tag and re-pushing it. v5.16.0 passed CI on the first attempt; that was
luck, not evidence.

The rate also appears higher than the roughly 1-in-8 recorded in
`ROADMAP.md`. Three consecutive full local runs on 2026-08-12 failed
**twice**, on two different tests — `serve_json_resources::products_csv_
converts_to_json` and `routing::rule_set::rule::when::request::rule_op::
not_matches_starts_with_2` — both panicking at the same place:

```
thread '...' panicked at crates/apimock/tests/util/http/test_request.rs:117:14:
failed to get https response
```

Different test each time, same call site, and each passes in isolation.
That is the signature of a harness fault rather than a product fault.

### Cause 1 — the port is probed, not held

`dynamic_port()` (`crates/apimock/tests/util/test_setup.rs:117`):

```rust
fn dynamic_port() -> u16 {
    let port = rand::rng().random_range(49152..=65535);
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    match TcpListener::bind(addr) {
        Ok(_) => port,
        Err(_) => dynamic_port(),
    }
}
```

The listener is bound and **immediately dropped** — `Ok(_)` discards it —
so the port is free again before the function returns. The server binds
it later, after a `tokio::spawn` and a 400 ms sleep. Two tests running
concurrently can also probe the same port and both see it free.

### Cause 2 — readiness is a guess, and bind failure is silent

`launch_impl` spawns the server and then sleeps:

```rust
tokio::spawn(async move {
    let app = App::new(&app_env_args, None, true).await
        .expect("App::new failed in test setup");
    app.server.start().await
});

// wait for server started
tokio::time::sleep(std::time::Duration::from_millis(400)).await;
```

Two problems. The 400 ms is unrelated to whether the server is listening
— on a loaded CI runner it may not be. And the spawned task's outcome is
dropped, so if the port was taken in the meantime and `start()` fails to
bind, **nothing reports it**. The test proceeds and fails at its first
request with a connection error, which is why the panic message says
nothing about ports.

This is why the two causes must be fixed together: fixing the race alone
leaves a harness that still cannot tell "server not ready yet" from
"server failed to start".

### A related hazard, in scope to record and out of scope to fix

`TestSetup.current_dir_path` calls `env::set_current_dir`, which is
process-global while tests run concurrently. The field's own doc comment
says *"caution: affects globally"*. This RFC does not fix it, but any
work here must not make it worse, and it should be recorded in
`ROADMAP.md`'s findings table if it is not already.

### A previous attempt failed, which is why this is an RFC

A fix was attempted during RFC 036, regressed **every IPv6 bound-address
test**, and was reverted in full. The obvious change is therefore known
not to work. Establish from the source — do not assume from this document
— what addresses the server actually binds, and whether any test asserts
on a bound address, before choosing an approach.

## Goals

1. A test cannot lose its port to another test between selection and
   bind.
2. A test proceeds when the server is accepting connections, not after a
   fixed interval.
3. A server that fails to start fails the test **with that reason**,
   rather than surfacing later as a connection error.
4. Every test that passes today still passes, including the IPv6
   bound-address tests that the previous attempt regressed.

## Non-goals

- Changing what any test asserts. This is harness work.
- Speeding up the suite. Removing a 400 ms sleep per test may do so
  incidentally; that is a side effect, not a goal, and no test may be
  weakened in pursuit of it.
- Fixing `set_current_dir`. Recorded above, deliberately separate.

## Proposed design

Direction, not prescription — the implementer chooses, having read the
server's binding code.

**Preferred: bind once, hand the socket over.** Let the OS assign a free
port by binding port 0, then have the server use *that* listener, and
read the actual port back from it. There is then no window at all,
because the port is never released. This requires the server to accept a
pre-bound listener or to report its bound address; if it does neither
today, Goal 3 makes a small library addition worthwhile anyway.

**Fallback, if the server must bind by number:** keep the probe, but hold
the listener until the moment the server binds, and have the harness
retry on collision. This narrows the window without closing it, so prefer
it only if the first is genuinely impractical — and say why in the review
request.

**Readiness, either way:** replace the fixed sleep with a poll that
attempts a connection until it succeeds or a timeout expires, and make
the timeout failure message name the port and the address. Propagate the
spawned task's error rather than discarding it.

## Testing and verification

- The full suite passes. Because this is a flakiness fix, a single green
  run is **not** evidence: run `cargo test --workspace` **at least 10
  consecutive times** and report the count of failures, which should be
  zero. Report the number actually run.
- The IPv6 bound-address tests pass specifically, and are named in the
  review request.
- Demonstrate Goal 3: temporarily force a bind failure and show the test
  fails with a message naming the cause. Revert the forcing.

## Risks

| Risk | Mitigation |
|---|---|
| Repeating RFC 036's IPv6 regression | Those tests are named and run explicitly, not just covered by "the suite passes" |
| A library change to expose the bound address widens scope | Keep it minimal and additive; if it grows beyond a small accessor, stop and raise it |
| Flakiness appears fixed because 10 runs got lucky | Report the raw count; if any run fails, the fix is not done |

## Unresolved questions

1. Can the server accept a pre-bound listener today, or report its bound
   address? This decides which design above applies, and is the first
   thing to establish from source.
2. Is the ~1-in-8 figure in `ROADMAP.md` still accurate, or was
   2026-08-12's 2-in-3 a change? Not blocking; worth noting if the cause
   turns out to explain the difference.
