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

import {
  Alert,
  Anchor,
  Button,
  Card,
  Group,
  List,
  Modal,
  Stack,
  Text,
  Textarea,
  Title,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { useDeleteWorkflow, useExecuteWorkflow, useWorkflow } from "../api/workflows";
import { useWorkflowTaskRuntimes } from "../api/executions";
import { BuildStatusBadge } from "../components/BuildStatusBadge";
import { CombinedTimeline } from "../components/CombinedTimeline";
import { TaskRuntimeChart } from "../components/TaskRuntimeChart";
import { WorkflowGraph } from "../components/WorkflowGraph";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { classifyError } from "../api/errors";
import { formatTimestamp } from "../util/format";
import { topoRank } from "../util/topo";

/**
 * Workflow detail (T-0652 read + T-0657 write). Execute (with optional JSON
 * context) → redirect to the new execution's detail (the UC-1 hand-off to
 * the live stream). Delete with a confirm. Errors surface typed (REQ-007).
 */
export function WorkflowDetail() {
  const { name = "" } = useParams();
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useWorkflow(name);

  const [execOpen, execModal] = useDisclosure(false);
  const [delOpen, delModal] = useDisclosure(false);
  const [contextText, setContextText] = useState("");
  const [contextErr, setContextErr] = useState<string | null>(null);

  const execute = useExecuteWorkflow();
  const del = useDeleteWorkflow();

  // Cross-run task-duration aggregate (Airflow "Task Duration"). Sampled over
  // the most recent runs and ordered by the DAG's topological rank so the chart
  // reads in nominal run order. Keyed by the registered workflow name.
  const RUNS_SAMPLED = 40;
  const runtimes = useWorkflowTaskRuntimes(data?.workflow_name ?? "", { runs: RUNS_SAMPLED });
  const runtimeRank = data?.task_graph ? topoRank(data.task_graph) : undefined;
  const runtimeStats = [...runtimes.stats].sort((a, b) => {
    const ar = runtimeRank?.get(a.taskName) ?? Number.MAX_SAFE_INTEGER;
    const br = runtimeRank?.get(b.taskName) ?? Number.MAX_SAFE_INTEGER;
    return ar - br || b.avgMs - a.avgMs;
  });

  function onExecute() {
    let context: unknown;
    const trimmed = contextText.trim();
    if (trimmed) {
      try {
        context = JSON.parse(trimmed);
      } catch {
        setContextErr("Context must be valid JSON.");
        return;
      }
    }
    setContextErr(null);
    // Execute by the registered workflow name, not the package name: the runner
    // registry is keyed by workflow name and the two differ under the standard
    // convention (package `demo-slow-rust` → workflow `demo_slow_workflow`).
    // Fall back to the package name for packages predating workflow-name
    // persistence. (CLOACI-T-0671)
    const execName = data?.workflow_name || name;
    execute.mutate(
      { name: execName, context },
      {
        onSuccess: (res) => {
          execModal.close();
          navigate(`/executions/${res.execution_id}`);
        },
      },
    );
  }

  function onDelete() {
    if (!data) return;
    del.mutate(
      { name, version: data.version },
      { onSuccess: () => navigate("/workflows") },
    );
  }

  return (
    <Stack>
      <Group justify="space-between" align="flex-start">
        <div>
          <Anchor component={Link} to="/workflows" size="sm">
            ← Workflows
          </Anchor>
          <Title order={2}>{name}</Title>
        </div>
        <Group gap="xs">
          <Button onClick={execModal.open} disabled={!data}>
            Execute
          </Button>
          <Button color="red" variant="light" onClick={delModal.open} disabled={!data}>
            Delete
          </Button>
        </Group>
      </Group>

      {isPending ? (
        <Loading label="Loading workflow…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : !data ? (
        <Empty message="Workflow not found." />
      ) : (
        <Card withBorder padding="lg">
          <Stack gap="md">
            <Group>
              <BuildStatusBadge status={data.build_status} />
              <Text c="dimmed" size="sm">
                v{data.version} · created {formatTimestamp(data.created_at)}
              </Text>
            </Group>
            {data.description && <Text>{data.description}</Text>}
            {data.build_error && (
              <Alert color="red" title="Build error" role="alert">
                <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
                  {data.build_error}
                </Text>
              </Alert>
            )}
            <div>
              <Text fw={600} mb="xs">
                Tasks ({data.tasks.length})
              </Text>
              {data.tasks.length === 0 ? (
                <Text c="dimmed" size="sm">
                  No tasks.
                </Text>
              ) : data.task_graph && data.task_graph.length > 0 ? (
                <WorkflowGraph tasks={data.task_graph} />
              ) : (
                <List size="sm">
                  {data.tasks.map((t) => (
                    <List.Item key={t}>{t}</List.Item>
                  ))}
                </List>
              )}
            </div>
          </Stack>
        </Card>
      )}

      {data && (
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Task runtimes</Title>
            <Text size="xs" c="dimmed">
              avg over last {runtimes.runsCounted} run{runtimes.runsCounted === 1 ? "" : "s"}
            </Text>
          </Group>
          {runtimes.isPending ? (
            <Loading label="Aggregating run durations…" />
          ) : runtimes.isError ? (
            <Text size="sm" c="dimmed">
              Couldn't load run history.
            </Text>
          ) : (
            <TaskRuntimeChart stats={runtimeStats} />
          )}
        </Card>
      )}

      {data && (
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Combined timeline</Title>
            <Text size="xs" c="dimmed">
              span &amp; wait distribution · last {runtimes.runsCounted} run
              {runtimes.runsCounted === 1 ? "" : "s"}
            </Text>
          </Group>
          {runtimes.isPending ? (
            <Loading label="Aligning run timelines…" />
          ) : runtimes.isError ? (
            <Text size="sm" c="dimmed">
              Couldn't load run history.
            </Text>
          ) : (
            <CombinedTimeline runs={runtimes.runs} graph={data.task_graph} />
          )}
        </Card>
      )}

      {/* Execute */}
      <Modal opened={execOpen} onClose={execModal.close} title={`Execute ${name}`}>
        <Stack>
          <Textarea
            label="Context (JSON, optional)"
            placeholder='{ "input": 42 }'
            autosize
            minRows={4}
            value={contextText}
            onChange={(e) => setContextText(e.currentTarget.value)}
            error={contextErr}
          />
          {execute.isError && (
            <Text c="red" size="sm">
              {classifyError(execute.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={execModal.close}>
              Cancel
            </Button>
            <Button loading={execute.isPending} onClick={onExecute}>
              Execute
            </Button>
          </Group>
        </Stack>
      </Modal>

      {/* Delete confirm */}
      <Modal opened={delOpen} onClose={delModal.close} title="Delete workflow?">
        <Stack>
          <Text size="sm">
            Unregister <b>{name}</b> v{data?.version}? This removes the package from the tenant.
          </Text>
          {del.isError && (
            <Text c="red" size="sm">
              {classifyError(del.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={delModal.close}>
              Cancel
            </Button>
            <Button color="red" loading={del.isPending} onClick={onDelete}>
              Delete
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
