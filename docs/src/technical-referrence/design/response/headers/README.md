# Response headers

The proper configuration of **HTTP response headers** is paramount for any API. Even in the context of mock servers, these seemingly minor details significantly influence the stability of communication between the browser (client) and the server, serve as the primary mechanism for CORS (Cross-Origin Resource Sharing) compliance, and ultimately contribute directly to the productivity of API developers

## Default response headers

### For connection stability

| Header | Value |
| --- | --- |
| `Date` | Response date |
| `Content-Length` | Calculated from content body |
| `Connection` | `keep-alive` |
| `Cache-Control` | `no-store` |

### For security

| Header | Value |
| --- | --- |
| `x-content-type-options` | `nosniff` |
