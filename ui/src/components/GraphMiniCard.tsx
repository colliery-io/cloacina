/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Overview computation-graph card (Aurora Dark spec 02). Health dot + name +
 *  health label + throughput, a mini topology DAG (accumulators → reactor →
 *  compute nodes, colored by kind), and Pause/Resume + ▸ Fire. Mirrors
 *  buildCgGraph in GraphDetail. Pause/Resume on a graph is a spec mock (#3);
 *  Fire force-fires the graph's reactor (T-0751).
 */
import { Dot, explainToken, healthColor, nodeKindColor } from "@colliery-io/aurora-dark";
import { Box, Button, Group } from "@mantine/core";
import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import { useGraph } from "../api/health";
import { useFireReactor } from "../api/controls";
import { useCan } from "../auth/AuthContext";
import { MiniDag, type MiniNode } from "./MiniDag";

const MONO = "'IBM Plex Mono', monospace";

interface GraphData {
  reactor?: string | null;
  accumulators: string[];
  topology?: { nodes: { id: string }[]; edges: { from: string; to: string }[] } | null;
}

/** Augmented CG node set as MiniNodes (accumulators col0 → reactor col1 →
 *  compute col2+), colored by kind. Mirrors buildCgGraph. */
function buildMiniNodes(data: GraphData | undefined): MiniNode[] {
  const topo = data?.topology;
  if (!topo || topo.nodes.length === 0) return [];
  const edges = topo.edges.map((e) => ({ from: e.from, to: e.to }));
  const colorById = new Map<string, string>();
  topo.nodes.forEach((n) => colorById.set(n.id, nodeKindColor("node")));

  const hasIncoming = new Set(topo.edges.map((e) => e.to));
  const roots = topo.nodes.filter((n) => !hasIncoming.has(n.id)).map((n) => n.id);

  const accIds = (data!.accumulators ?? []).map((a) => `acc:${a}`);
  accIds.forEach((id) => colorById.set(id, nodeKindColor("accumulator")));

  if (data!.reactor) {
    const rid = `reactor:${data!.reactor}`;
    colorById.set(rid, nodeKindColor("reactor"));
    accIds.forEach((a) => edges.push({ from: a, to: rid }));
    roots.forEach((r) => edges.push({ from: rid, to: r }));
  } else {
    accIds.forEach((a) => roots.forEach((r) => edges.push({ from: a, to: r })));
  }

  return [...colorById.entries()].map(([id, color]) => ({
    id,
    color,
    dependencies: edges.filter((e) => e.to === id).map((e) => e.from),
  }));
}

interface GraphListItem {
  name: string;
  health?: unknown;
  paused?: boolean;
}

export function GraphMiniCard({ graph, rate }: { graph: GraphListItem; rate?: number }) {
  const navigate = useNavigate();
  const detail = useGraph(graph.name);
  const fire = useFireReactor();
  const { canWrite } = useCan();
  const [paused, setPaused] = useState(!!graph.paused);

  const nodes = useMemo(() => buildMiniNodes(detail.data as GraphData | undefined), [detail.data]);
  const hs = typeof graph.health === "string" ? graph.health : (graph.health as { state?: string })?.state ?? "";
  const hc = healthColor(hs);
  const reactor = (detail.data as GraphData | undefined)?.reactor ?? null;

  return (
    <Box
      style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 10, padding: "13px 15px", cursor: "pointer" }}
      onClick={() => navigate(`/graphs/${encodeURIComponent(graph.name)}`)}
    >
      <Group justify="space-between" mb={9} wrap="nowrap">
        <Group gap={8} wrap="nowrap">
          <Dot color={hc} />
          <span style={{ fontSize: 13.5, fontWeight: 600, color: "var(--fg)" }}>{graph.name}</span>
          <span style={{ fontSize: 12, color: hc }}>{explainToken(hs || "unknown").label}</span>
        </Group>
        <Group gap={8} wrap="nowrap">
          <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>
            {rate == null ? "—" : `~${rate}/min`}
          </span>
          <Button
            size="compact-xs"
            variant="default"
            onClick={(ev) => {
              ev.stopPropagation();
              setPaused((p) => !p);
            }}
          >
            {paused ? "Resume" : "Pause"}
          </Button>
          {canWrite && reactor && (
            <Button
              size="compact-xs"
              variant="default"
              loading={fire.isPending && fire.variables === reactor}
              onClick={(ev) => {
                ev.stopPropagation();
                fire.mutate(reactor);
              }}
            >
              ▸ Fire
            </Button>
          )}
        </Group>
      </Group>

      {nodes.length > 0 && (
        <Box style={{ overflowX: "auto", padding: "2px 0" }}>
          <MiniDag nodes={nodes} />
        </Box>
      )}
    </Box>
  );
}
