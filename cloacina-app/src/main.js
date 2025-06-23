const { invoke } = window.__TAURI__.core;

let currentView = 'local-runners';
let runners = [];

// Initialize the application
async function initializeApp() {
  try {
    const status = await invoke("initialize_app");
    updateAppStatus(status);
    await loadRunners();
  } catch (error) {
    updateAppStatus(null, `Initialization error: ${error}`);
  }
}

// Update application status display
function updateAppStatus(status, errorMessage = null) {
  const statusEl = document.querySelector("#app-status");
  const statusDot = document.querySelector("#app-status-dot");
  const messageEl = document.querySelector("#app-message");

  if (errorMessage) {
    statusEl.textContent = "Error";
    statusDot.className = "status-dot stopped";
    messageEl.textContent = errorMessage;
    return;
  }

  if (status) {
    statusEl.textContent = "Running";
    statusDot.className = "status-dot running";
    messageEl.textContent = `${status.total_runners} runners registered, ${status.running_runners} running, ${status.paused_runners} paused`;
  }
}

// Load and display all runners
async function loadRunners() {
  try {
    runners = await invoke("get_local_runners");
    renderRunnersList();
  } catch (error) {
    console.error("Failed to load runners:", error);
    renderRunnersError(error);
  }
}

// Render the runners list
function renderRunnersList() {
  const container = document.querySelector("#runners-list");

  if (runners.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <h3>No runners created yet</h3>
        <p>Create your first local runner to start orchestrating workflows</p>
      </div>
    `;
    return;
  }

  container.innerHTML = runners.map(runner => {
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
        <span class="status-dot ${runner.running ? 'running' : (runner.is_paused ? 'stopped' : 'stopped')}"></span>
        <span>${runner.message}</span>
      </div>
      <div class="runner-controls">
        ${!runner.running ?
          `<button class="btn btn-primary btn-sm" onclick="startRunner('${runner.id}')">
            <span class="btn-icon">▶️</span>
            Start
          </button>` :
          `<button class="btn btn-danger btn-sm" onclick="stopRunner('${runner.id}')">
            <span class="btn-icon">⏹️</span>
            Stop
          </button>`
        }
        <button class="btn btn-outline btn-sm" onclick="deleteRunner('${runner.id}')">
          <span class="btn-icon">🗑️</span>
          Delete
        </button>
      </div>
    </div>
    `;
  }).join('');
}

// Render error state
function renderRunnersError(error) {
  const container = document.querySelector("#runners-list");
  container.innerHTML = `
    <div class="empty-state">
      <h3>Failed to load runners</h3>
      <p>Error: ${error}</p>
    </div>
  `;
}

// Create a new runner
async function createRunner() {
  try {
    const runnerName = document.querySelector("#new-runner-name").value.trim();
    if (!runnerName) {
      alert("Please enter a runner name");
      return;
    }

    // Convert relative path to full path
    const relativePath = document.querySelector("#new-db-path").value;
    const fullPath = await invoke("get_full_path", { relativePath });

    const config = {
      name: runnerName,
      db_path: fullPath,
      max_concurrent_tasks: parseInt(document.querySelector("#new-max-tasks").value),
      enable_cron_scheduling: true,
      enable_registry_reconciler: true,
      // Advanced configuration options
      cron_poll_interval: parseInt(document.querySelector("#cron-poll-interval").value || "30"),
      cron_recovery_interval: parseInt(document.querySelector("#cron-recovery-interval").value || "5"),
      cron_lost_threshold: parseInt(document.querySelector("#cron-lost-threshold").value || "10"),
      registry_reconcile_interval: parseInt(document.querySelector("#registry-reconcile-interval").value || "60"),
      executor_poll_interval: parseInt(document.querySelector("#executor-poll-interval").value || "100"),
      scheduler_poll_interval: parseInt(document.querySelector("#scheduler-poll-interval").value || "100"),
      task_timeout: parseInt(document.querySelector("#task-timeout").value || "5"),
    };

    await invoke("create_runner", { config });

    // Clear form and hide it
    document.querySelector("#new-runner-name").value = "";
    document.querySelector("#create-runner-form").classList.add("hidden");
    await loadRunners();

    // Update app status
    await refreshAppStatus();
  } catch (error) {
    alert(`Failed to create runner: ${error}`);
  }
}

// Start a specific runner
async function startRunner(runnerId) {
  try {
    await invoke("start_local_runner", { runnerId: runnerId });
    await loadRunners();
    await refreshAppStatus();
  } catch (error) {
    alert(`Failed to start runner: ${error}`);
  }
}

// Stop a specific runner
async function stopRunner(runnerId) {
  console.log(`Attempting to stop runner: ${runnerId}`);
  try {
    const result = await invoke("stop_local_runner", { runnerId: runnerId });
    console.log("Stop runner result:", result);
    await loadRunners();
    await refreshAppStatus();
  } catch (error) {
    console.error("Stop runner error:", error);
    alert(`Failed to stop runner: ${error}`);
  }
}

// Delete a runner
async function deleteRunner(runnerId) {
  console.log(`deleteRunner called with: ${runnerId}`);

  // Show custom confirmation modal
  const confirmed = await showDeleteConfirmation();

  if (!confirmed.delete) {
    console.log("Delete cancelled by user");
    return;
  }

  console.log(`Confirmed - attempting to delete runner: ${runnerId}. Delete database: ${confirmed.deleteDatabase}`);
  try {
    await invoke("delete_runner", {
      runnerId: runnerId,
      deleteDatabase: confirmed.deleteDatabase
    });
    console.log("Runner deleted successfully");
    await loadRunners();
    await refreshAppStatus();
  } catch (error) {
    console.error("Delete runner error:", error);
    alert(`Failed to delete runner: ${error}`);
  }
}

// Show custom delete confirmation modal
function showDeleteConfirmation() {
  return new Promise((resolve) => {
    // Create modal HTML
    const modalHTML = `
      <div id="delete-modal" class="modal-overlay">
        <div class="modal-content">
          <div class="modal-header">
            <h3>🗑️ Delete Runner</h3>
          </div>
          <div class="modal-body">
            <p>Are you sure you want to delete this runner configuration?</p>
            <div class="checkbox-item">
              <label>
                <input type="checkbox" id="delete-database-checkbox">
                Also delete the database file
              </label>
            </div>
          </div>
          <div class="modal-footer">
            <button id="confirm-delete-btn" class="btn btn-danger">Delete</button>
            <button id="cancel-delete-btn" class="btn btn-outline">Cancel</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to DOM
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    // Add event listeners
    document.getElementById('confirm-delete-btn').addEventListener('click', () => {
      const deleteDatabase = document.getElementById('delete-database-checkbox').checked;
      document.getElementById('delete-modal').remove();
      resolve({ delete: true, deleteDatabase });
    });

    document.getElementById('cancel-delete-btn').addEventListener('click', () => {
      document.getElementById('delete-modal').remove();
      resolve({ delete: false, deleteDatabase: false });
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

// Refresh app status
async function refreshAppStatus() {
  try {
    const status = await invoke("initialize_app");
    updateAppStatus(status);
  } catch (error) {
    console.error("Failed to refresh app status:", error);
  }
}

// Navigation handling
function switchView(viewName) {
  // Update navigation
  document.querySelectorAll('.nav-item').forEach(item => {
    item.classList.remove('active');
  });
  document.querySelector(`[data-view="${viewName}"]`).classList.add('active');

  // Update views
  document.querySelectorAll('.view').forEach(view => {
    view.classList.remove('active');
  });
  document.querySelector(`#${viewName}-view`).classList.add('active');

  currentView = viewName;
}

// Legacy test functions
async function testCloacina() {
  try {
    const result = await invoke("test_cloacina");
    document.querySelector("#cloacina-msg").textContent = result;
  } catch (error) {
    document.querySelector("#cloacina-msg").textContent = `Error: ${error}`;
  }
}

function greet() {
  const greetInputEl = document.querySelector("#greet-input");
  const greetMsgEl = document.querySelector("#greet-msg");
  if (greetInputEl && greetMsgEl) {
    invoke("greet", { name: greetInputEl.value }).then(result => {
      greetMsgEl.textContent = result;
    });
  }
}

// Toggle advanced configuration section
function toggleAdvancedConfig() {
  const content = document.querySelector("#advanced-config-content");
  const icon = document.querySelector("#advanced-expand-icon");

  if (content.style.display === "none" || !content.style.display) {
    content.style.display = "block";
    content.classList.add("expanded");
    icon.classList.add("expanded");
  } else {
    content.style.display = "none";
    content.classList.remove("expanded");
    icon.classList.remove("expanded");
  }
}

// Make functions globally accessible for onclick handlers
window.startRunner = startRunner;
window.stopRunner = stopRunner;
window.deleteRunner = deleteRunner;
window.toggleAdvancedConfig = toggleAdvancedConfig;

// Load settings
async function loadSettings() {
  try {
    const settings = await invoke("get_settings");
    document.querySelector("#data-directory").value = settings.data_directory;
    document.querySelector("#log-level").value = settings.log_level;
    document.querySelector("#max-log-files").value = settings.max_log_files;
  } catch (error) {
    console.error("Failed to load settings:", error);
  }
}

// Save settings
async function saveSettings() {
  try {
    const dataDirectory = document.querySelector("#data-directory").value.trim();
    const logLevel = document.querySelector("#log-level").value;
    const maxLogFiles = parseInt(document.querySelector("#max-log-files").value);

    if (!dataDirectory) {
      alert("Please enter a valid data directory path");
      return;
    }

    // Get current settings to compare
    const currentSettings = await invoke("get_settings");

    // For now, just save the settings - data migration can be implemented later
    // Check if data directory changed
    if (currentSettings.data_directory !== dataDirectory) {
      const confirmed = confirm(
        `Change data directory to: ${dataDirectory}\n\n` +
        "This will update all application paths.\n" +
        "You may need to restart the application.\n\n" +
        "Do you want to continue?"
      );

      if (!confirmed) {
        return;
      }
    }

    // Save all settings with updated paths
    const settings = {
      data_directory: dataDirectory,
      app_database_path: `${dataDirectory}/cloacina-app.db`,
      log_directory: `${dataDirectory}/logs`,
      log_level: logLevel,
      max_log_files: maxLogFiles,
    };

    await invoke("save_settings", { settings });

    // Reload logging configuration
    await invoke("reload_logging_config");

    alert("Settings saved successfully!\n\nLog level changes have been applied immediately.");
  } catch (error) {
    console.error("Failed to save settings:", error);
    alert(`Failed to save settings: ${error}`);
  }
}

// Reset settings to defaults
async function resetSettings() {
  try {
    const defaultDataDir = await invoke("get_data_directory");
    document.querySelector("#data-directory").value = defaultDataDir;

    // Reset other settings to defaults
    document.querySelector("#log-level").value = "info";
    document.querySelector("#max-log-files").value = "10";
  } catch (error) {
    console.error("Failed to reset settings:", error);
    alert(`Failed to reset settings: ${error}`);
  }
}


// Open log directory in system file manager
async function viewLogFiles() {
  try {
    await invoke("open_log_directory");
  } catch (error) {
    console.error("Failed to open log directory:", error);
    alert(`Failed to open log directory: ${error}`);
  }
}

// Full system reset with confirmation
async function fullSystemReset() {
  try {
    // Generate confirmation string
    const confirmationString = await invoke("generate_reset_confirmation");

    // Show custom confirmation modal
    const confirmed = await showFullResetConfirmation(confirmationString);

    if (!confirmed) {
      return;
    }

    // Show progress message
    alert("Performing system reset...\n\nThe application will restart automatically.");

    // Perform the reset
    await invoke("full_system_reset");

    // The application should restart automatically after this point
    // In development mode, you may need to manually restart if auto-restart fails
  } catch (error) {
    console.error("Failed to perform full system reset:", error);
    alert(`Failed to perform full system reset: ${error}`);
  }
}

// Show full reset confirmation modal
function showFullResetConfirmation(confirmationString) {
  return new Promise((resolve) => {
    // Create modal HTML
    const modalHTML = `
      <div id="reset-modal" class="modal-overlay">
        <div class="modal-content">
          <div class="modal-header">
            <h3>Full System Reset</h3>
          </div>
          <div class="modal-body">
            <p><strong>WARNING: This action cannot be undone!</strong></p>
            <p>This will permanently delete:</p>
            <ul>
              <li>All runner configurations</li>
              <li>Application database</li>
              <li>All log files</li>
              <li>Application settings</li>
            </ul>
            <p>To confirm this action, type the following code:</p>
            <div class="confirmation-code">${confirmationString}</div>
            <input type="text" id="confirmation-input" placeholder="Enter confirmation code" />
          </div>
          <div class="modal-footer">
            <button id="confirm-reset-btn" class="btn btn-danger" disabled>Reset Everything</button>
            <button id="cancel-reset-btn" class="btn btn-outline">Cancel</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to DOM
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    const confirmInput = document.getElementById('confirmation-input');
    const confirmBtn = document.getElementById('confirm-reset-btn');

    // Enable/disable confirm button based on input
    confirmInput.addEventListener('input', () => {
      confirmBtn.disabled = confirmInput.value.trim() !== confirmationString;
    });

    // Add event listeners
    confirmBtn.addEventListener('click', () => {
      if (confirmInput.value.trim() === confirmationString) {
        document.getElementById('reset-modal').remove();
        resolve(true);
      }
    });

    document.getElementById('cancel-reset-btn').addEventListener('click', () => {
      document.getElementById('reset-modal').remove();
      resolve(false);
    });

    // Close on overlay click
    document.getElementById('reset-modal').addEventListener('click', (e) => {
      if (e.target.id === 'reset-modal') {
        document.getElementById('reset-modal').remove();
        resolve(false);
      }
    });

    // Focus the input
    setTimeout(() => confirmInput.focus(), 100);
  });
}

// Change data directory using folder picker
async function changeDataDirectory() {
  try {
    const selectedPath = await invoke("select_database_folder");

    if (selectedPath) {
      // Remove the database filename to get just the folder
      const folderPath = selectedPath.replace("/cloacina-app.db", "");
      document.querySelector("#data-directory").value = folderPath;
    }
    // If selectedPath is null, user cancelled - do nothing
  } catch (error) {
    console.error("Failed to select folder:", error);
    alert(`Failed to open folder picker: ${error}`);
  }
}

// Window load event
window.addEventListener("DOMContentLoaded", () => {
  // Initialize app
  initializeApp();

  // Load settings when app starts
  loadSettings();

  // Navigation listeners
  document.querySelectorAll('.nav-item').forEach(item => {
    item.addEventListener('click', () => {
      const viewName = item.getAttribute('data-view');
      switchView(viewName);
    });
  });

  // Function to update database path based on runner name
  async function updateDatabasePath() {
    const runnerName = document.querySelector("#new-runner-name").value.trim();
    const dbPathInput = document.querySelector("#new-db-path");

    if (runnerName) {
      try {
        const dbPath = await invoke("get_runner_db_path", { runnerName });
        dbPathInput.value = dbPath;
      } catch (error) {
        console.error("Failed to get runner db path:", error);
      }
    } else {
      // Clear or set placeholder when no name
      dbPathInput.value = "";
    }
  }

  // Runner management listeners
  document.querySelector("#create-runner-btn").addEventListener("click", async () => {
    document.querySelector("#create-runner-form").classList.remove("hidden");
    // Set initial database path if there's a default name
    await updateDatabasePath();
  });

  // Update database path when runner name changes
  document.querySelector("#new-runner-name").addEventListener("input", updateDatabasePath);

  document.querySelector("#save-runner-btn").addEventListener("click", createRunner);

  document.querySelector("#cancel-runner-btn").addEventListener("click", () => {
    document.querySelector("#create-runner-form").classList.add("hidden");
  });

  // Settings listeners
  document.querySelector("#change-data-dir-btn").addEventListener("click", changeDataDirectory);
  document.querySelector("#view-logs-btn").addEventListener("click", viewLogFiles);
  document.querySelector("#save-settings-btn").addEventListener("click", saveSettings);
  document.querySelector("#reset-settings-btn").addEventListener("click", resetSettings);
  document.querySelector("#full-reset-btn").addEventListener("click", fullSystemReset);

  // Legacy listeners (for debugging section)
  const testBtn = document.querySelector("#test-cloacina-btn");
  if (testBtn) {
    testBtn.addEventListener("click", testCloacina);
  }

  const greetForm = document.querySelector("#greet-form");
  if (greetForm) {
    greetForm.addEventListener("submit", (e) => {
      e.preventDefault();
      greet();
    });
  }
});
