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

use super::settings::AppSettings;
use anyhow::Result;
use chrono::Local;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{span::Attributes, Event, Id, Subscriber};
use tracing_appender::non_blocking;
use tracing_subscriber::{
    layer::Context as LayerContext, layer::SubscriberExt, registry::LookupSpan, reload,
    util::SubscriberInitExt, EnvFilter, Layer,
};

// Global handle for reloading the filter
static RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();

// Global registry for runner-specific loggers
static RUNNER_LOGGERS: OnceLock<Arc<Mutex<HashMap<String, RunnerLogger>>>> = OnceLock::new();

// Global registry for runner-specific file appenders
static RUNNER_APPENDERS: OnceLock<Arc<Mutex<HashMap<String, Arc<Mutex<DailyRollingAppender>>>>>> =
    OnceLock::new();

// Runner context stored in span extensions
#[derive(Debug, Clone)]
struct RunnerContext {
    runner_id: String,
    runner_name: Option<String>,
}

// Custom layer that extracts runner_id from spans and routes logs
pub struct RunnerContextLayer {
    router: Arc<DynamicFileRouter>,
}

impl RunnerContextLayer {
    pub fn new() -> Self {
        Self {
            router: Arc::new(DynamicFileRouter::new()),
        }
    }
}

// Visitor pattern for field extraction
struct RunnerFieldVisitor {
    runner_id: Option<String>,
    runner_name: Option<String>,
}

// Visitor for extracting log message
struct MessageVisitor {
    message: Option<String>,
}

impl MessageVisitor {
    fn new() -> Self {
        Self { message: None }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }
}

impl RunnerFieldVisitor {
    fn new() -> Self {
        Self {
            runner_id: None,
            runner_name: None,
        }
    }
}

impl tracing::field::Visit for RunnerFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "runner_id" => self.runner_id = Some(value.to_string()),
            "runner_name" => self.runner_name = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "runner_id" => {
                let debug_str = format!("{:?}", value);
                self.runner_id = Some(debug_str.trim_matches('"').to_string());
            }
            "runner_name" => {
                let debug_str = format!("{:?}", value);
                self.runner_name = Some(debug_str.trim_matches('"').to_string());
            }
            _ => {}
        }
    }
}

impl<S> Layer<S> for RunnerContextLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: LayerContext<'_, S>) {
        let mut visitor = RunnerFieldVisitor::new();
        attrs.record(&mut visitor);

        // Store runner context if we found a runner_id
        if let Some(runner_id) = visitor.runner_id {
            if is_valid_uuid_like(&runner_id) {
                let runner_context = RunnerContext {
                    runner_id: runner_id.clone(),
                    runner_name: visitor.runner_name,
                };

                if let Some(span) = ctx.span(id) {
                    let mut extensions = span.extensions_mut();
                    extensions.insert(runner_context);
                }
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: LayerContext<'_, S>) {
        // Only process runner-related events from cloacina, but exclude internal logging
        let target = event.metadata().target();
        if target.starts_with("cloacina") &&
           !target.contains("logging") && // Avoid recursion from logging operations
           !target.contains("settings")
        {
            // Avoid recursion from settings loading
            // Look for runner context in the span hierarchy
            if let Some(scope) = ctx.event_scope(event) {
                for span in scope.from_root() {
                    let extensions = span.extensions();
                    if let Some(runner_context) = extensions.get::<RunnerContext>() {
                        // Route this event to the runner-specific file with runner name
                        self.router.route_event_to_runner_with_name(
                            &runner_context.runner_id,
                            &runner_context.runner_name,
                            event,
                        );
                        break;
                    }
                }
            }
        }
    }
}

// Dynamic file router for runner-specific logs
pub struct DynamicFileRouter {
    // We'll use the existing RUNNER_APPENDERS global for file handles
}

impl DynamicFileRouter {
    pub fn new() -> Self {
        Self {}
    }

    fn route_event_to_runner_with_name(
        &self,
        runner_id: &str,
        runner_name: &Option<String>,
        event: &Event<'_>,
    ) {
        // Extract the actual message from the event
        let mut message_visitor = MessageVisitor::new();
        event.record(&mut message_visitor);

        // Format the event to a string with clean, readable content
        let formatted = format!(
            "{} {:?} {}: {}",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
            event.metadata().level(),
            event.metadata().target(),
            message_visitor
                .message
                .unwrap_or_else(|| "No message".to_string())
        );

        // Write to runner-specific file (but don't let errors block)
        if let Err(e) = self.write_to_runner_file_with_name(runner_id, runner_name, &formatted) {
            // Use stderr directly to avoid recursive logging
            eprintln!("ERROR: Failed to write to runner log file: {:?}", e);
        }
    }

    fn write_to_runner_file_with_name(
        &self,
        runner_id: &str,
        runner_name: &Option<String>,
        content: &str,
    ) -> std::io::Result<()> {
        match get_runner_appender_with_name(runner_id, runner_name) {
            Ok(appender) => {
                let mut writer = appender.lock().map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("Lock error: {}", e))
                })?;
                writeln!(writer, "{}", content)?;
                writer.flush()?;
                Ok(())
            }
            Err(e) => {
                eprintln!("ERROR: Failed to get runner appender: {:?}", e);
                Err(e)
            }
        }
    }
}

// Runner-specific logger
#[derive(Clone, Serialize)]
pub struct RunnerLogger {
    pub runner_id: String,
    pub runner_name: String,
    pub log_directory: String,
}

impl RunnerLogger {
    pub fn new(runner_id: String, runner_name: String, log_directory: String) -> Self {
        Self {
            runner_id,
            runner_name,
            log_directory,
        }
    }

    pub fn get_runner_log_directory(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.log_directory).join("runners")
    }
}

// Custom rolling file appender with proper naming convention
struct DailyRollingAppender {
    log_directory: String,
    base_filename: String,
    current_file: Option<std::fs::File>,
    current_date: String,
}

impl DailyRollingAppender {
    fn new(log_directory: String, base_filename: String) -> Self {
        Self {
            log_directory,
            base_filename,
            current_file: None,
            current_date: String::new(),
        }
    }

    fn get_current_date() -> String {
        Local::now().format("%Y-%m-%d").to_string()
    }

    fn get_log_file_path(&self, date: &str) -> std::path::PathBuf {
        std::path::Path::new(&self.log_directory)
            .join(format!("{}-{}.log", self.base_filename, date))
    }

    fn ensure_current_file(&mut self) -> Result<&mut std::fs::File> {
        let current_date = Self::get_current_date();

        // Check if we need to rotate to a new file
        if self.current_date != current_date || self.current_file.is_none() {
            let log_path = self.get_log_file_path(&current_date);

            // Ensure directory exists
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Open new log file (append mode)
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;

            self.current_file = Some(file);
            self.current_date = current_date;
        }

        Ok(self.current_file.as_mut().unwrap())
    }
}

impl Write for DailyRollingAppender {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let file = self
            .ensure_current_file()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut file) = self.current_file {
            file.flush()
        } else {
            Ok(())
        }
    }
}

// Simplified writer - runner routing is now handled by the layer
struct SimpleAppWriter {
    writer: Arc<Mutex<DailyRollingAppender>>,
}

impl SimpleAppWriter {
    fn new(writer: DailyRollingAppender) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

impl Write for SimpleAppWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut writer = self.writer.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Lock error: {}", e))
        })?;
        writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut writer = self.writer.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Lock error: {}", e))
        })?;
        writer.flush()
    }
}

// Helper function to check if a string looks like a UUID
fn is_valid_uuid_like(s: &str) -> bool {
    // UUID format: 8-4-4-4-12 hex characters with dashes
    // e.g., 1537f636-5a9f-4387-8b86-1109e4921b93
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

// Get or create runner-specific appender with known runner name
fn get_runner_appender_with_name(
    runner_id: &str,
    runner_name: &Option<String>,
) -> Result<Arc<Mutex<DailyRollingAppender>>, std::io::Error> {
    // Initialize appenders registry if needed
    if RUNNER_APPENDERS.get().is_none() {
        let _ = RUNNER_APPENDERS.set(Arc::new(Mutex::new(HashMap::new())));
    }

    let appenders_registry = RUNNER_APPENDERS.get().unwrap();

    // First, check if appender already exists (quick operation)
    {
        let appenders = appenders_registry.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Lock error: {}", e))
        })?;

        if let Some(appender) = appenders.get(runner_id) {
            return Ok(appender.clone());
        }
        // Lock is automatically released here when appenders goes out of scope
    }

    // Create new appender for this runner (do slow I/O without holding the lock)
    let settings = match AppSettings::load() {
        Ok(settings) => settings,
        Err(_e) => {
            // Create a fallback settings object to avoid infinite recursion
            let data_dir = "/Users/dstorey/Library/Application Support/Cloacina".to_string();
            AppSettings {
                data_directory: data_dir.clone(),
                app_database_path: format!("{}/cloacina-app.db", data_dir),
                log_directory: format!("{}/logs", data_dir),
                log_level: "info".to_string(),
                max_log_files: 10,
            }
        }
    };

    let runners_dir = std::path::Path::new(&settings.log_directory).join("runners");
    std::fs::create_dir_all(&runners_dir)?;

    // Use the runner name from span context instead of registry lookup
    let runner_name_str = runner_name
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("unknown");

    let filename = format!("{}-{}", runner_name_str, runner_id);
    let appender = DailyRollingAppender::new(runners_dir.to_string_lossy().to_string(), filename);

    let appender_arc = Arc::new(Mutex::new(appender));

    // Now acquire the lock again to insert the new appender
    {
        let mut appenders = appenders_registry.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Lock error: {}", e))
        })?;

        // Double-check that another thread didn't create it while we were doing I/O
        if let Some(existing_appender) = appenders.get(runner_id) {
            return Ok(existing_appender.clone());
        }

        appenders.insert(runner_id.to_string(), appender_arc.clone());
    }

    Ok(appender_arc)
}

// Initialize runner loggers registry
pub fn initialize_runner_logging() -> Result<()> {
    let registry = Arc::new(Mutex::new(HashMap::new()));
    RUNNER_LOGGERS.set(registry).map_err(|_| {
        anyhow::anyhow!("Failed to set runner loggers registry - already initialized")
    })?;
    Ok(())
}

// Create a runner-specific logger
pub fn create_runner_logger(runner_id: &str, runner_name: &str) -> Result<()> {
    let settings = AppSettings::load()?;
    let logger = RunnerLogger::new(
        runner_id.to_string(),
        runner_name.to_string(),
        settings.log_directory.clone(),
    );

    // Ensure runner log directory exists
    let runner_log_dir = logger.get_runner_log_directory();
    std::fs::create_dir_all(&runner_log_dir)?;

    // Store in registry
    if let Some(registry) = RUNNER_LOGGERS.get() {
        let mut loggers = registry
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock runner loggers registry: {}", e))?;
        loggers.insert(runner_id.to_string(), logger);
    }

    tracing::info!(
        runner_id = %runner_id,
        runner_name = %runner_name,
        log_directory = %runner_log_dir.display(),
        "Created runner-specific logger"
    );

    Ok(())
}

// Remove a runner logger
pub fn remove_runner_logger(runner_id: &str) -> Result<()> {
    if let Some(registry) = RUNNER_LOGGERS.get() {
        let mut loggers = registry
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock runner loggers registry: {}", e))?;
        if loggers.remove(runner_id).is_some() {
            tracing::info!(runner_id = %runner_id, "Removed runner logger");
        }
    }

    // Also remove the runner appender
    if let Some(appenders_registry) = RUNNER_APPENDERS.get() {
        if let Ok(mut appenders) = appenders_registry.lock() {
            appenders.remove(runner_id);
        }
    }

    Ok(())
}

// Get runner logger information

// List all runner loggers
pub fn list_runner_loggers() -> Vec<RunnerLogger> {
    RUNNER_LOGGERS
        .get()
        .and_then(|registry| registry.lock().ok())
        .map(|loggers| loggers.values().cloned().collect())
        .unwrap_or_default()
}

pub fn initialize_logging() -> Result<()> {
    // Load settings to get log configuration
    let settings = AppSettings::load()?;

    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&settings.log_directory)?;

    // Simple app writer for main application logs
    let default_appender =
        DailyRollingAppender::new(settings.log_directory.clone(), "cloacina-app".to_string());
    let app_writer = SimpleAppWriter::new(default_appender);

    // Use non-blocking writer for performance
    let (non_blocking_appender, guard) = non_blocking(app_writer);

    // Leak the guard to keep it alive forever
    Box::leak(Box::new(guard));

    // Create initial filter
    let initial_filter = create_filter(&settings)?;

    // Create reloadable filter layer
    let (filter_layer, reload_handle) = reload::Layer::new(initial_filter);

    // Store the reload handle globally
    RELOAD_HANDLE.set(reload_handle).map_err(|_| {
        anyhow::anyhow!("Failed to set reload handle - logging already initialized")
    })?;

    // STEP 2: Context-aware file layer - routes logs to runner-specific files
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_appender)
        .with_ansi(false);

    // Console layer with filter for app logs only
    let console_filter = EnvFilter::from_default_env()
        .add_directive("cloacina_app=info".parse()?) // App logs at info level
        .add_directive("cloacina=warn".parse()?); // Only warnings/errors from cloacina library

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(console_filter);

    // Create the runner context layer for routing runner logs
    let runner_layer = RunnerContextLayer::new();

    // Initialize the subscriber with all layers
    // Apply filter layer first, then add other layers
    // TOGGLE: Comment out runner_layer if it causes hanging
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(runner_layer) // Comment this line to disable runner routing
        .with(file_layer)
        .with(console_layer)
        .init();

    // Initialize runner logging registry
    initialize_runner_logging()?;

    // Clean up old log files
    cleanup_old_logs(&settings)?;

    tracing::info!(
        "Logging initialized - level: {}, directory: {}",
        settings.log_level,
        settings.log_directory
    );

    Ok(())
}

fn create_filter(settings: &AppSettings) -> Result<EnvFilter> {
    let log_level_str = settings.log_level.to_lowercase();
    let filter = EnvFilter::from_default_env()
        .add_directive(settings.log_level.parse()?)
        .add_directive(format!("cloacina={}", log_level_str).parse()?) // Use user's log level for cloacina
        .add_directive(format!("cloacina_app={}", log_level_str).parse()?); // Use user's log level for app

    Ok(filter)
}

fn cleanup_old_logs(settings: &AppSettings) -> Result<()> {
    let log_dir = std::path::Path::new(&settings.log_directory);

    if !log_dir.exists() {
        return Ok(());
    }

    // Clean up main app log files
    cleanup_app_logs(log_dir, settings)?;

    // Clean up runner log files
    cleanup_runner_logs(log_dir, settings)?;

    Ok(())
}

fn cleanup_app_logs(log_dir: &std::path::Path, settings: &AppSettings) -> Result<()> {
    // Get all main app log files
    let mut log_files: Vec<_> = std::fs::read_dir(log_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            // Only consider files that match our log pattern: cloacina-app-YYYY-MM-DD.log
            if path.is_file() {
                let filename = path.file_name()?.to_str()?;
                if filename.starts_with("cloacina-app-") && filename.ends_with(".log") {
                    let metadata = entry.metadata().ok()?;
                    Some((path, metadata.modified().ok()?))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Sort by modification time (newest first)
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove files beyond the max count
    for (path, _) in log_files.iter().skip(settings.max_log_files as usize) {
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!("Failed to remove old app log file {:?}: {}", path, e);
        } else {
            tracing::info!("Removed old app log file: {:?}", path);
        }
    }

    Ok(())
}

fn cleanup_runner_logs(log_dir: &std::path::Path, settings: &AppSettings) -> Result<()> {
    let runners_dir = log_dir.join("runners");

    if !runners_dir.exists() {
        return Ok(());
    }

    // Get all runner log files
    let mut runner_log_files: Vec<_> = std::fs::read_dir(&runners_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name()?.to_str()?;
                if filename.ends_with(".log") {
                    let metadata = entry.metadata().ok()?;
                    Some((path, metadata.modified().ok()?))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Sort by modification time (newest first)
    runner_log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Keep more runner logs since there might be multiple runners
    let max_runner_logs = (settings.max_log_files as usize) * 2;

    // Remove files beyond the max count
    for (path, _) in runner_log_files.iter().skip(max_runner_logs) {
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!("Failed to remove old runner log file {:?}: {}", path, e);
        } else {
            tracing::info!("Removed old runner log file: {:?}", path);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn reload_logging_config() -> Result<(), String> {
    // Load current settings
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;

    // Get the reload handle
    let reload_handle = RELOAD_HANDLE
        .get()
        .ok_or("Logging not initialized or reload handle not available")?;

    // Create new filter with updated settings
    let new_filter =
        create_filter(&settings).map_err(|e| format!("Failed to create new filter: {}", e))?;

    // Reload the filter
    reload_handle
        .reload(new_filter)
        .map_err(|e| format!("Failed to reload logging filter: {}", e))?;

    tracing::info!(
        "Logging configuration reloaded successfully. New level: {}",
        settings.log_level
    );

    Ok(())
}

#[tauri::command]
pub async fn get_log_files(settings: AppSettings) -> Result<Vec<String>, String> {
    let log_dir = std::path::Path::new(&settings.log_directory);

    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut log_files: Vec<String> = std::fs::read_dir(log_dir)
        .map_err(|e| format!("Failed to read log directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name()?.to_str()?;
                if filename.starts_with("cloacina-app-") && filename.ends_with(".log") {
                    Some(filename.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    log_files.sort();
    log_files.reverse(); // Most recent first

    Ok(log_files)
}

#[tauri::command]
pub async fn get_runner_log_files() -> Result<Vec<String>, String> {
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;
    let runners_dir = std::path::Path::new(&settings.log_directory).join("runners");

    if !runners_dir.exists() {
        return Ok(Vec::new());
    }

    let mut log_files: Vec<String> = std::fs::read_dir(&runners_dir)
        .map_err(|e| format!("Failed to read runner log directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name()?.to_str()?;
                if filename.ends_with(".log") {
                    Some(filename.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    log_files.sort();
    log_files.reverse(); // Most recent first

    Ok(log_files)
}

#[tauri::command]
pub async fn get_runner_log_files_for_runner(runner_id: String) -> Result<Vec<String>, String> {
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;
    let runners_dir = std::path::Path::new(&settings.log_directory).join("runners");

    if !runners_dir.exists() {
        return Ok(Vec::new());
    }

    let mut log_files: Vec<String> = std::fs::read_dir(&runners_dir)
        .map_err(|e| format!("Failed to read runner log directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name()?.to_str()?;
                if filename.ends_with(".log") && filename.contains(&runner_id) {
                    Some(filename.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    log_files.sort();
    log_files.reverse(); // Most recent first

    Ok(log_files)
}

#[tauri::command]
pub async fn read_runner_log_file(
    filename: String,
    lines: Option<usize>,
) -> Result<String, String> {
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;
    let log_file_path = std::path::Path::new(&settings.log_directory)
        .join("runners")
        .join(&filename);

    if !log_file_path.exists() {
        return Err(format!("Log file not found: {}", filename));
    }

    // Security check: ensure the file is within the runners directory
    let canonical_log_path = log_file_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve log file path: {}", e))?;
    let canonical_runners_dir = std::path::Path::new(&settings.log_directory)
        .join("runners")
        .canonicalize()
        .map_err(|e| format!("Failed to resolve runners directory: {}", e))?;

    if !canonical_log_path.starts_with(&canonical_runners_dir) {
        return Err("Access denied: file is outside runners directory".to_string());
    }

    let content = std::fs::read_to_string(&log_file_path)
        .map_err(|e| format!("Failed to read log file: {}", e))?;

    // If lines limit specified, return only the last N lines
    if let Some(line_limit) = lines {
        let lines: Vec<&str> = content.lines().collect();
        let start_index = lines.len().saturating_sub(line_limit);
        Ok(lines[start_index..].join("\n"))
    } else {
        Ok(content)
    }
}

#[tauri::command]
pub async fn get_active_runner_loggers() -> Result<Vec<RunnerLogger>, String> {
    Ok(list_runner_loggers())
}

#[tauri::command]
pub async fn get_runner_logs_from_main_log(
    runner_id: String,
    lines: Option<usize>,
) -> Result<String, String> {
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;

    // Get the most recent log file
    let log_dir = std::path::Path::new(&settings.log_directory);
    let current_date = Local::now().format("%Y-%m-%d").to_string();
    let log_file_path = log_dir.join(format!("cloacina-app-{}.log", current_date));

    if !log_file_path.exists() {
        return Err("Current log file not found".to_string());
    }

    let content = std::fs::read_to_string(&log_file_path)
        .map_err(|e| format!("Failed to read log file: {}", e))?;

    // Filter lines that contain the runner_id or runner context
    let runner_lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            // Look for span context with our runner_id
            line.contains(&format!("runner_id={}", runner_id)) ||
            line.contains(&format!("runner{{runner_id={}", runner_id)) ||
            line.contains(&format!("create_runner{{runner_id={}", runner_id)) ||
            line.contains(&format!("start_runner{{runner_id={}", runner_id)) ||
            line.contains(&format!("stop_runner{{runner_id={}", runner_id)) ||
            line.contains(&format!("startup_runner{{runner_id={}", runner_id)) ||
            // Also check if cloacina logs appear within our runner span context
            (line.contains("cloacina::") &&
             (line.contains(&format!("runner_id={}", runner_id)) ||
              line.contains("component=cloacina_runner")))
        })
        .collect();

    // If lines limit specified, return only the last N lines
    let result_lines = if let Some(line_limit) = lines {
        let start_index = runner_lines.len().saturating_sub(line_limit);
        &runner_lines[start_index..]
    } else {
        &runner_lines
    };

    Ok(result_lines.join("\n"))
}

#[tauri::command]
pub async fn get_all_runner_activity_from_logs(lines: Option<usize>) -> Result<String, String> {
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;

    // Get the most recent log file
    let log_dir = std::path::Path::new(&settings.log_directory);
    let current_date = Local::now().format("%Y-%m-%d").to_string();
    let log_file_path = log_dir.join(format!("cloacina-app-{}.log", current_date));

    if !log_file_path.exists() {
        return Err("Current log file not found".to_string());
    }

    let content = std::fs::read_to_string(&log_file_path)
        .map_err(|e| format!("Failed to read log file: {}", e))?;

    // Filter lines that contain runner-related activity
    let runner_lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            // Look for any span contexts with runner information
            line.contains("runner_id=") ||
            line.contains("runner_name=") ||
            line.contains("component=cloacina_runner") ||
            // Look for our specific operation spans
            line.contains("create_runner{") ||
            line.contains("start_runner{") ||
            line.contains("stop_runner{") ||
            line.contains("startup_runner{") ||
            line.contains("runner{") ||
            // Include cloacina library logs that might be within runner context
            line.contains("cloacina::") ||
            // Include runner management messages
            line.contains("Starting runner") ||
            line.contains("Runner ") ||
            line.contains("Created runner")
        })
        .collect();

    // If lines limit specified, return only the last N lines
    let result_lines = if let Some(line_limit) = lines {
        let start_index = runner_lines.len().saturating_sub(line_limit);
        &runner_lines[start_index..]
    } else {
        &runner_lines
    };

    Ok(result_lines.join("\n"))
}
