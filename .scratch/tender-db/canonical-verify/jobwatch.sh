#!/usr/bin/env bash
# Durable job-failure watch for tender-db (task #32, extends task #23 monitoring).
#
# Built in tender-db-diskwatch.sh's idiom: read-only, journal-only, detection NOT
# notification, one greppable TENDERDB_JOBWATCH tag per line, and a distinct ERROR
# arm so a watch that could not run never renders as a quiet pass.
#
# WHY. Issue 27's projection-time guard refuses to commit a Tender head that is not
# the last version. When it fires, the job ROLLS BACK — the data is safe, which is
# the point — but the only evidence is a `job_log` row with outcome `error` that
# nobody reads. A guard whose success is silent and whose failure is also silent is
# indistinguishable from a guard that never fired. Same for any other failed daily:
# a wedged fetch, a failed process, a projection that died mid-fold.
#
# This is the ANSWER/PATH/COST family one step over: the guard protects the data
# (correct) and reports nothing (invisible). Rollback is not remediation — the work
# still did not happen, and the next run inherits the gap.
#
# WHAT IT DOES NOT DO. It does not page anyone; paging needs an external channel and
# that is Lennart's call. It writes to the journal, where `journalctl -t` or a grep
# for TENDERDB_JOBWATCH finds it. Detection is the deliverable.
#
# SOURCE OF TRUTH is the app's own `/admin/jobs`, not the DB file: the endpoint is
# the designed operator surface, it is served from memory + the durable job table,
# and reading it costs no data pages on the serving DB. Ops logic lives in the app.
#
# usage: tender-db-jobwatch.sh        (env: WINDOW_H, STUCK_H, ADMIN_SECRET_FILE, ENDPOINT)
set -uo pipefail

WINDOW_H="${WINDOW_H:-26}"        # a failure this recent is NEW; covers one daily cycle with slack
STUCK_H="${STUCK_H:-6}"           # a single job running longer than this is probably wedged
SECRET_FILE="${ADMIN_SECRET_FILE:-/root/tender-admin-secret}"
ENDPOINT="${ENDPOINT:-http://127.0.0.1:8080/admin/jobs}"

fail() { echo "TENDERDB_JOBWATCH ERROR $1"; exit 0; }   # exit 0: the journal line IS the report

command -v curl >/dev/null || fail "curl not on PATH; this watch did NOT run."
command -v jq   >/dev/null || fail "jq not on PATH; this watch did NOT run."
[ -r "$SECRET_FILE" ] || fail "$SECRET_FILE unreadable; cannot authenticate to $ENDPOINT; this watch did NOT run."

SECRET=$(grep -ho '^TENDER_ADMIN_SECRET=.*' "$SECRET_FILE" 2>/dev/null | head -1 | cut -d= -f2- | tr -d '"'"'"'')
[ -n "$SECRET" ] || fail "no TENDER_ADMIN_SECRET in $SECRET_FILE; this watch did NOT run."

BODY=$(curl -s -m 25 -H "X-Admin-Secret: $SECRET" "$ENDPOINT" 2>/dev/null)
[ -n "$BODY" ] || fail "$ENDPOINT returned nothing (service down, or admin surface disabled); this watch did NOT run."
echo "$BODY" | jq -e . >/dev/null 2>&1 || fail "$ENDPOINT returned non-JSON (auth rejected, or the surface answered 404); this watch did NOT run."
echo "$BODY" | jq -e 'has("recent")' >/dev/null 2>&1 || fail "$ENDPOINT JSON has no .recent array; the admin contract changed and this watch no longer understands it."

NOW=$(date +%s)
CUTOFF=$(( NOW - WINDOW_H * 3600 ))

# outcome != "ok" rather than == "error": a future third outcome value must be
# CAUGHT, not silently treated as success. The supervisor writes exactly "ok" on
# Ok(_) and "error" on Err(_) today (supervisor.rs:682).
recent_fail=$(echo "$BODY" | jq -r --argjson c "$CUTOFF" \
  '.recent[]? | select(.outcome != "ok") | select((.finished_at // 0) >= $c)
   | "id=\(.id) kind=\(.kind) params=\(.params) finished=\(.finished_at) detail=\(.counts)"')

older_fail=$(echo "$BODY" | jq -r --argjson c "$CUTOFF" \
  '[.recent[]? | select(.outcome != "ok") | select((.finished_at // 0) < $c)] | length')

scanned=$(echo "$BODY" | jq -r '.recent | length')

# A job stuck in `current` is a different fault from a job that failed: it never
# reaches the log at all, so a failure-only scan would never see it.
stuck=$(echo "$BODY" | jq -r --argjson n "$NOW" --argjson s "$STUCK_H" \
  '.current // empty | select((.started_at // $n) < ($n - $s*3600))
   | "id=\(.id) kind=\(.kind) started=\(.started_at)"')

warned=0
if [ -n "$recent_fail" ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    # CLASSIFY, don't just report. A transient projection failure and a #27
    # head-assertion rollback are byte-identical from outside — both "job failed,
    # error text in the row" — but they are opposite findings: one is a retry, the
    # other is real corruption caught and prevented. A warning that says only "a
    # job failed" arrives without saying what it means, so the operator has to go
    # read the row to learn whether it matters. The error text is carried either
    # way; the classification says which question to ask. (proj-fix, via team-lead.)
    case "$line" in
      *"head that is not the last version"*)
        echo "TENDERDB_JOBWATCH WARNING #27 HEAD-ASSERTION ROLLBACK within ${WINDOW_H}h: $line — this is NOT a transient. The projection-time guard found a Tender whose head is not its last version and refused to commit it, so the data is safe and the rollback worked. Investigate the DATA, not the job: a retry will hit the same violation." ;;
      *)
        echo "TENDERDB_JOBWATCH WARNING failed job within ${WINDOW_H}h: $line — the job rolled back, so the data is safe, but the work did not happen and the next run inherits the gap. Read the detail= text above to tell a transient from a real fault." ;;
    esac
    warned=1
  done <<< "$recent_fail"
fi

if [ -n "$stuck" ]; then
  echo "TENDERDB_JOBWATCH WARNING job running longer than ${STUCK_H}h: $stuck — probably wedged; a job that never finishes never reaches the job log, so no failure scan will ever see it."
  warned=1
fi

if [ "$warned" -eq 0 ]; then
  # The older-failure count keeps an unactioned failure from vanishing entirely once
  # it ages out of the window — the alternative is a detector that goes quiet on a
  # problem nobody fixed.
  if [ "${older_fail:-0}" -gt 0 ]; then
    echo "TENDERDB_JOBWATCH ok no failure within ${WINDOW_H}h (scanned=$scanned) — but ${older_fail} older failure(s) remain in the log, unactioned. See /admin/jobs."
  else
    echo "TENDERDB_JOBWATCH ok no failed jobs (scanned=$scanned)"
  fi
fi
exit 0
