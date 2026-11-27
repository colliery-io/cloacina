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

//! Server settings for task logs (CLOACINA-T-1024).
//!
//! The server stores no task output. It can point at the place that does:
//! [`TaskLogUrlTemplate`] is a URL of the operator's log stack with
//! placeholders, and the task API fills it in for each task as `log_url`.
//! The values are the ones on the task's log lines (see
//! [`cloacina::task_logs::task_span`]), so a query built from them finds the
//! lines.
//!
//! The module also holds the format of the server's console log lines.

use std::sync::OnceLock;

use cloacina::task_logs::LogFormat;

/// The placeholders a template can hold.
pub const PLACEHOLDERS: [&str; 4] = ["{tenant}", "{execution_id}", "{task}", "{attempt}"];

/// A log URL with placeholders, checked at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLogUrlTemplate(String);

impl TaskLogUrlTemplate {
    /// Checks a template. It must be an `http://` or `https://` URL: the UI
    /// renders it as a link, so no other scheme is let through.
    pub fn parse(template: &str) -> Result<Self, String> {
        let template = template.trim();
        let lower = template.to_ascii_lowercase();
        let rest = lower
            .strip_prefix("https://")
            .or_else(|| lower.strip_prefix("http://"))
            .ok_or_else(|| "the template must start with http:// or https://".to_string())?;
        if rest.is_empty() || rest.starts_with('/') {
            return Err("the template has no host".to_string());
        }
        if template
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(
                "the template holds a space or a control character: URL-encode it".to_string(),
            );
        }
        Ok(Self(template.to_string()))
    }

    /// The URL for one attempt of a task. Each value is percent-encoded, so
    /// it cannot change the shape of the URL. `task` is the full task name,
    /// as the task API gives it and as the log lines carry it.
    pub fn render(&self, tenant: &str, execution_id: &str, task: &str, attempt: i32) -> String {
        let values = [
            ("{tenant}", percent_encode(tenant)),
            ("{execution_id}", percent_encode(execution_id)),
            ("{task}", percent_encode(task)),
            ("{attempt}", attempt.to_string()),
        ];
        // One pass over the template, so a value that looks like a
        // placeholder is not filled in again.
        let mut out = String::with_capacity(self.0.len() + 64);
        let mut rest = self.0.as_str();
        'scan: while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            rest = &rest[open..];
            for (placeholder, value) in &values {
                if let Some(after) = rest.strip_prefix(placeholder) {
                    out.push_str(value);
                    rest = after;
                    continue 'scan;
                }
            }
            out.push('{');
            rest = &rest[1..];
        }
        out.push_str(rest);
        out
    }
}

/// Percent-encodes all but the unreserved characters of RFC 3986.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

static URL_TEMPLATE: OnceLock<TaskLogUrlTemplate> = OnceLock::new();

/// Gives the template to the server. `main` calls it before `run`. The first
/// call wins.
pub fn install_url_template(template: TaskLogUrlTemplate) {
    let _ = URL_TEMPLATE.set(template);
}

/// The template given with [`install_url_template`]. `None` = no log link.
pub fn url_template() -> Option<&'static TaskLogUrlTemplate> {
    URL_TEMPLATE.get()
}

static LOG_FORMAT: OnceLock<LogFormat> = OnceLock::new();

/// Sets the format of the console log lines. `main` calls it before `run`
/// sets up logging. The first call wins.
pub fn set_log_format(format: LogFormat) {
    let _ = LOG_FORMAT.set(format);
}

/// The format set with [`set_log_format`]; text when none was set.
pub fn log_format() -> LogFormat {
    LOG_FORMAT.get().copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_is_filled_in_with_encoded_values() {
        let template = TaskLogUrlTemplate::parse(
            "https://logs.example.com/search?tenant={tenant}&run={execution_id}&task={task}&try={attempt}",
        )
        .unwrap();
        assert_eq!(
            template.render("acme", "0b6e-11", "acme::pkg::etl::load rows", 2),
            "https://logs.example.com/search?tenant=acme&run=0b6e-11\
             &task=acme%3A%3Apkg%3A%3Aetl%3A%3Aload%20rows&try=2"
        );
    }

    #[test]
    fn a_placeholder_can_repeat_and_other_braces_stay() {
        let template = TaskLogUrlTemplate::parse(
            "https://logs.example.com/{tenant}/{tenant}?q={app=%22agent%22}&x={unknown}",
        )
        .unwrap();
        assert_eq!(
            template.render("t1", "e", "k", 1),
            "https://logs.example.com/t1/t1?q={app=%22agent%22}&x={unknown}"
        );
    }

    #[test]
    fn a_value_cannot_add_a_placeholder_or_a_parameter() {
        let template =
            TaskLogUrlTemplate::parse("https://logs.example.com/?t={tenant}&k={task}").unwrap();
        assert_eq!(
            template.render("{task}", "e", "a&b=c#d", 1),
            "https://logs.example.com/?t=%7Btask%7D&k=a%26b%3Dc%23d"
        );
    }

    #[test]
    fn only_http_and_https_templates_pass() {
        assert!(TaskLogUrlTemplate::parse("http://grafana:3000/explore?x={task}").is_ok());
        assert!(TaskLogUrlTemplate::parse("  HTTPS://logs.example.com/  ").is_ok());
        for bad in [
            "",
            "javascript:alert(1)",
            "logs.example.com/{task}",
            "ftp://logs.example.com/",
            "https://",
            "https:///path",
            "https://logs.example.com/a b",
        ] {
            assert!(TaskLogUrlTemplate::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_log_format_is_text_until_set() {
        // Other tests of this binary do not set it.
        assert_eq!(log_format(), LogFormat::Text);
    }
}
