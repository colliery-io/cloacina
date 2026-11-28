"""The behaviour suite's checker (CLOACINA-T-1108).

A program outside the system under test. It reads the server's API and its
database, judges each promise on every pass, and writes the report: a page
and a JSON file, served on localhost.

    /usr/bin/python3 -I .angreal/test/bdd/suite/check.py --config suite.json

`suite.json` (owner-only access; it holds keys):

    {
      "server": "http://127.0.0.1:18100",
      "database_url": "postgres://user:password@127.0.0.1:15432/cloacina",
      "keys": {"team_a": "...", "team_b": "...", "team_c": "..."},
      "alerts_url": "http://127.0.0.1:18090/alerts",
      "metrics_url": "http://127.0.0.1:19090/metrics"
    }

`alerts_url` (the alert receiver, `alerts.py`) and `metrics_url` (the
server's metrics listener) are optional: without them the rules that read
alerts or metrics cannot judge.

The server URL, the keys file and the database URL can also be given one by
one: `--server`, `--keys` (a JSON file, tenant to key), and the environment
variable `CLOACINA_SUITE_DATABASE_URL` or `--database-url`. Prefer the file or
the variable for the database URL: a command line is visible to other users.

It runs until stopped. `--once` makes one pass. `--replay FILE...` judges
snapshots that `--record` kept, with no server. `--list` prints the rules and
the promises that have no rule yet.

Standard library only. The database is read with the `psql` program.
"""

from __future__ import annotations

import argparse
import json
import os
import stat
import sys
import time

if __package__ in (None, ""):
    # Run as a script: make the `suite` package importable, whatever the
    # current folder and whatever isolation flag Python was started with.
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    from suite import engine, promises, report, rules, sources, workload
else:
    from . import engine, promises, report, rules, sources, workload

DEFAULT_OUT = "bdd-standing-report"
DEFAULT_PORT = 8377


def parse_args(argv):
    parser = argparse.ArgumentParser(
        prog="check.py", description="Judge the promises of a Cloacina deployment, all the time.")
    parser.add_argument("--config", help="JSON file with server, database_url and keys")
    parser.add_argument("--server", help="the server's URL, for example http://127.0.0.1:18100")
    parser.add_argument("--keys", help="JSON file: tenant name to API key")
    parser.add_argument("--database-url",
                        help="postgres://… (or set CLOACINA_SUITE_DATABASE_URL)")
    parser.add_argument("--psql", help="path of the psql program")
    parser.add_argument("--alerts-url",
                        help="the alert receiver's list, for example http://127.0.0.1:18090/alerts")
    parser.add_argument("--metrics-url",
                        help="the server's metrics, for example http://127.0.0.1:19090/metrics")
    parser.add_argument("--out", default=DEFAULT_OUT,
                        help=f"folder for the report, the state and the record (default {DEFAULT_OUT})")
    parser.add_argument("--disturbances",
                        help="the disturbance record (default <out>/disturbances.jsonl)")
    parser.add_argument("--interval", type=int, default=60, help="seconds between passes (default 60)")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT,
                        help=f"serve the report on 127.0.0.1:PORT (default {DEFAULT_PORT}; 0 = do not serve)")
    parser.add_argument("--once", action="store_true", help="make one pass and stop")
    parser.add_argument("--record", action="store_true",
                        help="keep each snapshot in <out>/snapshots")
    parser.add_argument("--replay", nargs="+", metavar="FILE",
                        help="judge these snapshot files, oldest first, with no server")
    parser.add_argument("--list", action="store_true",
                        help="print the rules and the promises with no rule, then stop")
    return parser.parse_args(argv)


def _read_json(path, what):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, ValueError) as exc:
        raise SystemExit(f"cannot read {what} {path}: {exc}")
    if not isinstance(value, dict):
        raise SystemExit(f"{what} {path} is not a JSON object")
    return value


def _warn_if_open(path):
    """Keys belong in a file only its owner can read."""
    try:
        mode = os.stat(path).st_mode
    except OSError:
        return
    if mode & (stat.S_IRWXG | stat.S_IRWXO):
        print(f"warning: {path} holds keys and others can read it; run: chmod 600 {path}",
              file=sys.stderr)


def settings(args):
    """The server, the keys and the database URL, from the config file and
    the flags. A flag wins over the file."""
    config = {}
    if args.config:
        config = _read_json(args.config, "the config file")
        _warn_if_open(args.config)
    keys = config.get("keys") or {}
    if args.keys:
        keys = _read_json(args.keys, "the keys file")
        _warn_if_open(args.keys)
    server = args.server or config.get("server")
    database_url = (args.database_url or os.environ.get("CLOACINA_SUITE_DATABASE_URL")
                    or config.get("database_url"))
    if not server:
        raise SystemExit("no server URL: give --server, or `server` in the config file")
    if not keys or not all(isinstance(v, str) and v for v in keys.values()):
        raise SystemExit("no keys: give --keys, or `keys` in the config file (tenant to key)")
    return server, keys, database_url


def channels(args):
    """The alert receiver's URL and the metrics URL (CLOACINA-T-1115), from
    the flags or the config file. Either can be absent: the rules that need
    it then cannot judge."""
    config = _read_json(args.config, "the config file") if args.config else {}
    return (args.alerts_url or config.get("alerts_url"),
            args.metrics_url or config.get("metrics_url"))


def list_rules(registry):
    lines = []
    judged = set()
    for rule in registry.rules:
        judged.add(rule.promise)
        gap = f"  [gap {rule.gap}]" if rule.gap else ""
        lines.append(f"{rule.id}  ({rule.chapter})  judges {rule.promise}{gap}\n    {rule.about}")
    lines.append("")
    lines.append("Promises with no rule yet:")
    for promise_id, (row, sentence) in promises.PROMISES.items():
        if promise_id not in judged:
            lines.append(f"  {row}: {sentence}  (chapter task {promises.CHAPTER_TASK.get(row)})")
    return "\n".join(lines)


def one_pass(registry, ledger, snapshot_data, out, interval):
    snapshot = engine.Snapshot(snapshot_data)
    results = engine.judge_all(registry, ledger, snapshot)
    ledger.save()
    made = report.build(registry, ledger, results, snapshot, interval_s=interval)
    report.write(os.path.join(out, "report"), made)
    return made


def summary_line(made):
    counts = made["summary"]
    parts = [f"{report.LABEL[status]} {count}" for status, count in counts.items() if count]
    return f"{made['generated_at']}  " + ", ".join(parts)


def main(argv=None):
    args = parse_args(sys.argv[1:] if argv is None else argv)
    registry = rules.load()
    faults = registry.problems() + promises.problems() + workload.problems()
    if faults:
        for fault in faults:
            print(f"fault: {fault}", file=sys.stderr)
        return 2
    if args.list:
        print(list_rules(registry))
        return 0

    out = args.out
    ledger = engine.Ledger(os.path.join(out, "state.json"), max_gap_s=max(900, 5 * args.interval))

    if args.replay:
        made = None
        for path in args.replay:
            made = one_pass(registry, ledger, sources.read_snapshot(path), out, args.interval)
            print(summary_line(made))
        return 1 if made and made["red"] else 0

    server, keys, database_url = settings(args)
    alerts_url, metrics_url = channels(args)
    api = sources.Api(server)
    database = sources.Psql(database_url, binary=args.psql) if database_url else None
    if database is None:
        print("warning: no database URL; the rules that read the database cannot judge",
              file=sys.stderr)
    elif not database.binary:
        print("warning: no psql program found; give its path with --psql", file=sys.stderr)
    disturbances = args.disturbances or os.path.join(out, "disturbances.jsonl")

    served = None
    if args.port and not args.once:
        os.makedirs(os.path.join(out, "report"), exist_ok=True)
        served = report.serve(os.path.join(out, "report"), args.port)
        print(f"report: http://127.0.0.1:{args.port}/  (JSON: /report.json)")

    made = None
    try:
        while True:
            started = time.time()
            data = sources.collect(registry, api, database, keys, disturbances_path=disturbances,
                                   alerts_url=alerts_url, metrics_url=metrics_url)
            if args.record:
                sources.write_snapshot(os.path.join(out, "snapshots"), data)
            made = one_pass(registry, ledger, data, out, args.interval)
            print(summary_line(made), flush=True)
            if args.once:
                break
            time.sleep(max(1.0, args.interval - (time.time() - started)))
    except KeyboardInterrupt:
        print("stopped")
    finally:
        if served is not None:
            served.shutdown()
    return 1 if args.once and made and made["red"] else 0


if __name__ == "__main__":
    sys.exit(main())
