/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { API_KEY, SERVER_URL, TENANT } from "./env";

/**
 * CLOACI-T-0935 (I-0141 Wave 4) — secrets lifecycle from the UI: create a
 * named-field secret (write-only values), rotate it, delete it.
 * Requires the demo stack (`angreal ui up`).
 */
test("secret create → rotate → delete round-trip", async ({ page }) => {
  const name = `e2e_secret_${Date.now()}`;

  await page.goto("/connect");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(API_KEY);
  await page.getByLabel("Tenant").fill(TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();

  await page.goto("/secrets");
  await expect(page.getByRole("heading", { name: "Secrets" })).toBeVisible();

  // Create with one field.
  await page.getByLabel("Name").fill(name);
  await page.getByPlaceholder("password").fill("api_token");
  await page.getByPlaceholder("value (write-only)").fill("hunter2");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.getByText(name)).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("api_token").first()).toBeVisible();

  // Rotate — seeded with the known field name, empty value.
  const row = page.locator("tr", { hasText: name });
  await row.getByRole("button", { name: "Rotate" }).click();
  await page.getByPlaceholder("new value").fill("hunter3");
  await page.getByRole("dialog").getByRole("button", { name: "Rotate", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0, { timeout: 15_000 });

  // Delete.
  await row.getByRole("button", { name: "Delete" }).click();
  await expect(page.locator("tr", { hasText: name })).toHaveCount(0, { timeout: 15_000 });
});
