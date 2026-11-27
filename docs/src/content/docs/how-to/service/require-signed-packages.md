---
title: "Require signed packages"
description: "Turn on fail-closed package-signature enforcement on cloacina-server: --require-signatures, the trusted keys of each tenant, the optional server-wide org, the audit trail, and recovery if you lock yourself out."
sidebar:
  order: 81
---

This recipe walks through enabling fail-closed package-signature verification on a running `cloacina-server`. Once configured, the server rejects every package upload that isn't signed by a key its tenant trusts. The path is fail-closed (no soft-fail; failed verification = upload rejected).

For the rationale (what threat model this protects against, what it doesn't), see [Security Model](../../../explanation/service/security-model/#package-signature-verification).

## Prerequisites

- A running `cloacina-server` (PostgreSQL backend; SQLite deployments do not have the trust-key DAL).
- An `admin` API key of each tenant that uploads, or a platform admin key.
- A signing key (Ed25519). If you don't have one yet, see [Package signing](../security/package-signing/#sign-and-publish-a-package-with-cloacinactl) for how to make one.

## Background

| Flag | Env var | Purpose |
|---|---|---|
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | Toggle. When set, every package upload runs through cryptographic signature verification before registration. |
| `--verification-org-id <UUID>` | `CLOACINA_VERIFICATION_ORG_ID` | Optional. The server-wide org: a key trusted in it is accepted for every tenant. |

There is no soft-fail path: an upload with no signature, or signed by a key that is not trusted, is refused.

### How an upload is checked

The signature comes with the archive: it is the `signature` field of the upload to `/v1/tenants/{tenant_id}/workflows`, beside the `file` field. It is a detached signature, the content of the `.sig` file that `cloacinactl package pack --sign` writes: Ed25519 over the SHA-256 of the archive. `cloacinactl package publish --sign` and `package upload` send it.

The server checks the signature against the bytes of the same request. Then it looks the signer up in this order:

1. **The trusted keys of the tenant in the path.** The tenant `admin` manages them with `cloacinactl trusted-key`.
2. **The trusted keys of the server-wide org**, if `--verification-org-id` is set. The operator manages them.

The first list that holds the signer's key, live, decides. Each upload that does not pass has its own answer:

| Answer | The upload |
|---|---|
| `403 signature_required` | Has no `signature` field, and the server requires signatures. |
| `400 malformed_signature` | Has a `signature` field that is not a detached signature, or one in a format the server does not know. |
| `403 package_tampered` | Has a signature that was made for other bytes. |
| `403 untrusted_signer` | Is signed by a key that neither list holds. A key that only another tenant trusts gets this answer too. |
| `403 signer_revoked` | Is signed by a key that a list holds revoked, and no list holds live. |
| `403 invalid_signature` | Names a trusted key, and the signature does not verify with it. |

What follows from the order:

- A package signed for one tenant is not accepted by another tenant, unless the signer's key is also in the server-wide org.
- A key in the server-wide org is trusted for every tenant. For strict per-tenant trust, set no `--verification-org-id`.
- `--require-signatures` with no `--verification-org-id` is a valid posture: each tenant trusts only its own keys, and a tenant with no key can upload nothing.

A signature that is sent to a server without `--require-signatures` is checked in the same way, and a bad one is refused. An upload with no signature passes there.

### Where the signature is stored

The server stores the signature of an accepted upload in the tenant's own schema, in `package_signatures`, beside the package. The row's `org_id` is the org whose key list held the signer: the tenant's org, or the server-wide org. Two tenants that upload the same bytes each need a signature and each keep their own row. A second upload of the same bytes by the same key replaces the row.

### How a package is checked at load

With `--require-signatures`, the server verifies each package again when it loads it into a tenant: at the start of the server, and when a new version is built. The package loads when a signature row for its bytes is stored for the tenant's org or the server-wide org, the signer is a live trusted key of one of the two (the tenant's org first), and the signature verifies. A package that does not pass is not loaded, and the server log and the audit log say why (`package.load.failure` with `missing_signature`, `untrusted_signer`, `revoked_signer` or `invalid_signature`).

What follows:

- A package that reached the tenant's tables without the upload route is not loaded.
- **A revoke does not unload a package.** A package that is loaded keeps running after its signer's key is revoked. The next load refuses it: after the next start of the server it is not loaded. Each replica loads on its own start, so a replica that starts after the revoke does not load the package while older replicas still run it. To stop it at once, delete the package. To keep it, publish it again signed by a trusted key before the next start.
- **Packages uploaded before `--require-signatures` was on** have no signature row unless they were uploaded with a signature. At the first start with `--require-signatures` they are not loaded. Publish them signed first: a server without `--require-signatures` checks and stores a signature that is sent.

## Steps

### 1. Add the first trusted key of each tenant

Trusted keys can be added before enforcement is on. As the tenant `admin` (or as a platform admin, with `--tenant <tenant>`):

```sh
cloacinactl --profile prod trusted-key add --pem-file release-key.pub.pem --name release-key-v1
cloacinactl --profile prod trusted-key list
```

`add` takes a PEM file (`--pem-file`) or the raw 32-byte public key as 64 hexadecimal characters (`--public-key`). See [Package signing](../security/package-signing/#manage-a-tenants-trusted-keys-on-a-server).

### 2. Optional: a server-wide org

To accept one key set for every tenant (a platform release key), pick an org id and trust the keys in it:

```sh
export VERIFICATION_ORG_ID="$(uuidgen)"
echo "$VERIFICATION_ORG_ID"
# e.g. 12345678-1234-1234-1234-123456789abc
```

Store this somewhere durable — it has to match across server restarts. Treat it as a configuration constant, not a secret (it's an opaque ID; the actual trust is in the keys registered against it).

No route and no `cloacinactl` command manages the keys of the server-wide org. Register them with a one-shot Rust program that calls `cloacina::security::DbKeyManager::trust_public_key(org_id, …)` against the server's database. See [Package signing](../security/package-signing/) for the API surface.

Skip this step for strict per-tenant trust.

### 3. Start the server with signatures required

```sh
cloacinactl server stop   # if currently running
cloacinactl server start \
  --bind 127.0.0.1:8080 \
  --database-url "$DATABASE_URL" \
  --require-signatures
```

Add `--verification-org-id "$VERIFICATION_ORG_ID"` if you set up a server-wide org in step 2.

Or via env vars (recommended for systemd / container deployments):

```sh
export CLOACINA_REQUIRE_SIGNATURES=true
export CLOACINA_VERIFICATION_ORG_ID="$VERIFICATION_ORG_ID"   # only with a server-wide org
cloacinactl server start --bind 127.0.0.1:8080 --database-url "$DATABASE_URL"
```

### 4. Sign your packages when you publish them

Sign with the private key of a key pair whose public key the tenant trusts. The key is a file on the publisher's machine; the server never gets it.

```sh
cloacinactl --profile prod package publish ./my-workflow --sign release-key.pem
```

Or sign when you pack, and upload later. `pack --sign` writes `my-workflow.cloacina.sig` beside the archive, and `upload` sends it:

```sh
cloacinactl package pack ./my-workflow --out my-workflow.cloacina --sign release-key.pem
cloacinactl --profile prod package upload my-workflow.cloacina
```

The server checks the signature against the uploaded bytes, looks the signer up in the tenant's trusted keys and then in the server-wide org, stores the signature, and registers the package only on success. See [Package signing](../security/package-signing/#sign-and-publish-a-package-with-cloacinactl) for the key file.

### 5. Verify enforcement is working

**Successful upload:**

```sh
cloacinactl --profile prod package upload signed-workflow.cloacina
# expect: 201 Created + package metadata
```

**Failed upload (an archive with no signature file beside it):**

```sh
cloacinactl --profile prod package upload unsigned-workflow.cloacina
# expect: HTTP 403 with code "signature_required"
```

**Failed upload (signed by a key the tenant does not trust):**

```sh
cloacinactl --profile prod package upload wrong-key-workflow.cloacina
# expect: HTTP 403 with code "untrusted_signer"
```

**Failed upload (signed by a key the tenant revoked):**

```sh
cloacinactl --profile prod package upload revoked-key-workflow.cloacina
# expect: HTTP 403 with code "signer_revoked"
```

The full error-code catalog for signature failures is in [API Error Envelope](../../../reference/api-error-envelope/#403-forbidden).

### 6. Check the audit log

Every verification attempt (successful or failed) is logged to the structured audit log. Tail the server logs:

```sh
journalctl -u cloacina-server --since "10 minutes ago" \
  | grep -E "package\.load\.(success|failure)"
```

Successful uploads emit a `package.load.success` audit event with the package path context and the id of the org whose key list held the signer (the tenant's org, or the server-wide org). Failures emit `package.load.failure` with the same context plus the failure reason (`invalid_signature`, `signature_required`, etc.). The load check of a tenant's runner writes the same two events, with the package name and version as the context. The `package.upload` row of the tenant's [audit record](../../../reference/http-api/#audit) carries `signer_fingerprint` and `signer_trusted_by_org` for a signed upload. The library-level verification calls additionally emit `verification.success` / `verification.failure` events — the full event catalog is in the [Package Signing API Reference](../../../reference/package-signing-api/).

## Recovery: "I locked myself out by enabling signatures with no trusted keys"

**Symptom:** Every package upload of a tenant returns `403 untrusted_signer` because the tenant trusts no key (or `403 signature_required` because the upload has no signature).

**Recovery options, in order of safety:**

1. **Add a trusted key without restarting.** The key lists are read at every upload. As the tenant `admin` or a platform admin:

   ```sh
   cloacinactl --profile prod trusted-key add --pem-file release-key.pub.pem --name release-key-v1
   ```

   Subsequent uploads of packages signed by that key succeed.

2. **Restart with `--require-signatures` off** (emergency only). Removes enforcement entirely. Add a key, then re-enable. **Do not leave the server running without enforcement in production.**

```sh
cloacinactl server stop
unset CLOACINA_REQUIRE_SIGNATURES
cloacinactl server start --bind ... --database-url "$DATABASE_URL"
# add the key, then:
cloacinactl server stop
export CLOACINA_REQUIRE_SIGNATURES=true
cloacinactl server start ...
```

## What this how-to does NOT cover

- **The key file and the library-side signing API.** See [Package signing](../security/package-signing/).
- **Key rotation.** A tenant can trust several keys. Add the new key (`cloacinactl trusted-key add`), move your signing pipeline to it, then revoke the old key (`cloacinactl trusted-key revoke`). See [Package signing](../security/package-signing/#rotate-signing-keys).
- **Per-tenant signing requirements.** The trusted keys are per tenant; the requirement itself (`--require-signatures`) is server-wide. If some tenants must upload unsigned packages, run separate `cloacina-server` instances.
- **Upload from the web UI.** The upload form of the web UI sends no signature. On a server with `--require-signatures` it is refused with `signature_required`: publish with `cloacinactl` or a client library.
- **Compiler-side signing.** The `cloacina-compiler` build worker does not currently sign the artifacts it produces; signatures are applied by the package author before upload.

## See also

- [Security Model](../../../explanation/service/security-model/#package-signature-verification) — rationale and threat model.
- [Package signing](../security/package-signing/) — signing with `cloacinactl`, and the library-side API.
- [Security: local development](../security/local-development/) — iterating on a signing pipeline locally.
- [API Error Envelope](../../../reference/api-error-envelope/#403-forbidden) — full enumeration of signature-related error codes.
- [HTTP API Reference](../../../reference/http-api/#post-v1tenantstenant_idworkflows) — the upload route.
- **CLOACI-I-0103** — Wire signature verification on the server upload + load path (opt-in).
- **CLOACINA-A-0018** — package signing trust is per tenant: the tenant's org first, then the server-wide org.
