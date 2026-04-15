# Response to `OPTIONS` request

It is crucial for the server to immediately respond to `OPTIONS` method requests, unlike other HTTP methods. This is because `OPTIONS` requests trigger what is known as a "preflight request" in CORS.

The purpose of this preflight `OPTIONS` request is to ask the server for permission before sending the actual request. The browser needs to know if the server understands the method, headers, and credentials that will be used in the actual request.

The server must be configured to process `OPTIONS` requests as preflight checks, responding with the appropriate CORS headers without attempting to process them as regular API calls. This ensures that the browser receives the necessary permissions to proceed with the actual cross-origin requests, thereby stabilizing browser-server communication according to HTTP and CORS specifications.

#### Response headers to `OPTIONS` request

The headers include the server's default `Access-Control-Allow-Methods`, but differ in the key aspects below:

- HTTP Status Code: `204 No Content`
- `Content-Length`: `0`
