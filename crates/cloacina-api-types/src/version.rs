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

//! Release versions between a server, its agents and its clients
//! (CLOACINA-T-1086).
//!
//! - An agent registers with its release. The server accepts an agent of its
//!   own minor (not a newer patch) and of the minor before; it refuses an
//!   older or a newer one ([`check_agent_release`]). An agent that sends no
//!   release is from before the field: it counts as
//!   [`RELEASE_OF_AN_AGENT_WITH_NO_VERSION`].
//! - The server tells its release in `/health` and in the header
//!   [`VERSION_HEADER`] on every answer. A client of another minor gets a
//!   warning ([`client_skew_warning`]), never a refusal.

use std::fmt;

/// The release of this build. The server, the agent, `cloacinactl` and the
/// SDKs are released together, with one version.
pub const RELEASE: &str = env!("CARGO_PKG_VERSION");

/// The header with the server's release, on every answer of the server.
pub const VERSION_HEADER: &str = "x-cloacina-version";

/// The release an agent counts as when its register has no `agent_version`:
/// the field came after 0.12.
pub const RELEASE_OF_AN_AGENT_WITH_NO_VERSION: &str = "0.12.0";

/// The error code of a register refused for the agent's release.
pub const AGENT_VERSION_UNSUPPORTED: &str = "agent_version_unsupported";

/// A release `major.minor.patch`; a pre-release or build suffix is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Release {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Release {
    pub fn parse(text: &str) -> Result<Self, String> {
        let core = text.trim().split(['-', '+']).next().unwrap_or_default();
        let parts: Vec<&str> = core.split('.').collect();
        let bad = || format!("'{text}' is not a release (major.minor.patch)");
        if parts.len() != 3 {
            return Err(bad());
        }
        let n = |p: &str| p.parse::<u64>().map_err(|_| bad());
        Ok(Self {
            major: n(parts[0])?,
            minor: n(parts[1])?,
            patch: n(parts[2])?,
        })
    }

    fn same_minor(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor
    }

    /// `0.13`.
    pub fn minor_label(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    /// The minor before this one, if there is one in the same major.
    fn previous_minor(&self) -> Option<Self> {
        self.minor.checked_sub(1).map(|minor| Self {
            major: self.major,
            minor,
            patch: 0,
        })
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Whether a server of release `server` accepts an agent of release `agent`
/// (`None`: the agent sent none). `Err` is the message of the refusal, with
/// both releases and the step to take.
pub fn check_agent_release(server: &str, agent: Option<&str>) -> Result<(), String> {
    let server = Release::parse(server)?;
    let sent_none = agent.is_none();
    let sent = agent.unwrap_or(RELEASE_OF_AN_AGENT_WITH_NO_VERSION);
    let Ok(agent) = Release::parse(sent) else {
        return Err(format!(
            "agent release '{sent}' cannot be read; this server is {server}: run an agent of {}",
            accepted(&server)
        ));
    };
    let unnamed = if sent_none {
        " (it sent no release; an agent from before 0.13 counts as 0.12)"
    } else {
        ""
    };
    if agent > server {
        return Err(format!(
            "agent {agent}{unnamed} is newer than server {server}: upgrade the server first"
        ));
    }
    let previous = server.previous_minor();
    if agent.same_minor(&server) || previous.is_some_and(|p| agent.same_minor(&p)) {
        return Ok(());
    }
    Err(format!(
        "agent {agent}{unnamed} is older than this server accepts ({}; server {server}): \
         upgrade the agent",
        accepted(&server)
    ))
}

/// `0.13 or 0.14`, the minors a server accepts.
fn accepted(server: &Release) -> String {
    match server.previous_minor() {
        Some(previous) => format!("{} or {}", previous.minor_label(), server.minor_label()),
        None => server.minor_label(),
    }
}

/// The warning a client of release `client` gives when the server is of
/// release `server` and of another minor. `None`: same minor, or a release
/// that cannot be read.
pub fn client_skew_warning(client: &str, server: &str) -> Option<String> {
    let (Ok(c), Ok(s)) = (Release::parse(client), Release::parse(server)) else {
        return None;
    };
    if c.same_minor(&s) {
        return None;
    }
    let step = if c > s {
        "this client is newer than the server; some calls may not exist there"
    } else {
        "upgrade this client"
    };
    Some(format!(
        "warning: this client is {c} and the server is {s} ({step})"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_accepts_its_own_minor_and_the_one_before() {
        assert_eq!(check_agent_release("0.14.1", Some("0.14.1")), Ok(()));
        assert_eq!(check_agent_release("0.14.1", Some("0.14.0")), Ok(()));
        assert_eq!(check_agent_release("0.14.1", Some("0.13.7")), Ok(()));
    }

    #[test]
    fn an_older_agent_is_refused_with_both_releases() {
        let e = check_agent_release("0.14.1", Some("0.12.3")).unwrap_err();
        assert!(e.contains("0.12.3") && e.contains("0.14.1"), "{e}");
        assert!(e.contains("0.13 or 0.14"), "{e}");
        assert!(e.contains("upgrade the agent"), "{e}");
    }

    #[test]
    fn a_newer_agent_is_refused_minor_or_patch() {
        let e = check_agent_release("0.14.1", Some("0.15.0")).unwrap_err();
        assert!(e.contains("0.15.0") && e.contains("0.14.1"), "{e}");
        assert!(e.contains("upgrade the server first"), "{e}");
        assert!(check_agent_release("0.14.1", Some("0.14.2")).is_err());
    }

    #[test]
    fn an_agent_with_no_release_counts_as_0_12() {
        assert_eq!(check_agent_release("0.13.0", None), Ok(()));
        assert_eq!(check_agent_release("0.12.0", None), Ok(()));
        let e = check_agent_release("0.14.0", None).unwrap_err();
        assert!(e.contains("0.12.0") && e.contains("sent no release"), "{e}");
    }

    #[test]
    fn an_unreadable_agent_release_is_refused() {
        assert!(check_agent_release("0.14.0", Some("latest")).is_err());
    }

    #[test]
    fn a_client_of_another_minor_gets_a_warning() {
        assert_eq!(client_skew_warning("0.14.0", "0.14.3"), None);
        let newer = client_skew_warning("0.15.0", "0.14.3").unwrap();
        assert!(
            newer.contains("0.15.0") && newer.contains("0.14.3"),
            "{newer}"
        );
        assert!(newer.contains("newer"), "{newer}");
        let older = client_skew_warning("0.13.0", "0.14.3").unwrap();
        assert!(older.contains("upgrade this client"), "{older}");
        assert_eq!(client_skew_warning("0.14.0", "garbage"), None);
    }

    #[test]
    fn this_release_reads() {
        Release::parse(RELEASE).unwrap();
    }
}
