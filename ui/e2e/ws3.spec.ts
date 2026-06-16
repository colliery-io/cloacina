import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws3";
fs.mkdirSync(OUT, { recursive: true });
test("overview lists", async ({ page }) => {
  test.setTimeout(60_000);
  await seedConnection(page);
  await page.goto("/");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${OUT}/overview.png`, fullPage: true });
  console.log("SHOT overview ::", page.url());
});
