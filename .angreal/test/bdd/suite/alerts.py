"""The alert receiver of the behaviour suite (CLOACINA-T-1115).

The standing deployment's alert webhook (`alertWebhook.url`) points at this
program. It keeps each alert it gets and serves them back; the checker and
the "Operations" chapter read them through a port-forward.

    python3 alerts.py --port 8090 --out /data/alerts.jsonl

- `POST <any path>`: one alert. Answers 200 and keeps one line: when it
  came, the `X-Cloacina-Event` and `X-Cloacina-Delivery` headers, whether it
  was signed, and the JSON body.
- `GET /alerts?since=<unix seconds>`: `{"started_at": ..., "records": [...]}`.
  `started_at` is when this receiver came up: an alert of a run that ended
  before it may have been sent to an earlier receiver, and is not here.
- `GET /health`: 200.

It runs in the cluster, on the agent image (Python 3.11), standard library
only. `manifests()` gives its ConfigMap, Deployment and Service. It is not
part of the system under test: a receiver that is away is "cannot judge".
"""

from __future__ import annotations

import argparse
import json
import os
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

NAME = "cloacina-bdd-alerts"
PORT = 8090
OUT = "/data/alerts.jsonl"
PATH = "/alerts"
# The largest alert body kept. The server's payload is a few KiB.
MAX_BODY = 256 * 1024


class Store:
    """The alerts received, in memory and in a file of JSON lines."""

    def __init__(self, path=None):
        self.path = path
        self.started_at = time.time()
        self.lock = threading.Lock()
        self.records = []
        if path and os.path.exists(path):
            with open(path, "r", encoding="utf-8") as handle:
                for line in handle:
                    try:
                        self.records.append(json.loads(line))
                    except ValueError:
                        continue

    def add(self, headers, raw):
        try:
            body = json.loads(raw.decode("utf-8")) if raw else None
        except (UnicodeDecodeError, ValueError):
            body = {"unreadable": raw[:200].decode("utf-8", "replace")}
        record = {
            "received_at": time.time(),
            "event": headers.get("X-Cloacina-Event"),
            "delivery": headers.get("X-Cloacina-Delivery"),
            "signed": bool(headers.get("X-Cloacina-Signature")),
            "body": body,
        }
        with self.lock:
            self.records.append(record)
            if self.path:
                with open(self.path, "a", encoding="utf-8") as handle:
                    handle.write(json.dumps(record, sort_keys=True) + "\n")
        return record

    def since(self, stamp):
        with self.lock:
            return [r for r in self.records if (r.get("received_at") or 0) >= stamp]


def handler_for(store):
    class Handler(BaseHTTPRequestHandler):
        def _answer(self, code, value):
            data = json.dumps(value).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_POST(self):  # noqa: N802 - the name http.server calls
            length = int(self.headers.get("Content-Length") or 0)
            if length > MAX_BODY:
                self._answer(413, {"error": "too large"})
                return
            store.add(self.headers, self.rfile.read(length))
            self._answer(200, {"ok": True})

        def do_GET(self):  # noqa: N802
            url = urllib.parse.urlparse(self.path)
            if url.path == "/health":
                self._answer(200, {"ok": True})
                return
            if url.path == PATH:
                query = urllib.parse.parse_qs(url.query)
                try:
                    since = float((query.get("since") or ["0"])[0])
                except ValueError:
                    since = 0.0
                self._answer(200, {"started_at": store.started_at, "records": store.since(since)})
                return
            self._answer(404, {"error": "not found"})

        def log_message(self, *_args):
            pass

    return Handler


def serve(store, host="0.0.0.0", port=PORT):  # noqa: S104 - a pod's own port
    server = ThreadingHTTPServer((host, port), handler_for(store))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def webhook_url(namespace):
    """The URL the server sends alerts to."""
    return f"http://{NAME}.{namespace}.svc.cluster.local:{PORT}{PATH}"


def manifests(namespace, image, script_text):
    """The receiver's ConfigMap (this file), Deployment and Service."""
    labels = {"app.kubernetes.io/name": NAME}
    return [
        {"apiVersion": "v1", "kind": "ConfigMap",
         "metadata": {"name": NAME, "namespace": namespace, "labels": labels},
         "data": {"alerts.py": script_text}},
        {"apiVersion": "apps/v1", "kind": "Deployment",
         "metadata": {"name": NAME, "namespace": namespace, "labels": labels},
         "spec": {
             "replicas": 1,
             "selector": {"matchLabels": labels},
             "template": {
                 "metadata": {"labels": labels},
                 "spec": {
                     "securityContext": {"runAsNonRoot": True, "runAsUser": 10001,
                                         "seccompProfile": {"type": "RuntimeDefault"}},
                     "containers": [{
                         "name": "receiver",
                         "image": image,
                         "imagePullPolicy": "IfNotPresent",
                         "command": ["python3.11", "/receiver/alerts.py", "--port", str(PORT),
                                     "--out", OUT],
                         "ports": [{"containerPort": PORT, "name": "http"}],
                         "readinessProbe": {"httpGet": {"path": "/health", "port": PORT}},
                         "securityContext": {"allowPrivilegeEscalation": False,
                                             "readOnlyRootFilesystem": True,
                                             "capabilities": {"drop": ["ALL"]}},
                         "resources": {"requests": {"cpu": "10m", "memory": "32Mi"},
                                       "limits": {"cpu": "200m", "memory": "128Mi"}},
                         "volumeMounts": [{"name": "script", "mountPath": "/receiver"},
                                          {"name": "data", "mountPath": "/data"}],
                     }],
                     "volumes": [{"name": "script", "configMap": {"name": NAME}},
                                 {"name": "data", "emptyDir": {"sizeLimit": "64Mi"}}],
                 }}}},
        {"apiVersion": "v1", "kind": "Service",
         "metadata": {"name": NAME, "namespace": namespace, "labels": labels},
         "spec": {"selector": labels, "ports": [{"port": PORT, "targetPort": PORT, "name": "http"}]}},
    ]


def main(argv=None):
    parser = argparse.ArgumentParser(prog="alerts.py", description="Keep the alerts a server sends.")
    parser.add_argument("--host", default="0.0.0.0")  # noqa: S104
    parser.add_argument("--port", type=int, default=PORT)
    parser.add_argument("--out", default=None, help="file of JSON lines that keeps the alerts")
    args = parser.parse_args(argv)
    store = Store(args.out)
    server = serve(store, args.host, args.port)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        server.shutdown()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
