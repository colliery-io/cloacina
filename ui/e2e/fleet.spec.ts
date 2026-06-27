/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { SERVER_URL } from "./env";

/**
 * CLOACI-T-0813 — tenant agent-fleet management in the UI. A seeded acme
 * tenant-admin connects, opens the Agent fleet view, reads the desired/running/
 * limit stats, and scales the fleet up (+1) then back down (−1) through the
 * tenant-scoped `/v1/tenants/{t}/fleet` provision/deprovision surface
 * (T-0809). Role-gating, the 409-at-capacity path, and the −1 floor are
 * enforced server-side; the UI only reflects them.
 *
 * Stack-gated: requires the demo stack (`angreal ui up`) or
 * `angreal test ui-e2e`, which seeds `acme:clk_demo_acme_key_0002:admin`.
 */
const ACME_ADMIN_KEY = process.env.E2E_ACME_ADMIN_KEY ?? "clk_demo_acme_key_0002";
const ACME_TENANT = "acme";

test("tenant-admin scales the agent fleet up then down @smoke", async ({ page }) => {
  // Connect as the acme tenant-admin (API-key path).
  await page.goto("/connect");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(ACME_ADMIN_KEY);
  await page.getByLabel("Tenant").fill(ACME_TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();

  // Fleet view → the stats + admin controls are visible.
  await page.goto("/fleet");
  await expect(page.getByRole("heading", { name: "Agent fleet" })).toBeVisible();
  await expect(page.getByText("Provisioned")).toBeVisible();
  await expect(page.getByText("Running")).toBeVisible();
  await expect(page.getByText("Effective limit")).toBeVisible();

  const provision = page.getByRole("button", { name: "Provision +1" });
  const deprovision = page.getByRole("button", { name: "Deprovision −1" });
  await expect(provision).toBeVisible();
  await expect(deprovision).toBeVisible();

  // Provision +1 (unless already at capacity), then deprovision −1 to restore.
  if (await provision.isEnabled()) {
    await provision.click();
    await expect(deprovision).toBeEnabled();
    await deprovision.click();
  }
});
