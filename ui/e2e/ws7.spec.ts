import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws7";
fs.mkdirSync(OUT, { recursive: true });
async function shot(page, name: string) {
  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log("SHOT", name);
}
test("naive-user polish", async ({ page }) => {
  test.setTimeout(90_000);
  await seedConnection(page);

  // Workflows: CG package shows a "graph" badge, not "Tasks: 0".
  await page.goto("/workflows");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await shot(page, "01-workflows-graph-badge");

  // Graphs: accumulator status shows a friendly label, not quoted "socket_only".
  await page.goto("/graphs");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await shot(page, "02-graphs-status");

  // Graph detail: reaction_mode / input_strategy as friendly labels.
  await page.goto("/graphs/demo_kafka_graph");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1200);
  await shot(page, "03-graph-detail-vocab");

  // Settings: neutral "coming soon", no leaked task code.
  await page.goto("/settings");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await shot(page, "04-settings");
});
