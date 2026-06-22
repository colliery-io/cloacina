/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Task source viewer (CLOACI-I-0129 → wires CLOACI-T-0750). Opened from an
 *  execution DAG node. Source is package-level in the compiled archive, but a
 *  whole-file dump is noise — by default we extract just the clicked task's
 *  definition (its `#[task]`/`@task` function) and offer a "full file" toggle.
 */
import { Box, Group, Modal } from "@mantine/core";
import { useEffect, useMemo, useState } from "react";

import { useWorkflowSource, type WorkflowSourceFile } from "../api/controls";
import { MONO } from "./aurora";

/** Extract a single task's definition (decorators/attributes + body) from a
 *  source file, returning the snippet + its 1-based start line, or null if the
 *  task isn't found in this file. Brace/indent matching is heuristic — good
 *  enough for read-only display of the simple functions cloacina tasks are. */
function extractTask(
  contents: string,
  taskName: string,
  path: string,
): { snippet: string; start: number } | null {
  const lines = contents.split("\n");

  if (/\.py$/.test(path)) {
    for (let i = 0; i < lines.length; i++) {
      const m = lines[i].match(/^(\s*)(?:async\s+)?def\s+([A-Za-z0-9_]+)\s*\(/);
      if (!m || m[2] !== taskName) continue;
      const indent = m[1].length;
      let start = i;
      while (start - 1 >= 0 && /^\s*(@|#)/.test(lines[start - 1])) start--;
      let end = i + 1;
      for (; end < lines.length; end++) {
        if (lines[end].trim() === "") continue;
        const ind = lines[end].match(/^(\s*)/)![1].length;
        if (ind <= indent) break;
      }
      return { snippet: lines.slice(start, end).join("\n"), start: start + 1 };
    }
    return null;
  }

  // Rust: a task is `fn <name>` preceded by attributes incl. `#[task...]`.
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/\bfn\s+([A-Za-z0-9_]+)\s*[(<]/);
    if (!m || m[1] !== taskName) continue;
    let start = i;
    while (start - 1 >= 0 && /^\s*(#\[|\/\/\/|\/\/!|\/\/|pub\b)/.test(lines[start - 1])) start--;
    // Require a #[task attribute in the gathered prelude, else it's a plain fn.
    const prelude = lines.slice(start, i + 1).join("\n");
    if (!/#\[\s*task/.test(prelude)) continue;
    let depth = 0;
    let opened = false;
    let end = i;
    for (let j = i; j < lines.length; j++) {
      for (const ch of lines[j]) {
        if (ch === "{") {
          depth++;
          opened = true;
        } else if (ch === "}") {
          depth--;
        }
      }
      end = j + 1;
      if (opened && depth <= 0) break;
    }
    return { snippet: lines.slice(start, end).join("\n"), start: start + 1 };
  }
  return null;
}

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

  // Find the clicked task's definition across the code files.
  const found = useMemo(() => {
    for (const f of files) {
      if (!/\.(rs|py)$/.test(f.path)) continue;
      const hit = extractTask(f.contents, taskName, f.path);
      if (hit) return { file: f, ...hit };
    }
    return null;
  }, [files, taskName]);

  const [mode, setMode] = useState<"task" | "file">("task");
  const [sel, setSel] = useState<string | null>(null);
  // Default to the file holding the task (or the first code file) and to
  // task-only view whenever the task could be isolated.
  useEffect(() => {
    setSel(found?.file.path ?? files[0]?.path ?? null);
    setMode(found ? "task" : "file");
  }, [found, files]);

  const active: WorkflowSourceFile | undefined = files.find((f) => f.path === sel) ?? files[0];
  const showTask = mode === "task" && found;
  const body = showTask ? found.snippet : active?.contents ?? "";
  const lineBase = showTask ? found.start : 1;
  const lines = body.split("\n");

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      size="xl"
      centered
      title={
        <Group gap={8}>
          <span style={{ fontFamily: MONO, fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>{taskName}</span>
          <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>· {showTask ? "task" : "source"}</span>
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
          {/* View toggle + (in file mode) file picker */}
          <Group justify="space-between" mb={10} align="center" style={{ flexWrap: "wrap", gap: 8 }}>
            <Group gap={6}>
              <Toggle active={mode === "task"} disabled={!found} onClick={() => setMode("task")} label="task only" />
              <Toggle active={mode === "file"} onClick={() => setMode("file")} label="full file" />
            </Group>
            <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
              {showTask ? found.file.path : active?.path}
            </span>
          </Group>

          {mode === "file" && (
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
          )}

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
                    {lineBase + i}
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

function Toggle({ active, disabled, onClick, label }: { active: boolean; disabled?: boolean; onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      style={{
        fontFamily: MONO,
        fontSize: 10.5,
        padding: "3px 10px",
        borderRadius: 8,
        cursor: disabled ? "not-allowed" : "pointer",
        border: "1px solid var(--border)",
        background: active ? "rgba(127,178,255,.13)" : "var(--panel)",
        color: disabled ? "var(--fainter)" : active ? "var(--fg)" : "var(--muted)",
      }}
    >
      {label}
    </button>
  );
}
