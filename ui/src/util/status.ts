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

/**
 * Execution-status helpers (T-0653). Status strings come from the server;
 * we match case-insensitively and fall back gracefully rather than asserting
 * a fixed enum (REQ-007 defensive rendering).
 */

const COLOR: Record<string, string> = {
  completed: "green",
  failed: "red",
  running: "blue",
  pending: "gray",
  scheduled: "gray",
  cancelled: "orange",
  canceled: "orange",
};

const TERMINAL = new Set(["completed", "failed", "cancelled", "canceled"]);

export function executionStatusColor(status: string): string {
  return COLOR[status.toLowerCase()] ?? "gray";
}

/**
 * Whether a status is terminal (run is over). T-0656 uses this to decide
 * whether to open a live stream at all — a terminal execution just shows
 * its history.
 */
export function isTerminalStatus(status: string): boolean {
  return TERMINAL.has(status.toLowerCase());
}
