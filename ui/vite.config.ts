/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// CLOACI-I-0134 / T-0866 — single source of version truth: inject the UI version
// from package.json at build time so no hand-typed literal can drift (design D-2).
import pkg from "./package.json";

// CLOACI-I-0117 / T-0651 — Vite config for the Cloacina web UI.
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  plugins: [react()],
  server: {
    port: 5173,
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    // Vitest owns the unit tests under src/; the Playwright e2e specs in e2e/
    // run via `playwright test`, not vitest (CLOACI-I-0129).
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
});
