// Runner Management Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';
import { RUNNER_STATUS } from '../../constants/app-constants.js';

export class RunnerManager {
  constructor() {
    this.apiClient = new ApiClient();
    this.runners = [];
    this.statusUpdateTimer = null;
    this.init();
    this.exposeGlobalFunctions();
    this.startStatusUpdateTimer();
  }

  /**
   * Initialize runner management event listeners
   */
  init() {
    // Runner creation
    document.querySelector("#create-runner-btn")?.addEventListener("click", () => this.showCreateRunnerForm());
    document.querySelector("#save-runner-btn")?.addEventListener("click", () => this.createRunner());
    document.querySelector("#cancel-runner-btn")?.addEventListener("click", () => this.hideCreateRunnerForm());

    // Auto-update runner name path
    document.querySelector("#new-runner-name")?.addEventListener("input", async (e) => {
      await this.updateRunnerDbPath(e.target.value.trim());
    });
  }

  /**
   * Load and display all runners
   */
  async loadRunners() {
    try {
      this.runners = await this.apiClient.getLocalRunners();
      this.renderRunnersList();
    } catch (error) {
      console.error("Failed to load runners:", error);
      this.renderRunnersError(error);
    }
  }

  /**
   * Start periodic status updates
   */
  startStatusUpdateTimer() {
    // Update every 5 seconds
    this.statusUpdateTimer = setInterval(() => {
      this.updateRunnerStatus();
    }, 5000);
  }

  /**
   * Stop periodic status updates
   */
  stopStatusUpdateTimer() {
    if (this.statusUpdateTimer) {
      clearInterval(this.statusUpdateTimer);
      this.statusUpdateTimer = null;
    }
  }

  /**
   * Update runner status in background without full UI refresh
   */
  async updateRunnerStatus() {
    try {
      this.runners = await this.apiClient.getLocalRunners();
      this.updateAppStatusMessage();
    } catch (error) {
      console.error("Failed to update runner status:", error);
    }
  }

  /**
   * Update the app status message with current runner counts
   */
  updateAppStatusMessage() {
    const totalRunners = this.runners.length;
    const runningRunners = this.runners.filter(r => r.running).length;
    const pausedRunners = this.runners.filter(r => r.paused).length;

    const messageEl = document.querySelector("#app-message");
    if (messageEl) {
      messageEl.textContent = `${totalRunners} runners registered, ${runningRunners} running, ${pausedRunners} paused`;
    }
  }

  /**
   * Render the runners list
   */
  renderRunnersList() {
    const container = document.querySelector("#runners-list");
    if (!container) return;

    // Update status message whenever we render the list
    this.updateAppStatusMessage();

    if (this.runners.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <h3>No runners created yet</h3>
          <p>Create your first local runner to start orchestrating workflows</p>
        </div>
      `;
      return;
    }

    container.innerHTML = this.runners.map(runner => this.createRunnerItemHTML(runner)).join('');
  }

  /**
   * Create HTML for a single runner item
   */
  createRunnerItemHTML(runner) {
    const registryType = 'Full Runner (Cron + Registry)';

    return `
    <div class="runner-item">
      <div class="runner-info">
        <div class="runner-header">
          <div class="runner-title">${runner.config.name}</div>
          <div class="runner-uuid">UUID: ${runner.id}</div>
        </div>
        <div class="runner-details">
          <div class="detail-row">
            <span class="detail-label">Database:</span>
            <span class="detail-value">${runner.config.db_path}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Registry Type:</span>
            <span class="detail-value">${registryType}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Max Concurrent Tasks:</span>
            <span class="detail-value">${runner.config.max_concurrent_tasks}</span>
          </div>
        </div>
      </div>
      <div class="runner-status">
        <span class="status-dot ${this.getRunnerStatusClass(runner)}"></span>
        <span>${runner.message}</span>
      </div>
      <div class="runner-controls">
        ${this.createRunnerControlsHTML(runner)}
      </div>
    </div>
    `;
  }

  /**
   * Get CSS class for runner status
   */
  getRunnerStatusClass(runner) {
    if (runner.running) return RUNNER_STATUS.RUNNING;
    if (runner.is_paused) return RUNNER_STATUS.STOPPED;
    return RUNNER_STATUS.STOPPED;
  }

  /**
   * Create HTML for runner controls
   */
  createRunnerControlsHTML(runner) {
    const startStopButton = !runner.running ?
      `<button class="btn btn-primary btn-sm" onclick="startRunner('${runner.id}')">
        <span class="btn-icon">▶️</span>
        Start
      </button>` :
      `<button class="btn btn-danger btn-sm" onclick="stopRunner('${runner.id}')">
        <span class="btn-icon">⏹️</span>
        Stop
      </button>`;

    const deleteButton = `
      <button class="btn btn-outline btn-sm" onclick="deleteRunner('${runner.id}')">
        <span class="btn-icon">🗑️</span>
        Delete
      </button>
    `;

    return startStopButton + deleteButton;
  }

  /**
   * Make functions globally accessible for onclick handlers
   */
  exposeGlobalFunctions() {
    window.startRunner = (runnerId) => this.startRunner(runnerId);
    window.stopRunner = (runnerId) => this.stopRunner(runnerId);
    window.deleteRunner = (runnerId) => this.deleteRunner(runnerId);
  }

  /**
   * Render runners error state
   */
  renderRunnersError(error) {
    const container = document.querySelector("#runners-list");
    if (container) {
      container.innerHTML = `
        <div class="error-state">
          <h3>Failed to load runners</h3>
          <p>Error: ${error}</p>
          <button class="btn btn-outline" onclick="location.reload()">Retry</button>
        </div>
      `;
    }
  }

  /**
   * Show create runner form
   */
  showCreateRunnerForm() {
    UiHelpers.show("#create-runner-form");

    // Auto-focus on name input
    const nameInput = document.querySelector("#new-runner-name");
    if (nameInput) {
      nameInput.focus();
    }
  }

  /**
   * Hide create runner form
   */
  hideCreateRunnerForm() {
    UiHelpers.hide("#create-runner-form");
    this.clearCreateRunnerForm();
  }

  /**
   * Clear create runner form
   */
  clearCreateRunnerForm() {
    UiHelpers.clearForm("#create-runner-form");

    // Reset the database path since it's auto-generated
    UiHelpers.setText("#new-db-path", "");
    const dbPathInput = document.querySelector("#new-db-path");
    if (dbPathInput) {
      dbPathInput.value = "";
    }
  }

  /**
   * Update runner database path based on name
   */
  async updateRunnerDbPath(runnerName) {
    const dbPathInput = document.querySelector("#new-db-path");

    if (!runnerName) {
      if (dbPathInput) {
        dbPathInput.value = "";
      }
      return;
    }

    try {
      const dbPath = await this.apiClient.getRunnerDbPath(runnerName);
      if (dbPathInput) {
        dbPathInput.value = dbPath;
      }
    } catch (error) {
      console.error("Failed to get runner db path:", error);
      // Fallback to client-side generation
      const sanitizedName = runnerName.toLowerCase().replace(/[^a-z0-9]/g, '_');
      const fallbackPath = `runners/${sanitizedName}.db`;
      if (dbPathInput) {
        dbPathInput.value = fallbackPath;
      }
    }
  }

  /**
   * Create a new runner
   */
  async createRunner() {
    try {
      UiHelpers.setText("#save-runner-btn", "Creating...");
      UiHelpers.setButtonState("#save-runner-btn", true);

      const config = await this.collectRunnerConfig(); // Now async

      if (!this.validateRunnerConfig(config)) {
        return;
      }

      await this.apiClient.createRunner(config);

      // Clear form and hide it (like original code)
      this.hideCreateRunnerForm();
      await this.loadRunners();

      console.log("Runner created and started successfully!");
    } catch (error) {
      console.error("Failed to create runner:", error);
      console.log(`Failed to create runner: ${error}`);
    } finally {
      UiHelpers.setText("#save-runner-btn", "Create & Start Runner");
      UiHelpers.setButtonState("#save-runner-btn", false);
    }
  }

  /**
   * Collect runner configuration from form
   */
  async collectRunnerConfig() {
    // Convert relative path to full path like the original code
    const relativePath = document.querySelector("#new-db-path")?.value || "";
    const fullPath = await this.apiClient.getFullPath(relativePath);

    return {
      name: document.querySelector("#new-runner-name")?.value || "",
      db_path: fullPath,
      max_concurrent_tasks: parseInt(document.querySelector("#new-max-tasks")?.value || "8"),
      enable_cron_scheduling: true,
      enable_registry_reconciler: true, // This was missing!
      cron_poll_interval: parseInt(document.querySelector("#cron-poll-interval")?.value || "30"),
      cron_recovery_interval: parseInt(document.querySelector("#cron-recovery-interval")?.value || "5"),
      cron_lost_threshold: parseInt(document.querySelector("#cron-lost-threshold")?.value || "10"),
      registry_reconcile_interval: parseInt(document.querySelector("#registry-reconcile-interval")?.value || "60"),
      executor_poll_interval: parseInt(document.querySelector("#executor-poll-interval")?.value || "100"),
      scheduler_poll_interval: parseInt(document.querySelector("#scheduler-poll-interval")?.value || "100"),
      task_timeout: parseInt(document.querySelector("#task-timeout")?.value || "5")
    };
  }

  /**
   * Validate runner configuration
   */
  validateRunnerConfig(config) {
    if (!config.name.trim()) {
      UiHelpers.showAlert("Runner name is required");
      return false;
    }

    if (config.max_concurrent_tasks < 1 || config.max_concurrent_tasks > 64) {
      UiHelpers.showAlert("Max concurrent tasks must be between 1 and 64");
      return false;
    }

    return true;
  }

  /**
   * Start a runner
   */
  async startRunner(runnerId) {
    try {
      await this.apiClient.startRunner(runnerId);
      await this.loadRunners(); // Refresh the list
    } catch (error) {
      console.error("Failed to start runner:", error);
      UiHelpers.showAlert(`Failed to start runner: ${error}`);
    }
  }

  /**
   * Stop a runner
   */
  async stopRunner(runnerId) {
    try {
      await this.apiClient.stopRunner(runnerId);
      await this.loadRunners(); // Refresh the list
    } catch (error) {
      console.error("Failed to stop runner:", error);
      UiHelpers.showAlert(`Failed to stop runner: ${error}`);
    }
  }

  /**
   * Delete a runner
   */
  async deleteRunner(runnerId) {
    console.log(`deleteRunner called with: ${runnerId}`);

    // Show custom confirmation modal
    const confirmed = await this.showDeleteConfirmation();

    if (!confirmed.delete) {
      console.log("Delete cancelled by user");
      return;
    }

    console.log(`Confirmed - attempting to delete runner: ${runnerId}. Delete database: ${confirmed.deleteDatabase}`);
    try {
      await this.apiClient.deleteRunnerWithOptions(runnerId, confirmed.deleteDatabase);
      console.log("Runner deleted successfully");
      await this.loadRunners();
    } catch (error) {
      console.error("Delete runner error:", error);
      console.log(`Failed to delete runner: ${error}`);
    }
  }

  /**
   * Show custom delete confirmation modal
   */
  showDeleteConfirmation() {
    return new Promise((resolve) => {
      // Create modal HTML
      const modalHTML = `
        <div id="delete-modal" class="modal-overlay">
          <div class="modal-content">
            <div class="modal-header">
              <h3>Delete Runner</h3>
            </div>
            <div class="modal-body">
              <p>Are you sure you want to delete this runner?</p>
              <div class="checkbox-item">
                <label>
                  <input type="checkbox" id="delete-database-checkbox" checked>
                  Also delete the runner's database and all its data
                </label>
              </div>
            </div>
            <div class="modal-footer">
              <button id="cancel-delete" class="btn btn-outline">Cancel</button>
              <button id="confirm-delete" class="btn btn-danger">Delete Runner</button>
            </div>
          </div>
        </div>
      `;

      // Add modal to page
      document.body.insertAdjacentHTML('beforeend', modalHTML);

      // Add event listeners
      document.getElementById('cancel-delete').addEventListener('click', () => {
        document.getElementById('delete-modal').remove();
        resolve({ delete: false, deleteDatabase: false });
      });

      document.getElementById('confirm-delete').addEventListener('click', () => {
        const deleteDatabase = document.getElementById('delete-database-checkbox').checked;
        document.getElementById('delete-modal').remove();
        resolve({ delete: true, deleteDatabase });
      });

      // Close on overlay click
      document.getElementById('delete-modal').addEventListener('click', (e) => {
        if (e.target.id === 'delete-modal') {
          document.getElementById('delete-modal').remove();
          resolve({ delete: false, deleteDatabase: false });
        }
      });
    });
  }
}
