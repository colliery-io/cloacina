---
title: "Quick Start"
description: "Get a cloacina-server running locally and hit it with the CLI in about five minutes."
weight: 5
aliases:
  - "/platform/quick-start/"

---

# Quick Start (Service)

Stand up a `cloacina-server` against a local SQLite database, capture the
bootstrap admin key, and confirm it answers. This is the fast path; the
**[full tutorial]({{< ref "/service/tutorials/01-deploy-a-server" >}})** walks the
same ground in detail and adds tenants, packages, and executions.

You'll need the `cloacinactl` and `cloacina-server` binaries on your `PATH`, plus
`curl` (and optionally `jq`).

## 1. Start the server

SQLite needs no setup — the server creates the file on first start:

```bash
export DATABASE_URL='sqlite:///tmp/cloacina-quickstart.db'

cloacinactl server start --bind 127.0.0.1:8080 --database-url "$DATABASE_URL"
```

The server runs in the foreground. Open a second terminal for the next steps.

## 2. Capture the bootstrap admin key

On first startup with no API keys, the server writes a generated admin key to
`~/.cloacina/bootstrap-key`. **This is the only time the plaintext is surfaced** —
in a real deployment, move it into your secret manager immediately.

```bash
ADMIN_KEY=$(cat ~/.cloacina/bootstrap-key)
echo "Admin key captured: ${ADMIN_KEY:0:8}..."
```

## 3. Confirm it's up

```bash
curl -s http://127.0.0.1:8080/health | jq .
# {"status": "ok"}

curl -s http://127.0.0.1:8080/ready | jq .
# {"status": "ready"}
```

You now have a running server answering on `127.0.0.1:8080` with an admin key in
hand. That's the door open.

## Next

- **[01 — Deploy a Server]({{< ref "/service/tutorials/01-deploy-a-server" >}})** —
  the full first tutorial: create a tenant, upload a `.cloacina` package, run an
  execution, and verify it.
- **[Service tutorials]({{< ref "/service/tutorials" >}})** — build up from here, step by step.
- **[Engine & Primitives]({{< ref "/engine" >}})** — what a Workflow, Package, and Runner actually are.
