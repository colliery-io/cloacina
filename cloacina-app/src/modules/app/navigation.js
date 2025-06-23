// Navigation Module
import { APP_VIEWS, UI_CLASSES } from '../../constants/app-constants.js';
import { UiHelpers } from '../../utils/ui-helpers.js';

export class NavigationManager {
  constructor() {
    this.currentView = APP_VIEWS.LOCAL_RUNNERS;
    this.init();
  }

  /**
   * Initialize navigation event listeners
   */
  init() {
    // Navigation listeners
    document.querySelectorAll('.nav-item').forEach(item => {
      if (!item.classList.contains('dropdown')) {
        item.addEventListener('click', () => {
          const viewName = item.getAttribute('data-view');
          this.switchView(viewName);
        });
      }
    });

    // Dropdown listeners
    document.querySelectorAll('.dropdown-item').forEach(item => {
      item.addEventListener('click', () => {
        const viewName = item.getAttribute('data-view');
        this.switchView(viewName);

        // Close dropdown after selection
        const dropdown = item.closest('.dropdown');
        if (dropdown) {
          dropdown.classList.remove('active');
        }
      });
    });

    // Packages dropdown toggle
    const packagesDropdown = document.querySelector('#packages-dropdown');
    const packagesButton = packagesDropdown?.querySelector('.nav-button');

    if (packagesButton) {
      packagesButton.addEventListener('click', (e) => {
        e.stopPropagation();
        packagesDropdown.classList.toggle('active');
      });
    }

    // Close dropdown when clicking outside
    document.addEventListener('click', (e) => {
      if (packagesDropdown && !packagesDropdown.contains(e.target)) {
        packagesDropdown.classList.remove('active');
      }
    });
  }

  /**
   * Switch to a different view
   */
  switchView(viewName) {
    if (!viewName || !Object.values(APP_VIEWS).includes(viewName)) {
      console.warn(`Invalid view name: ${viewName}`);
      return;
    }

    // Hide all views
    document.querySelectorAll('.view').forEach(view => {
      view.classList.remove(UI_CLASSES.ACTIVE);
    });

    // Show target view
    const targetView = document.querySelector(`#${viewName}-view`);
    if (targetView) {
      targetView.classList.add(UI_CLASSES.ACTIVE);
    }

    // Update navigation active state
    document.querySelectorAll('.nav-item').forEach(item => {
      item.classList.remove(UI_CLASSES.ACTIVE);
    });

    // Handle dropdown items
    document.querySelectorAll('.dropdown-item').forEach(item => {
      if (item.getAttribute('data-view') === viewName) {
        const dropdown = item.closest('.dropdown');
        if (dropdown) {
          dropdown.querySelector('.nav-button').classList.add(UI_CLASSES.ACTIVE);
        }
      }
    });

    // Handle regular nav items
    const activeNavItem = document.querySelector(`[data-view="${viewName}"]`);
    if (activeNavItem && !activeNavItem.classList.contains('dropdown-item')) {
      activeNavItem.classList.add(UI_CLASSES.ACTIVE);
    }

    this.currentView = viewName;
    console.log(`Switched to view: ${viewName}`);

    // Trigger view-specific actions
    this.handleViewSwitch(viewName);
  }

  /**
   * Handle view-specific actions when switching views
   */
  handleViewSwitch(viewName) {
    // Trigger settings reload when switching to settings view
    if (viewName === APP_VIEWS.SETTINGS) {
      setTimeout(() => {
        const event = new CustomEvent('reloadSettings');
        document.dispatchEvent(event);
      }, 100);
    }
  }

  /**
   * Get current view
   */
  getCurrentView() {
    return this.currentView;
  }

  /**
   * Navigate to specific views with context
   */
  navigateToDebugPackage(packagePath = null) {
    this.switchView(APP_VIEWS.DEBUG_PACKAGE);

    if (packagePath) {
      // Pre-populate the debug package path
      const pathInput = document.querySelector("#debug-package-path");
      const loadButton = document.querySelector("#load-debug-package-btn");

      if (pathInput) pathInput.value = packagePath;
      if (loadButton) loadButton.disabled = false;

      // Trigger auto-load after view switch
      setTimeout(() => {
        const event = new CustomEvent('autoLoadDebugPackage', { detail: { packagePath } });
        document.dispatchEvent(event);
      }, 100);
    }
  }

  navigateToInspectPackage(packagePath = null) {
    this.switchView(APP_VIEWS.INSPECT_PACKAGE);

    if (packagePath) {
      const pathInput = document.querySelector("#inspect-package-path");
      const inspectButton = document.querySelector("#inspect-package-btn");

      if (pathInput) pathInput.value = packagePath;
      if (inspectButton) inspectButton.disabled = false;

      // Trigger auto-load after view switch
      setTimeout(() => {
        const event = new CustomEvent('autoLoadInspectPackage', { detail: { packagePath } });
        document.dispatchEvent(event);
      }, 100);
    }
  }
}
