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

import { Alert, Button, Center, Loader, Stack, Text } from "@mantine/core";

import { classifyError } from "../../api/errors";

/**
 * Shared loading / empty / error primitives (CLOACI-I-0117 / NFR-001).
 * Every async view uses these — no indefinite spinners, no blank error
 * screens. The error component renders by classified kind (REQ-007).
 */

export function Loading({ label = "Loading…" }: { label?: string }) {
  return (
    <Center mih={160} aria-busy="true">
      <Stack align="center" gap="xs">
        <Loader />
        <Text c="dimmed" size="sm">
          {label}
        </Text>
      </Stack>
    </Center>
  );
}

export function Empty({ message }: { message: string }) {
  return (
    <Center mih={160}>
      <Text c="dimmed">{message}</Text>
    </Center>
  );
}

export function ErrorState({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const c = classifyError(error);
  const title =
    c.kind === "auth"
      ? "Not authorized"
      : c.kind === "notfound"
        ? "Not found"
        : c.kind === "validation"
          ? "Invalid request"
          : c.kind === "network"
            ? "Cannot reach server"
            : "Something went wrong";
  const retryable = c.kind === "server" || c.kind === "network";
  return (
    <Alert color={c.kind === "validation" ? "yellow" : "red"} title={title} role="alert">
      <Stack gap="xs">
        <Text size="sm">{c.message}</Text>
        {c.code && (
          <Text size="xs" c="dimmed">
            code: {c.code}
          </Text>
        )}
        {retryable && onRetry && (
          <Button size="xs" variant="light" onClick={onRetry} w="fit-content">
            Retry
          </Button>
        )}
      </Stack>
    </Alert>
  );
}
