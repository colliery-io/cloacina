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

import { Route, Routes } from "react-router-dom";

import { RequireAuth } from "./components/RequireAuth";
import { Shell } from "./components/Shell";
import { Connect } from "./routes/Connect";
import { Overview } from "./routes/Overview";
import { NotFound, Placeholder } from "./routes/Placeholder";
import { Workflows } from "./routes/Workflows";
import { WorkflowDetail } from "./routes/WorkflowDetail";
import { WorkflowUpload } from "./routes/WorkflowUpload";
import { Executions } from "./routes/Executions";
import { ExecutionDetail } from "./routes/ExecutionDetail";
import { Triggers } from "./routes/Triggers";
import { TriggerDetail } from "./routes/TriggerDetail";
import { Graphs } from "./routes/Graphs";
import { GraphDetail } from "./routes/GraphDetail";
import { Operations } from "./routes/Operations";
import { Keys } from "./routes/Keys";

/**
 * Route map (CLOACI-I-0117 IA). Feature views replace the placeholders as
 * their tasks land:
 *   /workflows*  → T-0652 / T-0657   /executions* → T-0653 / T-0656
 *   /triggers*   → T-0654            /keys        → T-0658
 *   overview     → T-0655
 */
export function App() {
  return (
    <Routes>
      <Route path="/connect" element={<Connect />} />

      <Route element={<RequireAuth />}>
        <Route element={<Shell />}>
          <Route index element={<Overview />} />
          <Route path="workflows" element={<Workflows />} />
          <Route path="workflows/upload" element={<WorkflowUpload />} />
          <Route path="workflows/:name" element={<WorkflowDetail />} />
          <Route path="executions" element={<Executions />} />
          <Route path="executions/:id" element={<ExecutionDetail />} />
          <Route path="triggers" element={<Triggers />} />
          <Route path="triggers/:name" element={<TriggerDetail />} />
          <Route path="graphs" element={<Graphs />} />
          <Route path="graphs/:name" element={<GraphDetail />} />
          <Route path="operations" element={<Operations />} />
          <Route path="keys" element={<Keys />} />
          <Route path="settings" element={<Placeholder title="Settings" task="T-0651" />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Route>
    </Routes>
  );
}
