/*
 *  Copyright 2025-2026 Colliery Software
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

//! Every connector guest carries a copy of the Sync boundary types with
//! `WitType` derives, because `fidius_build::emit_wit` reads only the guest's
//! own source tree (COLLIERY-I-0632 / CLOACINA-T-0979). This test keeps every
//! copy byte-identical to the canonical file `guest/sync_guest_types.rs` in
//! this crate, and checks that the canonical file names every boundary type
//! the host side declares. Guests are found by glob over the two guest roots,
//! so a new guest cannot escape the check.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn canonical() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("guest/sync_guest_types.rs"))
        .expect("the canonical guest types file")
}

fn guest_copies() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in ["examples/constructor-contract", "connectors"] {
        let dir = repo_root().join(root);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let copy = entry.path().join("src/sync_guest_types.rs");
            if copy.is_file() {
                out.push(copy);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_guest_copy_matches_the_canonical_guest_types() {
    let canonical = canonical();
    let copies = guest_copies();
    assert!(
        copies.len() >= 3,
        "expected the echo, faulty and postgres guests at least; found {copies:?}"
    );
    for copy in copies {
        let text = std::fs::read_to_string(&copy).unwrap();
        assert!(
            text == canonical,
            "{} differs from crates/cloacina-constructor-contract/guest/sync_guest_types.rs; \
             copy the canonical file over it (never edit a guest's copy by hand)",
            copy.display()
        );
    }
}

#[test]
fn the_canonical_file_names_every_boundary_type() {
    let canonical = canonical();
    for ty in [
        "Config",
        "ArrowSchemaIpc",
        "ArrowIpc",
        "SyncMode",
        "ConnectorRole",
        "ConnectorSpec",
        "CheckResult",
        "PartitionScheme",
        "StreamInfo",
        "Catalog",
        "ErrorKind",
        "ContextPair",
        "ConnectorError",
        "DiscoverOutcome",
        "WriteMode",
        "MappingOp",
        "CastType",
        "CompareOp",
        "ComputeExpr",
        "MappingSpec",
        "ConfiguredStream",
        "Partition",
        "StreamState",
        "RecordBatch",
        "ChangeOp",
        "ChangeRecord",
        "LogLevel",
        "LogEntry",
        "DeadLetter",
        "ReadContext",
        "ReadMessage",
        "WriteContext",
        "WriteReceipt",
        "WriteResult",
        "WriteOutcome",
    ] {
        assert!(
            canonical.contains(&format!("pub struct {ty}"))
                || canonical.contains(&format!("pub enum {ty}")),
            "canonical guest types lack `{ty}`"
        );
    }
}
