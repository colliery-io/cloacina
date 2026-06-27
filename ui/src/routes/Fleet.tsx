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

import { classifyError, TOKEN } from "@colliery-io/aurora-dark";
import { Alert, Box, Button, Group } from "@mantine/core";

import {
  useDeprovision,
  useFleet,
  useProvision,
  useTenantLimit,
  type FleetState,
} from "../api/fleet";
import { useCan, useTenant } from "../auth/AuthContext";

const MONO = "'IBM Plex Mono', monospace";

/** A single fleet metric — large mono number over a labeled caption. */
function Stat({ value, label, color }: { value: number; label: string; color?: string }) {
  return (
    <Box
      style={{
        flex: 1,
        background: "var(--sidebar)",
        border: "1px solid var(--border)",
        borderRadius: 12,
        padding: "14px 16px",
      }}
    >
      <Box style={{ fontFamily: MONO, fontSize: 28, fontWeight: 600, color: color ?? "var(--fg)" }}>
        {value}
      </Box>
      <Box
        style={{
          fontFamily: MONO,
          fontSize: 10,
          letterSpacing: ".1em",
          textTransform: "uppercase",
          color: "var(--faint)",
          marginTop: 4,
        }}
      >
        {label}
      </Box>
    </Box>
  );
}

/**
 * Tenant agent-fleet management (CLOACI-T-0813). Shows the fleet state —
 * desired (provisioned) vs actual (running) vs effective limit — and lets a
 * tenant-admin Provision (+1) / Deprovision (−1). Read-scope keys see the state
 * but not the controls; provisioning is disabled at capacity (the server
 * returns 409, surfaced here as an "at capacity" Alert). Role-gating mirrors the
 * server, which enforces it regardless of what the UI shows.
 */
export function Fleet() {
  const { canAdmin } = useCan();
  const tenant = useTenant();
  const fleet = useFleet();
  const limit = useTenantLimit();
  const provision = useProvision();
  const deprovision = useDeprovision();

  const state: FleetState | undefined = fleet.data;
  const atCapacity = state ? state.desired_count >= state.effective_limit : false;

  return (
    <Box style={{ maxWidth: 820 }}>
      <Box
        component="h2"
        style={{ fontSize: 20, fontWeight: 600, color: "var(--fg)", margin: 0, marginBottom: 4 }}
      >
        Agent fleet
      </Box>
      <Box style={{ fontSize: 12.5, color: "var(--muted)", marginBottom: 18 }}>
        Provisioned vs running agents for tenant <b>{tenant}</b>, against this tenant's effective
        agent limit. Scale the fleet up or down with Provision / Deprovision.
      </Box>

      {/* Fleet state */}
      {fleet.isError ? (
        <Alert color="bad" variant="light" mb={18}>
          {classifyError(fleet.error).message}
        </Alert>
      ) : fleet.isPending ? (
        <Box style={{ color: "var(--muted)", fontSize: 13, marginBottom: 18 }}>Loading…</Box>
      ) : (
        <Group gap={12} grow mb={18} align="stretch">
          <Stat value={state!.desired_count} label="Provisioned" />
          <Stat value={state!.actual_count} label="Running" color={TOKEN.ice} />
          <Stat value={state!.effective_limit} label="Effective limit" />
        </Group>
      )}

      {/* Limit detail (read-only) */}
      {limit.data && (
        <Box style={{ fontSize: 12, color: "var(--muted)", marginBottom: 18 }}>
          Effective limit <b>{limit.data.effective_limit}</b> ={" "}
          {limit.data.tenant_override !== null ? (
            <>
              tenant override <b>{limit.data.tenant_override}</b>
            </>
          ) : (
            <>
              platform default <b>{limit.data.default_max_agents}</b> (no tenant override)
            </>
          )}
          .
        </Box>
      )}

      {/* Controls — admin only; non-admins see an explanatory Alert. */}
      {canAdmin ? (
        <Box
          style={{
            background: "var(--sidebar)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            padding: 16,
          }}
        >
          <Box style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)", marginBottom: 10 }}>
            Scale fleet
          </Box>
          <Group gap={10}>
            <Button
              color="ice"
              onClick={() => provision.mutate()}
              loading={provision.isPending}
              disabled={fleet.isPending || atCapacity}
              styles={{ root: { color: "#0b0d10", fontWeight: 600 } }}
            >
              Provision +1
            </Button>
            <Button
              variant="default"
              onClick={() => deprovision.mutate()}
              loading={deprovision.isPending}
              disabled={fleet.isPending || (state ? state.desired_count <= 0 : true)}
            >
              Deprovision −1
            </Button>
            {atCapacity && (
              <Box style={{ fontSize: 12, color: "var(--muted)" }}>
                At capacity ({state!.effective_limit}).
              </Box>
            )}
          </Group>
          {provision.isError && (
            <Alert color={classifyError(provision.error).status === 409 ? "neutral" : "bad"} variant="light" mt={10}>
              {classifyError(provision.error).status === 409
                ? `Fleet is at capacity (${state?.effective_limit ?? ""}). Deprovision an agent or raise the tenant limit first.`
                : classifyError(provision.error).message}
            </Alert>
          )}
          {deprovision.isError && (
            <Alert color="bad" variant="light" mt={10}>
              {classifyError(deprovision.error).message}
            </Alert>
          )}
        </Box>
      ) : (
        <Alert color="neutral" variant="light">
          You need admin access to provision or deprovision agents.
        </Alert>
      )}
    </Box>
  );
}
