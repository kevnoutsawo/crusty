# Agent Prompt: Build "Crusty" — A Rust HTTP Client for Developers

You are building **Crusty**, an open-source, developer-grade HTTP client written in Rust. It is a full-featured alternative to Postman, Insomnia, and httpie — architected for precision, speed, and accuracy. It ships as three frontends on a single shared core: a **Tauri desktop app**, a **browser-deployed WASM app**, and a **TUI** (terminal UI).

Every design decision must serve developers and software engineers. The UX must feel like it was built by someone who debugs APIs for a living.

---

## 1. Architecture

### 1.1 Crate Topology (Cargo Workspace)

```
crusty/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── crusty-core/          # Pure Rust. Zero UI. All business logic.
│   ├── crusty-http/          # HTTP engine (reqwest + hyper abstraction)
│   ├── crusty-proto/         # Protocol adapters: REST, GraphQL, gRPC, WebSocket, SSE, MQTT, Socket.IO
│   ├── crusty-scripting/     # Pre/post-request scripting engine (Rhai or mlua)
│   ├── crusty-store/         # Persistence: collections, environments, history (SQLite via rusqlite)
│   ├── crusty-auth/          # Auth flows: OAuth2, JWT, API Key, Basic, Bearer, Digest, AWS Sig v4, mTLS
│   ├── crusty-export/        # Import/export: Postman, Insomnia, OpenAPI, cURL, HAR
│   ├── crusty-testing/       # Test runner, assertions, CI mode, collection runner
│   ├── crusty-proxy/         # Proxy interception, MITM-style request capture
│   ├── crusty-mock/          # Mock server engine
│   ├── crusty-tauri/         # Tauri v2 desktop frontend
│   ├── crusty-web/           # WASM frontend (Leptos or Dioxus + wasm-bindgen)
│   └── crusty-tui/           # Terminal UI (ratatui)
├── proto/                      # .proto files for gRPC support
├── assets/                     # Icons, fonts, themes
├── tests/                      # Integration tests
└── docs/
```

### 1.2 Core Principles

- **`crusty-core` is the gravity center.** It owns request construction, response parsing, variable interpolation, environment resolution, collection tree operations, and orchestration. It is frontend-agnostic, `#![no_std]`-compatible where feasible, and fully unit-tested.
- **`crusty-http` wraps `reqwest` (native) and `gloo-net`/`web-sys` (WASM).** Use conditional compilation (`#[cfg(target_arch = "wasm32")]`) to switch transports. Never leak transport details into core.
- **Every crate exposes a clean trait boundary.** Frontends consume traits, never concrete types. This enforces the contract: if it compiles for Tauri, it compiles for WASM and TUI.
- **Async-first via `tokio` (native) and `wasm-bindgen-futures` (WASM).** Use `async-trait` or associated futures at crate boundaries.
- **Error handling:** Use `thiserror` for library crates, `miette` or `color-eyre` for user-facing error display. Every error must be actionable — include what went wrong, why, and what the user can do.

### 1.3 Data Flow

```
User action (UI event)
  → Frontend adapter (Tauri command / WASM binding / TUI event)
    → crusty-core (orchestration: interpolate vars, resolve env, run pre-scripts)
      → crusty-http (execute request)
      → crusty-proto (protocol-specific encoding/decoding)
    ← Response + metadata (timing, TLS info, redirect chain, size)
  ← crusty-core (run post-scripts, assertions, store history)
← Frontend renders response
```

---

## 2. Feature Set (Complete Specification)

### 2.1 Request Builder

- **Methods:** GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE, CONNECT, plus custom methods.
- **URL bar:** Autocomplete from history + collection endpoints. Inline environment variable highlighting (`{{var}}` rendered as pills/chips with resolved preview on hover).
- **Query params:** Dedicated key-value editor with bulk-edit (raw text) toggle. Support disable/enable per param without deleting.
- **Headers:** Key-value editor. Auto-suggest common headers (Content-Type, Authorization, Accept, Cache-Control, etc.). Show inherited headers from collection/folder level. Computed headers (Content-Length, Host) shown as read-only ghost entries.
- **Body types:**
  - `none`
  - `JSON` — with schema-aware editor, syntax highlighting (tree-sitter), auto-format, collapse/expand, JSONPath support.
  - `Form Data` (multipart/form-data) — file uploads, key-value with type selector (text/file).
  - `x-www-form-urlencoded`
  - `Raw` — with language selector: Text, JSON, XML, YAML, HTML, JavaScript.
  - `Binary` — file picker.
  - `GraphQL` — query editor with variable pane, introspection-powered autocomplete, schema explorer.
  - `Protocol Buffers` — load `.proto` files, select service/method, JSON-to-proto serialization.
- **Auth tab:** Dropdown selector for auth type. Each type has its own form:
  - Bearer Token
  - Basic Auth
  - API Key (header, query param, or cookie)
  - OAuth 2.0 (Authorization Code, Client Credentials, PKCE, Implicit, Password — full flow with token refresh, callback server)
  - Digest Auth
  - AWS Signature v4
  - mTLS (client cert + key file pickers)
  - JWT Bearer
  - Hawk
  - NTLM
  - Inherit from parent (collection/folder)
- **Pre-request scripts:** Editor pane with scripting API access to set variables, modify headers, log, skip request conditionally.
- **Settings tab per request:** Follow redirects (toggle + max count), timeout (connect + read, separate), SSL verification toggle, proxy override, HTTP version preference (1.1, 2, 3/QUIC).

### 2.2 Response Viewer

- **Body renderers:**
  - Pretty (auto-formatted JSON/XML/HTML with syntax highlighting, collapsible tree).
  - Raw (exact bytes as received).
  - Preview (render HTML in sandboxed iframe/webview, render images, render PDF).
  - Hex viewer for binary responses.
- **Headers panel:** Sortable, filterable, with human-readable descriptions on hover.
- **Cookies panel:** Parsed cookie jar with attributes (domain, path, expires, secure, httpOnly, SameSite).
- **Timeline/Waterfall:**
  - DNS lookup, TCP handshake, TLS negotiation, TTFB, content transfer — each with duration.
  - Full redirect chain visualization (each hop as a row).
  - TLS certificate details (issuer, expiry, chain, protocol version, cipher suite).
- **Size breakdown:** Headers size + body size, compressed vs. decompressed.
- **Response meta:** Status code + text (with color coding), response time (ms), response size.
- **Response assertions:** Inline test results shown as pass/fail badges.
- **Copy/export:** Copy response body, copy as cURL, save to file, copy specific header value.

### 2.3 Environments & Variables

- **Environment hierarchy:** Global → Collection → Folder → Request (each level can override).
- **Variable types:** String, secret (masked, never exported), dynamic (auto-generated: `{{$timestamp}}`, `{{$randomUUID}}`, `{{$randomInt}}`, `{{$randomEmail}}`, etc.).
- **Environment quick-switcher:** Dropdown in top bar, keyboard shortcut (Ctrl+E).
- **Vault/secret management:** Secrets encrypted at rest (AES-256-GCM via `ring` or `aes-gcm`). Never shown in plain text in exports or history. Configurable unlock (password, system keychain integration via `keyring` crate).
- **Session variables:** Variables that persist for a session but aren't saved to environment files. Useful for tokens obtained mid-flow.
- **Variable preview:** Hover any `{{var}}` anywhere in the UI to see its resolved value and source (which environment level).

### 2.4 Collections & Organization

- **Collection tree:** Folders with arbitrary nesting. Drag-and-drop reorder.
- **Folder-level configuration:** Shared auth, headers, pre-request scripts, tests — inherited by all children.
- **Collection runner:** Execute all requests in a folder/collection sequentially or in parallel. Configurable delay, iteration count, data file (CSV/JSON) for parameterized runs.
- **Import/export:**
  - Import from: Postman Collection v2.1, Insomnia v4, OpenAPI 3.x, Swagger 2.0, cURL commands, HAR files, WSDL.
  - Export to: Postman Collection, OpenAPI, cURL, HAR, code snippets (Rust reqwest, Python requests, JavaScript fetch, Go http, Java HttpClient, PHP cURL, Ruby Net::HTTP, C# HttpClient, Swift URLSession, Kotlin OkHttp).
- **Collaborative sync (stretch):** Git-based collection sync. Collections stored as human-readable YAML/JSON files designed for version control.

### 2.5 Protocol Support

- **REST/HTTP:** Full featured as described above.
- **GraphQL:**
  - Introspection query on connect.
  - Schema explorer sidebar with types, queries, mutations, subscriptions.
  - Query editor with autocomplete, variable pane, header pane.
  - Subscription support via WebSocket.
- **WebSocket:**
  - Connect/disconnect with custom headers and protocols.
  - Message composer (text/binary).
  - Message log with timestamps, direction (sent/received), size.
  - Auto-reconnect with backoff.
  - Filter/search messages.
- **Server-Sent Events (SSE):**
  - Connect with GET request + headers.
  - Live event stream display with event type, data, ID, retry.
  - Reconnection tracking.
- **gRPC:**
  - Load `.proto` files or use server reflection.
  - Unary, server streaming, client streaming, bidirectional streaming.
  - Metadata (headers/trailers) editor.
  - Message editor with proto schema validation.
  - Deadline/timeout configuration.
- **MQTT (v3.1.1 & v5.0):**
  - Connect with broker URL, client ID, credentials, TLS.
  - Subscribe to topics with QoS selection.
  - Publish messages with topic, QoS, retain flag.
  - Live message feed with topic filter.
- **Socket.IO:**
  - Connect with namespace and auth.
  - Emit/listen on events.
  - Live event log.

### 2.6 Testing & Assertions

- **Test script editor** (per-request and per-folder):
  - Assert status code, header values, body content (JSONPath, XPath, regex, contains).
  - Assert response time thresholds.
  - Assert schema (JSON Schema validation).
  - Chain requests: extract values from responses into variables for subsequent requests.
- **Test runner UI:** Run results with pass/fail/skip counts, duration, expandable details per assertion.
- **CI/CLI mode:** `crusty run <collection> --environment <env> --reporters cli,junit,json` — outputs JUnit XML, JSON report, or custom format. Exit code reflects pass/fail. This is the `crusty-testing` crate invoked directly from the TUI or a headless CLI binary.
- **Load testing (lightweight):** Configurable concurrent requests, iterations, ramp-up. Display RPS, latency percentiles (p50, p95, p99), error rate. Not a full load tester — positioned as quick sanity checks.

### 2.7 Mock Server

- **Create mock endpoints** from existing responses (one-click from response viewer: "Mock this").
- **Configurable responses:** Status code, headers, body, delay simulation, conditional matching (match by header, query param, body content).
- **Dynamic responses** via scripting: access request data, return computed responses.
- **Runs locally** on configurable port. Auto-assigned port with display.
- **OpenAPI-driven mocks:** Import an OpenAPI spec, auto-generate mock endpoints with example data.

### 2.8 Proxy & Capture

- **Built-in proxy server:** Capture HTTP/HTTPS traffic from browsers or other tools. MITM TLS with auto-generated CA cert.
- **Traffic log:** All captured requests displayed in real-time list. Click to inspect full request/response. Filter by method, status, host, content type.
- **Import captured requests** into collections.
- **Proxy configuration per request:** Route through external proxy (HTTP/HTTPS/SOCKS5). Bypass list.

### 2.9 Utilities & Developer Ergonomics

- **Code generation:** Generate request code from any saved request in 12+ languages (see export list above).
- **cURL import/export:** Paste a cURL command → auto-populates the request builder. Export any request as cURL.
- **Request comparison:** Side-by-side diff of two responses (headers and body).
- **Request history:** Timestamped log of every request sent. Search, filter by method/status/URL, restore any past request.
- **Bulk edit mode:** Edit raw headers, params, form data as text.
- **Console/log:** Dedicated console panel showing script output, variable resolutions, network events, errors.
- **Search everywhere:** Cmd+K / Ctrl+K omnibar. Search collections, history, environments, settings.
- **Keyboard-driven:** Full keyboard navigation. Vim-style keybindings option. Every action has a shortcut. Shortcut reference sheet (Ctrl+/).
- **Tabs:** Multi-tab interface. Pin tabs. Reorder tabs. Duplicate tab. Close others.
- **Snippets library:** Save and reuse body templates, header sets, auth configurations.

---

## 3. Frontend Specification

### 3.1 Design System (Shared Across All Frontends)

**Philosophy:** Precision, clarity, density. This is a power tool — it should feel like a cockpit, not a toy. Every pixel earns its place.

**Color System:**

- **Dark theme (default):** Background `#0D1117` (near-black with blue undertone), surface `#161B22`, elevated surface `#21262D`, border `#30363D`. Text primary `#E6EDF3`, secondary `#8B949E`. Accent: electric blue `#58A6FF` for primary actions, green `#3FB950` for success/2xx, amber `#D29922` for warnings/3xx, red `#F85149` for errors/4xx-5xx. Status code colors follow HTTP semantics (1xx blue, 2xx green, 3xx amber, 4xx red, 5xx deep red).
- **Light theme:** Respectful inversion. Background `#FFFFFF`, surface `#F6F8FA`, elevated `#FFFFFF` with shadow. Text `#1F2328`, secondary `#656D76`. Same accent hues, adjusted for contrast.
- **High-contrast theme:** WCAG AAA compliance.

**Typography:**

- UI: Inter or system font stack, 13px base.
- Code/monospace: JetBrains Mono or Fira Code, with ligatures, 12.5px.
- Use font weight to establish hierarchy, not size inflation. Titles 14-15px semibold max.

**Spacing:** 4px grid. Density toggle: Compact (4px gaps, 28px row height), Default (8px gaps, 32px row height), Comfortable (12px gaps, 36px row height).

**Iconography:** Lucide icon set or Phosphor. 16px standard, 20px for primary actions. Never decorative — every icon communicates.

**Component Design Principles:**

- Inputs have visible focus rings (2px accent outline).
- Buttons: Primary (filled accent), Secondary (ghost/outline), Danger (red). No gradients. Subtle press states (scale 0.98, darken 5%).
- Key-value editors: Table-style with inline editing. Checkbox to enable/disable rows. Drag handles. Type indicators.
- Tabs: Underline-style, not boxed. Active tab has 2px accent bottom border.
- Panels: Resizable with drag handles. Collapsible. Remember sizes.
- Toasts: Bottom-right, auto-dismiss, action buttons. Never blocking.
- Modals: Minimal use. Prefer inline editing and panels.

### 3.2 Animations & Micro-Interactions

**Guiding Principle:** Animations communicate state changes, guide attention, and confirm actions. They are never decorative or slow. Every animation must complete in ≤200ms (most in 100-150ms). Use `ease-out` for entrances, `ease-in` for exits, `ease-in-out` for morphs.

**Required animations:**

- **Send button:** On click, subtle pulse + transform to a spinner. On response arrival, morph spinner into status code badge with color-coded flash (green flash for 2xx, red for errors). 100ms transition.
- **Request/response panels:** Smooth height/width transitions when resizing. Spring physics for panel snapping.
- **Tab transitions:** Horizontal slide with opacity crossfade. New tabs grow from zero width. Closing tabs shrink + fade.
- **Status codes:** Appear with a quick number-roll animation (like a departure board), settling on the final code.
- **Response time:** Counter that ticks up in real-time during request, freezes on completion.
- **Tree view (collections):** Folder expand/collapse with smooth height animation. Drag-and-drop with hover-triggered gap opening.
- **Toast notifications:** Slide in from right, auto-dismiss with shrinking progress bar.
- **Keyboard shortcut hints:** Fade in next to elements when Ctrl/Cmd is held.
- **Loading states:** Skeleton screens, not spinners (for panels). Subtle shimmer effect on skeleton elements.
- **Hover states:** Key-value rows highlight with 50ms fade. Buttons show tooltip after 400ms delay.
- **Error states:** Red border with subtle shake animation (2px, 3 oscillations, 300ms). Error messages slide down with spring easing.
- **Success confirmations:** Brief green checkmark with scale-up + fade-out (saved, copied, etc.).
- **Panel focus:** Active panel gets a subtle glow or brighter border to show which panel is focused.
- **WebSocket/SSE message arrival:** New messages slide in from bottom with a brief highlight flash, then settle to normal background.
- **Timeline waterfall:** Bars animate from left to right, each segment filling in sequence to show the actual timing.
- **Request diff:** Changed lines highlight with a brief yellow pulse that fades to the normal diff color.

**Forbidden:**

- No animation longer than 300ms.
- No bounce effects. No overshoot.
- No parallax, no page transitions that delay interaction.
- No animation on initial page load (content appears instantly; only subsequent state changes animate).
- Respect `prefers-reduced-motion`: disable all animations if set.

### 3.3 Tauri Desktop Frontend (`crusty-tauri`)

- **Framework:** Tauri v2 with a Rust backend. Frontend in TypeScript with **SolidJS** (not React — smaller bundle, true reactivity, no virtual DOM overhead — this is a performance tool).
- **Styling:** Tailwind CSS v4 + CSS custom properties for theming. No CSS-in-JS.
- **State management:** SolidJS stores + fine-grained reactivity. No global state library. Core state flows through Tauri commands (invoke) to the Rust backend.
- **Editor:** CodeMirror 6 for all code editing (request body, scripts, response body). Configure with: JSON/XML/YAML/GraphQL modes, bracket matching, autocomplete, line numbers, minimap (optional), search/replace.
- **Layout:** Three-column default: sidebar (collections, 240px) | request builder (flex) | response viewer (flex). Drag-resizable panes. Collapsible sidebar (Ctrl+B). Detachable response panel to secondary monitor.
- **System integration:** Native menus, native file dialogs, system tray with quick-launch, global shortcut for quick request (think Spotlight/Alfred), auto-update via Tauri updater.
- **Window management:** Multi-window support (open requests in new windows). Remember window position and size.

### 3.4 Browser WASM Frontend (`crusty-web`)

- **Framework:** Same SolidJS component library as Tauri, shared via a `crusty-ui` package. WASM core compiled with `wasm-pack`.
- **Limitations to handle gracefully:**
  - No filesystem access → Use IndexedDB (via `idb` crate or JS interop) for persistence. File import/export via browser APIs.
  - No raw TCP/TLS → gRPC and MQTT via browser-compatible transports only (gRPC-web, MQTT over WebSocket). Show clear messaging when a feature requires native transport: "This protocol requires the desktop app."
  - CORS restrictions → Document clearly. Provide a companion `crusty-cors-proxy` (tiny Rust binary) users can run locally. Show a help banner when CORS errors are detected.
  - No proxy/MITM features → Disable gracefully with "Desktop only" badges.
- **PWA:** Full PWA support (Service Worker, manifest, offline capable for cached collections).
- **Deployment:** Static site. Deploy to Vercel/Netlify/Cloudflare Pages. Single `index.html` + WASM bundle + JS glue.

### 3.5 TUI Frontend (`crusty-tui`)

- **Framework:** `ratatui` with `crossterm` backend.
- **Layout:**
  - Left pane: Collection tree (navigable with j/k, expand/collapse with Enter/l/h).
  - Center: Request builder (tab-cycled sections: URL, params, headers, body, auth, scripts).
  - Right/bottom: Response viewer (toggle between body, headers, timeline, tests).
  - Bottom bar: Status (method, URL, status code, time), keybinding hints.
- **Editor integration:** For long body edits, open `$EDITOR` (vim, nvim, etc.) in a temp file, read back on close. Inline editing for short inputs.
- **Mouse support:** Optional. Click to focus panes, scroll, select tree items.
- **Color:** True-color (24-bit). Graceful degradation to 256-color and 16-color terminals.
- **Keybindings:** Vim-inspired defaults. Fully remappable via config file.
  - `Ctrl+Enter` / `Ctrl+R`: Send request.
  - `Tab` / `Shift+Tab`: Cycle panes.
  - `/`: Search/filter.
  - `e`: Edit current field in `$EDITOR`.
  - `y`: Yank/copy response body.
  - `:w`: Save request to collection.
  - `q` / `Ctrl+C`: Quit.
  - `?`: Help/keybinding sheet overlay.
- **Feature parity strategy:** The TUI must support: full request building, response viewing (pretty-printed, raw, headers, timing), environments, collections, history, test running, code generation, cURL import/export. It does NOT need: drag-and-drop, mock server GUI (use CLI flags), proxy GUI.

---

## 4. Persistence & Configuration

### 4.1 Storage

- **Engine:** SQLite via `rusqlite` (native) or `sql.js` (WASM).
- **Schema:** Collections, requests, responses (optionally cached), environments, history, settings, snippets, certificates.
- **File-based export:** Collections exportable as a directory of YAML files (one per request, with folder structure mirroring collection tree). Designed for Git.
- **Migrations:** Embed schema migrations using `rusqlite`'s user_version pragma. Forward-only.

### 4.2 Configuration

- **Config file:** `~/.config/crusty/config.toml` (native) or IndexedDB (WASM).
- **Configurable:** Theme, font size, density, keybindings, default headers, proxy settings, TLS settings, editor preferences (word wrap, minimap, ligatures), auto-save interval, history retention, telemetry opt-in/out.
- **CLI flags override config file.** Environment variables override both (`CRUSTY_THEME=dark`).

---

## 5. Build, Test, CI

### 5.1 Build Targets

```bash
# Desktop (Tauri)
cargo tauri build                    # Release build for current OS
cargo tauri dev                      # Dev mode with hot-reload

# WASM
wasm-pack build crates/crusty-web --target web
# Serve with any static server

# TUI
cargo build --bin crusty-tui --release

# CLI (headless test runner)
cargo build --bin crusty-cli --release
```

### 5.2 Testing Strategy

- **Unit tests:** Every crate, `cargo test --workspace`. Target ≥80% coverage on core.
- **Integration tests:** In `tests/`, spin up mock HTTP servers (`wiremock` crate), run full request flows through the core.
- **E2E (Tauri):** WebDriver-based tests via `tauri-driver`.
- **E2E (WASM):** Playwright or `wasm-bindgen-test`.
- **TUI snapshot tests:** `insta` crate for terminal output snapshot testing.
- **Protocol tests:** Per-protocol test suites — mock GraphQL server, mock WebSocket server, mock gRPC server, etc.
- **CI:** GitHub Actions. Matrix: Linux, macOS, Windows. Lint (`clippy`), format (`rustfmt`), audit (`cargo-audit`), test, build all targets.

---

## 6. Implementation Sequence

### Phase 1: Foundation (Core + TUI MVP)

1. Set up Cargo workspace with all crate skeletons.
2. `crusty-core`: Request/response models, environment variable interpolation, collection tree data structures.
3. `crusty-http`: Basic HTTP client (GET, POST, PUT, DELETE) with `reqwest`. Timing instrumentation.
4. `crusty-store`: SQLite schema, CRUD for collections, requests, environments, history.
5. `crusty-tui`: Basic layout — URL input, method selector, response viewer (body + headers + status).
6. End-to-end: Type a URL in TUI → send request → see response.

### Phase 2: Request Builder Completeness

7. All body types (JSON, form-data, binary, raw, GraphQL body editor).
8. Auth tab: Bearer, Basic, API Key, OAuth 2.0 (authorization code + PKCE).
9. Key-value editors for headers, query params, form data.
10. Environment variable resolution with `{{var}}` syntax.
11. Request history with search.

### Phase 3: Tauri Desktop

12. SolidJS app scaffold with Tailwind, CodeMirror integration.
13. Wire all Tauri commands to core crate functions.
14. Implement full UI: sidebar, tabs, request builder, response viewer, environment switcher.
15. Animations and micro-interactions.
16. Native menus, file dialogs, system tray, auto-update.

### Phase 4: Advanced Protocols

17. WebSocket support (connect, send, receive, log).
18. SSE support.
19. GraphQL introspection + schema explorer.
20. gRPC support with proto file loading.

### Phase 5: Testing & Scripting

21. `crusty-scripting`: Rhai integration. Pre/post-request script API (set variables, assert, log).
22. `crusty-testing`: Assertion engine, collection runner, CI reporter (JUnit, JSON).
23. Test runner UI in Tauri and TUI.

### Phase 6: Browser WASM

24. Compile core to WASM. IndexedDB persistence layer.
25. Shared SolidJS components, WASM bindings.
26. CORS proxy companion tool.
27. PWA manifest + service worker.

### Phase 7: Polish & Ecosystem

28. Mock server engine + UI.
29. Proxy/capture mode (desktop only).
30. Import/export (Postman, Insomnia, OpenAPI, cURL, HAR).
31. Code generation for 12+ languages.
32. Request diff/comparison.
33. Load testing module.
34. MQTT and Socket.IO support.

---

## 7. Technical Constraints & Non-Negotiables

- **No `unwrap()` in library crates.** All errors propagated. `unwrap()` only in tests and `main()`.
- **No `clone()` without justification.** Audit clone usage. Prefer borrows, `Arc`, or `Cow`.
- **No `unsafe` without a `// SAFETY:` comment and a compelling reason.**
- **All public APIs documented.** `#![warn(missing_docs)]` on all crates.
- **Serde everywhere.** Every data structure that crosses a boundary (IPC, persistence, export) derives `Serialize` + `Deserialize`.
- **Audit dependencies.** Run `cargo-audit` and `cargo-deny` in CI. Minimize dependency tree.
- **Deterministic builds.** Lock `Cargo.lock` in version control.
- **Accessibility:** Tauri and Web apps must support screen readers, keyboard navigation, and high-contrast mode. ARIA labels on all interactive elements.
- **Internationalization (stretch):** Use `fluent-rs` for i18n. English first, structure for future localization.
- **Performance budget:** Cold start < 500ms (desktop), < 2s (WASM). Request send-to-render < 50ms overhead beyond network time. UI must maintain 60fps during animations.

---

## 8. File & Naming Conventions

- Crate names: `crusty-*` (kebab-case).
- Rust modules: `snake_case`.
- TypeScript/SolidJS: PascalCase components, camelCase functions, kebab-case files.
- CSS: Tailwind utilities. Custom CSS in `*.module.css` only when Tailwind insufficient.
- Config/data files: TOML for config, YAML for collection export, JSON for Postman compat.
- Git: Conventional Commits. `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `ci:`, `chore:`.

---

## 9. Out of Scope (Explicit Exclusions)

- **Cloud sync / team collaboration** — design for Git-based sync instead.
- **API documentation generation** — this is a client, not a doc tool.
- **Full load testing** — keep it lightweight; point users to k6/Locust for serious load testing.
- **Browser extension** — out of scope for v1.
- **Mobile apps** — out of scope for v1.

---

## 10. Success Criteria

When complete, a developer should be able to:

1. Open Crusty and immediately send a GET request in under 3 seconds.
2. Build a complex OAuth2-authenticated, multi-step API workflow with variable chaining across requests.
3. Debug a WebSocket connection with message filtering and auto-reconnect.
4. Run a full collection of 200 requests with parameterized data from a CSV and get a JUnit report for CI.
5. Switch between desktop, browser, and terminal with the same collections and environments synced via files.
6. Never once think "I wish this had feature X from Postman."

Build this tool like your own API debugging career depends on it.
