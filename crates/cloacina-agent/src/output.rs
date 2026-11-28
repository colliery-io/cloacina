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

//! The output of a worker process (CLOACINA-T-1091).
//!
//! A worker's stdout and stderr are one pipe, so the agent reads them as one
//! stream, a line at a time, on a thread of its own for the worker's life.
//! While a task runs on the worker, each line:
//!
//! 1. has the exact values of the task's own secrets replaced by `***`
//!    ([`Masker`]; on by default, `--task-output-masking`);
//! 2. is written to the agent's log at info, target `task_output`, in the span
//!    of the task, so it carries the task's ids;
//! 3. goes into a buffer of the last bytes of the attempt ([`Tail`]).
//!
//! The worker ends each job with a line that holds a value only that job
//! knows ([`WorkerJob::output_end`](crate::worker::WorkerJob)). The reader
//! closes the attempt's buffer at that line, or when the pipe closes (the
//! worker exited, or was killed for its memory or time limit). The agent
//! sends the buffer with the result ([`cloacina::fleet::AttemptOutput`]).
//!
//! Lines a worker writes while no task runs on it (its start, a graph firing,
//! a poll) are written to the agent's log with no task ids and kept nowhere.

use std::collections::{HashMap, VecDeque};
use std::io::BufRead;
use std::sync::{Arc, Mutex, OnceLock};

use cloacina::fleet::AttemptOutput;
use tokio::sync::oneshot;

/// The longest line the reader keeps whole; a longer line is read in pieces
/// of this size.
const MAX_LINE_BYTES: usize = 64 * 1024;

/// What masking writes in place of a secret value.
pub(crate) const MASK: &str = "***";

/// Replaces the exact values of one attempt's secrets in a line. Nothing more:
/// a value that was encoded, cut, joined into a longer string, or split over
/// two lines is not found.
pub(crate) struct Masker {
    /// Longest first, so a value that holds another is replaced whole.
    values: Vec<String>,
}

impl Masker {
    pub(crate) fn new(values: impl IntoIterator<Item = String>) -> Self {
        let mut values: Vec<String> = values.into_iter().filter(|v| !v.is_empty()).collect();
        values.sort_by_key(|v| std::cmp::Reverse(v.len()));
        values.dedup();
        Self { values }
    }

    /// A masker that replaces nothing.
    pub(crate) fn off() -> Self {
        Self { values: Vec::new() }
    }

    pub(crate) fn mask(&self, line: &str) -> String {
        let mut out = line.to_string();
        for value in &self.values {
            if out.contains(value.as_str()) {
                out = out.replace(value.as_str(), MASK);
            }
        }
        out
    }
}

/// The last `max` bytes of a stream, and the count of all of it.
pub(crate) struct Tail {
    max: usize,
    kept: VecDeque<u8>,
    total: u64,
    cut: bool,
}

impl Tail {
    pub(crate) fn new(max: u64) -> Self {
        Self {
            max: usize::try_from(max).unwrap_or(usize::MAX),
            kept: VecDeque::new(),
            total: 0,
            cut: false,
        }
    }

    /// Add `bytes` to the stream. `counted` is how many bytes the task wrote
    /// for them (a masked line can be shorter or longer than it was).
    pub(crate) fn push(&mut self, bytes: &[u8], counted: usize) {
        self.total = self.total.saturating_add(counted as u64);
        if self.max == 0 {
            self.cut |= !bytes.is_empty();
            return;
        }
        if bytes.len() >= self.max {
            self.kept.clear();
            self.kept.extend(&bytes[bytes.len() - self.max..]);
            self.cut = true;
            return;
        }
        let over = (self.kept.len() + bytes.len()).saturating_sub(self.max);
        if over > 0 {
            self.kept.drain(..over);
            self.cut = true;
        }
        self.kept.extend(bytes);
    }

    /// The text kept, as UTF-8. A cut that fell in a character drops the
    /// rest of that character; any other byte that is not UTF-8 is replaced.
    pub(crate) fn finish(self) -> AttemptOutput {
        let mut bytes: Vec<u8> = self.kept.into_iter().collect();
        if self.cut {
            let start = bytes
                .iter()
                .take(4)
                .position(|b| (b & 0b1100_0000) != 0b1000_0000)
                .unwrap_or(0);
            bytes.drain(..start);
        }
        AttemptOutput {
            text: String::from_utf8_lossy(&bytes).into_owned(),
            total_bytes: self.total,
            cut: self.cut,
        }
    }
}

/// The capture of one attempt on a worker.
pub(crate) struct Capture {
    span: tracing::Span,
    masker: Masker,
    tail: Tail,
    /// The line the worker writes when the job ends.
    end_line: String,
    done: Option<oneshot::Sender<AttemptOutput>>,
}

impl Capture {
    /// A capture for the job whose end line is `end_line`; `done` gets the
    /// output.
    pub(crate) fn new(
        span: tracing::Span,
        masker: Masker,
        max_bytes: u64,
        end_line: String,
        done: oneshot::Sender<AttemptOutput>,
    ) -> Self {
        Self {
            span,
            masker,
            tail: Tail::new(max_bytes),
            end_line,
            done: Some(done),
        }
    }

    fn line(&mut self, raw: &[u8]) {
        let text = String::from_utf8_lossy(raw);
        let text = text.strip_suffix('\n').unwrap_or(&text);
        let text = text.strip_suffix('\r').unwrap_or(text);
        let masked = self.masker.mask(text);
        self.span.in_scope(|| {
            tracing::info!(target: "task_output", "{masked}");
        });
        let mut bytes = masked.into_bytes();
        if raw.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        self.tail.push(&bytes, raw.len());
    }

    fn finish(mut self) {
        if let Some(done) = self.done.take() {
            let _ = done.send(self.tail.finish());
        }
    }
}

/// Where the reader of one worker puts the lines of the job that runs now.
pub(crate) type CaptureSlot = Arc<Mutex<Option<Capture>>>;

/// Start the reader of a worker's output pipe. It runs until the pipe
/// closes; at that point it closes the capture that is still open.
pub(crate) fn spawn_reader(pipe: std::io::PipeReader, pid: u32) -> CaptureSlot {
    let slot: CaptureSlot = Arc::new(Mutex::new(None));
    let reader_slot = slot.clone();
    std::thread::spawn(move || read_lines(std::io::BufReader::new(pipe), &reader_slot, pid));
    slot
}

/// The reader loop, on any buffered stream (tests use a byte slice).
pub(crate) fn read_lines(mut reader: impl BufRead, slot: &CaptureSlot, pid: u32) {
    let mut line = Vec::with_capacity(1024);
    loop {
        line.clear();
        match read_bounded_line(&mut reader, &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let mut held = slot.lock().unwrap_or_else(|e| e.into_inner());
        match held.as_mut() {
            Some(capture) if is_end_line(&line, &capture.end_line) => {
                if let Some(capture) = held.take() {
                    capture.finish();
                }
            }
            Some(capture) => capture.line(&line),
            None => {
                drop(held);
                let text = String::from_utf8_lossy(&line);
                tracing::info!(target: "worker_output", pid, "{}", text.trim_end());
            }
        }
    }
    close(slot);
}

/// Close the capture that is open in `slot`, if any, and send its output.
pub(crate) fn close(slot: &CaptureSlot) {
    let open = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(capture) = open {
        capture.finish();
    }
}

fn is_end_line(line: &[u8], end_line: &str) -> bool {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line == end_line.as_bytes()
}

/// Read up to a newline, or [`MAX_LINE_BYTES`], whichever comes first.
fn read_bounded_line(reader: &mut impl BufRead, out: &mut Vec<u8>) -> std::io::Result<usize> {
    loop {
        let available = match reader.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if available.is_empty() {
            return Ok(out.len());
        }
        let room = MAX_LINE_BYTES - out.len();
        let window = &available[..available.len().min(room)];
        let (taken, ended) = match window.iter().position(|b| *b == b'\n') {
            Some(at) => (at + 1, true),
            None => (window.len(), out.len() + window.len() >= MAX_LINE_BYTES),
        };
        out.extend_from_slice(&available[..taken]);
        reader.consume(taken);
        if ended {
            return Ok(out.len());
        }
    }
}

/// The value of a job's end line: random, so a task cannot guess it.
pub(crate) fn new_end_line() -> String {
    format!(
        "cloacina-output-end {}",
        cloacina::UniversalUuid::new_v4().0.simple()
    )
}

/// The outputs of attempts that ran in a worker, until the agent sends them
/// with the result. Keyed by [`output_key`].
fn outputs() -> std::sync::MutexGuard<'static, HashMap<String, AttemptOutput>> {
    static OUTPUTS: OnceLock<Mutex<HashMap<String, AttemptOutput>>> = OnceLock::new();
    OUTPUTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// The key of an attempt: its dispatch, else its task and attempt.
pub(crate) fn output_key(
    task_execution_id: &str,
    attempt: i32,
    dispatch_id: Option<&str>,
) -> String {
    match dispatch_id {
        Some(id) => format!("dispatch:{id}"),
        None => format!("task:{task_execution_id}:{attempt}"),
    }
}

pub(crate) fn keep_output(key: String, output: AttemptOutput) {
    outputs().insert(key, output);
}

/// The output of the attempt, taken: the agent calls this once per packet,
/// whether it reports or not, so nothing is left behind.
pub(crate) fn take_output(key: &str) -> Option<AttemptOutput> {
    outputs().remove(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masking_replaces_exact_values_longest_first_and_nothing_else() {
        let masker = Masker::new([
            "hunter2".to_string(),
            "hunter2-long".to_string(),
            String::new(),
        ]);
        assert_eq!(masker.mask("pw=hunter2-long and hunter2"), "pw=*** and ***");
        // An encoded value is not found.
        assert_eq!(masker.mask("aHVudGVyMg=="), "aHVudGVyMg==");
        // An empty value masks nothing.
        assert_eq!(masker.mask("plain"), "plain");
        assert_eq!(Masker::off().mask("hunter2"), "hunter2");
    }

    #[test]
    fn the_tail_keeps_the_last_bytes_and_counts_all() {
        let mut tail = Tail::new(8);
        tail.push(b"0123\n", 5);
        tail.push(b"4567\n", 5);
        let out = tail.finish();
        assert_eq!(out.text, "23\n4567\n");
        assert_eq!(out.total_bytes, 10);
        assert!(out.cut);

        let mut tail = Tail::new(1024);
        tail.push(b"short\n", 6);
        let out = tail.finish();
        assert_eq!(out.text, "short\n");
        assert!(!out.cut);

        // A line longer than the cap keeps its end.
        let mut tail = Tail::new(4);
        tail.push(b"abcdefgh", 8);
        assert_eq!(tail.finish().text, "efgh");

        // A cut in a character drops what is left of it.
        let mut tail = Tail::new(3);
        tail.push("aé€".as_bytes(), 6);
        assert_eq!(tail.finish().text, "€");

        // A cap of 0 keeps nothing and says it cut.
        let mut tail = Tail::new(0);
        tail.push(b"x", 1);
        let out = tail.finish();
        assert_eq!(out.text, "");
        assert!(out.cut);
    }

    fn capture(
        end: &str,
        secrets: &[&str],
        max: u64,
    ) -> (CaptureSlot, oneshot::Receiver<AttemptOutput>) {
        let (tx, rx) = oneshot::channel();
        let slot: CaptureSlot = Arc::new(Mutex::new(Some(Capture::new(
            tracing::Span::none(),
            Masker::new(secrets.iter().map(|s| s.to_string())),
            max,
            end.to_string(),
            tx,
        ))));
        (slot, rx)
    }

    /// The lines up to the end line are the attempt's, masked; the lines
    /// after it are kept nowhere.
    #[test]
    fn the_end_line_closes_the_attempt() {
        let (slot, mut rx) = capture("END-1", &["s3cret"], 1024);
        let stream = b"loading\ntoken=s3cret\nEND-1\nafter the job\n";
        read_lines(&stream[..], &slot, 1);
        let out = rx.try_recv().expect("closed at the end line");
        assert_eq!(out.text, "loading\ntoken=***\n");
        assert_eq!(out.total_bytes, "loading\ntoken=s3cret\n".len() as u64);
        assert!(!out.cut);
        assert!(slot.lock().unwrap().is_none());
    }

    /// A stream that ends with no end line (the worker was killed) still
    /// gives its tail, a last line with no newline included.
    #[test]
    fn a_closed_pipe_closes_the_attempt() {
        let (slot, mut rx) = capture("END-2", &[], 1024);
        read_lines(&b"one\ntwo"[..], &slot, 1);
        assert_eq!(rx.try_recv().expect("closed at EOF").text, "one\ntwo");
    }

    /// A task that prints the end line of another job does not end its own.
    #[test]
    fn only_the_jobs_own_end_line_counts() {
        let (slot, mut rx) = capture("END-3", &[], 1024);
        read_lines(&b"END-other\nEND-3x\nlast\n"[..], &slot, 1);
        assert_eq!(rx.try_recv().unwrap().text, "END-other\nEND-3x\nlast\n");
    }

    #[test]
    fn a_long_line_is_read_in_pieces() {
        let (slot, mut rx) = capture("END-4", &[], 16);
        let long = vec![b'x'; MAX_LINE_BYTES * 2 + 10];
        read_lines(&long[..], &slot, 1);
        let out = rx.try_recv().unwrap();
        assert_eq!(out.total_bytes, long.len() as u64);
        assert_eq!(out.text, "x".repeat(16));
        assert!(out.cut);
    }

    #[test]
    fn an_output_is_taken_once() {
        let key = output_key("t", 1, Some("d-1"));
        keep_output(
            key.clone(),
            AttemptOutput {
                text: "x".into(),
                total_bytes: 1,
                cut: false,
            },
        );
        assert!(take_output(&key).is_some());
        assert!(take_output(&key).is_none());
        assert_eq!(output_key("t", 2, None), "task:t:2");
    }
}
