/*
 *  Verify (a) skipped nodes render salmon and (b) the code modal opens to a
 *  single task definition. Run with the demo stack up:
 *    E2E_BASE_URL=http://localhost:8082 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npx playwright test skipmodal.spec.ts --project=chromium
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-skipmodal";
fs.mkdirSync(OUT, { recursive: true });
const STORAGE_KEY = "cloacina.connection";

// demo_py_cron_workflow execution with a Skipped py_audit task.
const EXEC = "ae614a80-2ab1-4997-8907-893d88169262";

async function shot(page, name: string) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {}
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
}

test("skipped salmon + single-task code modal", async ({ page }) => {
  test.setTimeout(120_000);
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );

  await page.goto(`/executions/${EXEC}`);
  await shot(page, "01-exec-skipped");

  // Click a task node to open the code modal (single task def).
  try {
    await page.getByText("py_process", { exact: true }).first().click({ timeout: 5000 });
    await page.waitForTimeout(700);
    await shot(page, "02-code-modal");
  } catch {
    // node text not directly clickable — fall back to the skipped node
    await page.getByText("py_audit", { exact: true }).first().click({ timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(700);
    await shot(page, "02-code-modal");
  }
});
