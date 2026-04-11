# cloacina::registry::storage <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Storage backend implementations for the workflow registry.

This module provides storage backends for persisting workflow binaries:
- `UnifiedRegistryStorage` - Database storage (PostgreSQL or SQLite)
- `FilesystemRegistryStorage` - Filesystem-based storage
