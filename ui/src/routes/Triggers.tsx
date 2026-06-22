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

import { ActionIcon, Button, Group, Table, Tooltip } from "@mantine/core";
import { IconPlayerPlay } from "@tabler/icons-react";
import cronstrue from "cronstrue";
import { type CSSProperties } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";

import { useTriggers } from "../api/triggers";
import { useExecuteWorkflow } from "../api/workflows";
import { Dot, MONO, PageHeader } from "../components/aurora";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";
import { formatPollInterval } from "../util/triggers";
import { TOKEN, pillBg } from "../util/tokens";

const PAGE_SIZE = 50;

function humanizeCron(expr: string): string {
  try {
    return cronstrue.toString(expr, { verbose: false });
  } catch {
    return expr;
  }
}

function TypePill({ cron }: { cron: boolean }) {
  const color = cron ? TOKEN.violet : TOKEN.teal;
  return (
    <span style={{ background: pillBg(color), color, borderRadius: 10, padding: "2px 9px", fontFamily: MONO, fontSize: 10.5 }}>
      {cron ? "cron" : "poll"}
    </span>
  );
}

/** Triggers/schedules list (Aurora Dark, spec 07). */
export function Triggers() {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const offset = Math.max(0, Number(params.get("offset") ?? "0") || 0);
  const execute = useExecuteWorkflow();

  const { data, isPending, isError, error, refetch } = useTriggers({ limit: PAGE_SIZE, offset });

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

  const items = data?.items ?? [];
  const count = items.length;
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
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <PageHeader title="Triggers" sub={`${count} schedule${count === 1 ? "" : "s"}`} />

      {isPending ? (
        <Loading label="Loading schedules…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : count === 0 ? (
        <Empty message={offset > 0 ? "No more schedules." : "No schedules."} />
      ) : (
        <>
          <Table verticalSpacing={10} highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th style={th}>Workflow</Table.Th>
                <Table.Th style={th}>Type</Table.Th>
                <Table.Th style={th}>Schedule</Table.Th>
                <Table.Th style={th}>State</Table.Th>
                <Table.Th style={th}>Next run</Table.Th>
                <Table.Th style={th}>Last run</Table.Th>
                <Table.Th style={th} />
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {items.map((t) => (
                <Table.Tr
                  key={t.id}
                  style={{ cursor: "pointer" }}
                  onClick={() => navigate(`/triggers/${encodeURIComponent(t.trigger_name ?? t.workflow_name)}`)}
                >
                  <Table.Td>
                    <span style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)" }}>{t.workflow_name}</span>
                  </Table.Td>
                  <Table.Td>
                    <TypePill cron={!!t.cron_expression} />
                  </Table.Td>
                  <Table.Td>
                    <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--fg-2)" }}>
                      {t.cron_expression
                        ? humanizeCron(t.cron_expression)
                        : t.poll_interval_ms != null
                          ? `every ${formatPollInterval(t.poll_interval_ms)}`
                          : (t.trigger_name ?? "—")}
                    </span>
                  </Table.Td>
                  <Table.Td>
                    <Group gap={6} wrap="nowrap">
                      <Dot color={t.enabled ? TOKEN.ok : TOKEN.faint} size={7} />
                      <span style={{ fontSize: 12, color: t.enabled ? "var(--fg-2)" : "var(--faint)" }}>
                        {t.enabled ? "enabled" : "disabled"}
                      </span>
                    </Group>
                  </Table.Td>
                  <Table.Td>
                    <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>{formatTimestamp(t.next_run_at)}</span>
                  </Table.Td>
                  <Table.Td>
                    <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>{formatTimestamp(t.last_run_at)}</span>
                  </Table.Td>
                  <Table.Td>
                    <Tooltip label={`Run ${t.workflow_name} now`} withArrow>
                      <ActionIcon
                        variant="subtle"
                        color="ice"
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
              ))}
            </Table.Tbody>
          </Table>

          <Group justify="flex-end">
            <Button variant="default" size="xs" disabled={offset === 0} onClick={() => page(-1)}>
              Previous
            </Button>
            <Button variant="default" size="xs" disabled={count < PAGE_SIZE} onClick={() => page(1)}>
              Next
            </Button>
          </Group>
        </>
      )}
    </div>
  );
}
