/// <reference types="vite/client" />

interface CloacinaRuntimeConfig {
  /** Default server URL injected by the deploy container (T-0659). */
  defaultServerUrl?: string;
}

interface Window {
  __CLOACINA_CONFIG__?: CloacinaRuntimeConfig;
}
