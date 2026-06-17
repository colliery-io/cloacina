/* WS-1 screenshot check — execution drill-down task table (CLOACI-I-0124). */
import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";

const OUT = "/tmp/cloacina-ui-uat/ws1";
fs.mkdirSync(OUT, { recursive: true });

// Seeded execution ids from the demo seed harness (passed via env, with fallbacks).
const FAILED = process.env.WS1_FAILED ?? "";
const COMPLETED = process.env.WS1_COMPLETED ?? "";

async function shot(page, name: string) {
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log(`SHOT ${name} :: ${page.url()}`);
}

test("execution drill-down task table", async ({ page }) => {
  test.setTimeout(120_000);
  await seedConnection(page);

  // Land on the executions list first (also a screenshot of the entry point).
  await page.goto("/executions");
  await shot(page, "00-executions-list");

  if (FAILED) {
    await page.goto(`/executions/${FAILED}`);
    await shot(page, "01-failed-detail");
  }
  if (COMPLETED) {
    await page.goto(`/executions/${COMPLETED}`);
    await shot(page, "02-completed-detail");
  }

  // Fallback: click the first execution row if no ids supplied.
  if (!FAILED && !COMPLETED) {
    await page.goto("/executions");
    const row = page.locator("tbody tr").first();
    if (await row.count()) {
      await row.click();
      await shot(page, "03-first-detail");
    }
  }
});
