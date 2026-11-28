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

//! The end of each task attempt's output (CLOACINA-T-1091).
//!
//! An agent runs a task in a worker process, reads the worker's stdout and
//! stderr as one stream, and sends the last bytes with the result
//! ([`cloacina::fleet::AttemptOutput`]). The server cuts it again to
//! `--task-output-max-bytes` when the report arrives, before the result inbox
//! can store it, and the fleet executor stores one row per attempt in the
//! tenant's schema when the wait of the dispatch takes the report.
//!
//! The output is task data: no log line of the server holds it.

use std::sync::atomic::{AtomicU64, Ordering};

use cloacina::fleet::{AttemptOutput, DEFAULT_TASK_OUTPUT_MAX_BYTES};

static MAX_BYTES: AtomicU64 = AtomicU64::new(DEFAULT_TASK_OUTPUT_MAX_BYTES);

/// Set `--task-output-max-bytes`. `0` keeps no output.
pub fn set_max_bytes(max_bytes: u64) {
    MAX_BYTES.store(max_bytes, Ordering::Relaxed);
}

/// The bytes of the end of an attempt's output the server keeps, as set at
/// start. Also sent to the agent in each work packet.
pub fn max_bytes() -> u64 {
    MAX_BYTES.load(Ordering::Relaxed)
}

/// `output` cut to `max_bytes`, the end kept. `None` with no output, or with
/// a cap of 0.
pub fn cap(output: Option<AttemptOutput>, max_bytes: u64) -> Option<AttemptOutput> {
    if max_bytes == 0 {
        return None;
    }
    output.map(|o| o.capped(max_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(text: &str) -> AttemptOutput {
        AttemptOutput {
            text: text.into(),
            total_bytes: text.len() as u64,
            cut: false,
        }
    }

    #[test]
    fn the_cap_keeps_the_end_and_a_cap_of_zero_keeps_nothing() {
        let kept = cap(Some(output("0123456789")), 4).expect("kept");
        assert_eq!(kept.text, "6789");
        assert!(kept.cut);
        assert_eq!(kept.total_bytes, 10);

        let whole = cap(Some(output("short")), 1024).expect("kept");
        assert_eq!(whole.text, "short");
        assert!(!whole.cut);

        assert!(cap(Some(output("anything")), 0).is_none());
        assert!(cap(None, 1024).is_none());
    }

    /// An agent that keeps more than the cap is cut, and its own cut flag and
    /// total stay.
    #[test]
    fn a_cut_from_the_agent_stays_cut() {
        let from_agent = AttemptOutput {
            text: "abc".into(),
            total_bytes: 1_000_000,
            cut: true,
        };
        let kept = cap(Some(from_agent), 1024).expect("kept");
        assert!(kept.cut);
        assert_eq!(kept.total_bytes, 1_000_000);
    }
}
