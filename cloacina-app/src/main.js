// Modular Main Application Entry Point
import { AppInitializer } from './modules/app/initialization.js';
import { NavigationManager } from './modules/app/navigation.js';
import { SettingsManager } from './modules/app/settings.js';
import { RunnerManager } from './modules/runners/management.js';
import { PackageBuildManager } from './modules/packages/build.js';
import { PackageInspectManager } from './modules/packages/inspect.js';
import { DebugPackageManager } from './modules/packages/debug.js';

class CloacinaDesktopApp {
  constructor() {
    this.modules = {};
    this.isInitialized = false;
  }

  /**
   * Initialize the application and all modules
   */
  async init() {
    try {
      console.log('Initializing Cloacina Desktop App...');

      // Core app modules
      this.modules.appInitializer = new AppInitializer();
      this.modules.navigation = new NavigationManager();
      this.modules.settings = new SettingsManager();

      // Feature modules
      this.modules.runnerManager = new RunnerManager();
      this.modules.packageBuild = new PackageBuildManager();
      this.modules.packageInspect = new PackageInspectManager();
      this.modules.debugPackage = new DebugPackageManager();

      // Setup global functions and error handlers
      this.modules.appInitializer.setupGlobalFunctions();
      this.modules.appInitializer.setupErrorHandlers();

      // Initialize the backend
      await this.modules.appInitializer.initializeApp();

      // Load initial data
      await this.loadInitialData();

      // Setup cross-module event listeners
      this.setupCrossModuleEvents();

      this.isInitialized = true;
      console.log('Cloacina Desktop App initialized successfully');

    } catch (error) {
      console.error('Failed to initialize application:', error);
      this.modules.appInitializer?.updateAppStatus(null, `Initialization failed: ${error}`);
    }
  }

  /**
   * Load initial application data
   */
  async loadInitialData() {
    try {
      // Load runners
      await this.modules.runnerManager.loadRunners();

      // Load settings
      await this.modules.settings.loadSettings();

    } catch (error) {
      console.error('Failed to load initial data:', error);
    }
  }

  /**
   * Setup cross-module event listeners for communication between modules
   */
  setupCrossModuleEvents() {
    // Navigation events from package build to inspect
    document.addEventListener('navigateToInspect', (event) => {
      this.modules.navigation.navigateToInspectPackage(event.detail.packagePath);
    });

    // Navigation events to debug package
    document.addEventListener('navigateToDebug', (event) => {
      this.modules.navigation.navigateToDebugPackage(event.detail.packagePath);
    });

    // Auto-load debug package event
    document.addEventListener('autoLoadDebugPackage', (event) => {
      this.modules.debugPackage.autoLoadPackage(event.detail.packagePath);
    });

    // Auto-load inspect package event
    document.addEventListener('autoLoadInspectPackage', (event) => {
      if (event.detail.packagePath) {
        document.querySelector("#inspect-package-path").value = event.detail.packagePath;
        this.modules.packageInspect.inspectPackage();
      }
    });
  }

  /**
   * Get a specific module instance
   */
  getModule(moduleName) {
    return this.modules[moduleName];
  }

  /**
   * Check if app is initialized
   */
  get initialized() {
    return this.isInitialized;
  }
}

// Create global app instance
const app = new CloacinaDesktopApp();

// Initialize when DOM is ready
window.addEventListener("DOMContentLoaded", async () => {
  await app.init();
});

// Export for debugging and development
window.CloacinaApp = app;

export default app;
