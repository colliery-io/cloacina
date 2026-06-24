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
  Box,
  Button,
  Group,
  PasswordInput,
  SegmentedControl,
  Stack,
  TextInput,
} from "@mantine/core";
import { useForm } from "@mantine/form";
import { useEffect, useRef, useState } from "react";
import { Navigate, useNavigate, useSearchParams } from "react-router-dom";
import { CloacinaClient } from "@cloacina/client";

import { useAuth } from "../auth/AuthContext";
import { classifyError } from "../api/errors";
import { runtimeConfig } from "../config";
import { BrandMark, MONO } from "../components/aurora";

/** How the operator authenticates at the connect gate (CLOACI-T-0796/0798/0800). */
type Mode = "key" | "password" | "sso";

/** Connection gate (Aurora Dark, spec 14). Supports a pasted API key OR
 *  self-managed username/password login (mints a short-TTL key server-side). */
export function Connect() {
  const { connection, connect } = useAuth();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const addMode = searchParams.get("add") === "1";
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>("key");

  const form = useForm({
    initialValues: {
      serverUrl: runtimeConfig.defaultServerUrl,
      apiKey: runtimeConfig.demoApiKey,
      username: "",
      password: "",
      tenant: runtimeConfig.demoTenant || "public",
    },
    // Credential fields are validated per-mode inside the submit handlers
    // (mantine's static validate can't see the current `mode`).
    validate: {
      serverUrl: (v) =>
        /^https?:\/\//.test(v) ? null : "Must start with http:// or https://",
      tenant: (v) => (v.trim() ? null : "Required"),
    },
  });

  async function onSubmit(values: typeof form.values) {
    setSubmitting(true);
    setError(null);
    try {
      const serverUrl = values.serverUrl.replace(/\/+$/, "");
      const tenant = values.tenant.trim();

      let apiKey: string;
      if (mode === "password") {
        if (!values.username.trim() || !values.password) {
          throw new Error("Username and password are required");
        }
        // Public login — mint a short-TTL key, then connect with it.
        const loginClient = new CloacinaClient({ baseUrl: serverUrl, apiKey: "", tenant });
        try {
          const res = await loginClient.localLogin({
            username: values.username.trim(),
            password: values.password,
            tenant,
          });
          apiKey = res.key;
        } catch (err) {
          throw new Error(classifyError(err).message);
        }
      } else {
        if (!values.apiKey.trim()) throw new Error("API key is required");
        apiKey = values.apiKey.trim();
      }

      await connect({ label: tenant, serverUrl, apiKey, tenant });
      navigate("/", { replace: true });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }

  /** Start the OIDC browser flow (CLOACI-T-0800): stash the server URL so the
   *  post-callback fragment pickup can reconnect, then full-page navigate to
   *  the server's login route, which 302s to the identity provider. */
  function startSso() {
    const serverUrl = form.values.serverUrl.replace(/\/+$/, "");
    if (!/^https?:\/\//.test(serverUrl)) {
      setError("Server URL must start with http:// or https://");
      return;
    }
    setSubmitting(true);
    sessionStorage.setItem("cloacina.sso.server", serverUrl);
    window.location.href = `${serverUrl}/v1/auth/oidc/login`;
  }

  // The OIDC callback returns the browser to `/connect#key=…&tenant=…&role=…`
  // (a fragment is never sent to a server or logged). Pick it up and connect.
  const ssoTried = useRef(false);
  useEffect(() => {
    if (ssoTried.current) return;
    const hash = window.location.hash;
    if (!hash.includes("key=")) return;
    ssoTried.current = true;
    const params = new URLSearchParams(hash.slice(1));
    const key = params.get("key");
    const tenant = params.get("tenant") || "public";
    // Strip the key out of the URL bar / history immediately.
    window.history.replaceState(null, "", window.location.pathname);
    if (!key) return;
    const serverUrl =
      sessionStorage.getItem("cloacina.sso.server") ??
      form.values.serverUrl.replace(/\/+$/, "");
    sessionStorage.removeItem("cloacina.sso.server");
    void (async () => {
      setSubmitting(true);
      try {
        await connect({ label: tenant, serverUrl, apiKey: key, tenant });
        navigate("/", { replace: true });
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setSubmitting(false);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  if (connection && !addMode) return <Navigate to="/" replace />;

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
          <Box style={{ fontSize: 12.5, color: "var(--muted)", marginTop: 3, marginBottom: 14 }}>
            {mode === "password"
              ? "Sign in with your username and password."
              : mode === "sso"
                ? "Sign in through your identity provider."
                : "Enter a server URL and a tenant API key."}
          </Box>

          <SegmentedControl
            fullWidth
            size="xs"
            mb={14}
            value={mode}
            onChange={(v) => {
              setMode(v as Mode);
              setError(null);
            }}
            data={[
              { label: "Username & password", value: "password" },
              { label: "API key", value: "key" },
              { label: "SSO", value: "sso" },
            ]}
          />

          <form onSubmit={form.onSubmit(onSubmit)}>
            <Stack gap={12}>
              <TextInput
                label="Server URL"
                placeholder="http://localhost:8080"
                styles={monoInput}
                {...form.getInputProps("serverUrl")}
              />

              {mode === "sso" ? (
                <>
                  {error && (
                    <Alert color="bad" role="alert" variant="light">
                      {error}
                    </Alert>
                  )}
                  <Button
                    type="button"
                    onClick={startSso}
                    loading={submitting}
                    fullWidth
                    color="ice"
                    radius={9}
                    styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
                  >
                    Continue with SSO
                  </Button>
                </>
              ) : (
                <>
                  {mode === "password" ? (
                    <>
                      <TextInput
                        label="Username"
                        placeholder="alice"
                        autoComplete="username"
                        styles={monoInput}
                        {...form.getInputProps("username")}
                      />
                      <PasswordInput
                        label="Password"
                        autoComplete="current-password"
                        styles={monoInput}
                        {...form.getInputProps("password")}
                      />
                    </>
                  ) : (
                    <PasswordInput
                      label="API key"
                      placeholder="clk_…"
                      autoComplete="off"
                      styles={monoInput}
                      {...form.getInputProps("apiKey")}
                    />
                  )}

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
                    {mode === "password" ? "Sign in" : "Connect"}
                  </Button>
                </>
              )}
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
