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

//! The envelope codec: bincode 1.3 with its default options, the same wire the
//! fidius typed boundary uses. A connector guest built on weir's contract and a
//! host built on this one encode every boundary type to the same bytes.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Error encoding or decoding an envelope.
#[derive(Debug)]
pub struct CodecError(pub bincode::Error);

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "envelope codec error: {}", self.0)
    }
}

impl std::error::Error for CodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

impl From<bincode::Error> for CodecError {
    fn from(e: bincode::Error) -> Self {
        Self(e)
    }
}

/// Encode a value to envelope bytes.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    Ok(bincode::serialize(value)?)
}

/// Decode a value from envelope bytes.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
    Ok(bincode::deserialize(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::super::types::{ChangeOp, ChangeRecord, RecordBatch, StreamState};
    use super::*;

    #[test]
    fn stream_state_bytes_are_pinned() {
        // bincode 1.3 default options: Option tag u8, lengths u64 little-endian,
        // strings and byte vectors verbatim. These bytes are the wire weir's
        // guests already speak; a change here is a contract break.
        let state = StreamState {
            cursor: Some("1".into()),
            opaque: vec![7],
        };
        let bytes = encode(&state).unwrap();
        assert_eq!(
            bytes,
            vec![1, 1, 0, 0, 0, 0, 0, 0, 0, b'1', 1, 0, 0, 0, 0, 0, 0, 0, 7]
        );
        let back: StreamState = decode(&bytes).unwrap();
        assert_eq!(back, state);
    }

    #[test]
    fn record_batch_round_trips() {
        let batch = RecordBatch::Changes(vec![ChangeRecord {
            op: ChangeOp::Delete,
            data: r#"{"id":3}"#.into(),
        }]);
        let bytes = encode(&batch).unwrap();
        // enum variant index u32 LE (Changes = 2), then the vec.
        assert_eq!(&bytes[..4], &[2, 0, 0, 0]);
        let back: RecordBatch = decode(&bytes).unwrap();
        assert_eq!(back, batch);
    }

    #[test]
    fn truncated_bytes_fail() {
        let bytes = encode(&StreamState::default()).unwrap();
        assert!(decode::<StreamState>(&bytes[..bytes.len() - 1]).is_err());
    }
}
