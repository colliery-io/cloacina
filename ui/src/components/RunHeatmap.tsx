/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Recent-runs heatmap (CLOACI-T-0764, operational DAG view §5). Airflow-style:
 *  one bar per run over the last N, height = duration, color = status, newest
 *  pulsing while in flight; click a bar → that execution.
 */
import { Tooltip } from "@mantine/core";
import { useNavigate } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { MONO } from "./aurora";
import { Empty, ErrorState, Loading } from "./states/States";
import { formatAgo } from "../util/activity";
import { statusColor } from "../util/tokens";

const isRunning = (s: string) => s.toLowerCase() === "running";
const isTerminalDone = (s: string) =>
  ["completed", "failed", "cancelled", "canceled"].includes(s.toLowerCase());

function durationMs(e: { started_at?: string | null; completed_at?: string | null; status: string }): number {
  const s = e.started_at ? Date.parse(e.started_at) : NaN;
  if (Number.isNaN(s)) return 0;
  const end = e.completed_at ? Date.parse(e.completed_at) : isRunning(e.status) ? Date.now() : s;
  return Math.max(0, end - s);
}

function fmtMs(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60_000)}m ${Math.round((ms % 60_000) / 1000)}s`;
}

function Swatch({ color, label }: { color: string; label: string }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
      <span style={{ width: 9, height: 9, borderRadius: 2, background: color }} />
      <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>{label}</span>
    </span>
  );
}

export function RunHeatmap({ workflow }: { workflow: string }) {
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useExecutions({
    workflow: workflow || undefined,
    limit: 40,
  });

  if (isPending) return <Loading label="Loading recent runs…" />;
  if (isError) return <ErrorState error={error} onRetry={refetch} />;

  // Newest-first from the API → oldest→newest left-to-right.
  const items = [...(data?.items ?? [])].reverse();
  if (items.length === 0) return <Empty message="No runs in the sampled window yet." />;

  const maxMs = Math.max(1, ...items.map(durationMs));
  const newestId = items[items.length - 1]?.id;

  return (
    <div>
      <div style={{ display: "flex", alignItems: "flex-end", gap: 4, height: 72 }}>
        {items.map((e, i) => {
          const ms = durationMs(e);
          const c = statusColor(e.status);
          const running = isRunning(e.status);
          const newest = e.id === newestId;
          return (
            <Tooltip
              key={e.id}
              label={`run #${i + 1} · ${e.status.toLowerCase()} · ${fmtMs(ms)} · ${formatAgo(e.started_at)}`}
              withArrow
              openDelay={80}
            >
              <div
                onClick={() => navigate(`/executions/${e.id}`)}
                className={running && newest ? "cl-pulse" : undefined}
                style={{
                  flex: "1 1 0",
                  minWidth: 7,
                  height: `${Math.max(6, (ms / maxMs) * 72)}px`,
                  background: c,
                  borderRadius: 2,
                  cursor: "pointer",
                  outline: newest ? "2px solid rgba(127,178,255,.4)" : undefined,
                  outlineOffset: newest ? 1 : undefined,
                  transition: "filter .12s",
                }}
                onMouseEnter={(ev) => (ev.currentTarget.style.filter = "brightness(1.35)")}
                onMouseLeave={(ev) => (ev.currentTarget.style.filter = "none")}
              />
            </Tooltip>
          );
        })}
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", marginTop: 6 }}>
        <span style={{ fontFamily: MONO, fontSize: 10, color: "var(--fainter)" }}>oldest</span>
        <span style={{ fontFamily: MONO, fontSize: 10, color: "var(--fainter)" }}>newest →</span>
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 10 }}>
        <div style={{ display: "flex", gap: 14 }}>
          <Swatch color={statusColor("completed")} label="completed" />
          <Swatch color={statusColor("failed")} label="failed" />
          <Swatch color={statusColor("running")} label="running" />
        </div>
        <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>click a run to open it</span>
      </div>
    </div>
  );
}

export { durationMs, fmtMs, isRunning, isTerminalDone };
