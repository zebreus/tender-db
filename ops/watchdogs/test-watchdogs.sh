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
# The fixture checks `x-admin-secret` and answers a mismatch with the real deny()
# body (issue 459). It used to answer 200 to anything, which held AUTHENTICATION
# constant — the "both-ways is per-axis" trap in instrument-discipline.md — so a
# script that never looked at the HTTP status passed every case here while it read
# the server's JSON 403 as "idle, 0 queued, 0 recent" on the real box. The 403 and
# 404 cases below are that axis, varied.
#
# Needs only bash, python3, curl and jq — nothing from the box, no root.
#
# Run: ops/watchdogs/test-watchdogs.sh   (exits 0 on success, 1 on any failure)
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# An unchecked `work=$(mktemp -d)` that failed would write the fixture to /secret,
# /server.py, … (review of 459).
work=$(mktemp -d) && [ -n "$work" ] && [ -d "$work" ] || { echo "FAIL mktemp -d gave no directory"; exit 1; }
trap 'rm -rf "$work"; [ -n "${server_pid:-}" ] && kill "$server_pid" 2>/dev/null' EXIT

echo "TENDER_ADMIN_SECRET=test-secret" >"$work/secret"
failures=0

# The probe's `down` needs systemd's word that tender-db.service has no process (issue
# 459), and this harness runs where systemd is absent (a container) or real (install.sh
# on the box) — neither of which a case can set. So `systemctl` is a stub on PATH that
# answers from $work/systemctl.out with exit $work/systemctl.rc, and refuses any
# question but the one the probe must ask (a probe that asked about another unit would
# find that one stopped). Default: loaded, no main process — the app is down.
mkdir -p "$work/bin"
cat >"$work/bin/systemctl" <<STUB
#!/bin/sh
if [ "\$*" != "show --property=LoadState --property=MainPID tender-db.service" ]; then
    echo "stub systemctl: unexpected question: \$*" >&2
    exit 3
fi
cat "$work/systemctl.out"
exit "\$(cat "$work/systemctl.rc")"
STUB
chmod +x "$work/bin/systemctl"
export PATH="$work/bin:$PATH"
systemd_says() { printf '%s\n' "$1" >"$work/systemctl.out"; echo "${2:-0}" >"$work/systemctl.rc"; }
systemd_says $'MainPID=0\nLoadState=loaded'

# A fixture server that answers /admin/jobs from a file the test rewrites, and
# honours `?limit=` exactly as the real endpoint does (newest N, default 20) —
# because the truncation IS the behaviour under test.
#
# It gates /admin/jobs the way admin.rs deny() does (issue 459): the header
# `x-admin-secret` must equal the fixture secret or the answer is the server's own
# 403 body, byte for byte as the real endpoint sent it on 2026-10-01; and in mode
# `secret-unset` (the service started without TENDER_ADMIN_SECRET) every request
# gets deny()'s 404; in mode `raw` an authenticated request gets the state file's
# bytes as they are, under a 200. `$work/mode` is re-read per request so a test can
# flip it mid-run. /releases serves `$work/releases.json` for driftwatch, and any other path
# is a GitLab-shaped JSON 404.
cat >"$work/server.py" <<'PY'
import json, os, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs

STATE, SECRET, MODE, RELEASES = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
DEFAULT_LIMIT = 20
CAP = 200

def mode():
    try:
        with open(MODE) as f:
            return f.read().strip()
    except OSError:
        return "normal"

class H(BaseHTTPRequestHandler):
    def send(self, code, out):
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers(); self.wfile.write(out)
    def do_GET(self):
        u = urlparse(self.path)
        if u.path == "/releases":
            with open(RELEASES, "rb") as f:
                self.send(200, f.read())
            return
        if u.path != "/admin/jobs":
            self.send(404, b'{"message":"404 Not Found"}'); return
        # admin.rs deny() through error(): serde_json writes the keys sorted.
        if mode() == "secret-unset":
            self.send(404, b'{"error":{"message":"not found","status":404}}'); return
        if self.headers.get("x-admin-secret", "") != SECRET:
            self.send(403, b'{"error":{"message":"bad or missing operator secret","status":403}}'); return
        if mode() == "raw":   # the state file's bytes as they are, under a 200
            with open(STATE, "rb") as f:
                self.send(200, f.read())
            return
        with open(STATE) as f:
            body = json.load(f)
        q = parse_qs(u.query)
        limit = DEFAULT_LIMIT
        if "limit" in q:
            try: limit = max(1, min(CAP, int(q["limit"][0])))
            except ValueError: pass
        if isinstance(body.get("recent"), list):
            body["recent"] = body["recent"][:limit]
        self.send(200, json.dumps(body).encode())
    # POST /admin/jobs, gated the same way, answers what it RECEIVED — its method,
    # content-type and the body parsed as JSON — so a case pins the client's call shape
    # (aj.sh, issue 464). The real endpoint enqueues and answers {"enqueued":[…]}.
    def do_POST(self):
        u = urlparse(self.path)
        raw = self.rfile.read(int(self.headers.get("content-length", "0") or 0))
        if u.path != "/admin/jobs":
            self.send(404, b'{"message":"404 Not Found"}'); return
        if mode() == "secret-unset":
            self.send(404, b'{"error":{"message":"not found","status":404}}'); return
        if self.headers.get("x-admin-secret", "") != SECRET:
            self.send(403, b'{"error":{"message":"bad or missing operator secret","status":403}}'); return
        try:
            posted = json.loads(raw)
        except ValueError:
            self.send(400, b'{"error":{"message":"body is not JSON","status":400}}'); return
        self.send(200, json.dumps({"method": "POST", "content_type": self.headers.get("content-type"),
                                   "posted": posted}).encode())
    def log_message(self, *a): pass

srv = HTTPServer(("127.0.0.1", 0), H)
print(srv.server_port, flush=True)
srv.serve_forever()
PY
echo normal >"$work/mode"
echo '[]' >"$work/releases.json"

# `recent` newest-first, entries `age_hours` apart, all outcome ok unless named.
write_state() {
    python3 - "$work/state.json" "$1" "$2" "${3:-}" "${4:-}" "${5:-}" <<'PY'
import json, sys, time
path, count, spacing_h, failed_at = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), sys.argv[4]
failed_kind, failed_counts = sys.argv[5] or "probe", sys.argv[6] or "fixture failure"
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
    recent[idx]["kind"] = failed_kind
    recent[idx]["counts"] = failed_counts
tmp = path + ".tmp"
json.dump({"current": None, "queued": [], "recent": recent, "measured_at": now}, open(tmp, "w"))
import os; os.replace(tmp, path)
PY
}

start_server() {
    python3 "$work/server.py" "$work/state.json" test-secret "$work/mode" "$work/releases.json" \
        >"$work/port" 2>/dev/null &
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

# refute <name> <text> <out>: the output must NOT contain text — the half of a
# refusal that `check` cannot see (an ERROR line next to a "dry run" line is a
# snapshot that ran anyway).
refute() {
    local name=$1 text=$2 out=$3
    if grep -qF -- "$text" <<<"$out"; then
        echo "FAIL $name — must not contain '$text'"
        echo "     got: $out"
        failures=$((failures + 1))
    else
        echo "ok   $name"
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

# A sizing probe is not a failure: the runbooks size a corpus-wide refold with
# `expect: 1`, which aborts by design with the real count and writes nothing. It
# kept the unit red for 26 h after 479's sizing run (job 1999, 2026-10-05).
write_state 300 1 3 refold 'refold aborted: 14887661 notices match ["text"], expected ~1 — check the profile strings (nothing was written)'
out=$(run_jobwatch); rc=$?
check "an expect=1 refold sizing abort is not reported" 0 "ok jobwatch" "$out" "$rc"
write_state 300 1 3 refold-fields 'refold-fields aborted: 5170 notices carry ["TED-LOT_TITLE"], expected ~1 — check the field ids (nothing was written)'
out=$(run_jobwatch); rc=$?
check "an expect=1 refold-fields sizing abort is not reported" 0 "ok jobwatch" "$out" "$rc"
# ...but a real expectation that missed is: the profile strings were wrong.
write_state 300 1 3 refold 'refold aborted: 0 notices match ["fts:ocds"], expected ~314106 — check the profile strings (nothing was written)'
out=$(run_jobwatch); rc=$?
check "a refold that missed a real expectation is reported" 1 "failed run in last 26h: refold" "$out" "$rc"
write_state 300 1 3 refold 'refold aborted: 14 notices match ["text"], expected ~10 — check the profile strings (nothing was written)'
out=$(run_jobwatch); rc=$?
check "an expectation of ~10 is not mistaken for a sizing probe" 1 "failed run in last 26h: refold" "$out" "$rc"

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

# --- jobwatch: a run that could not look is an ERROR, not ok (issue 459) ---------
# The server's 403 is JSON, so the old `jq -e .` passed it and the null .recent /
# .queued summed to "ok jobwatch: idle, 0 queued, 0 recent covering 0h".
echo "TENDER_ADMIN_SECRET=not-the-secret" >"$work/wrong-secret"
out=$(TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/wrong-secret" \
      bash "$here/tender-db-jobwatch.sh" 2>&1); rc=$?
check "a wrong operator secret fails jobwatch" 1 "ERROR jobwatch: /admin/jobs answered HTTP 403: bad or missing operator secret" "$out" "$rc"
refute "…and it does not also say ok" "ok jobwatch" "$out"

echo secret-unset >"$work/mode"
out=$(run_jobwatch); rc=$?
check "a service without an admin secret (deny()'s 404) fails jobwatch" 1 "HTTP 404: not found" "$out" "$rc"
echo normal >"$work/mode"

# A 200 that is not the jobs object: the summary would otherwise invent zeros.
cp "$work/state.json" "$work/state.keep"
echo '{"measured_at": 1}' >"$work/state.json"
out=$(run_jobwatch); rc=$?
check "a 200 without current/queued/recent fails jobwatch" 1 "not the jobs object" "$out" "$rc"
python3 - "$work/state.json" <<'PY'
import json, sys
json.dump({"current": None, "queued": [], "recent": None, "measured_at": 1}, open(sys.argv[1], "w"))
PY
out=$(run_jobwatch); rc=$?
check "a 200 whose recent is not an array fails jobwatch" 1 "not the jobs object" "$out" "$rc"
cp "$work/state.keep" "$work/state.json"
out=$(run_jobwatch); rc=$?
check "…and the restored fixture passes again (the 403/404 cases are not a broken server)" 0 "ok jobwatch" "$out" "$rc"

# Every conjunct of the shape check, dropped one at a time (review of 459): each of these
# bodies made the old script print the issue's own "ok jobwatch: idle, 0 queued, 0
# recent covering 0h", or name a failed run in its ok line, or crash without an ERROR.
mutate_state() {
    python3 - "$work/state.json" "$1" <<'PY'
import json, os, sys
path, stmt = sys.argv[1], sys.argv[2]
d = json.load(open(path))
exec(stmt)
json.dump(d, open(path + ".tmp", "w")); os.replace(path + ".tmp", path)
PY
}
for m in 'd.pop("current")' 'd["current"] = "fold"' 'd["current"] = False' \
         'd["current"] = {"id": 5, "kind": "project", "params": "p"}' \
         'd["queued"] = None' 'd.pop("queued")' \
         'd["recent"][0].pop("finished_at")' 'd["recent"][0]["finished_at"] = None' \
         'd["recent"][0]["outcome"] = None' 'd["recent"][3] = "a string"'; do
    cp "$work/state.keep" "$work/state.json"
    mutate_state "$m"
    out=$(run_jobwatch); rc=$?
    check "a jobs body with $m fails jobwatch" 1 "not the jobs object" "$out" "$rc"
    refute "…and $m is not called ok" "ok jobwatch" "$out"
done
cp "$work/state.keep" "$work/state.json"
mutate_state 'd["recent"] = []'
out=$(run_jobwatch); rc=$?
check "an empty job log fails jobwatch (a 0-hour window is not a 26-hour check)" 1 "ERROR jobwatch: /admin/jobs answered an EMPTY job log" "$out" "$rc"
refute "…and the empty log is not called ok" "ok jobwatch" "$out"
cp "$work/state.keep" "$work/state.json"

# --- the queue probe (issue 459): named answers, shared by deploy.sh and the snapshot ---
# deploy.sh runs it as `ssh … bash -s < tender-db-queue-probe.sh`, so every case runs
# it that way — the exact bytes, read from stdin, which also pins that nothing in
# the probe reads stdin (it would eat the rest of the script) — and the snapshot's
# way (`bash <file>`), and the two must agree. Output is ONE line by contract;
# stderr is folded in so a stray warning breaks the count.
probe_out=""; probe_rc=0
run_probe() {
    local stdin_out stdin_rc=0 file_out file_rc=0
    stdin_out=$(env TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/secret" \
        "$@" bash -s <"$here/tender-db-queue-probe.sh" 2>&1) || stdin_rc=$?
    file_out=$(env TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/secret" \
        "$@" bash "$here/tender-db-queue-probe.sh" </dev/null 2>&1) || file_rc=$?
    if [ "$stdin_out" != "$file_out" ] || [ "$stdin_rc" != "$file_rc" ]; then
        echo "FAIL the probe answers differently piped (deploy.sh) and as a file (snapshot)"
        echo "     piped: exit $stdin_rc: $stdin_out"
        echo "     file:  exit $file_rc: $file_out"
        failures=$((failures + 1))
    fi
    if [ "$(printf '%s\n' "$stdin_out" | wc -l)" -ne 1 ] || [ -z "$stdin_out" ]; then
        echo "FAIL the probe must print exactly one line — got: $stdin_out"
        failures=$((failures + 1))
    fi
    case "$stdin_out" in
        *[[:space:]]) echo "FAIL the probe's line ends in whitespace — got: '$stdin_out'"; failures=$((failures + 1)) ;;
    esac
    probe_out=$stdin_out; probe_rc=$stdin_rc
}
set_current_full() {
    python3 - "$work/state.json" "$@" <<'PY'
import json, sys
path = sys.argv[1]
d = json.load(open(path))
if len(sys.argv) > 2 and sys.argv[2] == "--drop":
    d.pop("current", None)
elif len(sys.argv) > 2 and sys.argv[2] == "--string":
    d["current"] = "fold"
elif len(sys.argv) > 4:
    d["current"] = {"id": int(sys.argv[2]), "kind": sys.argv[3], "params": sys.argv[4],
                    "started_at": 0}
else:
    d["current"] = None
import os
json.dump(d, open(path + ".tmp", "w")); os.replace(path + ".tmp", path)
PY
}

set_current_full
run_probe
check "the probe reads an idle queue as idle" 0 "idle" "$probe_out" "$probe_rc"
[ "$probe_out" = idle ] || { echo "FAIL the idle answer is exactly 'idle' — got: $probe_out"; failures=$((failures + 1)); }

set_current_full 1796 fetch "ted daily 2026-00190"
run_probe
check "the probe reads a running job as busy <id> <kind> <params>" 0 "busy 1796 fetch ted daily 2026-00190" "$probe_out" "$probe_rc"

set_current_full
run_probe TENDER_ADMIN_SECRET_FILE="$work/wrong-secret"
check "a wrong operator secret is an error, not idle" 1 "error HTTP 403 bad or missing operator secret" "$probe_out" "$probe_rc"

echo secret-unset >"$work/mode"
run_probe
check "a service without an admin secret (404) is an error, not idle" 1 "error HTTP 404 not found" "$probe_out" "$probe_rc"
echo normal >"$work/mode"

run_probe TENDER_ADMIN_SECRET_FILE="$work/missing"
check "a missing secret file is an error, not a skipped gate" 1 "error no readable secret file" "$probe_out" "$probe_rc"
echo "SOMETHING_ELSE=1" >"$work/no-secret-line"
run_probe TENDER_ADMIN_SECRET_FILE="$work/no-secret-line"
check "a secret file without TENDER_ADMIN_SECRET is an error" 1 "error no TENDER_ADMIN_SECRET in" "$probe_out" "$probe_rc"

run_probe TENDER_ADMIN_URL="http://127.0.0.1:1"
check "nothing listening, and a unit with no process, is the named down arm" 0 "down" "$probe_out" "$probe_rc"
[ "$probe_out" = down ] || { echo "FAIL the down answer is exactly 'down' — got: $probe_out"; failures=$((failures + 1)); }
run_probe TENDER_ADMIN_URL="http://localhost:1"
check "…on localhost too" 0 "down" "$probe_out" "$probe_rc"
run_probe TENDER_ADMIN_URL="http://[::1]:1"
check "…and on [::1]" 0 "down" "$probe_out" "$probe_rc"

# A refusal is down ONLY with systemd's word that the service has no process (review of
# 459): with the app up, curl exits 7 just the same for a changed PORT, a proxy that
# refuses, or an address that is not this box's loopback.
systemd_says $'MainPID=4242\nLoadState=loaded'
run_probe TENDER_ADMIN_URL="http://127.0.0.1:1"
check "a refused port while the service has a process is an error, not down (a changed PORT)" 1 "but tender-db.service is not stopped (LoadState=loaded MainPID=4242)" "$probe_out" "$probe_rc"
systemd_says $'MainPID=0\nLoadState=not-found'
run_probe TENDER_ADMIN_URL="http://127.0.0.1:1"
check "a refused port with a unit systemd does not know is an error" 1 "LoadState=not-found" "$probe_out" "$probe_rc"
systemd_says "" 1
run_probe TENDER_ADMIN_URL="http://127.0.0.1:1"
check "a refused port with a systemctl that cannot answer is an error" 1 "systemctl could not say" "$probe_out" "$probe_rc"
systemd_says $'MainPID=0\nLoadState=loaded'
run_probe TENDER_ADMIN_URL="http://0.0.0.0:1"
check "a refusal at an address that is not loopback is an error, not down" 1 "0.0.0.0 is not a loopback address" "$probe_out" "$probe_rc"
# The app is up (the fixture answers) and a proxy that refuses sits in the environment:
# the probe must go direct to the loopback, or it reads the proxy's refusal as down.
run_probe http_proxy="http://127.0.0.1:1" HTTP_PROXY="http://127.0.0.1:1" ALL_PROXY="http://127.0.0.1:1" all_proxy="http://127.0.0.1:1" no_proxy= NO_PROXY=
check "a refusing proxy in the environment is bypassed for the loopback" 0 "idle" "$probe_out" "$probe_rc"
[ "$probe_out" = idle ] || { echo "FAIL …the proxy case reads exactly 'idle' — got: $probe_out"; failures=$((failures + 1)); }

# A curl failure other than "refused" (exit 7) is not down: here exit 1, an unsupported
# scheme. On the box the same arm takes a timeout (exit 28) under load.
run_probe TENDER_ADMIN_URL="nosuchscheme://127.0.0.1:$port"
check "a curl failure other than refused is an error, not down" 1 "error curl exit 1" "$probe_out" "$probe_rc"

set_current_full --drop
run_probe
check "an HTTP 200 without a current key is an error, not idle" 1 "error HTTP 200 without a current key" "$probe_out" "$probe_rc"
set_current_full --string
run_probe
check "an HTTP 200 whose current is neither null nor an object is an error" 1 "error HTTP 200 with a current of type string" "$probe_out" "$probe_rc"
# A 200 that is not JSON at all (a proxy's error page served with the wrong status).
set_current_full
cp "$work/state.json" "$work/state.keep"
echo '<html>upstream hiccup</html>' >"$work/state.json"
echo raw >"$work/mode"
run_probe
check "an HTTP 200 that is not JSON is an error, not idle" 1 "error HTTP 200 but the body is not JSON" "$probe_out" "$probe_rc"
out=$(run_jobwatch); rc=$?
check "…and fails jobwatch too" 1 "not the jobs object" "$out" "$rc"
echo normal >"$work/mode"
cp "$work/state.keep" "$work/state.json"

# --- the snapshot's quiescence gate (issue 420): wait, then skip ---------------
# The reflink itself needs XFS and is not exercised here; TENDER_SNAP_DRY=1 stops
# the script right after the gate, which is the part that lost 2026-09-13's
# snapshot by skipping while the weekly data-quality run was still going.
set_current() {
    python3 - "$work/state.json" "$1" <<'PY'
import json, sys
path, cur = sys.argv[1], sys.argv[2]
d = json.load(open(path))
d["current"] = {"id": int(cur), "kind": "data-quality", "params": "weekly"} if cur else None
# Atomically: the fixture server may be reading this file while a background helper
# rewrites it, and a torn read is a 500 — which the snapshot would (rightly) poll on.
import os
json.dump(d, open(path + ".tmp", "w")); os.replace(path + ".tmp", path)
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

# --- the snapshot reads the queue through the probe (issue 459) -------------------
# Before, a wrong secret or a missing secret file both printed "dry run: would
# snapshot" and exited 0: the 403 had no ['current'], and no secret skipped the gate.
set_current ""
out=$(run_snapshot TENDER_SNAP_WAIT_MIN=0 TENDER_ADMIN_SECRET_FILE="$work/wrong-secret"); rc=$?
check "the snapshot does not snapshot on a wrong secret" 1 "ERROR snapshot: could not read the queue (HTTP 403 bad or missing operator secret)" "$out" "$rc"
refute "…no dry-run line on a wrong secret" "dry run" "$out"

out=$(run_snapshot TENDER_SNAP_WAIT_MIN=0 TENDER_ADMIN_SECRET_FILE="$work/missing"); rc=$?
check "…nor without a secret file" 1 "ERROR snapshot: could not read the queue (no readable secret file" "$out" "$rc"
refute "…no dry-run line without a secret file" "dry run" "$out"

echo secret-unset >"$work/mode"
out=$(run_snapshot TENDER_SNAP_WAIT_MIN=0); rc=$?
check "…nor when the service has no admin secret (404)" 1 "ERROR snapshot: could not read the queue (HTTP 404 not found)" "$out" "$rc"
refute "…no dry-run line on a 404" "dry run" "$out"
echo normal >"$work/mode"

out=$(run_snapshot TENDER_SNAP_WAIT_MIN=0 TENDER_ADMIN_URL="http://127.0.0.1:1"); rc=$?
check "a down app is snapshotted (the named down arm)" 0 "probe: down" "$out" "$rc"
check "…and it is a snapshot" 0 "dry run: would snapshot" "$out" "$rc"
systemd_says $'MainPID=4242\nLoadState=loaded'
out=$(run_snapshot TENDER_SNAP_WAIT_MIN=0 TENDER_ADMIN_URL="http://127.0.0.1:1"); rc=$?
check "a refused port while the service runs is not snapshotted" 1 "ERROR snapshot: could not read the queue (curl exit 7" "$out" "$rc"
refute "…no dry-run line on a refusal with a running service" "dry run" "$out"
systemd_says $'MainPID=0\nLoadState=loaded'

# A failed read polls like a busy queue, because a timeout under load can clear: the
# service answers 404 for two seconds, then the queue reads idle.
echo secret-unset >"$work/mode"
( sleep 2; echo normal >"$work/mode" ) &
helper=$!
out=$(run_snapshot TENDER_SNAP_WAIT_MIN=30); rc=$?
check "an unreadable queue is polled, not skipped or snapshotted" 0 "waiting: could not read the queue (HTTP 404 not found)" "$out" "$rc"
check "…and when it reads idle, the snapshot proceeds" 0 "queue idle after" "$out" "$rc"
check "…into a snapshot" 0 "dry run: would snapshot" "$out" "$rc"
wait "$helper" 2>/dev/null || true
echo normal >"$work/mode"

# The installed snapshot runs the probe from beside itself; an install that left the
# probe out must not read as idle.
mkdir -p "$work/lonely"
cp "$here/tender-db-snapshot.sh" "$here/tender-db-queue-verdict.sh" "$work/lonely/"
out=$(TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/secret" \
      TENDER_SNAP_DRY=1 TENDER_SNAP_POLL_SECS=1 TENDER_SNAP_WAIT_MIN=0 \
      bash "$work/lonely/tender-db-snapshot.sh" 2>&1); rc=$?
check "a snapshot installed without its probe refuses" 1 "ERROR snapshot: could not read the queue" "$out" "$rc"
refute "…no dry-run line without the probe" "dry run" "$out"

# The probe's exit status is half its answer (review of 459). A stub probe beside the
# real snapshot and verdict says a known word with the wrong status, or two words; the
# snapshot must refuse each, and must proceed on the stub's clean `idle` (or the stub
# set-up, not the verdict, would be what refuses).
run_stub_snapshot() {   # run_stub_snapshot <name> <probe body…>
    local d="$work/stub-$1"; shift
    mkdir -p "$d"
    cp "$here/tender-db-snapshot.sh" "$here/tender-db-queue-verdict.sh" "$d/"
    printf '%s\n' "$@" >"$d/tender-db-queue-probe.sh"
    TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/secret" \
        TENDER_SNAP_DRY=1 TENDER_SNAP_POLL_SECS=1 TENDER_SNAP_WAIT_MIN=0 \
        bash "$d/tender-db-snapshot.sh" 2>&1
}
out=$(run_stub_snapshot clean 'echo idle'); rc=$?
check "a stub probe answering idle with exit 0 is snapshotted (the stub set-up works)" 0 "dry run: would snapshot" "$out" "$rc"
out=$(run_stub_snapshot idle1 'echo idle' 'exit 1'); rc=$?
check "idle with exit 1 is not idle: the snapshot refuses" 1 "ERROR snapshot: could not read the queue (unexpected answer 'idle' (exit 1))" "$out" "$rc"
refute "…no dry-run line on idle with exit 1" "dry run" "$out"
out=$(run_stub_snapshot down3 'echo down' 'exit 3'); rc=$?
check "down with exit 3 is not down: the snapshot refuses" 1 "ERROR snapshot: could not read the queue (unexpected answer 'down' (exit 3))" "$out" "$rc"
refute "…no dry-run line on down with exit 3" "dry run" "$out"
out=$(run_stub_snapshot twolines 'printf "idle\nbusy 1 fetch x\n"'); rc=$?
check "an answer over two lines is not idle: the snapshot refuses" 1 "ERROR snapshot: could not read the queue (the probe answered more than one line" "$out" "$rc"
refute "…no dry-run line on a two-line answer" "dry run" "$out"
out=$(run_stub_snapshot silent 'exit 0'); rc=$?
check "no answer with exit 0 is not idle: the snapshot refuses" 1 "ERROR snapshot: could not read the queue (no answer (exit 0))" "$out" "$rc"
refute "…no dry-run line on no answer" "dry run" "$out"
mkdir -p "$work/stub-noverdict"
cp "$here/tender-db-snapshot.sh" "$here/tender-db-queue-probe.sh" "$work/stub-noverdict/"
out=$(TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$work/secret" \
      TENDER_SNAP_DRY=1 TENDER_SNAP_POLL_SECS=1 TENDER_SNAP_WAIT_MIN=0 \
      bash "$work/stub-noverdict/tender-db-snapshot.sh" 2>&1); rc=$?
check "a snapshot installed without its queue verdict refuses" 1 "ERROR snapshot: cannot load" "$out" "$rc"
refute "…no dry-run line without the verdict" "dry run" "$out"

# --- queue_verdict: the rule deploy.sh and the snapshot both act on (issue 459) -----------
# deploy.sh's `case` only maps this function's line to an action, so its whole decision
# is pinned here. Each case sources the real file in a subshell: a verdict that called
# `exit` could not end this harness.
verdict_is() {   # verdict_is <name> <rc> <out> <want: exact line, or "error" for any error line>
    local name=$1 rc=$2 out=$3 want=$4 got
    got=$( . "$here/tender-db-queue-verdict.sh" && queue_verdict "$rc" "$out" ) || got="(verdict exit $?) $got"
    if [ "$(printf '%s\n' "$got" | wc -l)" -ne 1 ]; then
        echo "FAIL $name — the verdict must be one line, got: $got"; failures=$((failures + 1)); return
    fi
    case "$want" in
        error) [[ "$got" == "error "?* ]] || { echo "FAIL $name — wanted an error line, got: $got"; failures=$((failures + 1)); return; } ;;
        *)     [ "$got" = "$want" ] || { echo "FAIL $name — wanted '$want', got: $got"; failures=$((failures + 1)); return; } ;;
    esac
    echo "ok   $name"
}
verdict_is "verdict: idle with exit 0 proceeds" 0 idle idle
verdict_is "verdict: down with exit 0 proceeds" 0 down down
verdict_is "verdict: busy with exit 0 is busy" 0 "busy 1796 fetch ted daily 2026-00190" "busy 1796 fetch ted daily 2026-00190"
verdict_is "verdict: the probe's own error with exit 1 is that error" 1 "error HTTP 403 bad or missing operator secret" "error HTTP 403 bad or missing operator secret"
verdict_is "verdict: idle with exit 1 is an error" 1 idle error
verdict_is "verdict: down with exit 3 is an error" 3 down error
verdict_is "verdict: busy with exit 1 is an error" 1 "busy 1 fetch x" error
verdict_is "verdict: an error line with exit 0 is still an error" 0 "error something" error
verdict_is "verdict: idle then busy on two lines is an error" 0 $'idle\nbusy 1 fetch x' error
verdict_is "verdict: busy then idle on two lines is an error" 0 $'busy 1 fetch x\nidle' error
verdict_is "verdict: idle with a carriage return is an error" 0 $'idle\r' error
verdict_is "verdict: idle with a trailing blank is an error" 0 "idle " error
verdict_is "verdict: IDLE is not idle" 0 IDLE error
verdict_is "verdict: a bare 'busy ' names no job and is an error" 0 "busy " error
verdict_is "verdict: no answer with exit 0 is an error" 0 "" error
verdict_is "verdict: no answer from a failed ssh (255) is an error" 255 "" error
verdict_is "verdict: no exit status at all is an error" "" idle error

# --- aj.sh: the handover's admin helper, versioned (issue 464) ---------------------------
# `/root/aj.sh <path>` GETs, `/root/aj.sh <path> '<json>'|@file` POSTs. It lived only on
# the box while 24 issue files, six open Verify lines and the handover called it — issue
# 224's shape. The fixture compares the header with the BARE value, so the 200 below is
# also the proof that aj.sh strips `TENDER_ADMIN_SECRET=` the way admin.sh and the
# watchdogs do: the whole line, as `SECRET=$(cat /root/tender-admin-secret)` sent it, is
# the 403 of the wrong-secret case.
aj_case() {   # aj_case <name> <want exit> <jq test on stdout | EMPTY | -> <stderr text | -> <secret file> <aj.sh args…>
    local name=$1 want_rc=$2 want_out=$3 want_err=$4 secret_file=$5; shift 5
    local out err rc=0 ok=1
    out=$(TENDER_ADMIN_URL="http://127.0.0.1:$port" TENDER_ADMIN_SECRET_FILE="$secret_file" \
          bash "$here/../aj.sh" "$@" 2>"$work/aj.err") || rc=$?
    err=$(cat "$work/aj.err")
    [ "$rc" = "$want_rc" ] || ok=0
    case "$want_out" in
        -) ;;
        EMPTY) [ -z "$out" ] || ok=0 ;;
        *) jq -e "$want_out" <<<"$out" >/dev/null 2>&1 || ok=0 ;;
    esac
    case "$want_err" in
        -) [ -z "$err" ] || ok=0 ;;   # a clean answer says nothing on stderr
        *) grep -qF -- "$want_err" <<<"$err" || ok=0 ;;
    esac
    if [ "$ok" = 1 ]; then
        echo "ok   $name"
    else
        echo "FAIL $name — wanted exit $want_rc, stdout $want_out, stderr '$want_err'"
        echo "     got exit $rc"
        echo "     stdout: $out"
        echo "     stderr: $err"
        failures=$((failures + 1))
    fi
}
write_state 3 1
echo normal >"$work/mode"
jobs_object='(.current == null) and (.queued | type == "array") and (.recent | type == "array") and (.recent | length == 3)'
aj_case "aj.sh GETs /admin/jobs with the stripped secret and gets the jobs object" \
    0 "$jobs_object" - "$work/secret" /admin/jobs
aj_case "aj.sh sends the path whole, query string included" \
    0 '.recent | length == 2' - "$work/secret" '/admin/jobs?limit=2'
aj_case "aj.sh with a wrong secret prints the server's 403 and exits 1" \
    1 '.error.status == 403' "aj.sh: GET /admin/jobs answered HTTP 403" "$work/wrong-secret" /admin/jobs
# A file whose stripped value is the whole real line, so aj.sh sends what `$(cat …)` did.
sed 's/^/TENDER_ADMIN_SECRET=/' "$work/secret" >"$work/whole-line-secret"
aj_case "…and so does the whole KEY=VALUE line sent as the header (the old documented reader)" \
    1 '.error.status == 403' "answered HTTP 403" "$work/whole-line-secret" /admin/jobs
echo secret-unset >"$work/mode"
aj_case "aj.sh against a service without an admin secret prints the 404 and exits 1" \
    1 '.error.status == 404' "aj.sh: GET /admin/jobs answered HTTP 404" "$work/secret" /admin/jobs
echo normal >"$work/mode"
aj_case "aj.sh without a secret file refuses before asking" \
    1 EMPTY "aj.sh: no operator secret in $work/missing" "$work/missing" /admin/jobs
aj_case "aj.sh with a secret file that holds no TENDER_ADMIN_SECRET refuses" \
    1 EMPTY "aj.sh: no operator secret in" "$work/no-secret-line" /admin/jobs
aj_case "aj.sh <path> '<json>' POSTs the JSON" \
    0 '.method == "POST" and .content_type == "application/json" and .posted == {"kind": "project"}' - \
    "$work/secret" /admin/jobs '{"kind":"project"}'
printf '%s\n' '{"kind":"backfill","source":"doe","range":["2024-01","2024-12"]}' >"$work/body.json"
aj_case "aj.sh <path> @file POSTs the file's bytes" \
    0 '.method == "POST" and .posted.kind == "backfill" and .posted.range == ["2024-01","2024-12"]' - \
    "$work/secret" /admin/jobs "@$work/body.json"
aj_case "aj.sh <path> @missing refuses instead of POSTing an empty body" \
    2 EMPTY "aj.sh: cannot read $work/nope.json" "$work/secret" /admin/jobs "@$work/nope.json"
aj_case "aj.sh POST <path> <json> is refused: there is no method word" \
    2 EMPTY "there is no method word" "$work/secret" POST /admin/jobs '{"kind":"project"}'
aj_case "aj.sh POST <path> is refused too (a method word is not a path)" \
    2 EMPTY "'POST' is not a path" "$work/secret" POST /admin/jobs
aj_case "aj.sh with no arguments prints its usage" \
    2 EMPTY "usage: aj.sh <path>" "$work/secret"
# A valid path, so only the arity check can refuse it: without it the method stays GET
# (POST needs exactly two arguments) and the jobs object comes back with exit 0 — a no-op
# that reads as success (review of 464). The "POST <path> <json>" case above cannot pin
# this: the path check refuses 'POST' first, with the same usage text.
aj_case "aj.sh <path> <json> <extra> is refused, not sent as a GET" \
    2 EMPTY "usage: aj.sh <path>" "$work/secret" /admin/jobs '{"kind":"fetch"}' '{"kind":"process"}'
out=$(TENDER_ADMIN_URL="http://127.0.0.1:1" TENDER_ADMIN_SECRET_FILE="$work/secret" \
      bash "$here/../aj.sh" /admin/jobs 2>&1); rc=$?
check "aj.sh says so when nothing answers" 1 "aj.sh: GET /admin/jobs: no answer from http://127.0.0.1:1 (curl exit 7)" "$out" "$rc"
# install.sh needs root and writes /root, so it cannot run here; this pins that it still
# puts aj.sh where the issue files call it, root-only like the secret it reads.
if grep -qxF 'install -m 0700 -o root -g root "$here/../aj.sh" /root/aj.sh' "$here/install.sh"; then
    echo "ok   install.sh installs ops/aj.sh as /root/aj.sh, mode 0700"
else
    echo "FAIL install.sh no longer installs ops/aj.sh as /root/aj.sh (mode 0700, root)"
    failures=$((failures + 1))
fi

# --- driftwatch: a probe that could not look exits 1 (issue 459) ------------------
# Both-ways first: the fixture's release list must be able to pass and to alarm, or
# "exit 1 on a 404" below would also be satisfied by a script that always exits 1.
run_drift() {
    TENDER_DRIFT_API="$1" bash "$here/tender-db-driftwatch.sh" 2>&1
}
echo '[{"tag_name": "1.14.3"}, {"tag_name": "1.14.2"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch passes on releases inside the vendored line" 0 "ok drift: no SDK-eforms-de release beyond the vendored 1.14.x line (2 releases read, highest 1.14.3)" "$out" "$rc"

# The real feed's order on 2026-10-01: an older line's fix (1.13.3) above a 1.14.x.
echo '[{"tag_name": "1.14.4"}, {"tag_name": "1.13.3"}, {"tag_name": "1.14.2"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch passes when an older line's fix is listed above the vendored line" 0 "highest 1.14.4" "$out" "$rc"

echo '[{"tag_name": "2.2.0"}, {"tag_name": "1.14.3"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch alarms on a release beyond the vendored line" 1 "BEYOND the vendored 1.14.x line" "$out" "$rc"

# Index 0 is the newest by date, not the newest line: once 1.15.0 ships, a later 1.14.x
# fix sits above it (review of 459).
echo '[{"tag_name": "1.14.5"}, {"tag_name": "1.15.0"}, {"tag_name": "1.14.4"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch alarms on a successor that is not the first entry" 1 "SDK-eforms-de released 1.15.0 — BEYOND the vendored 1.14.x line" "$out" "$rc"
refute "…and does not call it ok" "ok drift" "$out"

echo '[{"tag_name": "1.140.0"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch compares lines as numbers (1.140 is beyond 1.14, not on it)" 1 "BEYOND the vendored 1.14.x line" "$out" "$rc"

echo '[{"tag_name": "1.14.4"}, {"tag_name": "release-2026-09"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch fails on a tag it cannot parse, anywhere on the page" 1 "answered unparseably (release tag 'release-2026-09' is not" "$out" "$rc"

echo '[{"tag_name": "1.14.4"}, {"name": "untagged"}]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch fails on a release without a tag, anywhere on the page" 1 "answered unparseably (release #1 on the page has no tag_name)" "$out" "$rc"

echo '[{"tag_name": "1.14.4"}]' >"$work/releases.json"
out=$(TENDER_DRIFT_KNOWN_PREFIX=1.14.x run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch fails on a vendored line that is not <major>.<minor>" 1 "TENDER_DRIFT_KNOWN_PREFIX='1.14.x' is not <major>.<minor>" "$out" "$rc"

out=$(run_drift "http://127.0.0.1:$port/api/v4/projects/gone/releases"); rc=$?
check "driftwatch fails on an answer that is not a release list (404)" 1 "ERROR drift: SDK-eforms-de release probe answered HTTP 404" "$out" "$rc"
refute "…and does not call it ok" "ok drift" "$out"

out=$(run_drift "http://127.0.0.1:$port/admin/jobs"); rc=$?
check "driftwatch fails on a 403 (the issue's Verify shape)" 1 "answered HTTP 403" "$out" "$rc"

out=$(run_drift "http://127.0.0.1:1/releases"); rc=$?
check "driftwatch fails when unreachable" 1 "ERROR drift: SDK-eforms-de release probe failed (network: curl exit 7)" "$out" "$rc"

echo '{"message": "moved"}' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch fails on a 200 that is not a list" 1 "answered unparseably (the body is a JSON dict, not a list)" "$out" "$rc"

echo '[]' >"$work/releases.json"
out=$(run_drift "http://127.0.0.1:$port/releases"); rc=$?
check "driftwatch fails on an empty release list" 1 "answered unparseably (the release list is empty)" "$out" "$rc"

echo
if [ "$failures" -eq 0 ]; then
    echo "all watchdog tests passed"
    exit 0
fi
echo "$failures watchdog test(s) failed"
exit 1
