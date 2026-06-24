/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Task health table (CLOACI-T-0764 §7): per-task mean + min–max duration plus
 *  FAILS and RETRIES over the sampled window. Replaces the bare runtime chart.
 *  Rows are pre-ordered by DAG topological rank by the caller.
 */
import { Dot, MONO, statusColor, TOKEN } from "@colliery-io/aurora-dark";
import { type TaskRuntimeStat } from "../api/executions";
import { fmtMs } from "./RunHeatmap";

const ICE = "#7fb2ff";
const COLS = "200px 1fr 56px 56px 56px 60px";

function HeaderCell({ children, right }: { children: string; right?: boolean }) {
  return (
    <span style={{ fontFamily: MONO, fontSize: 9, letterSpacing: ".06em", textTransform: "uppercase", color: "var(--faint)", textAlign: right ? "right" : "left" }}>
      {children}
    </span>
  );
}

export function TaskHealthTable({ stats }: { stats: TaskRuntimeStat[] }) {
  if (stats.length === 0) {
    return <div style={{ color: "var(--faint)", fontSize: 12.5 }}>No completed runs in the sampled window yet.</div>;
  }
  const scaleMax = Math.max(1, ...stats.map((s) => s.maxMs));

  return (
    <div>
      <div style={{ display: "grid", gridTemplateColumns: COLS, gap: 10, alignItems: "center", padding: "0 0 9px" }}>
        <HeaderCell>Task</HeaderCell>
        <HeaderCell>Duration · mean + min–max</HeaderCell>
        <HeaderCell right>Avg</HeaderCell>
        <HeaderCell right>P95</HeaderCell>
        <HeaderCell right>Fails</HeaderCell>
        <HeaderCell right>Retries</HeaderCell>
      </div>
      {stats.map((s) => {
        const meanPct = (s.avgMs / scaleMax) * 100;
        const minPct = (s.minMs / scaleMax) * 100;
        const maxPct = (s.maxMs / scaleMax) * 100;
        return (
          <div
            key={s.taskName}
            style={{ display: "grid", gridTemplateColumns: COLS, gap: 10, alignItems: "center", padding: "8px 0", borderTop: "1px solid var(--border-fainter)" }}
          >
            <span style={{ display: "inline-flex", alignItems: "center", gap: 8, minWidth: 0 }}>
              <Dot color={statusColor(s.lastStatus ?? "")} size={7} />
              <span style={{ fontFamily: MONO, fontSize: 12, color: "var(--fg-2)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.taskName}</span>
            </span>

            {/* Duration: min–max whisker band + mean bar from 0, shared scale. */}
            <div style={{ position: "relative", height: 14, background: "var(--inset)", borderRadius: 3 }}>
              <div style={{ position: "absolute", top: 5, height: 4, left: `${minPct}%`, width: `${Math.max(0, maxPct - minPct)}%`, background: "rgba(127,178,255,.5)", borderRadius: 2 }} />
              <div style={{ position: "absolute", top: 2, bottom: 2, width: 1.5, left: `${minPct}%`, background: "rgba(127,178,255,.7)" }} />
              <div style={{ position: "absolute", top: 2, bottom: 2, width: 1.5, left: `calc(${maxPct}% - 1.5px)`, background: "rgba(127,178,255,.7)" }} />
              <div style={{ position: "absolute", top: 3, bottom: 3, left: 0, width: `${meanPct}%`, background: ICE, borderRadius: 2 }} />
            </div>

            <span style={{ fontFamily: MONO, fontSize: 11.5, color: "#dce2e9", textAlign: "right" }}>{s.count ? fmtMs(s.avgMs) : "—"}</span>
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--muted)", textAlign: "right" }}>{s.count ? fmtMs(s.maxMs) : "—"}</span>
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: s.failCount > 0 ? TOKEN.bad : "var(--faint)", textAlign: "right" }}>{s.failCount}</span>
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: s.retrySum > 0 ? TOKEN.gold : "var(--faint)", textAlign: "right" }}>{s.retrySum}</span>
          </div>
        );
      })}
    </div>
  );
}
