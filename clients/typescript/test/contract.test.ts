/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

/**
 * Live-server contract suite (CLOACI-I-0113 / REQ-007).
 *
 * Exercises the GENERATED client against a real cloacina-server — this is
 * the drift detector between utoipa annotations and handler behavior, not
 * a spec-vs-spec diff. Every documented endpoint is hit at least once;
 * endpoints whose success path needs out-of-band fixtures (a compiled
 * .cloacina package) assert their documented error contract instead, and
 * the full execute→push flow is covered by the seeded harness in
 * `angreal test sdk-contract` (T-0648).
 *
 * Requires:
 *   CLOACINA_SERVER_URL  e.g. http://localhost:8080
 *   CLOACINA_API_KEY     a god-mode (bootstrap) key
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  CloacinaApiError,
  CloacinaClient,
  DELIVERY_PROTOCOL_VERSION,
} from "../src/index.js";

const SERVER = process.env.CLOACINA_SERVER_URL;
const API_KEY = process.env.CLOACINA_API_KEY;
const RUN = SERVER !== undefined && API_KEY !== undefined;

const TENANT = `sdk_ts_contract_${Date.now()}`;

const RANDOM_UUID = "00000000-0000-4000-8000-000000000000";

describe.skipIf(!RUN)("TS SDK live-server contract", () => {
  let client: CloacinaClient;
  let createdKeyId: string;

  beforeAll(async () => {
    client = new CloacinaClient({
      baseUrl: SERVER as string,
      apiKey: API_KEY as string,
      tenant: TENANT,
    });
    await client.createTenant({ name: TENANT });
  });

  afterAll(async () => {
    try {
      await client.removeTenant(TENANT);
    } catch {
      // best-effort cleanup
    }
  });

  // ---- operational ----

  it("GET /health", async () => {
    const body = (await client.health()) as { status: string };
    expect(body.status).toBe("ok");
  });

  it("GET /ready", async () => {
    const resp = await client.ready();
    expect([200, 503]).toContain(resp.status);
  });

  it("GET /openapi.json serves the contract", async () => {
    const resp = await fetch(`${SERVER}/openapi.json`);
    expect(resp.status).toBe(200);
    const doc = (await resp.json()) as { openapi: string };
    expect(doc.openapi).toMatch(/^3\.1/);
  });

  // ---- keys ----

  it("POST /v1/auth/keys creates a key with one-time plaintext", async () => {
    const created = await client.createKey({
      name: `ts-contract-${Date.now()}`,
      role: "read",
    });
    expect(created.key).toBeTruthy();
    expect(created.permissions).toBe("read");
    createdKeyId = created.id;
  });

  it("GET /v1/auth/keys lists keys without plaintext", async () => {
    const page = await client.listKeys();
    expect(page.total).toBeGreaterThan(0);
    const mine = page.items.find((k) => k.id === createdKeyId);
    expect(mine).toBeDefined();
    expect((mine as unknown as Record<string, unknown>).key).toBeUndefined();
  });

  it("DELETE /v1/auth/keys/{key_id} revokes", async () => {
    const revoked = await client.revokeKey(createdKeyId);
    expect(revoked.status).toBe("revoked");
  });

  it("POST /v1/tenants/{tenant_id}/keys creates a tenant-scoped key", async () => {
    const created = await client.createTenantKey({
      name: `ts-contract-tenant-${Date.now()}`,
      role: "write",
    });
    expect(created.tenant_id).toBe(TENANT);
    await client.revokeKey(created.id);
  });

  it("POST /v1/auth/ws-ticket mints a single-use ticket", async () => {
    const ticket = await client.createWsTicket();
    expect(ticket.ticket).toBeTruthy();
    expect(ticket.expires_in_seconds).toBeGreaterThan(0);
  });

  // ---- tenants ----

  it("GET /v1/tenants lists the created tenant", async () => {
    const page = await client.listTenants();
    expect(page.items.map((t) => t.name)).toContain(TENANT);
  });

  // (create/remove are exercised in beforeAll/afterAll)

  // ---- audit record (CLOACINA-T-1024) ----

  it("GET /v1/tenants/{tenant_id}/audit has the row of the tenant create", async () => {
    const page = await client.listTenantAudit({ action: "tenant.create" });
    expect(page.total).toBe(page.items.length);
    const row = page.items.find((r) => r.target === TENANT);
    expect(row).toBeDefined();
    expect(row?.tenant_id).toBe(TENANT);
    expect(row?.outcome).toBe("ok");
    expect(row?.actor_key_id).toBeTruthy();
  });

  it("GET /v1/audit filters by tenant and pages with the cursor", async () => {
    const page = await client.listAudit({ tenant: TENANT, limit: 1 });
    expect(page.items).toHaveLength(1);
    expect(page.items[0].tenant_id).toBe(TENANT);
    expect(page.next_cursor).toBe(page.items[0].id);

    const older = await client.listAudit({
      tenant: TENANT,
      limit: 1,
      cursor: page.next_cursor as number,
    });
    for (const row of older.items) {
      expect(row.id).toBeLessThan(page.items[0].id);
    }

    const bad = await client.listAudit({ limit: 0 }).catch((e) => e);
    expect(bad).toBeInstanceOf(CloacinaApiError);
    expect((bad as CloacinaApiError).status).toBe(400);
  });

  // ---- secrets (CLOACI-I-0133 / T-0862) ----

  it("secrets lifecycle: create → list → get → rotate → delete, metadata only", async () => {
    const name = `ts-contract-secret-${Date.now()}`;
    const created = await client.createSecret({
      name,
      fields: { token: "s3cr3t" },
    });
    expect(created.name).toBe(name);
    expect(created.field_names).toEqual(["token"]);
    // Metadata-only contract: no value ever comes back on a read.
    expect(JSON.stringify(created)).not.toContain("s3cr3t");

    const page = await client.listSecrets();
    expect(page.items.map((s) => s.name)).toContain(name);

    const fetched = await client.getSecret(name);
    expect(fetched.id).toBe(created.id);

    const rotated = await client.rotateSecret(name, {
      fields: { token: "rotated", extra: "field" },
    });
    expect(rotated.field_names).toHaveLength(2);

    const missing = await client.getSecret("does-not-exist").catch((e) => e);
    expect(missing).toBeInstanceOf(CloacinaApiError);
    expect((missing as CloacinaApiError).status).toBe(404);

    const deleted = await client.deleteSecret(name);
    expect(deleted.status).toBe("deleted");
    expect(deleted.name).toBe(name);
  });

  it("trusted keys lifecycle: add → list → revoke, a revoked key stays listed", async () => {
    const publicKey = Array.from(crypto.getRandomValues(new Uint8Array(32)))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    const name = `ts-contract-key-${Date.now()}`;

    const added = await client.addTrustedKey({ name, public_key: publicKey });
    expect(added.name).toBe(name);
    expect(added.fingerprint).toHaveLength(64);
    expect(added.revoked).toBe(false);

    const page = await client.listTrustedKeys();
    expect(page.items.map((k) => k.id)).toContain(added.id);

    // The tenant holds a key once.
    const again = await client
      .addTrustedKey({ public_key: publicKey })
      .catch((e) => e);
    expect(again).toBeInstanceOf(CloacinaApiError);
    expect((again as CloacinaApiError).status).toBe(409);

    const revoked = await client.revokeTrustedKey(added.id);
    expect(revoked.status).toBe("revoked");
    expect(revoked.id).toBe(added.id);

    const after = await client.listTrustedKeys();
    expect(after.items.find((k) => k.id === added.id)?.revoked).toBe(true);

    const missing = await client
      .revokeTrustedKey("00000000-0000-4000-8000-000000000000")
      .catch((e) => e);
    expect(missing).toBeInstanceOf(CloacinaApiError);
    expect((missing as CloacinaApiError).status).toBe(404);
  });

  // ---- agent pools (CLOACINA-T-1021) ----

  it("agent pool lifecycle: list → put → get → replace → delete", async () => {
    const pool = `ts-${Date.now().toString(36)}`;

    // Every tenant has the pool `default`.
    const before = await client.listPools();
    expect(before.items.map((p) => p.pool)).toContain("default");

    const created = await client.putPool(pool, {
      memory_limit: "2Gi",
      max_concurrency: 1,
      pod_labels: { team: "data" },
      max_agents: 0,
    });
    expect(created.pool).toBe(pool);
    expect(created.settings.memory_limit).toBe("2Gi");
    expect(created.desired_agents).toBe(0);

    const fetched = await client.getPool(pool);
    expect(fetched.settings.max_concurrency).toBe(1);

    const page = await client.listPools();
    expect(page.items.map((p) => p.pool)).toContain(pool);
    expect(page.total).toBe(page.items.length);

    // PUT replaces the settings as a whole.
    const replaced = await client.putPool(pool, { max_agents: 0 });
    expect(replaced.settings.memory_limit).toBeUndefined();

    // A malformed quantity is a 400 at the API.
    const bad = await client
      .putPool(pool, { memory_limit: "2GB", max_agents: 0 })
      .catch((e) => e);
    expect(bad).toBeInstanceOf(CloacinaApiError);
    expect((bad as CloacinaApiError).status).toBe(400);

    await client.deletePool(pool);
    const gone = await client.getPool(pool).catch((e) => e);
    expect(gone).toBeInstanceOf(CloacinaApiError);
    expect((gone as CloacinaApiError).status).toBe(404);
  });

  // ---- backfills (CLOACINA-T-1020) ----

  it("backfill lifecycle: create → list → inspect → retry (409) → cancel", async () => {
    const workflow = `ts_backfill_${Date.now().toString(36)}`;

    // A range in the future: the scheduler takes none of its intervals, so
    // the progress is the same on each read.
    const created = await client.createBackfill(workflow, {
      cron: "0 * * * *",
      start: "2099-01-01T00:00:00Z",
      end: "2099-01-01T04:00:00Z",
      max_active_runs: 2,
      params: { region: "eu" },
    });
    expect(created.workflow_name).toBe(workflow);
    expect(created.status).toBe("running");
    expect(created.max_active_runs).toBe(2);
    expect(created.progress.total).toBe(5);
    expect(created.progress.remaining).toBe(5);

    const page = await client.listBackfills(workflow, { limit: 10, offset: 0 });
    expect(page.items.map((b) => b.id)).toContain(created.id);

    const detail = await client.getBackfill(workflow, created.id);
    expect(detail.backfill.id).toBe(created.id);
    expect(detail.failed_intervals).toEqual([]);
    expect(detail.failed_intervals_truncated).toBe(false);

    // A retry needs a failed interval.
    const noRetry = await client.retryBackfill(workflow, created.id).catch((e) => e);
    expect(noRetry).toBeInstanceOf(CloacinaApiError);
    expect((noRetry as CloacinaApiError).status).toBe(409);

    const cancelled = await client.cancelBackfill(workflow, created.id);
    expect(cancelled.status).toBe("cancelled");

    // The end is before the start: a 400 at the API.
    const bad = await client
      .createBackfill(workflow, {
        cron: "0 * * * *",
        start: "2099-01-02T00:00:00Z",
        end: "2099-01-01T00:00:00Z",
      })
      .catch((e) => e);
    expect(bad).toBeInstanceOf(CloacinaApiError);
    expect((bad as CloacinaApiError).status).toBe(400);

    const gone = await client
      .getBackfill(workflow, "00000000-0000-4000-8000-000000000000")
      .catch((e) => e);
    expect(gone).toBeInstanceOf(CloacinaApiError);
    expect((gone as CloacinaApiError).status).toBe(404);
  });

  // ---- workflows ----

  it("POST /v1/tenants/{t}/workflows rejects a garbage package with the documented error shape", async () => {
    const err = await client
      .uploadWorkflow(new TextEncoder().encode("not a real package"))
      .then(() => null)
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(CloacinaApiError);
    expect((err as CloacinaApiError).status).toBe(400);
    expect((err as CloacinaApiError).code).toBeTruthy();
  });

  it("GET /v1/tenants/{t}/workflows returns the list envelope", async () => {
    const page = await client.listWorkflows();
    expect(page.tenant_id).toBe(TENANT);
    expect(Array.isArray(page.items)).toBe(true);
    expect(page.total).toBe(page.items.length);
  });

  it("GET /v1/tenants/{t}/workflows/{name} 404s for unknown workflow", async () => {
    const err = await client
      .getWorkflow("does-not-exist")
      .then(() => null)
      .catch((e: unknown) => e);
    expect((err as CloacinaApiError).status).toBe(404);
    expect((err as CloacinaApiError).code).toBe("workflow_not_found");
  });

  it("DELETE /v1/tenants/{t}/workflows/{name}/{version} is idempotent (200 even when absent)", async () => {
    // Documented contract decision (T-0645): unregister is idempotent —
    // deleting a workflow that was never registered still returns the
    // deleted envelope.
    const deleted = await client.deleteWorkflow("does-not-exist", "0.0.0");
    expect(deleted.status).toBe("deleted");
    expect(deleted.package_name).toBe("does-not-exist");
  });

  it("GET /v1/tenants/{t}/workflows/{name}/versions 404s for a package with no version", async () => {
    // CLOACINA-T-1018.
    const err = await client
      .listWorkflowVersions("does-not-exist")
      .then(() => null)
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(CloacinaApiError);
    expect((err as CloacinaApiError).status).toBe(404);
    expect((err as CloacinaApiError).code).toBe("workflow_not_found");
  });

  it("POST /v1/tenants/{t}/workflows/{name}/versions/{version}/activate 404s for an unknown version", async () => {
    const err = await client
      .activateWorkflowVersion("does-not-exist", "0.0.0")
      .then(() => null)
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(CloacinaApiError);
    expect((err as CloacinaApiError).status).toBe(404);
    expect((err as CloacinaApiError).code).toBe("workflow_version_not_found");
  });

  // ---- triggers ----

  it("GET /v1/tenants/{t}/triggers returns the paged envelope", async () => {
    const page = await client.listTriggers({ limit: 10, offset: 0 });
    expect(page.tenant_id).toBe(TENANT);
    expect(Array.isArray(page.items)).toBe(true);
  });

  it("GET /v1/tenants/{t}/triggers rejects bad pagination", async () => {
    const err = await client
      .listTriggers({ limit: 100000 })
      .then(() => null)
      .catch((e: unknown) => e);
    expect((err as CloacinaApiError).status).toBe(400);
    expect((err as CloacinaApiError).code).toBe("invalid_pagination");
  });

  it("GET /v1/tenants/{t}/triggers/{name} 404s for unknown trigger", async () => {
    const err = await client
      .getTrigger("does-not-exist")
      .then(() => null)
      .catch((e: unknown) => e);
    expect((err as CloacinaApiError).status).toBe(404);
    expect((err as CloacinaApiError).code).toBe("trigger_not_found");
  });

  // ---- executions ----

  it("POST /v1/tenants/{t}/workflows/{name}/execute rejects unknown workflow with documented error", async () => {
    // I-0138 cross-tenant gate: a workflow not registered for THIS tenant
    // fails closed with 404 (the `public` tenant is exempt).
    const err = await client
      .executeWorkflow("does-not-exist", { context: { k: "v" } })
      .then(() => null)
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(CloacinaApiError);
    expect((err as CloacinaApiError).status).toBe(404);
    expect((err as CloacinaApiError).code).toBe("workflow_not_found");
  });

  it("GET /v1/tenants/{t}/executions returns the paged envelope and honors filters", async () => {
    const page = await client.listExecutions({
      status: "Completed",
      limit: 5,
    });
    expect(page.tenant_id).toBe(TENANT);
    expect(Array.isArray(page.items)).toBe(true);
  });

  it("iterateExecutions drains pages", async () => {
    const seen: unknown[] = [];
    for await (const item of client.iterateExecutions({ limit: 2 })) {
      seen.push(item);
      if (seen.length > 10) break;
    }
    expect(Array.isArray(seen)).toBe(true);
  });

  it("GET /v1/tenants/{t}/executions/{id} validates the id", async () => {
    const err = await client
      .getExecution("not-a-uuid")
      .then(() => null)
      .catch((e: unknown) => e);
    expect((err as CloacinaApiError).status).toBe(400);
  });

  it("GET /v1/tenants/{t}/executions/{id} 404s for unknown execution", async () => {
    const err = await client
      .getExecution(RANDOM_UUID)
      .then(() => null)
      .catch((e: unknown) => e);
    expect((err as CloacinaApiError).status).toBe(404);
  });

  it("GET /v1/tenants/{t}/executions/{id}/events returns the envelope for unknown id", async () => {
    // The events endpoint returns an empty event list rather than 404 for
    // an unknown-but-valid UUID — assert the envelope shape.
    const events = await client.getExecutionEvents(RANDOM_UUID);
    expect(events.execution_id).toBe(RANDOM_UUID);
    expect(Array.isArray(events.events)).toBe(true);
  });

  // ---- computation-graph health ----

  it("GET /v1/health/accumulators returns the list envelope", async () => {
    const page = await client.listAccumulators();
    expect(Array.isArray(page.items)).toBe(true);
  });

  it("GET /v1/health/graphs returns the list envelope", async () => {
    const page = await client.listGraphs();
    expect(Array.isArray(page.items)).toBe(true);
  });

  it("GET /v1/health/graphs/{name} 404s with graph_not_found", async () => {
    const err = await client
      .getGraph("does-not-exist")
      .then(() => null)
      .catch((e: unknown) => e);
    expect((err as CloacinaApiError).status).toBe(404);
    expect((err as CloacinaApiError).code).toBe("graph_not_found");
  });

  // ---- WS subscription lifecycle ----

  it("delivery WS: welcome → hello(v1) accepted → ack idempotent", async () => {
    const { ticket } = await client.createWsTicket();
    const wsUrl = `${(SERVER as string).replace(/^http/, "ws")}/v1/ws/delivery/${encodeURIComponent(`exec_events:${RANDOM_UUID}`)}?token=${encodeURIComponent(ticket)}`;
    const socket = new WebSocket(wsUrl);

    const welcome = await new Promise<Record<string, unknown>>(
      (resolve, reject) => {
        const timer = setTimeout(
          () => reject(new Error("no welcome frame within 5s")),
          5000,
        );
        socket.addEventListener("message", (event) => {
          clearTimeout(timer);
          resolve(JSON.parse(String(event.data)) as Record<string, unknown>);
        });
        socket.addEventListener("error", () =>
          reject(new Error("ws error before welcome")),
        );
      },
    );
    expect(welcome.type).toBe("welcome");
    expect(welcome.protocol_version).toBe(DELIVERY_PROTOCOL_VERSION);

    // hello with the supported version + an idempotent ack for a row that
    // doesn't exist: both must leave the connection open.
    socket.send(
      JSON.stringify({
        type: "hello",
        protocol_version: DELIVERY_PROTOCOL_VERSION,
        since_id: null,
      }),
    );
    socket.send(
      JSON.stringify({
        type: "ack",
        protocol_version: DELIVERY_PROTOCOL_VERSION,
        id: 999_999_999,
      }),
    );
    const closedEarly = await new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => resolve(false), 1000);
      socket.addEventListener("close", () => {
        clearTimeout(timer);
        resolve(true);
      });
    });
    expect(closedEarly).toBe(false);
    socket.close(1000);
  });

  it("delivery WS: unsupported hello protocol_version closes 4426", async () => {
    const { ticket } = await client.createWsTicket();
    const wsUrl = `${(SERVER as string).replace(/^http/, "ws")}/v1/ws/delivery/${encodeURIComponent(`exec_events:${RANDOM_UUID}`)}?token=${encodeURIComponent(ticket)}`;
    const socket = new WebSocket(wsUrl);

    const code = await new Promise<number>((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error("no close frame within 5s")),
        5000,
      );
      socket.addEventListener("open", () => {
        socket.send(
          JSON.stringify({ type: "hello", protocol_version: 99, since_id: null }),
        );
      });
      socket.addEventListener("close", (event) => {
        clearTimeout(timer);
        resolve(event.code);
      });
    });
    expect(code).toBe(4426);
  });
});
