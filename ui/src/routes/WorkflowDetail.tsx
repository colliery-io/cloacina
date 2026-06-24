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

import { BuildStatusBadge, classifyError, Empty, ErrorState, Loading, MONO, Panel, Pill, TOKEN } from "@colliery-io/aurora-dark";
import { Alert, Anchor, Box, Button, Group, List, Modal, Stack, Text } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { useDeleteWorkflow, useWorkflow } from "../api/workflows";
import { usePauseWorkflow } from "../api/controls";
import { useCan } from "../auth/AuthContext";
import { useWorkflowTaskRuntimes } from "../api/executions";
import { CombinedTimeline } from "../components/CombinedTimeline";
import { InputsCard } from "../components/InputsCard";
import { RunHeatmap } from "../components/RunHeatmap";
import { RunWorkflowModal } from "../components/RunWorkflowModal";
import { ScheduleCard } from "../components/ScheduleCard";
import { StatusStrip } from "../components/StatusStrip";
import { TaskCodeModal } from "../components/TaskCodeModal";
import { TaskHealthTable } from "../components/TaskHealthTable";
import { WorkflowGraph } from "../components/WorkflowGraph";
import { formatTimestamp } from "../util/format";
import { topoRank } from "../util/topo";

/**
 * Workflow detail — operational DAG view (CLOACI-T-0764). Restores the operator
 * questions the redesign dropped: is it green lately, what's running, when does
 * it next fire, which task is flaky. Status strip + recent-runs heatmap +
 * schedule/inputs + reliability-overlay DAG + task-health table + scheduler-wait
 * timeline, over the existing hooks. Execute (typed) / Pause / Delete from the
 * header.
 */
export function WorkflowDetail() {
  const { name = "" } = useParams();
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useWorkflow(name);

  const [execOpen, execModal] = useDisclosure(false);
  const [delOpen, delModal] = useDisclosure(false);
  const [codeTask, setCodeTask] = useState<string | null>(null);

  const del = useDeleteWorkflow();
  const pause = usePauseWorkflow();
  const { canWrite } = useCan();

  const RUNS = 40;
  const runtimes = useWorkflowTaskRuntimes(data?.workflow_name ?? "", { runs: RUNS });
  const rank = data?.task_graph ? topoRank(data.task_graph) : undefined;
  const stats = useMemo(
    () =>
      [...runtimes.stats].sort((a, b) => {
        const ar = rank?.get(a.taskName) ?? Number.MAX_SAFE_INTEGER;
        const br = rank?.get(b.taskName) ?? Number.MAX_SAFE_INTEGER;
        return ar - br || b.avgMs - a.avgMs;
      }),
    [runtimes.stats, rank],
  );

  // Per-task last status + failure counts drive the DAG overlay + health dots.
  const { statusByTask, failByTask } = useMemo(() => {
    const s: Record<string, string> = {};
    const f: Record<string, number> = {};
    for (const r of runtimes.stats) {
      if (r.lastStatus) s[r.taskName] = r.lastStatus;
      if (r.failCount > 0) f[r.taskName] = r.failCount;
    }
    return { statusByTask: s, failByTask: f };
  }, [runtimes.stats]);

  function onDelete() {
    if (!data) return;
    del.mutate({ name, version: data.version }, { onSuccess: () => navigate("/workflows") });
  }

  if (isPending) return <Loading label="Loading workflow…" />;
  if (isError) return <ErrorState error={error} onRetry={refetch} />;
  if (!data) return <Empty message="Workflow not found." />;

  const wfName = data.workflow_name || name;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
      {/* Header */}
      <Group justify="space-between" align="flex-start">
        <Box>
          <Anchor component={Link} to="/workflows" size="xs" c="dimmed" style={{ fontFamily: MONO }}>
            ← Workflows
          </Anchor>
          <Group gap={10} mt={3}>
            <span style={{ fontSize: 23, fontWeight: 600, color: "var(--fg-bright)", letterSpacing: "-.01em" }}>{name}</span>
            {data.paused && <Pill color={TOKEN.gold}>⏸ paused</Pill>}
          </Group>
          <Group gap={8} mt={5}>
            <BuildStatusBadge status={data.build_status} />
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>
              v{data.version} · created {formatTimestamp(data.created_at)} · workflow{" "}
              <span style={{ color: "var(--muted)" }}>{wfName}</span>
            </span>
          </Group>
        </Box>
        <Group gap={8}>
          {canWrite && (
            <Button color="ice" radius={8} styles={{ root: { color: "#0b0d10", fontWeight: 600 } }} onClick={execModal.open}>
              ▸ Execute
            </Button>
          )}
          {canWrite && (
            <Button
              variant="default"
              radius={8}
              loading={pause.isPending}
              onClick={() => pause.mutate({ name, paused: !data.paused })}
            >
              {data.paused ? "▸ Resume" : "⏸ Pause"}
            </Button>
          )}
          <Button color="bad" variant="subtle" radius={8} onClick={delModal.open}>
            Delete
          </Button>
        </Group>
      </Group>

      {/* Build error (if any) */}
      {data.build_error && (
        <Alert color="bad" title="Build error" role="alert">
          <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
            {data.build_error}
          </Text>
        </Alert>
      )}

      {/* Status strip */}
      <StatusStrip workflow={wfName} />

      {/* Schedule + Inputs */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 18, alignItems: "stretch" }}>
        <ScheduleCard workflow={wfName} />
        <InputsCard packageName={name} />
      </div>

      {/* Recent runs */}
      <Panel title="Recent runs" caption="last 40 · bar height = duration · hover for detail">
        <RunHeatmap workflow={wfName} />
      </Panel>

      {/* Task graph */}
      <Panel
        title="Task graph"
        right={
          <Group gap={14}>
            <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
              <span style={{ width: 8, height: 8, borderRadius: "50%", background: TOKEN.ok }} /> last run ok
            </span>
            <span style={{ fontFamily: MONO, fontSize: 10.5, color: TOKEN.gold }}>⚠ failures in window</span>
          </Group>
        }
      >
        {data.task_graph && data.task_graph.length > 0 ? (
          <>
            <WorkflowGraph
              tasks={data.task_graph}
              statusByTask={statusByTask}
              failByTask={failByTask}
              onNodeClick={(id) => setCodeTask(id)}
            />
            <Text size="xs" c="dimmed" mt={6}>
              Click a task to view its source.
            </Text>
          </>
        ) : data.tasks.length === 0 ? (
          <Text c="dimmed" size="sm">
            No tasks.
          </Text>
        ) : (
          <List size="sm">
            {data.tasks.map((t) => (
              <List.Item key={t}>{t}</List.Item>
            ))}
          </List>
        )}
      </Panel>

      {/* Task health */}
      <Panel title="Task health" caption="duration, failures & retries over last 40 runs">
        {runtimes.isPending ? (
          <Loading label="Aggregating task health…" />
        ) : runtimes.isError ? (
          <Text size="sm" c="dimmed">
            Couldn't load run history.
          </Text>
        ) : (
          <TaskHealthTable stats={stats} />
        )}
      </Panel>

      {/* Combined timeline */}
      <Panel title="Combined timeline" caption={`scheduler wait · last ${runtimes.runsCounted} run${runtimes.runsCounted === 1 ? "" : "s"}`}>
        {runtimes.isPending ? (
          <Loading label="Aligning run timelines…" />
        ) : runtimes.isError ? (
          <Text size="sm" c="dimmed">
            Couldn't load run history.
          </Text>
        ) : (
          <CombinedTimeline runs={runtimes.runs} graph={data.task_graph} />
        )}
      </Panel>

      {/* Modals */}
      <RunWorkflowModal opened={execOpen} packageName={name} workflowName={wfName} onClose={execModal.close} />
      <TaskCodeModal opened={codeTask !== null} packageName={name} taskName={codeTask ?? ""} onClose={() => setCodeTask(null)} />

      <Modal opened={delOpen} onClose={delModal.close} title="Delete workflow?" centered>
        <Stack>
          <Text size="sm">
            Unregister <b>{name}</b> v{data.version}? This removes the package from the tenant.
          </Text>
          {del.isError && (
            <Text c="bad" size="sm">
              {classifyError(del.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={delModal.close}>
              Cancel
            </Button>
            <Button color="bad" loading={del.isPending} onClick={onDelete}>
              Delete
            </Button>
          </Group>
        </Stack>
      </Modal>
    </div>
  );
}
