---
title: "cloacina::crypto"
description: "Cryptographic utilities for package signing."
editUrl: false
sidebar:
  order: 58
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Cryptographic utilities for package signing.

This module provides:
- Ed25519 key generation and signing
- AES-256-GCM encryption for private key storage at rest
- Key fingerprint computation

## Re-exports

- [`EnvelopeError`](../../../rust/cloacina/crypto/envelope/#enum.EnvelopeError)
- [`EphemeralKeypair`](../../../rust/cloacina/crypto/envelope/#struct.EphemeralKeypair)
- [`EphemeralPrivateKey`](../../../rust/cloacina/crypto/envelope/#struct.EphemeralPrivateKey)
- [`generate_ephemeral_keypair`](../../../rust/cloacina/crypto/envelope/#fn.generate_ephemeral_keypair)
- [`envelope_unwrap`](../../../rust/cloacina/crypto/envelope/#fn.unwrap) (`envelope::unwrap`)
- [`envelope_wrap`](../../../rust/cloacina/crypto/envelope/#fn.wrap) (`envelope::wrap`)
- [`KeyEncryptionError`](../../../rust/cloacina/crypto/key_encryption/#enum.KeyEncryptionError)
- [`decrypt_bytes`](../../../rust/cloacina/crypto/key_encryption/#fn.decrypt_bytes)
- [`decrypt_private_key`](../../../rust/cloacina/crypto/key_encryption/#fn.decrypt_private_key)
- [`encrypt_bytes`](../../../rust/cloacina/crypto/key_encryption/#fn.encrypt_bytes)
- [`encrypt_private_key`](../../../rust/cloacina/crypto/key_encryption/#fn.encrypt_private_key)
- [`GeneratedKeypair`](../../../rust/cloacina/crypto/signing/#struct.GeneratedKeypair)
- [`SigningError`](../../../rust/cloacina/crypto/signing/#enum.SigningError)
- [`compute_key_fingerprint`](../../../rust/cloacina/crypto/signing/#fn.compute_key_fingerprint)
- [`generate_signing_keypair`](../../../rust/cloacina/crypto/signing/#fn.generate_signing_keypair)
- [`sign_package`](../../../rust/cloacina/crypto/signing/#fn.sign_package)
- [`verify_signature`](../../../rust/cloacina/crypto/signing/#fn.verify_signature)
