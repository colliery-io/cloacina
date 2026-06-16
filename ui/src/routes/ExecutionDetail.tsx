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

import { Anchor, Badge, Button, Card, Group, Stack, Text, Title } from "@mantine/core";
import { useEffect } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import {
  useExecution,
  useExecutionEvents,
  useExecutionTasks,
  useLiveExecutionEvents,
} from "../api/executions";
import { EventLog } from "../components/EventLog";
import { StatusBadge } from "../components/StatusBadge";
import { TaskTable } from "../components/TaskTable";
import { ErrorState, Loading } from "../components/states/States";
import { useExecuteWorkflow, useWorkflow } from "../api/workflows";
import { mergeEvents } from "../util/events";
import { formatDuration, formatTimestamp } from "../util/format";
import { isTerminalStatus } from "../util/status";
import { topoRank } from "../util/topo";

/**
 * Execution detail (T-0653 + T-0656). Non-live half shows the REST event
 * log; the live half tails the delivery WS while the run is in progress
 * and merges into the same log.
 *
 * OQ-6 merge: REST history (`useExecutionEvents`) is the backfill; the live
 * tail (`useLiveExecutionEvents`) is layered on top, deduped on
 * `sequence_num` by `mergeEvents`. The status is polled (`livePoll`) so the
 * badge transitions to terminal, at which point the stream tears down and
 * the REST log is refetched for the authoritative final history.
 */
export function ExecutionDetail() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const detail = useExecution(id, { livePoll: true });
  const events = useExecutionEvents(id);

  const terminal = detail.data ? isTerminalStatus(detail.data.status) : true;
  const liveEvents = useLiveExecutionEvents(id, !terminal);
  const tasks = useExecutionTasks(id, { poll: !terminal });

  // Fixed nominal run order for the task table: pull the workflow's task DAG and
  // topologically rank it. The package name is the 2nd segment of a task's
  // namespaced id (`tenant::package::workflow::task`). Fetched once the tasks
  // load; the table falls back to created_at order until the graph arrives.
  const taskParts = tasks.data?.tasks[0]?.task_name.split("::");
  const pkgName = taskParts?.[1] ?? "";
  // Executable workflow name (the 3rd namespace segment) — the detail endpoint
  // doesn't return it, so derive it from a task's namespaced id (WS-12).
  const workflowName = taskParts?.[2] ?? "";
  const workflow = useWorkflow(pkgName, { enabled: !!pkgName });
  const taskOrder = workflow.data?.task_graph
    ? topoRank(workflow.data.task_graph)
    : undefined;

  // Execution start/end derived from the task rows (the detail endpoint exposes
  // only status). start = earliest task start; end = latest completion (or now
  // while running, for a live duration).
  const taskList = tasks.data?.tasks ?? [];
  const starts = taskList
    .map((t) => t.started_at ?? t.created_at)
    .filter(Boolean)
    .map((s) => Date.parse(s as string))
    .filter((n) => !Number.isNaN(n));
  const startedAt = starts.length ? new Date(Math.min(...starts)).toISOString() : null;
  const ends = taskList
    .map((t) => t.completed_at ?? t.updated_at)
    .filter(Boolean)
    .map((s) => Date.parse(s as string))
    .filter((n) => !Number.isNaN(n));
  const endedAt = terminal && ends.length ? new Date(Math.max(...ends)).toISOString() : null;

  const reExecute = useExecuteWorkflow();
  function onReRun() {
    if (!workflowName) return;
    reExecute.mutate(
      { name: workflowName },
      { onSuccess: (res) => navigate(`/executions/${res.execution_id}`) },
    );
  }

  // On the in-progress → terminal transition, refetch the REST log + task rows
  // so the final view is the server's authoritative history (not just what the
  // live tail happened to catch).
  useEffect(() => {
    if (terminal) {
      events.refetch();
      tasks.refetch();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terminal]);

  const merged = mergeEvents(events.data?.events ?? [], liveEvents);

  return (
    <Stack>
      <Group justify="space-between" align="flex-start">
        <div>
          <Anchor component={Link} to="/executions" size="sm">
            ← Executions
          </Anchor>
          <Title order={2}>{workflowName || "Execution"}</Title>
          <Text size="xs" c="dimmed">
            {id}
          </Text>
        </div>
        <Button
          size="sm"
          variant="light"
          onClick={onReRun}
          loading={reExecute.isPending}
          disabled={!workflowName}
        >
          Re-run
        </Button>
      </Group>

      {detail.isPending ? (
        <Loading label="Loading execution…" />
      ) : detail.isError ? (
        <ErrorState error={detail.error} onRetry={detail.refetch} />
      ) : (
        <Card withBorder padding="lg">
          <Group gap="xl">
            <span data-testid="execution-status">
              <StatusBadge status={detail.data.status} />
            </span>
            {!terminal && (
              <Badge color="blue" variant="dot">
                live
              </Badge>
            )}
            {workflowName && (
              <Field label="Workflow">
                <Text size="sm">{workflowName}</Text>
              </Field>
            )}
            <Field label="Started">
              <Text size="sm">{formatTimestamp(startedAt)}</Text>
            </Field>
            <Field label={terminal ? "Duration" : "Elapsed"}>
              <Text size="sm">{formatDuration(startedAt, endedAt)}</Text>
            </Field>
          </Group>
        </Card>
      )}

      <Card withBorder padding="lg">
        <Group justify="space-between" mb="sm">
          <Title order={4}>Tasks</Title>
          {!terminal && (
            <Text size="xs" c="blue">
              live
            </Text>
          )}
        </Group>
        {tasks.isPending ? (
          <Loading label="Loading tasks…" />
        ) : tasks.isError ? (
          <ErrorState error={tasks.error} onRetry={tasks.refetch} />
        ) : (
          <TaskTable tasks={tasks.data.tasks} order={taskOrder} />
        )}
      </Card>

      <Card withBorder padding="lg">
        <Group justify="space-between" mb="sm">
          <Title order={4}>Event log</Title>
          {!terminal && (
            <Text size="xs" c="blue">
              streaming…
            </Text>
          )}
        </Group>
        {events.isPending ? (
          <Loading label="Loading events…" />
        ) : events.isError ? (
          <ErrorState error={events.error} onRetry={events.refetch} />
        ) : (
          <EventLog events={merged} />
        )}
      </Card>
    </Stack>
  );
}

/** A small label-over-value pair for the execution summary row. */
function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <Text size="xs" c="dimmed">
        {label}
      </Text>
      {children}
    </div>
  );
}
