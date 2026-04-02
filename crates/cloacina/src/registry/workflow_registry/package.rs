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

//! Package format detection utilities.

use super::WorkflowRegistryImpl;
use crate::registry::traits::RegistryStorage;

impl<S: RegistryStorage> WorkflowRegistryImpl<S> {
    /// Check if package data is a bzip2-compressed `.cloacina` source archive.
    pub(super) fn is_cloacina_package(data: &[u8]) -> bool {
        // bzip2 magic: 0x42 0x5a ('B' 'Z')
        data.len() >= 2 && data[0] == 0x42 && data[1] == 0x5a
    }
}
