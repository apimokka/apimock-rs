# Architecture

## Designed in mind with

- Easy setup / usage
    - Built as single (and small) executable, integrated configuration. (No need to write scripts, config-less mode is also supported.)
- Performance
    - Fast speed, low memory consumption.
- Cross-platform support

## Core Components

- **`src/config.rs`**  
  Defines configuration structures for customizable parameters like the server port, root path, logging levels, etc.

- **`src/server.rs`**  
  The HTTP server entry point powered by `hyper`. It handles incoming requests and delegates them to the core logic.

- **`src/core/server/routing.rs`**  
  Contains core logic including request routing, matching rules, response rendering, and script evaluation.

- **`tests/`**  
  Includes test cases run by `cargo test`.
