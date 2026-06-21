/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Schedule card (CLOACI-T-0764 §3): cron/poll cadence, humanized text, next/last
 *  run, and an enable switch. Cadence + next/last-run come from the trigger list
 *  item (the detail `schedule` doesn't carry next/last); toggle hits the trigger
 *  pause/resume endpoint. Empty state when the workflow has no schedule.
 */
import { Group, Switch } from "@mantine/core";
import cronstrue from "cronstrue";

import { useTriggers } from "../api/triggers";
import { useToggleTrigger } from "../api/controls";
import { MONO, Panel, Pill } from "./aurora";
import { formatTimestamp } from "../util/format";
import { formatAgo } from "../util/activity";
import { TOKEN } from "../util/tokens";

function humanizeCron(expr: string): string {
  try {
    return cronstrue.toString(expr, { verbose: false });
  } catch {
    return expr;
  }
}

export function ScheduleCard({ workflow }: { workflow: string }) {
  const { data } = useTriggers({ limit: 200, offset: 0 });
  const toggle = useToggleTrigger();
  const sched = (data?.items ?? []).find(
    (t) => t.workflow_name === workflow || t.trigger_name === workflow,
  );

  if (!sched) {
    return (
      <Panel title="Schedule" style={{ height: "100%" }}>
        <div style={{ color: "var(--faint)", fontSize: 12.5 }}>
          No schedule — runs on manual / trigger only.
        </div>
      </Panel>
    );
  }

  const isCron = !!sched.cron_expression;
  const toggleName = sched.trigger_name ?? sched.workflow_name;

  return (
    <Panel
      title="Schedule"
      right={<Pill color={isCron ? TOKEN.violet : TOKEN.teal}>{isCron ? "cron" : "poll"}</Pill>}
      style={{ display: "flex", flexDirection: "column", height: "100%" }}
    >
      <div style={{ flex: 1 }}>
        {isCron ? (
          <>
            <div style={{ fontFamily: MONO, fontSize: 13, color: TOKEN.ice }}>{sched.cron_expression}</div>
            <div style={{ fontSize: 11.5, color: "var(--muted)", marginTop: 3 }}>{humanizeCron(sched.cron_expression!)}</div>
          </>
        ) : (
          <div style={{ fontFamily: MONO, fontSize: 13, color: TOKEN.teal }}>
            every {sched.poll_interval_ms} ms
          </div>
        )}

        <Group gap={40} mt={16}>
          <div>
            <div style={{ fontFamily: MONO, fontSize: 9.5, letterSpacing: ".09em", textTransform: "uppercase", color: "var(--muted)" }}>Next run</div>
            <div style={{ fontFamily: MONO, fontSize: 12.5, color: TOKEN.violet, marginTop: 4 }}>{formatTimestamp(sched.next_run_at)}</div>
          </div>
          <div>
            <div style={{ fontFamily: MONO, fontSize: 9.5, letterSpacing: ".09em", textTransform: "uppercase", color: "var(--muted)" }}>Last run</div>
            <div style={{ fontFamily: MONO, fontSize: 12, color: "var(--fg-2)", marginTop: 4 }}>{formatAgo(sched.last_run_at)}</div>
          </div>
        </Group>
      </div>

      <Group justify="space-between" mt={16} pt={12} style={{ borderTop: "1px solid var(--border-fainter)" }}>
        <span style={{ fontSize: 12, color: "var(--muted)" }}>
          Schedule <span style={{ color: sched.enabled ? TOKEN.ok : "var(--faint)" }}>{sched.enabled ? "enabled" : "disabled"}</span>
        </span>
        <Switch
          size="sm"
          color="ice"
          checked={!!sched.enabled}
          disabled={toggle.isPending}
          onChange={(e) => toggle.mutate({ name: toggleName, enabled: e.currentTarget.checked })}
        />
      </Group>
    </Panel>
  );
}
