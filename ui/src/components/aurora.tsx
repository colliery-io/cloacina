/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Small shared Aurora Dark primitives (CLOACI-I-0129) — the repeated chrome the
 *  per-screen redesigns lean on: page header, status dot, filter chip, and the
 *  standard card surface. Kept intentionally light (inline styles over the
 *  theme.css tokens) so each route reads close to the spec.
 */
import { Box, Group } from "@mantine/core";
import type { CSSProperties, ReactNode } from "react";

export const MONO = "'IBM Plex Mono', monospace";

/** Standard card surface (panel + border + radius 10/11). */
export const cardSurface: CSSProperties = {
  background: "var(--panel)",
  border: "1px solid var(--border)",
  borderRadius: 11,
};

/** A small status dot, optionally with a soft glow ring. */
export function Dot({ color, glow, size = 8 }: { color: string; glow?: boolean; size?: number }) {
  return (
    <span
      style={{
        width: size,
        height: size,
        borderRadius: "50%",
        background: color,
        flex: "none",
        boxShadow: glow ? `0 0 0 3px ${color}22` : undefined,
      }}
    />
  );
}

/** Page header: title + Mono subtitle, with an optional right-aligned slot. */
export function PageHeader({ title, sub, right }: { title: string; sub?: ReactNode; right?: ReactNode }) {
  return (
    <Group justify="space-between" align="flex-start" mb={4}>
      <Box>
        <Box style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)" }}>{title}</Box>
        {sub != null && (
          <Box style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 2 }}>{sub}</Box>
        )}
      </Box>
      {right}
    </Group>
  );
}

/** Filter chip: active = ice fill + dark text; inactive = panel + border. */
export function Chip({
  label,
  count,
  active,
  onClick,
}: {
  label: string;
  count?: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        padding: "5px 12px",
        borderRadius: 20,
        fontSize: 12.5,
        cursor: "pointer",
        border: active ? "1px solid transparent" : "1px solid var(--border)",
        background: active ? "var(--ice)" : "var(--panel)",
        color: active ? "#0b0d10" : "var(--fg-2)",
        fontWeight: active ? 600 : 500,
      }}
    >
      {label}
      {count != null && (
        <span style={{ fontFamily: MONO, fontSize: 11, opacity: active ? 0.85 : 0.7 }}>{count}</span>
      )}
    </button>
  );
}
