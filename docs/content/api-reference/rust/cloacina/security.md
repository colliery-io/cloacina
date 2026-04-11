# cloacina::security <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Security module for package signing and key management.

This module provides:
- [`KeyManager`] trait for managing signing keys and trust relationships
- [`DbKeyManager`] database-backed implementation
- Key generation, encryption, and PEM export/import
- Security audit logging for SIEM integration
