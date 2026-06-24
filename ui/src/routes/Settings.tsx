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

import { Dot, MONO, TOKEN } from "@colliery-io/aurora-dark";
import { Box, Group, SimpleGrid } from "@mantine/core";
import { type ReactNode } from "react";

import { useAuth } from "../auth/AuthContext";

/** Hairline section header (label + bottom rule). */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <Box>
      <Box style={{ fontSize: 14, fontWeight: 600, color: "var(--fg)", borderBottom: "1px solid var(--border-soft)", paddingBottom: 8, marginBottom: 12 }}>
        {title}
      </Box>
      {children}
    </Box>
  );
}

/** Read-only config card: Mono uppercase key + value. */
function ConfigCard({ label, value, color }: { label: string; value: ReactNode; color?: string }) {
  return (
    <Box style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 11, padding: "13px 16px" }}>
      <Box style={{ fontFamily: MONO, fontSize: 10.5, letterSpacing: ".06em", textTransform: "uppercase", color: "var(--faint)", marginBottom: 6 }}>
        {label}
      </Box>
      <Box style={{ fontFamily: MONO, fontSize: 12.5, color: color ?? "var(--fg)" }}>{value}</Box>
    </Box>
  );
}

/**
 * Settings (Aurora Dark spec 13). Connection (real, from the active session),
 * Server (config the server owns — shown read-only; values it doesn't expose to
 * the UI are marked server-managed), and Appearance.
 */
export function Settings() {
  const { connection } = useAuth();

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 22 }}>
      <Box style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)" }}>Settings</Box>

      <Section title="Connection">
        <SimpleGrid cols={{ base: 1, sm: 2 }} spacing={13}>
          <ConfigCard label="Tenant" value={connection?.tenant ?? "—"} />
          <ConfigCard label="Server URL" value={connection?.serverUrl ?? "—"} />
        </SimpleGrid>
      </Section>

      <Section title="Server">
        <SimpleGrid cols={{ base: 1, sm: 2 }} spacing={13}>
          <ConfigCard label="CLOACINA_BIND_ADDR" value={<span style={{ color: "var(--faint)" }}>server-managed</span>} />
          <ConfigCard label="DATABASE_URL" value={<span style={{ color: "var(--faint)" }}>server-managed</span>} />
          <ConfigCard label="SECRET_KEY" value="set · credentials encrypted" color={TOKEN.ok} />
          <ConfigCard label="SCHEDULER" value="enabled" />
        </SimpleGrid>
      </Section>

      <Section title="Appearance">
        <SimpleGrid cols={{ base: 1, sm: 2 }} spacing={13}>
          <Box style={{ background: "var(--panel)", border: `1px solid ${TOKEN.ice}7a`, borderRadius: 11, padding: "14px 16px" }}>
            <Group justify="space-between">
              <Group gap={9}>
                <Dot color={TOKEN.ice} />
                <span style={{ fontSize: 13, fontWeight: 500, color: "var(--fg)" }}>Aurora dark</span>
              </Group>
              <span style={{ fontFamily: MONO, fontSize: 10.5, color: TOKEN.ice }}>active</span>
            </Group>
          </Box>
          <Box style={{ background: "var(--panel)", border: "1px solid var(--border)", borderRadius: 11, padding: "14px 16px", opacity: 0.6 }}>
            <Group justify="space-between">
              <Group gap={9}>
                <Dot color={TOKEN.muted} />
                <span style={{ fontSize: 13, fontWeight: 500, color: "var(--muted)" }}>Light</span>
              </Group>
              <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>soon</span>
            </Group>
          </Box>
        </SimpleGrid>
      </Section>
    </div>
  );
}
