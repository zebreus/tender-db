#!/usr/bin/env bash
# Offline tests for the watchdog scripts — no production box, no admin secret.
#
# WHY this exists (issue 373). `tender-db-jobwatch.sh` spent an unknown stretch
# applying a 26 h "failed daily" lookback to whatever `GET /admin/jobs` returns
# by DEFAULT, which is the newest 20 runs. On a busy box those are not the same
# window: measured on prod 2026-09-09 the default reached back 17.7 h, an 8.3 h
# band in which a failed daily was invisible while the journal said "ok". The
# bug was in three characters of a curl URL and survived because these scripts
# had exactly one way to be checked — running them against the real box, where
# a saturated window looks identical to a quiet one.
#
# So each case here pins an ARITHMETIC relationship between the lookback and the
# data the script can actually see, using a fixture server instead of prod. The
# saturation case is the one that would have caught issue 373.
#
# Run: ops/watchdogs/test-watchdogs.sh   (exits 0 on success, 1 on any failure)
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"; [ -n "${server_pid:-}" ] && kill "$server_pid" 2>/dev/null' EXIT

echo "TENDER_ADMIN_SECRET=test-secret" >"$work/secret"
failures=0

# A fixture server that answers /admin/jobs from a file the test rewrites, and
# honours `?limit=` exactly as the real endpoint does (newest N, default 20) —
# because the truncation IS the behaviour under test.
cat >"$work/server.py" <<'PY'
import json, os, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs

STATE = sys.argv[1]
DEFAULT_LIMIT = 20
CAP = 200

class H(BaseHTTPRequestHandler):
    def do_GET(self):
        u = urlparse(self.path)
        if u.path != "/admin/jobs":
            self.send_response(404); self.end_headers(); return
        with open(STATE) as f:
            body = json.load(f)
        q = parse_qs(u.query)
        limit = DEFAULT_LIMIT
        if "limit" in q:
            try: limit = max(1, min(CAP, int(q["limit"][0])))
            except ValueError: pass
        body["recent"] = body.get("recent", [])[:limit]
        out = json.dumps(body).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers(); self.wfile.write(out)
    def log_message(self, *a): pass

srv = HTTPServer(("127.0.0.1", 0), H)
print(srv.server_port, flush=True)
srv.serve_forever()
PY

# `recent` newest-first, entries `age_hours` apart, all outcome ok unless named.
write_state() {
    python3 - "$work/state.json" "$1" "$2" "${3:-}" <<'PY'
import json, sys, time
path, count, spacing_h, failed_at = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), sys.argv[4]
now = int(time.time())
recent = []
for i in range(count):
    fin = now - int(i * spacing_h * 3600)
    recent.append({
        "id": 1000 + i, "job_id": 2000 + i, "kind": "project",
        "params": "rebuild=false", "started_at": fin - 30, "finished_at": fin,
        "outcome": "ok", "counts": "",
    })
if failed_at:
    idx = int(failed_at)
    recent[idx]["outcome"] = "error"
    recent[idx]["kind"] = "probe"
    recent[idx]["counts"] = "fixture failure"
json.dump({"current": None, "queued": [], "recent": recent, "measured_at": now}, open(path, "w"))
PY
}

start_server() {
    python3 "$work/server.py" "$work/state.json" >"$work/port" 2>/dev/null &
    server_pid=$!
    for _ in $(seq 1 50); do
        port=$(cat "$work/port" 2>/dev/null) && [ -n "$port" ] && return 0
        sleep 0.1
    done
    echo "FAIL: fixture server did not start"; exit 1
}

run_jobwatch() {
    TENDER_ADMIN_URL="http://127.0.0.1:$port" \
    TENDER_ADMIN_SECRET_FILE="$work/secret" \
    env "$@" bash "$here/tender-db-jobwatch.sh" 2>&1
}

check() {
    local name=$1 want_exit=$2 want_text=$3 out=$4 got_exit=$5
    local ok=1
    [ "$got_exit" = "$want_exit" ] || ok=0
    [ -z "$want_text" ] || grep -qF -- "$want_text" <<<"$out" || ok=0
    if [ "$ok" = 1 ]; then
        echo "ok   $name"
    else
        echo "FAIL $name — wanted exit $want_exit containing '$want_text'"
        echo "     got exit $got_exit: $out"
        failures=$((failures + 1))
    fi
}

# 300 entries 1 h apart: the deep window spans well past the 26 h lookback.
write_state 300 1
start_server

out=$(run_jobwatch); rc=$?
check "a quiet log with deep coverage passes" 0 "ok jobwatch" "$out" "$rc"

# THE ISSUE-373 CASE. Same log, but the script only reads 20 entries: it can see
# 19 h and is asked about 26 h. Before the fix this printed "ok"; now it must
# refuse to make a claim it cannot support.
out=$(run_jobwatch TENDER_JOB_LOG_DEPTH=20); rc=$?
check "a window shallower than the lookback is refused, not called ok" 1 "job log saturated" "$out" "$rc"

# And the fix's own arithmetic: depth 200 over the same fixture is NOT saturated
# (200 h of coverage against 26 h), so it must not cry wolf either.
out=$(run_jobwatch TENDER_JOB_LOG_DEPTH=200); rc=$?
check "a window deeper than the lookback stays quiet" 0 "ok jobwatch" "$out" "$rc"

# A failure inside the lookback is reported...
write_state 300 1 3
out=$(run_jobwatch); rc=$?
check "a failure inside the lookback is reported" 1 "failed run in last 26h" "$out" "$rc"

# ...and one outside it is not. Entry 200 is 200 h old, well past 26 h — and
# with depth 200 the window still covers the lookback, so silence here means
# "looked and found nothing", not "could not see".
write_state 300 1 200
out=$(run_jobwatch TENDER_JOB_LOG_DEPTH=250); rc=$?
check "a failure older than the lookback is not reported" 0 "ok jobwatch" "$out" "$rc"

# The summary must state the span it actually checked, so a reader can tell a
# 134 h check from a 19 h one without re-deriving it.
write_state 300 1
out=$(run_jobwatch TENDER_JOB_LOG_DEPTH=100); rc=$?
check "the ok line states the span it checked" 0 "recent covering 99h" "$out" "$rc"

# Unreachable endpoint and a missing secret must both warn rather than pass.
out=$(TENDER_ADMIN_URL="http://127.0.0.1:1" TENDER_ADMIN_SECRET_FILE="$work/secret" \
      bash "$here/tender-db-jobwatch.sh" 2>&1); rc=$?
check "an unreachable endpoint warns" 1 "unreachable" "$out" "$rc"

out=$(TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/missing" \
      bash "$here/tender-db-jobwatch.sh" 2>&1); rc=$?
check "a missing operator secret warns" 1 "no operator secret" "$out" "$rc"

# --- the snapshot's quiescence gate (issue 420): wait, then skip ---------------
# The reflink itself needs XFS and is not exercised here; TENDER_SNAP_DRY=1 stops
# the script right after the gate, which is the part that lost 2026-09-13's
# snapshot by skipping while the weekly data-quality run was still going.
set_current() {
    python3 - "$work/state.json" "$1" <<'PY'
import json, sys
path, cur = sys.argv[1], sys.argv[2]
d = json.load(open(path))
d["current"] = {"id": int(cur), "kind": "data-quality"} if cur else None
json.dump(d, open(path, "w"))
PY
}
run_snapshot() {
    TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/secret" \
    TENDER_SNAP_DRY=1 TENDER_SNAP_POLL_SECS=1 env "$@" bash "$here/tender-db-snapshot.sh" 2>&1
}

write_state 3 1
set_current ""
out=$(run_snapshot); rc=$?
check "an idle queue snapshots at once (no wait)" 0 "dry run: would snapshot" "$out" "$rc"
if grep -q "waiting:" <<<"$out"; then echo "FAIL idle queue must not wait: $out"; failures=$((failures + 1)); fi

set_current 1341
out=$(run_snapshot TENDER_SNAP_WAIT_MIN=0); rc=$?
check "a running job past the wait budget skips loudly" 0 "skipped snapshot: job 1341 still running" "$out" "$rc"
if grep -q "dry run" <<<"$out"; then echo "FAIL a skip must not snapshot: $out"; failures=$((failures + 1)); fi

set_current 1341
( sleep 2; set_current "" ) &
helper=$!
out=$(run_snapshot TENDER_SNAP_WAIT_MIN=30); rc=$?
check "a running job is waited out, then the snapshot proceeds" 0 "queue idle after" "$out" "$rc"
check "…and the wait ends in a snapshot, not a skip" 0 "dry run: would snapshot" "$out" "$rc"
wait "$helper" 2>/dev/null || true

echo
if [ "$failures" -eq 0 ]; then
    echo "all watchdog tests passed"
    exit 0
fi
echo "$failures watchdog test(s) failed"
exit 1
