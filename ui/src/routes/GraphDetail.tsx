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

import { Anchor, Badge, Card, Group, List, Stack, Text, Title } from "@mantine/core";
import { Link, useParams } from "react-router-dom";

import { useGraph } from "../api/health";
import { Dag, type DagEdge, type DagNode } from "../components/Dag";
import { GraphHealth } from "../components/GraphHealth";
import { Empty, ErrorState, Loading } from "../components/states/States";

type GraphData = {
  reactor?: string | null;
  accumulators: string[];
  topology?: { nodes: { id: string }[]; edges: { from: string; to: string; label?: string | null }[] } | null;
};

/**
 * Build the augmented CG view (CLOACI-I-0124 / WS-4): the compute nodes plus
 * the **accumulators** and the **reactor** as upstream nodes, so the full data
 * flow — sources → reactor → graph — reads as one picture. Accumulators feed
 * the reactor; the reactor fires the graph's entry (root) nodes.
 */
function buildCgGraph(data: GraphData): { nodes: DagNode[]; edges: DagEdge[] } | null {
  const topo = data.topology;
  if (!topo || topo.nodes.length === 0) return null;

  const nodes: DagNode[] = topo.nodes.map((n) => ({ id: n.id, kind: "compute" }));
  const edges: DagEdge[] = topo.edges.map((e) => ({ from: e.from, to: e.to, label: e.label }));

  const hasIncoming = new Set(topo.edges.map((e) => e.to));
  const roots = topo.nodes.filter((n) => !hasIncoming.has(n.id)).map((n) => n.id);

  const accIds = data.accumulators.map((a) => `acc:${a}`);
  data.accumulators.forEach((a) => nodes.push({ id: `acc:${a}`, label: a, kind: "accumulator" }));

  if (data.reactor) {
    const reactorId = `reactor:${data.reactor}`;
    nodes.push({ id: reactorId, label: data.reactor, kind: "reactor" });
    accIds.forEach((a) => edges.push({ from: a, to: reactorId }));
    roots.forEach((r) => edges.push({ from: reactorId, to: r }));
  } else {
    // No reactor: wire accumulators straight to the entry nodes.
    accIds.forEach((a) => roots.forEach((r) => edges.push({ from: a, to: r })));
  }

  return { nodes, edges };
}

/**
 * Single computation-graph detail (T-0655; CLOACI-I-0124 WS-4). Renders the
 * reactor + accumulators as first-class nodes in the graph rather than as
 * side text.
 */
export function GraphDetail() {
  const { name = "" } = useParams();
  const { data, isPending, isError, error, refetch } = useGraph(name);

  const graph = data ? buildCgGraph(data as GraphData) : null;

  return (
    <Stack>
      <div>
        <Anchor component={Link} to="/graphs" size="sm">
          ← Graphs
        </Anchor>
        <Title order={2}>{name}</Title>
      </div>

      {isPending ? (
        <Loading label="Loading graph…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : !data ? (
        <Empty message="Graph not found." />
      ) : (
        <Card withBorder padding="lg">
          <Stack gap="md">
            <Group>
              <GraphHealth value={data.health} />
              {data.paused && (
                <Badge color="orange" variant="light">
                  paused
                </Badge>
              )}
              {data.reaction_mode && (
                <Badge variant="light" color="grape">
                  {data.reaction_mode}
                </Badge>
              )}
              {data.input_strategy && (
                <Badge variant="light" color="cyan">
                  {data.input_strategy}
                </Badge>
              )}
            </Group>

            {graph ? (
              <div>
                <Group justify="space-between" mb="xs">
                  <Text fw={600}>Graph</Text>
                  <Group gap="sm">
                    <LegendDot color="var(--mantine-color-blue-4)" label="accumulator" />
                    <LegendDot color="var(--mantine-color-grape-4)" label="reactor" />
                    <LegendDot color="var(--mantine-color-default-border)" label="node" />
                  </Group>
                </Group>
                <Dag nodes={graph.nodes} edges={graph.edges} testId="graph-dag" />
              </div>
            ) : (
              // No topology available — fall back to the text summary.
              <>
                {data.reactor && (
                  <div>
                    <Text fw={600} mb="xs">
                      Reactor
                    </Text>
                    <Text size="sm">{data.reactor}</Text>
                  </div>
                )}
                <div>
                  <Text fw={600} mb="xs">
                    Accumulators ({data.accumulators.length})
                  </Text>
                  {data.accumulators.length === 0 ? (
                    <Text c="dimmed" size="sm">
                      None.
                    </Text>
                  ) : (
                    <List size="sm">
                      {data.accumulators.map((a) => (
                        <List.Item key={a}>{a}</List.Item>
                      ))}
                    </List>
                  )}
                </div>
              </>
            )}
          </Stack>
        </Card>
      )}
    </Stack>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <Group gap={4}>
      <span
        style={{
          width: 10,
          height: 10,
          borderRadius: 3,
          background: color,
          display: "inline-block",
        }}
      />
      <Text size="xs" c="dimmed">
        {label}
      </Text>
    </Group>
  );
}
