#!/usr/bin/env node
/*
 *  Copyright 2026 Colliery Software
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

// Cloacina UI seed/demo workload harness (CLOACI-I-0117 / T-0660).
//
// Drives a target cloacina-server through the SAME SDK the UI ships on
// (@cloacina/client): ensures a tenant, uploads the demo `.cloacina`
// packages, and runs executions in one of two modes:
//
//   seed  — produces a deterministic state for automated UAT (T-0661):
//           one COMPLETED run, one FAILED run, one IN-FLIGHT run, then exits.
//   loop  — fires executions forever on an interval, mixing fast/slow/failing
//           runs so the UI always has live activity to watch (the demo).
//
// All config is via env (see README). Notably:
//   - executions are keyed by WORKFLOW NAME, not package name. The server's
//     execute route resolves against the scheduler registry (workflow name,
//     e.g. `demo_slow_workflow`), while list/detail use the package name
//     (`demo-slow-rust`) — a known platform naming-drift gap. The harness
//     therefore executes by the workflow names below.

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

import { CloacinaClient } from "@cloacina/client";

const env = process.env;
const cfg = {
  serverUrl: env.HARNESS_SERVER_URL ?? "http://localhost:8080",
  apiKey: env.HARNESS_API_KEY ?? "",
  tenant: env.HARNESS_TENANT ?? "public",
  packageDir: env.HARNESS_PACKAGE_DIR ?? "./packages",
  mode: (env.HARNESS_MODE ?? "seed").toLowerCase(),
  intervalMs: intEnv("HARNESS_INTERVAL_MS", 8000),
  stepSeconds: intEnv("HARNESS_STEP_SECONDS", 4),
  slowWorkflow: env.HARNESS_SLOW_WORKFLOW ?? "demo_slow_workflow",
  failWorkflow: env.HARNESS_FAIL_WORKFLOW ?? "demo_fail_workflow",
  healthTimeoutMs: intEnv("HARNESS_HEALTH_TIMEOUT_MS", 60000),
  runTimeoutMs: intEnv("HARNESS_RUN_TIMEOUT_MS", 60000),
  registerTimeoutMs: intEnv("HARNESS_REGISTER_TIMEOUT_MS", 120000),
};

function intEnv(name, dflt) {
  const v = env[name];
  if (v === undefined || v === "") return dflt;
  const n = Number.parseInt(v, 10);
  return Number.isFinite(n) ? n : dflt;
}

const log = (...args) => console.log(`[seed-harness]`, ...args);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function makeClient() {
  if (!cfg.apiKey) throw new Error("HARNESS_API_KEY is required");
  return new CloacinaClient({
    baseUrl: cfg.serverUrl,
    apiKey: cfg.apiKey,
    tenant: cfg.tenant,
  });
}

async function waitForHealth(client) {
  const deadline = Date.now() + cfg.healthTimeoutMs;
  for (;;) {
    try {
      await client.health();
      return;
    } catch (err) {
      if (Date.now() > deadline) {
        throw new Error(`server not healthy after ${cfg.healthTimeoutMs}ms: ${err}`);
      }
      await sleep(1000);
    }
  }
}

/** Best-effort tenant creation. `public` already exists; others we create. */
async function ensureTenant(client) {
  if (cfg.tenant === "public") {
    log(`tenant: public (server default)`);
    return;
  }
  try {
    await client.createTenant({ name: cfg.tenant });
    log(`tenant: created '${cfg.tenant}'`);
  } catch (err) {
    // Duck-type on .status rather than `instanceof` — the SDK error class can
    // differ by realm across the package symlink.
    const status = err && typeof err.status === "number" ? err.status : 0;
    if (status === 409 || status === 400) {
      log(`tenant: '${cfg.tenant}' already exists`);
    } else {
      throw err;
    }
  }
}

/** Upload every `.cloacina` package in the package dir. Idempotent-ish: an
 *  already-registered package is logged and skipped, not fatal. */
async function uploadPackages(client) {
  let entries;
  try {
    entries = (await readdir(cfg.packageDir)).filter((f) => f.endsWith(".cloacina"));
  } catch (err) {
    throw new Error(`cannot read package dir '${cfg.packageDir}': ${err}`);
  }
  if (entries.length === 0) {
    log(`WARNING: no .cloacina packages found in ${cfg.packageDir} — nothing to upload`);
    return;
  }
  for (const file of entries) {
    const bytes = new Uint8Array(await readFile(path.join(cfg.packageDir, file)));
    try {
      const res = await client.uploadWorkflow(bytes);
      log(`uploaded ${file} → ${res.package_id ?? "(registered)"}`);
    } catch (err) {
      if (err && typeof err.status === "number") {
        log(`upload ${file}: ${err.status} ${err.code} (${err.message}) — continuing`);
      } else {
        throw err;
      }
    }
  }
}

async function execute(client, workflow, context) {
  const body = context === undefined ? {} : { context };
  const res = await client.executeWorkflow(workflow, body);
  return res.execution_id;
}

/** Execute, retrying while the workflow isn't in the runtime registry yet.
 *  Packages build asynchronously after upload and the reconciler loads them
 *  on a periodic tick, so the first executes can 400 with "not found in
 *  registry" until that lands (same race the e2e harness polls through). */
async function executeReady(client, workflow, context) {
  const deadline = Date.now() + cfg.registerTimeoutMs;
  for (;;) {
    try {
      return await execute(client, workflow, context);
    } catch (err) {
      // The package builds asynchronously and the reconciler loads it on a
      // periodic tick, so early executes 400 with "not found in registry".
      // Match on the message text (robust across module boundaries) rather
      // than `instanceof`.
      const msg = err && err.message ? String(err.message) : String(err);
      const notReady = /not found in registry/i.test(msg);
      if (!notReady || Date.now() > deadline) throw err;
      await sleep(2000);
    }
  }
}

/** Poll an execution until it reaches a terminal status or times out. */
async function waitForTerminal(client, execId) {
  const terminal = new Set(["completed", "failed", "cancelled", "canceled"]);
  const deadline = Date.now() + cfg.runTimeoutMs;
  for (;;) {
    const detail = await client.getExecution(execId);
    const status = String(detail.status ?? "").toLowerCase();
    if (terminal.has(status)) return status;
    if (Date.now() > deadline) return `${status} (timed out waiting for terminal)`;
    await sleep(1000);
  }
}

async function seed(client) {
  log("seed mode: building a deterministic completed / failed / in-flight state");

  // 1) COMPLETED — slow workflow with no per-step pause finishes promptly.
  //    executeReady absorbs the post-upload build/registration race.
  const doneId = await executeReady(client, cfg.slowWorkflow, { step_seconds: 0 });
  log(`completed-run: ${doneId} (waiting for terminal…)`);
  const doneStatus = await waitForTerminal(client, doneId);
  log(`completed-run: ${doneId} → ${doneStatus}`);

  // 2) FAILED — the fail fixture errors deterministically.
  const failId = await executeReady(client, cfg.failWorkflow);
  log(`failed-run: ${failId} (waiting for terminal…)`);
  const failStatus = await waitForTerminal(client, failId);
  log(`failed-run: ${failId} → ${failStatus}`);

  // 3) IN-FLIGHT — slow workflow with per-step pause is still running on exit.
  const liveId = await executeReady(client, cfg.slowWorkflow, { step_seconds: cfg.stepSeconds });
  log(`in-flight-run: ${liveId} (left running, ~${cfg.stepSeconds * 5}s total)`);

  log("seed complete:");
  log(`  completed = ${doneId} (${doneStatus})`);
  log(`  failed    = ${failId} (${failStatus})`);
  log(`  in-flight = ${liveId} (running)`);
}

async function loop(client) {
  log(`loop mode: firing every ${cfg.intervalMs}ms (Ctrl-C to stop)`);
  let stop = false;
  process.on("SIGINT", () => {
    log("SIGINT — stopping after the current tick");
    stop = true;
  });
  process.on("SIGTERM", () => {
    stop = true;
  });

  let n = 0;
  while (!stop) {
    n += 1;
    try {
      if (n % 4 === 0) {
        const id = await executeReady(client, cfg.failWorkflow);
        log(`tick ${n}: failing run ${id}`);
      } else {
        // Alternate quick and watchable-slow runs so the dashboard shows a
        // mix of in-flight + completing executions.
        const secs = n % 2 === 0 ? cfg.stepSeconds : 1;
        const id = await executeReady(client, cfg.slowWorkflow, { step_seconds: secs });
        log(`tick ${n}: slow run ${id} (step_seconds=${secs})`);
      }
    } catch (err) {
      log(`tick ${n}: execute failed — ${err instanceof Error ? err.message : err}`);
    }
    await sleep(cfg.intervalMs);
  }
  log("loop stopped");
}

async function main() {
  log(`server=${cfg.serverUrl} tenant=${cfg.tenant} mode=${cfg.mode} packages=${cfg.packageDir}`);
  const client = makeClient();
  await waitForHealth(client);
  await ensureTenant(client);
  await uploadPackages(client);

  if (cfg.mode === "loop") {
    await loop(client);
  } else if (cfg.mode === "seed") {
    await seed(client);
  } else {
    throw new Error(`unknown HARNESS_MODE '${cfg.mode}' (expected seed|loop)`);
  }
}

main().catch((err) => {
  console.error(`[seed-harness] fatal: ${err instanceof Error ? err.stack : err}`);
  process.exit(1);
});
