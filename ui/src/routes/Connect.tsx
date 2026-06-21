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

import { Alert, Box, Button, Group, PasswordInput, Stack, TextInput } from "@mantine/core";
import { useForm } from "@mantine/form";
import { useEffect, useRef, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";

import { useAuth } from "../auth/AuthContext";
import { runtimeConfig } from "../config";
import { BrandMark, MONO } from "../components/aurora";

/** Connection gate (Aurora Dark, spec 14). */
export function Connect() {
  const { connection, connect } = useAuth();
  const navigate = useNavigate();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const form = useForm({
    initialValues: {
      serverUrl: runtimeConfig.defaultServerUrl,
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

  if (connection) return <Navigate to="/" replace />;

  const monoInput = { input: { fontFamily: MONO } };

  return (
    <Box
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 16,
        background: "radial-gradient(120% 90% at 50% -10%, #131922, #0e1116)",
      }}
    >
      <Box style={{ width: 430 }}>
        <Group justify="center" gap={9} mb={18}>
          <BrandMark size={26} />
          <span style={{ fontSize: 22, fontWeight: 600, color: "var(--fg-bright)" }}>Cloacina</span>
        </Group>

        <Box
          style={{
            background: "var(--sidebar)",
            border: "1px solid var(--border)",
            borderRadius: 14,
            padding: "22px 22px 20px",
            boxShadow: "0 24px 60px rgba(0,0,0,.5)",
          }}
        >
          <Box style={{ fontSize: 16, fontWeight: 600, color: "var(--fg)" }}>Connect to a server</Box>
          <Box style={{ fontSize: 12.5, color: "var(--muted)", marginTop: 3, marginBottom: 16 }}>
            Enter a server URL and a tenant API key.
          </Box>

          <form onSubmit={form.onSubmit(onSubmit)}>
            <Stack gap={12}>
              <TextInput
                label="Server URL"
                placeholder="http://localhost:8080"
                styles={monoInput}
                {...form.getInputProps("serverUrl")}
              />
              <PasswordInput
                label="API key"
                placeholder="clk_…"
                autoComplete="off"
                styles={monoInput}
                {...form.getInputProps("apiKey")}
              />
              <TextInput
                label="Tenant"
                placeholder="public"
                styles={monoInput}
                {...form.getInputProps("tenant")}
              />

              {error && (
                <Alert color="bad" role="alert" variant="light">
                  {error}
                </Alert>
              )}

              <Button
                type="submit"
                loading={submitting}
                fullWidth
                color="ice"
                radius={9}
                styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
              >
                Connect
              </Button>
            </Stack>
          </form>
        </Box>

        <Box style={{ textAlign: "center", marginTop: 14, fontFamily: MONO, fontSize: 10.5, color: "var(--faint)" }}>
          cloacina v0.8.0 · tenant-scoped control plane
        </Box>
      </Box>
    </Box>
  );
}
