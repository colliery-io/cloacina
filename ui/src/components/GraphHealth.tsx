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

import { Badge, Code } from "@mantine/core";

/**
 * Defensive renderer for the free-form `health`/`status` JSON (T-0655).
 * The API types these as `unknown` ("free-form JSON for now"), so we never
 * assume a shape: pull out a `state` string when present (the common
 * `{"state": "running"|"stopped"}` case) and badge it; otherwise pretty-
 * print the raw JSON. Never crashes on an unexpected shape (REQ-007).
 */
export function healthState(value: unknown): string | null {
  if (value && typeof value === "object" && "state" in value) {
    const s = (value as { state: unknown }).state;
    if (typeof s === "string") return s;
  }
  return null;
}

const STATE_COLOR: Record<string, string> = { running: "green", stopped: "gray" };

export function GraphHealth({ value }: { value: unknown }) {
  const state = healthState(value);
  if (state) {
    return (
      <Badge variant="light" color={STATE_COLOR[state.toLowerCase()] ?? "blue"}>
        {state}
      </Badge>
    );
  }
  if (value == null) {
    return (
      <Badge variant="light" color="gray">
        unknown
      </Badge>
    );
  }
  return <Code fz="xs">{JSON.stringify(value)}</Code>;
}
