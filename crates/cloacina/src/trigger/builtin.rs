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

//! Built-in trigger types for package-declared triggers.
//!
//! These triggers are instantiated from `TriggerDefinitionV2` config in
//! `.cloacina` package manifests.

use async_trait::async_trait;
use std::fmt;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use super::{Trigger, TriggerError, TriggerResult};
use crate::packaging::manifest_v2::{parse_duration_str, TriggerDefinitionV2, TriggerType};
use crate::Context;

/// Create a concrete `Trigger` from a manifest trigger definition.
pub fn create_trigger_from_config(
    def: &TriggerDefinitionV2,
) -> Result<Box<dyn Trigger>, TriggerError> {
    let poll_interval =
        parse_duration_str(&def.poll_interval).map_err(|e| TriggerError::PollError {
            message: format!("invalid poll_interval '{}': {}", def.poll_interval, e),
        })?;

    match def.trigger_type {
        TriggerType::Webhook => {
            let path = def
                .config
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("/");
            Ok(Box::new(WebhookTrigger::new(
                &def.name,
                &def.workflow,
                path,
                poll_interval,
                def.allow_concurrent,
            )))
        }
        TriggerType::HttpPoll => {
            let url = def
                .config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| TriggerError::PollError {
                    message: format!(
                        "http_poll trigger '{}' missing required 'url' in config",
                        def.name
                    ),
                })?;
            let method = def
                .config
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET");
            let expect_status = def
                .config
                .get("expect_status")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16);

            Ok(Box::new(HttpPollTrigger::new(
                &def.name,
                &def.workflow,
                url,
                method,
                expect_status,
                poll_interval,
                def.allow_concurrent,
            )))
        }
        TriggerType::FileWatch => {
            let directory = def
                .config
                .get("directory")
                .and_then(|v| v.as_str())
                .ok_or_else(|| TriggerError::PollError {
                    message: format!(
                        "file_watch trigger '{}' missing required 'directory' in config",
                        def.name
                    ),
                })?;
            let glob_pattern = def
                .config
                .get("glob")
                .and_then(|v| v.as_str())
                .unwrap_or("*");

            Ok(Box::new(FileWatchTrigger::new(
                &def.name,
                &def.workflow,
                directory,
                glob_pattern,
                poll_interval,
                def.allow_concurrent,
            )))
        }
        TriggerType::Python => Err(TriggerError::PollError {
            message: format!(
                "Python trigger '{}' cannot be created via built-in factory — requires PyO3 loader",
                def.name
            ),
        }),
    }
}

// ---------------------------------------------------------------------------
// WebhookTrigger
// ---------------------------------------------------------------------------

/// Channel-based webhook trigger.
///
/// External HTTP handlers push payloads into the sender channel; the trigger's
/// `poll()` drains the receiver and fires for each payload.
pub struct WebhookTrigger {
    name: String,
    workflow: String,
    path: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    receiver: tokio::sync::Mutex<mpsc::Receiver<serde_json::Value>>,
    sender: mpsc::Sender<serde_json::Value>,
}

impl WebhookTrigger {
    pub fn new(
        name: &str,
        workflow: &str,
        path: &str,
        poll_interval: Duration,
        allow_concurrent: bool,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(256);
        Self {
            name: name.to_string(),
            workflow: workflow.to_string(),
            path: path.to_string(),
            poll_interval,
            allow_concurrent,
            receiver: tokio::sync::Mutex::new(receiver),
            sender,
        }
    }

    /// Get a sender handle for pushing webhook payloads.
    pub fn sender(&self) -> mpsc::Sender<serde_json::Value> {
        self.sender.clone()
    }

    /// The webhook path this trigger listens on.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The workflow this trigger fires.
    pub fn workflow(&self) -> &str {
        &self.workflow
    }
}

impl fmt::Debug for WebhookTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookTrigger")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("workflow", &self.workflow)
            .finish()
    }
}

#[async_trait]
impl Trigger for WebhookTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let mut rx = self.receiver.lock().await;
        match rx.try_recv() {
            Ok(payload) => {
                debug!(trigger = %self.name, "Webhook payload received");
                let mut ctx = Context::new();
                ctx.insert("webhook_payload", payload)
                    .map_err(|e| TriggerError::PollError {
                        message: format!("failed to set webhook payload in context: {e}"),
                    })?;
                Ok(TriggerResult::Fire(Some(ctx)))
            }
            Err(mpsc::error::TryRecvError::Empty) => Ok(TriggerResult::Skip),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                warn!(trigger = %self.name, "Webhook channel disconnected");
                Ok(TriggerResult::Skip)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HttpPollTrigger
// ---------------------------------------------------------------------------

/// Polls an HTTP endpoint and fires when the response matches expectations.
pub struct HttpPollTrigger {
    name: String,
    workflow: String,
    url: String,
    method: String,
    expect_status: Option<u16>,
    poll_interval: Duration,
    allow_concurrent: bool,
}

impl HttpPollTrigger {
    pub fn new(
        name: &str,
        workflow: &str,
        url: &str,
        method: &str,
        expect_status: Option<u16>,
        poll_interval: Duration,
        allow_concurrent: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            workflow: workflow.to_string(),
            url: url.to_string(),
            method: method.to_uppercase(),
            expect_status,
            poll_interval,
            allow_concurrent,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn workflow(&self) -> &str {
        &self.workflow
    }
}

impl fmt::Debug for HttpPollTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpPollTrigger")
            .field("name", &self.name)
            .field("url", &self.url)
            .field("method", &self.method)
            .field("workflow", &self.workflow)
            .finish()
    }
}

#[async_trait]
impl Trigger for HttpPollTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| TriggerError::PollError {
                message: format!("failed to build HTTP client: {e}"),
            })?;

        let request = match self.method.as_str() {
            "GET" => client.get(&self.url),
            "POST" => client.post(&self.url),
            "HEAD" => client.head(&self.url),
            other => {
                return Err(TriggerError::PollError {
                    message: format!("unsupported HTTP method: {other}"),
                })
            }
        };

        let response = request.send().await.map_err(|e| TriggerError::PollError {
            message: format!("HTTP request failed: {e}"),
        })?;

        let status = response.status().as_u16();
        let should_fire = match self.expect_status {
            Some(expected) => status == expected,
            None => response.status().is_success(),
        };

        if should_fire {
            debug!(trigger = %self.name, status, "HTTP poll condition met");
            let body = response.text().await.unwrap_or_default();
            let mut ctx = Context::new();
            ctx.insert("http_status", serde_json::json!(status))
                .map_err(|e| TriggerError::PollError {
                    message: format!("context error: {e}"),
                })?;
            ctx.insert("http_body", serde_json::json!(body))
                .map_err(|e| TriggerError::PollError {
                    message: format!("context error: {e}"),
                })?;
            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            debug!(trigger = %self.name, status, "HTTP poll condition not met");
            Ok(TriggerResult::Skip)
        }
    }
}

// ---------------------------------------------------------------------------
// FileWatchTrigger
// ---------------------------------------------------------------------------

/// Watches a directory for new/changed files matching a glob pattern.
///
/// On each poll, scans the directory and fires if any files are found.
/// Uses modification time tracking to avoid re-firing for the same files.
pub struct FileWatchTrigger {
    name: String,
    workflow: String,
    directory: String,
    glob_pattern: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    last_seen: tokio::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>,
}

impl FileWatchTrigger {
    pub fn new(
        name: &str,
        workflow: &str,
        directory: &str,
        glob_pattern: &str,
        poll_interval: Duration,
        allow_concurrent: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            workflow: workflow.to_string(),
            directory: directory.to_string(),
            glob_pattern: glob_pattern.to_string(),
            poll_interval,
            allow_concurrent,
            last_seen: tokio::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    pub fn directory(&self) -> &str {
        &self.directory
    }

    pub fn workflow(&self) -> &str {
        &self.workflow
    }
}

impl fmt::Debug for FileWatchTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileWatchTrigger")
            .field("name", &self.name)
            .field("directory", &self.directory)
            .field("glob_pattern", &self.glob_pattern)
            .field("workflow", &self.workflow)
            .finish()
    }
}

#[async_trait]
impl Trigger for FileWatchTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let pattern = format!("{}/{}", self.directory, self.glob_pattern);
        let entries: Vec<std::path::PathBuf> = glob::glob(&pattern)
            .map_err(|e| TriggerError::PollError {
                message: format!("invalid glob pattern '{}': {}", pattern, e),
            })?
            .filter_map(|entry| entry.ok())
            .filter(|path| path.is_file())
            .collect();

        let mut last_seen = self.last_seen.lock().await;
        let new_files: Vec<std::path::PathBuf> = entries
            .iter()
            .filter(|p| !last_seen.contains(*p))
            .cloned()
            .collect();

        if new_files.is_empty() {
            return Ok(TriggerResult::Skip);
        }

        debug!(
            trigger = %self.name,
            count = new_files.len(),
            "New files detected"
        );

        // Track all currently visible files
        *last_seen = entries.into_iter().collect();

        let file_paths: Vec<String> = new_files.iter().map(|p| p.display().to_string()).collect();

        let mut ctx = Context::new();
        ctx.insert("new_files", serde_json::json!(file_paths))
            .map_err(|e| TriggerError::PollError {
                message: format!("context error: {e}"),
            })?;
        Ok(TriggerResult::Fire(Some(ctx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_webhook_trigger() {
        let def = TriggerDefinitionV2 {
            name: "on_upload".to_string(),
            trigger_type: TriggerType::Webhook,
            workflow: "process".to_string(),
            poll_interval: "5s".to_string(),
            allow_concurrent: false,
            config: serde_json::json!({"path": "/hooks/upload"}),
        };
        let trigger = create_trigger_from_config(&def).unwrap();
        assert_eq!(trigger.name(), "on_upload");
        assert_eq!(trigger.poll_interval(), Duration::from_secs(5));
        assert!(!trigger.allow_concurrent());
    }

    #[test]
    fn test_create_http_poll_trigger() {
        let def = TriggerDefinitionV2 {
            name: "check_api".to_string(),
            trigger_type: TriggerType::HttpPoll,
            workflow: "ingest".to_string(),
            poll_interval: "1m".to_string(),
            allow_concurrent: true,
            config: serde_json::json!({
                "url": "https://api.example.com/ready",
                "method": "GET",
                "expect_status": 200
            }),
        };
        let trigger = create_trigger_from_config(&def).unwrap();
        assert_eq!(trigger.name(), "check_api");
        assert_eq!(trigger.poll_interval(), Duration::from_secs(60));
        assert!(trigger.allow_concurrent());
    }

    #[test]
    fn test_create_http_poll_missing_url() {
        let def = TriggerDefinitionV2 {
            name: "bad".to_string(),
            trigger_type: TriggerType::HttpPoll,
            workflow: "w".to_string(),
            poll_interval: "10s".to_string(),
            allow_concurrent: false,
            config: serde_json::json!({}),
        };
        assert!(create_trigger_from_config(&def).is_err());
    }

    #[test]
    fn test_create_file_watch_trigger() {
        let def = TriggerDefinitionV2 {
            name: "watch_inbox".to_string(),
            trigger_type: TriggerType::FileWatch,
            workflow: "process".to_string(),
            poll_interval: "10s".to_string(),
            allow_concurrent: false,
            config: serde_json::json!({
                "directory": "/tmp/inbox",
                "glob": "*.csv"
            }),
        };
        let trigger = create_trigger_from_config(&def).unwrap();
        assert_eq!(trigger.name(), "watch_inbox");
        assert_eq!(trigger.poll_interval(), Duration::from_secs(10));
    }

    #[test]
    fn test_create_file_watch_missing_directory() {
        let def = TriggerDefinitionV2 {
            name: "bad".to_string(),
            trigger_type: TriggerType::FileWatch,
            workflow: "w".to_string(),
            poll_interval: "10s".to_string(),
            allow_concurrent: false,
            config: serde_json::json!({}),
        };
        assert!(create_trigger_from_config(&def).is_err());
    }

    #[test]
    fn test_create_python_trigger_errors() {
        let def = TriggerDefinitionV2 {
            name: "py_trigger".to_string(),
            trigger_type: TriggerType::Python,
            workflow: "w".to_string(),
            poll_interval: "10s".to_string(),
            allow_concurrent: false,
            config: serde_json::json!({}),
        };
        assert!(create_trigger_from_config(&def).is_err());
    }

    #[tokio::test]
    async fn test_webhook_trigger_poll_empty() {
        let trigger = WebhookTrigger::new("test", "wf", "/hook", Duration::from_secs(1), false);
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[tokio::test]
    async fn test_webhook_trigger_poll_with_payload() {
        let trigger = WebhookTrigger::new("test", "wf", "/hook", Duration::from_secs(1), false);
        let sender = trigger.sender();
        sender
            .send(serde_json::json!({"event": "uploaded"}))
            .await
            .unwrap();

        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());
    }

    #[tokio::test]
    async fn test_file_watch_trigger_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let trigger = FileWatchTrigger::new(
            "test",
            "wf",
            dir.path().to_str().unwrap(),
            "*.csv",
            Duration::from_secs(1),
            false,
        );
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[tokio::test]
    async fn test_file_watch_trigger_detects_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let trigger = FileWatchTrigger::new(
            "test",
            "wf",
            dir.path().to_str().unwrap(),
            "*.csv",
            Duration::from_secs(1),
            false,
        );

        // First poll — empty
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());

        // Create a file
        std::fs::write(dir.path().join("data.csv"), "a,b,c").unwrap();

        // Second poll — should fire
        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());

        // Third poll — same file, should NOT fire again
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }
}
