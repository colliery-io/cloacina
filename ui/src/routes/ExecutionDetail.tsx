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

import { Anchor, Badge, Card, Group, Stack, Text, Title } from "@mantine/core";
import { useEffect } from "react";
import { Link, useParams } from "react-router-dom";

import { useExecution, useExecutionEvents, useLiveExecutionEvents } from "../api/executions";
import { EventLog } from "../components/EventLog";
import { StatusBadge } from "../components/StatusBadge";
import { ErrorState, Loading } from "../components/states/States";
import { mergeEvents } from "../util/events";
import { isTerminalStatus } from "../util/status";

/**
 * Execution detail (T-0653 + T-0656). Non-live half shows the REST event
 * log; the live half tails the delivery WS while the run is in progress
 * and merges into the same log.
 *
 * OQ-6 merge: REST history (`useExecutionEvents`) is the backfill; the live
 * tail (`useLiveExecutionEvents`) is layered on top, deduped on
 * `sequence_num` by `mergeEvents`. The status is polled (`livePoll`) so the
 * badge transitions to terminal, at which point the stream tears down and
 * the REST log is refetched for the authoritative final history.
 */
export function ExecutionDetail() {
  const { id = "" } = useParams();
  const detail = useExecution(id, { livePoll: true });
  const events = useExecutionEvents(id);

  const terminal = detail.data ? isTerminalStatus(detail.data.status) : true;
  const liveEvents = useLiveExecutionEvents(id, !terminal);

  // On the in-progress → terminal transition, refetch the REST log so the
  // final view is the server's authoritative history (not just what the
  // live tail happened to catch).
  useEffect(() => {
    if (terminal) events.refetch();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terminal]);

  const merged = mergeEvents(events.data?.events ?? [], liveEvents);

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
              <Badge color="blue" variant="dot">
                live
              </Badge>
            )}
          </Group>
        </Card>
      )}

      <Card withBorder padding="lg">
        <Group justify="space-between" mb="sm">
          <Title order={4}>Event log</Title>
          {!terminal && (
            <Text size="xs" c="blue">
              streaming…
            </Text>
          )}
        </Group>
        {events.isPending ? (
          <Loading label="Loading events…" />
        ) : events.isError ? (
          <ErrorState error={events.error} onRetry={events.refetch} />
        ) : (
          <EventLog events={merged} />
        )}
      </Card>
    </Stack>
  );
}
