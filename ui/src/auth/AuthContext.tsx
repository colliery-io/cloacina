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

import { CloacinaClient } from "@cloacina/client";
import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

import { classifyError } from "../api/errors";

/**
 * Connection identity (CLOACI-I-0117 / REQ-001). The credential is a bearer
 * key in `sessionStorage` (NFR-005) — cleared on tab close. Whether the key
 * was typed manually or minted by OIDC (T-0662) is invisible here: it's
 * always a bearer key.
 */
export interface Connection {
  serverUrl: string;
  apiKey: string;
  tenant: string;
}

const STORAGE_KEY = "cloacina.connection";

function loadConnection(): Connection | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Connection) : null;
  } catch {
    return null;
  }
}

interface AuthContextValue {
  connection: Connection | null;
  client: CloacinaClient | null;
  /** Validate a connection against the server, then persist it. */
  connect: (conn: Connection) => Promise<void>;
  disconnect: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [connection, setConnection] = useState<Connection | null>(loadConnection);

  const client = useMemo(
    () =>
      connection
        ? new CloacinaClient({
            baseUrl: connection.serverUrl,
            apiKey: connection.apiKey,
            tenant: connection.tenant,
          })
        : null,
    [connection],
  );

  async function connect(conn: Connection): Promise<void> {
    const probe = new CloacinaClient({
      baseUrl: conn.serverUrl,
      apiKey: conn.apiKey,
      tenant: conn.tenant,
    });
    // Validate reachability + that the key/tenant authorize a scoped read.
    // `health` proves the server is up; `listWorkflows` proves the key works
    // for this tenant (surfaces 401/403 before we let the user in).
    try {
      await probe.health();
      await probe.listWorkflows();
    } catch (err) {
      // Re-throw classified so the connect form can render it (REQ-007).
      const c = classifyError(err);
      throw new Error(c.message);
    }
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(conn));
    setConnection(conn);
  }

  function disconnect(): void {
    sessionStorage.removeItem(STORAGE_KEY);
    setConnection(null);
  }

  const value = useMemo<AuthContextValue>(
    () => ({ connection, client, connect, disconnect }),
    [connection, client],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within <AuthProvider>");
  return ctx;
}

/**
 * The authenticated client. Throws if used outside a connected state — by
 * construction it's only rendered under <RequireAuth>, so this is a guard,
 * not a runtime path.
 */
export function useClient(): CloacinaClient {
  const { client } = useAuth();
  if (!client) throw new Error("useClient used while disconnected");
  return client;
}
