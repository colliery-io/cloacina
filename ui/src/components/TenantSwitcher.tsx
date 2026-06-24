/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  In-app tenant switcher (CLOACI-T-0779). Flips the active connection between
 *  the saved tenants so an operator can see tenant isolation live — switching
 *  rescopes every query (data is schema-isolated per tenant). "Add tenant…"
 *  reuses the connect form (`/connect?add=1`).
 */
import { MONO } from "@colliery-io/aurora-dark";
import { Box, Menu } from "@mantine/core";
import { IconCheck, IconChevronDown, IconPlus, IconX } from "@tabler/icons-react";
import { useNavigate } from "react-router-dom";

import { useAuth } from "../auth/AuthContext";

export function TenantSwitcher({ dot }: { dot: string }) {
  const { connection, connections, switchTo, removeConnection } = useAuth();
  const navigate = useNavigate();
  if (!connection) return null;

  return (
    <Menu position="top-start" width={232} withinPortal shadow="md">
      <Menu.Target>
        <Box
          component="button"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            width: "100%",
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
            fontSize: 12,
            color: "var(--fg-2)",
            textAlign: "left",
          }}
        >
          <span style={{ width: 7, height: 7, borderRadius: "50%", background: dot, flex: "none" }} />
          <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {connection.label}
          </span>
          <IconChevronDown size={12} style={{ color: "var(--faint)", flex: "none" }} />
        </Box>
      </Menu.Target>

      <Menu.Dropdown>
        <Menu.Label>Tenants</Menu.Label>
        {connections.map((c) => {
          const isActive = c.label === connection.label;
          return (
            <Menu.Item
              key={c.label}
              onClick={() => switchTo(c.label)}
              leftSection={
                isActive ? (
                  <IconCheck size={14} style={{ color: "var(--ok, #4bd07f)" }} />
                ) : (
                  <Box style={{ width: 14 }} />
                )
              }
              rightSection={
                connections.length > 1 ? (
                  <Box
                    component="span"
                    title={`Remove ${c.label}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      removeConnection(c.label);
                    }}
                    style={{ display: "flex", color: "var(--faint)" }}
                  >
                    <IconX size={13} />
                  </Box>
                ) : null
              }
            >
              <span style={{ fontSize: 12.5, color: "var(--fg)" }}>{c.label}</span>
              <Box style={{ fontFamily: MONO, fontSize: 10, color: "var(--faint)" }}>
                {c.tenant}
              </Box>
            </Menu.Item>
          );
        })}
        <Menu.Divider />
        <Menu.Item
          leftSection={<IconPlus size={14} />}
          onClick={() => navigate("/connect?add=1")}
        >
          <span style={{ fontSize: 12.5 }}>Add tenant…</span>
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  );
}
