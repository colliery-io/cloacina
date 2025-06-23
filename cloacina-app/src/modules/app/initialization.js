// App Initialization Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';

export class AppInitializer {
  constructor() {
    this.apiClient = new ApiClient();
  }

  /**
   * Initialize the application
   */
  async initializeApp() {
    try {
      const status = await this.apiClient.initializeApp();
      this.updateAppStatus(status);
      return status;
    } catch (error) {
      this.updateAppStatus(null, `Initialization error: ${error}`);
      throw error;
    }
  }

  /**
   * Update application status display
   */
  updateAppStatus(status, errorMessage = null) {
    const statusEl = UiHelpers.setText("#app-status", "");
    const statusDot = document.querySelector("#app-status-dot");
    const messageEl = UiHelpers.setText("#app-message", "");

    if (errorMessage) {
      UiHelpers.setText("#app-status", "Error");
      if (statusDot) statusDot.className = "status-dot stopped";
      UiHelpers.setText("#app-message", errorMessage);
      return;
    }

    if (status) {
      UiHelpers.setText("#app-status", "Running");
      if (statusDot) statusDot.className = "status-dot running";
      UiHelpers.setText("#app-message",
        `${status.total_runners} runners registered, ${status.running_runners} running, ${status.paused_runners} paused`
      );
    }
  }

  /**
   * Setup global error handlers
   */
  setupErrorHandlers() {
    window.addEventListener('error', (event) => {
      console.error('Global error:', event.error);
      this.updateAppStatus(null, `Application error: ${event.error.message}`);
    });

    window.addEventListener('unhandledrejection', (event) => {
      console.error('Unhandled promise rejection:', event.reason);
      this.updateAppStatus(null, `Promise rejection: ${event.reason}`);
    });
  }

  /**
   * Setup global utility functions for onclick handlers
   */
  setupGlobalFunctions() {
    // Export functions that are called from HTML onclick attributes
    window.toggleAdvancedConfig = () => {
      const content = document.querySelector("#advanced-config-content");
      const icon = document.querySelector("#advanced-expand-icon");

      if (content && icon) {
        if (content.style.display === "none") {
          content.style.display = "block";
          icon.textContent = "▲";
        } else {
          content.style.display = "none";
          icon.textContent = "▼";
        }
      }
    };

    // Add other global functions as needed
    window.removeEnvironmentVariable = (button) => {
      button.closest('.env-var-item').remove();
    };
  }
}
