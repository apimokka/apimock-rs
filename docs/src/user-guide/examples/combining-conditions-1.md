# Combining Conditions

## Example

Here's an example where we combine a URL path with two HTTP headers as conditions:

```toml
[[rules]]
[rules.when.request]
url_path = "/api/check"
[rules.when.request.headers]
User = { value = "user1" }
X-Request-Id = { value = "abc123" }

[rules.respond]
text = "Matched with headers"
```

### Important Note on Combined Conditions

These conditions are evaluated using **AND logic**. This means all defined conditions within a single rule must be true for the rule to match the incoming request. If even one condition isn't met, the matching fails for that particular rule.
