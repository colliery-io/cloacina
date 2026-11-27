/*
 *  Copyright 2026 Colliery Software
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

//! Package signing on the publisher's machine (CLOACINA-T-1062,
//! CLOACINA-A-0018): `package pack --sign`, `package publish --sign` and the
//! signature that `package upload` sends.
//!
//! The key is a file on this machine: an unencrypted Ed25519 private key in
//! PKCS#8 PEM, as `openssl genpkey -algorithm ed25519` writes it. The archive
//! is signed here. Only the archive and its signature go to the server; the
//! private key does not leave this process, and no message holds it.

use std::path::{Path, PathBuf};

use cloacina::security::DetachedSignature;

use crate::shared::error::CliError;

/// The path of the signature file of an archive: the archive's path with
/// `.sig` added (`demo.cloacina` → `demo.cloacina.sig`).
pub fn signature_path(archive: &Path) -> PathBuf {
    let mut name = archive.as_os_str().to_os_string();
    name.push(".sig");
    PathBuf::from(name)
}

/// Read the private key of `--sign`: the 32-byte Ed25519 seed.
///
/// On Unix a key file that group or others can read is refused, as `ssh`
/// refuses one: the file's permissions are what protects the key.
fn load_signing_key(path: &Path) -> Result<Vec<u8>, CliError> {
    let metadata = std::fs::metadata(path).map_err(|e| {
        CliError::UserError(format!(
            "cannot read the signing key {}: {e}",
            path.display()
        ))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(CliError::UserError(format!(
                "the signing key {} can be read by group or others (mode {mode:o}). \
                 Run `chmod 600 {}` and sign again.",
                path.display(),
                path.display()
            )));
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    let pem = std::fs::read_to_string(path).map_err(|e| {
        CliError::UserError(format!(
            "cannot read the signing key {}: {e}",
            path.display()
        ))
    })?;
    cloacina::crypto::decode_private_key_pem(&pem).map_err(|e| {
        CliError::UserError(format!(
            "{} is not a signing key this command reads: {e}. It reads an unencrypted \
             Ed25519 private key in PKCS#8 PEM (`openssl genpkey -algorithm ed25519`).",
            path.display()
        ))
    })
}

/// Check that the key file of `--sign` can be used, before a long build.
pub fn check_signing_key(key_file: &Path) -> Result<(), CliError> {
    load_signing_key(key_file).map(|_| ())
}

/// Sign the archive with the key file. Ed25519 over the SHA-256 of the
/// archive: the signature the server and `cloacina::security` verify.
pub fn sign_archive(archive: &Path, key_file: &Path) -> Result<DetachedSignature, CliError> {
    let private_key = load_signing_key(key_file)?;
    let bytes = std::fs::read(archive).map_err(CliError::Io)?;
    DetachedSignature::sign(&bytes, &private_key)
        .map_err(|e| CliError::UserError(format!("signing failed: {e}")))
}

/// Sign the archive and write the signature file beside it. Returns the path
/// of the signature file.
pub fn sign_archive_to_file(archive: &Path, key_file: &Path) -> Result<PathBuf, CliError> {
    let signature = sign_archive(archive, key_file)?;
    let path = signature_path(archive);
    signature
        .write_to_file(&path)
        .map_err(|e| CliError::UserError(format!("cannot write {}: {e}", path.display())))?;
    Ok(path)
}

/// The signature an upload sends.
pub enum UploadSignature {
    /// The signature file beside the archive (`<archive>.sig`), when there
    /// is one; else none.
    Sidecar,
    /// This signature file.
    File(PathBuf),
    /// A signature made in this process (`publish --sign`).
    Made(DetachedSignature),
}

impl UploadSignature {
    /// The text of the `signature` field for the archive, or `None` for an
    /// upload with no signature. A signature file that was made for other
    /// bytes is refused here: the server would refuse it too.
    pub fn resolve(self, archive: &Path, archive_bytes: &[u8]) -> Result<Option<String>, CliError> {
        let (signature, source) = match self {
            UploadSignature::Made(signature) => (signature, "the signature".to_string()),
            UploadSignature::File(path) => (read_signature(&path)?, path.display().to_string()),
            UploadSignature::Sidecar => {
                let path = signature_path(archive);
                if !path.exists() {
                    return Ok(None);
                }
                eprintln!("sending the signature {}", path.display());
                (read_signature(&path)?, path.display().to_string())
            }
        };
        if !signature.is_for(archive_bytes) {
            return Err(CliError::UserError(format!(
                "{source} was made for another archive: its hash is not the hash of {}. \
                 Sign the archive again, or remove the signature file to upload with none.",
                archive.display()
            )));
        }
        signature
            .to_json()
            .map(Some)
            .map_err(|e| CliError::UserError(format!("{source}: {e}")))
    }
}

fn read_signature(path: &Path) -> Result<DetachedSignature, CliError> {
    DetachedSignature::read_from_file(path).map_err(|e| {
        CliError::UserError(format!("{} is not a signature file: {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina::crypto::{GeneratedKeypair, encode_private_key_pem, generate_signing_keypair};
    use tempfile::TempDir;

    /// Writes a new test key to a file that only the owner can read.
    fn key_file(dir: &Path) -> (PathBuf, GeneratedKeypair) {
        let keypair = generate_signing_keypair();
        let path = dir.join("signing-key.pem");
        std::fs::write(&path, encode_private_key_pem(&keypair.private_key).unwrap()).unwrap();
        set_mode(&path, 0o600);
        (path, keypair)
    }

    fn set_mode(path: &Path, mode: u32) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        #[cfg(not(unix))]
        let _ = (path, mode);
    }

    /// `package pack --sign` on a scaffolded package: the archive, and beside
    /// it a signature that verifies against the archive with the public key
    /// of the key file. The `signature` field that `package upload` then
    /// sends is that signature, and the library that the server verifies
    /// with reads it and accepts it.
    #[test]
    fn pack_sign_writes_a_signature_that_upload_sends_and_the_library_verifies() {
        let tmp = TempDir::new().unwrap();
        let (key, keypair) = key_file(tmp.path());
        let source = tmp.path().join("t1062-demo");
        super::super::new::run(
            "t1062-demo",
            super::super::new::ScaffoldLang::Python,
            super::super::new::ScaffoldKind::Workflow,
            Some(&source),
        )
        .expect("scaffold a package");
        let archive = tmp.path().join("t1062-demo.cloacina");

        super::super::pack::run(&source, Some(&archive), Some(&key)).expect("pack --sign");

        let signature_file = signature_path(&archive);
        assert_eq!(
            signature_file.file_name().unwrap().to_string_lossy(),
            "t1062-demo.cloacina.sig"
        );
        // The check a holder of the public key makes.
        let verified = cloacina::security::verify_package_offline(
            &archive,
            &signature_file,
            &keypair.public_key,
        )
        .expect("the signature verifies against the archive");
        assert_eq!(verified.signer_fingerprint, keypair.fingerprint);

        // What the upload sends: the `signature` field, from the file beside
        // the archive. The server parses the field with the same type.
        let bytes = std::fs::read(&archive).unwrap();
        let field = UploadSignature::Sidecar
            .resolve(&archive, &bytes)
            .expect("resolve")
            .expect("a signature is sent");
        let sent = DetachedSignature::from_json(&field).expect("the field is a signature");
        assert_eq!(sent.key_fingerprint, keypair.fingerprint);
        assert_eq!(sent.algorithm, DetachedSignature::ALGORITHM);
        assert!(sent.is_for(&bytes));
        let field_file = tmp.path().join("field.sig");
        std::fs::write(&field_file, &field).unwrap();
        cloacina::security::verify_package_offline(&archive, &field_file, &keypair.public_key)
            .expect("the signature that is sent verifies");

        // `publish --sign` signs the archive it built the same way.
        let made = sign_archive(&archive, &key).expect("sign");
        assert_eq!(made.package_hash, sent.package_hash);
        assert_eq!(made.signature, sent.signature, "Ed25519 is deterministic");
        let field_of_publish = UploadSignature::Made(made)
            .resolve(&archive, &bytes)
            .unwrap()
            .unwrap();
        assert!(DetachedSignature::from_json(&field_of_publish).is_ok());
    }

    #[test]
    fn an_archive_with_no_signature_file_sends_no_signature() {
        let tmp = TempDir::new().unwrap();
        let archive = tmp.path().join("plain.cloacina");
        std::fs::write(&archive, b"t1062 archive bytes").unwrap();
        let sent = UploadSignature::Sidecar
            .resolve(&archive, b"t1062 archive bytes")
            .unwrap();
        assert!(sent.is_none());
    }

    #[test]
    fn a_signature_file_of_another_archive_is_refused_before_the_upload() {
        let tmp = TempDir::new().unwrap();
        let (key, _) = key_file(tmp.path());
        let archive = tmp.path().join("one.cloacina");
        std::fs::write(&archive, b"t1062 first bytes").unwrap();
        let signature_file = sign_archive_to_file(&archive, &key).unwrap();

        // The archive changes after it was signed.
        std::fs::write(&archive, b"t1062 changed bytes").unwrap();
        for signature in [
            UploadSignature::Sidecar,
            UploadSignature::File(signature_file.clone()),
        ] {
            let err = signature
                .resolve(&archive, b"t1062 changed bytes")
                .unwrap_err();
            assert!(format!("{err:?}").contains("another archive"), "{err:?}");
        }

        // A file that is no signature.
        let junk = tmp.path().join("junk.sig");
        std::fs::write(&junk, "not a signature").unwrap();
        let err = UploadSignature::File(junk)
            .resolve(&archive, b"t1062 changed bytes")
            .unwrap_err();
        assert!(
            format!("{err:?}").contains("not a signature file"),
            "{err:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_key_file_that_others_can_read_is_refused() {
        let tmp = TempDir::new().unwrap();
        let (key, _) = key_file(tmp.path());
        let archive = tmp.path().join("one.cloacina");
        std::fs::write(&archive, b"t1062 bytes").unwrap();

        for mode in [0o644, 0o640, 0o604] {
            set_mode(&key, mode);
            let err = sign_archive(&archive, &key).unwrap_err();
            let text = format!("{err:?}");
            assert!(text.contains("chmod 600"), "{mode:o}: {text}");
        }
        set_mode(&key, 0o400);
        sign_archive(&archive, &key).expect("a key only the owner reads signs");
    }

    #[test]
    fn a_file_that_is_no_key_is_refused() {
        let tmp = TempDir::new().unwrap();
        let archive = tmp.path().join("one.cloacina");
        std::fs::write(&archive, b"t1062 bytes").unwrap();

        // A missing file.
        let err = sign_archive(&archive, &tmp.path().join("absent.pem")).unwrap_err();
        assert!(format!("{err:?}").contains("cannot read"), "{err:?}");

        // A file that is no private key.
        let not_a_key = tmp.path().join("other.pem");
        std::fs::write(&not_a_key, "t1062 not a key").unwrap();
        set_mode(&not_a_key, 0o600);
        let err = sign_archive(&archive, &not_a_key).unwrap_err();
        let text = format!("{err:?}");
        assert!(text.contains("PKCS#8"), "{text}");
        assert!(!text.contains("t1062 not a key"), "{text}");
    }
}
