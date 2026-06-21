/*
 *  Screenshot the GraphDetail operational view (CLOACI-T-0767) on the live demo.
 *  Not a CI test. Run with:
 *    E2E_BASE_URL=http://localhost:8082 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npx playwright test graphops.spec.ts --project=chromium
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-graphops";
fs.mkdirSync(OUT, { recursive: true });

const STORAGE_KEY = "cloacina.connection";

async function shot(page, name: string) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {}
  await page.waitForTimeout(1200); // let charts + polled fires settle
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  // eslint-disable-next-line no-console
  console.log(`SHOT ${name} :: url=${page.url()}`);
}

test("graph operational view", async ({ page }) => {
  test.setTimeout(120_000);

  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );

  // High-rate single-source graph — exercises the fire heatmap + recent fires.
  await page.goto("/graphs/demo_kafka_graph");
  await shot(page, "01-demo_kafka_graph");

  // Two-source market graph — exercises readiness + accumulator table rows.
  await page.goto("/graphs/market_pipeline");
  await shot(page, "02-market_pipeline");

  // Python state graph.
  await page.goto("/graphs/demo_py_state");
  await shot(page, "03-demo_py_state");
});
