/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// CLOACI-I-0117 / T-0651 — Vite config for the Cloacina web UI.
export default defineConfig({
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
