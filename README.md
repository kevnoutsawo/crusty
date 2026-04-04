# Crusty

A lightweight, fully local HTTP client for your terminal. Think Postman — but fast, private, and keyboard-driven.

Crusty runs entirely on your machine. No accounts, no cloud sync, no telemetry. Just a single binary that launches instantly and gets out of your way.

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange)

![Crusty Demo](demo.gif)

---

## Why Crusty?

| | Crusty | Postman |
|---|---|---|
| Startup time | Instant | 5-10s |
| Memory usage | ~15 MB | ~500 MB |
| Account required | No | Yes |
| Cloud dependency | None | Baked in |
| Works over SSH | Yes | No |
| Single binary | Yes | No |

---

## Features

**Request Building**
- All HTTP methods — GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE
- Query parameters and headers via key-value editor
- JSON, raw, form-data, form-urlencoded, and binary body types
- Bearer, Basic, and API Key authentication
- Variable interpolation with `{{variable}}` syntax

**Response Analysis**
- Syntax-highlighted response body
- Response headers inspection
- Timing breakdown — DNS, TCP connect, TLS handshake, TTFB, content transfer
- Status code and response size at a glance

**Collections & History**
- Organize requests into collections and folders
- Full request history with search
- Save and reload any request

**Environments**
- Define variables per environment (dev, staging, prod)
- Hierarchical resolution — Global → Collection → Folder → Request
- Secret variable support

**Scripting & Testing**
- Pre-request and post-request scripts (Rhai)
- Built-in assertion engine
- Collection runner with test reports (JUnit XML, JSON)
- CI-friendly mode

**Import & Export**
- Import from cURL commands
- Import Postman Collection v2.1
- Export to HAR
- Generate code snippets

**Keyboard-Driven UX**
- Vim-style navigation (j/k, h/l)
- Numbered tab switching (1-5, F1-F4)
- Context-sensitive status bar hints
- Full help overlay (`?`)

---

## Install

### Download a binary

Grab the latest release for your platform from [Releases](https://github.com/kevnoutsawo/crusty/releases):

| Platform | File |
|----------|------|
| Linux (x86_64) | `crusty-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `crusty-aarch64-unknown-linux-gnu.tar.gz` |
| macOS (Intel) | `crusty-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `crusty-aarch64-apple-darwin.tar.gz` |
| Windows (x86_64) | `crusty-x86_64-pc-windows-msvc.zip` |

**Linux / macOS:**

```bash
tar xzf crusty-*.tar.gz
chmod +x crusty-tui
sudo mv crusty-tui /usr/local/bin/
```

**Windows:**

Extract the zip and add `crusty-tui.exe` to your PATH.

### Build from source

```bash
git clone https://github.com/kevnoutsawo/crusty.git
cd crusty
cargo build -p crusty-tui --release
# Binary is at target/release/crusty-tui
```

---

## Quick Start

```bash
crusty-tui
```

1. Type a URL in the address bar
2. Press `Enter` to send
3. Use `Tab` to navigate between panes
4. Press `?` for the full shortcut reference

### Key Shortcuts

| Key | Action |
|-----|--------|
| `Enter` / `Ctrl+R` | Send request |
| `Tab` / `Shift+Tab` | Cycle focus |
| `1-5` | Switch request tab (Params, Headers, Body, Auth, Script) |
| `F1-F4` | Switch response tab (Body, Headers, Timing, Tests) |
| `j` / `k` | Scroll up / down |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+S` | Save to collection |
| `Ctrl+I` | Import cURL |
| `Ctrl+G` | Generate code |
| `Ctrl+E` | Cycle environment |
| `Ctrl+T` | Run test script |
| `?` | Help |

---

## Architecture

Crusty is a Rust workspace of modular crates — the TUI is just one frontend. The core is UI-agnostic and ready for other interfaces.

```
crusty-tui          Terminal UI (ratatui + crossterm)
    │
    ├── crusty-core       Request/response models, collections, environments, orchestration
    ├── crusty-http       HTTP engine (reqwest, timing instrumentation)
    ├── crusty-auth       Authentication (Bearer, Basic, API Key)
    ├── crusty-store      Local persistence (SQLite)
    ├── crusty-export     Import/export (cURL, Postman, HAR, codegen)
    ├── crusty-scripting  Pre/post-request scripting (Rhai)
    ├── crusty-testing    Assertions, collection runner, CI reports
    ├── crusty-proto      Protocol adapters (REST, GraphQL, WebSocket, gRPC, SSE)
    ├── crusty-proxy      Proxy interception
    └── crusty-mock       Mock server
```

---

## Contributing

Crusty is open source and contributions are welcome.

### Getting started

```bash
git clone https://github.com/kevnoutsawo/crusty.git
cd crusty
cargo build --workspace
cargo test --workspace
cargo run -p crusty-tui
```

### Ways to contribute

- **Bug reports** — Open an issue with reproduction steps
- **Feature requests** — Describe the use case, not just the solution
- **Pull requests** — Fork, branch, and open a PR against `main`
- **Documentation** — Improve docs, add examples, fix typos
- **Testing** — Add tests, especially for edge cases in HTTP handling

### Guidelines

- Run `cargo fmt` and `cargo clippy` before committing
- Keep PRs focused — one feature or fix per PR
- Add tests for new functionality
- Follow [Conventional Commits](https://www.conventionalcommits.org/) for commit messages

---

## License

MIT — see [LICENSE](LICENSE) for details.
