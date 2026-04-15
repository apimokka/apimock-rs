# `prefix` table

## `prefix.url_path`

This defines a **URL path prefix** for all rules in this file. Only incoming requests whose URL paths start with this defined prefix will be considered for matching by the rules in this `apimock-rule-set.toml` file. This is useful for organizing rules for specific API versions or modules.

- Example: If `prefix.url_path = "/api/v2"`, then a rule with `url_path = "/users"` would effectively match `/api/v2/users`.

## `prefix.respond_dir`

When defining a response using `respond.file_path`, this prefix is prepended to your specified file path. This helps you organize all your response files in a single, common directory without having to repeat the full path in every rule.

- Example: If `prefix.respond_dir = "responses/"` and a rule has `respond.file_path = "user_data.json"`, the server will look for `responses/user_data.json`.

## Example

```toml
# apimock-rule-set.toml
[prefix]
url_path = "/api/v2"
respond_dir = "responses/"
```
