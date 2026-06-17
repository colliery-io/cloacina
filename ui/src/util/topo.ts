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

/** A workflow task-graph node: a task id plus the task ids it depends on. */
export interface TaskGraphNode {
  id: string;
  dependencies?: string[] | null;
}

/**
 * Topological rank of each task in a workflow DAG (CLOACI-I-0124 / WS-1):
 * `id → 0-based position` in nominal run order (dependencies before dependents).
 * Ties within a level break by id so the order is deterministic across renders.
 * Any node left over by a dependency cycle is appended in id order. Used to give
 * the execution task table a *fixed* run-order instead of one that reshuffles as
 * statuses change.
 */
export function topoRank(graph: TaskGraphNode[]): Map<string, number> {
  const deps = new Map<string, string[]>();
  for (const n of graph) deps.set(n.id, n.dependencies ?? []);

  const placed = new Set<string>();
  const order: string[] = [];

  let progressed = true;
  while (placed.size < graph.length && progressed) {
    progressed = false;
    const ready = graph
      .map((n) => n.id)
      .filter(
        (id) =>
          !placed.has(id) &&
          (deps.get(id) ?? []).every((d) => placed.has(d) || !deps.has(d)),
      )
      .sort();
    for (const id of ready) {
      order.push(id);
      placed.add(id);
      progressed = true;
    }
  }
  // Cycle remnant (shouldn't happen for a valid DAG) — append stably.
  graph
    .map((n) => n.id)
    .filter((id) => !placed.has(id))
    .sort()
    .forEach((id) => order.push(id));

  const rank = new Map<string, number>();
  order.forEach((id, i) => rank.set(id, i));
  return rank;
}
