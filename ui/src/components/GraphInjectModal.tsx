/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Typed inject / fire-with dialog for the graph operational view (CLOACI-T-0768).
 *  Renders a field per declared input slot (from the reactor/accumulator
 *  `/interface` DeclaredSurface) — typed by JSON-Schema — and submits:
 *   - accumulator → POST .../inject  { event: <boundary object> }
 *   - reactor     → POST .../fire    { mode: fire_with, inputs: { source: obj } }
 *  Falls back to a raw-JSON textarea for untyped ({}) slots.
 */
import { MONO, Pill, TOKEN } from "@colliery-io/aurora-dark";
import { Box, Button, Group, Modal, NumberInput, Stack, Switch, Text, TextInput, Textarea } from "@mantine/core";
import { useMemo, useState } from "react";

import {
  useAccumulatorInterface,
  useFireReactor,
  useInjectAccumulator,
  useReactorInterface,
  type InterfaceSlot,
} from "../api/controls";
import { useCan } from "../auth/AuthContext";

export type InjectTarget = { kind: "accumulator" | "reactor"; name: string };

type FieldSpec = { slot: string; field: string; type: string; objectMode: boolean };

/** Flatten declared slots into renderable fields. Object slots expand to one
 *  field per property; scalar slots become a single field; untyped ({}) slots
 *  become a raw-JSON field (field === "" sentinel). */
function fieldsFor(slots: InterfaceSlot[]): FieldSpec[] {
  const out: FieldSpec[] = [];
  for (const s of slots) {
    const props = s.schema?.properties;
    if (props && Object.keys(props).length > 0) {
      for (const [field, def] of Object.entries(props)) {
        out.push({ slot: s.name, field, type: def?.type ?? "string", objectMode: true });
      }
    } else if (s.schema?.type && s.schema.type !== "object") {
      out.push({ slot: s.name, field: s.name, type: s.schema.type, objectMode: false });
    } else {
      out.push({ slot: s.name, field: "", type: "json", objectMode: false });
    }
  }
  return out;
}

function coerce(type: string, v: unknown): unknown {
  if (type === "integer" || type === "number") return v === "" || v == null ? null : Number(v);
  if (type === "boolean") return Boolean(v);
  return v ?? "";
}

export function GraphInjectModal({
  target,
  opened,
  onClose,
}: {
  target: InjectTarget | null;
  opened: boolean;
  onClose: () => void;
}) {
  const isAcc = target?.kind === "accumulator";
  const accIface = useAccumulatorInterface(isAcc ? target?.name : null, { enabled: opened && isAcc });
  const rxIface = useReactorInterface(!isAcc ? target?.name : null, { enabled: opened && !isAcc });
  const iface = isAcc ? accIface : rxIface;
  const inject = useInjectAccumulator();
  const fire = useFireReactor();
  const { canWrite } = useCan();

  const slots = useMemo(() => iface.data?.slots ?? [], [iface.data]);
  const fields = useMemo(() => fieldsFor(slots), [slots]);
  // values[slot][field] — field "" means the whole-slot raw-JSON value.
  const [values, setValues] = useState<Record<string, Record<string, unknown>>>({});
  const set = (slot: string, field: string, v: unknown) =>
    setValues((p) => ({ ...p, [slot]: { ...(p[slot] ?? {}), [field]: v } }));

  const pending = inject.isPending || fire.isPending;
  const err = (inject.error || fire.error) as Error | null;

  function assembleSlot(slot: string): unknown {
    const f = fields.filter((x) => x.slot === slot);
    if (f.length === 1 && f[0].field === "") {
      try {
        return JSON.parse(String(values[slot]?.[""] ?? "{}"));
      } catch {
        return {};
      }
    }
    if (f.length === 1 && !f[0].objectMode) return coerce(f[0].type, values[slot]?.[f[0].field]);
    const obj: Record<string, unknown> = {};
    for (const x of f) obj[x.field] = coerce(x.type, values[slot]?.[x.field]);
    return obj;
  }

  async function submit() {
    if (!target) return;
    if (isAcc) {
      await inject.mutateAsync({ name: target.name, event: assembleSlot(slots[0]?.name ?? target.name) });
    } else {
      const inputs: Record<string, unknown> = {};
      for (const s of slots) inputs[s.name] = assembleSlot(s.name);
      await fire.mutateAsync({ name: target.name, mode: "fire_with", inputs });
    }
    onClose();
  }

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      centered
      size="md"
      title={
        <Group gap={8}>
          <span style={{ fontFamily: MONO, fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{target?.name}</span>
          <Pill color={isAcc ? TOKEN.ice : TOKEN.violet}>{isAcc ? "inject" : "fire with"}</Pill>
        </Group>
      }
    >
      {iface.isPending ? (
        <Box style={{ color: "var(--faint)", fontSize: 13 }}>Loading interface…</Box>
      ) : fields.length === 0 ? (
        <Box style={{ color: "var(--muted)", fontSize: 13 }}>This {target?.kind} declares no typed inputs.</Box>
      ) : (
        <Stack gap="sm">
          {!isAcc && slots.length > 1 && (
            <Text size="xs" c="dimmed">
              Provide a value per source, then fire the reactor with these inputs.
            </Text>
          )}
          {fields.map((f) => {
            const label = f.objectMode ? `${f.slot}.${f.field}` : f.field || f.slot;
            const cur = values[f.slot]?.[f.field];
            if (f.type === "boolean")
              return (
                <Switch
                  key={`${f.slot}.${f.field}`}
                  label={label}
                  checked={Boolean(cur)}
                  onChange={(e) => set(f.slot, f.field, e.currentTarget.checked)}
                />
              );
            if (f.type === "integer" || f.type === "number")
              return (
                <NumberInput
                  key={`${f.slot}.${f.field}`}
                  label={label}
                  value={(cur as number) ?? ""}
                  onChange={(v) => set(f.slot, f.field, v)}
                />
              );
            if (f.type === "json")
              return (
                <Textarea
                  key={`${f.slot}.${f.field}`}
                  label={`${f.slot} (JSON)`}
                  autosize
                  minRows={3}
                  placeholder='{ "key": "value" }'
                  value={(cur as string) ?? ""}
                  onChange={(e) => set(f.slot, f.field, e.currentTarget.value)}
                  styles={{ input: { fontFamily: MONO, fontSize: 12 } }}
                />
              );
            return (
              <TextInput
                key={`${f.slot}.${f.field}`}
                label={label}
                value={(cur as string) ?? ""}
                onChange={(e) => set(f.slot, f.field, e.currentTarget.value)}
              />
            );
          })}
          {err && <Text size="xs" c="red">{err.message}</Text>}
          <Group justify="flex-end" mt={4}>
            <Button variant="default" radius={8} onClick={onClose}>
              Cancel
            </Button>
            {canWrite && (
              <Button color="ice" radius={8} styles={{ root: { color: "#0b0d10", fontWeight: 600 } }} loading={pending} onClick={submit}>
                {isAcc ? "Inject" : "Fire"}
              </Button>
            )}
          </Group>
        </Stack>
      )}
    </Modal>
  );
}
