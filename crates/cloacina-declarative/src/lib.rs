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

//! Declarative connectors for cloacina's Sync (COLLIERY-I-0634).
//!
//! A **declarative connector** is a YAML **connector definition** run on the
//! shared REST runtime (the `rest` and `rest-dest` guests). This crate holds
//! what that needs and nothing of the engine, so a CLI can import and check
//! definitions without a WASM runtime:
//!
//! - [`definition`]: the model, and the compiler that turns a definition, a
//!   stream and a user's config into the runtime's config, the mapping, the
//!   typed schema and the credential spec.
//! - [`importer`]: Airbyte low-code sources to connector definitions.
//! - [`corpus`]: the definitions that ship with cloacina, embedded.

pub mod corpus;
pub mod definition;
pub mod importer;
