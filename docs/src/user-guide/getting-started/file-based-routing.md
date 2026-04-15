# File-based routing

The server uses your directory structure to automatically serve the correct response. When a request comes in, it checks your directories and files (like `data.json` for `/data`) to find a match for the URL. All you need to do is organize your files and directories where the server is running.

## Example Directory Structure

Here's an example of how you might set up:

```
(root)/
└── api/
    └── v1/
        ├── status.json
        ├── user/
        │   ├── 1.json
        │   └── 2.json5
        └── users.csv
```

- A request to `/api/v1/status` returns `status.json`.
- `/api/v1/user/1` returns the content of `1.json`.
- `/api/v1/user/2` returns the content of `2.json5`. `.json5` is equivalent to `.json` to the server.
- `/api/v1/users` returns the content of `users.csv` as list.
- Each of `/`, `/api` and `/api/v1` returns HTTP Status Code 404
    - unless an "index" file (e.g., **`index.json`, `.json5`, or `.csv`**. Also **`.html`**) is present in the respective directory.

## What's next ?

File-based routing is great for simple cases where your response directly maps to a URL. It gets you up and running quickly for many basic mocking needs.

### Expanding Your Mock Server's Capabilities

However, you'll quickly discover some limitations. For instance, you can't create responses that depend on <!-- **HTTP methods** (in the future), -->**request headers** or **body data**, limiting your flexibility.

When your mocking needs go beyond basic URL matching, **rule-based configuration** becomes incredibly powerful. This approach gives you much more control, but it requires a configuration file. Don't worry, we'll walk you through setting it up so you can harness its full potential !
