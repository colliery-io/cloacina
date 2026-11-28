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

//! What a task leaves behind for a person who debugs it (CLOACINA-T-1024).
//!
//! Two things live here:
//!
//! - [`task_span`]: the `tracing` span around one attempt of a task. Each log
//!   line written inside it carries the ids of the attempt.
//! - Error details: a failure text can carry details (a Python traceback)
//!   after [`ERROR_DETAILS_MARKER`]. The part before the marker is the
//!   message: it is what `last_error` stores and what the retry conditions
//!   read. The part after it is stored in `error_details`.

use crate::task::namespace::parse_namespace;

/// The line that parts the message of a failure from its details.
pub const ERROR_DETAILS_MARKER: &str = "\n\n--- error details ---\n";

/// The most bytes of details one failure carries. Longer details keep their
/// end, which for a traceback is the call that raised and the exception.
pub const MAX_ERROR_DETAILS_BYTES: usize = 16 * 1024;

/// `message` followed by the marker and `details`, with the details cut to
/// [`MAX_ERROR_DETAILS_BYTES`]. Empty details give the message alone.
pub fn with_error_details(message: &str, details: &str) -> String {
    let details = details.trim_end();
    if details.is_empty() {
        return message.to_string();
    }
    format!(
        "{message}{ERROR_DETAILS_MARKER}{}",
        bound_error_details(details)
    )
}

/// The message of a failure text, and its details when it has any.
pub fn split_error_details(text: &str) -> (&str, Option<&str>) {
    match text.split_once(ERROR_DETAILS_MARKER) {
        Some((message, details)) => (message, Some(details)),
        None => (text, None),
    }
}

/// The message of a failure text: the text with no details.
pub fn error_message(text: &str) -> &str {
    split_error_details(text).0
}

/// `details` cut to [`MAX_ERROR_DETAILS_BYTES`]. A cut keeps the end and
/// starts with a line that says how many bytes are gone.
pub fn bound_error_details(details: &str) -> String {
    if details.len() <= MAX_ERROR_DETAILS_BYTES {
        return details.to_string();
    }
    let mut start = details.len() - MAX_ERROR_DETAILS_BYTES;
    while !details.is_char_boundary(start) {
        start += 1;
    }
    format!("[first {start} bytes cut]\n{}", &details[start..])
}

/// The most characters of a failed task's error message that a log line at
/// info or above holds (CLOACINA-T-1096). A fixed value. It counts characters,
/// not bytes.
pub const MAX_LOGGED_ERROR_CHARS: usize = 300;

/// The error message of a failed task as a log line at info or above holds
/// it: whole when it has at most [`MAX_LOGGED_ERROR_CHARS`] characters, else
/// its first [`MAX_LOGGED_ERROR_CHARS`] characters and a note that says how
/// long the whole message is. The full message is task data: a line that
/// holds it is at trace.
pub fn error_for_log(message: &str) -> std::borrow::Cow<'_, str> {
    match message.char_indices().nth(MAX_LOGGED_ERROR_CHARS) {
        None => std::borrow::Cow::Borrowed(message),
        Some((end, _)) => std::borrow::Cow::Owned(format!(
            "{}... [cut: {MAX_LOGGED_ERROR_CHARS} of {} characters]",
            &message[..end],
            message.chars().count()
        )),
    }
}

/// The format of the log lines a process writes to its console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// One line of text per event.
    #[default]
    Text,
    /// One JSON object per line. The ids of [`task_span`] are under `span`.
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unknown log format `{other}`: use `text` or `json`"
            )),
        }
    }
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Text => "text",
            Self::Json => "json",
        })
    }
}

/// The span around one attempt of a task.
///
/// Fields: `tenant`, `workflow`, `execution_id` (the run), `task` (the full
/// task name, as the API gives it), `attempt`, and `dispatch_id` when the
/// attempt runs on an agent. A log line written inside the span carries them:
/// in the JSON format under `span`, in the text format before the message.
///
/// The span has the level ERROR so that a level filter such as
/// `RUST_LOG=warn` does not drop it, and the ids stay on the lines that pass
/// the filter. The span writes no line of its own.
pub fn task_span(
    tenant: &str,
    task_name: &str,
    execution_id: &str,
    attempt: i32,
    dispatch_id: Option<&str>,
) -> tracing::Span {
    let workflow = parse_namespace(task_name)
        .map(|ns| ns.workflow_id)
        .unwrap_or_else(|_| "unknown".to_string());
    let span = tracing::error_span!(
        "task",
        tenant = %tenant,
        workflow = %workflow,
        execution_id = %execution_id,
        task = %task_name,
        attempt = attempt,
        dispatch_id = tracing::field::Empty,
    );
    if let Some(id) = dispatch_id {
        span.record("dispatch_id", tracing::field::display(id));
    }
    span
}

/// The host side of `CloacinaLog` (CLOACINA-T-1099): writes the log lines of
/// a packaged library with this process's subscriber.
///
/// A line is written as a `log` record through `tracing-log`, so it keeps
/// its level and target, passes the host's filter by that target, and lands
/// in the span current on the calling thread: for a task, the [`task_span`]
/// the executor and the agent enter around the call.
pub struct EngineLog;

impl cloacina_workflow_plugin::CloacinaLog for EngineLog {
    fn max_level(&self) -> u32 {
        match tracing::level_filters::LevelFilter::current().into_level() {
            None => 0,
            Some(tracing::Level::ERROR) => 1,
            Some(tracing::Level::WARN) => 2,
            Some(tracing::Level::INFO) => 3,
            Some(tracing::Level::DEBUG) => 4,
            Some(tracing::Level::TRACE) => 5,
        }
    }

    fn write_line(
        &self,
        level: u32,
        target: String,
        message: String,
        module_path: Option<String>,
        file: Option<String>,
        line: Option<u32>,
    ) {
        let level = match level {
            1 => log::Level::Error,
            2 => log::Level::Warn,
            3 => log::Level::Info,
            4 => log::Level::Debug,
            5 => log::Level::Trace,
            _ => return,
        };
        let _ = tracing_log::format_trace(
            &log::Record::builder()
                .level(level)
                .target(&target)
                .args(format_args!("{message}"))
                .module_path(module_path.as_deref())
                .file(file.as_deref())
                .line(line)
                .build(),
        );
    }
}

/// Captures the text log lines of a test.
#[cfg(test)]
pub(crate) mod test_support {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub(crate) struct LogCapture(Arc<Mutex<Vec<u8>>>);

    impl LogCapture {
        /// A subscriber that writes text lines, with no colour, into this
        /// capture.
        pub(crate) fn subscriber(
            &self,
            max_level: tracing::Level,
        ) -> impl tracing::Subscriber + Send + Sync + 'static {
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(self.clone())
                .with_max_level(max_level)
                .finish()
        }

        /// What was written so far.
        pub(crate) fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for LogCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
        type Writer = LogCapture;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::LogCapture;
    use super::*;

    #[test]
    fn details_round_trip_through_the_marker() {
        let text = with_error_details(
            "ValueError: boom",
            "Traceback\n  File x\nValueError: boom\n",
        );
        let (message, details) = split_error_details(&text);
        assert_eq!(message, "ValueError: boom");
        assert_eq!(details, Some("Traceback\n  File x\nValueError: boom"));
        assert_eq!(error_message(&text), "ValueError: boom");
    }

    #[test]
    fn a_text_with_no_details_is_all_message() {
        assert_eq!(split_error_details("plain"), ("plain", None));
        assert_eq!(with_error_details("plain", "  \n"), "plain");
    }

    #[test]
    fn long_details_keep_their_end_and_stay_under_the_cap() {
        let long = format!("{}END", "é".repeat(MAX_ERROR_DETAILS_BYTES));
        let bound = bound_error_details(&long);
        assert!(bound.starts_with("[first "));
        assert!(bound.ends_with("END"));
        let kept = bound.split_once('\n').unwrap().1;
        assert!(kept.len() <= MAX_ERROR_DETAILS_BYTES);
        // A wrapper around the failure keeps the split working.
        let wrapped = format!("Task execution failed: {}", with_error_details("m", &long));
        let (message, details) = split_error_details(&wrapped);
        assert_eq!(message, "Task execution failed: m");
        assert!(details.unwrap().ends_with("END"));
    }

    #[test]
    fn an_error_for_a_log_line_is_whole_up_to_the_limit_and_cut_after_it() {
        let at_limit = "e".repeat(MAX_LOGGED_ERROR_CHARS);
        assert_eq!(error_for_log(&at_limit), at_limit.as_str());
        assert_eq!(error_for_log(""), "");

        let over = format!("{at_limit}TAIL");
        assert_eq!(
            error_for_log(&over),
            format!("{at_limit}... [cut: 300 of 304 characters]")
        );
    }

    #[test]
    fn a_cut_error_counts_characters_and_splits_none() {
        // 2 bytes each: a cut by bytes at 300 would be inside a character
        // for an odd offset, and would keep 150 of them.
        let over = format!("x{}", "é".repeat(MAX_LOGGED_ERROR_CHARS));
        let cut = error_for_log(&over);
        let kept = cut.split_once("... [cut: ").unwrap().0;
        assert_eq!(kept.chars().count(), MAX_LOGGED_ERROR_CHARS);
        assert!(cut.ends_with("[cut: 300 of 301 characters]"), "{cut}");
    }

    #[test]
    fn log_format_reads_text_and_json() {
        assert_eq!("text".parse::<LogFormat>(), Ok(LogFormat::Text));
        assert_eq!(" JSON ".parse::<LogFormat>(), Ok(LogFormat::Json));
        assert!("yaml".parse::<LogFormat>().is_err());
        assert_eq!(LogFormat::default().to_string(), "text");
        assert_eq!(LogFormat::Json.to_string(), "json");
    }

    fn logged(max_level: tracing::Level, body: impl FnOnce()) -> String {
        let capture = LogCapture::default();
        tracing::subscriber::with_default(capture.subscriber(max_level), body);
        capture.text()
    }

    #[test]
    fn a_line_logged_inside_the_span_carries_the_ids() {
        // A filter above INFO must not drop the span.
        let out = logged(tracing::Level::WARN, || {
            let span = task_span("acme", "acme::pkg::etl::load", "exec-1", 2, Some("d-7"));
            let _entered = span.enter();
            tracing::warn!("inside the task");
            tracing::info!("below the filter");
        });
        assert_eq!(out.lines().count(), 1, "{out}");
        assert!(
            out.contains(
                "task{tenant=acme workflow=etl execution_id=exec-1 \
                 task=acme::pkg::etl::load attempt=2 dispatch_id=d-7}: "
            ),
            "{out}"
        );
        assert!(out.contains("inside the task"), "{out}");
    }

    #[test]
    fn a_line_from_a_package_keeps_its_level_and_target_and_the_ids() {
        use cloacina_workflow_plugin::CloacinaLog;
        let out = logged(tracing::Level::INFO, || {
            let span = task_span("acme", "acme::pkg::etl::load", "exec-3", 1, None);
            let _entered = span.enter();
            EngineLog.write_line(
                2,
                "pkg::load".into(),
                "from the package".into(),
                None,
                None,
                None,
            );
            EngineLog.write_line(
                4,
                "pkg::load".into(),
                "debug from the package".into(),
                None,
                None,
                None,
            );
            EngineLog.write_line(
                9,
                "pkg::load".into(),
                "no such level".into(),
                None,
                None,
                None,
            );
        });
        assert_eq!(out.lines().count(), 1, "{out}");
        assert!(out.contains(" WARN "), "{out}");
        assert!(out.contains("pkg::load: from the package"), "{out}");
        assert!(
            out.contains("task{tenant=acme workflow=etl execution_id=exec-3"),
            "{out}"
        );
    }

    #[test]
    fn a_span_with_no_dispatch_has_no_dispatch_id() {
        let out = logged(tracing::Level::INFO, || {
            let span = task_span("public", "not a namespace", "exec-2", 1, None);
            let _entered = span.enter();
            tracing::info!("inside the task");
        });
        assert!(out.contains("workflow=unknown"), "{out}");
        assert!(!out.contains("dispatch_id"), "{out}");
    }
}
