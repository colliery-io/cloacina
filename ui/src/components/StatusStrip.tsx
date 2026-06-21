/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Operational status strip (CLOACI-T-0764 §2): six derived metric cards —
 *  last run, success rate, in-flight, runtime p50/p95, next run, failures — all
 *  computed client-side from useExecutions({workflow}) + useTrigger(name).
 */
import { type ReactNode } from "react";

import { useExecutions } from "../api/executions";
import { useTriggers } from "../api/triggers";
import { Dot, MONO } from "./aurora";
import { durationMs, fmtMs, isRunning, isTerminalDone } from "./RunHeatmap";
import { formatAgo } from "../util/activity";
import { TOKEN, statusColor } from "../util/tokens";

function quantile(sorted: number[], q: number): number {
  if (sorted.length === 0) return 0;
  const pos = (sorted.length - 1) * q;
  const base = Math.floor(pos);
  const rest = pos - base;
  return sorted[base + 1] !== undefined ? sorted[base] + rest * (sorted[base + 1] - sorted[base]) : sorted[base];
}

function nextRunCountdown(iso: string | null | undefined): string {
  if (!iso) return "—";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "—";
  const ms = t - Date.now();
  if (ms <= 0) return "due";
  const m = Math.floor(ms / 60_000);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ${m % 60}m`;
  return `${Math.floor(h / 24)}d ${h % 24}h`;
}

function Card({ label, value, color, dot, sub }: { label: string; value: ReactNode; color?: string; dot?: string; sub: ReactNode }) {
  return (
    <div style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 10, padding: "13px 15px" }}>
      <div style={{ fontFamily: MONO, fontSize: 9.5, letterSpacing: ".09em", textTransform: "uppercase", color: "var(--muted)" }}>
        {label}
      </div>
      <div className="cl-tnum" style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 25, fontWeight: 600, color: color ?? "var(--fg)", margin: "7px 0 4px" }}>
        {dot && <Dot color={dot} />}
        {value}
      </div>
      <div style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>{sub}</div>
    </div>
  );
}

export function StatusStrip({ workflow }: { workflow: string }) {
  const { data } = useExecutions({ workflow: workflow || undefined, limit: 40 });
  const triggers = useTriggers({ limit: 200, offset: 0 });
  const sched = (triggers.data?.items ?? []).find(
    (t) => t.workflow_name === workflow || t.trigger_name === workflow,
  );

  const items = data?.items ?? [];
  const lower = (s: string) => s.toLowerCase();
  const terminal = items.filter((e) => isTerminalDone(e.status));
  const completed = terminal.filter((e) => lower(e.status) === "completed");
  const failed = terminal.filter((e) => lower(e.status) === "failed");
  const running = items.filter((e) => isRunning(e.status));

  const last = items[0];
  const successRate = completed.length + failed.length > 0
    ? Math.round((completed.length / (completed.length + failed.length)) * 100)
    : null;
  const durs = completed.map(durationMs).sort((a, b) => a - b);
  const p50 = quantile(durs, 0.5);
  const p95 = quantile(durs, 0.95);
  const cadence = sched?.cron_expression ? "cron" : sched?.poll_interval_ms != null ? "poll" : null;
  const lastFailed = failed[0];

  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 12 }}>
      <Card
        label="Last run"
        value={last ? lower(last.status) : "—"}
        color={last ? statusColor(last.status) : undefined}
        dot={last ? statusColor(last.status) : undefined}
        sub={last ? `${formatAgo(last.started_at)} · ${fmtMs(durationMs(last))}` : "no runs yet"}
      />
      <Card
        label="Success rate"
        value={successRate == null ? "—" : `${successRate}%`}
        color={successRate == null ? undefined : successRate >= 90 ? TOKEN.ok : TOKEN.gold}
        sub={`${completed.length} / ${completed.length + failed.length} runs`}
      />
      <Card
        label="In flight"
        value={running.length}
        color={running.length ? TOKEN.ice : "var(--faint)"}
        dot={running.length ? TOKEN.ice : undefined}
        sub={running.length ? `running · ${fmtMs(durationMs(running[running.length - 1]))}` : "idle"}
      />
      <Card label="Runtime p50" value={fmtMs(p50)} sub={`p95 ${fmtMs(p95)}`} />
      <Card
        label="Next run"
        value={nextRunCountdown(sched?.next_run_at)}
        color={TOKEN.violet}
        sub={cadence ? `${cadence} · ${sched?.cron_expression ?? `${sched?.poll_interval_ms}ms`}` : "manual / trigger only"}
      />
      <Card
        label="Failures"
        value={failed.length}
        color={failed.length ? TOKEN.bad : "var(--faint)"}
        sub={lastFailed ? `last ${formatAgo(lastFailed.started_at)}` : "none"}
      />
    </div>
  );
}
