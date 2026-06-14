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

import { Stack, Text, Title } from "@mantine/core";

/**
 * Placeholder for routes whose views land in later tasks — keeps the nav +
 * routing real in the skeleton so each feature task drops into a wired slot.
 */
export function Placeholder({ title, task }: { title: string; task: string }) {
  return (
    <Stack>
      <Title order={2}>{title}</Title>
      <Text c="dimmed">Built in {task}.</Text>
    </Stack>
  );
}

export function NotFound() {
  return (
    <Stack>
      <Title order={2}>Not found</Title>
      <Text c="dimmed">No such page.</Text>
    </Stack>
  );
}
