import { test } from "@playwright/test";
import * as fs from "fs";
import { seedConnection } from "./env";
const OUT = "/tmp/cloacina-ui-uat/ws5";
fs.mkdirSync(OUT, { recursive: true });
async function shot(page, name: string) {
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
  console.log("SHOT", name);
}
test("node detail drawer", async ({ page }) => {
  test.setTimeout(90_000);
  await seedConnection(page);
  await page.goto("/graphs/demo_kafka_graph");
  try { await page.waitForLoadState("networkidle", { timeout: 8000 }); } catch {}
  await page.waitForTimeout(1500);
  // click the reactor node
  const reactor = page.locator(".react-flow__node", { hasText: "demo_kafka_rx" }).first();
  if (await reactor.count()) { await reactor.click(); await shot(page, "01-reactor-drawer"); }
  // close + click the compute node "process"
  await page.keyboard.press("Escape");
  await page.waitForTimeout(500);
  const proc = page.locator(".react-flow__node", { hasText: "process" }).first();
  if (await proc.count()) { await proc.click(); await shot(page, "02-node-drawer"); }
});
