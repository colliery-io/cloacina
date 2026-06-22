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

import { Pill } from "./aurora";
import { TOKEN } from "../util/tokens";

/**
 * Build-status badge (T-0652). Color is keyed off the server's status string;
 * unknown values fall back to muted rather than crash — the set of statuses
 * lives server-side, so we don't hard-assert the enum (REQ-007 "render
 * defensively"). Reused on the overview (T-0655). Aurora token pill.
 */
const COLOR: Record<string, string> = {
  success: TOKEN.ok,
  failed: TOKEN.bad,
  building: TOKEN.ice,
  pending: TOKEN.muted,
};

export function BuildStatusBadge({ status }: { status: string }) {
  return <Pill color={COLOR[status] ?? TOKEN.muted}>{status}</Pill>;
}
