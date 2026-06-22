/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Inputs card (CLOACI-T-0764 §4): the workflow's declared params (typed) shown
 *  read-only. Execution itself is the header action — no second Execute button.
 */
import { useWorkflow } from "../api/workflows";
import { MONO, Panel, Pill } from "./aurora";
import { TOKEN } from "../util/tokens";

interface Slot {
  name: string;
  schema?: { type?: string } | null;
  required: boolean;
  default?: unknown;
}

function typeColor(t: string | undefined): string {
  switch (t) {
    case "integer":
    case "number":
    case "float":
      return TOKEN.ice;
    case "string":
      return TOKEN.teal;
    case "boolean":
      return TOKEN.violet;
    default:
      return TOKEN.muted;
  }
}

export function InputsCard({ packageName }: { packageName: string }) {
  const { data } = useWorkflow(packageName);
  const params = (data?.declared_params ?? []) as Slot[];

  return (
    <Panel title="Inputs" caption={`${params.length} declared param${params.length === 1 ? "" : "s"}`} style={{ height: "100%" }}>
      {params.length === 0 ? (
        <div style={{ color: "var(--faint)", fontSize: 12.5 }}>
          No declared params — executes with free-form JSON context.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 11 }}>
          {params.map((p) => {
            const t = p.schema?.type;
            return (
              <div key={p.name} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <span style={{ fontFamily: MONO, fontSize: 12, color: "#dce2e9", flex: 1 }}>
                  {p.name}
                  {p.required ? <span style={{ color: TOKEN.bad }}> *</span> : null}
                </span>
                <Pill color={typeColor(t)}>{(t ?? "any").toUpperCase()}</Pill>
                <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", minWidth: 60, textAlign: "right" }}>
                  {p.default != null ? JSON.stringify(p.default) : "—"}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </Panel>
  );
}
