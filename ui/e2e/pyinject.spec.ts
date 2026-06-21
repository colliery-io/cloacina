import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";
const OUT = "/tmp/cloacina-pyinject"; fs.mkdirSync(OUT, { recursive: true });
test("python inject modal", async ({ page }) => {
  test.setTimeout(60000);
  await page.addInitScript(([k, v]) => window.sessionStorage.setItem(k, v),
    ["cloacina.connection", JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [string, string]);
  await page.goto("/graphs/demo_py_graph");
  await page.waitForTimeout(1200);
  await page.getByText("inject", { exact: false }).first().click({ timeout: 5000 });
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${OUT}/py-inject.png` });
});
