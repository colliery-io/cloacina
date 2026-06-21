/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Task source viewer (CLOACI-I-0129 → wires CLOACI-T-0750). Opened from an
 *  execution DAG node; shows the package's retained source (a file picker + a
 *  read-only code view). Source is package-level (not per-task) in the compiled
 *  archive, so the clicked task is shown as context in the title.
 */
import { Box, Group, Modal } from "@mantine/core";
import { useEffect, useMemo, useState } from "react";

import { useWorkflowSource } from "../api/controls";
import { MONO } from "./aurora";

export function TaskCodeModal({
  packageName,
  taskName,
  opened,
  onClose,
}: {
  packageName: string;
  taskName: string;
  opened: boolean;
  onClose: () => void;
}) {
  const src = useWorkflowSource(packageName, { enabled: opened && !!packageName });

  // Prefer code files (rust/python) over manifests for the default view.
  const files = useMemo(() => {
    const all = src.data?.files ?? [];
    const code = all.filter((f) => /\.(rs|py)$/.test(f.path));
    return code.length ? [...code, ...all.filter((f) => !code.includes(f))] : all;
  }, [src.data]);

  const [sel, setSel] = useState<string | null>(null);
  useEffect(() => {
    setSel(files[0]?.path ?? null);
  }, [files]);

  const active = files.find((f) => f.path === sel) ?? files[0];
  const lines = (active?.contents ?? "").split("\n");

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      size="xl"
      centered
      title={
        <Group gap={8}>
          <span style={{ fontFamily: MONO, fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{taskName}</span>
          <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>· source</span>
        </Group>
      }
    >
      {src.isPending ? (
        <Box style={{ color: "var(--faint)", fontSize: 13 }}>Loading source…</Box>
      ) : src.isError ? (
        <Box style={{ color: "var(--bad)", fontSize: 13 }}>
          Source unavailable{src.error instanceof Error ? `: ${src.error.message}` : ""}.
        </Box>
      ) : files.length === 0 ? (
        <Box style={{ color: "var(--muted)", fontSize: 13 }}>No source retained for this package.</Box>
      ) : (
        <Box>
          {/* File picker */}
          <Group gap={6} mb={10} style={{ flexWrap: "wrap" }}>
            {files.map((f) => (
              <button
                key={f.path}
                type="button"
                onClick={() => setSel(f.path)}
                style={{
                  fontFamily: MONO,
                  fontSize: 10.5,
                  padding: "3px 9px",
                  borderRadius: 8,
                  cursor: "pointer",
                  border: "1px solid var(--border)",
                  background: f.path === active?.path ? "rgba(127,178,255,.13)" : "var(--panel)",
                  color: f.path === active?.path ? "var(--fg)" : "var(--muted)",
                }}
              >
                {f.path}
              </button>
            ))}
          </Group>

          {/* Code view */}
          <Box
            style={{
              background: "var(--inset)",
              border: "1px solid var(--border-soft)",
              borderRadius: 10,
              maxHeight: "60vh",
              overflow: "auto",
            }}
          >
            <Box style={{ fontFamily: MONO, fontSize: 12, lineHeight: 1.65, padding: "10px 0" }}>
              {lines.map((line, i) => (
                <Box key={i} style={{ display: "flex", whiteSpace: "pre" }}>
                  <span style={{ width: 44, flex: "none", textAlign: "right", paddingRight: 12, color: "var(--fainter)", userSelect: "none" }}>
                    {i + 1}
                  </span>
                  <span style={{ color: "#cdd4dc" }}>{line || " "}</span>
                </Box>
              ))}
            </Box>
          </Box>
        </Box>
      )}
    </Modal>
  );
}
