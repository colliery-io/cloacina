import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws8";
fs.mkdirSync(OUT, { recursive: true });
async function shot(page, name: string) {
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log("SHOT", name);
}
test("richer demo graphs render real structure", async ({ page }) => {
  test.setTimeout(90_000);
  await seedConnection(page);

  // Graphs list now has three graphs.
  await page.goto("/graphs");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await shot(page, "01-graphs-list");

  // Fan-in: two accumulators → reactor → combine → evaluate → signal.
  await page.goto("/graphs/market_pipeline");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await shot(page, "02-pipeline-fanin");

  // Routing: decision node fanning out to signal_handler / audit_logger.
  await page.goto("/graphs/market_maker");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await shot(page, "03-routing-branches");
});
