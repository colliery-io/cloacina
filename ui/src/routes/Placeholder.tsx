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

import { Box } from "@mantine/core";

import { PageHeader } from "../components/aurora";

/**
 * Placeholder for routes whose views aren't built yet — keeps the nav +
 * routing real so the page exists. Shows a neutral "coming soon" rather than
 * leaking an internal task code to the user (CLOACI-I-0124 / WS-7).
 */
export function Placeholder({ title }: { title: string }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <PageHeader title={title} />
      <Box style={{ border: "1px dashed var(--border)", borderRadius: 10, padding: "18px 15px", color: "var(--faint)", fontSize: 12.5 }}>
        This area isn't available yet — coming soon.
      </Box>
    </div>
  );
}

export function NotFound() {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <PageHeader title="Not found" />
      <Box style={{ color: "var(--faint)", fontSize: 13 }}>No such page.</Box>
    </div>
  );
}
