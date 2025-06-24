// Package Inspect Module
import { ApiClient } from '../../utils/api-client.js';
import { UiHelpers } from '../../utils/ui-helpers.js';
import { FileDialogs } from '../../utils/file-dialogs.js';

export class PackageInspectManager {
  constructor() {
    this.apiClient = new ApiClient();
    this.fileDialogs = new FileDialogs();
    this.init();
  }

  /**
   * Initialize inspect package event listeners
   */
  init() {
    // File selection
    document.querySelector("#select-package-btn")?.addEventListener("click", () => this.selectPackageFile());
    document.querySelector("#inspect-package-btn")?.addEventListener("click", () => this.inspectPackage());

    // Form controls
    document.querySelector("#clear-inspect-btn")?.addEventListener("click", () => this.clearInspectForm());
    document.querySelector("#close-package-info")?.addEventListener("click", () => this.closePackageInfo());

    // Package actions
    document.querySelector("#debug-package-btn")?.addEventListener("click", () => this.debugPackageFromInspect());
    document.querySelector("#register-package-btn")?.addEventListener("click", () => this.registerPackageToSystem());
    document.querySelector("#open-package-folder-btn")?.addEventListener("click", () => this.openPackageFolder());

    // Visualization controls
    document.querySelector("#inspect-show-details")?.addEventListener("change", (e) => {
      this.toggleInspectVisualizationDetails(e.target.checked);
    });

    // Close inspect details panel
    document.querySelector("#inspect-close-details-btn")?.addEventListener("click", () => {
      this.closeInspectTaskDetails();
    });
  }

  /**
   * Select package file for inspection
   */
  async selectPackageFile() {
    try {
      const selectedPath = await this.apiClient.selectFileDialog({
        title: "Select Cloacina Package File to Inspect",
        filters: [
          { name: "Cloacina Package", extensions: ["cloacina"] },
          { name: "All Files", extensions: ["*"] }
        ]
      });

      if (selectedPath) {
        document.querySelector("#inspect-package-path").value = selectedPath;
        UiHelpers.setButtonState("#inspect-package-btn", false);
      }
    } catch (error) {
      console.error("Failed to select package file:", error);
      console.log(`Failed to open file dialog: ${error}`);
    }
  }

  /**
   * Inspect the selected package
   */
  async inspectPackage() {
    const packagePath = document.querySelector("#inspect-package-path").value;

    if (!packagePath) {
      console.log("Please select a package file first");
      return;
    }

    try {
      const request = {
        package_path: packagePath,
        format: "json"
      };
      const result = await this.apiClient.inspectPackage(request);

      if (result.success && result.manifest) {
        this.displayPackageInfo(result.manifest);

        // Load visualization
        const tasksArray = result.manifest.tasks || [];
        console.log("Tasks found:", tasksArray.length);
        if (tasksArray.length > 0) {
          console.log("Calling displayInspectVisualization...");
          this.displayInspectVisualization();
        } else {
          console.log("No tasks found, hiding visualization");
          // Hide visualization if no tasks
          UiHelpers.hide("#inspect-visualization-section");
        }
      } else {
        console.log(`Failed to inspect package: ${result.error || "Unknown error"}`);
      }
    } catch (error) {
      console.error("Failed to inspect package:", error);
      console.log(`Failed to inspect package: ${error}`);
    }
  }

  /**
   * Display package information
   */
  displayPackageInfo(manifest) {
    // Show the package info section
    UiHelpers.show("#package-info-section");

    // Update package details
    UiHelpers.setText("#package-name", manifest.package?.name || "Unknown");

    // Display workflow fingerprint instead of version
    const versionEl = document.querySelector("#package-version");
    if (versionEl && manifest.package?.workflow_fingerprint) {
      versionEl.innerHTML = `<span class="btn btn-outline btn-sm fingerprint-display">${manifest.package.workflow_fingerprint}</span>`;
    } else if (versionEl) {
      versionEl.textContent = "Unknown";
    }

    UiHelpers.setText("#package-description", manifest.package?.description || "No description");
    UiHelpers.setText("#package-authors", manifest.package?.author || "Unknown");
    UiHelpers.setText("#package-cloacina-version", manifest.package?.cloacina_version || "Unknown");
    UiHelpers.setText("#package-architecture", manifest.library?.architecture || "Unknown");

    // Task count
    const taskCount = manifest.tasks?.length || 0;
    UiHelpers.setText("#task-count", taskCount.toString());

    // Show validation status
    this.displayValidationStatus(manifest);
  }

  /**
   * Display package validation status
   */
  displayValidationStatus(manifest) {
    const validationEl = document.querySelector("#package-validation");
    if (!validationEl) return;

    // Simple validation - check if we have required fields
    const isValid = manifest.package?.name &&
                   manifest.library?.filename &&
                   manifest.tasks && manifest.tasks.length > 0;

    if (isValid) {
      validationEl.innerHTML = '<span class="validation-status valid">✓ Valid Package</span>';
    } else {
      validationEl.innerHTML = '<span class="validation-status invalid">⚠ Invalid Package</span>';
    }
  }

  /**
   * Display visualization for inspect view
   */
  async displayInspectVisualization(graphData) {
    try {
      console.log("displayInspectVisualization called");
      // Get the current package path from the input field
      const packagePath = document.querySelector("#inspect-package-path").value.trim();

      if (!packagePath) {
        console.error("No package path available for visualization");
        return;
      }

      console.log("Calling visualize_package for:", packagePath);
      // Call the backend visualization function
      const request = {
        package_path: packagePath,
        layout: "hierarchical",
        details: document.querySelector("#inspect-show-details").checked,
        format: "json"
      };

      const result = await this.apiClient.invoke("visualize_package", { request });
      console.log("Visualize result:", result);

      if (result.success && result.graph_data) {
        // Store graph data for the inspect view
        window.inspectGraphData = result.graph_data;

        // Display the visualization
        this.displayInspectWorkflowVisualization(result.graph_data);

        // Show the visualization section
        UiHelpers.show("#inspect-visualization-section");

      } else {
        console.error("Failed to generate visualization:", result.error);
        // Hide visualization section on error
        UiHelpers.hide("#inspect-visualization-section");
      }
    } catch (error) {
      console.error("Error generating inspect visualization:", error);
      UiHelpers.hide("#inspect-visualization-section");
    }
  }

  /**
   * Display workflow visualization in the inspect view (moved from main.js)
   */
  displayInspectWorkflowVisualization(graphData) {
    console.log("Displaying inspect visualization with data:", graphData);

    // Clear previous visualization
    d3.select("#inspect-workflow-svg").selectAll("*").remove();

    if (!graphData.nodes || graphData.nodes.length === 0) {
      d3.select("#inspect-workflow-svg")
        .append("text")
        .attr("x", "50%")
        .attr("y", "50%")
        .attr("text-anchor", "middle")
        .attr("fill", "var(--text-secondary)")
        .text("No tasks found in this workflow");
      return;
    }

    const svg = d3.select("#inspect-workflow-svg");
    const container = document.querySelector("#inspect-graph-container");
    const width = container.clientWidth;
    const height = 400; // Fixed height for inspect view

    svg.attr("width", width).attr("height", height);

    // Create a new directed graph
    const g = new dagreD3.graphlib.Graph()
      .setGraph({
        nodesep: 50,
        ranksep: 60,
        rankdir: "LR",
        marginx: 20,
        marginy: 20
      })
      .setDefaultEdgeLabel(() => ({}));

    // Add nodes to the graph
    graphData.nodes.forEach(node => {
      const showDetails = document.querySelector("#inspect-show-details")?.checked;

      g.setNode(node.id, {
        labelType: "html",
        label: `
          <div class="dagre-node" style="
            background: var(--primary);
            color: white;
            border-radius: 8px;
            padding: 8px 12px;
            text-align: center;
            min-width: 80px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
          ">
            <div style="font-weight: 500; font-size: 11px;">${node.label}</div>
            ${showDetails && node.description ?
              `<div style="font-size: 9px; opacity: 0.8; margin-top: 2px;">${node.description.length > 30 ? node.description.substring(0, 30) + '...' : node.description}</div>` :
              ''}
          </div>
        `,
        width: 100,
        height: showDetails && node.description ? 50 : 35,
        data: node
      });
    });

    // Add edges to the graph
    graphData.edges.forEach(edge => {
      g.setEdge(edge.source, edge.target, {
        style: "stroke: var(--text-secondary); stroke-width: 2px; fill: none;",
        arrowheadStyle: "fill: var(--text-secondary);"
      });
    });

    // Create the renderer
    const render = new dagreD3.render();
    const svgGroup = svg.append("g");

    // Run the renderer
    render(svgGroup, g);

    // Add click handlers to nodes for inspect view
    svgGroup.selectAll(".node")
      .on("click", (d) => {
        const nodeId = d;
        const nodeData = g.node(nodeId);
        if (nodeData && nodeData.data) {
          this.showInspectTaskDetails(d3.event, nodeData.data);
        }
      });

    // Center and scale the graph for the smaller inspect view
    const graphBounds = svgGroup.node().getBBox();
    const padding = 20;
    const scale = Math.min(
      (width - padding) / graphBounds.width,
      (height - padding) / graphBounds.height,
      1
    );

    const centerX = width / 2;
    const centerY = height / 2;
    const translateX = centerX - (graphBounds.width * scale) / 2;
    const translateY = centerY - (graphBounds.height * scale) / 2;

    svgGroup.attr("transform", `translate(${translateX}, ${translateY}) scale(${scale})`);

    // Add basic zoom for inspect view
    const zoom = d3.zoom()
      .scaleExtent([0.2, 2])
      .on("zoom", function() {
        svgGroup.attr("transform", d3.event.transform);
      });

    svg.call(zoom);

    // Set initial transform
    svg.call(zoom.transform, d3.zoomIdentity.translate(translateX, translateY).scale(scale));
  }

  /**
   * Show task details in inspect view (moved from main.js)
   */
  showInspectTaskDetails(event, nodeData) {
    console.log("Showing inspect task details for:", nodeData);

    // Find task data from inspect graph data
    const taskData = window.inspectGraphData.nodes.find(n => n.id === nodeData.id) || nodeData;

    // Show the task details panel
    const taskDetailsPanel = document.querySelector("#inspect-task-details-panel");
    taskDetailsPanel.classList.remove("hidden");

    // Apply proper positioning for inspect view
    taskDetailsPanel.style.position = "sticky";
    taskDetailsPanel.style.top = "20px";
    taskDetailsPanel.style.right = "0px";
    taskDetailsPanel.style.zIndex = "10";

    // Populate details panel
    document.querySelector("#inspect-detail-task-id").textContent = taskData.id;
    document.querySelector("#inspect-detail-task-description").textContent = taskData.description || "No description";

    // Show dependencies
    const dependenciesContainer = document.querySelector("#inspect-detail-task-dependencies");
    if (window.inspectGraphData && window.inspectGraphData.edges) {
      const dependencies = window.inspectGraphData.edges
        .filter(edge => edge.target === taskData.id)
        .map(edge => edge.source);

      if (dependencies.length > 0) {
        dependenciesContainer.innerHTML = dependencies
          .map(dep => `<span class="dependency-tag">${dep}</span>`)
          .join("");
      } else {
        dependenciesContainer.innerHTML = '<span class="dependency-tag">No dependencies</span>';
      }
    } else {
      dependenciesContainer.innerHTML = '<span class="dependency-tag">Dependencies unavailable</span>';
    }

    document.querySelector("#inspect-detail-task-source").textContent = "N/A";
  }

  /**
   * Close inspect task details panel (moved from main.js)
   */
  closeInspectTaskDetails() {
    document.querySelector("#inspect-task-details-panel").classList.add("hidden");
  }

  /**
   * Toggle visualization details
   */
  toggleInspectVisualizationDetails(showDetails) {
    // Re-render visualization with/without details
    const visualization = document.querySelector("#inspect-visualization-section");
    if (visualization && !visualization.classList.contains("hidden")) {
      // Get current graph data and re-render
      // This would need to store the graph data somewhere accessible
      console.log("Toggle visualization details:", showDetails);
    }
  }

  /**
   * Clear inspect form
   */
  clearInspectForm() {
    UiHelpers.setText("#inspect-package-path", "");
    UiHelpers.setButtonState("#inspect-package-btn", true);
    UiHelpers.hide("#package-info-section");
    UiHelpers.hide("#inspect-visualization-section");
    this.closeInspectTaskDetails();
  }

  /**
   * Close package info section
   */
  closePackageInfo() {
    UiHelpers.hide("#package-info-section");
    UiHelpers.hide("#inspect-visualization-section");
    this.closeInspectTaskDetails();
  }

  /**
   * Debug package from inspect view
   */
  debugPackageFromInspect() {
    const inspectPackagePath = document.querySelector("#inspect-package-path").value;
    if (inspectPackagePath) {
      // Dispatch event to navigate to debug with package path
      const event = new CustomEvent('navigateToDebug', {
        detail: { packagePath: inspectPackagePath }
      });
      document.dispatchEvent(event);
    } else {
      // Just navigate to debug view
      const event = new CustomEvent('navigateToDebug', { detail: {} });
      document.dispatchEvent(event);
    }
  }

  /**
   * Register package to system
   */
  async registerPackageToSystem() {
    const packagePath = document.querySelector("#inspect-package-path").value;
    if (!packagePath) {
      console.log("No package path available for registration");
      return;
    }

    try {
      // Get all available runners first
      const runnersResult = await this.apiClient.getLocalRunners();
      if (!runnersResult || runnersResult.length === 0) {
        console.log("No runners available. Please create a runner first.");
        return;
      }

      // For now, use the first available runner
      // In the future, could show a runner selection dialog
      const runner = runnersResult[0];

      console.log(`Registering package to runner: ${runner.config.name} (${runner.id})`);

      const result = await this.apiClient.invoke("register_workflow_package", {
        runner_id: runner.id,
        package_path: packagePath
      });

      if (result.success) {
        console.log(`Package registered successfully to runner: ${runner.config.name}`);
        console.log(`Package ID: ${result.package_id}`);
      } else {
        console.log(`Failed to register package: ${result.message || "Unknown error"}`);
      }
    } catch (error) {
      console.error("Failed to register package:", error);
      console.log(`Failed to register package: ${error}`);
    }
  }

  /**
   * Open package folder
   */
  async openPackageFolder() {
    const packagePath = document.querySelector("#inspect-package-path").value;
    if (!packagePath) {
      console.log("No package path available");
      return;
    }

    try {
      // Get directory path from file path
      const directoryPath = packagePath.substring(0, packagePath.lastIndexOf('/'));
      await this.apiClient.openPath(directoryPath);
    } catch (error) {
      console.error("Failed to open package folder:", error);
      console.log(`Failed to open package location: ${error}`);
    }
  }

}
