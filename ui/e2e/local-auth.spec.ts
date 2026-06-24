/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { SERVER_URL } from "./env";

/**
 * CLOACI-T-0799 — self-managed login, no IdP. Exercises the full local-accounts
 * flow end-to-end: an acme tenant-admin (seeded admin key) creates a local
 * account in the UI, then that account signs in with username/password and
 * lands authenticated — all without any external identity provider.
 *
 * Requires the demo stack (`angreal ui up`) or `angreal test ui-e2e`, which
 * seeds `acme:clk_demo_acme_key_0002:admin`.
 */
const ACME_ADMIN_KEY = process.env.E2E_ACME_ADMIN_KEY ?? "clk_demo_acme_key_0002";
const ACME_TENANT = "acme";

test("self-manage: tenant-admin creates a local account, then that user signs in @smoke", async ({
  page,
}) => {
  const username = `e2e-user-${Date.now()}`;
  const password = "e2e-password-123";

  // 1. Connect as the acme tenant-admin via the API-key path (default mode).
  await page.goto("/connect");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(ACME_ADMIN_KEY);
  await page.getByLabel("Tenant").fill(ACME_TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();

  // 2. Create a local account for the tenant.
  await page.goto("/accounts");
  await expect(page.getByRole("heading", { name: "Local accounts" })).toBeVisible();
  await page.getByLabel("Username").fill(username);
  await page.getByLabel("Initial password").fill(password);
  await page.getByRole("button", { name: "Create" }).click();
  // The new account appears in the list (active).
  await expect(page.getByText(username)).toBeVisible();

  // 3. Drop the admin session and sign in as the new local user.
  await page.evaluate(() => window.sessionStorage.clear());
  await page.goto("/connect");
  await page.getByText("Username & password").click();
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("Username").fill(username);
  await page.getByLabel("Password").fill(password);
  await page.getByLabel("Tenant").fill(ACME_TENANT);
  await page.getByRole("button", { name: "Sign in" }).click();

  // Minted a key, connected, landed on the overview — no IdP involved.
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();
  await expect(page).not.toHaveURL(/connect/);
});

/** A wrong password is rejected (no account enumeration — same opaque error). */
test("self-manage: wrong password is rejected", async ({ page }) => {
  await page.goto("/connect");
  await page.getByText("Username & password").click();
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("Username").fill("definitely-not-a-user");
  await page.getByLabel("Password").fill("nope");
  await page.getByLabel("Tenant").fill(ACME_TENANT);
  await page.getByRole("button", { name: "Sign in" }).click();

  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page).toHaveURL(/connect/);
});
