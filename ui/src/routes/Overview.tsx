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

import { Box, Grid, Group, SimpleGrid } from "@mantine/core";
import { type CSSProperties } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { useGraphs } from "../api/health";
import { useWorkflows } from "../api/workflows";
import { useLiveOpsMetrics } from "../api/operations";
import { useAuth } from "../auth/AuthContext";
import { ActiveRunCard } from "../components/ActiveRunCard";
import { GraphMiniCard } from "../components/GraphMiniCard";
import { formatDuration } from "../util/format";
import { statusColor, TOKEN } from "../util/tokens";
import { useGraphThroughput } from "../util/activity";

const MONO = "'IBM Plex Mono', monospace";

/** Aurora Dark Overview (CLOACI-I-0129, spec 01/02): metrics + health strip +
 *  active executions / computation graphs / recently-completed. */
export function Overview() {
  const navigate = useNavigate();
  const { connection } = useAuth();
  const workflows = useWorkflows();
  const graphs = useGraphs();
  const recent = useExecutions({ limit: 200, offset: 0 });
  const ops = useLiveOpsMetrics(!!connection);

  const wfItems = (workflows.data?.items ?? []).filter((w) => w.tasks.length > 0);
  const graphItems = graphs.data?.items ?? [];
  const graphTp = useGraphThroughput(graphItems);
  const recentItems = recent.data?.items ?? [];

  const isRunning = (s: string) => ["running", "paused"].includes(s.toLowerCase());
  const isDone = (s: string) =>
    ["completed", "failed", "cancelled", "canceled"].includes(s.toLowerCase());

  const active = recentItems.filter((e) => isRunning(e.status));
  const completed = recentItems.filter((e) => isDone(e.status));
  const runningCount = recentItems.filter((e) => e.status.toLowerCase() === "running").length;
  const completed24h = completed.filter((e) => e.status.toLowerCase() === "completed").length;
  const failed24h = completed.filter((e) => e.status.toLowerCase() === "failed").length;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
      {/* Header */}
      <Group justify="space-between" align="flex-start">
        <Box>
          <Box style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)" }}>Overview</Box>
          <Box style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 2 }}>
            tenant {connection?.tenant ?? "—"} · {recentItems.length} runs tracked
          </Box>
        </Box>
        <Box
          component={Link}
          to="/executions"
          style={{
            width: 300,
            background: "var(--panel)",
            border: "1px solid var(--border)",
            borderRadius: 9,
            padding: "8px 12px",
            color: "var(--faint)",
            fontSize: 12.5,
            textDecoration: "none",
          }}
        >
          ⌕ Find a workflow, run, or task…
        </Box>
      </Group>

      {/* Metrics */}
      <SimpleGrid cols={{ base: 2, md: 4 }} spacing={13}>
        <MetricCard label="Workflows" value={wfItems.length} color="var(--fg)" sub="registered" />
        <MetricCard label="Running" value={runningCount} color={TOKEN.ice} sub="in flight" />
        <MetricCard label="Completed" value={completed24h} color={TOKEN.ok} sub="recent" />
        <MetricCard label="Failed" value={failed24h} color={TOKEN.bad} sub="recent" />
      </SimpleGrid>

      {/* Health strip */}
      <SimpleGrid cols={{ base: 3, md: 6 }} spacing={9}>
        <HealthTile name="Server" ok={ops?.server.alive} detail={ops ? (ops.server.ready ? "alive · ready" : "alive") : "connecting…"} />
        <HealthTile name="Compiler" ok={!!ops} detail={ops ? `${ops.compiler.building} building · ${ops.compiler.pending} pending` : "connecting…"} />
        <HealthTile name="Reconciler" ok={ops?.reconciler.status === "ok"} detail={ops ? `${ops.reconciler.built} built · ${ops.reconciler.failed} failed` : "connecting…"} />
        <HealthTile name="Scheduler" ok={ops?.server.ready} detail={ops ? "ok" : "connecting…"} />
        <HealthTile name="Database" ok={ops?.server.ready} detail={ops ? (ops.server.ready ? "ok" : (ops.server.reason ?? "not ready")) : "connecting…"} />
        <HealthTile name="Agents" ok={(ops?.fleet.length ?? 0) > 0} detail={ops ? `${ops.fleet.length} online` : "connecting…"} />
      </SimpleGrid>

      {/* Two columns */}
      <Grid gutter={18}>
        <Grid.Col span={{ base: 12, lg: 7 }}>
          {/* Active executions */}
          <SectionHeader title="Active executions" right={`${active.length} in flight`} onClick={() => navigate("/executions")} />
          {active.length === 0 ? (
            <EmptyCard message="No executions in flight." />
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {active.map((e) => (
                <ActiveRunCard key={e.id} execution={e} />
              ))}
            </div>
          )}

          {/* Computation graphs */}
          <Box mt={18}>
            <SectionHeader title="Computation graphs" right={`${graphItems.length} active`} onClick={() => navigate("/graphs")} />
            {graphItems.length === 0 ? (
              <EmptyCard message="No computation graphs loaded." />
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                {graphItems.map((g) => (
                  <GraphMiniCard key={g.name} graph={g} rate={graphTp.get(g.name) ?? undefined} />
                ))}
              </div>
            )}
          </Box>
        </Grid.Col>

        {/* Recently completed */}
        <Grid.Col span={{ base: 12, lg: 5 }}>
          <SectionHeader title="Recently completed" right="View all" onClick={() => navigate("/executions")} />
          <Box style={cardStyle}>
            {completed.length === 0 ? (
              <Box style={{ color: "var(--faint)", fontSize: 12.5, padding: "8px 2px" }}>No completed runs yet.</Box>
            ) : (
              completed.slice(0, 8).map((e, i) => (
                <Box
                  key={e.id}
                  onClick={() => navigate(`/executions/${e.id}`)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    padding: "9px 2px",
                    borderTop: i === 0 ? "none" : "1px solid var(--border-fainter)",
                    cursor: "pointer",
                  }}
                >
                  <Dot color={statusColor(e.status)} glow />
                  <Box style={{ flex: 1, minWidth: 0 }}>
                    <Box style={{ fontSize: 13, color: "var(--fg)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {e.workflow_name}
                    </Box>
                    <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>{e.id.slice(0, 8)}</Box>
                  </Box>
                  <Box style={{ textAlign: "right" }}>
                    <Box
                      style={{
                        fontFamily: MONO,
                        fontSize: 11.5,
                        color: e.status.toLowerCase() === "failed" ? TOKEN.bad : "var(--fg-2)",
                      }}
                    >
                      {formatDuration(e.started_at, e.completed_at)}
                    </Box>
                    <Box style={{ fontFamily: MONO, fontSize: 10, color: "var(--fainter)" }}>{ago(e.started_at)}</Box>
                  </Box>
                </Box>
              ))
            )}
          </Box>
        </Grid.Col>
      </Grid>
    </div>
  );
}

const cardStyle: CSSProperties = {
  background: "var(--panel)",
  border: "1px solid var(--border)",
  borderRadius: 10,
  padding: "13px 15px",
  cursor: "pointer",
};

function MetricCard({ label, value, color, sub }: { label: string; value: number; color: string; sub: string }) {
  return (
    <Box style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 10, padding: "15px 16px" }}>
      <Box style={{ fontFamily: MONO, fontSize: 10.5, letterSpacing: ".07em", textTransform: "uppercase", color: "var(--muted)" }}>
        {label}
      </Box>
      <Box className="cl-tnum" style={{ fontSize: 30, fontWeight: 600, lineHeight: 1, color, margin: "6px 0 4px" }}>
        {value}
      </Box>
      <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>{sub}</Box>
    </Box>
  );
}

function HealthTile({ name, ok, detail }: { name: string; ok: boolean | undefined; detail: string }) {
  const color = ok === undefined ? TOKEN.muted : ok ? TOKEN.ok : TOKEN.bad;
  return (
    <Box style={{ background: "var(--panel-2)", border: "1px solid var(--border-soft)", borderRadius: 9, padding: "10px 12px" }}>
      <Group gap={6} mb={3}>
        <Dot color={color} />
        <span style={{ fontSize: 12, fontWeight: 500, color: "var(--fg)" }}>{name}</span>
      </Group>
      <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>{detail}</Box>
    </Box>
  );
}

function SectionHeader({ title, right, onClick }: { title: string; right: string; onClick?: () => void }) {
  return (
    <Group justify="space-between" mb={10} style={{ borderBottom: "1px solid var(--border-soft)", paddingBottom: 8 }}>
      <span style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)" }}>{title}</span>
      <span onClick={onClick} style={{ fontSize: 12, color: "var(--ice)", cursor: onClick ? "pointer" : "default" }}>
        {right}
      </span>
    </Group>
  );
}

function EmptyCard({ message }: { message: string }) {
  return (
    <Box style={{ border: "1px dashed var(--border)", borderRadius: 10, padding: "18px 15px", color: "var(--faint)", fontSize: 12.5 }}>
      {message}
    </Box>
  );
}

function Dot({ color, glow }: { color: string; glow?: boolean }) {
  return (
    <span
      style={{
        width: 8,
        height: 8,
        borderRadius: "50%",
        background: color,
        flex: "none",
        boxShadow: glow ? `0 0 0 3px ${color}22` : undefined,
      }}
    />
  );
}

function ago(ts: string | null | undefined): string {
  if (!ts) return "";
  const ms = Date.now() - new Date(ts).getTime();
  if (Number.isNaN(ms) || ms < 0) return "";
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}
