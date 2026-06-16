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

import { Anchor, Badge, Button, Card, Group, Stack, Text, Title } from "@mantine/core";

import { useAuth } from "../auth/AuthContext";

/** Show only the tail of the bearer key — enough to tell two keys apart
 *  without printing the secret on screen. */
function maskKey(key: string): string {
  if (key.length <= 4) return "••••";
  return `••••••••${key.slice(-4)}`;
}

/**
 * Settings (CLOACI-I-0124 / WS-7). The active connection is the one thing
 * "settings" can show today without inventing a new server capability: where
 * the UI is pointed, which tenant it's scoped to, and how it's authenticated —
 * plus the disconnect action. The key is masked (it lives in sessionStorage,
 * cleared on tab close); there's nothing server-side to persist here.
 */
export function Settings() {
  const { connection, disconnect } = useAuth();

  return (
    <Stack>
      <Title order={2}>Settings</Title>

      <Card withBorder padding="lg" maw={560}>
        <Group justify="space-between" mb="sm">
          <Title order={4}>Connection</Title>
          <Badge color="green" variant="light">
            connected
          </Badge>
        </Group>
        {connection ? (
          <Stack gap="sm">
            <Row label="Server">
              <Anchor href={connection.serverUrl} target="_blank" rel="noreferrer" size="sm">
                {connection.serverUrl}
              </Anchor>
            </Row>
            <Row label="Tenant">
              <Text size="sm">{connection.tenant}</Text>
            </Row>
            <Row label="Authentication">
              <Text size="sm">
                Bearer key <Text span c="dimmed">({maskKey(connection.apiKey)})</Text>
              </Text>
            </Row>
            <Text size="xs" c="dimmed">
              The key is held in this tab's session storage and cleared when the tab closes —
              it is never persisted by the UI.
            </Text>
            <Group justify="flex-end" mt="xs">
              <Button color="red" variant="light" onClick={disconnect}>
                Disconnect
              </Button>
            </Group>
          </Stack>
        ) : (
          <Text c="dimmed" size="sm">
            Not connected.
          </Text>
        )}
      </Card>
    </Stack>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Group justify="space-between" gap="xl">
      <Text size="sm" c="dimmed">
        {label}
      </Text>
      {children}
    </Group>
  );
}
