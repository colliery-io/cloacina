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

//! Trigger-less computation graph registration.
//!
//! T-0552 (I-0102 follow-up) relocated `TriggerlessGraphFn`,
//! `TriggerlessGraphRegistration`, and the `TriggerlessGraph` trait into
//! `cloacina-workflow-plugin` so packaged cdylibs can collect
//! `TriggerlessGraphEntry` inventory entries at link time. Engine paths
//! re-export.

pub use cloacina_workflow_plugin::{
    TriggerlessGraph, TriggerlessGraphFn, TriggerlessGraphRegistration,
};
