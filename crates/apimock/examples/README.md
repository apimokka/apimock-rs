# Examples

Each directory below is self-contained and runnable as-is: `cd` into
it and run `apimock` (no flags - it picks up `./apimock.toml`
automatically). Every one has its own `README.md` with what it
demonstrates, the exact command to run, and `curl` calls with their
verified expected response.

| Directory | Demonstrates |
|---|---|
| [`config/default/`](./config/default/) | The quickstart - what `apimock --init` scaffolds and every release archive ships |
| [`serve-json-resources/`](./serve-json-resources/) | File-based JSON/CSV responses - the headline "drop files into a folder" feature |
| [`match-headers-and-body/`](./match-headers-and-body/) | Conditional matching on request headers and JSON body content |
| [`status-codes-and-errors/`](./status-codes-and-errors/) | `respond.status`, alone and with a message body |
| [`vary-response-by-strategy/`](./vary-response-by-strategy/) | `priority`, `weight`, and `round_robin` response strategies |
| [`simulate-slow-backend/`](./simulate-slow-backend/) | `delay_response_milliseconds` |
| [`scripting-with-middleware/`](./scripting-with-middleware/) | A Rhai middleware script, all three response shapes |
| [`secure-with-tls/`](./secure-with-tls/) | HTTPS via `listener.tls` |
| [`validate-in-ci/`](./validate-in-ci/) | `apimock validate` and `apimock match-test` - no server needed |

`config/tests/` and `bench_load.rs` are not examples - the former is
test fixtures for the workspace's own test suite, the latter a load-
sampling harness. Neither is meant to be read as documentation.

## Verified automatically

Every `curl` call and expected response documented above is asserted
by an integration test - see
[`crates/apimock/tests/examples.rs`](../tests/examples.rs) and
[`crates/apimock/tests/examples/`](../tests/examples/). An example
that stops matching its README fails `cargo test --workspace`.
