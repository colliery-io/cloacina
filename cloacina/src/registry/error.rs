/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Error types for the workflow registry system.
//!
//! This module defines the various error conditions that can occur during
//! registry operations, providing detailed error information for debugging
//! and user feedback.

use thiserror::Error;

/// Main error type for registry operations.
///
/// This enum covers all the error conditions that can occur when working
/// with the workflow registry, from validation failures to storage errors.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// A workflow package with the same name and version already exists.
    #[error("Package already exists: {package_name} v{version}")]
    PackageExists {
        /// Name of the existing package
        package_name: String,
        /// Version that conflicts
        version: String,
    },

    /// The requested workflow package was not found.
    #[error("Package not found: {package_name} v{version}")]
    PackageNotFound {
        /// Name of the missing package
        package_name: String,
        /// Version that was requested
        version: String,
    },

    /// The workflow package cannot be unregistered because it's in use.
    #[error("Package is in use: {package_name} v{version} has {active_count} active executions")]
    PackageInUse {
        /// Name of the package
        package_name: String,
        /// Version of the package
        version: String,
        /// Number of active executions
        active_count: usize,
    },

    /// Package validation failed.
    #[error("Package validation failed: {reason}")]
    ValidationError {
        /// Detailed reason for validation failure
        reason: String,
    },

    /// Metadata extraction from package failed.
    #[error("Failed to extract metadata from package: {reason}")]
    MetadataExtractionError {
        /// Detailed reason for extraction failure
        reason: String,
    },

    /// Task registration failed.
    #[error("Failed to register tasks: {reason}")]
    TaskRegistrationError {
        /// Detailed reason for registration failure
        reason: String,
    },

    /// Registry operation failed.
    #[error("Registration failed: {message}")]
    RegistrationFailed {
        /// Detailed reason for registration failure
        message: String,
    },

    /// Storage operation failed.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(String),

    /// I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// UUID parsing failed.
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    /// Package loading failed.
    #[error("Package loader error: {0}")]
    Loader(#[from] LoaderError),

    /// Generic internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Error type for storage backend operations.
///
/// This enum covers errors specific to the binary storage layer,
/// whether using PostgreSQL, object storage, or filesystem backends.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Connection to storage backend failed.
    #[error("Storage connection failed: {reason}")]
    ConnectionFailed {
        /// Detailed connection error
        reason: String,
    },

    /// Storage operation timed out.
    #[error("Storage operation timed out after {seconds} seconds")]
    Timeout {
        /// Timeout duration in seconds
        seconds: u64,
    },

    /// Storage backend is full.
    #[error("Storage quota exceeded: {used_bytes} / {quota_bytes} bytes used")]
    QuotaExceeded {
        /// Current usage in bytes
        used_bytes: u64,
        /// Maximum quota in bytes
        quota_bytes: u64,
    },

    /// Data corruption detected.
    #[error("Data corruption detected for ID {id}: {reason}")]
    DataCorruption {
        /// ID of corrupted data
        id: String,
        /// Details about the corruption
        reason: String,
    },

    /// Invalid storage identifier.
    #[error("Invalid storage ID: {id}")]
    InvalidId {
        /// The invalid ID
        id: String,
    },

    /// Generic storage backend error.
    #[error("Storage backend error: {0}")]
    Backend(String),

    /// Database error from Diesel operations.
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
}

impl From<String> for RegistryError {
    fn from(s: String) -> Self {
        RegistryError::Internal(s)
    }
}

impl From<String> for StorageError {
    fn from(s: String) -> Self {
        StorageError::Backend(s)
    }
}

/// Error type for package loading and metadata extraction operations.
///
/// This enum covers errors specific to loading .so files, extracting metadata,
/// and validating package integrity.
#[derive(Debug, Error)]
pub enum LoaderError {
    /// Failed to create or access temporary directory.
    #[error("Temporary directory error: {error}")]
    TempDirectory {
        /// Details about the error
        error: String,
    },

    /// Failed to load dynamic library.
    #[error("Failed to load library at {path}: {error}")]
    LibraryLoad {
        /// Path to the library file
        path: String,
        /// Details about the load error
        error: String,
    },

    /// Required symbol not found in library.
    #[error("Symbol '{symbol}' not found: {error}")]
    SymbolNotFound {
        /// Name of the missing symbol
        symbol: String,
        /// Details about the error
        error: String,
    },

    /// Metadata extraction failed.
    #[error("Metadata extraction failed: {reason}")]
    MetadataExtraction {
        /// Reason for extraction failure
        reason: String,
    },

    /// File system operation failed.
    #[error("File system error at {path}: {error}")]
    FileSystem {
        /// Path where the error occurred
        path: String,
        /// Details about the error
        error: String,
    },

    /// Package validation failed.
    #[error("Package validation failed: {reason}")]
    Validation {
        /// Reason for validation failure
        reason: String,
    },

    /// Task registration failed.
    #[error("Task registration failed: {reason}")]
    TaskRegistration {
        /// Reason for registration failure
        reason: String,
    },
}
