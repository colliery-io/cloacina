/// <reference types="vite/client" />

// CLOACI-I-0134 / T-0866 — build-time injected UI version (from ui/package.json,
// via the Vite `define` in vite.config.ts). Single source of version truth (D-2).
declare const __APP_VERSION__: string;

interface CloacinaRuntimeConfig {
  /** Default server URL injected by the deploy container (T-0659). */
  defaultServerUrl?: string;
}

interface Window {
  __CLOACINA_CONFIG__?: CloacinaRuntimeConfig;
}
