# `respond` data

The `respond` table within each `[[rules]]` block specifies the response to be returned when the rule's `when` conditions are fully met. You must define one of the following.

## `respond.file_path`

Returns the content of a specified file. The `Content-Type` header is automatically determined from the file's extension (e.g., `.json` will set `application/json`).

```toml
# apimock-rule-set.toml
[[rules]]
# when ...
respond.file_path = "response.json"
```

## `respond.text`

Returns the specified string as the response body. The `Content-Type` header is `text/plain`.

```toml
# apimock-rule-set.toml
[[rules]]
# when ...
respond.text = "My reply !"
```

## `respond.status`

Sets the HTTP status code for the response (e.g., `200` for OK, `404` for Not Found).

```toml
# apimock-rule-set.toml
[[rules]]
# when ...
respond.status = 401
```

## Limitation

You cannot specify both `respond.file_path` and `respond.text` in the same rule.
