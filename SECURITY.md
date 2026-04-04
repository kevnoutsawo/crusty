# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Crusty, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email **security@crusty.dev** with:

1. A description of the vulnerability
2. Steps to reproduce the issue
3. The potential impact
4. Any suggested fixes (optional)

## What to Expect

- **Acknowledgement** within 48 hours of your report
- **Status update** within 7 days with an assessment and estimated timeline
- **Credit** in the release notes (unless you prefer to remain anonymous)

## Scope

The following are in scope:

- The `crusty-tui` binary and all workspace crates
- The `crusty-proxy` MITM TLS interception (by design, but misuse vectors are relevant)
- SQLite storage (`crusty-store`) — local data integrity and injection risks
- Script execution (`crusty-scripting`) — sandbox escapes or unintended side effects
- Import/export (`crusty-export`) — malicious input handling (crafted Postman collections, cURL commands, HAR files)

## Security Design Decisions

- **No network telemetry.** Crusty never phones home.
- **No accounts or cloud sync.** All data stays local.
- **rustls over OpenSSL.** Reduced attack surface for TLS.
- **Bundled SQLite.** No external database server dependency.
- **Rhai scripting sandbox.** Scripts cannot access the filesystem or network directly.
