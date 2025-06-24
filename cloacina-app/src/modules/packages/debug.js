// Debug Package Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';
import { FileDialogs } from '../../utils/file-dialogs.js';
import { EXECUTION_STATUS } from '../../constants/app-constants.js';

export class DebugPackageManager {
  constructor() {
    this.apiClient = new ApiClient();
    this.fileDialogs = new FileDialogs();
    this.currentPackage = null;
    this.selectedTask = null;
    this.envVarCounter = 0;
    this.init();
  }

  /**
   * Initialize debug package event listeners
   */
  init() {
    // File selection and loading
    document.querySelector("#select-debug-package-btn")?.addEventListener("click", () => this.selectPackageFile());
    document.querySelector("#load-debug-package-btn")?.addEventListener("click", () => this.loadPackage());

    // Task execution
    document.querySelector("#execute-task-btn")?.addEventListener("click", () => this.executeSelectedTask());

    // Form controls
    document.querySelector("#clear-debug-btn")?.addEventListener("click", () => this.clearForm());
    document.querySelector("#close-debug-info")?.addEventListener("click", () => this.closeDebugInfo());
    document.querySelector("#add-env-var-btn")?.addEventListener("click", () => this.addEnvironmentVariable());
    document.querySelector("#clear-execution-btn")?.addEventListener("click", () => this.clearExecutionPanel());
    document.querySelector("#copy-execution-output-btn")?.addEventListener("click", () => this.copyExecutionOutput());

    // Auto-load event listener
    document.addEventListener('autoLoadDebugPackage', (event) => {
      this.loadPackage();
    });
  }

  /**
   * Select debug package file
   */
  async selectPackageFile() {
    try {
      const selectedPath = await this.apiClient.selectFileDialog({
        title: "Select Cloacina Package File for Debug",
        filters: [
          { name: "Cloacina Package", extensions: ["cloacina"] },
          { name: "All Files", extensions: ["*"] }
        ]
      });

      if (selectedPath) {
        document.querySelector("#debug-package-path").value = selectedPath;
        UiHelpers.setButtonState("#load-debug-package-btn", false);
      }
    } catch (error) {
      console.error("Failed to select debug package file:", error);
      UiHelpers.showAlert(`Failed to open file dialog: ${error}`);
    }
  }

  /**
   * Load debug package and list tasks
   */
  async loadPackage() {
    const packagePath = document.querySelector("#debug-package-path").value;

    if (!packagePath) {
      UiHelpers.showAlert("Please select a package file first");
      return;
    }

    try {
      const request = {
        package_path: packagePath,
        task_identifier: null, // null means list tasks
        context: null,
        env_vars: null
      };

      const result = await this.apiClient.debugPackage(request);

      if (result.success && result.tasks) {
        this.currentPackage = {
          path: packagePath,
          tasks: result.tasks
        };

        this.displayPackageInfo(result.tasks);
        this.clearSelectedTask();
      } else {
        UiHelpers.showAlert(`Failed to load package: ${result.error || "Unknown error"}`);
      }
    } catch (error) {
      console.error("Failed to load debug package:", error);
      UiHelpers.showAlert(`Failed to load package: ${error}`);
    }
  }

  /**
   * Display debug package information and task list
   */
  displayPackageInfo(tasks) {
    // Show the debug info section
    UiHelpers.show("#debug-info-section");

    // Extract package name from path
    const packageName = UiHelpers.extractPackageName(this.currentPackage.path);
    UiHelpers.setText("#debug-package-name", packageName);

    // Clear and populate task list
    const taskList = document.querySelector("#debug-task-list");
    taskList.innerHTML = '';

    if (tasks.length === 0) {
      taskList.innerHTML = '<div class="empty-state"><p>No tasks found in this package</p></div>';
      return;
    }

    tasks.forEach((task) => {
      const taskItem = UiHelpers.createElement('div', 'debug-task-item');
      taskItem.dataset.taskIndex = task.index;
      taskItem.dataset.taskId = task.id;

      taskItem.innerHTML = `
        <div class="task-item-header">
          <span class="task-item-name">${task.id}</span>
          <span class="task-item-index">#${task.index}</span>
        </div>
        <div class="task-item-description">${task.description || 'No description'}</div>
      `;

      taskItem.addEventListener('click', () => this.selectTaskForExecution(task, taskItem));
      taskList.appendChild(taskItem);
    });
  }

  /**
   * Select a task for execution
   */
  selectTaskForExecution(task, taskElement) {
    // Clear previous selection
    UiHelpers.removeClassFromAll('.debug-task-item', 'selected');

    // Select current task
    taskElement.classList.add('selected');
    this.selectedTask = task;

    // Show selected task info
    UiHelpers.show("#selected-task-info");
    UiHelpers.setText("#selected-task-name", task.id);
    UiHelpers.setText("#selected-task-description", task.description || 'No description');

    // Show execution configuration
    UiHelpers.show("#execution-config");

    // Enable execute button
    UiHelpers.setButtonState("#execute-task-btn", false);

    // Clear previous execution output
    this.clearExecutionOutput();
  }

  /**
   * Execute the selected task
   */
  async executeSelectedTask() {
    if (!this.selectedTask) {
      UiHelpers.showAlert("Please select a task to execute");
      return;
    }

    const contextText = document.querySelector("#execution-context").value.trim();
    let contextJson = null;

    // Validate context JSON
    if (contextText) {
      if (!UiHelpers.isValidJSON(contextText)) {
        UiHelpers.showAlert("Invalid JSON in context data");
        return;
      }
      contextJson = JSON.parse(contextText);
    }

    // Collect environment variables
    const envVars = this.collectEnvironmentVariables();

    // Show execution status
    this.showExecutionStatus(EXECUTION_STATUS.RUNNING);

    try {
      const request = {
        package_path: this.currentPackage.path,
        task_identifier: this.selectedTask.id,
        context: contextJson ? JSON.stringify(contextJson) : null,
        env_vars: envVars.length > 0 ? envVars : null
      };

      const result = await this.apiClient.debugPackage(request);

      if (result.success) {
        this.showExecutionStatus(EXECUTION_STATUS.SUCCESS);
        this.displayExecutionOutput(result.output || "Task executed successfully");
      } else {
        this.showExecutionStatus(EXECUTION_STATUS.ERROR);
        this.displayExecutionOutput(result.error || "Unknown execution error");
      }
    } catch (error) {
      console.error("Failed to execute task:", error);
      this.showExecutionStatus(EXECUTION_STATUS.ERROR);
      this.displayExecutionOutput(`Execution failed: ${error}`);
    }
  }

  /**
   * Collect environment variables from form
   */
  collectEnvironmentVariables() {
    const envVars = [];
    document.querySelectorAll('.env-var-item').forEach(item => {
      const keyInput = item.querySelector('input[placeholder="KEY"]');
      const valueInput = item.querySelector('input[placeholder="VALUE"]');
      if (keyInput?.value.trim() && valueInput?.value.trim()) {
        envVars.push(`${keyInput.value.trim()}=${valueInput.value.trim()}`);
      }
    });
    return envVars;
  }

  /**
   * Show execution status
   */
  showExecutionStatus(status) {
    const statusElement = document.querySelector("#execution-status");
    statusElement.className = `execution-status ${status}`;

    const statusText = {
      [EXECUTION_STATUS.READY]: 'Ready',
      [EXECUTION_STATUS.RUNNING]: 'Running...',
      [EXECUTION_STATUS.SUCCESS]: 'Success',
      [EXECUTION_STATUS.ERROR]: 'Error'
    };

    UiHelpers.setText("#execution-status", statusText[status] || 'Unknown');
    UiHelpers.show("#execution-output-section");
  }

  /**
   * Display execution output
   */
  displayExecutionOutput(output) {
    UiHelpers.setText("#execution-output", output);
  }

  /**
   * Clear execution output
   */
  clearExecutionOutput() {
    UiHelpers.hide("#execution-output-section");
    this.showExecutionStatus(EXECUTION_STATUS.READY);
    UiHelpers.setText("#execution-output", '');
  }

  /**
   * Clear selected task
   */
  clearSelectedTask() {
    this.selectedTask = null;
    UiHelpers.removeClassFromAll('.debug-task-item', 'selected');
    UiHelpers.hide("#selected-task-info");
    UiHelpers.hide("#execution-config");
    UiHelpers.setButtonState("#execute-task-btn", true);
    this.clearExecutionOutput();
  }

  /**
   * Add environment variable input
   */
  addEnvironmentVariable() {
    const envVarsList = document.querySelector("#env-vars-list");
    const envVarItem = UiHelpers.createElement('div', 'env-var-item');
    envVarItem.dataset.envId = `env-${this.envVarCounter++}`;

    envVarItem.innerHTML = `
      <input type="text" class="env-var-input" placeholder="KEY">
      <span>=</span>
      <input type="text" class="env-var-input" placeholder="VALUE">
      <button type="button" class="env-var-remove-btn" onclick="removeEnvironmentVariable(this)">✕</button>
    `;

    envVarsList.appendChild(envVarItem);
  }

  /**
   * Clear debug form
   */
  clearForm() {
    UiHelpers.setText("#debug-package-path", '');
    UiHelpers.setButtonState("#load-debug-package-btn", true);
    UiHelpers.hide("#debug-info-section");
    UiHelpers.setText("#execution-context", '{}');
    UiHelpers.setHTML("#env-vars-list", '');
    this.clearSelectedTask();
    this.currentPackage = null;
    this.envVarCounter = 0;
  }

  /**
   * Close debug info section
   */
  closeDebugInfo() {
    UiHelpers.hide("#debug-info-section");
    this.clearSelectedTask();
  }

  /**
   * Clear execution panel
   */
  clearExecutionPanel() {
    UiHelpers.setText("#execution-context", '{}');
    UiHelpers.setHTML("#env-vars-list", '');
    this.clearExecutionOutput();
    this.envVarCounter = 0;
  }

  /**
   * Auto-load package from external navigation
   */
  autoLoadPackage(packagePath) {
    UiHelpers.setText("#debug-package-path", packagePath);
    UiHelpers.setButtonState("#load-debug-package-btn", false);
    this.loadPackage();
  }

  /**
   * Copy execution output to clipboard
   */
  async copyExecutionOutput() {
    try {
      const outputElement = document.querySelector("#execution-output");
      const outputText = outputElement.textContent || outputElement.innerText || '';

      if (!outputText.trim()) {
        UiHelpers.showAlert("No output to copy");
        return;
      }

      // Use the Clipboard API if available
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(outputText);
        UiHelpers.showAlert("Output copied to clipboard");
      } else {
        // Fallback for older browsers
        const textArea = document.createElement('textarea');
        textArea.value = outputText;
        textArea.style.position = 'fixed';
        textArea.style.opacity = '0';
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand('copy');
        document.body.removeChild(textArea);
        UiHelpers.showAlert("Output copied to clipboard");
      }
    } catch (error) {
      console.error("Failed to copy output:", error);
      UiHelpers.showAlert("Failed to copy output to clipboard");
    }
  }
}
