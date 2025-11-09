# apimock-rs (API Mock) Users Documentation

This guide walks through how to get started with `apimock-rs`, a mock HTTP(S) server for testing APIs. It covers basic setup, examples, frequently asked questions, deep configuration.

## 🏞️ Concept Overview

**apimock-rs (API Mock)** is a developer-friendly, super-lightweight and functional HTTP(S) mock server built in Rust. It provides file-based mechanism for mocking RESTful APIs using static JSON responses. Optionally, dynamic routing is supported: rule-based matching via `toml` and scripting via `rhai`.

### Key Features

- 👟 **Zero-config start** – Just point to a directory (folder) with JSON files and run.
- 🍬 **Simple setup** – Usable via a single CLI command, no compilation needed via npm-distributed binary.
- 🧳 **Static routing** – File-based simple responses. Uses directory paths and `.json`-like files to simulate API endpoints.
- 🎒 **Dynamic routing with matching** – Supports conditional responses with rule-based mechanism and scripting.
- 🍨 **High-performance** – Effortless speed and minimal presence. Built with async Rust using `tokio` and `hyper`.
