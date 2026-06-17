/* UX-audit detail walk — captures the detail pages the first walk missed. */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-ui-uat";
fs.mkdirSync(OUT, { recursive: true });
const STORAGE_KEY = "cloacina.connection";

async function shot(page, name: string) {
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1100);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log(`SHOT ${name} :: ${page.url()}`);
}

async function clickFirstRow(page) {
  // table rows navigate via onClick; click the first body row.
  const row = page.locator("tbody tr").first();
  if (await row.count()) { await row.click({ timeout: 4000 }); return true; }
  return false;
}

async function tryNodeClick(page, name: string) {
  for (const sel of [".react-flow__node", "svg g.node", "[data-id]", "svg rect", "svg circle"]) {
    const n = page.locator(sel).first();
    try { if (await n.count()) { await n.click({ timeout: 2500 }); await shot(page, name); return; } } catch {}
  }
}

test("detail walk", async ({ page }) => {
  test.setTimeout(180_000);
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [string, string],
  );

  // Workflow detail (known package).
  await page.goto("/workflows/demo-cron-rust");
  await shot(page, "04-workflow-detail");
  await tryNodeClick(page, "04b-workflow-node-click");

  // Execution detail (a completed run) + try expanding task log.
  await page.goto("/executions/b69cc8a7-04a2-4c9c-8554-d0e2bdeab5ee");
  await shot(page, "06-execution-detail");
  try {
    const btn = page.getByRole("button").filter({ hasText: /log|task|detail|expand|view|output|context/i }).first();
    if (await btn.count()) { await btn.click({ timeout: 3000 }); await shot(page, "06b-execution-expanded"); }
  } catch {}

  // Graph detail via first row.
  await page.goto("/graphs");
  if (await clickFirstRow(page)) { await shot(page, "10-graph-detail"); await tryNodeClick(page, "10b-graph-node-click"); }
  else console.log("graphs: no rows");

  // A second graph (the python graph) if present — navigate by clicking the 2nd row from list.
  await page.goto("/graphs");
  try {
    const rows = page.locator("tbody tr");
    if (await rows.count() > 1) { await rows.nth(1).click({ timeout: 4000 }); await shot(page, "10c-graph-detail-2"); await tryNodeClick(page, "10d-graph-node-click-2"); }
  } catch {}

  // Trigger detail via first row.
  await page.goto("/triggers");
  if (await clickFirstRow(page)) await shot(page, "08-trigger-detail");
  else console.log("triggers: no rows");
});
