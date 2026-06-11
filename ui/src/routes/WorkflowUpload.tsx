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

import { Alert, Anchor, Button, Card, Group, Stack, Text, Title } from "@mantine/core";
import { useRef, useState } from "react";
import { Link } from "react-router-dom";

import { useUploadWorkflow } from "../api/workflows";
import { ErrorState } from "../components/states/States";

/**
 * Workflow package upload (T-0657 / REQ-003 write half, UC-3). Select a
 * `.cloacina` file → upload → result. Errors surface the server's typed
 * `code`/`error` inline.
 *
 * Note: the SDK upload is fetch-based, so we show a busy state, not a
 * byte-progress bar — fetch can't report upload progress. Packages are
 * small; true progress would need an XHR path in `@cloacina/client`.
 */
export function WorkflowUpload() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const upload = useUploadWorkflow();

  return (
    <Stack maw={560}>
      <div>
        <Anchor component={Link} to="/workflows" size="sm">
          ← Workflows
        </Anchor>
        <Title order={2}>Upload workflow</Title>
      </div>

      <Card withBorder padding="lg">
        <Stack>
          <input
            ref={inputRef}
            type="file"
            accept=".cloacina"
            style={{ display: "none" }}
            onChange={(e) => {
              setFile(e.currentTarget.files?.[0] ?? null);
              upload.reset();
            }}
          />
          <Group>
            <Button variant="default" onClick={() => inputRef.current?.click()}>
              Choose .cloacina file
            </Button>
            <Text size="sm" c={file ? undefined : "dimmed"}>
              {file ? file.name : "no file selected"}
            </Text>
          </Group>

          <Button
            disabled={!file}
            loading={upload.isPending}
            onClick={() => file && upload.mutate(file)}
          >
            Upload
          </Button>

          {upload.isError && <ErrorState error={upload.error} />}

          {upload.isSuccess && (
            <Alert color="green" title="Uploaded">
              <Text size="sm">
                Package registered.{" "}
                <Anchor component={Link} to={`/workflows/${upload.data.package_id}`}>
                  View
                </Anchor>
              </Text>
            </Alert>
          )}
        </Stack>
      </Card>
    </Stack>
  );
}
