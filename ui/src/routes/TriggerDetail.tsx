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

import { Anchor, Badge, Card, Group, Stack, Table, Text, Title } from "@mantine/core";
import { Link, useParams } from "react-router-dom";

import { useTrigger } from "../api/triggers";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";

/**
 * Trigger detail (T-0654 / REQ-005): schedule fields + recent executions.
 *
 * Note: `recent_executions` rows carry only a *schedule-execution* id, not
 * the workflow-execution id `/executions/:id` needs — so they're shown
 * informationally without a deep-link. Wiring that link would need the
 * server's trigger-detail to expose the workflow_execution_id (SDK/server
 * gap, noted in the task).
 */
export function TriggerDetail() {
  const { name = "" } = useParams();
  const { data, isPending, isError, error, refetch } = useTrigger(name);

  return (
    <Stack>
      <div>
        <Anchor component={Link} to="/triggers" size="sm">
          ← Triggers
        </Anchor>
        <Title order={2}>{name}</Title>
      </div>

      {isPending ? (
        <Loading label="Loading schedule…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : !data ? (
        <Empty message="Trigger not found." />
      ) : (
        <>
          <Card withBorder padding="lg">
            <Stack gap="sm">
              <Group>
                <Badge
                  variant="light"
                  color={data.schedule.schedule_type === "cron" ? "grape" : "teal"}
                >
                  {data.schedule.schedule_type}
                </Badge>
                <Badge variant="dot" color={data.schedule.enabled ? "green" : "gray"}>
                  {data.schedule.enabled ? "enabled" : "disabled"}
                </Badge>
              </Group>
              <Text size="sm">
                <b>Workflow:</b> {data.schedule.workflow_name}
              </Text>
              {data.schedule.cron_expression && (
                <Text size="sm">
                  <b>Cron:</b> {data.schedule.cron_expression}
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
