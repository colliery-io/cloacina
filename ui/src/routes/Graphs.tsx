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
import { useMemo } from "react";
import { useNavigate } from "react-router-dom";

import { useAccumulators, useGraphs, useReactors } from "../api/health";
import { useFireReactor } from "../api/controls";
import { Dot, MONO, PageHeader, cardSurface } from "../components/aurora";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { explainToken } from "../util/vocab";
import { formatAgo, useGraphThroughput } from "../util/activity";
import { healthColor, nodeKindColor, pillBg, TOKEN } from "../util/tokens";

function healthState(value: unknown): string {
  if (typeof value === "string") return value;
  if (value && typeof value === "object" && "state" in value) {
    return String((value as { state?: string }).state ?? "");
  }
  return "";
}

function SectionLabel({ children }: { children: string }) {
  return (
    <Box
      style={{
        fontFamily: MONO,
        fontSize: 11,
        letterSpacing: ".06em",
        textTransform: "uppercase",
        color: "var(--muted)",
        margin: "6px 0 8px",
      }}
    >
      {children}
    </Box>
  );
}

function tintPill(label: string, color: string) {
  return (
    <span style={{ background: pillBg(color), color, borderRadius: 10, padding: "1px 8px", fontFamily: MONO, fontSize: 10.5 }}>
      {label}
    </span>
  );
}

/** Computation graphs (Aurora Dark, spec 08): graphs / reactors / accumulators
 *  as card rows. Graph rows → per-graph topology detail. */
export function Graphs() {
  const navigate = useNavigate();
  const graphs = useGraphs();
  const reactors = useReactors();
  const accs = useAccumulators();
  const fire = useFireReactor();

  const accStatus = useMemo(() => {
    const m = new Map<string, string>();
    for (const a of accs.data?.items ?? []) m.set(a.name, String(a.status ?? ""));
    return m;
  }, [accs.data]);

  const throughput = useGraphThroughput(graphs.data?.items ?? []);
  const reactorThroughput = useGraphThroughput(reactors.data?.items ?? []);

  const graphItems = graphs.data?.items ?? [];
  const reactorItems = reactors.data?.items ?? [];
  const accItems = accs.data?.items ?? [];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <PageHeader
        title="Computation graphs"
        sub={`${graphItems.length} graphs · ${reactorItems.length} reactors · ${accItems.length} accumulators`}
      />

      {/* Graphs */}
      <Box>
        <SectionLabel>Graphs</SectionLabel>
        {graphs.isPending ? (
          <Loading label="Loading graphs…" />
        ) : graphs.isError ? (
          <ErrorState error={graphs.error} onRetry={graphs.refetch} />
        ) : graphItems.length === 0 ? (
          <Empty message="No graphs loaded." />
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {graphItems.map((g) => {
              const rate = throughput.get(g.name);
              const hs = healthState(g.health);
              return (
                <Box
                  key={g.name}
                  style={{ ...cardSurface, padding: "12px 15px", cursor: "pointer" }}
                  onClick={() => navigate(`/graphs/${encodeURIComponent(g.name)}`)}
                >
                  <Group justify="space-between" wrap="nowrap">
                    <Group gap={10} wrap="nowrap" style={{ minWidth: 0 }}>
                      <Dot color={healthColor(hs)} />
                      <span style={{ fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{g.name}</span>
                      <span style={{ fontSize: 12, color: healthColor(hs) }}>{explainToken(hs || "unknown").label}</span>
                      {g.paused && tintPill("paused", TOKEN.gold)}
                    </Group>
                    <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>
                      {rate == null ? "—" : `~${rate}/min`}
                    </span>
                  </Group>
                  {g.accumulators.length > 0 && (
                    <Box style={{ marginTop: 7, display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                      {g.accumulators.map((name) => (
                        <span key={name} style={{ display: "inline-flex", alignItems: "center", gap: 5, fontFamily: MONO, fontSize: 11, color: "var(--fg-2)" }}>
                          <Dot color={nodeKindColor("accumulator")} size={6} />
                          {name}
                        </span>
                      ))}
                      <span style={{ color: "var(--faint)" }}>→</span>
                      {tintPill(g.name, TOKEN.violet)}
                      {g.reaction_mode && (
                        <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
                          {explainToken(g.reaction_mode).label}
                        </span>
                      )}
                    </Box>
                  )}
                </Box>
              );
            })}
          </div>
        )}
      </Box>

      {/* Reactors */}
      <Box>
        <SectionLabel>Reactors</SectionLabel>
        {reactors.isPending ? (
          <Loading label="Loading reactors…" />
        ) : reactors.isError ? (
          <ErrorState error={reactors.error} onRetry={reactors.refetch} />
        ) : reactorItems.length === 0 ? (
          <Empty message="No reactors loaded." />
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {reactorItems.map((r) => {
              const rate = reactorThroughput.get(r.name);
              const boundGraphs = (r as { bound_graphs?: string[] }).bound_graphs ?? [];
              const lastFired = (r as { last_fired_at?: string | null }).last_fired_at ?? null;
              const first = boundGraphs[0];
              return (
                <Box
                  key={r.name}
                  style={{ ...cardSurface, padding: "12px 15px", cursor: first ? "pointer" : "default" }}
                  onClick={first ? () => navigate(`/graphs/${encodeURIComponent(first)}`) : undefined}
                >
                  <Group justify="space-between" wrap="nowrap">
                    <Group gap={10} wrap="nowrap" style={{ minWidth: 0 }}>
                      <Dot color={nodeKindColor("reactor")} />
                      <span style={{ fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{r.name}</span>
                      {r.reaction_mode && tintPill(explainToken(r.reaction_mode).label, TOKEN.violet)}
                      {r.paused && tintPill("paused", TOKEN.gold)}
                    </Group>
                    <Group gap={12} wrap="nowrap">
                      <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>
                        {rate == null ? "—" : `~${rate}/min`}
                      </span>
                      <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--fainter)" }}>{formatAgo(lastFired)}</span>
                      <Button
                        size="compact-xs"
                        variant="default"
                        loading={fire.isPending && fire.variables === r.name}
                        onClick={(ev) => {
                          ev.stopPropagation();
                          fire.mutate(r.name);
                        }}
                      >
                        ▸ Fire
                      </Button>
                    </Group>
                  </Group>
                  <Box style={{ marginTop: 7, display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                    {r.accumulators.map((name) => (
                      <span key={name} style={{ display: "inline-flex", alignItems: "center", gap: 5, fontFamily: MONO, fontSize: 11, color: "var(--fg-2)" }}>
                        <Dot color={healthColor(accStatus.get(name) ?? "")} size={6} />
                        {name}
                      </span>
                    ))}
                    <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
                      {boundGraphs.length ? `→ ${boundGraphs.join(", ")}` : "→ unbound"}
                    </span>
                  </Box>
                </Box>
              );
            })}
          </div>
        )}
      </Box>

      {/* Accumulators */}
      <Box>
        <SectionLabel>Accumulators</SectionLabel>
        {accs.isPending ? (
          <Loading label="Loading accumulators…" />
        ) : accs.isError ? (
          <ErrorState error={accs.error} onRetry={accs.refetch} />
        ) : accItems.length === 0 ? (
          <Empty message="No accumulators registered." />
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {accItems.map((a) => {
              const reactor = (a as { reactor?: string | null }).reactor ?? null;
              return (
                <Box key={a.name} style={{ ...cardSurface, padding: "9px 15px" }}>
                  <Group justify="space-between" wrap="nowrap">
                    <Group gap={9} wrap="nowrap">
                      <Dot color={nodeKindColor("accumulator")} size={7} />
                      <span style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)" }}>{a.name}</span>
                      <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
                        {explainToken(String(a.status ?? "unknown")).label}
                      </span>
                    </Group>
                    {reactor && (
                      <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>→ {reactor}</span>
                    )}
                  </Group>
                </Box>
              );
            })}
          </div>
        )}
      </Box>
    </div>
  );
}
