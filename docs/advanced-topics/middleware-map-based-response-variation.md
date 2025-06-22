# Middleware: Map-based response variation

Our mock server leverages Rhai scripts to handle dynamic HTTP responses.

Beyond returning a simple string representing a file path, it supports returning map-type values, enabling even greater flexibility in crafting responses. It enables us to return **JSON or plain text bodies**, providing a dynamic and powerful way to mock complex API behaviors.

## Supported Map Keys

The process for generating map-based response is straightforward. You can determine the behavior **by specifying a map key**: either of `file_path`, `json`, `text`. Any of the **associated value should be string**.

### `file_path`

The associated value will be treated as a file path. (The same bahavior when a string-type value is returned instead of map)

#### Example

```js
return #{ "file_path": "some-dir/file.ext" };
```

### `json`

The server will send an `application/json` response. The associated value will be used directly as the HTTP response body. This allows you to generate dynamic JSON responses within your Rhai script.

#### Example

```js
return #{ "json": "{ \"myjsonkey\": \"myjsonvalue\" }" };
```

*Note: Remember to properly escape double quotes within your JSON string.*

### `text`

The server will send a `text/plain; utf-8` response.

#### Example

```js
return #{ "text": "Hello, this is a plain text response !" };
```
