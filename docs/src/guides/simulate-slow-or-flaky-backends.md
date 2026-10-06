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

Set it **per rule**, on `respond` — or set a default for a whole
rule-set file with `[default] delay_response_milliseconds`, which
applies to every rule in that file that does not set its own. A
per-rule value always wins, and `respond.delay_response_milliseconds =
0` cancels the default for that one rule. See
[Rule-set schema](../reference/rule-set-schema.md#default).

> **Corrected 2026-10-06.** This guide previously said the rule-set-wide
> default "has no effect on any response". That was false — it has
> worked since RFC 045 (Defect 2), and this page was steering you away
> from a feature that does what its name says.

There's no built-in mechanism for a genuinely *flaky* backend (randomly
failing a fraction of requests) — only a fixed, deterministic delay. If
you need actual failure injection, [Rhai middleware](./script-with-rhai-middleware.md)
can implement it directly.

A worked, verified example with three endpoints at increasing delays:
[`crates/apimock/examples/simulate-slow-backend/`](https://github.com/apimokka/apimock-rs/tree/main/crates/apimock/examples/simulate-slow-backend).
