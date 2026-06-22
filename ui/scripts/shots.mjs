/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Aurora Dark self-review screenshots (CLOACI-I-0129). Renders the real SPA
 *  (served by `vite preview`) against a fully MOCKED API — no backend needed —
 *  so each redesigned screen can be eyeballed against ui/design/aurora-dark/
 *  screenshots/. Run: `node scripts/shots.mjs` with a preview server on :4173.
 */
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.SHOTS_BASE ?? "http://localhost:4173";
const OUT = process.env.SHOTS_OUT ?? "/tmp/aurora-shots";
const SERVER = "http://localhost:8080";
const TENANT = "public";
mkdirSync(OUT, { recursive: true });

const list = (items) => ({ items, total: items.length });

const tasksFor = (statuses) =>
  statuses.map((status, i) => ({
    task_name: ["extract", "validate", "transform", "report"][i] ?? `task_${i}`,
    status,
    started_at: "2026-06-21T01:00:0" + i + "Z",
    completed_at: "2026-06-21T01:00:1" + i + "Z",
    attempt: 1,
  }));

const executions = list([
  { id: "f47ac10b-58cc-4372-a567-0e02b2c3d479", workflow_name: "analytics_workflow", status: "running", trigger: "manual", started_at: "2026-06-21T01:00:00Z", completed_at: null },
  { id: "a1b2c3d4-58cc-4372-a567-0e02b2c3d480", workflow_name: "demo_slow_workflow", status: "completed", trigger: "cron", started_at: "2026-06-21T00:55:00Z", completed_at: "2026-06-21T00:57:30Z" },
  { id: "b2c3d4e5-58cc-4372-a567-0e02b2c3d481", workflow_name: "demo_fail_workflow", status: "failed", trigger: "manual", started_at: "2026-06-21T00:50:00Z", completed_at: "2026-06-21T00:50:40Z" },
  { id: "c3d4e5f6-58cc-4372-a567-0e02b2c3d482", workflow_name: "demo_pipeline", status: "scheduled", trigger: "cron", started_at: "2026-06-21T01:05:00Z", completed_at: null },
]);

const workflows = list([
  { package_name: "packaged-workflows", workflow_name: "analytics_workflow", version: "1.0.0", description: "Analytics and data processing pipeline", author: "Analytics Team", updated_at: "2026-06-20T12:00:00Z", build_status: "success", paused: false, tasks: ["extract_data", "validate_data", "transform_data", "generate_reports"], task_graph: [], declared_params: [{ name: "source_id", schema: { type: "string" }, required: true }, { name: "batch_size", schema: { type: "integer" }, required: false, default: 500 }] },
  { package_name: "demo-slow-rust", workflow_name: "demo_slow_workflow", version: "0.1.0", description: "Deliberately slow multi-task run", author: "demo", updated_at: "2026-06-20T11:00:00Z", build_status: "success", paused: false, tasks: ["step_a", "step_b", "step_c"], task_graph: [], declared_params: [] },
]);

const graphs = list([
  { name: "market_maker", status: "live", health: { state: "running" }, accumulators: ["orderbook", "pricing"], reaction_mode: "when_any", input_strategy: "latest", paused: false },
  { name: "mixed_graph", status: "warming", health: { state: "warming" }, accumulators: ["alpha"], reaction_mode: "when_any", input_strategy: "latest", paused: false },
]);

const accumulators = list([
  { name: "orderbook", status: "socket_only", reactor: "market_maker", tenant_id: "public" },
  { name: "pricing", status: "socket_only", reactor: "market_maker", tenant_id: "public" },
  { name: "alpha", status: "live", reactor: "mixed_graph", tenant_id: "public" },
]);

const reactors = list([
  { name: "market_maker", health: { state: "running" }, accumulators: ["orderbook", "pricing"], reaction_mode: "when_any", input_strategy: "latest", bound_graphs: ["market_maker"], paused: false, fires: 128, last_fired_at: "2026-06-21T01:00:00Z" },
]);

const triggers = list([
  { workflow_name: "demo_slow_workflow", type: "cron", schedule: "0 */5 * * * *", enabled: true, next_run: "2026-06-21T01:05:00Z", last_run: "2026-06-21T01:00:00Z" },
  { workflow_name: "analytics_workflow", type: "poll", schedule: "every 30s", enabled: false, next_run: null, last_run: "2026-06-21T00:30:00Z" },
]);

const keys = list([
  { id: "k1", name: "ci-deploy", prefix: "clk_abc", role: "write", created_at: "2026-06-01T00:00:00Z", last_used_at: "2026-06-21T00:00:00Z" },
]);

function json(route, body) {
  return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
}

function mock(route) {
  const url = route.request().url();
  if (!url.includes("localhost:8080")) return route.continue();
  const p = new URL(url).pathname;
  if (p.endsWith("/health") || p.endsWith("/ready")) return json(route, { status: "ok" });
  if (p.includes("ws-ticket")) return json(route, { ticket: "t" });
  if (p.includes("/executions/") && p.endsWith("/tasks"))
    return json(route, list(tasksFor(["completed", "completed", "running", "pending"])));
  if (p.includes("/executions/")) return json(route, executions.items[0]);
  if (p.includes("/executions")) return json(route, executions);
  if (p.includes("/workflows")) return json(route, workflows);
  if (p.includes("/triggers")) return json(route, triggers);
  if (p.includes("/graphs")) return json(route, graphs);
  if (p.includes("/accumulators")) return json(route, accumulators);
  if (p.includes("/reactors")) return json(route, reactors);
  if (p.includes("/keys")) return json(route, keys);
  return json(route, list([]));
}

const browser = await chromium.launch();

// Connect gate — fresh context, no seeded connection.
{
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  await page.route("**/*", mock);
  await page.goto(`${BASE}/connect`, { waitUntil: "domcontentloaded" }).catch(() => {});
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${OUT}/connect.png`, fullPage: true });
  console.log("shot connect");
  await ctx.close();
}

// Authenticated screens — seeded connection.
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
await ctx.addInitScript(
  ([key, value]) => window.sessionStorage.setItem(key, value),
  ["cloacina.connection", JSON.stringify({ serverUrl: SERVER, apiKey: "clk_demo", tenant: TENANT })],
);
const page = await ctx.newPage();
await page.route("**/*", mock);

const screens = [
  ["overview", "/"],
  ["executions", "/executions"],
  ["workflows", "/workflows"],
  ["triggers", "/triggers"],
  ["graphs", "/graphs"],
  ["operations", "/operations"],
  ["keys", "/keys"],
  ["settings", "/settings"],
];
for (const [name, path] of screens) {
  await page.goto(`${BASE}${path}`, { waitUntil: "domcontentloaded" }).catch(() => {});
  await page.waitForTimeout(900);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log("shot", name);
}

await browser.close();
console.log("done →", OUT);
