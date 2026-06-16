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
// Dev convenience (CLOACI-I-0124): when served by `vite dev` (i.e. the local
// `angreal ui up` demo stack), prefill — and auto-connect — the localhost demo
// credentials so the connect gate doesn't have to be filled in by hand every
// session. `import.meta.env.DEV` is false in any production build, so these
// defaults never ship. The key matches the dev `--bootstrap-key` the harness
// passes to the server (.angreal/task_ui.py `DEV_BOOTSTRAP_KEY`).
const DEV = import.meta.env.DEV;

export const runtimeConfig = {
  /** Prefill for the /connect form; empty means "ask the user". The deploy
   *  container may inject an empty string, so fall through (||, not ??) to the
   *  dev default when it's blank. */
  defaultServerUrl:
    (window.__CLOACINA_CONFIG__?.defaultServerUrl || "") ||
    (DEV ? "http://localhost:8080" : ""),
  /** Dev-only demo API key prefill (empty in production). */
  demoApiKey: DEV ? "clk_dev_ui_bootstrap_key_0001" : "",
  /** Dev-only demo tenant prefill. */
  demoTenant: DEV ? "public" : "",
  /** Whether to auto-submit the prefilled demo connection on the connect gate. */
  demoAutoConnect: DEV,
};
