# Middleware: Basics with Rhai scripts

Our mock server supports **middlewares** written using **Rhai scripts** for highly dynamic scenarios.

For some unique cases, Rhai is a powerful, embedded scripting language that feels very similar to JavaScript or Rust.

Here are some basic Rhai code examples you might use.

## Note: File-based and rule-based routing is preferred

Our goal is to minimize the need for custom script creation and maintenance. We believe that **file-based and rule-based matching definitions can cover almost all practical needs** you'll encounter.

Therefore, **we generally do not recommend using middlewares** unless you have a very specific and complex requirement that cannot be met by combining rules and operators.

## Variable Definition

```js
let my_variable = "Hello, Rhai !";
let count = 10;
```

## If / Else Statements

```js
if url_path == "/dynamic_response" {
    return "response_for_dynamic.json";\

// else clause is also available:
// } else {
//    return "default_response.json";

}
```

## Switch (Match) Statements

```js
switch (url_path) {
    "/middleware-test/dummy" if body.middleware == "isHere" => {
        // exit() is useful when run in fn (here, equivalent to return statement):
        exit(returned_json_file_path);
    },
    _ => ()
}
```

## Return Statement

Middleware scripts primarily return a file path string. If a middleware returns a value, the server will use it as the response.

```rust
return "path/to/response.json";
```

## To learn more about Rhai's syntax and capabilities

you can refer to the [official Rhai documentation](https://rhai.rs/book/language/values-and-types.html). Use this feature judiciously and only when other options fall short.
