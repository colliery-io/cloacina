/* Capture the typed inject modal on a graph with typed boundary schemas. */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-injectmodal";
fs.mkdirSync(OUT, { recursive: true });
const STORAGE_KEY = "cloacina.connection";

test("typed inject modal", async ({ page }) => {
  test.setTimeout(60_000);
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );
  await page.goto("/graphs/market_pipeline");
  await page.waitForTimeout(1200);
  // Click the first "inject ▸" button (orderbook row).
  await page.getByText("inject", { exact: false }).first().click({ timeout: 5000 });
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${OUT}/01-inject-modal.png` });

  // Also capture the Fire menu.
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);
  await page.getByRole("button", { name: /fire/i }).first().click({ timeout: 5000 });
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${OUT}/02-fire-menu.png` });
});
