/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Typed Run dialog (CLOACI-I-0129 → wires CLOACI-I-0128). When a workflow
 *  declares params (`WorkflowDetail.declared_params`), this renders a field per
 *  param — typed by the JSON-Schema `type` — and executes with the assembled
 *  context. Params-less workflows just get a confirm-and-run. The server still
 *  validates (`400 workflow_input_invalid`); this is the friendly front door.
 */
import { MONO } from "@colliery-io/aurora-dark";
import { Box, Button, Group, Modal, NumberInput, Stack, Switch, TextInput } from "@mantine/core";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { useExecuteWorkflow, useWorkflow } from "../api/workflows";

interface Slot {
  name: string;
  schema?: { type?: string } | null;
  required: boolean;
  default?: unknown;
}

export function RunWorkflowModal({
  packageName,
  workflowName,
  opened,
  onClose,
}: {
  packageName: string;
  workflowName: string;
  opened: boolean;
  onClose: () => void;
}) {
  const navigate = useNavigate();
  const detail = useWorkflow(packageName, { enabled: opened });
  const execute = useExecuteWorkflow();
  const params = (detail.data?.declared_params ?? []) as Slot[];
  const [values, setValues] = useState<Record<string, string | number | boolean>>({});

  const set = (name: string, v: string | number | boolean) => setValues((p) => ({ ...p, [name]: v }));

  function run() {
    const context: Record<string, unknown> = {};
    for (const p of params) {
      const v = values[p.name];
      if (v === undefined || v === "") continue;
      context[p.name] = v;
    }
    execute.mutate(
      { name: workflowName, context },
      {
        onSuccess: (res) => {
          onClose();
          setValues({});
          navigate(`/executions/${res.execution_id}`);
        },
      },
    );
  }

  return (
    <Modal opened={opened} onClose={onClose} title={`Run ${workflowName}`} centered>
      <Stack gap={14}>
        {detail.isPending ? (
          <Box style={{ color: "var(--faint)", fontSize: 13 }}>Loading declared inputs…</Box>
        ) : params.length === 0 ? (
          <Box style={{ color: "var(--muted)", fontSize: 13 }}>
            This workflow declares no inputs — run it with an empty context.
          </Box>
        ) : (
          params.map((p) => {
            const t = p.schema?.type;
            const label = (
              <span style={{ fontFamily: MONO, fontSize: 12 }}>
                {p.name}
                {p.required ? <span style={{ color: "var(--bad)" }}> *</span> : null}
                <span style={{ color: "var(--faint)" }}> · {t ?? "any"}</span>
              </span>
            );
            if (t === "boolean") {
              return (
                <Switch
                  key={p.name}
                  label={label}
                  checked={Boolean(values[p.name] ?? p.default ?? false)}
                  onChange={(e) => set(p.name, e.currentTarget.checked)}
                />
              );
            }
            if (t === "integer" || t === "number") {
              return (
                <NumberInput
                  key={p.name}
                  label={label}
                  placeholder={p.default != null ? String(p.default) : ""}
                  allowDecimal={t === "number"}
                  value={(values[p.name] as number) ?? ""}
                  onChange={(v) => set(p.name, v as number)}
                />
              );
            }
            return (
              <TextInput
                key={p.name}
                label={label}
                placeholder={p.default != null ? String(p.default) : ""}
                value={(values[p.name] as string) ?? ""}
                onChange={(e) => set(p.name, e.currentTarget.value)}
              />
            );
          })
        )}

        {execute.isError && (
          <Box style={{ color: "var(--bad)", fontSize: 12.5 }}>
            {execute.error instanceof Error ? execute.error.message : "Run failed"}
          </Box>
        )}

        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button
            color="ice"
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            loading={execute.isPending}
            onClick={run}
          >
            ▸ Run
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
