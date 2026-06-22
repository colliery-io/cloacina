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

import { Box, Button, Group, Loader, Modal, SimpleGrid, Table, TextInput } from "@mantine/core";
import { type CSSProperties, type ReactNode, useState } from "react";

import { useOpsMetrics } from "../api/operations";
import { Dot, MONO } from "../components/aurora";
import { TOKEN, pillBg } from "../util/tokens";

function fmtTime(ts: string | null): string {
  if (!ts) return "never";
  const t = Date.parse(ts);
  return Number.isNaN(t) ? "—" : new Date(t).toLocaleTimeString();
}
function ago(seconds: number | null): string {
  if (seconds == null) return "—";
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

/** Aurora metric card: title + tinted state pill + Mono stat rows (spec 10). */
function MetricCard({ title, state, color, rows }: { title: string; state: string; color: string; rows: [string, ReactNode][] }) {
  return (
    <Box style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 11, padding: "15px 16px" }}>
      <Group justify="space-between" mb={12}>
        <span style={{ fontSize: 14.5, fontWeight: 600, color: "var(--fg)" }}>{title}</span>
        <span style={{ background: pillBg(color), color, borderRadius: 10, padding: "2px 9px", fontFamily: MONO, fontSize: 10.5 }}>
          {state}
        </span>
      </Group>
      <div style={{ display: "flex", flexDirection: "column", gap: 7 }}>
        {rows.map(([label, value]) => (
          <Group key={label} justify="space-between" gap="xs">
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>{label}</span>
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--fg-2)" }}>{value}</span>
          </Group>
        ))}
      </div>
    </Box>
  );
}

const compilerColor = (s: string) =>
  s === "building" ? TOKEN.ice : s === "backlogged" ? TOKEN.gold : TOKEN.muted;

/**
 * Operations / deployment health (Aurora Dark spec 10/11). Server, compiler,
 * reconciler, and fleet as metric cards driven by the WS-pushed ops snapshot,
 * plus the execution-agent roster and the (mock) add-agent enrollment flow.
 */
export function Operations() {
  const m = useOpsMetrics();
  const [addOpen, setAddOpen] = useState(false);

  const busy = m ? m.fleet.reduce((a, f) => a + f.in_flight, 0) : 0;
  const capacity = m ? m.fleet.reduce((a, f) => a + f.max_concurrency, 0) : 0;

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
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      {/* Header */}
      <Box>
        <Group gap={10}>
          <span style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)" }}>Operations</span>
          <span style={{ background: pillBg(m ? TOKEN.ok : TOKEN.muted), color: m ? TOKEN.ok : TOKEN.muted, borderRadius: 10, padding: "2px 10px", fontFamily: MONO, fontSize: 10.5, display: "inline-flex", alignItems: "center", gap: 5 }}>
            <Dot color={m ? TOKEN.ok : TOKEN.muted} size={6} /> {m ? "live" : "connecting…"}
          </span>
        </Group>
        <Box style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 3 }}>
          Deployment health for the connected server, pushed over the control-plane socket.
        </Box>
      </Box>

      {!m ? (
        <Group gap="xs" mt="lg">
          <Loader size="sm" color="ice" />
          <span style={{ color: "var(--faint)", fontSize: 13 }}>Subscribing to operational metrics…</span>
        </Group>
      ) : (
        <>
          <SimpleGrid cols={{ base: 1, sm: 2, lg: 4 }} spacing={13}>
            <MetricCard
              title="Server"
              state={m.server.alive ? "alive" : "down"}
              color={m.server.alive ? TOKEN.ok : TOKEN.bad}
              rows={[
                ["readiness", <span style={{ color: m.server.ready ? TOKEN.ok : TOKEN.bad }}>{m.server.ready ? "ready" : (m.server.reason ?? "not ready")}</span>],
                ["liveness", <span style={{ color: m.server.alive ? TOKEN.ok : TOKEN.bad }}>{m.server.alive ? "alive" : "down"}</span>],
              ]}
            />
            <MetricCard
              title="Compiler"
              state={m.compiler.status}
              color={compilerColor(m.compiler.status)}
              rows={[
                ["pending", m.compiler.pending],
                ["building", m.compiler.building],
                ["last success", fmtTime(m.compiler.last_success_at)],
              ]}
            />
            <MetricCard
              title="Reconciler"
              state={m.reconciler.failed > 0 ? "degraded" : "healthy"}
              color={m.reconciler.failed > 0 ? TOKEN.bad : TOKEN.ok}
              rows={[
                ["available", m.reconciler.built],
                ["failed builds", <span style={{ color: m.reconciler.failed > 0 ? TOKEN.bad : "var(--fg-2)" }}>{m.reconciler.failed}</span>],
                ["last built", fmtTime(m.reconciler.last_built_at)],
              ]}
            />
            <MetricCard
              title="Fleet"
              state={`${m.fleet.length} agent${m.fleet.length === 1 ? "" : "s"}`}
              color={m.fleet.length > 0 ? TOKEN.ok : TOKEN.muted}
              rows={[
                ["in flight", <span style={{ color: TOKEN.ice }}>{busy}</span>],
                ["capacity", capacity],
                ["idle", Math.max(0, capacity - busy)],
              ]}
            />
          </SimpleGrid>

          {/* Agents */}
          <Group justify="space-between" mt={6} mb={2} style={{ borderBottom: "1px solid var(--border-soft)", paddingBottom: 8 }}>
            <span style={{ fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>Execution agents</span>
            <Button size="xs" variant="default" onClick={() => setAddOpen(true)}>
              + Add agent
            </Button>
          </Group>

          {m.fleet.length === 0 ? (
            <Box style={{ border: "1px dashed var(--border)", borderRadius: 10, padding: "18px 15px", color: "var(--faint)", fontSize: 12.5 }}>
              No agents registered — work runs on the in-process executor.
            </Box>
          ) : (
            <Table verticalSpacing={10}>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th style={th}>Agent</Table.Th>
                  <Table.Th style={th}>Target</Table.Th>
                  <Table.Th style={th}>Capacity</Table.Th>
                  <Table.Th style={th}>Heartbeat</Table.Th>
                  <Table.Th style={th}>Tenant</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {m.fleet.map((a) => {
                  const stale = a.seconds_since_heartbeat != null && a.seconds_since_heartbeat > 60;
                  return (
                    <Table.Tr key={a.agent_id}>
                      <Table.Td>
                        <Group gap={8} wrap="nowrap">
                          <Dot color={stale ? TOKEN.gold : TOKEN.ok} size={7} />
                          <span style={{ fontSize: 13, fontWeight: 500, color: "var(--fg)" }}>{a.agent_id}</span>
                        </Group>
                      </Table.Td>
                      <Table.Td>
                        <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>{a.target_triple}</span>
                      </Table.Td>
                      <Table.Td>
                        <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--fg-2)" }}>{a.in_flight}/{a.max_concurrency} in flight</span>
                      </Table.Td>
                      <Table.Td>
                        <span style={{ fontFamily: MONO, fontSize: 11.5, color: stale ? TOKEN.gold : "var(--faint)" }}>{ago(a.seconds_since_heartbeat)}</span>
                      </Table.Td>
                      <Table.Td>
                        <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--faint)" }}>{a.tenant_id ?? "—"}</span>
                      </Table.Td>
                    </Table.Tr>
                  );
                })}
              </Table.Tbody>
            </Table>
          )}
        </>
      )}

      <AddAgentModal opened={addOpen} onClose={() => setAddOpen(false)} />
    </div>
  );
}

/** Add-agent enrollment (spec 11, MOCK). Models issuing a one-time enrollment
 *  token; the agent self-registers and then appears in the fleet table. */
function AddAgentModal({ opened, onClose }: { opened: boolean; onClose: () => void }) {
  const [name, setName] = useState("");
  const [conc, setConc] = useState("8");
  const [triple, setTriple] = useState("x86_64-linux");
  const [issued, setIssued] = useState<string | null>(null);

  const monoInput = { input: { fontFamily: MONO } };
  const close = () => {
    setIssued(null);
    onClose();
  };

  return (
    <Modal opened={opened} onClose={close} size={520} centered title={<span style={{ fontWeight: 600 }}>Register an agent</span>}>
      <Box style={{ fontSize: 12.5, color: "var(--muted)", marginBottom: 14 }}>
        Issue a one-time enrollment token. The agent self-registers and appears in the fleet.
      </Box>
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <TextInput label="Name" placeholder="agent-eu-west-1" value={name} onChange={(e) => setName(e.currentTarget.value)} styles={monoInput} />
        <TextInput label="Max concurrency" value={conc} onChange={(e) => setConc(e.currentTarget.value)} styles={monoInput} />
        <TextInput label="Target triple" value={triple} onChange={(e) => setTriple(e.currentTarget.value)} styles={monoInput} />

        {issued && (
          <Box style={{ background: "var(--inset)", border: "1px solid var(--border-soft)", borderRadius: 10, padding: "12px 14px" }}>
            <Box style={{ fontFamily: MONO, fontSize: 10.5, letterSpacing: ".06em", textTransform: "uppercase", color: "var(--faint)", marginBottom: 8 }}>
              Enrollment
            </Box>
            <Box style={{ fontFamily: MONO, fontSize: 11.5, color: TOKEN.ice, lineHeight: 1.7, whiteSpace: "pre-wrap" }}>
              {`cloacinactl agent join \\\n  --server <server-url> \\\n  --token ${issued} \\\n  --tenant public`}
            </Box>
            <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)", marginTop: 8 }}>
              Token {issued.slice(0, 8)} · expires in 15m · single use
            </Box>
          </Box>
        )}

        <Group justify="flex-end" mt={2}>
          <Button variant="default" onClick={close}>
            Cancel
          </Button>
          <Button
            color="ice"
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            onClick={() => setIssued(`enr_${Math.abs(hash(name + conc + triple)).toString(36)}${conc}`)}
          >
            Issue token
          </Button>
        </Group>
      </div>
    </Modal>
  );
}

// Deterministic pseudo-id for the mock token (no Math.random in this codebase).
function hash(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
  return h;
}
