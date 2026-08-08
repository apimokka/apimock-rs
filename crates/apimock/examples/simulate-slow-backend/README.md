# Simulate a slow backend

`respond.delay_response_milliseconds` sleeps before responding - useful
for exercising a client's timeout, retry, or loading-state handling
against a predictable, artificial delay.

## Run it

```sh
cd crates/apimock/examples/simulate-slow-backend
apimock
```

## Try it

```sh
$ time curl http://127.0.0.1:3001/fast
instant response
real    0m0.004s

$ time curl http://127.0.0.1:3001/slow
eventually...
real    0m0.806s

$ time curl http://127.0.0.1:3001/very-slow
much later...
real    0m2.006s
```

The delay is set per rule, on `respond`:

```toml
respond = { text = "eventually...", delay_response_milliseconds = 800 }
```
