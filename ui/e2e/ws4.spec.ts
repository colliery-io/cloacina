import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws4";
fs.mkdirSync(OUT, { recursive: true });
async function shot(page, name: string) {
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log("SHOT", name, "::", page.url());
}
test("graph detail with reactor/accumulator nodes", async ({ page }) => {
  test.setTimeout(90_000);
  await seedConnection(page);
  await page.goto("/graphs");
  await shot(page, "00-graphs-list");
  const row = page.locator("tbody tr").first();
  if (await row.count()) { await row.click(); await shot(page, "01-graph-detail"); }
  else console.log("no graph rows");
});
