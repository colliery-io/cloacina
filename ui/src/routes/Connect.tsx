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
  Button,
  Card,
  Center,
  PasswordInput,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useForm } from "@mantine/form";
import { useEffect, useRef, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";

import { useAuth } from "../auth/AuthContext";
import { runtimeConfig } from "../config";

/**
 * Connection gate (CLOACI-I-0117 / REQ-001), manual API-key path. The OIDC
 * "Login with <provider>" buttons are added by T-0662 once I-0118 ships the
 * `/auth/*` contract — this manual path remains the fallback regardless.
 */
export function Connect() {
  const { connection, connect } = useAuth();
  const navigate = useNavigate();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const form = useForm({
    initialValues: {
      serverUrl: runtimeConfig.defaultServerUrl,
      // Dev convenience: prefill the demo key/tenant so the local stack connects
      // without re-typing (empty in production builds). See config.ts.
      apiKey: runtimeConfig.demoApiKey,
      tenant: runtimeConfig.demoTenant || "public",
    },
    validate: {
      serverUrl: (v) => (/^https?:\/\//.test(v) ? null : "Must start with http:// or https://"),
      apiKey: (v) => (v.trim() ? null : "Required"),
      tenant: (v) => (v.trim() ? null : "Required"),
    },
  });

  async function onSubmit(values: typeof form.values) {
    setSubmitting(true);
    setError(null);
    try {
      await connect({
        serverUrl: values.serverUrl.replace(/\/+$/, ""),
        apiKey: values.apiKey.trim(),
        tenant: values.tenant.trim(),
      });
      navigate("/", { replace: true });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }

  // Dev-only: auto-connect once with the prefilled demo credentials so the gate
  // is skipped on the local stack. No-op in production (demoAutoConnect=false)
  // and once connected. If the server is down it surfaces the error and leaves
  // the (prefilled) form for a manual retry.
  const autoTried = useRef(false);
  useEffect(() => {
    if (
      runtimeConfig.demoAutoConnect &&
      !autoTried.current &&
      !connection &&
      form.values.apiKey.trim()
    ) {
      autoTried.current = true;
      void onSubmit(form.values);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Already connected → don't show the gate.
  if (connection) return <Navigate to="/" replace />;

  return (
    <Center mih="100vh" p="md">
      <Card withBorder shadow="sm" w={420} padding="lg">
        <Stack>
          <div>
            <Title order={3}>Connect to Cloacina</Title>
            <Text c="dimmed" size="sm">
              Enter a server URL and a tenant API key.
            </Text>
          </div>

          <form onSubmit={form.onSubmit(onSubmit)}>
            <Stack>
              <TextInput
                label="Server URL"
                placeholder="http://localhost:8080"
                {...form.getInputProps("serverUrl")}
              />
              <PasswordInput
                label="API key"
                placeholder="clk_…"
                autoComplete="off"
                {...form.getInputProps("apiKey")}
              />
              <TextInput label="Tenant" placeholder="public" {...form.getInputProps("tenant")} />

              {error && (
                <Alert color="red" role="alert">
                  {error}
                </Alert>
              )}

              <Button type="submit" loading={submitting} fullWidth>
                Connect
              </Button>
            </Stack>
          </form>
        </Stack>
      </Card>
    </Center>
  );
}
