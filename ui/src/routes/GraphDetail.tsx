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

/**
 * Single computation-graph detail (T-0655). Exercises `getGraph` — its
 * 404 (unknown graph, or one not visible to the key) renders the typed
 * not-found state.
 */
export function GraphDetail() {
  const { name = "" } = useParams();
  const { data, isPending, isError, error, refetch } = useGraph(name);

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
            {data.topology && data.topology.nodes.length > 0 && (
              <div>
                <Text fw={600} mb="xs">
                  Graph ({data.topology.nodes.length} nodes)
                </Text>
                <Dag
                  nodes={data.topology.nodes.map((n): DagNode => ({ id: n.id }))}
                  edges={data.topology.edges.map(
                    (e): DagEdge => ({ from: e.from, to: e.to, label: e.label }),
                  )}
                  testId="graph-dag"
                />
              </div>
            )}
          </Stack>
        </Card>
      )}
    </Stack>
  );
}
