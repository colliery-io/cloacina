/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright config for the UI acceptance e2e (CLOACI-I-0117 / T-0661).
 *
 * The stack (server + compiler + seeded data + a served SPA) is orchestrated
 * externally by `angreal test ui-e2e`, which exports the E2E_* env the specs
 * read. This config only points the browser at the served UI; it does NOT
 * start a webServer of its own.
 *
 * Single browser project (chromium) — that doubles as the I-0117 real-browser
 * smoke (NFR). Serial + single worker because the specs share one seeded
 * server's state.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  timeout: 90_000,
  expect: { timeout: 15_000 },
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: process.env.E2E_BASE_URL ?? "http://localhost:4173",
    headless: true,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
