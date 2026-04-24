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

//! Graph IR (Internal Representation) for computation graphs.
//!
//! Transforms `ParsedTopology` into a validated, sorted graph structure
//! suitable for code generation. Independent of `syn` types.

use std::collections::{HashMap, HashSet, VecDeque};

use super::parser::{ParsedEdge, ParsedTopology, TriggerSpec};

/// The complete validated graph, ready for code generation.
#[derive(Debug)]
pub struct GraphIR {
    /// How this graph is triggered — bundled reactor, named reactor, or
    /// trigger-less.
    pub trigger: TriggerSpec,
    /// Nodes in topological order (entry nodes first, terminal nodes last)
    pub sorted_nodes: Vec<String>,
    /// Node details keyed by name
    pub nodes: HashMap<String, GraphNode>,
}

/// A node in the graph IR.
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Node function name
    pub name: String,
    /// Cache inputs (accumulator names from parenthesized inputs in topology)
    pub cache_inputs: Vec<String>,
    /// Outgoing edges
    pub edges_out: Vec<GraphEdge>,
    /// Incoming edges (populated during IR build)
    pub edges_in: Vec<IncomingEdge>,
    /// Whether this node is terminal (no outgoing edges)
    pub is_terminal: bool,
}

/// An outgoing edge from a node.
#[derive(Debug, Clone)]
pub enum GraphEdge {
    /// `node -> target` — direct linear connection
    Linear { target: String },
    /// `node => { Variant -> target, ... }` — enum routing
    Routing { variants: Vec<GraphRoutingVariant> },
}

/// A single variant -> target mapping.
#[derive(Debug, Clone)]
pub struct GraphRoutingVariant {
    pub variant_name: String,
    pub target: String,
}

/// An incoming edge to a node (who feeds this node).
#[derive(Debug, Clone)]
pub struct IncomingEdge {
    /// Source node name
    pub from: String,
    /// If this comes from a routing edge, which variant
    pub variant: Option<String>,
}

/// Errors during graph IR construction.
#[derive(Debug, thiserror::Error)]
pub enum GraphIRError {
    #[error("cycle detected in graph: {0}")]
    Cycle(String),

    #[error("node '{0}' referenced in graph but not defined as a function in the module")]
    DanglingReference(String),

    #[error("duplicate edge: node '{from}' has multiple edges to '{to}'")]
    DuplicateEdge { from: String, to: String },
}

impl GraphIR {
    /// Build a GraphIR from a ParsedTopology.
    ///
    /// This resolves all node references, builds the adjacency lists,
    /// computes topological order, and identifies terminal nodes.
    pub fn from_parsed(parsed: ParsedTopology) -> Result<Self, GraphIRError> {
        let mut nodes: HashMap<String, GraphNode> = HashMap::new();

        // First pass: collect all node names and their cache inputs from edges
        for edge in &parsed.edges {
            let from_name = edge.from_name().to_string();
            let from_inputs: Vec<String> =
                edge.from_inputs().iter().map(|i| i.to_string()).collect();

            let node = nodes.entry(from_name.clone()).or_insert_with(|| GraphNode {
                name: from_name.clone(),
                cache_inputs: Vec::new(),
                edges_out: Vec::new(),
                edges_in: Vec::new(),
                is_terminal: false,
            });

            // Merge cache inputs (first declaration wins, subsequent are ignored if same)
            if node.cache_inputs.is_empty() && !from_inputs.is_empty() {
                node.cache_inputs = from_inputs;
            }

            // Collect target nodes (ensure they exist in the map)
            match edge {
                ParsedEdge::Linear { to, .. } => {
                    let to_name = to.to_string();
                    nodes.entry(to_name).or_insert_with(|| GraphNode {
                        name: to.to_string(),
                        cache_inputs: Vec::new(),
                        edges_out: Vec::new(),
                        edges_in: Vec::new(),
                        is_terminal: false,
                    });
                }
                ParsedEdge::Routing { variants, .. } => {
                    for v in variants {
                        let target_name = v.target.to_string();
                        nodes.entry(target_name).or_insert_with(|| GraphNode {
                            name: v.target.to_string(),
                            cache_inputs: Vec::new(),
                            edges_out: Vec::new(),
                            edges_in: Vec::new(),
                            is_terminal: false,
                        });
                    }
                }
            }
        }

        // Second pass: build edges
        for edge in &parsed.edges {
            let from_name = edge.from_name().to_string();

            match edge {
                ParsedEdge::Linear { to, .. } => {
                    let to_name = to.to_string();

                    // Add outgoing edge
                    nodes
                        .get_mut(&from_name)
                        .unwrap()
                        .edges_out
                        .push(GraphEdge::Linear {
                            target: to_name.clone(),
                        });

                    // Add incoming edge
                    nodes
                        .get_mut(&to_name)
                        .unwrap()
                        .edges_in
                        .push(IncomingEdge {
                            from: from_name,
                            variant: None,
                        });
                }
                ParsedEdge::Routing { variants, .. } => {
                    let graph_variants: Vec<GraphRoutingVariant> = variants
                        .iter()
                        .map(|v| GraphRoutingVariant {
                            variant_name: v.variant_name.to_string(),
                            target: v.target.to_string(),
                        })
                        .collect();

                    // Add incoming edges for each variant target
                    for v in &graph_variants {
                        nodes
                            .get_mut(&v.target)
                            .unwrap()
                            .edges_in
                            .push(IncomingEdge {
                                from: from_name.clone(),
                                variant: Some(v.variant_name.clone()),
                            });
                    }

                    // Add outgoing routing edge
                    nodes
                        .get_mut(&from_name)
                        .unwrap()
                        .edges_out
                        .push(GraphEdge::Routing {
                            variants: graph_variants,
                        });
                }
            }
        }

        // Mark terminal nodes (no outgoing edges)
        for node in nodes.values_mut() {
            node.is_terminal = node.edges_out.is_empty();
        }

        // Topological sort
        let sorted_nodes = topological_sort(&nodes)?;

        Ok(GraphIR {
            trigger: parsed.trigger,
            sorted_nodes,
            nodes,
        })
    }

    /// Return the set of accumulator names referenced by the graph's entry
    /// nodes (nodes with no incoming edges). This is the set checked at
    /// compile time against the bound reactor's accumulator list for split
    /// form; it's also what we publish on `ComputationGraphRegistration`.
    pub fn entry_accumulators(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for node in self.entry_nodes() {
            for input in &node.cache_inputs {
                if seen.insert(input.clone()) {
                    out.push(input.clone());
                }
            }
        }
        out
    }

    /// Get all terminal nodes (leaves of the graph).
    pub fn terminal_nodes(&self) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.is_terminal).collect()
    }

    /// Get all entry nodes (nodes with no incoming edges).
    pub fn entry_nodes(&self) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| n.edges_in.is_empty())
            .collect()
    }

    /// Get a node by name.
    pub fn get_node(&self, name: &str) -> Option<&GraphNode> {
        self.nodes.get(name)
    }

    /// Get all node names that feed into a given node.
    pub fn incoming_sources(&self, name: &str) -> Vec<&IncomingEdge> {
        self.nodes
            .get(name)
            .map(|n| n.edges_in.iter().collect())
            .unwrap_or_default()
    }
}

/// Kahn's algorithm for topological sorting with cycle detection.
fn topological_sort(nodes: &HashMap<String, GraphNode>) -> Result<Vec<String>, GraphIRError> {
    // Compute in-degree for each node
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for node in nodes.values() {
        in_degree.entry(node.name.clone()).or_insert(0);
        for edge in &node.edges_out {
            match edge {
                GraphEdge::Linear { target } => {
                    *in_degree.entry(target.clone()).or_insert(0) += 1;
                }
                GraphEdge::Routing { variants } => {
                    for v in variants {
                        *in_degree.entry(v.target.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Start with nodes that have no incoming edges
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();

    // Sort the initial queue for deterministic output
    let mut sorted_queue: Vec<String> = queue.drain(..).collect();
    sorted_queue.sort();
    queue.extend(sorted_queue);

    let mut sorted = Vec::new();

    while let Some(name) = queue.pop_front() {
        sorted.push(name.clone());

        if let Some(node) = nodes.get(&name) {
            let mut next_candidates = Vec::new();
            for edge in &node.edges_out {
                match edge {
                    GraphEdge::Linear { target } => {
                        if let Some(deg) = in_degree.get_mut(target) {
                            *deg -= 1;
                            if *deg == 0 {
                                next_candidates.push(target.clone());
                            }
                        }
                    }
                    GraphEdge::Routing { variants } => {
                        for v in variants {
                            if let Some(deg) = in_degree.get_mut(&v.target) {
                                *deg -= 1;
                                if *deg == 0 {
                                    next_candidates.push(v.target.clone());
                                }
                            }
                        }
                    }
                }
            }
            // Sort candidates for deterministic output
            next_candidates.sort();
            queue.extend(next_candidates);
        }
    }

    // If we didn't visit all nodes, there's a cycle
    if sorted.len() != nodes.len() {
        let remaining: Vec<String> = nodes
            .keys()
            .filter(|k| !sorted.contains(k))
            .cloned()
            .collect();
        return Err(GraphIRError::Cycle(format!(
            "nodes involved in cycle: {}",
            remaining.join(", ")
        )));
    }

    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computation_graph::parser::{ReactionCriteria, ReactionMode, RoutingVariant};
    use syn::Ident;

    fn ident(name: &str) -> Ident {
        Ident::new(name, proc_macro2::Span::call_site())
    }

    fn make_topology(edges: Vec<ParsedEdge>) -> ParsedTopology {
        ParsedTopology {
            trigger: TriggerSpec::Bundled(ReactionCriteria {
                mode: ReactionMode::WhenAny,
                accumulators: vec![ident("alpha")],
            }),
            edges,
        }
    }

    #[test]
    fn test_linear_chain() {
        let topology = make_topology(vec![
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![ident("alpha")],
                to: ident("b"),
            },
            ParsedEdge::Linear {
                from: ident("b"),
                from_inputs: vec![],
                to: ident("c"),
            },
        ]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        assert_eq!(ir.sorted_nodes, vec!["a", "b", "c"]);
        assert!(ir.get_node("c").unwrap().is_terminal);
        assert!(!ir.get_node("a").unwrap().is_terminal);
        assert!(!ir.get_node("b").unwrap().is_terminal);
    }

    #[test]
    fn test_routing() {
        let topology = make_topology(vec![ParsedEdge::Routing {
            from: ident("decision"),
            from_inputs: vec![ident("alpha")],
            variants: vec![
                RoutingVariant {
                    variant_name: ident("Signal"),
                    target: ident("handler_a"),
                },
                RoutingVariant {
                    variant_name: ident("NoAction"),
                    target: ident("handler_b"),
                },
            ],
        }]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        assert_eq!(ir.sorted_nodes[0], "decision");
        assert!(ir.get_node("handler_a").unwrap().is_terminal);
        assert!(ir.get_node("handler_b").unwrap().is_terminal);
        assert_eq!(ir.terminal_nodes().len(), 2);
        assert_eq!(ir.entry_nodes().len(), 1);
    }

    #[test]
    fn test_diamond_graph() {
        // a -> b, a -> c, b -> d, c -> d
        let topology = make_topology(vec![
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![ident("alpha")],
                to: ident("b"),
            },
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![],
                to: ident("c"),
            },
            ParsedEdge::Linear {
                from: ident("b"),
                from_inputs: vec![],
                to: ident("d"),
            },
            ParsedEdge::Linear {
                from: ident("c"),
                from_inputs: vec![],
                to: ident("d"),
            },
        ]);

        let ir = GraphIR::from_parsed(topology).unwrap();

        // a must come first
        assert_eq!(ir.sorted_nodes[0], "a");
        // d must come last
        assert_eq!(*ir.sorted_nodes.last().unwrap(), "d");
        // b and c must come before d
        let b_pos = ir.sorted_nodes.iter().position(|n| n == "b").unwrap();
        let c_pos = ir.sorted_nodes.iter().position(|n| n == "c").unwrap();
        let d_pos = ir.sorted_nodes.iter().position(|n| n == "d").unwrap();
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);

        // d has 2 incoming edges (fan-in)
        assert_eq!(ir.get_node("d").unwrap().edges_in.len(), 2);
        assert!(ir.get_node("d").unwrap().is_terminal);
    }

    #[test]
    fn test_cycle_detection() {
        // a -> b, b -> a (cycle)
        let topology = make_topology(vec![
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![ident("alpha")],
                to: ident("b"),
            },
            ParsedEdge::Linear {
                from: ident("b"),
                from_inputs: vec![],
                to: ident("a"),
            },
        ]);

        let result = GraphIR::from_parsed(topology);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"), "got: {}", err);
    }

    #[test]
    fn test_terminal_nodes() {
        // a -> b, a -> c (both b and c are terminal)
        let topology = make_topology(vec![
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![ident("alpha")],
                to: ident("b"),
            },
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![],
                to: ident("c"),
            },
        ]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        let terminals: HashSet<String> =
            ir.terminal_nodes().iter().map(|n| n.name.clone()).collect();
        assert_eq!(terminals.len(), 2);
        assert!(terminals.contains("b"));
        assert!(terminals.contains("c"));
    }

    #[test]
    fn test_entry_nodes() {
        // a -> c, b -> c (both a and b are entry nodes)
        let topology = make_topology(vec![
            ParsedEdge::Linear {
                from: ident("a"),
                from_inputs: vec![ident("alpha")],
                to: ident("c"),
            },
            ParsedEdge::Linear {
                from: ident("b"),
                from_inputs: vec![],
                to: ident("c"),
            },
        ]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        let entries: HashSet<String> = ir.entry_nodes().iter().map(|n| n.name.clone()).collect();
        assert_eq!(entries.len(), 2);
        assert!(entries.contains("a"));
        assert!(entries.contains("b"));
    }

    #[test]
    fn test_cache_inputs_preserved() {
        let topology = make_topology(vec![ParsedEdge::Linear {
            from: ident("entry"),
            from_inputs: vec![ident("alpha"), ident("beta"), ident("gamma")],
            to: ident("output"),
        }]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        let entry = ir.get_node("entry").unwrap();
        assert_eq!(entry.cache_inputs, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_incoming_edges_with_variants() {
        let topology = make_topology(vec![ParsedEdge::Routing {
            from: ident("decision"),
            from_inputs: vec![ident("alpha")],
            variants: vec![RoutingVariant {
                variant_name: ident("Signal"),
                target: ident("handler"),
            }],
        }]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        let handler = ir.get_node("handler").unwrap();
        assert_eq!(handler.edges_in.len(), 1);
        assert_eq!(handler.edges_in[0].from, "decision");
        assert_eq!(handler.edges_in[0].variant.as_deref(), Some("Signal"));
    }

    #[test]
    fn test_mixed_routing_and_linear() {
        let topology = make_topology(vec![
            ParsedEdge::Routing {
                from: ident("decision"),
                from_inputs: vec![ident("alpha")],
                variants: vec![
                    RoutingVariant {
                        variant_name: ident("Signal"),
                        target: ident("risk_check"),
                    },
                    RoutingVariant {
                        variant_name: ident("NoAction"),
                        target: ident("audit"),
                    },
                ],
            },
            ParsedEdge::Linear {
                from: ident("risk_check"),
                from_inputs: vec![],
                to: ident("output"),
            },
        ]);

        let ir = GraphIR::from_parsed(topology).unwrap();
        assert_eq!(ir.sorted_nodes[0], "decision");
        assert!(ir.get_node("audit").unwrap().is_terminal);
        assert!(ir.get_node("output").unwrap().is_terminal);
        assert!(!ir.get_node("risk_check").unwrap().is_terminal);
    }
}
