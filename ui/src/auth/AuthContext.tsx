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
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { classifyError } from "../api/errors";

/**
 * Connection identity (CLOACI-I-0117 / REQ-001). The credential is a bearer
 * key in `sessionStorage` (NFR-005) — cleared on tab close. Whether the key
 * was typed manually or minted by OIDC (T-0662) is invisible here: it's
 * always a bearer key.
 *
 * CLOACI-T-0779: the app now holds a LIST of labeled connections (one per
 * tenant) with an active one, so an operator can switch tenants in-app to see
 * isolation. `label` is the display name (defaults to the tenant).
 */
export interface Connection {
  label: string;
  serverUrl: string;
  apiKey: string;
  tenant: string;
}

const STORE_KEY = "cloacina.connections";
const ACTIVE_KEY = "cloacina.active";
const LEGACY_KEY = "cloacina.connection";

interface LoadedState {
  connections: Connection[];
  active: string | null;
}

function loadState(): LoadedState {
  try {
    const raw = sessionStorage.getItem(STORE_KEY);
    let connections: Connection[] = raw ? (JSON.parse(raw) as Connection[]) : [];
    // Migrate a pre-T-0779 single connection.
    if (connections.length === 0) {
      const legacy = sessionStorage.getItem(LEGACY_KEY);
      if (legacy) {
        const c = JSON.parse(legacy) as Omit<Connection, "label"> & { label?: string };
        connections = [{ label: c.label ?? c.tenant, ...c }];
        sessionStorage.removeItem(LEGACY_KEY);
      }
    }
    let active = sessionStorage.getItem(ACTIVE_KEY);
    if ((!active || !connections.some((c) => c.label === active)) && connections.length > 0) {
      active = connections[0].label;
    }
    return { connections, active: connections.length > 0 ? active : null };
  } catch {
    return { connections: [], active: null };
  }
}

function persist(connections: Connection[], active: string | null): void {
  sessionStorage.setItem(STORE_KEY, JSON.stringify(connections));
  if (active) sessionStorage.setItem(ACTIVE_KEY, active);
  else sessionStorage.removeItem(ACTIVE_KEY);
}

interface AuthContextValue {
  /** The active connection, or `null` when none is selected. */
  connection: Connection | null;
  client: CloacinaClient | null;
  /** All saved connections (one per tenant the operator has added). */
  connections: Connection[];
  /** Validate a connection against the server, then save it and make it active.
   *  Upserts by `label` (re-adding a label replaces it). */
  connect: (conn: Connection) => Promise<void>;
  /** Save several connections at once (an OIDC login granting multiple tenant
   *  memberships) and enter one. Validates the one being entered; the rest are
   *  freshly-minted keys, available via the tenant switcher. */
  enterMemberships: (conns: Connection[], activeLabel: string) => Promise<void>;
  /** Switch the active connection to a previously-saved one (no re-validation). */
  switchTo: (label: string) => void;
  /** Remove a saved connection; if it was active, fall back to another. */
  removeConnection: (label: string) => void;
  /** Clear ALL connections and return to the connect screen. */
  disconnect: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const initial = loadState();
  const [connections, setConnections] = useState<Connection[]>(initial.connections);
  const [active, setActive] = useState<string | null>(initial.active);

  const connection = useMemo(
    () => connections.find((c) => c.label === active) ?? null,
    [connections, active],
  );

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

  // CLOACI-T-0800: silent refresh. A minted (local/OIDC) key has a short TTL;
  // re-mint it before it lapses so a demo/session survives. A non-refreshable
  // pasted key returns an error from /auth/refresh — we just stop the loop for
  // that connection (the key doesn't expire anyway).
  useEffect(() => {
    if (!connection || !client) return;
    let stopped = false;
    const REFRESH_MS = 10 * 60 * 1000; // minted TTL is ~15m
    const id = setInterval(() => {
      void (async () => {
        if (stopped) return;
        try {
          const res = await client.refresh();
          setConnections((prev) => {
            const next = prev.map((c) =>
              c.label === connection.label ? { ...c, apiKey: res.key } : c,
            );
            persist(next, connection.label);
            return next;
          });
        } catch {
          stopped = true;
          clearInterval(id);
        }
      })();
    }, REFRESH_MS);
    return () => {
      stopped = true;
      clearInterval(id);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connection?.label, connection?.apiKey, client]);

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
      const c = classifyError(err);
      throw new Error(c.message);
    }
    setConnections((prev) => {
      const next = [...prev.filter((c) => c.label !== conn.label), conn];
      persist(next, conn.label);
      return next;
    });
    setActive(conn.label);
  }

  async function enterMemberships(conns: Connection[], activeLabel: string): Promise<void> {
    const chosen = conns.find((c) => c.label === activeLabel) ?? conns[0];
    if (!chosen) throw new Error("no memberships to enter");
    // Validate the one we're entering; trust the rest (just minted by us).
    const probe = new CloacinaClient({
      baseUrl: chosen.serverUrl,
      apiKey: chosen.apiKey,
      tenant: chosen.tenant,
    });
    try {
      await probe.health();
      await probe.listWorkflows();
    } catch (err) {
      throw new Error(classifyError(err).message);
    }
    setConnections((prev) => {
      const byLabel = new Map(prev.map((c) => [c.label, c]));
      for (const c of conns) byLabel.set(c.label, c);
      const next = Array.from(byLabel.values());
      persist(next, chosen.label);
      return next;
    });
    setActive(chosen.label);
  }

  function switchTo(label: string): void {
    setConnections((prev) => {
      if (prev.some((c) => c.label === label)) {
        setActive(label);
        sessionStorage.setItem(ACTIVE_KEY, label);
      }
      return prev;
    });
  }

  function removeConnection(label: string): void {
    setConnections((prev) => {
      const next = prev.filter((c) => c.label !== label);
      const nextActive = active === label ? (next[0]?.label ?? null) : active;
      persist(next, nextActive);
      setActive(nextActive);
      return next;
    });
  }

  function disconnect(): void {
    setConnections([]);
    setActive(null);
    sessionStorage.removeItem(STORE_KEY);
    sessionStorage.removeItem(ACTIVE_KEY);
    sessionStorage.removeItem(LEGACY_KEY);
  }

  const value = useMemo<AuthContextValue>(
    () => ({
      connection,
      client,
      connections,
      connect,
      enterMemberships,
      switchTo,
      removeConnection,
      disconnect,
    }),
    [connection, client, connections],
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

/** The active tenant — used to scope query keys (only valid while connected). */
export function useTenant(): string {
  const { connection } = useAuth();
  if (!connection) throw new Error("useTenant used while disconnected");
  return connection.tenant;
}
