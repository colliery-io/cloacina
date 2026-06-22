/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Typed manual-fire dialog for a trigger (CLOACI-T-0777). Pushes an event to a
 *  trigger, which fans out to every subscribed workflow. The form fields come
 *  from the trigger's declared pass-through interface — the union of its
 *  subscribers' declared params (GET .../triggers/{name}/interface). On success
 *  it shows the fan-out result (which workflows were fired). An untyped trigger
 *  (no declared params) fires with no event.
 */
import { Box, Button, Group, Modal, NumberInput, Stack, Switch, Text, TextInput } from "@mantine/core";
import { useMemo, useState } from "react";

import { useFireTrigger, useTriggerInterface, type InterfaceSlot } from "../api/controls";
import { MONO, Pill } from "./aurora";
import { TOKEN } from "../util/tokens";

type Field = { name: string; type: string };

/** Trigger pass-through slots are flat scalar params (one InputSlot per declared
 *  param across the subscribers). Map each to a typed field. */
function fieldsFor(slots: InterfaceSlot[]): Field[] {
  return slots.map((s) => ({ name: s.name, type: s.schema?.type ?? "string" }));
}

function coerce(type: string, v: unknown): unknown {
  if (type === "integer" || type === "number") return v === "" || v == null ? null : Number(v);
  if (type === "boolean") return Boolean(v);
  return v ?? "";
}

export function TriggerFireModal({
  triggerName,
  opened,
  onClose,
}: {
  triggerName: string | null;
  opened: boolean;
  onClose: () => void;
}) {
  const iface = useTriggerInterface(triggerName, { enabled: opened && !!triggerName });
  const fire = useFireTrigger();
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [result, setResult] = useState<{ fired: number; executions: { workflow_name: string }[] } | null>(null);

  const slots = useMemo(() => iface.data?.slots ?? [], [iface.data]);
  const fields = useMemo(() => fieldsFor(slots), [slots]);
  const set = (name: string, v: unknown) => setValues((p) => ({ ...p, [name]: v }));

  function close() {
    setValues({});
    setResult(null);
    onClose();
  }

  async function submit() {
    if (!triggerName) return;
    const event: Record<string, unknown> = {};
    for (const f of fields) {
      const v = values[f.name];
      if (v !== undefined && v !== "") event[f.name] = coerce(f.type, v);
    }
    const res = await fire.mutateAsync({
      name: triggerName,
      event: Object.keys(event).length > 0 ? event : null,
    });
    setResult(res);
  }

  return (
    <Modal
      opened={opened}
      onClose={close}
      centered
      size="md"
      title={
        <Group gap={8}>
          <span style={{ fontFamily: MONO, fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{triggerName}</span>
          <Pill color={TOKEN.gold}>fire</Pill>
        </Group>
      }
    >
      {result ? (
        <Stack gap="sm">
          <Text size="sm" c="var(--fg)">
            Fired <b>{result.fired}</b> workflow{result.fired === 1 ? "" : "s"}:
          </Text>
          <Stack gap={4}>
            {result.executions.map((e) => (
              <Box key={e.workflow_name} style={{ fontFamily: MONO, fontSize: 12, color: "var(--fg-2)" }}>
                ↳ {e.workflow_name}
              </Box>
            ))}
          </Stack>
          <Group justify="flex-end" mt={4}>
            <Button color="ice" radius={8} styles={{ root: { color: "#0b0d10", fontWeight: 600 } }} onClick={close}>
              Done
            </Button>
          </Group>
        </Stack>
      ) : iface.isPending ? (
        <Box style={{ color: "var(--faint)", fontSize: 13 }}>Loading interface…</Box>
      ) : (
        <Stack gap="sm">
          <Text size="xs" c="dimmed">
            {fields.length === 0
              ? "This trigger declares no typed inputs — fire it with no event."
              : "Push a typed event to this trigger; it fans out to every subscribed workflow."}
          </Text>
          {fields.map((f) => {
            const cur = values[f.name];
            if (f.type === "boolean")
              return (
                <Switch
                  key={f.name}
                  label={f.name}
                  checked={Boolean(cur)}
                  onChange={(e) => set(f.name, e.currentTarget.checked)}
                />
              );
            if (f.type === "integer" || f.type === "number")
              return (
                <NumberInput
                  key={f.name}
                  label={f.name}
                  value={(cur as number) ?? ""}
                  onChange={(v) => set(f.name, v)}
                />
              );
            return (
              <TextInput
                key={f.name}
                label={f.name}
                value={(cur as string) ?? ""}
                onChange={(e) => set(f.name, e.currentTarget.value)}
              />
            );
          })}
          {fire.error && (
            <Text size="xs" c="red">
              {(fire.error as Error).message}
            </Text>
          )}
          <Group justify="flex-end" mt={4}>
            <Button variant="default" radius={8} onClick={close}>
              Cancel
            </Button>
            <Button
              color="ice"
              radius={8}
              styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
              loading={fire.isPending}
              onClick={submit}
            >
              Fire
            </Button>
          </Group>
        </Stack>
      )}
    </Modal>
  );
}
