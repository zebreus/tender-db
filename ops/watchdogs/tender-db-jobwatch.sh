#!/usr/bin/env bash
# tender-db job-failure watch — warn (to the journal) on a recently-FAILED job run
# or a job WEDGED in-flight past a threshold. Detection only; runs hourly via
# tender-db-jobwatch.timer. Reads the local admin API with the operator secret.
#
# Sources of truth (GET /admin/jobs, see model::ingestion):
#   .recent[]  JobRun  { id, kind, params, started_at, finished_at, outcome, counts }
#   .current   JobProgress?  { id, kind, params, started_at, … }  (null when idle)
#   .queued[]  QueuedJob { id, kind, params }
#
# A deliberate sizing probe is NOT a failure: `refold`/`refold-fields` with
# `expect: 1` abort by design with the real count ("… expected ~1 — …", nothing
# written), and the runbooks size every corpus-wide refold that way. Only that
# exact expectation is exempt; a refold that missed a real `expect` still warns.
#
# A failed daily is a .recent entry whose outcome != "ok" that finished inside the
# lookback window. A wedged job is a .current that has been running longer than the
# wedged threshold — set above the ~5 h a full `project rebuild=true` legitimately
# takes (id 697 ran 5.1 h), so a healthy rebuild does not trip it — AND whose progress
# record (`.current.phase.updated_at`) is older than the stall threshold, or absent.
# Runtime alone was the old rule, and the all-profile refold of 2026-10-07 (project
# 2044, 8.78M Tenders rewritten) tripped it at 8.8 h while folding ~1M Tenders an hour;
# a job that keeps reporting progress is slow, not wedged. Past the hard cap (24 h) a
# job warns even while it reports progress: no job here is meant to run that long. A clean run
# exits 0 with one OK summary line; any finding exits non-zero so it also shows in
# `systemctl --failed`.
#
# A run that could not LOOK is not a clean run (issue 459). Only an HTTP 200 whose
# body has `current`, an array `queued` and a NON-EMPTY array `recent`, with the fields
# the checks read present and typed, is read at all; anything else — no secret,
# unreachable, a 403/404/500, a body of another shape, an empty log — prints
# `ERROR jobwatch: … — jobs NOT checked` and exits 1. Before that, curl had no
# status check and a JSON 403 passed `jq -e .`: with `.recent` and `.queued` null the
# script printed `ok jobwatch: idle, 0 queued, 0 recent covering 0h` and exited 0,
# reproduced 2026-10-01 against the real endpoint with a wrong secret.
#
# Versioned in the repo (ops/watchdogs/) and installed to /usr/local/bin by
# ops/watchdogs/install.sh — see the diskwatch header for why these live in git
# (issue 224).
set -euo pipefail

base=${TENDER_ADMIN_URL:-http://localhost:8080}
secret_file=${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}
lookback=${TENDER_JOB_FAIL_LOOKBACK_SECS:-93600}   # 26 h — one daily cycle + slack
wedged=${TENDER_JOB_WEDGED_SECS:-28800}            # 8 h — clears the ~5 h full rebuild
stall=${TENDER_JOB_STALL_SECS:-7200}               # 2 h without a phase update — wedged
hard=${TENDER_JOB_HARD_WEDGED_SECS:-86400}         # 24 h — wedged even while progressing
# How deep to read the job log. This MUST be asked for explicitly: `GET
# /admin/jobs` with no `limit` returns the newest 20 runs, and a 26 h lookback
# over a 20-entry window is only a 26 h check on a quiet box. Measured 2026-09-09
# — the default 20 reached back just 17.7 h while the lookback claimed 26 h, an
# 8.3 h blind band, and one session's eight refold jobs had eaten 40 % of the
# window. Issue 313 added this parameter for exactly this reason ("a hard-coded 20
# hid a day of history during an incident hunt") and this watchdog never used it.
# 200 spanned ~5 days at the observed rate; the supervisor caps a request at JOB_LOG_MAX (2,000 since).
depth=${TENDER_JOB_LOG_DEPTH:-200}

# /root/tender-admin-secret is a systemd EnvironmentFile (`TENDER_ADMIN_SECRET=…`),
# so the operator secret is the value after the first '='.
secret=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' "$secret_file" 2>/dev/null || true)
if [ -z "$secret" ]; then
    echo "ERROR jobwatch: no operator secret in $secret_file — jobs NOT checked"
    exit 1
fi

body_file=$(mktemp)
trap 'rm -f "$body_file"' EXIT
now=$(date +%s)
curl_rc=0
code=$(curl -sS --max-time 15 -o "$body_file" -w '%{http_code}' \
    "$base/admin/jobs?limit=$depth" -H "x-admin-secret: $secret" 2>/dev/null) || curl_rc=$?
if [ "$curl_rc" -ne 0 ]; then
    echo "ERROR jobwatch: /admin/jobs unreachable at $base (curl exit $curl_rc) — jobs NOT checked"
    exit 1
fi
# The status first: this server's 403 and 404 are JSON (admin.rs deny(): 403 `bad or
# missing operator secret` on a mismatch, 404 `not found` when the service has no
# secret), so "is it JSON" says nothing about whether the queue was read.
if [ "$code" != "200" ]; then
    msg=$(jq -r '.error.message // empty' "$body_file" 2>/dev/null || true)
    [ -n "$msg" ] || msg=$(head -c 200 "$body_file" | tr '\r\n' '  ')
    echo "ERROR jobwatch: /admin/jobs answered HTTP $code: ${msg:-(empty body)} — jobs NOT checked"
    exit 1
fi
# The shape, down to the fields the checks below read — each with no default. A
# `(.finished_at // 0)` read an entry without a finish time as ancient, so a failed run
# was never "inside the lookback" and the ok line named the failure itself; a
# `(.started_at // $now)` read a job without a start time as just started, never wedged
# (review of 459). model::ingestion types them as i64 and String, so a missing or
# mistyped one means this is not the endpoint it thinks it is.
if ! jq -e 'type == "object" and has("current") and has("queued") and has("recent")
            and (.queued | type) == "array" and (.recent | type) == "array"
            and (.current == null
                 or ((.current | type) == "object" and (.current.started_at | type) == "number"))
            and all(.recent[]; type == "object"
                               and (.finished_at | type) == "number"
                               and (.outcome | type) == "string")' "$body_file" >/dev/null 2>&1; then
    echo "ERROR jobwatch: /admin/jobs answered HTTP 200 but not the jobs object (current null or with a numeric started_at, array queued, array recent of runs with a numeric finished_at and a string outcome) — jobs NOT checked"
    exit 1
fi
# An empty log is a 0-hour window, not a clean 26-hour check. The log is persisted
# (job_log) and survives restarts, and this box runs dailies, so `recent: []` means the
# probe is reading the wrong service or database. A freshly built database is the one
# honest exception, and it clears after its first job.
if [ "$(jq -r '.recent | length' "$body_file")" = "0" ]; then
    echo "ERROR jobwatch: /admin/jobs answered an EMPTY job log — on a box that runs dailies that is the wrong service or database (or a database built from zero, until its first job) — jobs NOT checked"
    exit 1
fi
json=$(cat "$body_file")

status=0

failed=$(printf '%s' "$json" | jq -r --argjson now "$now" --argjson lb "$lookback" '
    .recent[]
    | select(.outcome != "ok")
    | select(.finished_at >= ($now - $lb))
    | select((.counts // "") | test("^(refold|refold-fields) aborted: .* expected ~1 — ") | not)
    | "\(.kind) #\(.id) [\(.params)] → \(.outcome)"')
if [ -n "$failed" ]; then
    while IFS= read -r line; do
        echo "WARN jobwatch: failed run in last $((lookback / 3600))h: $line"
    done <<<"$failed"
    status=1
fi

# Even at depth 200 a busy enough day can fill the window inside the lookback.
# That is the failure mode this watchdog just had, so it must never be silent
# again: when the log comes back full AND its oldest entry is younger than the
# lookback start, an older failure cannot be seen and the "ok" below would be a
# claim about a window, not about the day.
oldest=$(printf '%s' "$json" | jq -r '[.recent[].finished_at] | min // empty')
returned=$(printf '%s' "$json" | jq -r '.recent | length')
if [ -n "$oldest" ] && [ "$returned" -ge "$depth" ] && [ "$oldest" -gt "$((now - lookback))" ]; then
    echo "WARN jobwatch: job log saturated — $returned entries reach back only" \
         "$(((now - oldest) / 3600))h but the lookback is $((lookback / 3600))h;" \
         "a failure older than that is INVISIBLE here (raise TENDER_JOB_LOG_DEPTH)"
    status=1
fi

wedged_line=$(printf '%s' "$json" | jq -r --argjson now "$now" --argjson w "$wedged" \
        --argjson st "$stall" --argjson hard "$hard" '
    (.current // empty)
    | select(.started_at <= ($now - $w))
    | (.phase.updated_at? // null) as $u
    | select(.started_at <= ($now - $hard)
             or ($u | type) != "number"
             or $u <= ($now - $st))
    | "\(.kind) #\(.id) [\(.params)] running \($now - .started_at)s"
      + (if ($u | type) == "number" then ", last progress \($now - $u)s ago" else ", no progress record" end)')
if [ -n "$wedged_line" ]; then
    echo "WARN jobwatch: wedged job (threshold $((wedged / 3600))h): $wedged_line"
    status=1
fi

if [ "$status" -eq 0 ]; then
    summary=$(printf '%s' "$json" | jq -r --argjson now "$now" '
        (.recent | length) as $n
        | (.recent[0] // {}) as $last
        | (if .current then "running \(.current.kind) #\(.current.id) (\($now - .current.started_at)s)" else "idle" end) as $cur
        | ([.recent[].finished_at] | min) as $oldest
        | "ok jobwatch: \($cur), \((.queued | length)) queued, \($n) recent covering \(if $oldest then (($now - $oldest) / 3600 | floor) else 0 end)h, last \($last.kind // "?") → \($last.outcome // "?")"')
    echo "$summary"
fi

exit "$status"
