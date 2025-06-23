// Application Constants
export const APP_VIEWS = {
  LOCAL_RUNNERS: 'local-runners',
  BUILD_PACKAGE: 'build-package',
  INSPECT_PACKAGE: 'inspect-package',
  DEBUG_PACKAGE: 'debug-package',
  WORKFLOWS: 'workflows',
  SETTINGS: 'settings'
};

export const RUNNER_STATUS = {
  RUNNING: 'running',
  STOPPED: 'stopped',
  PAUSED: 'paused'
};

export const EXECUTION_STATUS = {
  READY: 'ready',
  RUNNING: 'running',
  SUCCESS: 'success',
  ERROR: 'error'
};

export const BUILD_PROFILES = {
  DEBUG: 'debug',
  RELEASE: 'release'
};

export const FILE_FILTERS = {
  CLOACINA_PACKAGE: [
    { name: "Cloacina Package", extensions: ["cloacina"] },
    { name: "All Files", extensions: ["*"] }
  ],
  DIRECTORY: {
    directory: true
  }
};

export const UI_CLASSES = {
  HIDDEN: 'hidden',
  ACTIVE: 'active',
  SELECTED: 'selected',
  EXPANDED: 'expanded'
};

export const LOG_LEVELS = {
  ERROR: 'error',
  WARN: 'warn',
  INFO: 'info',
  DEBUG: 'debug',
  TRACE: 'trace'
};
