---
title: "cloacina::security"
description: "Security module for package signing and key management."
editUrl: false
sidebar:
  order: 187
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Security module for package signing and key management.

This module provides:
- [`KeyManager`](../../../rust/cloacina/security/key_manager/#trait.KeyManager) trait for managing signing keys and trust relationships
- [`DbKeyManager`](../../../rust/cloacina/security/db_key_manager/#struct.DbKeyManager) database-backed implementation
- Key generation, encryption, and PEM export/import
- Security audit logging for SIEM integration

## Re-exports

- [`DbKeyManager`](../../../rust/cloacina/security/db_key_manager/#struct.DbKeyManager)
- [`AgentKeyPool`](../../../rust/cloacina/security/fleet_secret/#struct.AgentKeyPool)
- [`FleetSecretError`](../../../rust/cloacina/security/fleet_secret/#enum.FleetSecretError)
- [`InMemorySecretResolver`](../../../rust/cloacina/security/fleet_secret/#struct.InMemorySecretResolver)
- [`ServerKeyPool`](../../../rust/cloacina/security/fleet_secret/#struct.ServerKeyPool)
- [`decode_pool_public_key`](../../../rust/cloacina/security/fleet_secret/#fn.decode_pool_public_key)
- [`resolve_and_wrap_secrets`](../../../rust/cloacina/security/fleet_secret/#fn.resolve_and_wrap_secrets)
- [`secret_aad`](../../../rust/cloacina/security/fleet_secret/#fn.secret_aad)
- [`secret_ref_names`](../../../rust/cloacina/security/fleet_secret/#fn.secret_ref_names)
- [`wrap_field_map`](../../../rust/cloacina/security/fleet_secret/#fn.wrap_field_map)
- [`KeyError`](../../../rust/cloacina/security/key_manager/#enum.KeyError)
- [`KeyManager`](../../../rust/cloacina/security/key_manager/#trait.KeyManager)
- [`PublicKeyExport`](../../../rust/cloacina/security/key_manager/#struct.PublicKeyExport)
- [`SigningKeyInfo`](../../../rust/cloacina/security/key_manager/#struct.SigningKeyInfo)
- [`TrustedKeyInfo`](../../../rust/cloacina/security/key_manager/#struct.TrustedKeyInfo)
- [`DbPackageSigner`](../../../rust/cloacina/security/package_signer/#struct.DbPackageSigner)
- [`DetachedSignature`](../../../rust/cloacina/security/package_signer/#struct.DetachedSignature)
- [`PackageSignError`](../../../rust/cloacina/security/package_signer/#enum.PackageSignError)
- [`PackageSignatureInfo`](../../../rust/cloacina/security/package_signer/#struct.PackageSignatureInfo)
- [`PackageSigner`](../../../rust/cloacina/security/package_signer/#trait.PackageSigner)
- [`KEK_ENV_VAR`](../../../rust/cloacina/security/secret_resolver/#const.KEK_ENV_VAR)
- [`SecretAllow`](../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow)
- [`SecretResolverConfigError`](../../../rust/cloacina/security/secret_resolver/#enum.SecretResolverConfigError)
- [`SecretStoreResolver`](../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver)
- [`SecretError`](../../../rust/cloacina/security/secret_store/#enum.SecretError)
- [`SecretMetadata`](../../../rust/cloacina/security/secret_store/#struct.SecretMetadata)
- [`SecretStore`](../../../rust/cloacina/security/secret_store/#struct.SecretStore)
- [`SecurityConfig`](../../../rust/cloacina/security/verification/#struct.SecurityConfig)
- [`SignatureSource`](../../../rust/cloacina/security/verification/#enum.SignatureSource)
- [`VerificationError`](../../../rust/cloacina/security/verification/#enum.VerificationError)
- [`VerificationResult`](../../../rust/cloacina/security/verification/#struct.VerificationResult)
- [`verify_package`](../../../rust/cloacina/security/verification/#fn.verify_package)
- [`verify_package_bytes`](../../../rust/cloacina/security/verification/#fn.verify_package_bytes)
- [`verify_package_offline`](../../../rust/cloacina/security/verification/#fn.verify_package_offline)
