/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import type { Page } from "@playwright/test";

// Wiring from `angreal test ui-e2e` (which stands up its own stack and sets
// these). The defaults let a developer also point the specs at any running
// server — e.g. the compose demo stack (`angreal ui up`).
export const SERVER_URL = process.env.E2E_SERVER_URL ?? "http://localhost:8080";
export const API_KEY = process.env.E2E_API_KEY ?? "clk_dev_ui_bootstrap_key_0001";
export const TENANT = process.env.E2E_TENANT ?? "public";

// Execution IDs the seed harness produced (written to its summary file and
// forwarded by the orchestrator). Empty → the dependent spec skips.
export const INFLIGHT_ID = process.env.E2E_INFLIGHT_EXECUTION_ID ?? "";
export const FAILED_ID = process.env.E2E_FAILED_EXECUTION_ID ?? "";

// Package paths for the upload scenario.
export const VALID_PACKAGE = process.env.E2E_VALID_PACKAGE ?? "";
export const BAD_PACKAGE = process.env.E2E_BAD_PACKAGE ?? "";

const STORAGE_KEY = "cloacina.connection";

/**
 * Pre-seed the SPA's connection into sessionStorage so authenticated specs
 * skip the connect form. AuthContext restores from this key on load without
 * re-validating, so this lands the app authenticated. addInitScript re-runs
 * on every navigation, so it survives reloads within the context.
 */
export async function seedConnection(page: Page): Promise<void> {
  await page.addInitScript(
    ([key, value]) => {
      window.sessionStorage.setItem(key, value);
    },
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );
}
