#!/usr/bin/env bash
# The 1,905 remainder — run the four attribution queries through POST /v1/sql.
#
#   TDB_TOKEN=tdb_… [BASE_URL=https://tenders.zebreus.click] ./dtd_remainder_1905.sh
#
# The SQL, the hypothesis and the READING KEY live in dtd_remainder_1905.sql and
# are NOT repeated here — one copy, so the queries and their pre-registered
# meanings cannot drift apart. Read that file before reading these numbers.
#
# Authorized by team-lead 2026-08-05: a bounded single-table aggregate over
# `quarantine` against the LIVE database, off-peak. Read-only by construction —
# /v1/sql accepts a single bare SELECT and runs it on a `query_only` connection.
#
# Q0 RUNS FIRST ON PURPOSE. The endpoint caps at 10s and returns 408, so a slow
# query yields NOTHING rather than a slow answer. Q0 is the cheap COUNT that
# doubles as a cost probe: if it 408s, the approach needs a different bed and we
# know that before spending the expensive attribution on the same wall.
set -uo pipefail

BASE_URL="${BASE_URL:-https://tenders.zebreus.click}"
: "${TDB_TOKEN:?set TDB_TOKEN to a tdb_… API token}"

HERE="$(cd "$(dirname "$0")" && pwd)"

# Print the response verbatim on success; on failure print the server's message
# and the HTTP status. A 408 must be visibly a TIMEOUT and not an empty result —
# "no rows" and "we never got to look" are the two answers that must never wear
# the same face in this project.
q() {
  local label=$1 sql=$2 body status
  body=$(curl -sS --max-time 60 -w '\n%{http_code}' -X POST "$BASE_URL/v1/sql" \
    -H "Authorization: Bearer $TDB_TOKEN" -H "Content-Type: text/plain" \
    --data-binary "$sql") || { echo "[$label] curl failed" >&2; return 1; }
  status=$(printf '%s' "$body" | tail -n1)
  body=$(printf '%s' "$body" | sed '$d')
  if [ "$status" != "200" ]; then
    echo "[$label] HTTP $status — $(printf '%s' "$body" | jq -r '.error // .' 2>/dev/null | head -2)" >&2
    [ "$status" = "408" ] && echo "[$label] TIMED OUT at the 10s cap — this is 'we never got to look', NOT 'no rows'." >&2
    return 1
  fi
  echo "--- $label ---"
  printf '%s' "$body" | jq -r '(.columns // empty) | @tsv' 2>/dev/null
  printf '%s' "$body" | jq -r '.rows[] | @tsv'
  printf '%s' "$body" | jq -e '.truncated == true' >/dev/null 2>&1 \
    && echo "[$label] WARNING: result TRUNCATED by the row/byte cap — the tail is missing." >&2
  return 0
}

# Extract one statement from the .sql file by its `-- Qn —` banner. Keeping the
# SQL in the committed file rather than inlining it here means the thing that
# ran and the thing that was reviewed are the same text.
stmt() {
  awk -v want="$1" '
    /^-- Q[0-9] —/ { on = ($2 == want) ; next }
    on && !/^--/   { print }
  ' "$HERE/dtd_remainder_1905.sql"
}

rc=0
q Q0 "$(stmt Q0)" || { echo "Q0 failed — stopping before the expensive queries." >&2; exit 1; }
q Q1 "$(stmt Q1)" || rc=1
q Q2 "$(stmt Q2)" || rc=1
q Q3 "$(stmt Q3)" || rc=1

echo
echo "Read these against the READING KEY in dtd_remainder_1905.sql."
echo "The load-bearing check: Q1's in_scope bucket MUST be 593,010."
echo "If it is not, the dry-run and this attribution measure different sets,"
echo "and the remainder arithmetic is void — that is the finding, not a detail."
exit $rc
