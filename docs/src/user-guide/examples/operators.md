# Using Operators for Flexible Matching

So far, our rule conditions have only used **exact (equal) matching**. For instance, a header had to be exactly "Bearer xyz", or a path had to be exactly "/api/users".

But real-world scenarios often require more flexibility. Your mock server provides powerful **operators** that allow you to define various types of matching behavior:

- **`equal`**: (Default) Matches if the value is exactly the same.
- **`not_equal`**: Matches if the value is not exactly the same.
    - **Be careful !** Because it matches anything other than the specified value, it can often lead to broader matches than you intend. Consider if a more specific `equal` or other operator might be better for your use case.
- **`starts_with`**: Matches if the value begins with a specific string.
- **`contains`**: Matches if the value includes a specific string anywhere within it.
- **`wild_card`**: Matches using simple wildcard patterns (`*` for any sequence of characters, `?` for any single character). This is incredibly versatile for dynamic paths or values.
    - **Recommended for experienced users only.** The `wild_card` operator offers immense flexibility, but it's easy to create unintended matches. Use it with caution and test your rules thoroughly to ensure they behave as expected.

By choosing the right operator, you can define rules that are both precise and adaptable to varying request patterns.

## Example

```toml
[[rules]]
when.request.url_path = { value = "/disallowed", op = "starts_with" }
respond.status = 403 # FORBIDDEN
# you can't access both `http://localhost:3001/disallowed` and `http://localhost:3001/disallowed/some/room` (403 Forbidden) but can `http://localhost:3001/allowed/disallowed` (perhaps 404 Not Found)

[[rules]]
when.request.url_path = { value = "cookie", op = "contains" }
respond.text = "Cookie found !"
# test with `curl http://localhost:3001/delicious/cookie/in-the-can`
```
