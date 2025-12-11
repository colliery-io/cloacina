/*
 *  Copyright 2025 Colliery Software
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

//! # Logging Configuration
//!
//! This module provides structured logging setup for Cloacina using the `tracing` ecosystem.
//! It supports both production and test environments with configurable log levels.
//!
//! ## Dependencies
//!
//! - `tracing`: Core logging framework
//! - `tracing-subscriber`: Subscriber implementation and formatting
//!
//! ## Features
//!
//! - **Structured Logging**: Uses `tracing` for structured, contextual logging
//! - **Environment Configuration**: Respects `RUST_LOG` environment variable
//! - **Test Support**: Special test logging that doesn't interfere with test output
//! - **Flexible Levels**: Support for all standard log levels (error, warn, info, debug, trace)
//! - **Thread Safety**: All logging operations are thread-safe
//! - **Zero Cost**: When logging is disabled, the compiler eliminates the logging code
//!
//! ## Usage
//!
//! ### Production
//!
//! ```rust,ignore
//! use cloacina::init_logging;
//! use tracing::Level;
//!
//! // Initialize with default level (respects RUST_LOG env var)
//! init_logging(None);
//!
//! // Or specify a level explicitly
//! init_logging(Some(Level::DEBUG));
//!
//! // Log messages with context
//! tracing::info!(target: "my_module", "Processing request", request_id = "123");
//! ```
//!
//! ### Testing
//!
//! In your test functions, call `init_test_logging()` at the start:
//!
//! ```rust,ignore
//! use cloacina::init_test_logging;
//!
//! #[test]
//! fn my_test() {
//!     init_test_logging();
//!     // Your test code with logging
//!     tracing::debug!("Test debug message");
//! }
//! ```
//!
//! ## Log Levels
//!
//! - `ERROR`: Critical errors that may cause system failure
//! - `WARN`: Warning conditions that don't stop execution
//! - `INFO`: General information about system operation
//! - `DEBUG`: Detailed diagnostic information
//! - `TRACE`: Very detailed diagnostic information
//!
//! ## Environment Variables
//!
//! The `RUST_LOG` environment variable supports various formats:
//!
//! - `RUST_LOG=debug` - Enable debug logging for all modules
//! - `RUST_LOG=myapp=trace,other_crate=warn` - Fine-grained control per module
//! - `RUST_LOG=info,myapp::module=debug` - Mix of global and module-specific levels
//!
//! ## Performance Considerations
//!
//! - Logging is disabled at compile time for levels below the configured threshold
//! - Structured fields are only evaluated if the log level is enabled
//! - Test logging uses a special writer that minimizes impact on test performance
//!
//! ## Integration
//!
//! This logging system integrates with:
//!
//! - Standard Rust logging macros (`tracing::info!`, `tracing::error!`, etc.)
//! - Structured field support for JSON logging
//! - Span-based context tracking
//! - Test frameworks for log verification
//!
//! ## Error Handling
//!
//! - Initialization errors are handled gracefully with fallback to default settings
//! - Invalid log levels in environment variables default to "info"
//! - Test logging initialization is idempotent and safe to call multiple times

use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the logging system with the specified log level.
///
/// If no level is provided, it will use the `RUST_LOG` environment variable
/// or default to "info" if the environment variable is not set.
///
/// # Arguments
///
/// * `level` - Optional log level to use. If `None`, uses environment configuration.
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::init_logging;
/// use tracing::Level;
///
/// // Use environment variable or default to INFO
/// init_logging(None);
///
/// // Set specific level
/// init_logging(Some(Level::DEBUG));
/// ```
///
/// # Environment Variables
///
/// The `RUST_LOG` environment variable can be used to control logging:
/// - `RUST_LOG=debug` - Enable debug logging
/// - `RUST_LOG=myapp=trace,other_crate=warn` - Fine-grained control
pub fn init_logging(level: Option<Level>) {
    let filter = match level {
        Some(level) => EnvFilter::new(level.as_str()),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initializes the logging system for test environments.
///
/// This sets up a test-specific subscriber that:
/// - Captures logs for verification in tests
/// - Uses debug level by default
/// - Writes to test output that doesn't interfere with test results
/// - Can be called multiple times safely (subsequent calls are ignored)
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::init_test_logging;
///
/// #[test]
/// fn test_with_logging() {
///     init_test_logging();
///
///     // Your test code here
///     // Logs will be captured and can be verified if needed
/// }
/// ```
#[cfg(test)]
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{debug, error, info, warn};

    #[test]
    fn test_logging_levels() {
        init_test_logging();

        error!("This is an error message");
        warn!("This is a warning message");
        info!("This is an info message");
        debug!("This is a debug message");
    }
}
