---
title: "SDKs"
description: "Service-client SDKs for cloacina-server — Rust, Python, and TypeScript"
weight: 35
aliases:
  - "/sdks/"

---

# Service SDKs

Run cloacina-server as a managed orchestration service and call into it from your own backend, scripts, or UI. Three first-party SDKs are generated from (and version-locked to) the server's [OpenAPI contract](/openapi.json):

| SDK | Package | Install |
|---|---|---|
| [Rust](rust/) | `cloacina-client` on crates.io | `cargo add cloacina-client` |
| [Python](python/) | `cloacina-client` on PyPI | `pip install cloacina-client` |
| [TypeScript](typescript/) | `@cloacina/client` on npm | `npm install @cloacina/client` |

**Version lockstep:** SDK `X.Y.Z` is generated from, contract-tested against, and only supported on `cloacina-server X.Y.Z`. There is no independent SDK release cadence.

**Don't confuse the consumption modes.** These SDKs are *service clients* — they talk to a running server over HTTP/WebSocket. Embedding the workflow engine in your process is a different mode: the `cloacina` Rust crate or the `cloaca` Python package.

## Shared concepts

- **Auth** is an API key sent as `Authorization: Bearer <key>`; tenant scope rides the key and the URL path. WebSocket connections never carry the long-lived key — every SDK mints a single-use, 60-second ticket (`POST /v1/auth/ws-ticket`) per connection.
- **Errors** follow one envelope: `{"error": "<human message>", "code": "<machine code>"}`. Each SDK surfaces both fields on its typed error.
- **Lists** are paged `{items, total}` envelopes; each SDK ships a pagination iterator.
- **Live events** stream over the [substrate delivery WebSocket](/reference/websocket-protocol/) with at-least-once semantics — every SDK's wrapper handles dedup, acks, and reconnection for you.
- **The contract is enforced**, not aspirational: every endpoint and WS lifecycle is exercised against a live server in CI (`angreal test sdk-contract`), and generated code is diffed against the committed spec on every PR.
