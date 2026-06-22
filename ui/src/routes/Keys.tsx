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

import { Alert, Box, Button, Code, CopyButton, Group, Modal, Select, Stack, Text, TextInput } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { IconKey } from "@tabler/icons-react";
import { useState } from "react";

import { classifyError } from "../api/errors";
import { useCreateKey, useKeys, useRevokeKey, type KeyInfo, type KeyRole } from "../api/keys";
import { MONO, PageHeader } from "../components/aurora";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";
import { TOKEN, pillBg } from "../util/tokens";

/**
 * API key management (Aurora Dark spec 12). List as card rows (key glyph + name
 * + prefix·scopes + created + Revoke); create + one-time plaintext reveal +
 * revoke-confirm preserved from the original.
 */
export function Keys() {
  const { data, isPending, isError, error, refetch } = useKeys();

  const [createOpen, createModal] = useDisclosure(false);
  const [name, setName] = useState("");
  const [role, setRole] = useState<KeyRole>("read");
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

  const items = data?.items ?? [];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <PageHeader
        title="API Keys"
        sub="Tenant-scoped keys for the SDK, CLI, and agents. Shown once at creation."
        right={
          <Button color="ice" radius={9} size="sm" styles={{ root: { color: "#0b0d10", fontWeight: 600 } }} onClick={createModal.open}>
            + Create key
          </Button>
        }
      />

      {isPending ? (
        <Loading label="Loading keys…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : items.length === 0 ? (
        <Empty message="No API keys for this tenant yet." />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 9 }}>
          {items.map((k) => (
            <Box key={k.id} style={{ background: "var(--panel-2)", border: "1px solid var(--border)", borderRadius: 10, padding: "12px 15px" }}>
              <Group justify="space-between" wrap="nowrap">
                <Group gap={12} wrap="nowrap" style={{ minWidth: 0 }}>
                  <Box style={{ width: 30, height: 30, borderRadius: 8, background: "var(--panel)", border: "1px solid var(--border)", display: "flex", alignItems: "center", justifyContent: "center", flex: "none" }}>
                    <IconKey size={15} color={TOKEN.ice} />
                  </Box>
                  <Box style={{ minWidth: 0 }}>
                    <Group gap={8} wrap="nowrap">
                      <span style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)" }}>{k.name}</span>
                      {k.revoked && (
                        <span style={{ background: pillBg(TOKEN.muted), color: TOKEN.muted, borderRadius: 10, padding: "1px 7px", fontFamily: MONO, fontSize: 10 }}>revoked</span>
                      )}
                    </Group>
                    <Box style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 2 }}>
                      {`clk_…${k.id.slice(0, 4)}`} · {k.permissions}
                    </Box>
                  </Box>
                </Group>
                <Group gap={16} wrap="nowrap" style={{ flex: "none" }}>
                  <Box style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)", textAlign: "right" }}>
                    created {formatTimestamp(k.created_at)}
                  </Box>
                  {!k.revoked && (
                    <Button size="compact-sm" variant="subtle" color="bad" onClick={() => setRevokeTarget(k)}>
                      Revoke
                    </Button>
                  )}
                </Group>
              </Group>
            </Box>
          ))}
        </div>
      )}

      {/* Create */}
      <Modal opened={createOpen} onClose={createModal.close} title="Create API key" centered>
        <Stack>
          <TextInput label="Name" placeholder="ci-deploy" value={name} onChange={(e) => setName(e.currentTarget.value)} data-autofocus />
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
            <Text c="bad" size="sm">
              {classifyError(create.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={createModal.close}>
              Cancel
            </Button>
            <Button color="ice" styles={{ root: { color: "#0b0d10", fontWeight: 600 } }} loading={create.isPending} disabled={!name.trim()} onClick={onCreate}>
              Create
            </Button>
          </Group>
        </Stack>
      </Modal>

      {/* One-time plaintext reveal */}
      <Modal opened={plaintext !== null} onClose={() => setPlaintext(null)} closeOnClickOutside={false} title={`Key "${plaintext?.name}" created`} centered>
        <Stack>
          <Alert color="gold" title="Copy this now — you won't see it again">
            This plaintext key is shown only once and is never stored by this UI.
          </Alert>
          <Code block>{plaintext?.key}</Code>
          <Group justify="space-between">
            <CopyButton value={plaintext?.key ?? ""}>
              {({ copied, copy }) => (
                <Button variant="light" color={copied ? "ok" : "ice"} onClick={copy}>
                  {copied ? "Copied" : "Copy key"}
                </Button>
              )}
            </CopyButton>
            <Button color="ice" styles={{ root: { color: "#0b0d10", fontWeight: 600 } }} onClick={() => setPlaintext(null)}>
              Done
            </Button>
          </Group>
        </Stack>
      </Modal>

      {/* Revoke confirm */}
      <Modal opened={revokeTarget !== null} onClose={() => setRevokeTarget(null)} title="Revoke key?" centered>
        <Stack>
          <Text size="sm">
            Revoke <b>{revokeTarget?.name}</b>? Any client using it will stop authenticating immediately. This cannot be undone.
          </Text>
          {revoke.isError && (
            <Text c="bad" size="sm">
              {classifyError(revoke.error).message}
            </Text>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setRevokeTarget(null)}>
              Cancel
            </Button>
            <Button color="bad" loading={revoke.isPending} onClick={onRevoke}>
              Revoke
            </Button>
          </Group>
        </Stack>
      </Modal>
    </div>
  );
}
