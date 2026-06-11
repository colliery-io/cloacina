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

import { Card, SimpleGrid, Stack, Text, Title } from "@mantine/core";

import { useAuth } from "../auth/AuthContext";

/**
 * Overview landing (CLOACI-I-0117 / REQ-002). The skeleton renders the shell
 * + a connected-state confirmation; the real rollup (recent executions,
 * counts, graph health) is built in T-0655.
 */
export function Overview() {
  const { connection } = useAuth();
  return (
    <Stack>
      <Title order={2}>Overview</Title>
      <Text c="dimmed">
        Connected to {connection?.serverUrl} as tenant <b>{connection?.tenant}</b>.
      </Text>
      <SimpleGrid cols={{ base: 1, sm: 3 }}>
        {["Recent executions", "Status rollup", "Graph health"].map((t) => (
          <Card key={t} withBorder padding="lg">
            <Text fw={600}>{t}</Text>
            <Text c="dimmed" size="sm">
              Coming in T-0655.
            </Text>
          </Card>
        ))}
      </SimpleGrid>
    </Stack>
  );
}
