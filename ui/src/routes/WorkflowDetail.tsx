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

import {
  Alert,
  Anchor,
  Button,
  Card,
  Group,
  List,
  Stack,
  Text,
  Title,
  Tooltip,
} from "@mantine/core";
import { Link, useParams } from "react-router-dom";

import { useWorkflow } from "../api/workflows";
import { BuildStatusBadge } from "../components/BuildStatusBadge";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";

/**
 * Workflow detail (T-0652 / REQ-003 read half). Shows build state + metadata.
 * The Execute / Delete actions are stubbed (disabled) — wired in T-0657.
 */
export function WorkflowDetail() {
  const { name = "" } = useParams();
  const { data, isPending, isError, error, refetch } = useWorkflow(name);

  return (
    <Stack>
      <Group justify="space-between" align="flex-start">
        <div>
          <Anchor component={Link} to="/workflows" size="sm">
            ← Workflows
          </Anchor>
          <Title order={2}>{name}</Title>
        </div>
        {/* Write actions land in T-0657. */}
        <Group gap="xs">
          <Tooltip label="Coming in T-0657">
            <Button disabled>Execute</Button>
          </Tooltip>
          <Tooltip label="Coming in T-0657">
            <Button color="red" variant="light" disabled>
              Delete
            </Button>
          </Tooltip>
        </Group>
      </Group>

      {isPending ? (
        <Loading label="Loading workflow…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : !data ? (
        <Empty message="Workflow not found." />
      ) : (
        <Card withBorder padding="lg">
          <Stack gap="md">
            <Group>
              <BuildStatusBadge status={data.build_status} />
              <Text c="dimmed" size="sm">
                v{data.version} · created {formatTimestamp(data.created_at)}
              </Text>
            </Group>

            {data.description && <Text>{data.description}</Text>}

            {data.build_error && (
              <Alert color="red" title="Build error" role="alert">
                <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
                  {data.build_error}
                </Text>
              </Alert>
            )}

            <div>
              <Text fw={600} mb="xs">
                Tasks ({data.tasks.length})
              </Text>
              {data.tasks.length === 0 ? (
                <Text c="dimmed" size="sm">
                  No tasks.
                </Text>
              ) : (
                <List size="sm">
                  {data.tasks.map((t) => (
                    <List.Item key={t}>{t}</List.Item>
                  ))}
                </List>
              )}
            </div>
          </Stack>
        </Card>
      )}
    </Stack>
  );
}
