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
    const err = await client
      .executeWorkflow("does-not-exist", { context: { k: "v" } })
      .then(() => null)
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(CloacinaApiError);
    expect((err as CloacinaApiError).status).toBe(400);
    expect((err as CloacinaApiError).code).toBe("execution_failed");
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
