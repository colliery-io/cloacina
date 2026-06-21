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

import { statusColor, pillBg } from "../util/tokens";

/**
 * Execution-status pill (Aurora Dark, CLOACI-I-0129): the status color at full
 * strength on a tinted (`1c` alpha) background — radius 10, Plex Mono. Reused
 * across overview, executions, drawers.
 */
export function StatusBadge({ status }: { status: string }) {
  const color = statusColor(status);
  return (
    <span
      style={{
        display: "inline-block",
        background: pillBg(color),
        color,
        borderRadius: 10,
        padding: "2px 9px",
        fontFamily: "'IBM Plex Mono', monospace",
        fontSize: 10.5,
        fontWeight: 500,
        lineHeight: 1.5,
        textTransform: "lowercase",
        whiteSpace: "nowrap",
      }}
    >
      {status}
    </span>
  );
}
