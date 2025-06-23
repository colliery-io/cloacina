// Package Build Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';
import { FileDialogs } from '../../utils/file-dialogs.js';
import { BUILD_PROFILES } from '../../constants/app-constants.js';

export class PackageBuildManager {
  constructor() {
    this.apiClient = new ApiClient();
    this.fileDialogs = new FileDialogs();
    this.init();
  }

  /**
   * Initialize build package event listeners
   */
  init() {
    // Form submission
    document.querySelector("#build-package-form")?.addEventListener("submit", (e) => {
      e.preventDefault();
      this.buildPackage();
    });

    // File selection
    document.querySelector("#select-project-btn")?.addEventListener("click", () => this.selectProjectDirectory());
    document.querySelector("#select-output-btn")?.addEventListener("click", () => this.selectOutputPath());

    // Form controls
    document.querySelector("#clear-build-form-btn")?.addEventListener("click", () => this.clearBuildForm());
    document.querySelector("#build-advanced-toggle")?.addEventListener("click", () => this.toggleBuildAdvanced());

    // Output controls
    document.querySelector("#close-build-output")?.addEventListener("click", () => this.closeBuildOutput());
    document.querySelector("#open-package-location")?.addEventListener("click", () => this.openPackageLocation());
    document.querySelector("#inspect-built-package")?.addEventListener("click", () => this.inspectBuiltPackage());
  }

  /**
   * Build package from form data
   */
  async buildPackage() {
    console.log("buildPackage() called");
    const request = this.collectBuildRequest();
    console.log("Build request:", request);

    if (!this.validateBuildRequest(request)) {
      console.log("Build request validation failed");
      return;
    }

    try {
      console.log("Starting build...");
      // Show output section and clear previous content
      UiHelpers.show("#build-output-section");
      UiHelpers.setText("#build-output", "Starting package build...\n");
      UiHelpers.setText("#build-status", "Building...");
      UiHelpers.hide("#build-actions");

      console.log("Calling API buildPackage...");
      const result = await this.apiClient.buildPackage(request);
      console.log("Build result:", result);

      if (result.success) {
        this.handleBuildSuccess(result);
      } else {
        this.handleBuildError(result.error);
      }
    } catch (error) {
      console.error("Failed to build package:", error);
      this.handleBuildError(error.toString());
    }
  }

  /**
   * Collect build request from form
   */
  collectBuildRequest() {
    const cargoFlags = document.querySelector("#cargo-flags")?.value.trim() || "";
    const targetTriple = document.querySelector("#target-triple")?.value || "";

    // Parse cargo flags into array like original code
    const cargoFlagsArray = cargoFlags ? cargoFlags.split(/\s+/).filter(f => f.length > 0) : [];

    return {
      project_path: document.querySelector("#project-path")?.value.trim() || "",
      output_path: document.querySelector("#output-path")?.value.trim() || "",
      profile: document.querySelector("#build-profile")?.value || BUILD_PROFILES.DEBUG,
      target: targetTriple || null,
      cargo_flags: cargoFlagsArray
    };
  }

  /**
   * Validate build request
   */
  validateBuildRequest(request) {
    if (!request.project_path) {
      console.log("Please select a project directory");
      return false;
    }

    if (!request.output_path) {
      console.log("Please specify an output path");
      return false;
    }

    if (!Object.values(BUILD_PROFILES).includes(request.profile)) {
      console.log("Invalid build profile selected");
      return false;
    }

    return true;
  }

  /**
   * Handle successful build
   */
  handleBuildSuccess(result) {
    UiHelpers.setText("#build-status", "Build completed successfully!");
    UiHelpers.setText("#build-output", result.output || "Package built successfully.");
    UiHelpers.show("#build-actions");

    // Store output path for actions
    this.lastBuiltPackagePath = result.package_path || document.querySelector("#output-path")?.value;
  }

  /**
   * Handle build error
   */
  handleBuildError(error) {
    UiHelpers.setText("#build-status", "Build failed!");
    UiHelpers.setText("#build-output", `Build Error:\n${error}`);
    UiHelpers.hide("#build-actions");
  }

  /**
   * Select project directory
   */
  async selectProjectDirectory() {
    try {
      const selectedPath = await this.apiClient.selectDirectoryDialog({
        title: "Select Rust Project Directory"
      });

      if (selectedPath) {
        document.querySelector("#project-path").value = selectedPath;

        // Auto-generate output path if not set (like original code)
        const outputPathInput = document.querySelector("#output-path");
        if (!outputPathInput.value.trim()) {
          const projectName = selectedPath.split('/').pop() || 'workflow';
          try {
            const desktopPath = await this.apiClient.getDesktopPath();
            outputPathInput.value = `${desktopPath}/${projectName}.cloacina`;
          } catch (error) {
            console.error("Failed to get desktop path:", error);
            // Fallback to project directory
            outputPathInput.value = `${selectedPath}/${projectName}.cloacina`;
          }
        }
      }
    } catch (error) {
      console.error("Failed to select project directory:", error);
      console.log(`Failed to open directory dialog: ${error}`);
    }
  }

  /**
   * Update output path based on project path
   */
  updateOutputPathFromProject(projectPath) {
    const projectName = UiHelpers.extractFilename(projectPath);
    const outputPath = `${projectPath}/${projectName}.cloacina`;
    UiHelpers.setText("#output-path", outputPath);
  }

  /**
   * Select output path
   */
  async selectOutputPath() {
    try {
      const projectPath = document.querySelector("#project-path")?.value;
      const defaultName = projectPath ? `${UiHelpers.extractFilename(projectPath)}.cloacina` : "package.cloacina";

      const selectedPath = await this.apiClient.selectFileDialog({
        title: "Select Output Location",
        defaultPath: defaultName,
        filters: [
          { name: "Cloacina Package", extensions: ["cloacina"] },
          { name: "All Files", extensions: ["*"] }
        ]
      });

      if (selectedPath) {
        document.querySelector("#output-path").value = selectedPath;
      }
    } catch (error) {
      console.error("Failed to select output path:", error);
      console.log(`Failed to open file dialog: ${error}`);
    }
  }

  /**
   * Clear build form
   */
  clearBuildForm() {
    UiHelpers.clearForm("#build-package-form");
    UiHelpers.hide("#build-output-section");
    this.lastBuiltPackagePath = null;
  }

  /**
   * Toggle advanced build options
   */
  toggleBuildAdvanced() {
    const content = document.querySelector("#build-advanced-content");
    const icon = document.querySelector("#build-advanced-icon");

    if (content && icon) {
      // Toggle display style directly since it uses inline styling
      if (content.style.display === "none" || content.style.display === "") {
        content.style.display = "block";
        UiHelpers.addClass(icon, "expanded");
        icon.textContent = "▼";
      } else {
        content.style.display = "none";
        UiHelpers.removeClass(icon, "expanded");
        icon.textContent = "▶";
      }
    }
  }

  /**
   * Close build output
   */
  closeBuildOutput() {
    UiHelpers.hide("#build-output-section");
  }

  /**
   * Open package location in file explorer
   */
  async openPackageLocation() {
    if (!this.lastBuiltPackagePath) {
      UiHelpers.showAlert("No package path available");
      return;
    }

    try {
      // Get directory path from file path
      const directoryPath = this.lastBuiltPackagePath.substring(0, this.lastBuiltPackagePath.lastIndexOf('/'));
      await this.apiClient.openPath(directoryPath);
    } catch (error) {
      console.error("Failed to open package location:", error);
      UiHelpers.showAlert(`Failed to open package location: ${error}`);
    }
  }

  /**
   * Navigate to inspect the built package
   */
  inspectBuiltPackage() {
    if (!this.lastBuiltPackagePath) {
      UiHelpers.showAlert("No package path available");
      return;
    }

    // Dispatch custom event to navigate to inspect with package path
    const event = new CustomEvent('navigateToInspect', {
      detail: { packagePath: this.lastBuiltPackagePath }
    });
    document.dispatchEvent(event);
  }
}
