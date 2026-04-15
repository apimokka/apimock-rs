# Rule-based routing: More practices

In addition to URL path, you can match with HTTP method, headers and body JSON data.

### Example 2: Match with HTTP Method

```toml
# apimock-rule-set.toml
[[rules]]
when.request.method = "GET"
respond = { status = 403 }

[[rules]]
when.request.method = "POST"
respond = { status = 200 }
```

**Test:**

```sh
curl -i -X GET http://localhost:3001/
# 403 Forbidden

curl -i -X POST http://localhost:3001/
# 200 OK
```

### Example 3: Match with Headers

This example demonstrates how to match requests based on specific HTTP headers. This is useful for simulating authentication or content negotiation.

```toml
# apimock-rule-set.toml
[[rules]]
[rules.when.request.headers]
Authorization = { value = "Bearer eyJhb", op = "starts_with" }
[rules.respond]
text = "Authorized !"
```

**Test:**

```sh
curl -H "Authorization: Bearer eyJhb(...).(...).(...)" http://localhost:3001/
# Authorized !
```

### Example 4: Match with Body JSON data by Dot-Notation JSON Path

This powerful feature lets you match requests based on specific values within the JSON body of an incoming request. You define the target key using a dot-notation path.

```toml
# apimock-rule-set.toml
[[rules]]
[rules.when.request.body.json]
"a.b.c" = { value = "d" }
[rules.respond]
text = "Body JSON matched !"
```

**Test:**

```sh
curl http://localhost:3001/ \
    -H "Content-Type: application/json" \
    -d '{"a":{"b":{"c":"d"}}}'
# Body JSON matched !
```
