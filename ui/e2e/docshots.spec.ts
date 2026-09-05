/*
 *  Doc-site screenshot generator (not a CI test): recaptures the nine
 *  docs/static/images/web-ui/*.png images from the live Leptos UI on the
 *  demo stack (CLOACI-T-0940 — the committed images were React-era). Run:
 *    E2E_BASE_URL=http://localhost:8080 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npx playwright test e2e/docshots.spec.ts --project=chromium
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT, seedConnection } from "./env";

const OUT = "/tmp/docshots";
fs.mkdirSync(OUT, { recursive: true });

async function shot(page, name: string) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 4000 });
  } catch {}
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/${name}.png` });
  // eslint-disable-next-line no-console
  console.log(`SHOT ${name}`);
}

test("regenerate doc-site web-ui screenshots @audit", async ({ page }) => {
  test.setTimeout(300_000);
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedConnection(page);

  // Land, hop to Operations via CLIENT-SIDE nav (same shell → same warm ops
  // WS), wait for the live pill, then hop back — the overview health tiles
  // read the connected feed instead of "connecting…".
  await page.goto("/");
  await page.getByRole("link", { name: "Operations" }).click();
  try {
    await page.getByText("live", { exact: true }).first().waitFor({ timeout: 30_000 });
  } catch {}
  await page.getByRole("link", { name: "Overview" }).click();
  await shot(page, "01-overview");

  await page.goto("/workflows");
  await shot(page, "02-workflows");

  await page.goto("/workflows/demo-cron-rust");
  await page.getByRole("button", { name: "Operational history" }).click();
  await shot(page, "03-workflow-detail");

  await page.goto("/executions");
  await shot(page, "04-executions");

  // A completed run's detail (rows are divs — resolve an id via the API).
  const res = await page.request.get(
    `${SERVER_URL}/v1/tenants/${TENANT}/executions?status=Completed&limit=1`,
    { headers: { Authorization: `Bearer ${API_KEY}` } },
  );
  const execId = ((await res.json()).items ?? [])[0]?.id;
  if (execId) {
    await page.goto(`/executions/${execId}`);
    await shot(page, "05-execution-detail");
  }

  await page.goto("/triggers");
  await shot(page, "06-triggers");

  await page.goto("/graphs");
  await shot(page, "07-graphs");

  await page.goto("/graphs/market_pipeline");
  await shot(page, "08-graph-detail");

  await page.getByRole("button", { name: "＋ inject" }).first().click();
  await page.waitForTimeout(800);
  await shot(page, "09-inject-modal");
});
