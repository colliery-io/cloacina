/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Graph operational-view sections (CLOACI-T-0767): status strip, fire-activity
 *  heatmap, reactor readiness, accumulator freshness table, recent fires. These
 *  render on the instrumentation from T-0765 (accumulator freshness) + T-0766
 *  (reactor fires log / timeseries) plus the existing graph hooks.
 */
import { type CSSProperties, type ReactNode } from "react";
import { Tooltip } from "@mantine/core";

import { useReactorFires, useReactorFireTimeseries } from "../api/controls";
import { Dot, MONO, Panel, Pill } from "./aurora";
import { Empty, ErrorState, Loading } from "./states/States";
import { explainToken } from "../util/vocab";
import { formatAgo, useGraphThroughput } from "../util/activity";
import { healthColor, statusColor, TOKEN } from "../util/tokens";

/** Accumulator row as returned by `useAccumulators()` (CLOACI-T-0765 fields). */
export interface Acc {
  name: string;
  reactor?: string | null;
  state?: string | null;
  last_event_at?: string | null;
  events_total?: number | null;
  error?: string | null;
}

const STALE_MS = 30_000;

/** A source is stale if it's disconnected, never emitted, or hasn't emitted
 *  within the freshness window. */
export function accStale(a: Acc): boolean {
  const st = (a.state ?? "").toLowerCase();
  if (st === "disconnected" || st === "unreachable") return true;
  if (!a.last_event_at) return true;
  const age = Date.now() - Date.parse(a.last_event_at);
  return Number.isNaN(age) ? true : age > STALE_MS;
}

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

// ---- Degraded banner ----------------------------------------------------

export function DegradedBanner({ accumulators }: { accumulators: Acc[] }) {
  const stale = accumulators.filter(accStale);
  if (stale.length === 0) return null;
  const names = stale.map((a) => a.name).join(", ");
  const oldest = stale[0];
  const ago = oldest.last_event_at ? formatAgo(oldest.last_event_at) : "ever";
  return (
    <div style={{ background: "#d8a6571c", border: "1px solid #d8a65733", borderRadius: 9, padding: "10px 14px", display: "flex", gap: 10 }}>
      <span style={{ color: TOKEN.gold }}>⚠</span>
      <span style={{ fontSize: 12.5, color: "#e6c98a" }}>
        {stale.length} source{stale.length === 1 ? "" : "s"} degraded — <b>{names}</b> {stale.length === 1 ? "has" : "have"} no
        recent boundary data ({oldest.name} last seen {ago}). The graph still fires on the remaining sources,
        but that data is missing from output.
      </span>
    </div>
  );
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

// ---- Reactor readiness --------------------------------------------------

export function ReactorReadiness({
  reactor,
  reactionMode,
  inputStrategy,
  accumulators,
  lastFiredAt,
}: {
  reactor: string | null | undefined;
  reactionMode: string | null | undefined;
  inputStrategy: string | null | undefined;
  accumulators: Acc[];
  lastFiredAt: string | null | undefined;
}) {
  const mode = explainToken(reactionMode || "when_any").label;
  const inputLabel = explainToken(inputStrategy || "latest").label;
  const ready = accumulators.filter((a) => !accStale(a));
  const whenAll = (reactionMode || "").toLowerCase() === "when_all";

  return (
    <Panel title="Reactor readiness" caption={reactor ?? ""}>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 18, alignItems: "flex-start" }}>
        <div style={{ flex: "1 1 260px", minWidth: 200 }}>
          <span style={{ fontSize: 12.5, color: "var(--fg-2)" }}>
            Fires when <b style={{ color: TOKEN.violet }}>{mode.toLowerCase()}</b> bound accumulator has new data, passing each
            source's <b style={{ color: TOKEN.teal }}>{inputLabel.toLowerCase()}</b> value.
          </span>
        </div>
        <div style={{ flex: "1 1 220px", display: "flex", flexDirection: "column", gap: 8 }}>
          {accumulators.map((a) => {
            const fresh = !accStale(a);
            return (
              <div key={a.name} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ color: fresh ? TOKEN.ok : TOKEN.gold }}>{fresh ? "✓" : "⚠"}</span>
                <span style={{ fontFamily: MONO, fontSize: 12, color: "var(--fg-2)" }}>{a.name}</span>
                <span style={{ fontFamily: MONO, fontSize: 10.5, color: fresh ? "var(--faint)" : TOKEN.gold }}>
                  {fresh ? "fresh · ready" : "no data · stale"}
                </span>
              </div>
            );
          })}
        </div>
        <div style={{ flex: "1 1 220px", background: "var(--panel-2)", border: "1px solid var(--border-soft)", borderRadius: 9, padding: "11px 13px" }}>
          <div style={{ fontSize: 12.5, fontWeight: 600, color: ready.length === accumulators.length ? TOKEN.ok : TOKEN.gold }}>
            {whenAll && ready.length < accumulators.length
              ? `Waiting on ${accumulators.length - ready.length} of ${accumulators.length} sources`
              : `Firing on ${ready.length} of ${accumulators.length} sources`}
          </div>
          <div style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 5 }}>
            last fire {formatAgo(lastFiredAt)}
          </div>
        </div>
      </div>
    </Panel>
  );
}

// ---- Accumulator table --------------------------------------------------

export function AccumulatorTable({ accumulators, onInject }: { accumulators: Acc[]; onInject?: (name: string) => void }) {
  const rate = useGraphThroughput(accumulators.map((a) => ({ name: a.name, fires: a.events_total ?? 0 })));
  const th: CSSProperties = { fontFamily: MONO, fontSize: 9, letterSpacing: ".06em", textTransform: "uppercase", color: "var(--faint)", textAlign: "left" };
  const COLS = "minmax(0,1fr) 110px 120px 72px 150px 70px";

  if (accumulators.length === 0) return <Empty message="No accumulators bound." />;

  return (
    <div>
      <div style={{ display: "grid", gridTemplateColumns: COLS, gap: 12, paddingBottom: 9 }}>
        <span style={th}>Source</span>
        <span style={th}>State</span>
        <span style={th}>Last event</span>
        <span style={{ ...th, textAlign: "right" }}>Rate</span>
        <span style={th}>Freshness</span>
        <span style={th} />
      </div>
      {accumulators.map((a) => {
        const stale = accStale(a);
        const c = healthColor(a.state ?? "");
        const ageMs = a.last_event_at ? Date.now() - Date.parse(a.last_event_at) : Infinity;
        const freshPct = Number.isFinite(ageMs) ? Math.max(4, Math.min(100, 100 - (ageMs / STALE_MS) * 100)) : 0;
        const r = rate.get(a.name);
        return (
          <div key={a.name} style={{ borderTop: "1px solid var(--border-fainter)", padding: "8px 0" }}>
            <div style={{ display: "grid", gridTemplateColumns: COLS, gap: 12, alignItems: "center" }}>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 8, minWidth: 0 }}>
                <Dot color={c} size={7} />
                <span style={{ fontFamily: MONO, fontSize: 12.5, color: "#dce2e9", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{a.name}</span>
              </span>
              <span style={{ fontSize: 12, color: c }}>{explainToken(a.state ?? "unknown").label}</span>
              <span style={{ fontFamily: MONO, fontSize: 11.5, color: stale ? TOKEN.gold : "var(--muted)" }}>{formatAgo(a.last_event_at)}</span>
              <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--muted)", textAlign: "right" }}>{r == null ? "—" : `~${r}/min`}</span>
              <div style={{ height: 6, background: "var(--inset)", borderRadius: 3 }}>
                <div style={{ height: 6, width: `${freshPct}%`, background: stale ? TOKEN.bad : TOKEN.ok, borderRadius: 3 }} />
              </div>
              {onInject ? (
                <button
                  type="button"
                  onClick={() => onInject(a.name)}
                  style={{
                    fontFamily: MONO,
                    fontSize: 10,
                    padding: "3px 8px",
                    borderRadius: 7,
                    cursor: "pointer",
                    border: "1px solid var(--border-control)",
                    background: "var(--panel)",
                    color: TOKEN.ice,
                    justifySelf: "end",
                  }}
                >
                  inject ▸
                </button>
              ) : (
                <span />
              )}
            </div>
            {a.error && (
              <div style={{ fontFamily: MONO, fontSize: 11, marginTop: 4 }}>
                <span style={{ color: TOKEN.bad }}>✕ error</span> <span style={{ color: "#b97a7a" }}>{a.error}</span>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---- Recent fires -------------------------------------------------------

export function RecentFires({ reactor }: { reactor: string | null | undefined }) {
  const { data, isPending, isError, error, refetch } = useReactorFires(reactor, { limit: 30, poll: true });
  if (!reactor) return <Empty message="No reactor bound." />;
  if (isPending) return <Loading label="Loading fires…" />;
  if (isError) return <ErrorState error={error} onRetry={refetch} />;
  const fires = data?.items ?? [];
  if (fires.length === 0) return <Empty message="No fires recorded yet." />;

  return (
    <div style={{ display: "flex", flexDirection: "column" }}>
      {fires.map((f, i) => (
        <div key={i} style={{ display: "grid", gridTemplateColumns: "18px 1fr 80px 64px", gap: 10, alignItems: "center", padding: "8px 0", borderTop: i === 0 ? "none" : "1px solid var(--border-fainter)" }}>
          <Dot color={f.ok ? TOKEN.ok : TOKEN.bad} size={8} />
          <span style={{ fontFamily: MONO, fontSize: 11.5, color: f.ok ? "var(--muted)" : "#b97a7a", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {f.ok ? `ran in ${fmtMs(f.duration_ms)}` : (f.error ?? "graph execution failed")}
          </span>
          <span style={{ display: "inline-flex" }}>
            <Pill color={statusColor(f.ok ? "completed" : "failed")}>{f.ok ? "completed" : "failed"}</Pill>
          </span>
          <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)", textAlign: "right" }}>{formatAgo(f.fired_at)}</span>
        </div>
      ))}
    </div>
  );
}
