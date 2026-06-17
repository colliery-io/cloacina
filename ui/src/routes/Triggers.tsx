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

import { ActionIcon, Badge, Button, Group, Stack, Table, Text, Title, Tooltip } from "@mantine/core";
import { IconPlayerPlay } from "@tabler/icons-react";
import cronstrue from "cronstrue";
import { useNavigate, useSearchParams } from "react-router-dom";

import { useTriggers } from "../api/triggers";
import { useExecuteWorkflow } from "../api/workflows";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";
import { describeTriggerKind, formatPollInterval } from "../util/triggers";

const PAGE_SIZE = 50;

// Humanize a cron expression (e.g. a 6-field "seconds" cron → "Every 15
// seconds"); falls back to the raw expression if it can't be parsed.
function humanizeCron(expr: string): string {
  try {
    return cronstrue.toString(expr, { verbose: false });
  } catch {
    return expr;
  }
}

/**
 * Triggers/schedules list (T-0654 / REQ-005). Server-paginated (limit/offset),
 * URL-reflected. A schedule is identified by its workflow name or trigger
 * name — the detail route keys off that.
 */
export function Triggers() {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const offset = Math.max(0, Number(params.get("offset") ?? "0") || 0);
  const execute = useExecuteWorkflow();

  const { data, isPending, isError, error, refetch } = useTriggers({
    limit: PAGE_SIZE,
    offset,
  });

  function page(delta: number) {
    const next = Math.max(0, offset + delta * PAGE_SIZE);
    setParams(
      (prev) => {
        const p = new URLSearchParams(prev);
        if (next === 0) p.delete("offset");
        else p.set("offset", String(next));
        return p;
      },
      { replace: true },
    );
  }

  const count = data?.items.length ?? 0;

  return (
    <Stack>
      <Title order={2}>Triggers</Title>
      {isPending ? (
        <Loading label="Loading schedules…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : count === 0 ? (
        <Empty message={offset > 0 ? "No more schedules." : "No schedules."} />
      ) : (
        <>
          <Table highlightOnHover stickyHeader>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Workflow</Table.Th>
                <Table.Th>Type</Table.Th>
                <Table.Th>Schedule</Table.Th>
                <Table.Th>Enabled</Table.Th>
                <Table.Th>Next run</Table.Th>
                <Table.Th>Last run</Table.Th>
                <Table.Th />
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {data.items.map((t) => {
                const kind = describeTriggerKind(t.schedule_type);
                return (
                <Table.Tr
                  key={t.id}
                  style={{ cursor: "pointer" }}
                  onClick={() =>
                    navigate(`/triggers/${encodeURIComponent(t.trigger_name ?? t.workflow_name)}`)
                  }
                >
                  <Table.Td>
                    <Text fw={500}>{t.workflow_name}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Tooltip label={kind.tip} disabled={!kind.tip} multiline w={260} withArrow>
                      <Badge variant="light" color={kind.color}>
                        {kind.label}
                      </Badge>
                    </Tooltip>
                  </Table.Td>
                  <Table.Td>
                    {t.cron_expression ? (
                      <Tooltip label={t.cron_expression} withArrow openDelay={300}>
                        <Text size="sm">{humanizeCron(t.cron_expression)}</Text>
                      </Tooltip>
                    ) : (
                      <Text size="sm">
                        {t.poll_interval_ms != null
                          ? `every ${formatPollInterval(t.poll_interval_ms)}`
                          : (t.trigger_name ?? "—")}
                      </Text>
                    )}
                  </Table.Td>
                  <Table.Td>
                    <Badge variant="dot" color={t.enabled ? "green" : "gray"}>
                      {t.enabled ? "enabled" : "disabled"}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{formatTimestamp(t.next_run_at)}</Table.Td>
                  <Table.Td>{formatTimestamp(t.last_run_at)}</Table.Td>
                  <Table.Td>
                    <Tooltip label={`Run ${t.workflow_name} now`} withArrow>
                      <ActionIcon
                        variant="subtle"
                        color="gray"
                        loading={execute.isPending && execute.variables?.name === t.workflow_name}
                        onClick={(ev) => {
                          ev.stopPropagation();
                          execute.mutate(
                            { name: t.workflow_name },
                            { onSuccess: (res) => navigate(`/executions/${res.execution_id}`) },
                          );
                        }}
                      >
                        <IconPlayerPlay size={16} />
                      </ActionIcon>
                    </Tooltip>
                  </Table.Td>
                </Table.Tr>
                );
              })}
            </Table.Tbody>
          </Table>

          <Group justify="flex-end">
            <Button variant="default" size="xs" disabled={offset === 0} onClick={() => page(-1)}>
              Previous
            </Button>
            <Button
              variant="default"
              size="xs"
              disabled={count < PAGE_SIZE}
              onClick={() => page(1)}
            >
              Next
            </Button>
          </Group>
        </>
      )}
    </Stack>
  );
}
