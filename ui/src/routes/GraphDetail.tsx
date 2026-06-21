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

import { Anchor, Box, Button, Divider, Drawer, Group, Stack, Text, Tooltip } from "@mantine/core";
import { useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";

import { useAccumulators, useGraph } from "../api/health";
import { useFireReactor } from "../api/controls";
import { type DagEdge, type DagNode } from "../components/Dag";
import { FullDag } from "../components/FullDag";
import { GraphHealth } from "../components/GraphHealth";
import {
  AccumulatorTable,
  DegradedBanner,
  FireActivity,
  GraphStatusStrip,
  ReactorReadiness,
  RecentFires,
  accStale,
  type Acc,
} from "../components/graph-ops";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { explainToken } from "../util/vocab";
import { MONO, Panel, Pill } from "../components/aurora";
import { nodeKindColor, TOKEN } from "../util/tokens";

type TopoNode = { id: string; inputs?: string[] };
type TopoEdge = { from: string; to: string; label?: string | null };
type GraphData = {
  reactor?: string | null;
  accumulators: string[];
  reaction_mode?: string | null;
  input_strategy?: string | null;
  topology?: { nodes: TopoNode[]; edges: TopoEdge[] } | null;
};

/**
 * Build the augmented CG view (CLOACI-I-0124 / WS-4): compute nodes plus the
 * accumulators + reactor as upstream nodes — sources → reactor → graph.
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
    accIds.forEach((a) => roots.forEach((r) => edges.push({ from: a, to: r })));
  }

  return { nodes, edges };
}

/** Node drawer detail (CLOACI-I-0124 / WS-5). */
function describeNode(
  id: string,
  data: GraphData,
): { title: string; kind: string; rows: [string, string][] } {
  if (id.startsWith("acc:")) {
    return {
      title: id.slice(4),
      kind: "Accumulator",
      rows: [["Role", "Turns an external source into the boundary events the reactor consumes."]],
    };
  }
  if (id.startsWith("reactor:")) {
    return {
      title: id.slice(8),
      kind: "Reactor",
      rows: [
        ["Criteria", explainToken(data.reaction_mode).label],
        ["Input strategy", explainToken(data.input_strategy).label],
        ["Accumulators", data.accumulators.join(", ") || "—"],
        ["Role", "Fires the graph when its criteria over the bound accumulators are met."],
      ],
    };
  }
  const topo = data.topology;
  const node = topo?.nodes.find((n) => n.id === id);
  const outgoing = (topo?.edges ?? []).filter((e) => e.from === id);
  const incoming = (topo?.edges ?? []).filter((e) => e.to === id);
  return {
    title: id,
    kind: "Node",
    rows: [
      ["Inputs", node?.inputs?.join(", ") || "—"],
      ["Upstream", incoming.map((e) => e.from).join(", ") || "— (entry node)"],
      [
        "Routes to",
        outgoing.map((e) => (e.label ? `${e.to} (on ${e.label})` : e.to)).join(", ") || "— (terminal)",
      ],
    ],
  };
}

/**
 * Graph operational view (CLOACI-T-0767). Header + degraded banner + status
 * strip + fire activity + reactor readiness + accumulator freshness + topology
 * (degraded overlay) + recent fires, on the T-0765/T-0766 instrumentation.
 */
export function GraphDetail() {
  const { name = "" } = useParams();
  const { data, isPending, isError, error, refetch } = useGraph(name);
  const accumulators = useAccumulators();
  const fire = useFireReactor();
  const [selected, setSelected] = useState<string | null>(null);
  const [paused, setPaused] = useState(false);

  const gd = data as GraphData | undefined;
  const reactor = gd?.reactor ?? null;

  // The graph's bound accumulators with freshness (filtered from the list).
  const accs: Acc[] = useMemo(() => {
    const names = new Set(gd?.accumulators ?? []);
    return (accumulators.data?.items ?? []).filter((a) => names.has(a.name)) as Acc[];
  }, [accumulators.data, gd?.accumulators]);

  const graph = gd ? buildCgGraph(gd) : null;
  const detail = selected && gd ? describeNode(selected, gd) : null;

  // Degraded sources → gold ⚠ overlay on their topology nodes.
  const failByNode = useMemo(() => {
    const m: Record<string, number> = {};
    for (const a of accs) if (accStale(a)) m[`acc:${a.name}`] = 1;
    return m;
  }, [accs]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
      {/* Header */}
      <Group justify="space-between" align="flex-start">
        <Box>
          <Anchor component={Link} to="/graphs" size="xs" c="dimmed" style={{ fontFamily: MONO }}>
            ← Graphs
          </Anchor>
          <Box style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)", marginTop: 2, fontFamily: MONO }}>{name}</Box>
          {data && (
            <Group gap={8} mt={5}>
              <GraphHealth value={data.health} />
              {data.reaction_mode && (
                <Tooltip label={explainToken(data.reaction_mode).tip} disabled={!explainToken(data.reaction_mode).tip} multiline w={260} withArrow>
                  <span style={{ display: "inline-flex" }}>
                    <Pill color={TOKEN.violet}>{explainToken(data.reaction_mode).label}</Pill>
                  </span>
                </Tooltip>
              )}
              {data.input_strategy && (
                <Tooltip label={explainToken(data.input_strategy).tip} disabled={!explainToken(data.input_strategy).tip} multiline w={260} withArrow>
                  <span style={{ display: "inline-flex" }}>
                    <Pill color={TOKEN.teal}>{explainToken(data.input_strategy).label}</Pill>
                  </span>
                </Tooltip>
              )}
              {reactor && <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>reactor {reactor}</span>}
            </Group>
          )}
        </Box>
        {data && (
          <Group gap={8}>
            <Button
              color="ice"
              radius={8}
              styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
              loading={fire.isPending}
              disabled={!reactor}
              onClick={() => reactor && fire.mutate(reactor)}
            >
              ▸ Fire
            </Button>
            <Button variant="default" radius={8} onClick={() => setPaused((p) => !p)}>
              {paused ? "▸ Resume" : "⏸ Pause"}
            </Button>
          </Group>
        )}
      </Group>

      {isPending ? (
        <Loading label="Loading graph…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : !data ? (
        <Empty message="Graph not found." />
      ) : (
        <>
          <DegradedBanner accumulators={accs} />

          <GraphStatusStrip
            graphName={name}
            health={data.health}
            fires={(data as { fires?: number }).fires ?? 0}
            lastFiredAt={(data as { last_fired_at?: string | null }).last_fired_at ?? null}
            reactor={reactor}
            accumulators={accs}
          />

          <Panel title="Fire activity" caption="fires per minute · last 60 min">
            <FireActivity reactor={reactor} />
          </Panel>

          <ReactorReadiness
            reactor={reactor}
            reactionMode={data.reaction_mode}
            inputStrategy={data.input_strategy}
            accumulators={accs}
            lastFiredAt={(data as { last_fired_at?: string | null }).last_fired_at ?? null}
          />

          <Panel title="Accumulators" caption={`${accs.length} bound source${accs.length === 1 ? "" : "s"}`} right={<Pill color={TOKEN.gold}>proposed</Pill>}>
            <AccumulatorTable accumulators={accs} />
          </Panel>

          {graph ? (
            <Panel
              title="Topology"
              right={
                <Group gap="sm">
                  <LegendDot color={nodeKindColor("accumulator")} label="accumulator" />
                  <LegendDot color={nodeKindColor("reactor")} label="reactor" />
                  <LegendDot color={nodeKindColor("node")} label="node" />
                </Group>
              }
            >
              <FullDag nodes={graph.nodes} edges={graph.edges} testId="graph-dag" onNodeClick={setSelected} failByNode={failByNode} />
              <Text size="xs" c="dimmed" mt={6}>
                Click a node to inspect its role and routing.
              </Text>
            </Panel>
          ) : (
            <Panel title="Topology">
              <Text c="dimmed" size="sm">
                No topology available for this graph.
              </Text>
            </Panel>
          )}

          <Panel title="Recent fires" caption="each fire runs the graph">
            <RecentFires reactor={reactor} />
          </Panel>
        </>
      )}

      <Drawer
        opened={!!selected}
        onClose={() => setSelected(null)}
        position="right"
        size="md"
        title={detail ? `${detail.kind}: ${detail.title}` : ""}
      >
        {detail && (
          <Stack gap="sm">
            <span style={{ display: "inline-flex", width: "fit-content" }}>
              <Pill color={nodeKindColor(detail.kind.toLowerCase())}>{detail.kind}</Pill>
            </span>
            {detail.rows.map(([k, v]) => (
              <div key={k}>
                <Text size="xs" c="dimmed">
                  {k}
                </Text>
                <Text size="sm">{v}</Text>
              </div>
            ))}
            <Divider />
            <Text size="xs" c="dimmed">
              Node source isn't shipped in compiled <code>.cloacina</code> packages, so the function body isn't shown here.
            </Text>
          </Stack>
        )}
      </Drawer>
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <Group gap={4}>
      <span style={{ width: 10, height: 10, borderRadius: 3, background: color, display: "inline-block" }} />
      <Text size="xs" c="dimmed">
        {label}
      </Text>
    </Group>
  );
}
