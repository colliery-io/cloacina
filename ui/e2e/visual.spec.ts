/*
 *  Visual-regression suite (CLOACI-T-0771) — pixel gate for the Aurora design
 *  system in its two themes (CLOACINA-T-0943). Captures each route in light
 *  and in dark at a fixed 1440-wide viewport and asserts against committed
 *  baselines via `toHaveScreenshot`.
 *
 *  Run on the seeded e2e stack (the deterministic environment; from the repo
 *  root):
 *    angreal test ui-e2e --visual                       # assert
 *    angreal test ui-e2e --visual --update-baselines    # refresh baselines
 *
 *  Or against the live demo stack (`angreal ui up`):
 *    npm run test:visual            # assert
 *    npm run test:visual -- -u      # refresh baselines (after intended changes)
 *
 *  The detail routes come from the environment, so one spec serves both
 *  stacks: E2E_VISUAL_WORKFLOW (a package name; default `demo-py-workflow`,
 *  the demo stack's) and E2E_VISUAL_GRAPH (default `market_pipeline`). The
 *  graph-detail shot skips when the server has no such graph. The seeded e2e
 *  lane sets E2E_VISUAL_GRAPH=demo_py_graph, the Python graph it packs
 *  (CLOACINA-T-0939).
 *
 *  Theme: Aurora reads `localStorage["aurora-theme"]` before the first paint
 *  (THEME_INIT_SCRIPT in index.html), so each shot seeds that key and
 *  emulates the matching `prefers-color-scheme`.
 *
 *  What it gates: layout, color, typography, spacing — the design system. Live
 *  data (counts, timestamps, throughput, the status-colored DAG canvases) is
 *  MASKED so the gate is deterministic; it catches styling/layout regressions,
 *  not data drift.
 *
 *  Platform note: snapshots are OS-specific (font AA differs). The darwin
 *  baselines are captured on the maintainer host; the `ui-visual` workflow
 *  generates the linux ones (`update_baselines = true`).
 */
import { test, expect, type Locator } from "@playwright/test";
import { API_KEY, SERVER_URL, TENANT } from "./env";

const STORAGE_KEY = "cloacina.connection";
const THEME_KEY = "aurora-theme";
const THEMES = ["light", "dark"] as const;
type Theme = (typeof THEMES)[number];

const VISUAL_WORKFLOW = process.env.E2E_VISUAL_WORKFLOW ?? "demo-py-workflow";
const VISUAL_GRAPH = process.env.E2E_VISUAL_GRAPH ?? "market_pipeline";

/** Dynamic regions masked on every shot: live tabular numbers + the
 *  status-colored DAG canvases (all change with the seeded data). Selectors
 *  are best-effort; a selector that matches nothing is a no-op. */
function masks(page: import("@playwright/test").Page): Locator[] {
  return [
    page.locator(".cl-tnum"),
    page.locator('[data-testid="graph-dag"]'),
    page.locator('[data-testid="workflow-graph"]'),
  ];
}

// Fixed-viewport (not fullPage) capture: list/detail pages grow and shrink as
// seeded runs complete, so a fullPage height shift would diff the whole image.
// The 1440x900 viewport gates the chrome + above-the-fold design system at a
// stable size; live content within is masked + tolerated by the ratio.
const SHOT = {
  maxDiffPixelRatio: 0.06,
  animations: "disabled" as const,
  caret: "hide" as const,
  fullPage: false,
};

async function useTheme(page: import("@playwright/test").Page, theme: Theme) {
  await page.emulateMedia({ colorScheme: theme });
  await page.addInitScript(
    ([key, value]) => window.localStorage.setItem(key, value),
    [THEME_KEY, theme] as [string, string],
  );
}

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

/** The names of the computation graphs the server knows, or [] when it has
 *  none (the seeded e2e lane). */
async function graphNames(page: import("@playwright/test").Page): Promise<string[]> {
  try {
    const res = await page.request.get(`${SERVER_URL}/v1/health/graphs`, {
      headers: { Authorization: `Bearer ${API_KEY}` },
    });
    const items = ((await res.json()).items ?? []) as { name: string }[];
    return items.map((g) => g.name);
  } catch {
    return [];
  }
}

const ROUTES: Array<{ name: string; path: string }> = [
  { name: "overview", path: "/" },
  { name: "workflows", path: "/workflows" },
  { name: "workflow-detail", path: `/workflows/${encodeURIComponent(VISUAL_WORKFLOW)}` },
  { name: "executions", path: "/executions" },
  { name: "triggers", path: "/triggers" },
  { name: "graphs", path: "/graphs" },
];

for (const theme of THEMES) {
  test.describe(theme, () => {
    test.beforeEach(async ({ page }) => {
      await useTheme(page, theme);
    });

    test(`connect gate (${theme})`, { tag: "@visual" }, async ({ page }) => {
      await page.context().clearCookies();
      await page.addInitScript((key) => window.sessionStorage.removeItem(key), STORAGE_KEY);
      await page.goto("/connect");
      await settle(page);
      await expect(page).toHaveScreenshot(`connect-${theme}.png`, { ...SHOT, mask: masks(page) });
    });

    for (const { name, path } of ROUTES) {
      test(`${name} (${theme})`, { tag: "@visual" }, async ({ page }) => {
        await page.goto(path);
        await settle(page);
        await expect(page).toHaveScreenshot(`${name}-${theme}.png`, { ...SHOT, mask: masks(page) });
      });
    }

    test(`graph-detail (${theme})`, { tag: "@visual" }, async ({ page }) => {
      const names = await graphNames(page);
      test.skip(
        !names.includes(VISUAL_GRAPH),
        `the server has no graph "${VISUAL_GRAPH}"`,
      );
      await page.goto(`/graphs/${encodeURIComponent(VISUAL_GRAPH)}`);
      await settle(page);
      await expect(page).toHaveScreenshot(`graph-detail-${theme}.png`, { ...SHOT, mask: masks(page) });
    });
  });
}
