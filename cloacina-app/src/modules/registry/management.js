// Registry Management Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';
import { FileDialogs } from '../../utils/file-dialogs.js';

export class RegistryManager {
  constructor() {
    this.apiClient = new ApiClient();
    this.fileDialogs = new FileDialogs();
    this.currentRunnerId = null;
    this.workflows = [];
    this.init();
  }

  /**
   * Initialize registry management event listeners
   */
  init() {
    // Back to runners button
    document.querySelector("#back-to-runners-btn")?.addEventListener("click", () => this.goBackToRunners());

    // File selection
    document.querySelector("#select-workflow-file-btn")?.addEventListener("click", () => this.selectWorkflowFile());
    document.querySelector("#workflow-file-input")?.addEventListener("change", (e) => this.handleFileInputChange(e));

    // Register workflow
    document.querySelector("#register-workflow-btn")?.addEventListener("click", () => this.registerWorkflow());

    // Refresh workflows list
    document.querySelector("#refresh-registry-btn")?.addEventListener("click", () => this.loadWorkflows());
  }

  /**
   * Show the registry view for a specific runner
   */
  async showRegistryForRunner(runnerId, runnerName) {
    console.log(`Showing registry for runner: ${runnerId} (${runnerName})`);

    this.currentRunnerId = runnerId;

    // Update the header with runner info
    const runnerInfoElement = document.querySelector("#registry-runner-info");
    if (runnerInfoElement) {
      runnerInfoElement.textContent = `Managing workflows for runner: ${runnerName}`;
    }

    // Load workflows for this runner
    await this.loadWorkflows();
  }

  /**
   * Load workflows for the current runner
   */
  async loadWorkflows() {
    if (!this.currentRunnerId) {
      console.error("No current runner ID set");
      return;
    }

    try {
      console.log(`Loading workflows for runner: ${this.currentRunnerId}`);

      // Call the backend to list workflows for this runner
      const response = await this.apiClient.listWorkflowPackages(this.currentRunnerId);

      if (response.success) {
        this.workflows = response.workflows || [];
        this.renderWorkflowsList();
      } else {
        throw new Error("Failed to load workflows");
      }
    } catch (error) {
      console.error("Failed to load workflows:", error);
      this.renderWorkflowsError(error);
    }
  }

  /**
   * Render the workflows list
   */
  renderWorkflowsList() {
    const container = document.querySelector("#workflows-list");
    if (!container) return;

    if (this.workflows.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <h4>No workflows registered</h4>
          <p>Register your first workflow package to get started</p>
        </div>
      `;
      return;
    }

    container.innerHTML = this.workflows.map(workflow => this.createWorkflowItemHTML(workflow)).join('');
  }

  /**
   * Create HTML for a single workflow item
   */
  createWorkflowItemHTML(workflow) {
    return `
      <div class="workflow-item">
        <div class="workflow-info">
          <div class="workflow-header">
            <div class="workflow-title">${workflow.package_name}</div>
            <div class="workflow-version">v${workflow.version}</div>
          </div>
          <div class="workflow-details">
            ${workflow.description ? `<div class="workflow-description">${workflow.description}</div>` : ''}
            ${workflow.author ? `<div class="workflow-author">by ${workflow.author}</div>` : ''}
            <div class="workflow-metadata">
              <span class="metadata-item">📅 ${new Date(workflow.created_at).toLocaleDateString()}</span>
              <span class="metadata-item">🆔 ${workflow.id}</span>
            </div>
          </div>
        </div>
        <div class="workflow-controls">
          <button class="btn btn-danger btn-sm" onclick="unregisterWorkflow('${workflow.package_name}', '${workflow.version}')">
            <span class="btn-icon">🗑️</span>
            Unregister
          </button>
        </div>
      </div>
    `;
  }

  /**
   * Render workflows error state
   */
  renderWorkflowsError(error) {
    const container = document.querySelector("#workflows-list");
    if (!container) return;

    container.innerHTML = `
      <div class="error-state">
        <h4>Failed to load workflows</h4>
        <p>${error.message || error}</p>
        <button class="btn btn-outline btn-sm" onclick="window.registryManager.loadWorkflows()">
          <span class="btn-icon">🔄</span>
          Try Again
        </button>
      </div>
    `;
  }

  /**
   * Select a workflow file
   */
  async selectWorkflowFile() {
    try {
      const filePath = await this.fileDialogs.selectPackageFile("Select Workflow Package");

      if (filePath) {
        console.log(`Selected workflow file: ${filePath}`);

        // Update UI to show selected file
        const fileNameDisplay = document.querySelector("#selected-file-name");
        if (fileNameDisplay) {
          const fileName = filePath.split('/').pop() || filePath.split('\\').pop();
          fileNameDisplay.textContent = fileName;
        }

        // Store the file path
        this.selectedFilePath = filePath;

        // Enable the register button
        const registerBtn = document.querySelector("#register-workflow-btn");
        if (registerBtn) {
          registerBtn.disabled = false;
        }
      }
    } catch (error) {
      console.error("Failed to select file:", error);
      UiHelpers.showAlert(`Failed to select file: ${error.message}`, 'error');
    }
  }

  /**
   * Handle file input change (for drag & drop or direct input)
   */
  handleFileInputChange(event) {
    const file = event.target.files[0];
    if (file) {
      console.log(`File selected via input: ${file.name}`);

      // Update UI
      const fileNameDisplay = document.querySelector("#selected-file-name");
      if (fileNameDisplay) {
        fileNameDisplay.textContent = file.name;
      }

      // Store the file
      this.selectedFile = file;

      // Enable the register button
      const registerBtn = document.querySelector("#register-workflow-btn");
      if (registerBtn) {
        registerBtn.disabled = false;
      }
    }
  }

  /**
   * Register a workflow package
   */
  async registerWorkflow() {
    if (!this.currentRunnerId) {
      UiHelpers.showAlert("No runner selected", 'error');
      return;
    }

    if (!this.selectedFilePath && !this.selectedFile) {
      UiHelpers.showAlert("Please select a workflow file first", 'error');
      return;
    }

    try {
      console.log(`Registering workflow for runner: ${this.currentRunnerId}`);

      let filePath;
      if (this.selectedFilePath) {
        filePath = this.selectedFilePath;
      } else if (this.selectedFile) {
        // For file input, we need to handle it differently
        // This would require additional backend support for file uploads
        UiHelpers.showAlert("File upload not yet implemented. Please use file selection.", 'error');
        return;
      }

      const response = await this.apiClient.registerWorkflowPackage(this.currentRunnerId, filePath);

      if (response.success) {
        UiHelpers.showAlert("Workflow registered successfully!", 'success');

        // Clear the form
        this.clearForm();

        // Reload workflows
        await this.loadWorkflows();
      } else {
        throw new Error(response.message || "Failed to register workflow");
      }
    } catch (error) {
      console.error("Failed to register workflow:", error);
      UiHelpers.showAlert(`Failed to register workflow: ${error.message}`, 'error');
    }
  }

  /**
   * Unregister a workflow package
   */
  async unregisterWorkflow(packageName, version) {
    if (!this.currentRunnerId) {
      UiHelpers.showAlert("No runner selected", 'error');
      return;
    }

    const confirmed = confirm(`Are you sure you want to unregister ${packageName} v${version}?`);
    if (!confirmed) return;

    try {
      console.log(`Unregistering workflow: ${packageName} v${version} from runner: ${this.currentRunnerId}`);

      const response = await this.apiClient.unregisterWorkflowPackage(this.currentRunnerId, packageName, version);

      if (response.success) {
        UiHelpers.showAlert("Workflow unregistered successfully!", 'success');

        // Reload workflows
        await this.loadWorkflows();
      } else {
        throw new Error(response.message || "Failed to unregister workflow");
      }
    } catch (error) {
      console.error("Failed to unregister workflow:", error);
      UiHelpers.showAlert(`Failed to unregister workflow: ${error.message}`, 'error');
    }
  }

  /**
   * Clear the form
   */
  clearForm() {
    // Clear file selection
    this.selectedFilePath = null;
    this.selectedFile = null;

    // Clear UI
    const fileNameDisplay = document.querySelector("#selected-file-name");
    if (fileNameDisplay) {
      fileNameDisplay.textContent = "";
    }

    const fileInput = document.querySelector("#workflow-file-input");
    if (fileInput) {
      fileInput.value = "";
    }

    // Disable register button
    const registerBtn = document.querySelector("#register-workflow-btn");
    if (registerBtn) {
      registerBtn.disabled = true;
    }
  }

  /**
   * Go back to runners view
   */
  goBackToRunners() {
    // Clear current state
    this.currentRunnerId = null;
    this.workflows = [];
    this.clearForm();

    // Switch back to runners view
    const navigationManager = window.CloacinaApp?.modules?.navigation;
    if (navigationManager) {
      navigationManager.switchView('local-runners');
    }
  }
}

// Make unregisterWorkflow globally accessible for onclick handlers
window.unregisterWorkflow = (packageName, version) => {
  if (window.registryManager) {
    window.registryManager.unregisterWorkflow(packageName, version);
  }
};
