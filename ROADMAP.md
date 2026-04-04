# Crusty Roadmap

This document lays out where Crusty is today, where it's headed, and the concrete steps to get there. It's a living document — priorities will shift as the project evolves and the community grows.

---

## Where We Are

Crusty is a fully functional terminal-based HTTP client. The core workflow — build a request, send it, inspect the response — works end to end. Here's what's shipped:

### Shipped

- **Request builder** — All HTTP methods, URL bar, query params, headers, body (JSON, raw, form-urlencoded), auth (Bearer, Basic, API Key), variable interpolation
- **Response viewer** — Syntax-highlighted body, headers, timing breakdown (DNS/TCP/TLS/TTFB/transfer), status and size
- **Collections & folders** — Sidebar with tree navigation, save/load requests, expand/collapse
- **Environments** — Per-environment variables, `{{variable}}` interpolation, environment switcher
- **Request history** — Full history with search and restore
- **Scripting** — Pre/post-request scripts via Rhai, access to request/response context
- **Testing** — Assertion engine (status, headers, body, JSONPath), collection runner, JUnit XML and JSON reports, CI-friendly mode
- **Import/export** — cURL commands, Postman Collection v2.1, HAR format
- **Code generation** — Rust, Python, JavaScript, Go, Java, PHP, Ruby, C#, Swift, Kotlin
- **WebSocket & SSE** — Connect, send/receive messages, stream events
- **Mock server** — Create endpoints from responses, conditional matching, delay simulation, scripted dynamic responses
- **Capture proxy** — HTTP/HTTPS interception with MITM TLS, live traffic log, import captured requests
- **Keyboard-driven TUI** — Vim-style navigation, numbered tabs, context-sensitive hints, help overlay
- **CI/CD** — Automated formatting, linting, testing, and multi-platform release builds (Linux x86/ARM, macOS Intel/Apple Silicon, Windows)
- **119+ tests** across all library crates, all passing

---

## Where We're Going

Crusty aims to be the HTTP client developers reach for first — the one that starts before your IDE finishes loading, works everywhere (local terminal, SSH, CI), and never asks you to sign in.

### Guiding principles

1. **Stay fast and light.** Every feature must justify its cost in binary size and startup time.
2. **Local first.** No accounts, no telemetry, no cloud lock-in. Collaboration features use Git, not proprietary sync. A cloud option may be added later for convenience, but it will always be fully optional — Crusty must remain 100% functional offline with zero external dependencies.
3. **Keyboard first.** Every feature must be fully usable without a mouse.
4. **Modular core.** The TUI is one frontend. The library crates should support future GUIs, web UIs, or headless CLI modes.

---

## How We Get There

Work is organized into phases. Each phase builds on the previous one. Phases are roughly ordered by priority, but individual items within a phase can be tackled in any order.

---

### Phase 1 — Polish & Stability

**Goal:** Harden what's already built. Fix rough edges, improve reliability, and make the existing features production-solid.

| Item | Crate | Details |
|------|-------|---------|
| Scrollable response body | `crusty-tui` | Large responses should scroll smoothly with j/k and Page Up/Down. Currently truncated. |
| Copy to clipboard | `crusty-tui` | Copy response body, headers, or generated code to system clipboard. |
| Persistent settings | `crusty-store` | Save user preferences (theme, default headers, proxy config) to SQLite. |
| Error recovery | `crusty-tui` | Graceful handling of network errors, malformed URLs, and invalid JSON. Show inline error messages instead of crashing. |
| Connection reuse | `crusty-http` | Keep-alive connections and connection pooling for repeated requests to the same host. |
| Request cancellation | `crusty-tui`, `crusty-http` | Cancel in-flight requests with Esc or Ctrl+C without quitting the app. |
| Body type: multipart form-data | `crusty-core`, `crusty-tui` | File upload support via multipart form encoding. |
| Cookie jar | `crusty-http`, `crusty-tui` | Automatic cookie management with inspection UI. Show cookie attributes (domain, path, expiry). |

---

### Phase 2 — Protocol Expansion

**Goal:** Extend beyond REST to cover the protocols developers use daily.

| Item | Crate | Details |
|------|-------|---------|
| GraphQL support | `crusty-proto`, `crusty-tui` | Query editor, schema introspection, variable pane, subscription streaming. Auto-detect GraphQL endpoints. |
| gRPC support | `crusty-proto`, `crusty-tui` | Load `.proto` files or use server reflection. Unary and streaming RPCs. Message builder from proto definitions. |
| MQTT client | `crusty-proto` | Connect to brokers, subscribe to topics, publish messages. Show live message stream. |
| Socket.IO client | `crusty-proto` | Connect, emit events, listen on channels. Handle reconnection. |
| OpenAPI / Swagger import | `crusty-export` | Parse OpenAPI 3.x specs and generate a full collection of requests with examples. |

---

### Phase 3 — Advanced Authentication

**Goal:** Support the auth schemes developers encounter in production APIs.

| Item | Crate | Details |
|------|-------|---------|
| OAuth2 full flow | `crusty-auth`, `crusty-tui` | Authorization Code, Client Credentials, PKCE. Automatic token refresh. Local callback server for redirect URI. |
| Digest Auth | `crusty-auth` | RFC 7616 digest authentication with nonce handling. |
| AWS Signature v4 | `crusty-auth` | Sign requests for AWS APIs. Support for STS temporary credentials. |
| mTLS | `crusty-auth`, `crusty-http` | Client certificate authentication. Certificate picker UI. |
| Auth chaining | `crusty-core` | Inherit auth from collection or folder. Override at the request level. |

---

### Phase 4 — Response Intelligence

**Goal:** Make the response viewer smarter so developers spend less time manually inspecting payloads.

| Item | Crate | Details |
|------|-------|---------|
| JSON path filtering | `crusty-tui` | Type a JSONPath expression to filter the response body in real time. |
| Search in response | `crusty-tui` | `/` to search within the response body, highlighting matches. |
| Response diffing | `crusty-tui` | Compare two responses side by side. Useful for regression testing. |
| HTML preview | `crusty-tui` | Render basic HTML responses as structured text (strip tags, show structure). |
| Image preview | `crusty-tui` | Render images inline using terminal graphics protocols (Sixel, Kitty). |
| Hex viewer | `crusty-tui` | View binary response bodies as hex + ASCII. |
| Redirect chain | `crusty-http`, `crusty-tui` | Visualize the full chain of redirects with status codes and headers at each hop. |

---

### Phase 5 — Developer Workflow Integration

**Goal:** Make Crusty a natural part of the development workflow, not a standalone tool.

| Item | Crate | Details |
|------|-------|---------|
| Headless CLI mode | new: `crusty-cli` | Run requests from the command line without the TUI. `crusty run collection.json --env prod --report junit`. Ideal for CI pipelines. |
| Load testing | `crusty-testing` | Concurrent request execution with configurable concurrency. Report RPS, latency percentiles (p50, p95, p99), error rates. |
| Git-based sync | `crusty-store` | Store collections as JSON/YAML files on disk. Version control with Git. Teams share collections via their repo. |
| `.env` file support | `crusty-core` | Load environment variables from `.env` files. Auto-detect `.env` in the working directory. |
| Shell integration | `crusty-cli` | Pipe responses to `jq`, `grep`, etc. Accept request definitions from stdin. |
| Editor integration | new: `crusty-lsp` | Language server for `.crusty` collection files. Autocomplete variables, validate schemas. |

---

### Phase 6 — Theming & Accessibility

**Goal:** Make Crusty comfortable for everyone, regardless of visual preferences or needs.

| Item | Crate | Details |
|------|-------|---------|
| Custom themes | `crusty-tui` | User-defined color schemes via config file. Ship with light and dark presets. |
| High-contrast mode | `crusty-tui` | Accessible theme for visually impaired users. |
| Configurable keybindings | `crusty-tui` | Remap keys via config file. Support Emacs-style bindings as a preset. |
| Unicode / i18n | `crusty-tui` | Proper handling of wide characters, RTL text, and non-ASCII content in request/response bodies. |

---

## Release Strategy

- **Versioning:** [Semantic Versioning](https://semver.org/). We're pre-1.0 — breaking changes may happen between minor versions.
- **Releases:** Tagged releases trigger automated builds for 5 platforms (Linux x86/ARM, macOS Intel/ARM, Windows).
- **Milestones:** Each phase above corresponds roughly to a minor version bump (0.2, 0.3, etc.). Phase 1 completion targets the 0.2 release.
- **Breaking changes:** Documented in release notes. Migration guides provided when storage schema changes.

---

## How to Influence the Roadmap

- **Vote on issues.** Use thumbs-up on GitHub issues to signal demand.
- **Open a feature request.** Describe the problem you're solving, not just the feature you want. See [CONTRIBUTING.md](CONTRIBUTING.md#requesting-features).
- **Submit a PR.** The fastest way to move something up the roadmap is to build it. We are happy to help scope work in an issue before you start coding.
- **Start a discussion.** For larger ideas that need design input, open a GitHub Discussion.

---

## Status Legend

| Symbol | Meaning |
|--------|---------|
| Shipped | In the current release |
| Phase N | Planned, roughly prioritized |
| Community | Open for contribution — no core team bandwidth allocated yet |

Everything in Phases 2-6 is tagged **Community** — contributions are welcome on any of these items. Check the [CONTRIBUTING guide](CONTRIBUTING.md) to get started.
