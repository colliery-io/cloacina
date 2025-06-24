// UI Helper Functions
import { UI_CLASSES } from '../constants/app-constants.js';

export class UiHelpers {
  /**
   * Show an element by removing hidden class
   */
  static show(selector) {
    const element = document.querySelector(selector);
    if (element) {
      element.classList.remove(UI_CLASSES.HIDDEN);
    }
    return element;
  }

  /**
   * Hide an element by adding hidden class
   */
  static hide(selector) {
    const element = document.querySelector(selector);
    if (element) {
      element.classList.add(UI_CLASSES.HIDDEN);
    }
    return element;
  }

  /**
   * Toggle visibility of an element
   */
  static toggle(selector) {
    const element = document.querySelector(selector);
    if (element) {
      element.classList.toggle(UI_CLASSES.HIDDEN);
    }
    return element;
  }

  /**
   * Clear form inputs in a container
   */
  static clearForm(containerSelector) {
    const container = document.querySelector(containerSelector);
    if (container) {
      // Clear text inputs
      container.querySelectorAll('input[type="text"], input[type="number"], textarea').forEach(input => {
        input.value = '';
      });

      // Reset selects to first option
      container.querySelectorAll('select').forEach(select => {
        select.selectedIndex = 0;
      });

      // Uncheck checkboxes
      container.querySelectorAll('input[type="checkbox"]').forEach(checkbox => {
        checkbox.checked = false;
      });
    }
  }

  /**
   * Set button disabled state
   */
  static setButtonState(selector, disabled) {
    const button = document.querySelector(selector);
    if (button) {
      button.disabled = disabled;
    }
    return button;
  }

  /**
   * Update text content of an element
   */
  static setText(selector, text) {
    const element = document.querySelector(selector);
    if (element) {
      element.textContent = text;
    }
    return element;
  }

  /**
   * Update HTML content of an element
   */
  static setHTML(selector, html) {
    const element = document.querySelector(selector);
    if (element) {
      element.innerHTML = html;
    }
    return element;
  }

  /**
   * Add class to element
   */
  static addClass(selector, className) {
    const element = document.querySelector(selector);
    if (element) {
      element.classList.add(className);
    }
    return element;
  }

  /**
   * Remove class from element
   */
  static removeClass(selector, className) {
    const element = document.querySelector(selector);
    if (element) {
      element.classList.remove(className);
    }
    return element;
  }

  /**
   * Remove class from all elements matching selector
   */
  static removeClassFromAll(selector, className) {
    document.querySelectorAll(selector).forEach(element => {
      element.classList.remove(className);
    });
  }

  /**
   * Show alert message
   */
  static showAlert(message, type = 'info') {
    // For now, use console log since dialogs might have permission issues
    console.log(`[${type.toUpperCase()}] ${message}`);
  }

  /**
   * Show confirmation dialog
   */
  static showConfirm(message) {
    // For now, use console log and return true
    console.log(`[CONFIRM] ${message}`);
    return true; // Default to proceeding for now
  }

  /**
   * Create a simple HTML element
   */
  static createElement(tag, className = '', content = '') {
    const element = document.createElement(tag);
    if (className) element.className = className;
    if (content) element.innerHTML = content;
    return element;
  }

  /**
   * Format file size in human readable format
   */
  static formatFileSize(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  /**
   * Validate JSON string
   */
  static isValidJSON(jsonString) {
    try {
      JSON.parse(jsonString);
      return true;
    } catch (error) {
      return false;
    }
  }

  /**
   * Extract filename from path
   */
  static extractFilename(path) {
    return path.split('/').pop().split('\\').pop();
  }

  /**
   * Extract package name from .cloacina path
   */
  static extractPackageName(path) {
    return this.extractFilename(path).replace('.cloacina', '');
  }

  /**
   * Debounce function calls
   */
  static debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
      const later = () => {
        clearTimeout(timeout);
        func(...args);
      };
      clearTimeout(timeout);
      timeout = setTimeout(later, wait);
    };
  }
}
