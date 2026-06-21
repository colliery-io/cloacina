/*
 *  Injector surfaces (CLOACI-T-0768): workflow declared params (InputsCard + run
 *  modal) and graph inject/fire. Run with the demo stack up.
 */
import { test } from "@playwright/test";
import * as fs from "fs";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const OUT = "/tmp/cloacina-injectors";
fs.mkdirSync(OUT, { recursive: true });
const STORAGE_KEY = "cloacina.connection";

async function shot(page, name: string) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {}
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true });
}

test("injector surfaces", async ({ page }) => {
  test.setTimeout(120_000);
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );

  // Python workflow that already declares params (source_id, batch_size).
  await page.goto("/workflows/demo-py-workflow");
  await shot(page, "01-py-workflow-detail");

  // Open the run modal to see the typed param form.
  try {
    await page.getByRole("button", { name: /run workflow/i }).first().click({ timeout: 4000 });
    await page.waitForTimeout(600);
    await shot(page, "02-run-modal");
  } catch {}
});
