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

//! The log lines of a packaged task go to the host's log (CLOACINA-T-1099).
//!
//! A packaged workflow is a loaded library with its own copy of `tracing` and
//! `log`, and so its own global dispatcher and logger, which nobody sets. The
//! host binds [`CloacinaLog`] at load. At the first call into the library the
//! `package!()` shell calls [`install`]: it sets the library's dispatcher and
//! logger to a forwarder that sends each event to the host. The host writes
//! the line with its own subscriber, in the span it has current on the
//! calling thread, so the line carries the ids of the task.
//!
//! The interface is its own host import, not a method of `CloacinaHost`:
//! fidius binds host imports by name, so a package built before this exists
//! declares no `CloacinaLog` import and loads as before, and an older host
//! binds nothing and the package logs as before. Neither `CloacinaPlugin`
//! nor `CloacinaHost` changes version.

use std::cell::Cell;
use std::fmt::Write as _;
use std::sync::Once;
use std::sync::atomic::{AtomicU32, Ordering};

use tracing_core::field::{Field, Visit};
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::subscriber::{Interest, Subscriber};
use tracing_core::{Dispatch, Event, Level, LevelFilter, Metadata};

/// Where a packaged library writes its log lines: the host's log.
///
/// Levels are numbers: 1 ERROR, 2 WARN, 3 INFO, 4 DEBUG, 5 TRACE; 0 is off.
#[fidius::host_interface(version = 1)]
pub trait CloacinaLog: Send + Sync {
    /// The most verbose level the host writes, as a number. 0: none.
    fn max_level(&self) -> u32;

    /// Write one line in the host's log, at `level`, under `target`, in the
    /// span the host has current on the calling thread. The host's filter
    /// decides whether the line is written.
    fn write_line(
        &self,
        level: u32,
        target: String,
        message: String,
        module_path: Option<String>,
        file: Option<String>,
        line: Option<u32>,
    );
}

/// The number of a `tracing` level.
fn level_number(level: &Level) -> u32 {
    match *level {
        Level::ERROR => 1,
        Level::WARN => 2,
        Level::INFO => 3,
        Level::DEBUG => 4,
        Level::TRACE => 5,
    }
}

/// The number of a `log` level.
fn log_level_number(level: log::Level) -> u32 {
    match level {
        log::Level::Error => 1,
        log::Level::Warn => 2,
        log::Level::Info => 3,
        log::Level::Debug => 4,
        log::Level::Trace => 5,
    }
}

thread_local! {
    /// Set while this thread sends a line to the host, so that a line logged
    /// on the way is dropped and not sent again.
    static SENDING: Cell<bool> = const { Cell::new(false) };
}

fn send(
    level: u32,
    target: &str,
    message: String,
    module_path: Option<&str>,
    file: Option<&str>,
    line: Option<u32>,
) {
    if SENDING.with(|s| s.replace(true)) {
        return;
    }
    if let Ok(host) = CloacinaLogClient::bound() {
        let _ = host.write_line(
            &level,
            &target.to_string(),
            &message,
            &module_path.map(str::to_string),
            &file.map(str::to_string),
            &line,
        );
    }
    SENDING.with(|s| s.set(false));
}

/// The message of an event, then its other fields as `name=value`.
#[derive(Default)]
struct Line {
    message: String,
    fields: String,
}

impl Visit for Line {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            let _ = write!(self.fields, " {}={:?}", field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={:?}", field.name(), value);
        }
    }
}

/// The library's `tracing` subscriber: sends each event to the host.
///
/// Spans of the library are not kept: the line lands in the host's span.
struct Forwarder {
    max_level: u32,
}

impl Subscriber for Forwarder {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        !metadata.is_span() && level_number(metadata.level()) <= self.max_level
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(match self.max_level {
            0 => LevelFilter::OFF,
            1 => LevelFilter::ERROR,
            2 => LevelFilter::WARN,
            3 => LevelFilter::INFO,
            4 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        })
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut line = Line::default();
        event.record(&mut line);
        line.message.push_str(&line.fields);
        send(
            level_number(metadata.level()),
            metadata.target(),
            line.message,
            metadata.module_path(),
            metadata.file(),
            metadata.line(),
        );
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

/// The library's `log` logger: sends each record to the host.
struct LogForwarder {
    max_level: AtomicU32,
}

static LOG_FORWARDER: LogForwarder = LogForwarder {
    max_level: AtomicU32::new(0),
};

impl log::Log for LogForwarder {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        log_level_number(metadata.level()) <= self.max_level.load(Ordering::Relaxed)
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        send(
            log_level_number(record.level()),
            record.target(),
            record.args().to_string(),
            record.module_path(),
            record.file(),
            record.line(),
        );
    }

    fn flush(&self) {}
}

/// Send the library's `tracing` events and `log` records to the host, when
/// the host bound [`CloacinaLog`]. Called by the `package!()` shell at the
/// start of each call that runs package code; it acts once.
///
/// With no bound host (an older host) it sets nothing, and the library logs
/// as before. A dispatcher or logger the package set itself is kept.
#[doc(hidden)]
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(host) = CloacinaLogClient::bound() else {
            return;
        };
        let max_level = host.max_level().unwrap_or(0).min(5);
        if max_level == 0 {
            return;
        }
        let _ =
            tracing_core::dispatcher::set_global_default(Dispatch::new(Forwarder { max_level }));
        LOG_FORWARDER.max_level.store(max_level, Ordering::Relaxed);
        if log::set_logger(&LOG_FORWARDER).is_ok() {
            log::set_max_level(match max_level {
                1 => log::LevelFilter::Error,
                2 => log::LevelFilter::Warn,
                3 => log::LevelFilter::Info,
                4 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_numbered_from_error() {
        assert_eq!(level_number(&Level::ERROR), 1);
        assert_eq!(level_number(&Level::TRACE), 5);
        assert_eq!(log_level_number(log::Level::Warn), 2);
        assert_eq!(log_level_number(log::Level::Debug), 4);
    }

    #[test]
    fn the_forwarder_takes_events_up_to_the_hosts_level_and_no_spans() {
        let forwarder = Forwarder { max_level: 3 };
        assert_eq!(forwarder.max_level_hint(), Some(LevelFilter::INFO));
        let dispatch = Dispatch::new(forwarder);
        tracing_core::dispatcher::with_default(&dispatch, || {
            assert!(tracing::enabled!(tracing::Level::INFO));
            assert!(tracing::enabled!(tracing::Level::WARN));
            assert!(!tracing::enabled!(tracing::Level::DEBUG));
            assert!(tracing::info_span!("in the package").is_disabled());
        });
    }

    #[test]
    fn an_unbound_host_installs_nothing() {
        assert!(!CloacinaLogClient::is_bound());
        install();
        assert_eq!(log::max_level(), log::LevelFilter::Off);
    }
}
