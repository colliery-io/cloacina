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

import { Tooltip } from "@mantine/core";

import { explainToken } from "../util/vocab";
import { MONO, Pill } from "./aurora";
import { healthColor, TOKEN } from "../util/tokens";

/**
 * Defensive renderer for the free-form `health`/`status` JSON (T-0655).
 * The API types these as `unknown` ("free-form JSON for now"), so we never
 * assume a shape: a bare string (accumulator `status`, e.g. `"socket_only"`),
 * or a `{"state": ...}` object (graph health) both resolve to a state token;
 * anything else pretty-prints. Never crashes on an unexpected shape (REQ-007).
 */
export function healthState(value: unknown): string | null {
  if (typeof value === "string") return value;
  if (value && typeof value === "object" && "state" in value) {
    const s = (value as { state: unknown }).state;
    if (typeof s === "string") return s;
  }
  return null;
}

/**
 * Badges a graph/accumulator state with a plain-language label and an
 * explanatory tooltip (CLOACI-I-0124 / WS-7) — no more raw quoted enum
 * strings like `"socket_only"` leaking to the operator. Aurora token pill,
 * colored by health (healthy → ok, transient → gold, down → bad, else muted).
 */
export function GraphHealth({ value }: { value: unknown }) {
  const state = healthState(value);
  if (state != null) {
    const { label, tip } = explainToken(state);
    return (
      <Tooltip label={tip} disabled={!tip} multiline w={260} withArrow>
        <span style={{ display: "inline-flex" }}>
          <Pill color={healthColor(state)}>{label}</Pill>
        </span>
      </Tooltip>
    );
  }
  if (value == null) {
    return <Pill color={TOKEN.muted}>unknown</Pill>;
  }
  return (
    <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>{JSON.stringify(value)}</span>
  );
}
