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
 * Runtime config (CLOACI-I-0117 / OQ-5). The deploy container rewrites
 * `window.__CLOACINA_CONFIG__` at startup so one image works against any
 * server; the bundle itself is server-agnostic. The full mechanism is
 * finalized in T-0659 — this is just the read side.
 */
export const runtimeConfig = {
  /** Prefill for the /connect form; empty means "ask the user". */
  defaultServerUrl: window.__CLOACINA_CONFIG__?.defaultServerUrl ?? "",
};
