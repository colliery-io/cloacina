import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws6";
fs.mkdirSync(OUT, { recursive: true });
async function shot(page, name: string) {
  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log("SHOT", name);
}
test("non-cron triggers in the Triggers view", async ({ page }) => {
  test.setTimeout(90_000);
  await seedConnection(page);

  // List: cron + poll triggers both visible, with meaningful Type column.
  await page.goto("/triggers");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1200);
  await shot(page, "01-triggers-list");

  // Detail: the poll trigger — kind, poll interval, fires-workflow, Run now.
  const pollRow = page.locator("tr", { hasText: "demo_poll_workflow" }).first();
  if (await pollRow.count()) {
    await pollRow.click();
  } else {
    await page.goto("/triggers/demo_poll_trigger");
  }
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1000);
  await shot(page, "02-poll-trigger-detail");
});
