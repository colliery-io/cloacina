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
 * Human-readable labels + colors for execution event types (CLOACI-I-0124 /
 * WS-9). The API emits raw snake_case tokens (`task_marked_ready`,
 * `workflow_completed`, …); this turns them into a readable phrase and a
 * status color so the event log reads at a glance instead of as machine noise.
 */
export type EventDescriptor = { label: string; color: string };

const EVENTS: Record<string, EventDescriptor> = {
  workflow_started: { label: "Workflow started", color: "blue" },
  workflow_completed: { label: "Workflow completed", color: "green" },
  workflow_failed: { label: "Workflow failed", color: "red" },
  task_marked_ready: { label: "Task ready", color: "gray" },
  task_started: { label: "Task started", color: "blue" },
  task_completed: { label: "Task completed", color: "green" },
  task_failed: { label: "Task failed", color: "red" },
  task_skipped: { label: "Task skipped", color: "salmon" },
  task_retrying: { label: "Task retrying", color: "orange" },
};

export function describeEvent(eventType: string): EventDescriptor {
  const entry = EVENTS[eventType.toLowerCase()];
  if (entry) return entry;
  // Unknown token — title-case it (underscores → spaces) so it still reads.
  const label = eventType
    .replace(/_/g, " ")
    .replace(/^\w/, (c) => c.toUpperCase());
  return { label, color: "gray" };
}

/** Event payloads are JSON strings; an empty object carries nothing worth
 *  showing. Returns pretty-printed JSON only when there's real content. */
export function meaningfulData(raw: string | null | undefined): string | null {
  if (raw == null || raw === "") return null;
  try {
    const parsed = JSON.parse(raw);
    if (parsed == null) return null;
    if (typeof parsed === "object" && Object.keys(parsed).length === 0) return null;
    return JSON.stringify(parsed, null, 2);
  } catch {
    return raw;
  }
}
