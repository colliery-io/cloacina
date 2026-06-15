---
title: "Require signed packages"
description: "Turn on fail-closed package-signature enforcement on cloacina-server: --require-signatures + --verification-org-id, the audit trail, and recovery if you lock yourself out."
weight: 81
---

# Require signed packages

This recipe walks through enabling fail-closed package-signature verification on a running `cloacina-server`. Once configured, the server rejects every package upload that isn't signed by a trusted key for the configured verification org. The path is fail-fast (server refuses to start if misconfigured) and fail-closed (no soft-fail; failed verification = upload rejected).

For the rationale (what threat model this protects against, what it doesn't), see [Security Model]({{< ref "/service/explanation/security-model" >}}#package-signature-verification).

## Prerequisites

- A running `cloacina-server` (PostgreSQL backend; SQLite deployments do not have the trust-key DAL).
- An `is_admin` API key for that server.
- A signing key (Ed25519). If you don't have one yet, see [Package signing]({{< ref "security/package-signing" >}}) for the library-side mechanics of generating one.
- An organization UUID (`verification_org_id`) that you'll use as the trust root. Generate one with `uuidgen` or pick a stable existing identifier from your secrets system.

## Background

Two server flags work together (CLOACI-I-0103):

| Flag | Env var | Purpose |
|---|---|---|
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | Toggle. When set, every package upload runs through cryptographic signature verification before registration. |
| `--verification-org-id <UUID>` | `CLOACINA_VERIFICATION_ORG_ID` | Trust root. The server only accepts signatures from public keys registered under this org. |

The pair is **fail-fast**: if `--require-signatures` is set without `--verification-org-id`, the server refuses to start (logs `signature_verification_unconfigured` and exits). There is no soft-fail path — a misconfigured pipeline means no uploads, not "uploads accepted unverified."

Verification runs at the canonical load path (the upload route, the reconciler, and any future ingest paths), so every code path that registers a package goes through the same check.

## Steps

### 1. Pick a verification org ID

```sh
export VERIFICATION_ORG_ID="$(uuidgen)"
echo "$VERIFICATION_ORG_ID"
# e.g. 12345678-1234-1234-1234-123456789abc
```

Store this somewhere durable — it has to match across server restarts. Treat it as a configuration constant, not a secret (it's an opaque ID; the actual trust is in the key registered against it).

### 2. Generate (or import) the first trusted public key

The trusted-key DAL stores `(org_id, public_key)` rows. Adding a key today goes through the library-side `SecurityConfig` API; the CLI surface for trust-key management is on the roadmap but not yet shipped.

For a fresh deployment, the bootstrap path is to run a one-shot Rust program that uses `cloacina::security::DbKeyManager` to register the first key against the org. See [Package signing]({{< ref "security/package-signing" >}}) for the API surface.

### 3. Start the server with both flags

```sh
cloacinactl server stop   # if currently running
cloacinactl server start \
  --bind 127.0.0.1:8080 \
  --database-url "$DATABASE_URL" \
  --require-signatures \
  --verification-org-id "$VERIFICATION_ORG_ID"
```

Or via env vars (recommended for systemd / container deployments):

```sh
export CLOACINA_REQUIRE_SIGNATURES=true
export CLOACINA_VERIFICATION_ORG_ID="$VERIFICATION_ORG_ID"
cloacinactl server start --bind 127.0.0.1:8080 --database-url "$DATABASE_URL"
```

The server's startup banner now mentions the verification org. If you misconfigured (set `--require-signatures` without `--verification-org-id`), the server logs `signature_verification_unconfigured` and exits non-zero before binding the listen socket.

### 4. Sign your packages before uploading

> **Current limitation (2026-05):** `cloacinactl package pack --sign <key>` is a **fail-hard stub** — it accepts the flag and returns an error pointing operators at the library-side signing API. The CLI wire-up is tracked as a follow-up to CLOACI-I-0103. Until it lands, signing is done programmatically via `cloacina::security::package_signer` or out-of-band (e.g., as part of your CI pipeline using the library API in a small Rust program). See [Package signing]({{< ref "security/package-signing" >}}) for the signing recipe.

Once you have a signed package and its `.sig` sidecar, upload as normal:

```sh
cloacinactl --profile prod package upload my-workflow.cloacina
```

The server fetches the signature row from the trust DB, verifies the bytes against the registered public key for the configured org, and registers the package only on success.

### 5. Verify enforcement is working

**Successful upload:**

```sh
cloacinactl --profile prod package upload signed-workflow.cloacina
# expect: 201 Created + package metadata
```

**Failed upload (unsigned package):**

```sh
cloacinactl --profile prod package upload unsigned-workflow.cloacina
# expect: HTTP 403 with code "signature_not_found"
```

**Failed upload (signed by untrusted key):**

```sh
cloacinactl --profile prod package upload wrong-key-workflow.cloacina
# expect: HTTP 403 with code "invalid_signature"
```

The full error-code catalog for signature failures is in [API Error Envelope]({{< ref "/reference/api-error-envelope" >}}#403-forbidden).

### 6. Check the audit log

Every verification attempt (successful or failed) is logged to the structured audit log. Tail the server logs:

```sh
journalctl -u cloacina-server --since "10 minutes ago" \
  | grep -E "package_(verified|verification_failed)"
```

Successful uploads emit `package_verified` with the org id, package fingerprint, and verifying key id. Failures emit `package_verification_failed` with the same context plus the failure reason (`invalid_signature`, `signature_not_found`, etc.).

## Recovery: "I locked myself out by enabling signatures with no trusted keys"

**Symptom:** Every package upload returns `403 signature_not_found` because no keys are registered against the verification org.

**Recovery options, in order of safety:**

1. **Register a trusted key without restarting.** The trusted-key DAL is hot — registering a new `(org_id, public_key)` row makes the key available for subsequent uploads immediately. Run a one-shot Rust program against the same database that calls `DbKeyManager::add_trusted_key(org_id, public_key)`. Subsequent uploads with packages signed by that key succeed.

2. **Restart with `--require-signatures` off** (emergency only). Removes enforcement entirely. After register a key, re-enable. **Do not leave the server running without enforcement in production.**

```sh
cloacinactl server stop
unset CLOACINA_REQUIRE_SIGNATURES
cloacinactl server start --bind ... --database-url "$DATABASE_URL"
# register the key, then:
cloacinactl server stop
export CLOACINA_REQUIRE_SIGNATURES=true
export CLOACINA_VERIFICATION_ORG_ID="$VERIFICATION_ORG_ID"
cloacinactl server start ...
```

## What this how-to does NOT cover

- **The library-side signing API.** See [Package signing]({{< ref "security/package-signing" >}}).
- **Key rotation.** The trusted-key DAL supports multiple keys per org. Add a new key, deprecate the old one in your signing pipeline, then revoke the old key from the trust DB. The mechanics live in the library API.
- **Per-tenant signing requirements.** Signature enforcement is server-wide, not per-tenant. If you need different policies for different tenants, run separate `cloacina-server` instances.
- **Compiler-side signing.** The `cloacina-compiler` build worker does not currently sign the artifacts it produces; signatures are applied by the package author before upload.

## See also

- [Security Model]({{< ref "/service/explanation/security-model" >}}#package-signature-verification) — rationale and threat model.
- [Package signing]({{< ref "security/package-signing" >}}) — library-side API for generating signatures.
- [Security: local development]({{< ref "security/local-development" >}}) — iterating on a signing pipeline locally.
- [API Error Envelope]({{< ref "/reference/api-error-envelope" >}}#403-forbidden) — full enumeration of signature-related error codes.
- [HTTP API Reference]({{< ref "/reference/http-api" >}}#post-v1tenantstenant_idworkflows) — the upload route.
- **CLOACI-I-0103** — Wire signature verification on the server upload + load path (opt-in).
- **CLOACI-T-0567** — `--verification-org-id` fail-fast wiring.
