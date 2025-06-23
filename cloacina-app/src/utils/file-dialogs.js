// File Dialog Utilities
import { ApiClient } from './api-client.js';
import { FILE_FILTERS } from '../constants/app-constants.js';

export class FileDialogs {
  constructor() {
    this.apiClient = new ApiClient();
  }

  /**
   * Select a Cloacina package file
   */
  async selectPackageFile(title = "Select Cloacina Package File") {
    try {
      return await this.apiClient.selectFileDialog({
        title,
        filters: FILE_FILTERS.CLOACINA_PACKAGE
      });
    } catch (error) {
      console.error("Failed to open package file dialog:", error);
      throw new Error(`Failed to open file dialog: ${error}`);
    }
  }

  /**
   * Select a directory
   */
  async selectDirectory(title = "Select Directory") {
    try {
      return await this.apiClient.selectDirectoryDialog({
        title,
        ...FILE_FILTERS.DIRECTORY
      });
    } catch (error) {
      console.error("Failed to open directory dialog:", error);
      throw new Error(`Failed to open directory dialog: ${error}`);
    }
  }

  /**
   * Select output file location
   */
  async selectOutputFile(title = "Select Output Location", defaultName = "") {
    try {
      return await this.apiClient.selectFileDialog({
        title,
        defaultPath: defaultName,
        filters: FILE_FILTERS.CLOACINA_PACKAGE
      });
    } catch (error) {
      console.error("Failed to open output file dialog:", error);
      throw new Error(`Failed to open file dialog: ${error}`);
    }
  }

  /**
   * Select Rust project directory
   */
  async selectRustProject() {
    return this.selectDirectory("Select Rust Project Directory");
  }

  /**
   * Select package file for inspection
   */
  async selectPackageForInspection() {
    return this.selectPackageFile("Select Cloacina Package File to Inspect");
  }

  /**
   * Select package file for debugging
   */
  async selectPackageForDebug() {
    return this.selectPackageFile("Select Cloacina Package File for Debug");
  }
}
