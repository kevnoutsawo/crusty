# Contributing to Crusty

Thanks for your interest in Crusty. This guide covers everything you need to get started, from setting up your dev environment to getting your pull request merged.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Where to Contribute](#where-to-contribute)
- [Reporting Bugs](#reporting-bugs)
- [Requesting Features](#requesting-features)

---

## Code of Conduct

Be respectful, constructive, and collaborative. We are all here to build a better tool. Harassment, trolling, or dismissive behavior will not be tolerated.

---

## Getting Started

### Prerequisites

- **Rust** (stable, 1.70+) — install via [rustup](https://rustup.rs/)
- **Git**
- A terminal emulator with 256-color support (for TUI development)

### Clone and Build

```bash
git clone https://github.com/kevnoutsawo/crusty.git
cd crusty
cargo build --workspace
```

### Run the TUI

```bash
cargo run -p crusty-tui
```

### Run All Tests

```bash
cargo test --workspace
```

### Check Formatting and Lints

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

All four of these must pass before a PR will be reviewed.

---

## Project Structure

Crusty is a Rust workspace with 11 modular crates. Each crate has a single responsibility:

| Crate | What it does |
|-------|-------------|
| `crusty-core` | Domain models (requests, responses, collections, environments), variable interpolation, request orchestration. Pure logic — no I/O, no UI. |
| `crusty-http` | HTTP engine. Wraps `reqwest` with timing instrumentation (DNS, TCP, TLS, TTFB, content transfer). |
| `crusty-tui` | Terminal UI frontend. Built on `ratatui` + `crossterm`. This is the main binary. |
| `crusty-store` | SQLite persistence for collections, history, environments, and settings. |
| `crusty-auth` | Authentication providers (Bearer, Basic, API Key). Designed for extension. |
| `crusty-export` | Import/export (cURL, Postman v2.1, HAR) and code generation (Rust, Python, JS, Go, Java, PHP, Ruby, C#, Swift, Kotlin). |
| `crusty-scripting` | Pre/post-request scripting engine using Rhai. |
| `crusty-testing` | Assertion engine, collection runner, test reports (JUnit XML, JSON). |
| `crusty-proto` | Protocol adapters for WebSocket and SSE. |
| `crusty-proxy` | HTTP/HTTPS capture proxy with MITM TLS interception. |
| `crusty-mock` | Mock server with configurable endpoints, delay simulation, and conditional matching. |

### Key architectural principles

1. **Core is UI-agnostic.** All business logic lives in `crusty-core` and sibling library crates. The TUI is a thin frontend that wires things together.
2. **Async-first.** All I/O uses `tokio`. The TUI event loop runs on a tokio runtime.
3. **Errors have two tiers.** Library crates use `thiserror` for structured errors. The TUI layer uses `miette` for user-friendly display.
4. **SQLite is the only external dependency.** It ships bundled via `rusqlite` — no database server, no config.

When in doubt about where code belongs, ask: "Would a hypothetical GUI or web frontend need this?" If yes, it goes in a library crate. If no, it goes in `crusty-tui`.

---

## Development Workflow

### Branching

1. Fork the repository and clone your fork.
2. Create a feature branch from `main`:
   ```bash
   git checkout -b feat/your-feature
   ```
3. Make your changes, commit, push, and open a PR against `main`.

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

feat(export): add OpenAPI 3.0 import
fix(tui): prevent crash when response body is empty
refactor(core): simplify variable interpolation pipeline
test(auth): add edge cases for API Key in query params
docs: update CONTRIBUTING.md with testing section
```

**Types:** `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `ci`

**Scopes** (optional): Use the crate name without the `crusty-` prefix — `core`, `http`, `tui`, `store`, `auth`, `export`, `scripting`, `testing`, `proto`, `proxy`, `mock`.

### CI Pipeline

Every push and PR triggers the CI workflow (`.github/workflows/ci.yml`):

1. `cargo fmt --all -- --check` — formatting
2. `cargo clippy --workspace -- -D warnings` — lints (warnings are errors)
3. `cargo test --workspace` — all tests
4. `cargo build -p crusty-tui --release` — release build

Your PR must pass all four steps. Fix all compiler warnings — do not leave them.

---

## Pull Request Process

1. **Keep PRs focused.** One feature or fix per PR. If you find an unrelated issue while working, open a separate PR.
2. **Write a clear description.** Explain what changed and why. If it's a UI change, include a screenshot or recording.
3. **Add tests** for new functionality. See the [Testing](#testing) section.
4. **Update docs** if your change affects user-facing behavior or the architecture.
5. **Respond to review feedback** promptly. We aim for quick turnaround.

### PR checklist

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] New code has tests
- [ ] Commit messages follow Conventional Commits
- [ ] PR description explains the "why"

---

## Coding Standards

### Rust style

- Follow standard Rust idioms. When in doubt, refer to the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `thiserror` for error types in library crates.
- Prefer returning `Result` over panicking. Reserve `unwrap()` and `expect()` for cases where failure is genuinely impossible.
- Keep functions short and focused. If a function exceeds ~50 lines, consider splitting it.

### TUI-specific conventions

- All colors are defined as constants in `render.rs` using the project's design system (GitHub Dark palette). Do not hardcode color values elsewhere.
- Keyboard shortcuts must be documented in the help overlay and the status bar hints.
- New UI features should be keyboard-accessible. Mouse support is secondary.

### Dependencies

- Be conservative with new dependencies. Crusty ships as a single binary — every dependency adds to compile time and binary size.
- If you need a new crate, justify it in your PR description.
- Prefer well-maintained crates with minimal transitive dependencies.

---

## Testing

### Running tests

```bash
# All tests
cargo test --workspace

# A specific crate
cargo test -p crusty-core

# A specific test
cargo test -p crusty-core -- test_interpolation
```

### Writing tests

- Place unit tests in inline `#[cfg(test)] mod tests` blocks within the source file.
- Use descriptive test names: `test_bearer_auth_injects_header`, not `test_auth`.
- Test both success and failure paths.
- For HTTP-related tests, mock at the `reqwest` layer or test against the mock server crate.

### Current test coverage

The workspace has 119+ tests across all library crates. The TUI crate has no automated tests (it's tested manually). If you find a way to improve TUI testability, that's a welcome contribution.

---

## Where to Contribute

### Good first issues

Look for issues labeled `good first issue` on the [issue tracker](https://github.com/kevnoutsawo/crusty/issues). These are scoped, well-defined tasks suitable for newcomers.

### Areas that need help

- **Protocol adapters** — GraphQL, gRPC, MQTT, and Socket.IO are stubbed out in `crusty-proto` but not yet implemented.
- **Advanced auth** — OAuth2 flows, Digest Auth, AWS Signature v4.
- **Response renderers** — HTML preview, image preview, hex viewer for binary responses.
- **Test coverage** — More edge cases, integration tests, TUI testing strategies.
- **Documentation** — Usage guides, tutorials, architecture deep dives.
- **Accessibility** — Screen reader support, high-contrast themes.

See the [ROADMAP](ROADMAP.md) for the full picture of planned work.

---

## Reporting Bugs

Open an issue with:

1. **What you did** — Steps to reproduce the bug.
2. **What you expected** — The behavior you anticipated.
3. **What happened** — The actual behavior, including error messages or screenshots.
4. **Environment** — OS, terminal emulator, Rust version (`rustc --version`), Crusty version.

---

## Requesting Features

Open an issue describing:

1. **The problem** — What are you trying to do that Crusty doesn't support?
2. **The use case** — Why does this matter? How would you use it?
3. **Possible approaches** — If you have ideas for implementation, share them, but focus on the problem first.

We value "I need to do X" over "please add Y."

---

## Questions?

If something isn't covered here, open a discussion or reach out via an issue. We are happy to help you get oriented.
