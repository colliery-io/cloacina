/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Active-execution card for the Overview (Aurora Dark spec 01). Workflow name +
 *  status pill + Pause/Resume, a mini task-DAG, and a Mono meta line
 *  `N/M tasks · {elapsed} · on {currentTask}`. Pause/Resume on a running
 *  execution is a spec-declared mock (#3) — local toggle, no endpoint yet.
 */
import { Box, Button, Group } from "@mantine/core";
import { type CSSProperties, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import { useExecutionTasks } from "../api/executions";
import { useWorkflow } from "../api/workflows";
import { MiniDag, type MiniNode } from "./MiniDag";
import { StatusBadge } from "./StatusBadge";
import { statusColor } from "../util/tokens";
import { formatDuration } from "../util/format";

const MONO = "'IBM Plex Mono', monospace";
const localId = (n: string) => n.split("::").pop() || n;

const cardStyle: CSSProperties = {
  background: "var(--panel)",
  border: "1px solid var(--border)",
  borderRadius: 10,
  padding: "13px 15px",
  cursor: "pointer",
};

export interface ActiveExecution {
  id: string;
  workflow_name: string;
  status: string;
  started_at?: string | null;
}

export function ActiveRunCard({ execution: e }: { execution: ActiveExecution }) {
  const navigate = useNavigate();
  const tasks = useExecutionTasks(e.id, { poll: true });
  const taskList = tasks.data?.tasks ?? [];
  const pkg = taskList[0]?.task_name.split("::")[1] ?? "";
  const wf = useWorkflow(pkg, { enabled: !!pkg });
  const [paused, setPaused] = useState(false);

  const statusByTask = useMemo(() => {
    const m: Record<string, string> = {};
    for (const t of taskList) m[localId(t.task_name)] = t.status;
    return m;
  }, [taskList]);

  const nodes: MiniNode[] = useMemo(
    () =>
      (wf.data?.task_graph ?? []).map((n) => ({
        id: n.id,
        dependencies: n.dependencies,
        status: statusByTask[n.id],
      })),
    [wf.data, statusByTask],
  );

  const total = taskList.length || nodes.length;
  const done = taskList.filter((t) => t.status.toLowerCase() === "completed").length;
  const running = taskList.find((t) => t.status.toLowerCase() === "running");
  const starts = taskList
    .map((t) => t.started_at ?? t.created_at)
    .filter(Boolean)
    .map((s) => Date.parse(s as string))
    .filter((n) => !Number.isNaN(n));
  const startedAt = starts.length ? new Date(Math.min(...starts)).toISOString() : (e.started_at ?? null);

  return (
    <Box style={cardStyle} onClick={() => navigate(`/executions/${e.id}`)}>
      <Group justify="space-between" mb={9}>
        <Group gap={8}>
          <span style={{ fontSize: 13.5, fontWeight: 600, color: "var(--fg)" }}>{e.workflow_name}</span>
          <StatusBadge status={paused ? "paused" : e.status} />
        </Group>
        <Button
          size="compact-xs"
          variant="default"
          onClick={(ev) => {
            ev.stopPropagation();
            setPaused((p) => !p);
          }}
        >
          {paused ? "Resume" : "Pause"}
        </Button>
      </Group>

      {/* Mini task-DAG (falls back to a status strip until the graph loads). */}
      <Box style={{ overflowX: "auto", padding: "2px 0" }}>
        {nodes.length > 0 ? (
          <MiniDag nodes={nodes} />
        ) : (
          <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
            {taskList.map((t) => (
              <span
                key={t.id}
                title={`${localId(t.task_name)} · ${t.status}`}
                style={{ width: 9, height: 9, borderRadius: 2, background: statusColor(t.status) }}
              />
            ))}
          </div>
        )}
      </Box>

      <Box style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 9 }}>
        {done}/{total || "—"} tasks · {formatDuration(startedAt, null)}
        {running ? ` · on ${localId(running.task_name)}` : ""}
      </Box>
    </Box>
  );
}
