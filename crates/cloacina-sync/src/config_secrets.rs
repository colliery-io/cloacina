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

//! The secret fields of a sync's connector config (COLLIERY-I-0635 decision
//! 13 / CLOACINA-T-1008), weir's convention until connectors read secrets
//! (COLLIERY-I-0651). A database connector takes its password in its config,
//! which is stored in the sync's params: reads mask those fields with a fixed
//! placeholder, and an update that sends the placeholder back keeps the
//! stored value.

use serde_json::{Map, Value};

use crate::params::ParamError;

/// What a masked field reads as, and what an update sends to keep it.
pub const SECRET_PLACEHOLDER: &str = "__cloacina_secret_unchanged__";

/// The params that hold a side's config.
const CONFIG_KEYS: [(&str, &str); 2] = [
    ("source", "source_config"),
    ("destination", "destination_config"),
];

/// A config key whose value is a secret: its name says so, or it is a
/// connection URL, which may hold a password.
pub fn is_secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "credential",
    ]
    .iter()
    .any(|s| k.contains(s))
        || matches!(
            k.as_str(),
            "url" | "uri" | "dsn" | "connection_string" | "connection_url"
        )
}

/// A side's config as an object, and whether it was stored as JSON text.
fn config_of(params: &Map<String, Value>, key: &str) -> Option<(Map<String, Value>, bool)> {
    match params.get(key)? {
        Value::Object(o) => Some((o.clone(), false)),
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(Value::Object(o)) => Some((o, true)),
            _ => None,
        },
        _ => None,
    }
}

fn put(params: &mut Map<String, Value>, key: &str, config: Map<String, Value>, as_text: bool) {
    let v = Value::Object(config);
    params.insert(
        key.to_string(),
        if as_text {
            Value::String(v.to_string())
        } else {
            v
        },
    );
}

/// Mask the secret fields of each side's config, for a read.
pub fn mask(params: &mut Map<String, Value>) {
    for (_, key) in CONFIG_KEYS {
        let Some((mut config, as_text)) = config_of(params, key) else {
            continue;
        };
        let mut changed = false;
        for (k, v) in config.iter_mut() {
            if is_secret_key(k) && !v.is_null() && v.as_str() != Some("") {
                *v = Value::String(SECRET_PLACEHOLDER.into());
                changed = true;
            }
        }
        if changed {
            put(params, key, config, as_text);
        }
    }
}

/// Put the stored values back where an update sent the placeholder. A
/// placeholder with no stored value to keep is refused.
pub fn keep(old: &Map<String, Value>, new: &mut Map<String, Value>) -> Result<(), ParamError> {
    for (side, key) in CONFIG_KEYS {
        let Some((mut config, as_text)) = config_of(new, key) else {
            continue;
        };
        let stored = config_of(old, key).map(|(c, _)| c).unwrap_or_default();
        let mut changed = false;
        for (k, v) in config.iter_mut() {
            if v.as_str() == Some(SECRET_PLACEHOLDER) {
                *v = stored
                    .get(k)
                    .cloned()
                    .ok_or_else(|| ParamError::Placeholder {
                        side,
                        key: k.clone(),
                    })?;
                changed = true;
            }
        }
        if changed {
            put(new, key, config, as_text);
        }
    }
    Ok(())
}

/// A new sync has no stored values: a placeholder in its config is refused.
pub fn refuse_placeholders(params: &Map<String, Value>) -> Result<(), ParamError> {
    keep(&Map::new(), &mut params.clone())
}

/// One side's config with its placeholders filled from the stored params,
/// for a check or discover made while editing.
pub fn keep_side(
    stored: &Map<String, Value>,
    side: &str,
    config: &mut Map<String, Value>,
) -> Result<(), ParamError> {
    let key = format!("{side}_config");
    let mut new = Map::new();
    new.insert(key.clone(), Value::Object(config.clone()));
    keep(stored, &mut new)?;
    if let Some(Value::Object(o)) = new.remove(&key) {
        *config = o;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params() -> Map<String, Value> {
        json!({
            "sync": "s",
            "source_config": json!({"url": "postgres://u:pw@h/db", "table": "t"}).to_string(),
            "destination_config": {"api_token": "tok", "region": "eu"},
        })
        .as_object()
        .unwrap()
        .clone()
    }

    #[test]
    fn a_read_masks_the_secret_fields_and_nothing_else() {
        let mut p = params();
        mask(&mut p);
        let src: Value = serde_json::from_str(p["source_config"].as_str().unwrap()).unwrap();
        assert_eq!(src["url"], SECRET_PLACEHOLDER);
        assert_eq!(src["table"], "t");
        assert_eq!(p["destination_config"]["api_token"], SECRET_PLACEHOLDER);
        assert_eq!(p["destination_config"]["region"], "eu");
        assert!(!serde_json::to_string(&p).unwrap().contains("pw@"));
    }

    #[test]
    fn an_update_that_sends_the_placeholder_keeps_the_stored_value() {
        let old = params();
        let mut new = old.clone();
        mask(&mut new);
        keep(&old, &mut new).unwrap();
        assert_eq!(new, old);
    }

    #[test]
    fn a_placeholder_with_nothing_stored_is_refused() {
        let mut p = params();
        mask(&mut p);
        let err = refuse_placeholders(&p).unwrap_err();
        assert!(err.to_string().contains("url"), "{err}");
    }
}
