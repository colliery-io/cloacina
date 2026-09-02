/*
 *  UAT round-2 screenshot walk (not a CI test): captures the T-0938 surfaces
 *  for maintainer re-review. Run with:
 *    E2E_BASE_URL=http://localhost:8082 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npx playwright test uat2.spec.ts --project=chromium
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-uat2";
fs.mkdirSync(OUT, { recursive: true });

const STORAGE_KEY = "cloacina.connection";

async function shot(page, name: string) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {}
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  // eslint-disable-next-line no-console
  console.log(`SHOT ${name} :: url=${page.url()}`);
}

test("UAT round 2 walk @audit", async ({ page }) => {
  test.setTimeout(240_000);

  // Demo-stack walk only: it names compose-demo objects (market_pipeline,
  // demo-cron-rust) that the seeded ui-e2e lane doesn't create.
  const res = await page.request.get(`${SERVER_URL}/v1/health/graphs`, {
    headers: { Authorization: `Bearer ${API_KEY}` },
  });
  const names = ((await res.json()).items ?? []).map((g: { name: string }) => g.name);
  test.skip(!names.includes("market_pipeline"), "demo-stack fixtures not present");

  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );

  // 1) Graphs: operational dashboard strip above the sections.
  await page.goto("/graphs");
  await shot(page, "01-graphs-dashboard");

  // 2) Graph detail: Live view, then Operational history.
  await page.goto("/graphs/market_pipeline");
  await shot(page, "02-graph-detail-live");
  await page.getByRole("button", { name: "Operational history" }).click();
  await shot(page, "03-graph-detail-history");

  // 3) Triggers: cron vs polling sections, headed Fire/Run columns.
  await page.goto("/triggers");
  await shot(page, "04-triggers-sections");

  // 4) Workflows: headed table with Pause/Run columns.
  await page.goto("/workflows");
  await shot(page, "05-workflows-table");

  // 5) Workflow detail (one with run history): defaults to Current execution;
  // history behind the tab.
  await page.goto("/workflows/demo-cron-rust");
  await shot(page, "06-workflow-detail-default-current");
  await page.getByRole("button", { name: "Operational history" }).click();
  await shot(page, "07-workflow-detail-history");
});
