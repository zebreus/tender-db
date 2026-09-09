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
# A failed daily is a .recent entry whose outcome != "ok" that finished inside the
# lookback window. A wedged job is a .current that has been running longer than the
# wedged threshold — set above the ~5 h a full `project rebuild=true` legitimately
# takes (id 697 ran 5.1 h), so a healthy rebuild does not trip it. A clean run
# exits 0 with one OK summary line; any finding exits non-zero so it also shows in
# `systemctl --failed`.
#
# Versioned in the repo (ops/watchdogs/) and installed to /usr/local/bin by
# ops/watchdogs/install.sh — see the diskwatch header for why these live in git
# (issue 224).
set -euo pipefail

base=${TENDER_ADMIN_URL:-http://localhost:8080}
secret_file=${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}
lookback=${TENDER_JOB_FAIL_LOOKBACK_SECS:-93600}   # 26 h — one daily cycle + slack
wedged=${TENDER_JOB_WEDGED_SECS:-28800}            # 8 h — clears the ~5 h full rebuild
# How deep to read the job log. This MUST be asked for explicitly: `GET
# /admin/jobs` with no `limit` returns the newest 20 runs, and a 26 h lookback
# over a 20-entry window is only a 26 h check on a quiet box. Measured 2026-09-09
# — the default 20 reached back just 17.7 h while the lookback claimed 26 h, an
# 8.3 h blind band, and one session's eight refold jobs had eaten 40 % of the
# window. Issue 313 added this parameter for exactly this reason ("a hard-coded 20
# hid a day of history during an incident hunt") and this watchdog never used it.
# 200 is the supervisor's JOB_LOG_MAX; at the observed rate that spans ~5 days.
depth=${TENDER_JOB_LOG_DEPTH:-200}

# /root/tender-admin-secret is a systemd EnvironmentFile (`TENDER_ADMIN_SECRET=…`),
# so the operator secret is the value after the first '='.
secret=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' "$secret_file" 2>/dev/null || true)
if [ -z "$secret" ]; then
    echo "WARN jobwatch: no operator secret in $secret_file — cannot check jobs"
    exit 1
fi

now=$(date +%s)
if ! json=$(curl -sS --max-time 15 "$base/admin/jobs?limit=$depth" -H "x-admin-secret: $secret" 2>/dev/null); then
    echo "WARN jobwatch: /admin/jobs unreachable at $base"
    exit 1
fi
if ! printf '%s' "$json" | jq -e . >/dev/null 2>&1; then
    echo "WARN jobwatch: /admin/jobs returned non-JSON (bad secret or server error)"
    exit 1
fi

status=0

failed=$(printf '%s' "$json" | jq -r --argjson now "$now" --argjson lb "$lookback" '
    .recent[]?
    | select(.outcome != "ok")
    | select((.finished_at // 0) >= ($now - $lb))
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
oldest=$(printf '%s' "$json" | jq -r '[.recent[]?.finished_at // empty] | min // empty')
returned=$(printf '%s' "$json" | jq -r '.recent | length')
if [ -n "$oldest" ] && [ "$returned" -ge "$depth" ] && [ "$oldest" -gt "$((now - lookback))" ]; then
    echo "WARN jobwatch: job log saturated — $returned entries reach back only" \
         "$(((now - oldest) / 3600))h but the lookback is $((lookback / 3600))h;" \
         "a failure older than that is INVISIBLE here (raise TENDER_JOB_LOG_DEPTH)"
    status=1
fi

wedged_line=$(printf '%s' "$json" | jq -r --argjson now "$now" --argjson w "$wedged" '
    (.current // empty)
    | select((.started_at // $now) <= ($now - $w))
    | "\(.kind) #\(.id) [\(.params)] running \($now - .started_at)s"')
if [ -n "$wedged_line" ]; then
    echo "WARN jobwatch: wedged job (threshold $((wedged / 3600))h): $wedged_line"
    status=1
fi

if [ "$status" -eq 0 ]; then
    summary=$(printf '%s' "$json" | jq -r --argjson now "$now" '
        (.recent | length) as $n
        | (.recent[0] // {}) as $last
        | (if .current then "running \(.current.kind) #\(.current.id) (\($now - .current.started_at)s)" else "idle" end) as $cur
        | ([.recent[]?.finished_at // empty] | min) as $oldest
        | "ok jobwatch: \($cur), \((.queued | length)) queued, \($n) recent covering \(if $oldest then (($now - $oldest) / 3600 | floor) else 0 end)h, last \($last.kind // "?") → \($last.outcome // "?")"')
    echo "$summary"
fi

exit "$status"
