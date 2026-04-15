# Combining Conditions: Powerful Matching

## You can combine conditions in a few ways

- **Across Different Request Parts:** Define multiple conditions involving the **URL path**, **HTTP headers**, and **JSON body** within a single rule. All specified conditions must be met for the rule to trigger.
- **Multiple Conditions within Headers:** Specify multiple header key-value pairs that all must be present and match.
- **Multiple Conditions within Body JSON:** Define multiple JSONPath conditions within the request body that all must match.

Combining conditions gives you tighter control over when a mock response is returned, enabling you to simulate complex API behaviors with precision.

## Example

Here's an example where we combine **a URL path** with **two HTTP headers** and **a body JSON Path** as conditions:

```toml
# rule No.1 (priority)
[[rules]]
[rules.when.request]
url_path = "/api/check"
[rules.when.request.headers]
User = { value = "user1" }
X-Request-Id = { value = "abc123" }
[rules.when.request.body.json]
"a.b.c" = { value = "d" }

[rules.respond]
# Make sure to create `strictly-matched.json` in a JSON format!
file_path = "strictly-matched.json"

# rule No.2
[[rules]]
when.request.url_path = "/api/check"
when.request.headers.User = { value = "user1" }

[rules.respond]
text = "matched"
```

### Important Note on Rule Order

The mock server uses a **first-match strategy**. This means it checks your rules from top to bottom in your `apimock-rule-set.toml` file. The **first rule that completely matches** an incoming request will be applied, and no further rules will be checked. Therefore, place your most specific or highest-priority rules at the top of the file to ensure they are evaluated first.
