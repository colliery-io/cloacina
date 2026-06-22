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

import { FullDag, type FullDagEdge, type FullDagNode } from "./FullDag";

/** A task node: its id and the ids of the tasks it depends on. */
export interface TaskGraphNode {
  id: string;
  dependencies: string[];
  description?: string | null;
}

/**
 * Interactive workflow DAG (CLOACI-T-0663). Renders the task dependency graph;
 * edges point from each dependency to the dependent task.
 *
 * With `statusByTask` (local task id → execution status) the nodes are coloured
 * by per-task state — the live execution DAG (CLOACI-T-0719). Without it, the
 * graph shows structure only (the workflow-detail view).
 */
export function WorkflowGraph({
  tasks,
  statusByTask,
  onNodeClick,
  failByTask,
}: {
  tasks: TaskGraphNode[];
  statusByTask?: Record<string, string>;
  /** CLOACI-I-0129: click a task node → e.g. open its source (T-0750). */
  onNodeClick?: (id: string) => void;
  /** CLOACI-T-0764: local task id → failures in window (reliability overlay). */
  failByTask?: Record<string, number>;
}) {
  const nodes: FullDagNode[] = tasks.map((t) => ({ id: t.id, label: t.id, status: statusByTask?.[t.id] }));
  const edges: FullDagEdge[] = tasks.flatMap((t) =>
    t.dependencies.map((dep) => ({ from: dep, to: t.id })),
  );
  return (
    <FullDag nodes={nodes} edges={edges} testId="workflow-graph" onNodeClick={onNodeClick} failByNode={failByTask} />
  );
}
