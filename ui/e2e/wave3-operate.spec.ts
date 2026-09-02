/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import { expect, test } from "@playwright/test";

import { API_KEY, SERVER_URL, TENANT } from "./env";

/**
 * CLOACI-T-0934 (I-0141 Wave 3) — the graph/operate surfaces against a live
 * stack: graphs list, per-graph topology, accumulator inject round-trip,
 * triggers table, and the ops page fed by the live WS snapshot.
 * Requires the demo stack (`angreal ui up`).
 */

async function connect(page) {
  await page.goto("/connect");
  await page.getByLabel("Server URL").fill(SERVER_URL);
  await page.getByLabel("API key").fill(API_KEY);
  await page.getByLabel("Tenant").fill(TENANT);
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();
}

test("graphs list, topology, and an accumulator inject round-trip", async ({ page }) => {
  test.setTimeout(300_000);
  // Pick a real graph (with topology if any) from the API. Graphs register
  // asynchronously once the compiler finishes the CG fixture package — on a
  // cold runner that lands mid-suite, so poll instead of asserting instantly.
  let items: {
    name: string;
    topology?: { nodes: unknown[] } | null;
    accumulators: string[];
  }[] = [];
  await expect(async () => {
    const res = await page.request.get(`${SERVER_URL}/v1/health/graphs`, {
      headers: { Authorization: `Bearer ${API_KEY}` },
    });
    items = (await res.json()).items ?? [];
    expect(items.length).toBeGreaterThan(0);
  }).toPass({ timeout: 180_000, intervals: [5_000] });
  const withTopo = items.find((g) => (g.topology?.nodes?.length ?? 0) > 0) ?? items[0];

  await connect(page);

  await page.goto("/graphs");
  await expect(page.getByRole("heading", { name: "Computation graphs" })).toBeVisible();
  await expect(page.getByText(withTopo.name).first()).toBeVisible();

  await page.goto(`/graphs/${encodeURIComponent(withTopo.name)}`);
  await expect(page.getByRole("heading", { name: withTopo.name })).toBeVisible();
  await expect(page.getByText("Topology")).toBeVisible();
  if ((withTopo.topology?.nodes?.length ?? 0) > 0) {
    // The pack's SVG graph rendered.
    await expect(page.locator("svg").first()).toBeVisible();
  }

  // Inject round-trip (bootstrap key is god → write-gated controls visible).
  if (withTopo.accumulators.length > 0) {
    await page.getByRole("button", { name: "＋ inject" }).first().click();
    // Typed fields or the raw-JSON fallback — either way the Inject button fires.
    await page.getByRole("button", { name: "＋ Inject", exact: true }).click();
    await expect(page.getByText(/Delivered to \d+ receiver/)).toBeVisible({ timeout: 15_000 });
    await page.getByRole("button", { name: "Done" }).click();
  }
});

test("triggers table and operations live tiles render", async ({ page }) => {
  await connect(page);

  await page.goto("/triggers");
  await expect(page.getByRole("heading", { name: "Triggers" })).toBeVisible();

  await page.goto("/operations");
  await expect(page.getByRole("heading", { name: "Operations" })).toBeVisible();
  // The warm ops WS delivers a snapshot → the pill flips to "live" and the
  // Server tile renders.
  await expect(page.getByText("live", { exact: true })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText("Server", { exact: true })).toBeVisible();
});

test("trigger fire from the UI runs the subscribed workflow", async ({ page }) => {
  // Needs a named trigger (the demo stack seeds several, e.g. settlement_close).
  const res = await page.request.get(`${SERVER_URL}/v1/tenants/${TENANT}/triggers`, {
    headers: { Authorization: `Bearer ${API_KEY}` },
  });
  const items = (await res.json()).items as { trigger_name: string | null }[];
  const named = items.find((t) => t.trigger_name);
  test.skip(!named, "no named trigger seeded");

  await connect(page);
  await page.goto("/triggers");
  // Open the fire modal on the first fireable row.
  await page.getByRole("button", { name: "Fire trigger" }).first().click();
  await page.getByRole("button", { name: "⚡ Fire", exact: true }).click();
  await expect(page.getByText(/Fired \d+ workflow/)).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { name: "Done" }).click();
});
