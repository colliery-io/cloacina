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

import { Box, Button, Group, TextInput } from "@mantine/core";
import { useNavigate, useSearchParams } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { StatusBadge } from "../components/StatusBadge";
import { Chip, Dot, MONO, PageHeader, Pill, cardSurface } from "../components/aurora";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatDuration } from "../util/format";
import { formatAgo } from "../util/activity";
import { statusColor, TOKEN } from "../util/tokens";

const PAGE_SIZE = 50;

// Filter chips → the server `status` value they select ("" = All).
const CHIPS: { label: string; value: string }[] = [
  { label: "All", value: "" },
  { label: "Running", value: "Running" },
  { label: "Completed", value: "Completed" },
  { label: "Failed", value: "Failed" },
  { label: "Scheduled", value: "Scheduled" },
];

/**
 * Executions list (Aurora Dark, spec 03). URL-reflected status chips + a text
 * filter; rows are dark cards with a status dot + run id + pill + duration + ago.
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
        if (key !== "offset") next.delete("offset");
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

  const items = data?.items ?? [];
  const pageCount = items.length;
  const count = (s: string) => items.filter((e) => e.status.toLowerCase() === s).length;
  const running = count("running");
  const failed = count("failed");
  const total = data?.total ?? pageCount;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <PageHeader
        title="Executions"
        sub={`${total} runs · ${running} running · ${failed} failed`}
      />

      {/* Filter bar */}
      <Group justify="space-between" align="center" style={{ borderBottom: "1px solid var(--border-soft)", paddingBottom: 12 }}>
        <Group gap={8}>
          {CHIPS.map((c) => (
            <Chip
              key={c.value || "all"}
              label={c.label}
              count={c.value === "" ? total : count(c.value.toLowerCase())}
              active={status === c.value}
              onClick={() => setParam("status", c.value)}
            />
          ))}
        </Group>
        <TextInput
          placeholder="Filter by workflow or run id…"
          value={workflow}
          onChange={(e) => setParam("workflow", e.currentTarget.value)}
          w={240}
          size="xs"
        />
      </Group>

      {isPending ? (
        <Loading label="Loading executions…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : pageCount === 0 ? (
        <Empty message={offset > 0 ? "No more executions." : "No executions match."} />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {items.map((e) => (
            <Box
              key={e.id}
              onClick={() => navigate(`/executions/${e.id}`)}
              style={{ ...cardSurface, borderRadius: 10, padding: "11px 15px", cursor: "pointer" }}
            >
              <Group justify="space-between" wrap="nowrap">
                <Group gap={11} wrap="nowrap" style={{ minWidth: 0 }}>
                  <Dot color={statusColor(e.status)} glow={e.status.toLowerCase() === "running"} />
                  <Box style={{ minWidth: 0 }}>
                    <Box style={{ fontSize: 13.5, fontWeight: 600, color: "var(--fg)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {e.workflow_name}
                    </Box>
                    <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>{e.id}</Box>
                  </Box>
                </Group>
                <Group gap={18} wrap="nowrap" style={{ flex: "none" }}>
                  {(e as { trigger_origin?: string | null }).trigger_origin === "manual" && (
                    <Pill color={TOKEN.gold}>manual</Pill>
                  )}
                  <StatusBadge status={e.status} />
                  <Box className="cl-tnum" style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--fg-2)", width: 64, textAlign: "right" }}>
                    {formatDuration(e.started_at, e.completed_at)}
                  </Box>
                  <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--fainter)", width: 64, textAlign: "right" }}>
                    {formatAgo(e.started_at)}
                  </Box>
                </Group>
              </Group>
            </Box>
          ))}

          <Group justify="flex-end" mt={4}>
            <Button variant="default" size="xs" disabled={offset === 0} onClick={() => page(-1)}>
              Previous
            </Button>
            <Button variant="default" size="xs" disabled={pageCount < PAGE_SIZE} onClick={() => page(1)}>
              Next
            </Button>
          </Group>
        </div>
      )}
    </div>
  );
}
