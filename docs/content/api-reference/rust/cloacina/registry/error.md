# cloacina::registry::error <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Error types for the workflow registry system.

This module defines the various error conditions that can occur during
registry operations, providing detailed error information for debugging
and user feedback.

## Enums

### `cloacina::registry::error::RegistryError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Main error type for registry operations.

This enum covers all the error conditions that can occur when working
with the workflow registry, from validation failures to storage errors.

#### Variants

- **`PackageExists`** - A workflow package with the same name and version already exists.
- **`PackageNotFound`** - The requested workflow package was not found.
- **`PackageInUse`** - The workflow package cannot be unregistered because it's in use.
- **`ValidationError`** - Package validation failed.
- **`MetadataExtractionError`** - Metadata extraction from package failed.
- **`TaskRegistrationError`** - Task registration failed.
- **`RegistrationFailed`** - Registry operation failed.
- **`Storage`** - Storage operation failed.
- **`Database`** - Database operation failed.
- **`Io`** - I/O operation failed.
- **`Serialization`** - Serialization/deserialization failed.
- **`InvalidUuid`** - UUID parsing failed.
- **`Loader`** - Package loading failed.
- **`Internal`** - Generic internal error.



### `cloacina::registry::error::StorageError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Error type for storage backend operations.

This enum covers errors specific to the binary storage layer,
whether using PostgreSQL, object storage, or filesystem backends.

#### Variants

- **`ConnectionFailed`** - Connection to storage backend failed.
- **`Timeout`** - Storage operation timed out.
- **`QuotaExceeded`** - Storage backend is full.
- **`DataCorruption`** - Data corruption detected.
- **`InvalidId`** - Invalid storage identifier.
- **`Backend`** - Generic storage backend error.
- **`Database`** - Database error from Diesel operations.



### `cloacina::registry::error::LoaderError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Error type for package loading and metadata extraction operations.

This enum covers errors specific to loading .so files, extracting metadata,
and validating package integrity.

#### Variants

- **`TempDirectory`** - Failed to create or access temporary directory.
- **`LibraryLoad`** - Failed to load dynamic library.
- **`SymbolNotFound`** - Required symbol not found in library.
- **`MetadataExtraction`** - Metadata extraction failed.
- **`FileSystem`** - File system operation failed.
- **`Validation`** - Package validation failed.
- **`TaskRegistration`** - Task registration failed.
- **`WrongLanguage`** - Wrong package language for loader.
- **`MissingPythonConfig`** - Missing Python configuration in manifest.
- **`MissingManifest`** - Missing manifest.json in archive.
- **`ManifestParse`** - Manifest parse error.
- **`MissingSourceDir`** - Missing source directory in extracted package.
