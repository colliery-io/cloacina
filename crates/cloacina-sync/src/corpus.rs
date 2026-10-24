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

//! The declarative corpus (COLLIERY-I-0634 decision 1, CLOACINA-T-0992): the
//! connector definitions that ship with cloacina, embedded in this crate so a
//! server and an agent always agree on them.
//!
//! Sources are Airbyte low-code files in `connectors/declarative/sources/`,
//! imported when they are loaded. Destinations are cloacina destination
//! definitions in `connectors/declarative/destinations/`. Every file was
//! written from its API's own documentation (Apache-2.0); see the folder's
//! README.
//!
//! A server can add definitions from a directory ([`Definitions::with_dir`]).
//! A file there is an Airbyte low-code source (`type: DeclarativeSource`), a
//! destination definition (with `objects`), or a source definition; it is
//! named by its file stem, and a name it shares with the corpus replaces the
//! corpus entry.

use std::collections::BTreeMap;
use std::path::Path;

use crate::definition::{ConnectorDefinition, DefinitionError, DestinationDefinition};
use crate::importer::{ImportError, import_yaml};

/// The embedded sources: (name, Airbyte low-code YAML).
pub const SOURCES: &[(&str, &str)] = &[
    (
        "airtable",
        include_str!("../../../connectors/declarative/sources/airtable.yaml"),
    ),
    (
        "asana",
        include_str!("../../../connectors/declarative/sources/asana.yaml"),
    ),
    (
        "coingecko",
        include_str!("../../../connectors/declarative/sources/coingecko.yaml"),
    ),
    (
        "coinpaprika",
        include_str!("../../../connectors/declarative/sources/coinpaprika.yaml"),
    ),
    (
        "exchangerate",
        include_str!("../../../connectors/declarative/sources/exchangerate.yaml"),
    ),
    (
        "frankfurter",
        include_str!("../../../connectors/declarative/sources/frankfurter.yaml"),
    ),
    (
        "github",
        include_str!("../../../connectors/declarative/sources/github.yaml"),
    ),
    (
        "gitlab",
        include_str!("../../../connectors/declarative/sources/gitlab.yaml"),
    ),
    (
        "google-analytics",
        include_str!("../../../connectors/declarative/sources/google-analytics.yaml"),
    ),
    (
        "google-sheets",
        include_str!("../../../connectors/declarative/sources/google-sheets.yaml"),
    ),
    (
        "hubspot",
        include_str!("../../../connectors/declarative/sources/hubspot.yaml"),
    ),
    (
        "intercom",
        include_str!("../../../connectors/declarative/sources/intercom.yaml"),
    ),
    (
        "jira",
        include_str!("../../../connectors/declarative/sources/jira.yaml"),
    ),
    (
        "jsonplaceholder",
        include_str!("../../../connectors/declarative/sources/jsonplaceholder.yaml"),
    ),
    (
        "mediawiki",
        include_str!("../../../connectors/declarative/sources/mediawiki.yaml"),
    ),
    (
        "nasa",
        include_str!("../../../connectors/declarative/sources/nasa.yaml"),
    ),
    (
        "newsapi",
        include_str!("../../../connectors/declarative/sources/newsapi.yaml"),
    ),
    (
        "notion",
        include_str!("../../../connectors/declarative/sources/notion.yaml"),
    ),
    (
        "nps",
        include_str!("../../../connectors/declarative/sources/nps.yaml"),
    ),
    (
        "omdb",
        include_str!("../../../connectors/declarative/sources/omdb.yaml"),
    ),
    (
        "openaq",
        include_str!("../../../connectors/declarative/sources/openaq.yaml"),
    ),
    (
        "openfda",
        include_str!("../../../connectors/declarative/sources/openfda.yaml"),
    ),
    (
        "openlibrary",
        include_str!("../../../connectors/declarative/sources/openlibrary.yaml"),
    ),
    (
        "openweather",
        include_str!("../../../connectors/declarative/sources/openweather.yaml"),
    ),
    (
        "pagerduty",
        include_str!("../../../connectors/declarative/sources/pagerduty.yaml"),
    ),
    (
        "pokeapi",
        include_str!("../../../connectors/declarative/sources/pokeapi.yaml"),
    ),
    (
        "rickandmorty",
        include_str!("../../../connectors/declarative/sources/rickandmorty.yaml"),
    ),
    (
        "slack",
        include_str!("../../../connectors/declarative/sources/slack.yaml"),
    ),
    (
        "spotify",
        include_str!("../../../connectors/declarative/sources/spotify.yaml"),
    ),
    (
        "square",
        include_str!("../../../connectors/declarative/sources/square.yaml"),
    ),
    (
        "stripe",
        include_str!("../../../connectors/declarative/sources/stripe.yaml"),
    ),
    (
        "ticketmaster",
        include_str!("../../../connectors/declarative/sources/ticketmaster.yaml"),
    ),
    (
        "tmdb",
        include_str!("../../../connectors/declarative/sources/tmdb.yaml"),
    ),
    (
        "todoist",
        include_str!("../../../connectors/declarative/sources/todoist.yaml"),
    ),
    (
        "xkcd",
        include_str!("../../../connectors/declarative/sources/xkcd.yaml"),
    ),
    (
        "zendesk",
        include_str!("../../../connectors/declarative/sources/zendesk.yaml"),
    ),
];

/// The embedded destinations: (name, destination definition YAML).
pub const DESTINATIONS: &[(&str, &str)] = &[
    (
        "hubspot-dest",
        include_str!("../../../connectors/declarative/destinations/hubspot-dest.yaml"),
    ),
    (
        "salesforce-dest",
        include_str!("../../../connectors/declarative/destinations/salesforce-dest.yaml"),
    ),
];

/// A definition could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("no declarative {kind} named `{name}`")]
    Unknown { kind: &'static str, name: String },
    #[error("declarative source `{name}`: {error}")]
    Import { name: String, error: ImportError },
    #[error("declarative definition `{name}`: {error}")]
    Definition {
        name: String,
        error: DefinitionError,
    },
    #[error("definitions directory {path}: {error}")]
    Dir { path: String, error: std::io::Error },
}

/// The declarative definitions a process knows: the corpus, and those added
/// from a directory.
#[derive(Debug, Clone, Default)]
pub struct Definitions {
    sources: BTreeMap<String, ConnectorDefinition>,
    destinations: BTreeMap<String, DestinationDefinition>,
}

impl Definitions {
    /// The embedded corpus.
    pub fn corpus() -> Result<Self, CorpusError> {
        let mut d = Self::default();
        for (name, yaml) in SOURCES {
            d.sources
                .insert(name.to_string(), import_source(name, yaml)?);
        }
        for (name, yaml) in DESTINATIONS {
            d.destinations
                .insert(name.to_string(), parse_destination(name, yaml)?);
        }
        Ok(d)
    }

    /// Add every `*.yaml` of `dir`, replacing a corpus entry of the same name.
    pub fn with_dir(mut self, dir: &Path) -> Result<Self, CorpusError> {
        let io = |error| CorpusError::Dir {
            path: dir.display().to_string(),
            error,
        };
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .map_err(io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
            .collect();
        files.sort();
        for path in files {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let yaml = std::fs::read_to_string(&path).map_err(io)?;
            match classify(&yaml) {
                Kind::Airbyte => {
                    self.sources
                        .insert(name.clone(), import_source(&name, &yaml)?);
                }
                Kind::Destination => {
                    self.destinations
                        .insert(name.clone(), parse_destination(&name, &yaml)?);
                }
                Kind::Source => {
                    let def = ConnectorDefinition::from_yaml(&yaml).map_err(|error| {
                        CorpusError::Definition {
                            name: name.clone(),
                            error,
                        }
                    })?;
                    self.sources.insert(name, def);
                }
            }
        }
        Ok(self)
    }

    pub fn source(&self, name: &str) -> Result<&ConnectorDefinition, CorpusError> {
        self.sources.get(name).ok_or_else(|| CorpusError::Unknown {
            kind: "source",
            name: name.to_string(),
        })
    }

    pub fn destination(&self, name: &str) -> Result<&DestinationDefinition, CorpusError> {
        self.destinations
            .get(name)
            .ok_or_else(|| CorpusError::Unknown {
                kind: "destination",
                name: name.to_string(),
            })
    }

    pub fn sources(&self) -> impl Iterator<Item = (&str, &ConnectorDefinition)> {
        self.sources.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn destinations(&self) -> impl Iterator<Item = (&str, &DestinationDefinition)> {
        self.destinations.iter().map(|(k, v)| (k.as_str(), v))
    }
}

enum Kind {
    Airbyte,
    Destination,
    Source,
}

fn classify(yaml: &str) -> Kind {
    let v: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap_or(serde_yaml::Value::Null);
    if v.get("type").and_then(|t| t.as_str()) == Some("DeclarativeSource") {
        Kind::Airbyte
    } else if v.get("objects").is_some() {
        Kind::Destination
    } else {
        Kind::Source
    }
}

fn import_source(name: &str, yaml: &str) -> Result<ConnectorDefinition, CorpusError> {
    import_yaml(name, yaml).map_err(|error| CorpusError::Import {
        name: name.to_string(),
        error,
    })
}

fn parse_destination(name: &str, yaml: &str) -> Result<DestinationDefinition, CorpusError> {
    DestinationDefinition::from_yaml(yaml).map_err(|error| CorpusError::Definition {
        name: name.to_string(),
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{CompileError, compile_destination, compile_source};
    use serde_json::{Map, Value};

    fn names(dir: &str) -> Vec<String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(dir);
        let mut out: Vec<String> = std::fs::read_dir(path)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter_map(|n| n.strip_suffix(".yaml").map(str::to_string))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn the_embedded_lists_match_the_folders() {
        let embedded =
            |t: &[(&str, &str)]| t.iter().map(|(n, _)| n.to_string()).collect::<Vec<_>>();
        assert_eq!(embedded(SOURCES), names("connectors/declarative/sources"));
        assert_eq!(
            embedded(DESTINATIONS),
            names("connectors/declarative/destinations")
        );
        assert_eq!(SOURCES.len(), 36);
    }

    #[test]
    fn the_corpus_imports_without_error() {
        let d = Definitions::corpus().expect("every corpus file imports");
        assert_eq!(d.sources().count(), 36);
        assert_eq!(d.destinations().count(), 2);
        assert!(d.source("github").is_ok());
        assert!(matches!(d.source("nope"), Err(CorpusError::Unknown { .. })));
    }

    /// Compile with a config that has whatever the definition asks for: each
    /// missing key gets a placeholder and the compile is tried again.
    fn compile_all<T>(
        mut f: impl FnMut(&Map<String, Value>) -> Result<T, CompileError>,
    ) -> Result<T, CompileError> {
        let mut user = Map::new();
        loop {
            match f(&user) {
                Err(CompileError::MissingConfig { key, .. }) if !user.contains_key(&key) => {
                    user.insert(key, Value::String("x".into()));
                }
                other => return other,
            }
        }
    }

    #[test]
    fn every_corpus_stream_and_object_compiles() {
        let d = Definitions::corpus().unwrap();
        let mut streams = 0;
        for (name, def) in d.sources() {
            for s in &def.streams {
                compile_all(|u| compile_source(def, &s.name, u, Some("secret")))
                    .unwrap_or_else(|e| panic!("{name}/{}: {e}", s.name));
                streams += 1;
            }
        }
        assert!(streams >= 36, "{streams}");
        for (name, def) in d.destinations() {
            for o in &def.objects {
                compile_all(|u| compile_destination(def, &o.name, u, Some("secret")))
                    .unwrap_or_else(|e| panic!("{name}/{}: {e}", o.name));
            }
        }
    }

    #[test]
    fn a_directory_adds_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mine.yaml"),
            "spec: { name: mine }\nbase_url: https://x\nstreams:\n  - { name: s, path: /s, schema: [ { name: id, type: int64 } ] }\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("github.yaml"),
            SOURCES.iter().find(|(n, _)| *n == "xkcd").unwrap().1,
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
        let d = Definitions::corpus().unwrap().with_dir(dir.path()).unwrap();
        assert_eq!(d.source("mine").unwrap().spec.name, "mine");
        assert_eq!(
            d.source("github").unwrap().spec.name,
            "github",
            "named by the file"
        );
        assert!(
            d.source("github").unwrap().base_url.contains("xkcd"),
            "the directory's file wins"
        );
        std::fs::write(
            dir.path().join("bad.yaml"),
            "spec: { name: bad }\nbase_url: x\nstreams: []\n",
        )
        .unwrap();
        assert!(Definitions::corpus().unwrap().with_dir(dir.path()).is_err());
    }
}
