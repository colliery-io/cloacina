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

import { Text } from "@mantine/core";

import { useExecutionTasks } from "../api/executions";
import { RunCircles } from "./RunCircles";

/**
 * Airflow-style "Recent Tasks" cell (CLOACI-I-0124 / WS-10). Shows one colored
 * circle per task-instance state — with the count — for the workflow's most
 * recent run, mirroring Airflow's DAGs-view Recent Tasks column. Reuses the
 * per-execution tasks endpoint and the shared state-count circles.
 */
export function RecentTasksCell({ executionId }: { executionId: string | null }) {
  // Hook runs unconditionally; it self-disables for an empty id.
  const tasks = useExecutionTasks(executionId ?? "");

  if (!executionId) {
    return (
      <Text c="dimmed" size="xs">
        —
      </Text>
    );
  }
  if (tasks.isPending) {
    return (
      <Text c="dimmed" size="xs">
        …
      </Text>
    );
  }
  const runs = (tasks.data?.tasks ?? []).map((t) => ({ id: t.id, status: t.status }));
  return <RunCircles runs={runs} />;
}
