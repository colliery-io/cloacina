/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { SERVER_URL } from "./env";

/**
 * CLOACI-T-0787 — tenant-admin key management in the UI. A seeded tenant-admin
 * key logs in, mints a NEW tenant-scoped key (shown once), and revokes it — all
 * through the tenant-scoped `/v1/tenants/{t}/keys` surface (T-0786 rewired the
 * Keys view off the global `/auth/keys`).
 *
 * The isolation negatives — tenant-admin → 403 on a peer tenant's keys and on
 * the god-only global `/auth/keys` — are enforced server-side (ABAC) and proven
 * by the server tests + the live curl walkthrough recorded on this task; the UI
 * never exposes those surfaces. The scoped agent roster rides the full stack.
 *
 * Requires the demo stack (`angreal ui up`) which seeds
 * `acme:clk_demo_acme_key_0002:admin`.
 */
const ACME_ADMIN_KEY = process.env.E2E_ACME_ADMIN_KEY ?? "clk_demo_acme_key_0002";
const ACME_TENANT = "acme";

test("tenant-admin mints a tenant key (shown once) then revokes it @smoke", async ({ page }) => {
  const keyName = `e2e-key-${Date.now()}`;

  // Connect as the acme tenant-admin (API-key path).
  await page.goto("/connect");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(ACME_ADMIN_KEY);
  await page.getByLabel("Tenant").fill(ACME_TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();

  // Keys view → mint a key.
  await page.goto("/keys");
  await page.getByRole("button", { name: "+ Create key" }).click();
  await page.getByLabel("Name").fill(keyName);
  await page.getByRole("dialog").getByRole("button", { name: "Create", exact: true }).click();

  // One-time plaintext reveal (the key is shown exactly once).
  await expect(page.getByText("Copy this now — you won't see it again")).toBeVisible();
  await page.getByRole("button", { name: "Done" }).click();

  // The new key appears in the tenant's roster.
  const row = page.getByRole("row", { name: keyName });
  await expect(row).toBeVisible();

  // Revoke it (row action → confirm dialog).
  await row.getByRole("button", { name: "Revoke" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Revoke", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
