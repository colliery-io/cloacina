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

import { cardSurface, ErrorState, MONO } from "@colliery-io/aurora-dark";
import { Alert, Anchor, Box, Button, Stack, Text } from "@mantine/core";
import { useRef, useState } from "react";
import { Link } from "react-router-dom";

import { useUploadWorkflow } from "../api/workflows";
import { useCan } from "../auth/AuthContext";

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
  const { canWrite } = useCan();

  return (
    <Stack maw={560}>
      <Box>
        <Anchor component={Link} to="/workflows" size="xs" c="dimmed">
          ← Workflows
        </Anchor>
        <Box style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)", marginTop: 2 }}>Upload workflow</Box>
        <Box style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)", marginTop: 2 }}>
          Register a compiled .cloacina package for this tenant.
        </Box>
      </Box>

      {!canWrite ? (
        <Alert color="gold" title="Write access required">
          <Text size="sm">You need write access to upload packages.</Text>
        </Alert>
      ) : (
      <Box style={{ ...cardSurface, padding: "16px 18px" }}>
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
          {/* Drop area */}
          <Box
            onClick={() => inputRef.current?.click()}
            style={{
              border: "1px dashed var(--border-control)",
              borderRadius: 10,
              background: "var(--inset)",
              padding: "22px 16px",
              textAlign: "center",
              cursor: "pointer",
            }}
          >
            <Box style={{ fontSize: 13, color: "var(--fg-2)" }}>
              {file ? "Selected file" : "Choose a .cloacina package"}
            </Box>
            <Box style={{ fontFamily: MONO, fontSize: 12, color: file ? "var(--ice)" : "var(--faint)", marginTop: 4 }}>
              {file ? file.name : "click to browse"}
            </Box>
          </Box>

          <Button
            disabled={!file}
            loading={upload.isPending}
            color="ice"
            styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            onClick={() => file && upload.mutate(file)}
          >
            ↑ Upload
          </Button>

          {upload.isError && <ErrorState error={upload.error} />}

          {upload.isSuccess && (
            <Alert color="ok" title="Uploaded">
              <Text size="sm">
                Package registered.{" "}
                <Anchor component={Link} to={`/workflows/${upload.data.package_id}`}>
                  View
                </Anchor>
              </Text>
            </Alert>
          )}
        </Stack>
      </Box>
      )}
    </Stack>
  );
}
