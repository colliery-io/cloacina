---
title: "Load History with a Backfill"
description: "Run a workflow once for each interval of a past range, with its own limit on active runs, and run the failed intervals again."
sidebar:
  order: 84
---

A backfill runs one workflow once for each interval of a range, oldest
first. Use it to load history: the intervals from before the schedule of the
workflow existed, or a range that must be built again.

A backfill is a second schedule for the workflow, with a start and an end.
It has its own limit on active runs, apart from the live schedule. It turns
itself off after its last interval.

## Before you start

- The workflow reads its interval from the context keys `interval_start`
  and `interval_end` (see [the interval of a run](../../../explanation/engine/cron-scheduling/#the-interval-of-a-run)).
- A run for an interval is safe to repeat. A backfill can cover fire times
  that the live schedule ran or will run. Nothing checks for overlap.
- The API key has the `write` role in the tenant. `list` and `inspect` need
  `read`.

## Create the backfill

Give the workflow, the cron expression that defines the intervals, and the
range:

```bash
cloacinactl backfill create build_hourly_table \
  --cron "0 * * * *" \
  --start 2026-01-01T00:00:00Z \
  --end 2026-08-18T23:00:00Z \
  --max-active-runs 8
```

The same request over HTTP:

```bash
curl -X POST "$SERVER/v1/tenants/acme/workflows/build_hourly_table/backfills" \
  -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
  -d '{
    "cron": "0 * * * *",
    "start": "2026-01-01T00:00:00Z",
    "end": "2026-08-18T23:00:00Z",
    "max_active_runs": 8
  }'
```

The backfill takes each fire time `F` of the expression with
`start <= F <= end`. Each fire time is one interval: the interval ends at
`F` and starts at the fire time before `F`. The example has 5,520 intervals.
The first one runs with `interval_start = 2025-12-31T23:00:00+00:00` and
`interval_end = 2026-01-01T00:00:00+00:00`.

| Option | Meaning |
|---|---|
| `--cron` | The expression that gives the intervals. Omit it to use the cron schedule of the workflow, when the workflow has exactly one. |
| `--timezone` | IANA timezone of the expression. Default `UTC`, or the timezone of the workflow's schedule when `--cron` is omitted. |
| `--start`, `--end` | The range, as RFC 3339 timestamps. Both ends are in the range. |
| `--interval` | One interval: its fire time. The same as `--start` and `--end` with that value. |
| `--max-active-runs` | The most runs of this backfill active at once. Default 1. |
| `--param KEY=VALUE`, `--params FILE` | Parameter values for each run, checked against the declared params of the workflow. |

One backfill holds at most 100,000 intervals. Split a larger range.

The response is the backfill with its `id`. Each run has that id in its
context as `backfill_id`.

## What each run gets

Each run of a backfill has these context keys, next to its params:

| Key | Value |
|---|---|
| `interval_start` | The fire time before `interval_end`. |
| `interval_end` | The fire time of the interval. |
| `scheduled_time` | Equal to `interval_end`. |
| `backfill_id` | The id of the backfill. |
| `schedule_id` | The id of the backfill. |
| `schedule_expression`, `schedule_timezone` | The cron expression and timezone of the backfill. |

The times are RFC 3339 strings in UTC.

## Watch the progress

```bash
cloacinactl backfill inspect build_hourly_table <BACKFILL_ID>
cloacinactl backfill list build_hourly_table
```

`progress` holds:

| Field | Meaning |
|---|---|
| `total` | Intervals in the backfill. |
| `completed` | Intervals whose run ended `Completed`. |
| `failed` | Intervals whose run ended `Failed` or `Cancelled`, or could not be started. |
| `running` | Intervals whose run has not ended. |
| `remaining` | Intervals with no run yet. |
| `last_interval_end` | The position: the end of the last interval that got a run. |
| `next_interval_end` | The end of the next interval to get a run. |

`status` is one of:

| Status | Meaning |
|---|---|
| `running` | Intervals remain, or a run has not ended. |
| `completed` | Each interval got a run and each run ended. Look at `progress.failed` for the runs that failed. |
| `cancelled` | Cancelled with intervals remaining, and no run is active. |
| `paused` | The schedule row of the backfill is paused. No backfill route sets this. |

`completed` and `failed` are counted from the run history. When
`--run-history-retention-days` is set, the retention job deletes finished
runs after that period, and these two counts go down with them. `total`,
`remaining`, the position and the status do not change.

## How it runs

- The scheduler starts intervals oldest first. With a limit above 1, runs
  can end in any order.
- The scheduler starts no more runs than `max_active_runs` leaves room for.
  One pass takes at most `cron_max_catchup_executions` intervals (100 on
  the server). It looks again when a run of the backfill ends, and each
  second.
- The limit counts the runs of this backfill only. The live schedule of the
  workflow keeps its own limit and keeps firing. `max_concurrent_tasks`
  still bounds how many tasks run at once.
- A failed run does not stop the backfill. It goes on with the next
  interval.
- A range that reaches past the present is taken as its fire times pass.
- When each interval has a run and no run is active, the backfill disables
  itself.
- A backfill is not in the list of triggers and schedules
  (`GET /v1/tenants/{tenant_id}/triggers`), and the gauge
  `cloacina_schedule_lag_seconds` does not count it.

## Run the failed intervals again

`inspect` lists the failed intervals, oldest first, each with its
`execution_id` and its status (`Failed`, `Cancelled`, or `NotStarted` when
no run could be started). The list holds at most 1,000; above that,
`failed_intervals_truncated` is `true`.

To run them again:

```bash
cloacinactl backfill retry build_hourly_table <BACKFILL_ID>
```

This creates a new backfill of the intervals that are failed at the time of
the call, with the same cron expression, timezone and params. Its limit on
active runs is the limit of the first backfill, or `--max-active-runs` when
given. Its `retry_of` field holds the id of the first backfill. The first
backfill is not changed. A backfill with no failed interval answers `409`.

To run one interval again, create a backfill of that interval:

```bash
cloacinactl backfill create build_hourly_table \
  --cron "0 * * * *" --interval 2026-03-04T07:00:00Z
```

## Cancel a backfill

```bash
cloacinactl backfill cancel build_hourly_table <BACKFILL_ID>
```

No new interval gets a run. The runs that are active end on their own. A
cancelled backfill is not started again: create a new one for the range
that remains, from `progress.next_interval_end` to the end.

## See also

- [Cron scheduling: backfill](../../../explanation/engine/cron-scheduling/#backfill)
- [HTTP API: backfills](../../../reference/http-api/#backfills)
- [CLI: `backfill`](../../../reference/cli/#backfill)
