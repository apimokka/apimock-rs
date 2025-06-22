# Vision and Goals

## Vision

Aims to be a developer-friendly, featherlight and functional HTTP mock server which doesn't require complicated configuration and accepts rich customization around routing.

### Designed in mind with

- Easy setup / usage
    - Built as single (and small) executable, integrated configuration. (No need to write scripts, config-less mode is also supported.)
- Performance
    - Fast speed, low memory consumption.
- Cross-platform support

## Goals

### 1. Basic

- File-based routing frees us around hard setup
- Supports `.json` / `.json5` / `.csv` files treated as JSON Response

### 2. Customization

- Rule-based routing empowers us
- Can specify response time on all or each API path
- Custom HTTP response codes: `3xx` as redirects, and `4xx` and `5xx` as errors

### 3. Dynamic processing

- Flexible responses with condition combination. Even with the same API URL path, multiple responses can be returned.
- Optionally, middleware as Rhai scripts is available to customize request routing and response handling

### 4. Safe and observable usage

- Validates configuration: Missing JSON files, duplicate paths etc.
- Prints out routing at startup
- Describes request content on both HTTP headers and body (json or plain text) when [`verbose`](docs/CONFIGURE.md#generalverbose) on log config is enabled
- Integrated test cases for app stability and robustness

### 5. GUI wrapper integration

- Achieved via cargo feature: When `spawn` is activated, the server is available as subprocess. The output will be returned via tokio mpsc queue.
