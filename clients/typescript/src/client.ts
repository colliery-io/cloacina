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

import createClient, { type Client as FetchClient } from "openapi-fetch";
import type { components, paths } from "../generated/types.js";

/** All wire schemas from the OpenAPI document. */
export type schemas = components["schemas"];

export type ErrorBody = schemas["ErrorBody"];

/** Query string for execution listing (status/workflow filters + paging). */
export type ListExecutionsQuery = NonNullable<
  paths["/v1/tenants/{tenant_id}/executions"]["get"]["parameters"]["query"]
>;

/** Query string for trigger listing (paging). */
export type ListTriggersQuery = NonNullable<
  paths["/v1/tenants/{tenant_id}/triggers"]["get"]["parameters"]["query"]
>;

/** Error thrown by the ergonomic helpers when the server returns non-2xx. */
export class CloacinaApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, body: ErrorBody | undefined) {
    super(body?.error ?? `HTTP ${status}`);
    this.name = "CloacinaApiError";
    this.status = status;
    this.code = body?.code ?? "unknown";
  }
}

export interface CloacinaClientOptions {
  /** Server base URL, e.g. `http://localhost:8080`. */
  baseUrl: string;
  /**
   * API key, sent as `Authorization: Bearer <key>` on every request.
   *
   * Auth note (v1, deliberate): the API key in a browser context is the
   * accepted auth story for first-party admin UIs only. Browser-grade auth
   * (sessions/OIDC) is a future-initiative concern — do not embed keys in
   * pages served to untrusted users.
   */
  apiKey?: string;
  /** Default tenant for the tenant-scoped helpers. */
  tenant?: string;
  /** Custom fetch implementation (defaults to global fetch). */
  fetch?: typeof globalThis.fetch;
}

function unwrap<T>(result: {
  data?: T;
  error?: ErrorBody;
  response: Response;
}): T {
  if (result.error !== undefined || !result.response.ok) {
    throw new CloacinaApiError(result.response.status, result.error);
  }
  return result.data as T;
}

/**
 * Ergonomic client for the cloacina-server REST API.
 *
 * The typed low-level client (`openapi-fetch`) is exposed as `.api` for
 * anything the helpers don't cover; helpers exist for every documented
 * endpoint and throw {@link CloacinaApiError} on non-2xx responses.
 */
export class CloacinaClient {
  /** Typed low-level client — `client.api.GET("/v1/tenants", ...)`. */
  readonly api: FetchClient<paths>;
  readonly baseUrl: string;
  readonly tenant?: string;
  readonly #apiKey?: string;

  constructor(options: CloacinaClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.tenant = options.tenant;
    this.#apiKey = options.apiKey;
    this.api = createClient<paths>({
      baseUrl: this.baseUrl,
      fetch: options.fetch,
    });
    if (options.apiKey !== undefined) {
      const key = options.apiKey;
      this.api.use({
        onRequest({ request }) {
          request.headers.set("authorization", `Bearer ${key}`);
          return request;
        },
      });
    }
  }

  /** Scoped copy of this client with a different default tenant. */
  withTenant(tenant: string): CloacinaClient {
    return new CloacinaClient({
      baseUrl: this.baseUrl,
      apiKey: this.#apiKey,
      tenant,
    });
  }

  #tenant(override?: string): string {
    const t = override ?? this.tenant;
    if (t === undefined) {
      throw new Error(
        "tenant required: pass one to the call or set `tenant` on the client",
      );
    }
    return t;
  }

  // ---- operational ----

  async health(): Promise<unknown> {
    return unwrap(await this.api.GET("/health"));
  }

  async ready(): Promise<Response> {
    // /ready intentionally returns 503 when not ready; expose the raw
    // response so callers can branch on status without exceptions.
    const { response } = await this.api.GET("/ready");
    return response;
  }

  // ---- keys ----

  async createKey(
    body: schemas["CreateKeyRequest"],
  ): Promise<schemas["KeyCreatedResponse"]> {
    return unwrap(await this.api.POST("/v1/auth/keys", { body }));
  }

  async listKeys(): Promise<schemas["ListResponse_KeyInfo"]> {
    return unwrap(await this.api.GET("/v1/auth/keys"));
  }

  async revokeKey(keyId: string): Promise<schemas["KeyRevokedResponse"]> {
    return unwrap(
      await this.api.DELETE("/v1/auth/keys/{key_id}", {
        params: { path: { key_id: keyId } },
      }),
    );
  }

  async createTenantKey(
    body: schemas["CreateKeyRequest"],
    tenant?: string,
  ): Promise<schemas["KeyCreatedResponse"]> {
    return unwrap(
      await this.api.POST("/v1/tenants/{tenant_id}/keys", {
        params: { path: { tenant_id: this.#tenant(tenant) } },
        body,
      }),
    );
  }

  /** Mint a single-use, short-lived WebSocket ticket. */
  async createWsTicket(): Promise<schemas["WsTicketResponse"]> {
    return unwrap(await this.api.POST("/v1/auth/ws-ticket"));
  }

  // ---- tenants ----

  async createTenant(
    body: schemas["CreateTenantRequest"],
  ): Promise<schemas["TenantCreatedResponse"]> {
    return unwrap(await this.api.POST("/v1/tenants", { body }));
  }

  async listTenants(): Promise<schemas["ListResponse_TenantSummary"]> {
    return unwrap(await this.api.GET("/v1/tenants"));
  }

  async removeTenant(
    schemaName: string,
  ): Promise<schemas["TenantRemovedResponse"]> {
    return unwrap(
      await this.api.DELETE("/v1/tenants/{schema_name}", {
        params: { path: { schema_name: schemaName } },
      }),
    );
  }

  // ---- workflows ----

  /**
   * Upload a `.cloacina` package (multipart). `pkg` may be a Blob/File or
   * raw bytes.
   */
  async uploadWorkflow(
    pkg: Blob | Uint8Array,
    tenant?: string,
  ): Promise<schemas["WorkflowUploadedResponse"]> {
    const blob =
      pkg instanceof Blob
        ? pkg
        : new Blob([pkg as unknown as BlobPart], {
            type: "application/octet-stream",
          });
    const form = new FormData();
    form.set("file", blob, "package.cloacina");
    return unwrap(
      await this.api.POST("/v1/tenants/{tenant_id}/workflows", {
        params: { path: { tenant_id: this.#tenant(tenant) } },
        // openapi-fetch serializes plain objects as JSON; hand it FormData
        // through bodySerializer so the browser/undici sets the multipart
        // boundary itself.
        body: form as never,
        bodySerializer: (b: unknown) => b as FormData,
      }),
    );
  }

  async listWorkflows(
    tenant?: string,
  ): Promise<schemas["TenantListResponse_WorkflowSummary"]> {
    return unwrap(
      await this.api.GET("/v1/tenants/{tenant_id}/workflows", {
        params: { path: { tenant_id: this.#tenant(tenant) } },
      }),
    );
  }

  async getWorkflow(
    name: string,
    tenant?: string,
  ): Promise<schemas["WorkflowDetail"]> {
    return unwrap(
      await this.api.GET("/v1/tenants/{tenant_id}/workflows/{name}", {
        params: { path: { tenant_id: this.#tenant(tenant), name } },
      }),
    );
  }

  async deleteWorkflow(
    name: string,
    version: string,
    tenant?: string,
  ): Promise<schemas["WorkflowDeletedResponse"]> {
    return unwrap(
      await this.api.DELETE(
        "/v1/tenants/{tenant_id}/workflows/{name}/{version}",
        {
          params: {
            path: { tenant_id: this.#tenant(tenant), name, version },
          },
        },
      ),
    );
  }

  // ---- triggers ----

  async listTriggers(
    query?: ListTriggersQuery,
    tenant?: string,
  ): Promise<schemas["TenantListResponse_TriggerScheduleSummary"]> {
    return unwrap(
      await this.api.GET("/v1/tenants/{tenant_id}/triggers", {
        params: {
          path: { tenant_id: this.#tenant(tenant) },
          query: query as never,
        },
      }),
    );
  }

  async getTrigger(
    name: string,
    tenant?: string,
  ): Promise<schemas["TriggerDetailResponse"]> {
    return unwrap(
      await this.api.GET("/v1/tenants/{tenant_id}/triggers/{name}", {
        params: { path: { tenant_id: this.#tenant(tenant), name } },
      }),
    );
  }

  // ---- executions ----

  async executeWorkflow(
    name: string,
    body: schemas["ExecuteRequest"] = {},
    tenant?: string,
  ): Promise<schemas["ExecuteResponse"]> {
    return unwrap(
      await this.api.POST(
        "/v1/tenants/{tenant_id}/workflows/{name}/execute",
        {
          params: { path: { tenant_id: this.#tenant(tenant), name } },
          body,
        },
      ),
    );
  }

  async listExecutions(
    query?: ListExecutionsQuery,
    tenant?: string,
  ): Promise<schemas["TenantListResponse_ExecutionSummary"]> {
    return unwrap(
      await this.api.GET("/v1/tenants/{tenant_id}/executions", {
        params: {
          path: { tenant_id: this.#tenant(tenant) },
          query: query as never,
        },
      }),
    );
  }

  /**
   * Async iterator over execution pages — keeps fetching `limit`-sized
   * pages until a short page arrives.
   */
  async *iterateExecutions(
    query: Omit<ListExecutionsQuery, "offset"> = {},
    tenant?: string,
  ): AsyncGenerator<schemas["ExecutionSummary"], void, void> {
    const limit = query.limit ?? 100;
    let offset = 0;
    for (;;) {
      const page = await this.listExecutions(
        { ...query, limit, offset },
        tenant,
      );
      for (const item of page.items) {
        yield item;
      }
      if (page.items.length < limit) {
        return;
      }
      offset += limit;
    }
  }

  async getExecution(
    execId: string,
    tenant?: string,
  ): Promise<schemas["ExecutionDetail"]> {
    return unwrap(
      await this.api.GET("/v1/tenants/{tenant_id}/executions/{exec_id}", {
        params: { path: { tenant_id: this.#tenant(tenant), exec_id: execId } },
      }),
    );
  }

  async getExecutionEvents(
    execId: string,
    tenant?: string,
  ): Promise<schemas["ExecutionEventsResponse"]> {
    return unwrap(
      await this.api.GET(
        "/v1/tenants/{tenant_id}/executions/{exec_id}/events",
        {
          params: {
            path: { tenant_id: this.#tenant(tenant), exec_id: execId },
          },
        },
      ),
    );
  }

  // ---- computation-graph health ----

  async listAccumulators(): Promise<schemas["ListResponse_AccumulatorStatus"]> {
    return unwrap(await this.api.GET("/v1/health/accumulators"));
  }

  async listReactors(): Promise<schemas["ListResponse_ReactorStatus"]> {
    return unwrap(await this.api.GET("/v1/health/reactors"));
  }

  async listGraphs(): Promise<schemas["ListResponse_GraphStatus"]> {
    return unwrap(await this.api.GET("/v1/health/graphs"));
  }

  async getGraph(name: string): Promise<schemas["GraphStatus"]> {
    return unwrap(
      await this.api.GET("/v1/health/graphs/{name}", {
        params: { path: { name } },
      }),
    );
  }
}
