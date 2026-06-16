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

import { Anchor, Badge, Button, Card, Group, Stack, Table, Text, Title, Tooltip } from "@mantine/core";
import { Link, useNavigate, useParams } from "react-router-dom";

import { useTrigger } from "../api/triggers";
import { useExecuteWorkflow } from "../api/workflows";
import { classifyError } from "../api/errors";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";
import { describeTriggerKind, formatPollInterval } from "../util/triggers";

/**
 * Trigger detail (T-0654 / REQ-005; CLOACI-I-0124 / WS-6): schedule fields +
 * recent executions. Shows the meaningful trigger kind (cron / poll), the
 * cron expression or poll interval, and the workflow it fires — with a
 * **Run now** control that fires that workflow directly through the execute
 * endpoint.
 *
 * Enable/disable is read-only here: the server exposes no schedule-toggle
 * endpoint (it would be a new capability, an I-0124 non-goal), so the state
 * is shown but not editable.
 *
 * Note: `recent_executions` rows carry only a *schedule-execution* id, not
 * the workflow-execution id `/executions/:id` needs — so they're shown
 * informationally without a deep-link. Wiring that link would need the
 * server's trigger-detail to expose the workflow_execution_id (SDK/server
 * gap, noted in the task).
 */
export function TriggerDetail() {
  const { name = "" } = useParams();
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useTrigger(name);
  const execute = useExecuteWorkflow();

  function onRunNow() {
    const workflow = data?.schedule.workflow_name;
    if (!workflow) return;
    execute.mutate(
      { name: workflow },
      { onSuccess: (res) => navigate(`/executions/${res.execution_id}`) },
    );
  }

  const kind = data ? describeTriggerKind(data.schedule.schedule_type) : null;

  return (
    <Stack>
      <Group justify="space-between" align="flex-start">
        <div>
          <Anchor component={Link} to="/triggers" size="sm">
            ← Triggers
          </Anchor>
          <Title order={2}>{name}</Title>
        </div>
        {data && (
          <Button
            size="sm"
            loading={execute.isPending}
            onClick={onRunNow}
            disabled={!data.schedule.workflow_name}
          >
            Run now
          </Button>
        )}
      </Group>

      {isPending ? (
        <Loading label="Loading schedule…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : !data ? (
        <Empty message="Trigger not found." />
      ) : (
        <>
          {execute.isError && (
            <Text c="red" size="sm">
              {classifyError(execute.error).message}
            </Text>
          )}
          <Card withBorder padding="lg">
            <Stack gap="sm">
              <Group>
                {kind && (
                  <Tooltip label={kind.tip} disabled={!kind.tip} multiline w={260} withArrow>
                    <Badge variant="light" color={kind.color}>
                      {kind.label}
                    </Badge>
                  </Tooltip>
                )}
                <Tooltip
                  label="Schedule state is managed server-side; there is no enable/disable endpoint."
                  multiline
                  w={260}
                  withArrow
                >
                  <Badge variant="dot" color={data.schedule.enabled ? "green" : "gray"}>
                    {data.schedule.enabled ? "enabled" : "disabled"}
                  </Badge>
                </Tooltip>
              </Group>
              <Text size="sm">
                <b>Fires workflow:</b> {data.schedule.workflow_name}
              </Text>
              {data.schedule.cron_expression && (
                <Text size="sm">
                  <b>Cron:</b> {data.schedule.cron_expression}
                </Text>
              )}
              {data.schedule.poll_interval_ms != null && (
                <Text size="sm">
                  <b>Polls:</b> every {formatPollInterval(data.schedule.poll_interval_ms)}
                </Text>
              )}
              {data.schedule.trigger_name && (
                <Text size="sm">
                  <b>Trigger:</b> {data.schedule.trigger_name}
                </Text>
              )}
            </Stack>
          </Card>

          <Card withBorder padding="lg">
            <Title order={4} mb="sm">
              Recent executions
            </Title>
            {data.recent_executions.length === 0 ? (
              <Text c="dimmed" size="sm">
                No recent executions.
              </Text>
            ) : (
              <Table verticalSpacing="xs">
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>Scheduled</Table.Th>
                    <Table.Th>Started</Table.Th>
                    <Table.Th>Completed</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {data.recent_executions.map((e) => (
                    <Table.Tr key={e.id}>
                      <Table.Td>{formatTimestamp(e.scheduled_time)}</Table.Td>
                      <Table.Td>{formatTimestamp(e.started_at)}</Table.Td>
                      <Table.Td>{formatTimestamp(e.completed_at)}</Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>
            )}
          </Card>
        </>
      )}
    </Stack>
  );
}
