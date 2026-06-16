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
 * Plain-language vocabulary for the internal enum tokens the API returns
 * (CLOACI-I-0124 / WS-7). The audit found these leaking as raw quoted strings
 * ("socket_only", "WHEN_ANY", …) that mean nothing to a first-time operator.
 * `explainToken` turns a token into a friendly label (underscores → spaces,
 * lower-cased) plus a one-line tooltip, so the UI can show what each state
 * actually means instead of the raw enum.
 */
export type VocabEntry = { label: string; tip: string };

const VOCAB: Record<string, VocabEntry> = {
  // --- Graph / accumulator health states ---
  live: { label: "live", tip: "Connected and receiving data normally." },
  warming: {
    label: "warming",
    tip: "Starting up — connecting to its source and backfilling before it goes live.",
  },
  socket_only: {
    label: "socket only",
    tip: "The data connection is open but no boundary data has arrived yet.",
  },
  connecting: {
    label: "connecting",
    tip: "Establishing the connection to the upstream source.",
  },
  running: { label: "running", tip: "Active and processing." },
  stopped: { label: "stopped", tip: "Not running." },
  unknown: { label: "unknown", tip: "The server reported no health state for this item." },

  // --- Reactor firing criteria (reaction_mode) ---
  when_any: {
    label: "when any",
    tip: "Fires the graph as soon as ANY one of its bound accumulators receives new data.",
  },
  when_all: {
    label: "when all",
    tip: "Fires the graph only once ALL of its bound accumulators have new data.",
  },

  // --- Reactor input strategy (input_strategy) ---
  latest: {
    label: "latest",
    tip: "Passes only the most recent value from each accumulator into the graph.",
  },
  sequential: {
    label: "sequential",
    tip: "Processes accumulator values one at a time, in arrival order.",
  },
};

/** Friendly label + tooltip for an API enum token. Unknown tokens degrade to
 *  the token with underscores turned into spaces and no tooltip. */
export function explainToken(token: string | null | undefined): VocabEntry {
  if (!token) return { label: "—", tip: "" };
  const entry = VOCAB[token.toLowerCase()];
  if (entry) return entry;
  return { label: token.replace(/_/g, " "), tip: "" };
}
