// Settings Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';
import { LOG_LEVELS } from '../../constants/app-constants.js';

export class SettingsManager {
  constructor() {
    this.apiClient = new ApiClient();
    this.init();
  }

  /**
   * Initialize settings event listeners
   */
  init() {
    // Settings listeners
    document.querySelector("#save-settings-btn")?.addEventListener("click", () => this.saveSettings());
    document.querySelector("#view-logs-btn")?.addEventListener("click", () => this.viewLogFiles());
    document.querySelector("#full-reset-btn")?.addEventListener("click", () => this.fullSystemReset());
    document.querySelector("#change-data-dir-btn")?.addEventListener("click", () => this.changeDataDirectory());

    // Listen for settings reload events
    document.addEventListener('reloadSettings', () => this.loadSettings());
  }

  /**
   * Load application settings
   */
  async loadSettings() {
    try {
      const settings = await this.apiClient.getSettings();
      console.log("Loaded settings:", settings);
      await this.populateSettingsForm(settings);
    } catch (error) {
      console.error("Failed to load settings:", error);
      UiHelpers.showAlert(`Failed to load settings: ${error}`);
    }
  }

  /**
   * Populate settings form with data
   */
  async populateSettingsForm(settings) {
    console.log("Populating settings form with:", settings);
    if (!settings) {
      console.log("No settings provided");
      return;
    }

    // Data directory - if empty, get the default
    const dataDirectoryInput = document.querySelector("#data-directory");
    console.log("Data directory input element:", dataDirectoryInput);
    console.log("Settings data_directory:", settings.data_directory);

    if (dataDirectoryInput) {
      let dataDirectory = settings.data_directory;

      // If data directory is empty, get the default
      if (!dataDirectory) {
        try {
          dataDirectory = await this.apiClient.invoke("get_data_directory");
          console.log("Got default data directory:", dataDirectory);
        } catch (error) {
          console.error("Failed to get default data directory:", error);
          dataDirectory = "";
        }
      }

      dataDirectoryInput.value = dataDirectory;
      console.log("Set input value to:", dataDirectoryInput.value);
    }

    // Log level
    const logLevelSelect = document.querySelector("#log-level");
    if (logLevelSelect && settings.log_level) {
      logLevelSelect.value = settings.log_level;
    }

    // Max log files
    const maxLogFilesInput = document.querySelector("#max-log-files");
    if (maxLogFilesInput && settings.max_log_files) {
      maxLogFilesInput.value = settings.max_log_files;
    }
  }

  /**
   * Save application settings
   */
  async saveSettings() {
    try {
      const settings = this.collectSettingsFromForm();

      if (!this.validateSettings(settings)) {
        return;
      }

      await this.apiClient.saveSettings(settings);
      UiHelpers.showAlert("Settings saved successfully");
    } catch (error) {
      console.error("Failed to save settings:", error);
      UiHelpers.showAlert(`Failed to save settings: ${error}`);
    }
  }

  /**
   * Collect settings from form
   */
  collectSettingsFromForm() {
    const logLevelSelect = document.querySelector("#log-level");
    const maxLogFilesInput = document.querySelector("#max-log-files");
    const dataDirectoryInput = document.querySelector("#data-directory");

    const dataDirectory = dataDirectoryInput?.value || "";

    return {
      data_directory: dataDirectory,
      app_database_path: `${dataDirectory}/cloacina-app.db`,
      log_directory: `${dataDirectory}/logs`,
      log_level: logLevelSelect?.value || LOG_LEVELS.INFO,
      max_log_files: parseInt(maxLogFilesInput?.value || "10")
    };
  }

  /**
   * Validate settings
   */
  validateSettings(settings) {
    if (!Object.values(LOG_LEVELS).includes(settings.log_level)) {
      UiHelpers.showAlert("Invalid log level selected");
      return false;
    }

    if (settings.max_log_files < 1 || settings.max_log_files > 100) {
      UiHelpers.showAlert("Max log files must be between 1 and 100");
      return false;
    }

    return true;
  }

  /**
   * Reset settings to defaults
   */
  async resetSettings() {
    if (!UiHelpers.showConfirm("Are you sure you want to reset all settings to defaults?")) {
      return;
    }

    try {
      const result = await this.apiClient.resetSettings();

      if (result.success) {
        UiHelpers.showAlert("Settings reset to defaults");
        await this.loadSettings(); // Reload form
      } else {
        UiHelpers.showAlert(`Failed to reset settings: ${result.error}`);
      }
    } catch (error) {
      console.error("Failed to reset settings:", error);
      UiHelpers.showAlert(`Failed to reset settings: ${error}`);
    }
  }

  /**
   * View log files
   */
  async viewLogFiles() {
    try {
      const settings = await this.apiClient.getSettings();
      let dataDirectory = settings?.data_directory;

      // If data directory is empty, get the default
      if (!dataDirectory) {
        dataDirectory = await this.apiClient.invoke("get_data_directory");
      }

      if (dataDirectory) {
        const logPath = `${dataDirectory}/logs`;
        await this.apiClient.openPath(logPath);
      } else {
        UiHelpers.showAlert("Could not determine log directory location");
      }
    } catch (error) {
      console.error("Failed to open log folder:", error);
      UiHelpers.showAlert(`Failed to open log folder: ${error}`);
    }
  }

  /**
   * Change data directory
   */
  async changeDataDirectory() {
    try {
      // Show folder picker dialog
      const selectedPath = await this.apiClient.selectDirectoryDialog({
        title: "Select Data Directory"
      });

      if (!selectedPath) {
        // User cancelled the dialog
        return;
      }

      // Confirm the change
      const newDbPath = `${selectedPath}/cloacina-app.db`;
      const confirmed = UiHelpers.showConfirm(
        `Change data directory to:\n${selectedPath}\n\nDatabase will be located at:\n${newDbPath}\n\nThis will move your existing database to the new location. Continue?`
      );

      if (!confirmed) {
        return;
      }

      // Change the database location
      await this.apiClient.invoke("change_database_location", { newPath: newDbPath });

      // Reload settings to show the new path
      await this.loadSettings();

      UiHelpers.showAlert("Data directory changed successfully");

    } catch (error) {
      console.error("Failed to change data directory:", error);
      UiHelpers.showAlert(`Failed to change data directory: ${error}`);
    }
  }

  /**
   * Full system reset
   */
  async fullSystemReset() {
    const confirmed = UiHelpers.showConfirm(
      "⚠️ WARNING: This will permanently delete ALL data including:\n\n" +
      "• All runners and their databases\n" +
      "• All application settings\n" +
      "• All log files\n" +
      "• All cached data\n\n" +
      "This action CANNOT be undone!\n\n" +
      "Are you absolutely sure you want to proceed?"
    );

    if (!confirmed) {
      return;
    }

    // Double confirmation
    const doubleConfirmed = UiHelpers.showConfirm(
      "Last chance! This will delete EVERYTHING.\n\n" +
      "Type 'DELETE' to confirm you want to proceed with the full system reset."
    );

    if (!doubleConfirmed) {
      return;
    }

    try {
      UiHelpers.setText("#full-reset-btn", "Resetting...");
      UiHelpers.setButtonState("#full-reset-btn", true);

      const result = await this.apiClient.fullSystemReset();

      if (result.success) {
        UiHelpers.showAlert("System reset completed. The application will restart.");
        // The application should restart automatically
        window.location.reload();
      } else {
        UiHelpers.showAlert(`Failed to reset system: ${result.error}`);
      }
    } catch (error) {
      console.error("Failed to perform system reset:", error);
      UiHelpers.showAlert(`Failed to reset system: ${error}`);
    } finally {
      UiHelpers.setText("#full-reset-btn", "Full System Reset");
      UiHelpers.setButtonState("#full-reset-btn", false);
    }
  }
}
