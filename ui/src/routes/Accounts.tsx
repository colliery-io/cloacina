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
  Alert,
  Badge,
  Box,
  Button,
  Group,
  Modal,
  PasswordInput,
  Select,
  Stack,
  Table,
  TextInput,
} from "@mantine/core";
import { useState } from "react";

import {
  useAccounts,
  useCreateAccount,
  useDisableAccount,
  useResetPassword,
  type AccountInfo,
} from "../api/accounts";
import { useCan, useTenant } from "../auth/AuthContext";

/**
 * Tenant-admin local-account management (CLOACI-T-0798). Create / list /
 * disable / reset-password for the connected tenant's self-managed accounts.
 * A non-admin key gets 403 → a clear "insufficient permissions" state.
 */
export function Accounts() {
  const { canAdmin } = useCan();
  const tenant = useTenant();
  const list = useAccounts();
  const create = useCreateAccount();
  const disable = useDisableAccount();
  const reset = useResetPassword();

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<string>("read");
  const [resetFor, setResetFor] = useState<AccountInfo | null>(null);
  const [newPassword, setNewPassword] = useState("");

  function submitCreate() {
    if (!username.trim() || !password) return;
    create.mutate(
      { username: username.trim(), password, role },
      {
        onSuccess: () => {
          setUsername("");
          setPassword("");
          setRole("read");
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
        Local accounts
      </Box>
      <Box style={{ fontSize: 12.5, color: "var(--muted)", marginBottom: 18 }}>
        Self-managed username/password accounts for tenant <b>{tenant}</b>. Users sign in at the
        connect screen with these credentials.
      </Box>

      {/* Create — admin only; non-admins see an explanatory Alert. */}
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
            Create account
          </Box>
          <Group align="flex-end" gap={10}>
            <TextInput
              label="Username"
              value={username}
              onChange={(e) => setUsername(e.currentTarget.value)}
              style={{ flex: 1 }}
            />
            <PasswordInput
              label="Initial password"
              value={password}
              onChange={(e) => setPassword(e.currentTarget.value)}
              style={{ flex: 1 }}
            />
            <Select
              label="Role"
              data={["read", "write", "admin"]}
              value={role}
              onChange={(v) => setRole(v ?? "read")}
              allowDeselect={false}
              w={120}
            />
            <Button
              color="ice"
              onClick={submitCreate}
              loading={create.isPending}
              disabled={!username.trim() || !password}
              styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            >
              Create
            </Button>
          </Group>
          {create.isError && (
            <Alert color="bad" variant="light" mt={10}>
              {classifyError(create.error).message}
            </Alert>
          )}
        </Box>
      ) : (
        <Alert color="neutral" variant="light" mb={18}>
          You need admin access to manage accounts.
        </Alert>
      )}

      {/* List */}
      {list.isError ? (
        <Alert color="bad" variant="light">
          {classifyError(list.error).message}
        </Alert>
      ) : list.isPending ? (
        <Box style={{ color: "var(--muted)", fontSize: 13 }}>Loading…</Box>
      ) : list.data.items.length === 0 ? (
        <Box style={{ color: "var(--muted)", fontSize: 13 }}>No local accounts yet.</Box>
      ) : (
        <Table highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Username</Table.Th>
              <Table.Th>Role</Table.Th>
              <Table.Th>Status</Table.Th>
              <Table.Th />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {list.data.items.map((a) => (
              <Table.Tr key={a.id}>
                <Table.Td style={{ fontWeight: 500 }}>{a.username}</Table.Td>
                <Table.Td>{a.role}</Table.Td>
                <Table.Td>
                  <Badge color={a.status === "active" ? "ok" : "neutral"} variant="light">
                    {a.status}
                  </Badge>
                </Table.Td>
                <Table.Td>
                  {canAdmin && (
                    <Group gap={6} justify="flex-end">
                      <Button size="xs" variant="subtle" onClick={() => setResetFor(a)}>
                        Reset password
                      </Button>
                      <Button
                        size="xs"
                        variant="subtle"
                        color="bad"
                        disabled={a.status !== "active"}
                        onClick={() => disable.mutate(a.id)}
                      >
                        Disable
                      </Button>
                    </Group>
                  )}
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}

      {/* Reset-password modal */}
      <Modal
        opened={resetFor !== null}
        onClose={() => {
          setResetFor(null);
          setNewPassword("");
        }}
        title={resetFor ? `Reset password — ${resetFor.username}` : ""}
        centered
      >
        <Stack gap={12}>
          <PasswordInput
            label="New password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.currentTarget.value)}
          />
          {reset.isError && (
            <Alert color="bad" variant="light">
              {classifyError(reset.error).message}
            </Alert>
          )}
          <Button
            color="ice"
            disabled={!newPassword}
            loading={reset.isPending}
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            onClick={() => {
              if (!resetFor) return;
              reset.mutate(
                { accountId: resetFor.id, password: newPassword },
                {
                  onSuccess: () => {
                    setResetFor(null);
                    setNewPassword("");
                  },
                },
              );
            }}
          >
            Reset password
          </Button>
        </Stack>
      </Modal>
    </Box>
  );
}
