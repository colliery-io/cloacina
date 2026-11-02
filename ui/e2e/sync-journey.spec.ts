/*
 *  Copyright 2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

import http from "node:http";
import type { AddressInfo } from "node:net";

import { expect, test } from "@playwright/test";

import { API_KEY, SERVER_URL, SYNC_PG_GRANT, SYNC_PG_URL, TENANT, seedConnection } from "./env";

/**
 * CLOACINA-T-1007 — the "Your first sync" journey, from the UI: the
 * frankfurter definition into Postgres. A local server stands in for
 * frankfurter (the definition's `base_url` points at it), so the lane needs
 * no internet. Needs the Postgres and REST connectors staged on the server
 * (`angreal test ui-e2e` does it).
 */

const RATES = [
  { date: "2026-10-23", base: "EUR", quote: "USD", rate: 1.0812 },
  { date: "2026-10-23", base: "EUR", quote: "GBP", rate: 0.8341 },
  { date: "2026-10-23", base: "EUR", quote: "JPY", rate: 164.27 },
];

const headers = { Authorization: `Bearer ${API_KEY}`, "Content-Type": "application/json" };
const api = (path: string) => `${SERVER_URL}/v1/tenants/${TENANT}${path}`;

test("a first sync: frankfurter to Postgres from the UI", async ({ page }) => {
  test.skip(!SYNC_PG_URL, "E2E_SYNC_PG_URL is not set");
  test.setTimeout(240_000);

  let calls = 0;
  const mock = http.createServer((req, res) => {
    if (req.url?.startsWith("/v2/rates")) {
      calls += 1;
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify(RATES));
    } else {
      res.writeHead(404).end();
    }
  });
  await new Promise<void>((done) => mock.listen(0, "127.0.0.1", done));
  const baseUrl = `http://127.0.0.1:${(mock.address() as AddressInfo).port}`;

  const suffix = Date.now().toString(36);
  const name = `first-sync-${suffix}`;
  const table = `fx_rates_${suffix}`;
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(String(e)));

  try {
    await seedConnection(page);
    await page.goto("/syncs");
    await expect(page.getByRole("heading", { name: "Syncs" })).toBeVisible();
    await page.locator('a[href="/syncs/new"]').first().click();
    await expect(page.getByRole("heading", { name: "New sync" })).toBeVisible();

    await page.getByLabel(/^Name/).first().fill(name);

    // Source: the frankfurter definition, pointed at the mock.
    await page.getByLabel("Source", { exact: true }).selectOption("frankfurter");
    await page.getByLabel(/^base_url/).fill(baseUrl);
    await page.getByRole("button", { name: "Check" }).first().click();
    await expect(page.getByText("The connector answered").first()).toBeVisible({ timeout: 60_000 });

    const stream = page.getByLabel("Stream", { exact: true });
    await expect(stream.locator("option[value=latest]")).toHaveCount(1, { timeout: 60_000 });
    await stream.selectOption("latest");
    await page.getByLabel("Write mode").selectOption("Upsert");
    await page.getByLabel(/^Business keys|^Primary key/).fill("date, base, quote");

    // Destination: Postgres.
    await page.getByLabel("Destination", { exact: true }).selectOption("cloacina-connector-postgres");
    await page.getByLabel(/^url/).fill(SYNC_PG_URL);
    await page.getByLabel(/^table/).fill(table);
    await page.getByLabel(/^sslmode/).selectOption("disable");
    await page.getByLabel("Grants (JSON)").fill(JSON.stringify({ tcp: [SYNC_PG_GRANT] }));
    await page.getByRole("button", { name: "Check" }).nth(1).click();
    await expect(page.getByText("The connector answered")).toHaveCount(2, { timeout: 60_000 });

    await page.getByRole("button", { name: "Create sync" }).click();
    await expect(page.getByRole("heading", { name })).toBeVisible({ timeout: 20_000 });

    // Run it, and wait for the pass to complete.
    await page.getByRole("button", { name: /Run now/ }).click();
    await expect(async () => {
      const res = await page.request.get(api(`/syncs/${name}/passes`), { headers });
      expect(res.ok()).toBeTruthy();
      const items = ((await res.json()).items ?? []) as { status: string }[];
      expect(items[0]?.status).toBe("Completed");
    }).toPass({ timeout: 180_000, intervals: [2_000] });
    expect(calls).toBeGreaterThan(0);

    // The page shows the pass, and no dead-letters.
    await page.reload();
    await expect(page.getByRole("table", { name: "Passes" }).getByText("Completed").first()).toBeVisible();
    const st = await page.request.get(api(`/syncs/${name}/streams`), { headers });
    const states = ((await st.json()).items ?? []) as {
      stream: string;
      exhausted: boolean;
      fields: { name: string }[];
    }[];
    const latest = states.find((x) => x.stream === "latest" && x.exhausted);
    expect(latest, JSON.stringify(states)).toBeTruthy();
    // The schema the sync stored, from the definition.
    const fields = latest!.fields.map((f) => f.name);
    for (const f of ["date", "base", "quote", "rate"]) expect(fields).toContain(f);
    const dead = await page.request.get(api(`/syncs/${name}/dead-letters`), { headers });
    expect(((await dead.json()).items ?? []).length).toBe(0);

    // The table landed: the Postgres connector discovers it.
    const disc = await page.request.post(api("/connectors/discover"), {
      headers,
      data: {
        side: "source",
        provider: "cloacina-connector-postgres",
        member: "source",
        config: { url: SYNC_PG_URL, sslmode: "disable" },
        grants: { tcp: [SYNC_PG_GRANT] },
      },
    });
    expect(disc.ok(), await disc.text()).toBeTruthy();
    const streams = ((await disc.json()).streams ?? []) as { name: string }[];
    const names = streams.map((s) => s.name);
    expect(names.some((n) => n === table || n.endsWith(`.${table}`)), JSON.stringify(names)).toBeTruthy();

    expect(errors).toEqual([]);
  } finally {
    await page.request.delete(api(`/workflows/sync/instances/${name}`), { headers }).catch(() => {});
    mock.close();
  }
});
