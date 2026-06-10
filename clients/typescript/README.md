# @cloacina/client

TypeScript SDK for [cloacina-server](https://github.com/colliery-io/cloacina) — a typed REST client plus WebSocket execution-event streaming, for Node ≥ 20 and modern browsers.

**Version lockstep:** `@cloacina/client@X.Y.Z` is generated from, tested against, and only supported on `cloacina-server X.Y.Z`.

## Install

```bash
npm install @cloacina/client
```

## Quickstart

```ts
import { CloacinaClient, followExecutionEvents } from "@cloacina/client";

const client = new CloacinaClient({
  baseUrl: "http://localhost:8080",
  apiKey: process.env.CLOACINA_API_KEY!,
  tenant: "public",
});

// REST
const { execution_id } = await client.executeWorkflow("my_workflow", {
  context: { input: 42 },
});

// Live execution events over WebSocket (reconnect + ack handled for you)
for await (const event of followExecutionEvents(client, execution_id)) {
  console.log(event);
}
```

Every helper throws `CloacinaApiError` (with `status` and the server's machine-readable `code`) on non-2xx responses. The typed low-level [`openapi-fetch`](https://openapi-ts.dev/openapi-fetch/) client is available as `client.api` for anything the helpers don't cover.

## Pagination

```ts
for await (const execution of client.iterateExecutions({ status: "Failed" })) {
  console.log(execution.id, execution.workflow_name);
}
```

## Browser usage and auth (read this)

The server must opt in to CORS (`--cors-allowed-origins` / `CLOACINA_CORS_ALLOWED_ORIGINS`) before any browser can call it.

**API-key-in-browser is the deliberate v1 auth story for first-party admin UIs only.** The key is fully visible to whoever can open devtools — never embed one in a page served to untrusted users. Browser-grade auth (sessions/OIDC) is planned for the UI initiative and is out of scope for this SDK. WebSocket connections never carry the long-lived key: the SDK mints a single-use, 60-second ticket (`POST /v1/auth/ws-ticket`) per connection.

## WebSocket notes

- `subscribeDelivery` / `followExecutionEvents` use the platform `WebSocket` — native in browsers and Node ≥ 21. On Node 20, pass an implementation: `followExecutionEvents(client, id, { webSocket: require("ws").WebSocket })`.
- Delivery is at-least-once; the SDK dedups on row id and acks each frame after yielding it.
- Reconnects use exponential backoff (100 ms → 30 s). A `4426` close (protocol version mismatch) is terminal: upgrade the SDK.

## Regenerating

`generated/types.ts` is produced by `openapi-typescript` (pinned in `devDependencies`) from the committed server contract:

```bash
npm run generate    # regenerate from docs/static/openapi.json
npm run check:generated  # CI drift gate
```

## Contract tests

`npm run test:contract` exercises every documented endpoint plus the WebSocket lifecycle against a live server:

```bash
CLOACINA_SERVER_URL=http://localhost:8080 \
CLOACINA_API_KEY=<bootstrap-key> \
npm run test:contract
```

The full execute→event-stream flow (which needs a compiled `.cloacina` package) runs in the repo's `angreal test sdk-contract` harness.
