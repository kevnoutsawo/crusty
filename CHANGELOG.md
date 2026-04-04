# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-04

### Added

- **Request building** — All HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE), query params, headers, JSON/raw/form bodies, binary upload
- **Authentication** — Bearer token, Basic auth, API Key (header and query)
- **Response analysis** — Syntax-highlighted body, headers inspection, timing breakdown (DNS, TCP, TLS, TTFB, transfer)
- **Collections & history** — Organize requests into collections and folders, full request history with search
- **Environments** — Per-environment variables with hierarchical resolution (Global > Collection > Folder > Request), secret variable support
- **Scripting** — Pre-request and post-request scripts using Rhai
- **Testing** — Built-in assertion engine, collection runner, JUnit XML and JSON report generation
- **Import/export** — cURL import, Postman Collection v2.1 import, HAR export, code generation (Rust, Python, JS, Go, Java, PHP, Ruby, C#, Swift, Kotlin)
- **Protocols** — WebSocket and SSE support
- **Proxy** — HTTP/HTTPS capture proxy with MITM TLS interception
- **Mock server** — Configurable endpoints with delay simulation, conditional matching, request logging
- **Terminal UI** — Keyboard-driven interface with vim-style navigation, numbered tab switching, context-sensitive status bar hints, full help overlay
- **Local-first** — SQLite persistence, no accounts, no telemetry, single binary

[Unreleased]: https://github.com/kevnoutsawo/crusty/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kevnoutsawo/crusty/releases/tag/v0.1.0
