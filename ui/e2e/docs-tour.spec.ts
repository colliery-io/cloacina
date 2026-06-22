/*
 *  Capture the web-UI tour screenshots for the docs (service/tutorials/
 *  02-the-web-ui.md). Fixed 1440-wide viewport for consistency. Run with the
 *  demo stack up:
 *    E2E_BASE_URL=http://localhost:8082 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npx playwright test docs-tour.spec.ts --project=chromium
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-docs-tour";
fs.mkdirSync(OUT, { recursive: true });
const STORAGE_KEY = "cloacina.connection";

async function shot(page, name: string, full = false) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {}
  await page.waitForTimeout(1100);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: full });
}

test("web UI tour", async ({ page, request }) => {
  test.setTimeout(180_000);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );

  // Find an execution that has a Skipped task (demo_py_cron / demo_cron).
  let execId = "";
  try {
    const res = await request.get(`${SERVER_URL}/v1/tenants/${TENANT}/executions?limit=20`, {
      headers: { Authorization: `Bearer ${API_KEY}` },
    });
    const items = (await res.json()).items ?? [];
    const cand = items.find((e: { workflow_name?: string; status?: string }) =>
      (e.workflow_name ?? "").includes("cron") && (e.status ?? "").toLowerCase() === "completed",
    ) ?? items[0];
    execId = cand?.id ?? "";
  } catch {}

  await page.goto("/");
  await shot(page, "01-overview", true);

  await page.goto("/workflows");
  await shot(page, "02-workflows");

  await page.goto("/workflows/demo-py-workflow");
  await shot(page, "03-workflow-detail", true);

  await page.goto("/executions");
  await shot(page, "04-executions");

  if (execId) {
    await page.goto(`/executions/${execId}`);
    await shot(page, "05-execution-detail", true);
  }

  await page.goto("/triggers");
  await shot(page, "06-triggers");

  await page.goto("/graphs");
  await shot(page, "07-graphs");

  await page.goto("/graphs/market_pipeline");
  await shot(page, "08-graph-detail", true);

  // Typed inject modal.
  try {
    await page.getByText("inject", { exact: false }).first().click({ timeout: 5000 });
    await page.waitForTimeout(700);
    await shot(page, "09-inject-modal");
  } catch {}
});
