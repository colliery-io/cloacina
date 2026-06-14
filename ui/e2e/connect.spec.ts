/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { API_KEY, SERVER_URL, TENANT } from "./env";

/**
 * Acceptance scenario: connect via the manual API-key path → land on the
 * overview (REQ-001 / UC-0). Tagged @smoke for the PR subset.
 */
test("connect with an API key lands on the overview @smoke", async ({ page }) => {
  await page.goto("/connect");

  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(API_KEY);
  await page.getByLabel("Tenant").fill(TENANT);
  await page.getByRole("button", { name: "Connect" }).click();

  // Lands authenticated on the overview dashboard (no longer on /connect).
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();
  await expect(page).not.toHaveURL(/connect/);
});
