# Rule-based routing

## Defining Rules

Now, let's dive into `apimock-rule-set.toml` and define some powerful rules ! Each rule checks incoming requests against specific conditions (like URL path, headers, or body content) and, if they match, sends a predefined response.

Here are examples of how you can set up different types of rules in `apimock-rule-set.toml`:

### Example 1: Match with Request URL Path

These examples show how to define responses based on the incoming request's URL path.

```toml
# apimock-rule-set.toml
[[rules]]
when.request.url_path = ""
respond = { text = "I'm at root." }
```

```toml
# apimock-rule-set.toml
[[rules]]
when.request.url_path = "home"
# Make sure to create `home.json` in a JSON format!
respond.file_path = "home.json"
```

**Test:**

```sh
curl http://localhost:3001/
# I'm at root.

curl http://localhost:3001/home
# (home.json content)
```
