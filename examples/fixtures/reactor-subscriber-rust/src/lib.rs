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

//! Cross-package binding fixture (T-0550 / I-0102 T-D).
//!
//! Declares a computation graph that references the reactor declared in
//! `examples/fixtures/reactor-only-rust/` BY NAME (`shared_rx`). When the
//! reconciler loads this package, it must resolve the binding against the
//! already-loaded reactor; loading order matters and is fail-fast (subscriber
//! before publisher → clean rejection naming the missing primitive).

use serde::{Deserialize, Serialize};

cloacina_workflow_plugin::package!();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaIn {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaIn {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriberOutput {
    pub sum: f64,
}

#[cloacina_macros::computation_graph(
    trigger = reactor("shared_rx"),
    graph = {
        combine(alpha, beta) -> output,
    }
)]
pub mod subscriber_graph {
    use super::*;

    pub async fn combine(alpha: Option<&AlphaIn>, beta: Option<&BetaIn>) -> SubscriberOutput {
        SubscriberOutput {
            sum: alpha.map(|a| a.value).unwrap_or(0.0) + beta.map(|b| b.value).unwrap_or(0.0),
        }
    }

    pub async fn output(input: &SubscriberOutput) -> SubscriberOutput {
        input.clone()
    }
}
