/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Graph operational-view sections that depend on live reactor data
 *  (CLOACI-T-0767): status strip, fire-activity heatmap, recent fires. These
 *  read the reactor fires log / timeseries via the controls hooks. The
 *  prop-driven sections (DegradedBanner, ReactorReadiness, AccumulatorTable)
 *  live in the design system (@colliery-io/aurora-dark).
 */
import { type ReactNode, useState } from "react";
import { Tooltip } from "@mantine/core";

import { useReactorFires, useReactorFireTimeseries } from "../api/controls";
import {
  Dot,
  Empty,
  ErrorState,
  Loading,
  MONO,
  Pill,
  TOKEN,
  type Acc,
  accStale,
  explainToken,
  formatAgo,
  healthColor,
  statusColor,
  useGraphThroughput,
} from "@colliery-io/aurora-dark";

function fmtMs(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60_000)}m ${Math.round((ms % 60_000) / 1000)}s`;
}
function compact(n: number): string {
  if (n < 1000) return `${n}`;
  if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0)}K`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}

// ---- Status strip -------------------------------------------------------

function StripCard({ label, value, color, dot, sub }: { label: string; value: ReactNode; color?: string; dot?: string; sub: ReactNode }) {
  return (
    <div style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 10, padding: "13px 15px" }}>
      <div style={{ fontFamily: MONO, fontSize: 9.5, letterSpacing: ".09em", textTransform: "uppercase", color: "var(--muted)" }}>{label}</div>
      <div className="cl-tnum" style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 25, fontWeight: 600, color: color ?? "var(--fg)", margin: "7px 0 4px" }}>
        {dot && <Dot color={dot} />}
        {value}
      </div>
      <div style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>{sub}</div>
    </div>
  );
}

export function GraphStatusStrip({
  graphName,
  health,
  fires,
  lastFiredAt,
  reactor,
  accumulators,
}: {
  graphName: string;
  health: unknown;
  fires: number;
  lastFiredAt: string | null | undefined;
  reactor: string | null | undefined;
  accumulators: Acc[];
}) {
  const rate = useGraphThroughput([{ name: graphName, fires }]).get(graphName);
  const recentFires = useReactorFires(reactor, { limit: 200, poll: true });
  const hs = typeof health === "string" ? health : (health as { state?: string })?.state ?? "";
  const healthy = accumulators.filter((a) => !accStale(a)).length;
  const failures = (recentFires.data?.items ?? []).filter((f) => !f.ok).length;

  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 12 }}>
      <StripCard label="Health" value={explainToken(hs || "unknown").label} color={healthColor(hs)} dot={healthColor(hs)} sub={reactor ?? "—"} />
      <StripCard label="Throughput" value={rate == null ? "—" : `~${rate}`} color={TOKEN.ice} sub="fires / min" />
      <StripCard label="Last fire" value={formatAgo(lastFiredAt)} sub="last graph fire" />
      <StripCard label="Total fires" value={compact(fires)} sub="since load" />
      <StripCard
        label="Sources"
        value={`${healthy} / ${accumulators.length}`}
        color={healthy === accumulators.length ? TOKEN.ok : TOKEN.gold}
        sub="healthy"
      />
      <StripCard label="Fire failures" value={failures} color={failures ? TOKEN.bad : "var(--faint)"} sub="downstream · recent" />
    </div>
  );
}

// ---- Fire activity heatmap ---------------------------------------------

export function FireActivity({ reactor }: { reactor: string | null | undefined }) {
  const { data, isPending, isError, error, refetch } = useReactorFireTimeseries(reactor, { poll: true });
  if (!reactor) return <Empty message="No reactor bound." />;
  if (isPending) return <Loading label="Loading fire activity…" />;
  if (isError) return <ErrorState error={error} onRetry={refetch} />;

  const buckets = data?.buckets ?? [];
  const max = Math.max(1, ...buckets);
  const newest = buckets.length - 1;

  return (
    <div>
      <div style={{ display: "flex", alignItems: "flex-end", gap: 3, height: 64 }}>
        {buckets.map((c, i) => {
          const stall = c > 0 && c < max * 0.25;
          const color = c === 0 ? "var(--border-control)" : stall ? TOKEN.gold : TOKEN.ice;
          return (
            <Tooltip key={i} label={`${buckets.length - 1 - i}m ago · ${c} fires`} withArrow openDelay={80}>
              <div
                className={i === newest && c > 0 ? "cl-pulse" : undefined}
                style={{ flex: "1 1 0", minWidth: 5, height: `${Math.max(3, (c / max) * 64)}px`, background: color, borderRadius: 2 }}
              />
            </Tooltip>
          );
        })}
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", marginTop: 8 }}>
        <span style={{ display: "inline-flex", gap: 14 }}>
          <Legend color={TOKEN.ice} label="fires" />
          <Legend color={TOKEN.gold} label="stall" />
        </span>
        <span style={{ fontFamily: MONO, fontSize: 10, color: "var(--fainter)" }}>60 min ago ← now</span>
      </div>
    </div>
  );
}

function Legend({ color, label }: { color: string; label: string }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
      <span style={{ width: 9, height: 9, borderRadius: 2, background: color }} />
      <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>{label}</span>
    </span>
  );
}

// ---- Recent fires -------------------------------------------------------

/** Compact one-line render of a JSON value (CLOACI-T-0775). */
function compactValue(v: unknown): string {
  if (v == null) return "null";
  if (typeof v === "object") {
    const s = JSON.stringify(v);
    return s.length > 88 ? `${s.slice(0, 87)}…` : s;
  }
  return String(v);
}

/** One input/output line under a fire — the source/terminal value, compact by
 *  default, full pretty JSON when the fire is expanded (CLOACI-T-0775). */
function IOLine({ kind, label, value, expanded }: { kind: "in" | "out"; label?: string; value: unknown; expanded: boolean }) {
  const accent = kind === "in" ? TOKEN.teal : TOKEN.violet;
  return (
    <div style={{ display: "flex", gap: 8, alignItems: "baseline", minWidth: 0, fontFamily: MONO, fontSize: 10.5 }}>
      <span style={{ color: accent, flex: "none", fontSize: 9, letterSpacing: ".06em", textTransform: "uppercase", width: 22 }}>{kind}</span>
      {label && <span style={{ color: "var(--muted)", flex: "none" }}>{label}</span>}
      <span
        style={{
          color: "var(--fg-2)",
          minWidth: 0,
          whiteSpace: expanded ? "pre-wrap" : "nowrap",
          overflow: expanded ? "visible" : "hidden",
          textOverflow: "ellipsis",
          wordBreak: expanded ? "break-word" : "normal",
        }}
      >
        {expanded && typeof value === "object" ? JSON.stringify(value, null, 2) : compactValue(value)}
      </span>
    </div>
  );
}

export function RecentFires({ reactor }: { reactor: string | null | undefined }) {
  const { data, isPending, isError, error, refetch } = useReactorFires(reactor, { limit: 30, poll: true });
  const [open, setOpen] = useState<number | null>(null);
  if (!reactor) return <Empty message="No reactor bound." />;
  if (isPending) return <Loading label="Loading fires…" />;
  if (isError) return <ErrorState error={error} onRetry={refetch} />;
  const fires = data?.items ?? [];
  if (fires.length === 0) return <Empty message="No fires recorded yet." />;

  return (
    <div style={{ display: "flex", flexDirection: "column" }}>
      {fires.map((f, i) => {
        const inputs = Object.entries((f.inputs ?? {}) as Record<string, unknown>);
        const outputs = (f.outputs ?? []) as unknown[];
        const hasIO = inputs.length > 0 || outputs.length > 0;
        const expanded = open === i;
        return (
          <div key={i} style={{ padding: "9px 0", borderTop: i === 0 ? "none" : "1px solid var(--border-fainter)" }}>
            <div
              style={{ display: "grid", gridTemplateColumns: "18px 1fr 80px 64px", gap: 10, alignItems: "center", cursor: hasIO ? "pointer" : "default" }}
              onClick={() => hasIO && setOpen(expanded ? null : i)}
            >
              <Dot color={f.ok ? TOKEN.ok : TOKEN.bad} size={8} />
              <span style={{ display: "inline-flex", alignItems: "center", gap: 7, minWidth: 0 }}>
                {f.manual && <Pill color={TOKEN.gold}>manual</Pill>}
                <span style={{ fontFamily: MONO, fontSize: 11.5, color: f.ok ? "var(--muted)" : "#b97a7a", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", minWidth: 0 }}>
                  {f.ok ? `ran in ${fmtMs(f.duration_ms)}` : (f.error ?? "graph execution failed")}
                </span>
                {hasIO && <span style={{ color: "var(--fainter)", flex: "none" }}>{expanded ? "▾" : "▸"}</span>}
              </span>
              <span style={{ display: "inline-flex" }}>
                <Pill color={statusColor(f.ok ? "completed" : "failed")}>{f.ok ? "completed" : "failed"}</Pill>
              </span>
              <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)", textAlign: "right" }}>{formatAgo(f.fired_at)}</span>
            </div>
            {hasIO && (
              <div style={{ marginLeft: 28, marginTop: 6, display: "flex", flexDirection: "column", gap: 4 }}>
                {inputs.map(([src, val]) => (
                  <IOLine key={`in-${src}`} kind="in" label={src} value={val} expanded={expanded} />
                ))}
                {outputs.map((val, j) => (
                  <IOLine key={`out-${j}`} kind="out" label={outputs.length > 1 ? `${j}` : undefined} value={val} expanded={expanded} />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
