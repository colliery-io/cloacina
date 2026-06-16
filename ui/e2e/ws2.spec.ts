import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws2";
fs.mkdirSync(OUT, { recursive: true });
test("operations health view", async ({ page }) => {
  test.setTimeout(60_000);
  await seedConnection(page);
  await page.goto("/operations");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${OUT}/operations.png`, fullPage: true });
  console.log("SHOT operations ::", page.url());
});
