/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Graph-node inspector (CLOACI-T-0773). Replaces the slide-out Drawer with the
 *  same Modal experience as the workflow TaskCodeModal: the node's role/routing
 *  metadata up top, then its source code below. Graph source is retained under
 *  the package name (`GraphStatus.source_package`), so we fetch it and extract
 *  the clicked node's definition — a compute node's `fn`/`def`, or the
 *  `#[reactor(...)]` block — falling back to the full file at the first mention.
 */
import { Box, Divider, Group, Modal, Stack, Text } from "@mantine/core";
import { useEffect, useMemo, useState } from "react";

import { useWorkflowSource, type WorkflowSourceFile } from "../api/controls";
import { MONO, Pill } from "./aurora";
import { nodeKindColor } from "../util/tokens";

/** Node passed in from the graph view. `kind` is "Node" | "Reactor" |
 *  "Accumulator"; `name` is the id used to locate the definition in source. */
export interface GraphNodeTarget {
  name: string;
  kind: string;
  rows: [string, string][];
}

/** Walk back from a `fn`/`def` line over its doc-comment + (possibly multi-line)
 *  attribute/decorator prelude. Bracket-depth aware over `()`/`[]` so a
 *  multi-line `#[…]` / `@…(…)` is captured whole. (Mirrors TaskCodeModal.) */
function preludeStart(lines: string[], i: number, py: boolean): number {
  const head = py ? /^\s*[@#]/ : /^\s*(#!?\[|\/\/|pub\b|async\b|unsafe\b|const\b)/;
  let start = i;
  let depth = 0;
  while (start - 1 >= 0) {
    const prev = lines[start - 1];
    if (prev.trim() === "" && depth === 0) break;
    for (const ch of prev) {
      if (ch === ")" || ch === "]") depth++;
      else if (ch === "(" || ch === "[") depth--;
    }
    if (depth < 0) depth = 0;
    if (depth > 0 || head.test(prev)) {
      start--;
      continue;
    }
    break;
  }
  return start;
}

/** Extract a graph node's definition from a source file, or null if not found.
 *  Compute nodes resolve to their `fn`/`def`; reactors to the `#[reactor(...)]`
 *  attribute + the item it annotates. */
function extractFromFile(
  contents: string,
  path: string,
  target: GraphNodeTarget,
): { snippet: string; start: number } | null {
  const lines = contents.split("\n");
  const py = /\.py$/.test(path);

  if (target.kind === "Reactor") {
    // Rust: `#[reactor( … name = "<name>" … )]` + the struct it annotates.
    for (let i = 0; i < lines.length; i++) {
      if (!/#\[\s*reactor\b/.test(lines[i])) continue;
      let depth = 0;
      let end = i;
      for (let j = i; j < lines.length; j++) {
        for (const ch of lines[j]) {
          if (ch === "(" || ch === "[") depth++;
          else if (ch === ")" || ch === "]") depth--;
        }
        end = j;
        if (depth <= 0 && j > i) break;
      }
      // Absorb the annotated item line (the `pub struct …;`).
      let itemEnd = end + 1;
      while (itemEnd < lines.length && lines[itemEnd].trim() === "") itemEnd++;
      if (itemEnd < lines.length) itemEnd++;
      const block = lines.slice(i, itemEnd).join("\n");
      if (block.includes(target.name)) return { snippet: block, start: i + 1 };
    }
    return null;
  }

  // Compute node (and accumulator best-effort): the function definition.
  if (py) {
    for (let i = 0; i < lines.length; i++) {
      const m = lines[i].match(/^(\s*)(?:async\s+)?def\s+([A-Za-z0-9_]+)\s*\(/);
      if (!m || m[2] !== target.name) continue;
      const indent = m[1].length;
      const start = preludeStart(lines, i, true);
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
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/\bfn\s+([A-Za-z0-9_]+)\s*[(<]/);
    if (!m || m[1] !== target.name) continue;
    const start = preludeStart(lines, i, false);
    let depth = 0;
    let opened = false;
    let end = i;
    for (let j = i; j < lines.length; j++) {
      for (const ch of lines[j]) {
        if (ch === "{") {
          depth++;
          opened = true;
        } else if (ch === "}") depth--;
      }
      end = j + 1;
      if (opened && depth <= 0) break;
    }
    return { snippet: lines.slice(start, end).join("\n"), start: start + 1 };
  }
  return null;
}

export function GraphNodeModal({
  packageName,
  target,
  opened,
  onClose,
}: {
  packageName: string | null;
  target: GraphNodeTarget | null;
  opened: boolean;
  onClose: () => void;
}) {
  const src = useWorkflowSource(packageName ?? "", { enabled: opened && !!packageName });

  const files = useMemo(() => {
    const all = src.data?.files ?? [];
    const code = all.filter((f) => /\.(rs|py)$/.test(f.path));
    return code.length ? [...code, ...all.filter((f) => !code.includes(f))] : all;
  }, [src.data]);

  const found = useMemo(() => {
    if (!target) return null;
    for (const f of files) {
      if (!/\.(rs|py)$/.test(f.path)) continue;
      const hit = extractFromFile(f.contents, f.path, target);
      if (hit) return { file: f, ...hit };
    }
    return null;
  }, [files, target]);

  const [mode, setMode] = useState<"def" | "file">("def");
  const [sel, setSel] = useState<string | null>(null);
  useEffect(() => {
    const mentions = target
      ? files.find((f) => /\.(rs|py)$/.test(f.path) && f.contents.includes(target.name))
      : undefined;
    setSel(found?.file.path ?? mentions?.path ?? files[0]?.path ?? null);
    setMode(found ? "def" : "file");
  }, [found, files, target]);

  const active: WorkflowSourceFile | undefined = files.find((f) => f.path === sel) ?? files[0];
  const showDef = mode === "def" && found;
  const body = showDef ? found.snippet : active?.contents ?? "";
  const lineBase = showDef ? found.start : 1;
  const lines = body.split("\n");

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      size="xl"
      centered
      title={
        <Group gap={8}>
          <span style={{ fontFamily: MONO, fontSize: 14, fontWeight: 600, color: "var(--fg)" }}>
            {target?.name}
          </span>
          <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>
            · {target?.kind.toLowerCase()}
          </span>
        </Group>
      }
    >
      {target && (
        <Stack gap="sm">
          {/* Role / routing metadata (kept from the old Drawer). */}
          <span style={{ display: "inline-flex", width: "fit-content" }}>
            <Pill color={nodeKindColor(target.kind.toLowerCase())}>{target.kind}</Pill>
          </span>
          {target.rows.map(([k, v]) => (
            <div key={k}>
              <Text size="xs" c="dimmed">
                {k}
              </Text>
              <Text size="sm">{v}</Text>
            </div>
          ))}

          <Divider label="Source" labelPosition="left" />

          {!packageName ? (
            <Text size="xs" c="dimmed">
              Source package couldn't be resolved for this graph (it declares no typed surface).
            </Text>
          ) : src.isPending ? (
            <Box style={{ color: "var(--faint)", fontSize: 13 }}>Loading source…</Box>
          ) : src.isError ? (
            <Box style={{ color: "var(--bad)", fontSize: 13 }}>
              Source unavailable{src.error instanceof Error ? `: ${src.error.message}` : ""}.
            </Box>
          ) : files.length === 0 ? (
            <Box style={{ color: "var(--muted)", fontSize: 13 }}>No source retained for this package.</Box>
          ) : (
            <Box>
              <Group justify="space-between" mb={10} align="center" style={{ flexWrap: "wrap", gap: 8 }}>
                <Group gap={6}>
                  <Toggle active={mode === "def"} disabled={!found} onClick={() => setMode("def")} label={target.kind.toLowerCase()} />
                  <Toggle active={mode === "file"} onClick={() => setMode("file")} label="full file" />
                </Group>
                <span style={{ fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
                  {showDef ? found.file.path : active?.path}
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

              <Box
                style={{
                  background: "var(--inset)",
                  border: "1px solid var(--border-soft)",
                  borderRadius: 10,
                  maxHeight: "52vh",
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
        </Stack>
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
