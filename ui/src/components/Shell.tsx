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

import { TOKEN } from "@colliery-io/aurora-dark";
import { AppShell, Box, Button } from "@mantine/core";
import {
  IconActivity,
  IconGauge,
  IconHeartbeat,
  IconKey,
  IconSettings,
  IconUsers,
  type Icon as TablerIcon,
} from "@tabler/icons-react";
import { NavLink as RouterNavLink, Outlet, useNavigate } from "react-router-dom";

import { useAuth } from "../auth/AuthContext";
import { TenantSwitcher } from "./TenantSwitcher";
import { OpsMetricsProvider, useServerHealth } from "../api/operations";
import { useWorkflows } from "../api/workflows";
import { useGraphs } from "../api/health";
import { useExecutions } from "../api/executions";
import { useTriggers } from "../api/triggers";

/**
 * Authenticated shell — the Aurora Dark sidebar (CLOACI-I-0129): a fixed 232px
 * rail with the brand + server badge, a Run-workflow primary, grouped nav with
 * live counts, and a connection footer. Wraps every in-app route.
 */

const MONO = "'IBM Plex Mono', monospace";

/** Brand "confluence" mark — three strokes flowing down into three nodes. */
function BrandMark() {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden>
      <path d="M5 4 C5 12, 12 12, 12 19" stroke={TOKEN.ice} strokeWidth="1.6" strokeLinecap="round" />
      <path d="M12 4 C12 12, 12 12, 12 19" stroke={TOKEN.teal} strokeWidth="1.6" strokeLinecap="round" />
      <path d="M19 4 C19 12, 12 12, 12 19" stroke={TOKEN.violet} strokeWidth="1.6" strokeLinecap="round" />
      <circle cx="5" cy="4" r="1.8" fill={TOKEN.ice} />
      <circle cx="12" cy="4" r="1.8" fill={TOKEN.teal} />
      <circle cx="19" cy="4" r="1.8" fill={TOKEN.violet} />
      <circle cx="12" cy="20" r="2" fill="#8fbcff" />
    </svg>
  );
}

interface NavDef {
  to: string;
  label: string;
  end?: boolean;
  icon?: TablerIcon;
  /** A colored square marker instead of an icon (the orchestration trio). */
  square?: string;
  /** Right-aligned live count, shown when > 0. */
  count?: number;
  countColor?: string;
}

function NavItem({ def }: { def: NavDef }) {
  const Icon = def.icon;
  return (
    <RouterNavLink
      to={def.to}
      end={def.end}
      style={({ isActive }) => ({
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "7px 10px",
        borderRadius: 8,
        marginBottom: 2,
        fontSize: 13,
        fontWeight: 500,
        textDecoration: "none",
        color: isActive ? "var(--fg)" : "var(--muted)",
        background: isActive ? "rgba(127,178,255,.13)" : "transparent",
        boxShadow: isActive ? "inset 2px 0 0 #7fb2ff" : "none",
      })}
    >
      {def.square ? (
        <span style={{ width: 9, height: 9, borderRadius: 2, background: def.square, flex: "none" }} />
      ) : Icon ? (
        <Icon size={18} stroke={1.5} style={{ flex: "none" }} />
      ) : null}
      <span style={{ flex: 1, minWidth: 0 }}>{def.label}</span>
      {def.count ? (
        <span style={{ fontFamily: MONO, fontSize: 11, color: def.countColor ?? "var(--faint)" }}>
          {def.count}
        </span>
      ) : null}
    </RouterNavLink>
  );
}

function GroupLabel({ children }: { children: string }) {
  return (
    <Box
      style={{
        fontFamily: MONO,
        fontSize: 10,
        letterSpacing: ".1em",
        textTransform: "uppercase",
        color: "var(--faint)",
        padding: "14px 10px 6px",
      }}
    >
      {children}
    </Box>
  );
}

export function Shell() {
  const { connection, disconnect } = useAuth();
  const navigate = useNavigate();
  const health = useServerHealth();

  // Live nav counts — cached/shared with the route components via react-query.
  const workflows = useWorkflows();
  const graphs = useGraphs();
  const triggers = useTriggers({ limit: 200, offset: 0 });
  const recent = useExecutions({ limit: 200, offset: 0 });

  const wfCount = (workflows.data?.items ?? []).filter((w) => w.tasks.length > 0).length;
  const graphCount = (graphs.data?.items ?? []).length;
  const trigCount = (triggers.data?.items ?? []).length;
  const runningCount = (recent.data?.items ?? []).filter(
    (e) => e.status.toLowerCase() === "running",
  ).length;

  const serverReady = health.data?.alive && health.data?.ready;
  const serverDot = health.isPending ? TOKEN.muted : health.data?.alive ? TOKEN.ok : TOKEN.bad;
  const serverWord = health.isPending
    ? "…"
    : serverReady
      ? "ready"
      : health.data?.alive
        ? "starting"
        : "down";

  function handleDisconnect() {
    disconnect();
    navigate("/connect", { replace: true });
  }

  return (
    <AppShell navbar={{ width: 232, breakpoint: "sm" }} padding={0}>
      <AppShell.Navbar
        p={0}
        style={{
          background: "var(--sidebar)",
          borderRight: "1px solid var(--border-soft)",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Brand */}
        <Box style={{ padding: "18px 14px 10px" }}>
          <Box style={{ display: "flex", alignItems: "center", gap: 9 }}>
            <BrandMark />
            <span style={{ fontSize: 16, fontWeight: 600, color: "var(--fg-bright)" }}>Cloacina</span>
          </Box>
          <Box
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              marginTop: 8,
              fontFamily: MONO,
              fontSize: 11,
              color: "var(--muted)",
            }}
          >
            <span style={{ width: 7, height: 7, borderRadius: "50%", background: serverDot }} />
            server {serverWord} · v0.8.0
          </Box>
        </Box>

        {/* Run workflow */}
        <Box style={{ padding: "6px 14px 8px" }}>
          <Button
            fullWidth
            color="ice"
            radius={9}
            size="sm"
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            onClick={() => navigate("/workflows")}
          >
            ▸ Run workflow
          </Button>
        </Box>

        {/* Nav */}
        <Box style={{ flex: 1, overflowY: "auto", padding: "4px 8px" }}>
          <NavItem def={{ to: "/", label: "Overview", end: true, icon: IconGauge }} />
          <NavItem
            def={{
              to: "/executions",
              label: "Executions",
              icon: IconActivity,
              count: runningCount,
              countColor: TOKEN.ice,
            }}
          />

          <GroupLabel>Orchestration</GroupLabel>
          <NavItem def={{ to: "/workflows", label: "Workflows", square: TOKEN.ice, count: wfCount }} />
          <NavItem def={{ to: "/triggers", label: "Triggers", square: TOKEN.violet, count: trigCount }} />
          <NavItem def={{ to: "/graphs", label: "Graphs", square: TOKEN.teal, count: graphCount }} />

          <GroupLabel>System</GroupLabel>
          <NavItem def={{ to: "/operations", label: "Operations", icon: IconHeartbeat }} />
          <NavItem def={{ to: "/keys", label: "API Keys", icon: IconKey }} />
          <NavItem def={{ to: "/accounts", label: "Accounts", icon: IconUsers }} />
          <NavItem def={{ to: "/settings", label: "Settings", icon: IconSettings }} />
        </Box>

        {/* Connection footer */}
        <Box style={{ padding: "12px 14px", borderTop: "1px solid var(--border-soft)" }}>
          <Box
            style={{
              fontFamily: MONO,
              fontSize: 10,
              letterSpacing: ".1em",
              textTransform: "uppercase",
              color: "var(--faint)",
              marginBottom: 6,
            }}
          >
            Connection
          </Box>
          <TenantSwitcher dot={serverDot} />
          <Box
            title={connection?.serverUrl}
            style={{
              fontFamily: MONO,
              fontSize: 10.5,
              color: "var(--faint)",
              marginTop: 3,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {connection?.serverUrl ?? ""}
          </Box>
          <Box
            component="button"
            onClick={handleDisconnect}
            style={{
              marginTop: 8,
              background: "none",
              border: "none",
              padding: 0,
              cursor: "pointer",
              fontSize: 11.5,
              color: "var(--muted)",
            }}
          >
            Disconnect ↗
          </Box>
        </Box>
      </AppShell.Navbar>

      <AppShell.Main style={{ background: "var(--bg)" }}>
        <Box style={{ padding: "22px 28px" }}>
          {/* App-level live ops-metrics WS — connects at login, stays warm for
              the session so live pages never cold-start (CLOACI-T-0774). */}
          <OpsMetricsProvider>
            <Outlet />
          </OpsMetricsProvider>
        </Box>
      </AppShell.Main>
    </AppShell>
  );
}
