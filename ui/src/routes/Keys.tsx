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

import {
  Alert,
  Badge,
  Button,
  Code,
  CopyButton,
  Group,
  Modal,
  Select,
  Stack,
  Table,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useState } from "react";

import { classifyError } from "../api/errors";
import { useCreateKey, useKeys, useRevokeKey, type KeyInfo, type KeyRole } from "../api/keys";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";

const ROLE_COLOR: Record<string, string> = {
  admin: "red",
  write: "blue",
  read: "gray",
};

/**
 * API key management (T-0658 / REQ-006, UC-4). List + create + revoke for the
 * active tenant. The created key's plaintext is shown exactly once in a
 * dismiss-to-destroy panel — never persisted, never re-displayed.
 */
export function Keys() {
  const { data, isPending, isError, error, refetch } = useKeys();

  const [createOpen, createModal] = useDisclosure(false);
  const [name, setName] = useState("");
  const [role, setRole] = useState<KeyRole>("read");
  // One-time plaintext lives ONLY here, in transient state. Dismissing clears it.
  const [plaintext, setPlaintext] = useState<{ name: string; key: string } | null>(null);

  const [revokeTarget, setRevokeTarget] = useState<KeyInfo | null>(null);

  const create = useCreateKey();
  const revoke = useRevokeKey();

  function onCreate() {
    create.mutate(
      { name: name.trim(), role },
      {
        onSuccess: (res) => {
          createModal.close();
          setName("");
          setRole("read");
          setPlaintext({ name: res.name, key: res.key });
        },
      },
    );
  }

  function onRevoke() {
    if (!revokeTarget) return;
    revoke.mutate(revokeTarget.id, { onSuccess: () => setRevokeTarget(null) });
  }

  return (
    <Stack>
      <Group justify="space-between">
        <Title order={2}>API Keys</Title>
        <Button onClick={createModal.open}>Create key</Button>
      </Group>

      {isPending ? (
        <Loading label="Loading keys…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : data.items.length === 0 ? (
        <Empty message="No API keys for this tenant yet." />
      ) : (
        <Table highlightOnHover stickyHeader>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Name</Table.Th>
              <Table.Th>Role</Table.Th>
              <Table.Th>Created</Table.Th>
              <Table.Th>Status</Table.Th>
              <Table.Th />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {data.items.map((k) => (
              <Table.Tr key={k.id}>
                <Table.Td>
                  <Text fw={500}>{k.name}</Text>
                </Table.Td>
                <Table.Td>
                  <Badge color={ROLE_COLOR[k.permissions] ?? "gray"} variant="light">
                    {k.permissions}
                  </Badge>
                </Table.Td>
                <Table.Td>{formatTimestamp(k.created_at)}</Table.Td>
                <Table.Td>
                  {k.revoked ? (
                    <Badge color="gray" variant="outline">
                      revoked
                    </Badge>
                  ) : (
                    <Badge color="green" variant="light">
                      active
                    </Badge>
                  )}
                </Table.Td>
                <Table.Td>
                  {!k.revoked && (
                    <Button
                      size="xs"
                      color="red"
                      variant="subtle"
                      onClick={() => setRevokeTarget(k)}
                    >
                      Revoke
                    </Button>
                  )}
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}

      {/* Create */}
      <Modal opened={createOpen} onClose={createModal.close} title="Create API key">
        <Stack>
          <TextInput
            label="Name"
            placeholder="ci-deploy"
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            data-autofocus
          />
          <Select
            label="Role"
            data={[
              { value: "read", label: "read" },
              { value: "write", label: "write" },
              { value: "admin", label: "admin" },
            ]}
            value={role}
            onChange={(v) => v && setRole(v as KeyRole)}
            allowDeselect={false}
          />
          {create.isError && (
            <Text c="red" size="sm">
              {classifyError(create.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={createModal.close}>
              Cancel
            </Button>
            <Button loading={create.isPending} disabled={!name.trim()} onClick={onCreate}>
              Create
            </Button>
          </Group>
        </Stack>
      </Modal>

      {/* One-time plaintext reveal — dismiss to destroy. */}
      <Modal
        opened={plaintext !== null}
        onClose={() => setPlaintext(null)}
        closeOnClickOutside={false}
        title={`Key "${plaintext?.name}" created`}
      >
        <Stack>
          <Alert color="yellow" title="Copy this now — you won't see it again">
            This plaintext key is shown only once and is never stored by this UI. Copy it to a
            secure location before dismissing.
          </Alert>
          <Code block>{plaintext?.key}</Code>
          <Group justify="space-between">
            <CopyButton value={plaintext?.key ?? ""}>
              {({ copied, copy }) => (
                <Button variant="light" color={copied ? "teal" : "blue"} onClick={copy}>
                  {copied ? "Copied" : "Copy key"}
                </Button>
              )}
            </CopyButton>
            <Button onClick={() => setPlaintext(null)}>Done</Button>
          </Group>
        </Stack>
      </Modal>

      {/* Revoke confirm */}
      <Modal
        opened={revokeTarget !== null}
        onClose={() => setRevokeTarget(null)}
        title="Revoke key?"
      >
        <Stack>
          <Text size="sm">
            Revoke <b>{revokeTarget?.name}</b>? Any client using it will stop authenticating
            immediately. This cannot be undone.
          </Text>
          {revoke.isError && (
            <Text c="red" size="sm">
              {classifyError(revoke.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setRevokeTarget(null)}>
              Cancel
            </Button>
            <Button color="red" loading={revoke.isPending} onClick={onRevoke}>
              Revoke
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
