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

import { Box, Button, Group } from "@mantine/core";
import { useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { useWorkflows } from "../api/workflows";
import { RunCircles, type RunDot } from "../components/RunCircles";
import { RunWorkflowModal } from "../components/RunWorkflowModal";
import { MONO, PageHeader, cardSurface } from "../components/aurora";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatAgo } from "../util/activity";
import { TOKEN, pillBg } from "../util/tokens";

function useRecentRunsByWorkflow(): Map<string, RunDot[]> {
  const recent = useExecutions({ limit: 200, offset: 0 });
  return useMemo(() => {
    const m = new Map<string, RunDot[]>();
    for (const e of recent.data?.items ?? []) {
      const arr = m.get(e.workflow_name) ?? [];
      arr.push({ id: e.id, status: e.status, started_at: e.started_at });
      m.set(e.workflow_name, arr);
    }
    return m;
  }, [recent.data]);
}

/** Workflows list (Aurora Dark, spec 06): package cards with version badge,
 *  run-history, and a Run action. */
export function Workflows() {
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useWorkflows();
  const runsByWorkflow = useRecentRunsByWorkflow();
  const [runTarget, setRunTarget] = useState<{ pkg: string; workflow: string } | null>(null);

  const items = (data?.items ?? []).filter((w) => w.tasks.length > 0);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <PageHeader
        title="Workflows"
        sub={`${items.length} packages`}
        right={
          <Button
            component={Link}
            to="/workflows/upload"
            color="ice"
            radius={9}
            size="sm"
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
          >
            ↑ Upload package
          </Button>
        }
      />

      {isPending ? (
        <Loading label="Loading workflows…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : items.length === 0 ? (
        <Empty message="No workflows uploaded yet." />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 9 }}>
          {items.map((w) => (
            <Box
              key={w.id}
              style={{ ...cardSurface, padding: "13px 16px", cursor: "pointer" }}
              onClick={() => navigate(`/workflows/${encodeURIComponent(w.package_name)}`)}
            >
              <Group justify="space-between" wrap="nowrap">
                <Group gap={11} wrap="nowrap" style={{ minWidth: 0 }}>
                  <span style={{ width: 10, height: 10, borderRadius: 2, background: TOKEN.ice, flex: "none" }} />
                  <Box style={{ minWidth: 0 }}>
                    <Group gap={8} wrap="nowrap">
                      <span style={{ fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{w.package_name}</span>
                      <span style={{ background: pillBg(TOKEN.violet), color: TOKEN.violet, borderRadius: 10, padding: "1px 7px", fontFamily: MONO, fontSize: 10.5 }}>
                        v{w.version}
                      </span>
                      <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
                        {w.tasks.length} task{w.tasks.length === 1 ? "" : "s"}
                      </span>
                    </Group>
                    {w.description && (
                      <Box style={{ fontSize: 12, color: "var(--muted)", marginTop: 3, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {w.description} · updated {formatAgo(w.created_at)}
                      </Box>
                    )}
                  </Box>
                </Group>
                <Group gap={16} wrap="nowrap" style={{ flex: "none" }}>
                  <RunCircles runs={runsByWorkflow.get(w.workflow_name) ?? []} />
                  <Button
                    size="xs"
                    variant="default"
                    onClick={(ev) => {
                      ev.stopPropagation();
                      setRunTarget({ pkg: w.package_name, workflow: w.workflow_name });
                    }}
                  >
                    ▸ Run
                  </Button>
                </Group>
              </Group>
            </Box>
          ))}
        </div>
      )}

      <RunWorkflowModal
        opened={runTarget !== null}
        packageName={runTarget?.pkg ?? ""}
        workflowName={runTarget?.workflow ?? ""}
        onClose={() => setRunTarget(null)}
      />
    </div>
  );
}
