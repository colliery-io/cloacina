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

import { classifyError } from "@colliery-io/aurora-dark";
import {
  ActionIcon,
  Alert,
  Box,
  Button,
  Group,
  Modal,
  PasswordInput,
  Stack,
  Table,
  TextInput,
} from "@mantine/core";
import { useState } from "react";

import {
  useCreateSecret,
  useDeleteSecret,
  useRotateSecret,
  useSecrets,
  type SecretMetadata,
} from "../api/secrets";
import { useCan, useTenant } from "../auth/AuthContext";

interface FieldRow {
  key: string;
  value: string;
}

/** Build a `{key: value}` map from the editable rows, dropping empty keys. */
function rowsToFields(rows: FieldRow[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const r of rows) {
    const k = r.key.trim();
    if (k) out[k] = r.value;
  }
  return out;
}

/**
 * Tenant secrets (CLOACI-I-0133 / T-0862). Metadata list (name, fields,
 * timestamps) + create / rotate / delete. Value inputs are WRITE-ONLY: they are
 * never populated from a GET (reads carry no values), and rotate clears them.
 */
export function Secrets() {
  const { canAdmin } = useCan();
  const tenant = useTenant();
  const list = useSecrets();
  const create = useCreateSecret();
  const rotate = useRotateSecret();
  const del = useDeleteSecret();

  // Create form.
  const [name, setName] = useState("");
  const [rows, setRows] = useState<FieldRow[]>([{ key: "", value: "" }]);

  // Rotate modal.
  const [rotateFor, setRotateFor] = useState<SecretMetadata | null>(null);
  const [rotateRows, setRotateRows] = useState<FieldRow[]>([]);

  function resetCreate() {
    setName("");
    setRows([{ key: "", value: "" }]);
  }

  function submitCreate() {
    const fields = rowsToFields(rows);
    if (!name.trim() || Object.keys(fields).length === 0) return;
    create.mutate({ name: name.trim(), fields }, { onSuccess: resetCreate });
  }

  function openRotate(s: SecretMetadata) {
    setRotateFor(s);
    // Seed rows from the KNOWN field names but with EMPTY (write-only) values.
    setRotateRows(s.field_names.map((k) => ({ key: k, value: "" })));
  }

  function submitRotate() {
    if (!rotateFor) return;
    const fields = rowsToFields(rotateRows);
    if (Object.keys(fields).length === 0) return;
    rotate.mutate(
      { name: rotateFor.name, body: { fields } },
      {
        onSuccess: () => {
          setRotateFor(null);
          setRotateRows([]);
        },
      },
    );
  }

  return (
    <Box style={{ maxWidth: 820 }}>
      <Box
        component="h2"
        style={{ fontSize: 20, fontWeight: 600, color: "var(--fg)", margin: 0, marginBottom: 4 }}
      >
        Secrets
      </Box>
      <Box style={{ fontSize: 12.5, color: "var(--muted)", marginBottom: 18 }}>
        Encrypted, named-field credentials for tenant <b>{tenant}</b>. Values are write-only — they
        are never shown after creation. Packaged workflows reference a secret by name and resolve it
        at run time; rotating a value takes effect on the next fire.
      </Box>

      {/* Create — admin only. */}
      {canAdmin ? (
        <Box
          style={{
            background: "var(--sidebar)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            padding: 16,
            marginBottom: 18,
          }}
        >
          <Box style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)", marginBottom: 10 }}>
            Create secret
          </Box>
          <Stack gap={10}>
            <TextInput
              label="Name"
              placeholder="db_prod"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
            />
            <Box style={{ fontSize: 12, color: "var(--muted)" }}>Fields</Box>
            {rows.map((row, i) => (
              <Group key={i} align="flex-end" gap={8}>
                <TextInput
                  label={i === 0 ? "Field" : undefined}
                  placeholder="password"
                  value={row.key}
                  onChange={(e) => {
                    const next = [...rows];
                    next[i] = { ...next[i], key: e.currentTarget.value };
                    setRows(next);
                  }}
                  style={{ flex: 1 }}
                />
                <PasswordInput
                  label={i === 0 ? "Value (write-only)" : undefined}
                  value={row.value}
                  onChange={(e) => {
                    const next = [...rows];
                    next[i] = { ...next[i], value: e.currentTarget.value };
                    setRows(next);
                  }}
                  style={{ flex: 1 }}
                />
                <ActionIcon
                  variant="subtle"
                  color="bad"
                  disabled={rows.length === 1}
                  onClick={() => setRows(rows.filter((_, j) => j !== i))}
                  aria-label="remove field"
                >
                  ✕
                </ActionIcon>
              </Group>
            ))}
            <Group justify="space-between">
              <Button
                size="xs"
                variant="subtle"
                onClick={() => setRows([...rows, { key: "", value: "" }])}
              >
                + Add field
              </Button>
              <Button
                color="ice"
                onClick={submitCreate}
                loading={create.isPending}
                disabled={!name.trim()}
                styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
              >
                Create
              </Button>
            </Group>
          </Stack>
          {create.isError && (
            <Alert color="bad" variant="light" mt={10}>
              {classifyError(create.error).message}
            </Alert>
          )}
        </Box>
      ) : (
        <Alert color="neutral" variant="light" mb={18}>
          You need admin access to manage secrets.
        </Alert>
      )}

      {/* List — metadata only. */}
      {list.isError ? (
        <Alert color="bad" variant="light">
          {classifyError(list.error).message}
        </Alert>
      ) : list.isPending ? (
        <Box style={{ color: "var(--muted)", fontSize: 13 }}>Loading…</Box>
      ) : list.data.items.length === 0 ? (
        <Box style={{ color: "var(--muted)", fontSize: 13 }}>No secrets yet.</Box>
      ) : (
        <Table highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Name</Table.Th>
              <Table.Th>Fields</Table.Th>
              <Table.Th>Updated</Table.Th>
              <Table.Th />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {list.data.items.map((s) => (
              <Table.Tr key={s.id}>
                <Table.Td style={{ fontWeight: 500 }}>{s.name}</Table.Td>
                <Table.Td style={{ color: "var(--muted)", fontSize: 12.5 }}>
                  {s.field_names.join(", ")}
                </Table.Td>
                <Table.Td style={{ color: "var(--muted)", fontSize: 12.5 }}>
                  {new Date(s.updated_at).toLocaleString()}
                </Table.Td>
                <Table.Td>
                  {canAdmin && (
                    <Group gap={6} justify="flex-end">
                      <Button size="xs" variant="subtle" onClick={() => openRotate(s)}>
                        Rotate
                      </Button>
                      <Button
                        size="xs"
                        variant="subtle"
                        color="bad"
                        onClick={() => del.mutate(s.name)}
                      >
                        Delete
                      </Button>
                    </Group>
                  )}
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}

      {/* Rotate modal — write-only values, seeded with known field names. */}
      <Modal
        opened={rotateFor !== null}
        onClose={() => {
          setRotateFor(null);
          setRotateRows([]);
        }}
        title={rotateFor ? `Rotate — ${rotateFor.name}` : ""}
        centered
      >
        <Stack gap={10}>
          <Box style={{ fontSize: 12, color: "var(--muted)" }}>
            Enter new values. Existing values are never shown; rotation replaces the whole field
            map.
          </Box>
          {rotateRows.map((row, i) => (
            <Group key={i} align="flex-end" gap={8}>
              <TextInput
                label="Field"
                value={row.key}
                onChange={(e) => {
                  const next = [...rotateRows];
                  next[i] = { ...next[i], key: e.currentTarget.value };
                  setRotateRows(next);
                }}
                style={{ flex: 1 }}
              />
              <PasswordInput
                label="New value"
                value={row.value}
                onChange={(e) => {
                  const next = [...rotateRows];
                  next[i] = { ...next[i], value: e.currentTarget.value };
                  setRotateRows(next);
                }}
                style={{ flex: 1 }}
              />
            </Group>
          ))}
          <Group justify="space-between">
            <Button
              size="xs"
              variant="subtle"
              onClick={() => setRotateRows([...rotateRows, { key: "", value: "" }])}
            >
              + Add field
            </Button>
            <Button
              color="ice"
              loading={rotate.isPending}
              styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
              onClick={submitRotate}
            >
              Rotate
            </Button>
          </Group>
          {rotate.isError && (
            <Alert color="bad" variant="light">
              {classifyError(rotate.error).message}
            </Alert>
          )}
        </Stack>
      </Modal>

      {del.isError && (
        <Alert color="bad" variant="light" mt={12}>
          {classifyError(del.error).message}
        </Alert>
      )}
    </Box>
  );
}
