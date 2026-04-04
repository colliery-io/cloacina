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

//! End-to-end tests for the `#[computation_graph]` macro.
//!
//! These tests verify that the macro correctly parses topology, validates the
//! graph, and generates a callable async function that routes data correctly.

use cloacina::computation_graph::types::{serialize, GraphResult, InputCache, SourceName};
use serde::{Deserialize, Serialize};

// --- Test boundary types ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlphaData {
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedData {
    pub result: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputConfirmation {
    pub published: bool,
    pub value: f64,
}

// =============================================================================
// Test 1: Linear chain (A -> B -> C)
// =============================================================================

#[cloacina_macros::computation_graph(
    react = when_any(alpha),
    graph = {
        entry(alpha) -> process,
        process -> output,
    }
)]
pub mod linear_chain {
    use super::*;

    pub async fn entry(alpha: Option<&AlphaData>) -> ProcessedData {
        let a = alpha.unwrap();
        ProcessedData {
            result: a.value * 2.0,
        }
    }

    pub async fn process(input: &ProcessedData) -> ProcessedData {
        ProcessedData {
            result: input.result + 10.0,
        }
    }

    pub async fn output(input: &ProcessedData) -> OutputConfirmation {
        OutputConfirmation {
            published: true,
            value: input.result,
        }
    }
}

#[tokio::test]
async fn test_linear_chain() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 5.0 }).unwrap(),
    );

    let result: GraphResult = linear_chain_compiled(&cache).await;
    assert!(result.is_completed());
}

// =============================================================================
// Test 2: Enum routing (A => { X -> B, Y -> C })
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BetaData {
    pub estimate: f64,
}

#[cloacina_macros::computation_graph(
    react = when_any(alpha, beta),
    graph = {
        decision(alpha, beta) => {
            Signal -> signal_handler,
            NoAction -> audit_logger,
        },
    }
)]
pub mod routing_graph {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum DecisionOutcome {
        Signal(SignalData),
        NoAction(NoActionReason),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct SignalData {
        pub output: f64,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct NoActionReason {
        pub reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct AuditRecord {
        pub logged: bool,
    }

    pub async fn decision(alpha: Option<&AlphaData>, beta: Option<&BetaData>) -> DecisionOutcome {
        let a = alpha.map(|a| a.value).unwrap_or(0.0);
        let b = beta.map(|b| b.estimate).unwrap_or(0.0);
        if a + b > 10.0 {
            DecisionOutcome::Signal(SignalData { output: a + b })
        } else {
            DecisionOutcome::NoAction(NoActionReason {
                reason: "below threshold".to_string(),
            })
        }
    }

    pub async fn signal_handler(signal: &SignalData) -> OutputConfirmation {
        OutputConfirmation {
            published: true,
            value: signal.output,
        }
    }

    pub async fn audit_logger(reason: &NoActionReason) -> AuditRecord {
        AuditRecord {
            logged: !reason.reason.is_empty(),
        }
    }
}

#[tokio::test]
async fn test_routing_signal_path() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 8.0 }).unwrap(),
    );
    cache.update(
        SourceName::new("beta"),
        serialize(&BetaData { estimate: 5.0 }).unwrap(),
    );

    let result: GraphResult = routing_graph_compiled(&cache).await;
    assert!(result.is_completed());
}

#[tokio::test]
async fn test_routing_no_action_path() {
    let mut cache = InputCache::new();
    cache.update(
        SourceName::new("alpha"),
        serialize(&AlphaData { value: 2.0 }).unwrap(),
    );
    cache.update(
        SourceName::new("beta"),
        serialize(&BetaData { estimate: 1.0 }).unwrap(),
    );

    let result: GraphResult = routing_graph_compiled(&cache).await;
    assert!(result.is_completed());
}
