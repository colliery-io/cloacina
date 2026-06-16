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

import { Button, Group, Select, Stack, Table, Text, TextInput, Title, Tooltip } from "@mantine/core";
import { useNavigate, useSearchParams } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { StatusBadge } from "../components/StatusBadge";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatDuration, formatTimestamp } from "../util/format";
import { formatAgo } from "../util/activity";

const STATUS_OPTIONS = ["Running", "Completed", "Failed", "Pending", "Scheduled", "Cancelled"];

const PAGE_SIZE = 50;

/**
 * Executions list (T-0653 / REQ-004 non-live half). Filters (status,
 * workflow) and pagination are URL-reflected via search params so they're
 * linkable and back-button-safe — the `?status=Failed` debug entry point
 * (UC-2). Status is visually distinct via `StatusBadge`.
 */
export function Executions() {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();

  const status = params.get("status") ?? "";
  const workflow = params.get("workflow") ?? "";
  const offset = Math.max(0, Number(params.get("offset") ?? "0") || 0);

  const { data, isPending, isError, error, refetch } = useExecutions({
    status: status || undefined,
    workflow: workflow || undefined,
    limit: PAGE_SIZE,
    offset,
  });

  function setParam(key: string, value: string) {
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value) next.set(key, value);
        else next.delete(key);
        if (key !== "offset") next.delete("offset"); // reset paging on filter change
        return next;
      },
      { replace: true },
    );
  }

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

  const pageCount = data?.items.length ?? 0;

  return (
    <Stack>
      <Title order={2}>Executions</Title>

      <Group align="flex-end">
        <Select
          label="Status"
          placeholder="All statuses"
          data={STATUS_OPTIONS}
          value={status || null}
          onChange={(v) => setParam("status", v ?? "")}
          clearable
          w={180}
        />
        <TextInput
          label="Workflow"
          placeholder="workflow name"
          value={workflow}
          onChange={(e) => setParam("workflow", e.currentTarget.value)}
          w={220}
        />
      </Group>

      {isPending ? (
        <Loading label="Loading executions…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : pageCount === 0 ? (
        <Empty message={offset > 0 ? "No more executions." : "No executions match."} />
      ) : (
        <>
          <Table highlightOnHover stickyHeader>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Workflow</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Started</Table.Th>
                <Table.Th>Duration</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {data.items.map((e) => (
                <Table.Tr
                  key={e.id}
                  style={{ cursor: "pointer" }}
                  onClick={() => navigate(`/executions/${e.id}`)}
                >
                  <Table.Td>
                    <Text fw={500}>{e.workflow_name}</Text>
                    <Text size="xs" c="dimmed">
                      {e.id}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <StatusBadge status={e.status} />
                  </Table.Td>
                  <Table.Td style={{ whiteSpace: "nowrap" }}>
                    <Tooltip label={formatTimestamp(e.started_at)} withArrow openDelay={300}>
                      <Text size="sm" c="dimmed">
                        {formatAgo(e.started_at)}
                      </Text>
                    </Tooltip>
                  </Table.Td>
                  <Table.Td style={{ whiteSpace: "nowrap" }}>
                    <Text size="sm">{formatDuration(e.started_at, e.completed_at)}</Text>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>

          <Group justify="flex-end">
            <Button variant="default" size="xs" disabled={offset === 0} onClick={() => page(-1)}>
              Previous
            </Button>
            <Button
              variant="default"
              size="xs"
              disabled={pageCount < PAGE_SIZE}
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
