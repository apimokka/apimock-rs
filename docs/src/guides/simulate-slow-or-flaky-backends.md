# Simulate slow or flaky backends

```toml
[[rules]]
when.request.url_path = "/fast"
respond.text = "instant response"

[[rules]]
when.request.url_path = "/slow"
respond = { text = "eventually...", delay_response_milliseconds = 800 }
```

`respond.delay_response_milliseconds` sleeps before responding —
useful for exercising a client's timeout, retry, or loading-state
handling against a predictable, artificial delay.

Set it **per rule**, on `respond`. A rule-set-wide `[default]
delay_response_milliseconds` also exists in the schema, but currently
has no effect on any response — see
[Rule-set schema](../reference/rule-set-schema.md#default).

There's no built-in mechanism for a genuinely *flaky* backend (randomly
failing a fraction of requests) — only a fixed, deterministic delay. If
you need actual failure injection, [Rhai middleware](./script-with-rhai-middleware.md)
can implement it directly.

A worked, verified example with three endpoints at increasing delays:
[`crates/apimock/examples/simulate-slow-backend/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/simulate-slow-backend).
