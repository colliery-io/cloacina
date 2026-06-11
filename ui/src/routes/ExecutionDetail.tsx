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

import { Anchor, Card, Group, Stack, Text, Title } from "@mantine/core";
import { Link, useParams } from "react-router-dom";

import { useExecution, useExecutionEvents } from "../api/executions";
import { EventLog } from "../components/EventLog";
import { StatusBadge } from "../components/StatusBadge";
import { ErrorState, Loading } from "../components/states/States";
import { isTerminalStatus } from "../util/status";

/**
 * Execution detail (T-0653 / REQ-004 non-live half): status header + the
 * event log from the REST endpoint. **Built streaming-ready** — T-0656
 * adds a live tail that merges WS events into the same `EventLog`, and the
 * `isTerminalStatus` check here is what it uses to decide whether to open
 * a stream at all.
 */
export function ExecutionDetail() {
  const { id = "" } = useParams();
  const detail = useExecution(id);
  const events = useExecutionEvents(id);

  const terminal = detail.data ? isTerminalStatus(detail.data.status) : true;

  return (
    <Stack>
      <div>
        <Anchor component={Link} to="/executions" size="sm">
          ← Executions
        </Anchor>
        <Title order={2}>Execution</Title>
        <Text size="xs" c="dimmed">
          {id}
        </Text>
      </div>

      {detail.isPending ? (
        <Loading label="Loading execution…" />
      ) : detail.isError ? (
        <ErrorState error={detail.error} onRetry={detail.refetch} />
      ) : (
        <Card withBorder padding="lg">
          <Group>
            <StatusBadge status={detail.data.status} />
            {!terminal && (
              <Text c="blue" size="sm">
                in progress — live follow lands in T-0656
              </Text>
            )}
          </Group>
        </Card>
      )}

      <Card withBorder padding="lg">
        <Title order={4} mb="sm">
          Event log
        </Title>
        {events.isPending ? (
          <Loading label="Loading events…" />
        ) : events.isError ? (
          <ErrorState error={events.error} onRetry={events.refetch} />
        ) : (
          <EventLog events={events.data.events} />
        )}
      </Card>
    </Stack>
  );
}
