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

import { Anchor, Box, Button, Group, Stack, Table, Text, Tooltip } from "@mantine/core";
import { type CSSProperties, type ReactNode } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { useTrigger } from "../api/triggers";
import { useExecuteWorkflow } from "../api/workflows";
import { classifyError } from "../api/errors";
import { Dot, MONO, Pill, cardSurface } from "../components/aurora";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";
import { describeTriggerKind, formatPollInterval } from "../util/triggers";
import { TOKEN } from "../util/tokens";

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
  const isCron = !!data?.schedule.cron_expression;
  const th: CSSProperties = {
    fontFamily: MONO,
    fontSize: 10,
    letterSpacing: ".07em",
    textTransform: "uppercase",
    color: "var(--faint)",
    fontWeight: 500,
    textAlign: "left",
  };

  return (
    <Stack>
      <Group justify="space-between" align="flex-start">
        <Box>
          <Anchor component={Link} to="/triggers" size="xs" c="dimmed">
            ← Triggers
          </Anchor>
          <Box style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)", marginTop: 2 }}>{name}</Box>
        </Box>
        {data && (
          <Button
            size="sm"
            color="ice"
            radius={9}
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            loading={execute.isPending}
            onClick={onRunNow}
            disabled={!data.schedule.workflow_name}
          >
            ▸ Run now
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
            <Text c="bad" size="sm">
              {classifyError(execute.error).message}
            </Text>
          )}
          <Box style={{ ...cardSurface, padding: "15px 18px" }}>
            <Stack gap="sm">
              <Group gap={10}>
                {kind && (
                  <Tooltip label={kind.tip} disabled={!kind.tip} multiline w={260} withArrow>
                    <span style={{ display: "inline-flex" }}>
                      <Pill color={isCron ? TOKEN.violet : TOKEN.teal}>{kind.label}</Pill>
                    </span>
                  </Tooltip>
                )}
                <Tooltip
                  label="Schedule state is managed server-side; there is no enable/disable endpoint."
                  multiline
                  w={260}
                  withArrow
                >
                  <Group gap={6} wrap="nowrap">
                    <Dot color={data.schedule.enabled ? TOKEN.ok : TOKEN.faint} size={7} />
                    <span style={{ fontSize: 12, color: data.schedule.enabled ? "var(--fg-2)" : "var(--faint)" }}>
                      {data.schedule.enabled ? "enabled" : "disabled"}
                    </span>
                  </Group>
                </Tooltip>
              </Group>
              <Field label="Fires workflow" value={data.schedule.workflow_name} mono />
              {data.schedule.cron_expression && <Field label="Cron" value={data.schedule.cron_expression} mono />}
              {data.schedule.poll_interval_ms != null && (
                <Field label="Polls" value={`every ${formatPollInterval(data.schedule.poll_interval_ms)}`} mono />
              )}
              {data.schedule.trigger_name && <Field label="Trigger" value={data.schedule.trigger_name} mono />}
            </Stack>
          </Box>

          <Box>
            <Box style={{ fontSize: 14, fontWeight: 600, color: "var(--fg)", borderBottom: "1px solid var(--border-soft)", paddingBottom: 8, marginBottom: 10 }}>
              Recent executions
            </Box>
            {data.recent_executions.length === 0 ? (
              <Text c="dimmed" size="sm">
                No recent executions.
              </Text>
            ) : (
              <Table verticalSpacing={10}>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th style={th}>Scheduled</Table.Th>
                    <Table.Th style={th}>Started</Table.Th>
                    <Table.Th style={th}>Completed</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {data.recent_executions.map((e) => (
                    <Table.Tr key={e.id}>
                      <Table.Td><MonoCell>{formatTimestamp(e.scheduled_time)}</MonoCell></Table.Td>
                      <Table.Td><MonoCell>{formatTimestamp(e.started_at)}</MonoCell></Table.Td>
                      <Table.Td><MonoCell>{formatTimestamp(e.completed_at)}</MonoCell></Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>
            )}
          </Box>
        </>
      )}
    </Stack>
  );
}

/** Label : value row for the schedule card. */
function Field({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <Group gap={8}>
      <span style={{ fontFamily: MONO, fontSize: 10.5, letterSpacing: ".04em", textTransform: "uppercase", color: "var(--faint)" }}>{label}</span>
      <span style={{ fontFamily: mono ? MONO : undefined, fontSize: 12.5, color: "var(--fg-2)" }}>{value}</span>
    </Group>
  );
}

function MonoCell({ children }: { children: ReactNode }) {
  return <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>{children}</span>;
}
