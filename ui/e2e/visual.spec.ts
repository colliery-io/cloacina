/*
 *  Visual-regression suite (CLOACI-T-0771) — pixel gate for the Aurora Dark
 *  design system. Captures each route at a fixed 1440-wide viewport and asserts
 *  against committed baselines via `toHaveScreenshot`.
 *
 *  Run against the live demo stack (`angreal ui up`):
 *    E2E_BASE_URL=http://localhost:8082 E2E_SERVER_URL=http://localhost:8080 \
 *    E2E_API_KEY=clk_demo_bootstrap_key_0001 E2E_TENANT=public \
 *    npm run test:visual            # assert
 *    npm run test:visual -- -u      # refresh baselines (after intended changes)
 *
 *  What it gates: layout, color, typography, spacing — the design system. Live
 *  data (counts, timestamps, throughput, the status-colored DAG canvases + fire
 *  heatmaps) is MASKED so the gate is deterministic; it catches styling/layout
 *  regressions, not data drift.
 *
 *  Platform note: snapshots are OS-specific (font AA differs). Baselines here are
 *  captured on the maintainer host; regenerate in the CI image with `-u` before
 *  wiring this into CI.
 */
import { test, expect, type Locator } from "@playwright/test";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const STORAGE_KEY = "cloacina.connection";

/** Dynamic regions masked on every shot: live tabular numbers + the
 *  status-colored DAG canvases + the fire-activity heatmap (all change with
 *  demo data). Selectors are best-effort; missing ones are simply no-ops. */
function masks(page: import("@playwright/test").Page): Locator[] {
  return [
    page.locator(".cl-tnum"),
    page.locator('[data-testid="graph-dag"]'),
    page.locator('[data-testid="workflow-graph"]'),
    page.locator('[data-testid="mini-dag"]'),
  ];
}

// Fixed-viewport (not fullPage) capture: list/detail pages grow and shrink as
// demo runs complete, so a fullPage height shift would diff the whole image. The
// 1440x900 viewport gates the chrome + above-the-fold design system at a stable
// size; live content within is masked + tolerated by the ratio.
const SHOT = {
  maxDiffPixelRatio: 0.06,
  animations: "disabled" as const,
  caret: "hide" as const,
  fullPage: false,
};

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(
    ([key, value]) => window.sessionStorage.setItem(key, value),
    [STORAGE_KEY, JSON.stringify({ serverUrl: SERVER_URL, apiKey: API_KEY, tenant: TENANT })] as [
      string,
      string,
    ],
  );
});

async function settle(page: import("@playwright/test").Page) {
  try {
    await page.waitForLoadState("networkidle", { timeout: 8000 });
  } catch {
    /* live polling keeps the network busy; proceed */
  }
  await page.waitForTimeout(900);
}

test("connect gate", async ({ page }) => {
  await page.context().clearCookies();
  await page.addInitScript((key) => window.sessionStorage.removeItem(key), STORAGE_KEY);
  await page.goto("/connect");
  await settle(page);
  await expect(page).toHaveScreenshot("connect.png", { ...SHOT, mask: masks(page) });
});

const ROUTES: Array<{ name: string; path: string }> = [
  { name: "overview", path: "/" },
  { name: "workflows", path: "/workflows" },
  { name: "workflow-detail", path: "/workflows/demo-py-workflow" },
  { name: "executions", path: "/executions" },
  { name: "triggers", path: "/triggers" },
  { name: "graphs", path: "/graphs" },
  { name: "graph-detail", path: "/graphs/market_pipeline" },
];

for (const { name, path } of ROUTES) {
  test(name, async ({ page }) => {
    await page.goto(path);
    await settle(page);
    await expect(page).toHaveScreenshot(`${name}.png`, { ...SHOT, mask: masks(page) });
  });
}
