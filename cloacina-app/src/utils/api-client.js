// API Client for Tauri Backend Communication
const { invoke } = window.__TAURI__.core;

export class ApiClient {
  /**
   * Initialize the application
   */
  async initializeApp() {
    return await invoke("initialize_app");
  }

  /**
   * Get all local runners
   */
  async getLocalRunners() {
    return await invoke("get_local_runners");
  }

  /**
   * Create a new runner
   */
  async createRunner(config) {
    return await invoke("create_runner", { config });
  }

  /**
   * Start a runner
   */
  async startRunner(runnerId) {
    return await invoke("start_local_runner", { runnerId });
  }

  /**
   * Stop a runner
   */
  async stopRunner(runnerId) {
    return await invoke("stop_local_runner", { runnerId });
  }

  /**
   * Get full path from relative path
   */
  async getFullPath(relativePath) {
    return await invoke("get_full_path", { relativePath });
  }

  /**
   * Get runner database path from runner name
   */
  async getRunnerDbPath(runnerName) {
    return await invoke("get_runner_db_path", { runnerName });
  }

  /**
   * Get desktop path
   */
  async getDesktopPath() {
    return await invoke("get_desktop_path");
  }

  /**
   * Generic invoke method for direct API calls
   */
  async invoke(command, args = {}) {
    return await invoke(command, args);
  }

  /**
   * Delete a runner
   */
  async deleteRunner(runnerId) {
    return await invoke("delete_runner", { runnerId });
  }

  /**
   * Delete a runner with database options
   */
  async deleteRunnerWithOptions(runnerId, deleteDatabase) {
    return await invoke("delete_runner", {
      runnerId: runnerId,
      deleteDatabase: deleteDatabase
    });
  }

  /**
   * Build a package
   */
  async buildPackage(request) {
    return await invoke("build_package", { request });
  }

  /**
   * Inspect a package
   */
  async inspectPackage(request) {
    return await invoke("inspect_package", { request });
  }

  /**
   * Debug a package
   */
  async debugPackage(request) {
    return await invoke("debug_package", { request });
  }

  /**
   * Get package visualization data
   */
  async getPackageVisualization(packagePath) {
    return await invoke("get_package_visualization", { packagePath });
  }

  /**
   * Get application settings
   */
  async getSettings() {
    return await invoke("get_settings");
  }

  /**
   * Save application settings
   */
  async saveSettings(settings) {
    return await invoke("save_settings", { settings });
  }

  /**
   * Reset settings to defaults
   */
  async resetSettings() {
    return await invoke("reset_settings");
  }

  /**
   * Perform full system reset
   */
  async fullSystemReset() {
    return await invoke("full_system_reset");
  }

  /**
   * Open file/folder dialogs
   */
  async selectFileDialog(options) {
    return await invoke("select_file_dialog", options);
  }

  async selectDirectoryDialog(options) {
    return await invoke("select_directory_dialog", options);
  }

  /**
   * Open external locations
   */
  async openPath(path) {
    return await invoke("open_file_location", { path });
  }

  async openUrl(url) {
    return await invoke("open_url", { url });
  }
}
