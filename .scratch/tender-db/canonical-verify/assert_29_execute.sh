#!/usr/bin/env bash
# Post-execute assertion for issue 84 / #29 — run AFTER the marker fires.
#
#   scp assert_29_execute.sh root@zebreus.click:/tmp/ && \
#     ssh root@zebreus.click 'bash /tmp/assert_29_execute.sh'
#
# COPY IT, do not pipe it. This script contains a heredoc, and feeding a script
# with heredocs through `bash -s` puts the script text and the heredoc body on the
# same stdin. It happens to work, and "happens to work" is not what you want in
# the one command that checks a 593k-row write.
#
# INDEPENDENT BY CONSTRUCTION. A job's self-report is not verification: the run
# that did the writing is the last thing that should be trusted to say what it
# wrote. This reads the table directly, through the app's own turso path
# (`/v1/sql`) — the compliant route for bounded triage after the turso-only
# ruling. Not sqlite3, not a snapshot.
#
# THE EXPECTATIONS ARE FIXED IN WRITING SINCE ISSUE 138, before the re-spec
# existed and long before these numbers could be seen. They are not derived from
# the execute's output, which is the only thing that makes the check worth
# running at all.
#
# COST: one aggregate over a single `reason` value, seeking through the
# `quarantine_reason` index (issue 40 added it for exactly this shape). Bounded,
# read-only, one statement, 10s cap.
#
# NEVER RETRY A 408 (run-driver's preserved measurement): the cap bounds the
# WAIT, not the WORK — the query keeps running server-side, so a retry stacks a
# second copy of it rather than replacing the first.
set -uo pipefail

TOKEN_FILE=/root/tdb-diag-token
[ -r "$TOKEN_FILE" ] || { echo "FAIL: no token at $TOKEN_FILE" >&2; exit 2; }
T=$(cat "$TOKEN_FILE")   # never echoed, never in argv

# SCOPED TO WHAT THIS EXECUTE TOUCHED — corrected 2026-08-06 after the first run
# raised a false alarm on a clean execute. The original counted every row under the
# DTD label, so `reprocessed_at IS NOT NULL` swept up 26,948 rows RECLAIMED IN JULY
# by an unrelated operation, and reported them as "wrongly_reclaimed" — the arm
# whose whole job is to refute. It also compared a total (621,863 rows under the
# label) against 594,915, which was the OUTSTANDING figure from issue 137: a number
# derived from a filtered query, asserted against an unfiltered one.
#
# The refuting question is not "does any DTD row carry reprocessed_at" — legitimate
# history says yes. It is "does any row THIS RUN MARKED also carry reprocessed_at",
# and that is identified by its own marker, skipped_reason.
SQL="SELECT (SELECT COUNT(*) FROM quarantine WHERE skipped_reason = 'internal-ojs-non-english'), (SELECT COUNT(*) FROM quarantine WHERE skipped_reason = 'internal-ojs-non-english' AND reprocessed_at IS NOT NULL), (SELECT COUNT(*) FROM quarantine WHERE reason = 'unparsable-xml' AND detail LIKE 'XML with DTD detected%' AND skipped_at IS NULL AND reprocessed_at IS NULL), (SELECT COUNT(*) FROM quarantine WHERE reason = 'unparsable-xml' AND detail LIKE 'XML with DTD detected%')"

# The bearer header goes in on STDIN as a curl config, not as `-H` — an argv
# header is visible in `ps` to anything that can read /proc. Root-only on a
# single-tenant box, so this is habit rather than a live exposure, but a secret in
# argv is the kind of thing that stops being harmless when the script is copied.
resp=$(curl -sS --max-time 10 -o /tmp/a29.json -w '%{http_code}' \
  -X POST "http://127.0.0.1:8080/v1/sql" \
  -H "Content-Type: text/plain" \
  --data-binary "$SQL" \
  -K - <<EOF
header = "Authorization: Bearer $T"
EOF
) || { echo "FAIL: curl error (NOT retrying — a 408 means the work continues server-side)" >&2; exit 2; }

if [ "$resp" != "200" ]; then
  echo "FAIL: HTTP $resp — not retrying by design; see the 408 note in this file" >&2
  head -c 400 /tmp/a29.json >&2; echo; exit 2
fi

read -r marked marked_and_reclaimed outstanding dtd_total < <(
  jq -r '.rows[0] | "\(.[0]) \(.[1]) \(.[2]) \(.[3])"' /tmp/a29.json 2>/dev/null)
rm -f /tmp/a29.json

# An unparseable response must FAIL, never read as zeros — zeros here would
# silently satisfy `wrongly_reclaimed = 0`, which is the one arm that refutes.
case "${marked:-}${marked_and_reclaimed:-}${outstanding:-}${dtd_total:-}" in
  ''|*null*) echo "FAIL: could not parse a numeric result — refusing to treat an absent answer as a clean one" >&2; exit 2;;
esac

fail=0
check() { # check <label> <got> <want>
  if [ "$2" = "$3" ]; then printf '  PASS  %-22s %s\n' "$1" "$2"
  else printf '  FAIL  %-22s got %s, want %s\n' "$1" "$2" "$3"; fail=1; fi
}

echo "== issue 84/#29 post-execute assertion  $(date -u +%FT%TZ) =="
check "marked_by_this_run"  "$marked"              592856
check "marked_AND_reclaimed" "$marked_and_reclaimed" 0
check "still_outstanding"   "$outstanding"          2059
printf '  ctx   %-22s %s  (621,863 total under the label = 26,948 reclaimed in July + 594,915 that were outstanding)\n' "dtd_label_total" "$dtd_total"

echo
echo "still_outstanding 2,059 = 154 held-but-unextracted + 1,898 (2010-03) + 7 (language guard)"
if [ "$fail" -eq 0 ]; then
  echo "VERDICT ok — the execute wrote what it said, and wrote it to the right column."
else
  echo "VERDICT BROKEN — do NOT record #29 as resolved."
  echo "marked_AND_reclaimed != 0 is the severe case: this run wrote reprocessed_at,"
  echo "claiming ~593k notices entered the corpus that never did (store/src/lib.rs:158)."
fi
exit "$fail"
