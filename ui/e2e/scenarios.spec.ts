/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import {
  BAD_PACKAGE,
  FAILED_ID,
  INFLIGHT_ID,
  VALID_PACKAGE,
  seedConnection,
} from "./env";

// All scenarios below run authenticated — pre-seed the connection so each
// starts inside the app shell.
test.beforeEach(async ({ page }) => {
  await seedConnection(page);
});

/**
 * UC-0 / REQ-004: the executions list reflects the seeded runs, and the
 * `?status=Failed` debug entry point filters to the failed run.
 */
test("executions list reflects seeded runs and filters by status @smoke", async ({ page }) => {
  await page.goto("/executions");
  await expect(page.getByText("demo_slow_workflow").first()).toBeVisible();
  await expect(page.getByText("demo_fail_workflow").first()).toBeVisible();

  // Deep-link debug filter: only the failed run remains.
  await page.goto("/executions?status=Failed");
  await expect(page.getByText("demo_fail_workflow").first()).toBeVisible();
  await expect(page.getByText(/failed/i).first()).toBeVisible();
});

/**
 * UC-1 / NFR-002 (the centerpiece): open an in-flight run and watch it stream
 * to a terminal state. Assert on *eventual* terminal state, not exact event
 * timing, so the live stream's pacing can't make this flaky.
 */
test("following an in-flight run reaches a terminal state", async ({ page }) => {
  test.skip(!INFLIGHT_ID, "no in-flight execution id provided by the harness");

  await page.goto(`/executions/${INFLIGHT_ID}`);
  // Aurora redesign: the detail's landmarks are the workflow-name title + the
  // "Task graph" / "Event log" section labels (not a generic "Execution" heading).
  await expect(page.getByText("Task graph")).toBeVisible();
  await expect(page.getByText("Event log")).toBeVisible();

  // The chained steps finish and the status badge flips to a terminal state.
  // Assert on the badge specifically (testid, not stray page text) within a
  // generous window so the live stream's pacing can't make this flaky
  // (NFR-002: the in-flight run is followed through to completion).
  await expect(page.getByTestId("execution-status")).toHaveText(/complete/i, {
    timeout: 80_000,
  });
});

/**
 * UC-2: open the failed run → its status + event log are visible for debugging.
 */
test("the failed run shows its status and event log @smoke", async ({ page }) => {
  test.skip(!FAILED_ID, "no failed execution id provided by the harness");

  await page.goto(`/executions/${FAILED_ID}`);
  await expect(page.getByText("Task graph")).toBeVisible();
  await expect(page.getByText(/failed/i).first()).toBeVisible();
  await expect(page.getByText("Event log")).toBeVisible();
});

/**
 * UC-3: package upload — success path and the rejected-package error path.
 */
test("workflow upload — success and rejected-package paths", async ({ page }) => {
  test.skip(!VALID_PACKAGE || !BAD_PACKAGE, "no package paths provided by the harness");

  // Success: a real .cloacina registers and surfaces a confirmation.
  await page.goto("/workflows/upload");
  await page.setInputFiles('input[type="file"]', VALID_PACKAGE);
  await page.getByRole("button", { name: "Upload" }).click();
  await expect(page.getByText(/package registered/i)).toBeVisible({ timeout: 30_000 });

  // Rejected: a garbage package surfaces the server's typed error inline.
  await page.goto("/workflows/upload");
  await page.setInputFiles('input[type="file"]', BAD_PACKAGE);
  await page.getByRole("button", { name: "Upload" }).click();
  await expect(page.getByRole("alert")).toBeVisible({ timeout: 30_000 });
});

/**
 * UC-4: API key lifecycle — create (one-time plaintext shown once) → revoke.
 */
test("API key create (one-time plaintext) then revoke", async ({ page }) => {
  await page.goto("/keys");

  const keyName = "e2e-key";
  await page.getByRole("button", { name: "Create key" }).click();
  const createDialog = page.getByRole("dialog");
  await createDialog.getByLabel("Name").fill(keyName);
  await createDialog.getByRole("button", { name: "Create" }).click();

  // One-time plaintext reveal: shown exactly once with the warning.
  const revealDialog = page.getByRole("dialog");
  await expect(revealDialog.getByText(/won't see it again/i)).toBeVisible();
  await revealDialog.getByRole("button", { name: "Done" }).click();

  // The new key is in the list; revoke it via the confirm dialog.
  const row = page.getByRole("row", { name: new RegExp(keyName) });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Revoke" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Revoke" }).click();

  // The row reflects the revoked state.
  await expect(page.getByRole("row", { name: new RegExp(keyName) }).getByText(/revoked/i)).toBeVisible();
});
