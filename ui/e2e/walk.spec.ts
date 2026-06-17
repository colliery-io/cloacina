/*
 *  UX-audit walk (not a CI test): drives every UI surface on the seeded demo
 *  stack and screenshots it for analysis. Run with:
 *    E2E_BASE_URL=http://localhost:8082 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npx playwright test walk.spec.ts --project=chromium
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-ui-uat";
fs.mkdirSync(OUT, { recursive: true });

const STORAGE_KEY = "cloacina.connection";

async function shot(page, name: string) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {}
  await page.waitForTimeout(900); // let charts/graphs settle
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  // eslint-disable-next-line no-console
  console.log(`SHOT ${name} :: url=${page.url()}`);
}

test("walk the UI and screenshot everything", async ({ page }) => {
  test.setTimeout(240_000);

  // 1) Connect page, unauthenticated.
  await page.goto("/connect");
  await shot(page, "01-connect");

  // Pre-seed auth so the rest of the walk is authenticated.
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );

  // 2) Overview.
  await page.goto("/");
  await shot(page, "02-overview");

  // Helper: visit a list route, screenshot, then dive into the first detail row.
  async function listAndDetail(route: string, hrefPrefix: string, listName: string, detailName: string) {
    await page.goto(route);
    await shot(page, listName);
    try {
      const link = page.locator(`a[href^="${hrefPrefix}"]`).first();
      if (await link.count()) {
        const href = await link.getAttribute("href");
        await link.click();
        await shot(page, detailName);
        return href;
      } else {
        console.log(`NO detail link under ${hrefPrefix}`);
      }
    } catch (e) {
      console.log(`detail dive failed for ${route}: ${e}`);
    }
    return null;
  }

  // 3-4) Workflows + workflow detail (graph render).
  await listAndDetail("/workflows", "/workflows/", "03-workflows", "04-workflow-detail");
  // try clicking a graph node + any "view code" affordance on the workflow detail
  try {
    const node = page.locator(".react-flow__node, svg .node, [data-id]").first();
    if (await node.count()) {
      await node.click({ timeout: 3000 });
      await shot(page, "04b-workflow-node-click");
    }
  } catch {}

  // 5-6) Executions + execution detail (task log).
  await listAndDetail("/executions", "/executions/", "05-executions", "06-execution-detail");
  try {
    // expand anything expandable in the task log
    const expanders = page.getByRole("button").filter({ hasText: /log|detail|expand|view|task/i });
    if (await expanders.count()) {
      await expanders.first().click({ timeout: 3000 });
      await shot(page, "06b-execution-log-expanded");
    }
  } catch {}

  // 7-8) Triggers + trigger detail.
  await listAndDetail("/triggers", "/triggers/", "07-triggers", "08-trigger-detail");

  // 9-10) Graphs + graph detail (accumulators/reactors/nodes).
  await listAndDetail("/graphs", "/graphs/", "09-graphs", "10-graph-detail");
  try {
    const node = page.locator(".react-flow__node, svg .node, [data-id]").first();
    if (await node.count()) {
      await node.click({ timeout: 3000 });
      await shot(page, "10b-graph-node-click");
    }
  } catch {}

  // 11) Keys.
  await page.goto("/keys");
  await shot(page, "11-keys");

  // 12) Settings (placeholder).
  await page.goto("/settings");
  await shot(page, "12-settings");

  // 13) Workflow upload form.
  await page.goto("/workflows/upload");
  await shot(page, "13-workflow-upload");
});
