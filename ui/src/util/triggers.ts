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
 * Trigger-type vocabulary for the Triggers view (CLOACI-I-0124 / WS-6).
 *
 * The server's `schedules` table has exactly two rows kinds: `cron` (driven by
 * the cron scheduler) and `trigger` (a custom function polled on a fixed
 * interval — always carries a `poll_interval_ms`). Event/reactor-driven
 * triggers are *not* schedules; they live on computation graphs and surface in
 * the Graphs view. So here we present two meaningful types, **cron** and
 * **poll**, with a plain-language tooltip rather than leaking the raw
 * `schedule_type` string.
 */
export type TriggerKind = {
  /** Short label shown on the badge. */
  label: string;
  /** Mantine color for the badge. */
  color: string;
  /** One-line explanation (tooltip / detail). */
  tip: string;
};

export function describeTriggerKind(scheduleType: string): TriggerKind {
  if (scheduleType === "cron") {
    return {
      label: "cron",
      color: "grape",
      tip: "Fires its workflow on a cron schedule, driven by the cron scheduler.",
    };
  }
  if (scheduleType === "trigger") {
    return {
      label: "poll",
      color: "teal",
      tip: "A custom trigger function polled on a fixed interval; it fires the workflow when its condition is met.",
    };
  }
  // Unknown future type — show it verbatim rather than mislabel.
  return { label: scheduleType, color: "gray", tip: "" };
}

/** Human-friendly poll interval, e.g. 30000 → "30s", 90000 → "1m 30s". */
export function formatPollInterval(ms: number | null | undefined): string {
  if (ms == null) return "—";
  if (ms < 1000) return `${ms}ms`;
  const totalSec = Math.round(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  if (min === 0) return `${sec}s`;
  if (sec === 0) return `${min}m`;
  return `${min}m ${sec}s`;
}
