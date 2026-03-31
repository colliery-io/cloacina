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

// TODO(I-0058/T-0306): Migrate subgraph tests to use #[workflow] macro.
// These tests verify Workflow::subgraph() which requires Workflow objects.
// The new #[workflow] macro auto-registers workflows in the global registry
// rather than returning them as values. Tests need to be rewritten to
// construct workflows from the registry or use the builder API directly.
