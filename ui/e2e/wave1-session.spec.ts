/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { API_KEY, SERVER_URL, TENANT } from "./env";

/**
 * CLOACI-T-0932 (I-0141 Wave 1) — session lifecycle on the Leptos app:
 * multi-tenant connections (T-0779), the tenant switcher, and disconnect.
 * Requires the demo stack seeds (`acme:clk_demo_acme_key_0002:admin`).
 */
const ACME_ADMIN_KEY = process.env.E2E_ACME_ADMIN_KEY ?? "clk_demo_acme_key_0002";
const ACME_TENANT = "acme";

test("add a second tenant, switch between them, disconnect clears the session", async ({
  page,
}) => {
  // 1. Connect to the default tenant.
  await page.goto("/connect");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(API_KEY);
  await page.getByLabel("Tenant").fill(TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();

  // 2. Add the acme tenant via the add-mode connect gate (?add=1 keeps the
  //    gate open while already connected).
  await page.goto("/connect?add=1");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(ACME_ADMIN_KEY);
  await page.getByLabel("Tenant").fill(ACME_TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();

  // The switcher now shows acme (the just-added connection is active).
  const switcher = page.locator("nav").getByText(ACME_TENANT, { exact: true });
  await expect(switcher).toBeVisible();

  // 3. Switch back to the first tenant (no re-validation).
  await switcher.click();
  await page.locator("nav").getByRole("button", { name: TENANT, exact: true }).click();
  await expect(page.locator("nav").getByText(TENANT, { exact: true }).first()).toBeVisible();

  // 4. Disconnect clears ALL connections and lands on the connect gate.
  await page.getByRole("button", { name: "Disconnect ↗" }).click();
  await expect(page).toHaveURL(/connect/);
  const stored = await page.evaluate(() => window.sessionStorage.getItem("cloacina.connections"));
  expect(stored).toBeNull();
});
