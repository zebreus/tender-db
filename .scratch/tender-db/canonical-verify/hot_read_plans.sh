#!/usr/bin/env bash
# ============================================================================
# hot_read_plans.sh — issue 112. Do the hot reads actually USE an index?
#
# READ-ONLY and METADATA-ONLY. Reads `sqlite_master` and compiles query plans.
# It never executes a data query, so it touches no data pages and cannot compete
# with live traffic — unlike every other gate in this directory it is safe to run
# against the serving DB.
#
# WHY THIS EXISTS
#   Every other gate here counts rows. The layer can be perfectly correct while a
#   hot read full-scans, because a full scan returns the right answer — that is
#   the /v1/tenders/{id} ~2.2s regression, invisible to all of them.
#
# THE TRAP THIS GATE IS BUILT TO AVOID
#   The obvious implementation asserts that the expected indexes EXIST. That is a
#   CORRELATE, and on 2026-08-03 it was a correlate that was already TRUE while
#   the site served 2.2s pages: `tender_version_bid_parties_version` was present
#   (job 534 built it) and `lots_of` scanned 13.2M `lots` rows anyway, because
#   turso declines `UNIQUE(tender_id, lot_key)` under an `ORDER BY l.id LIMIT`
#   shape. The index existed and the planner scanned. So the load-bearing signal
#   is the PLAN, not the name.
#
# THE SECOND TRAP — WHICH ENGINE PRODUCED THE PLAN  (read this before "fixing"
# this script to just use sqlite3)
#   Stock SQLite and turso DISAGREE on exactly this query. Measured on the real
#   `lots` schema, with and without ANALYZE, stock sqlite3 says:
#       SEARCH l USING COVERING INDEX sqlite_autoindex_lots_1 (tender_id=?)
#   i.e. SEARCH, not SCAN — GREEN — for the very query turso scans in prod.
#   Running these plan checks under stock sqlite3 would therefore CERTIFY THE
#   LIVE DEFECT AS HEALTHY. A snapshot is the right DATA but sqlite3 is the wrong
#   ENGINE, and a plan verdict from the wrong engine is not evidence — it is the
#   same mirror as issue 110, one level deeper.
#   Hence: the plan half REQUIRES a turso-backed plan source ($TDB_PLAN_CMD) and
#   reports NO-INPUT without one. It never falls back to sqlite3 for a plan.
#
# THREE STATES, NEVER TWO
#   pass     the read is served by an index / the declared index is present
#   fail     the plan SCANs, or a declared index is missing or has wrong columns
#   no-input the expectation or the observation could not be established at all
#            (unknown build, no plan engine, unreachable DB) — LOUD, never a pass
#
# INPUTS
#   TDB_SNAPSHOT=/path/db     read sqlite_master with stock sqlite3 (metadata only)
#   BASE_URL=http://…:8080    used ONLY to ask the service which build it is
#   TDB_REV=<sha>             override the build to compare against (else asked)
#   TDB_PLAN_CMD='…'          REQUIRED for plan verdicts. A command that reads SQL
#                             on stdin and writes a turso-produced query plan on
#                             stdout, at the DEPLOYED turso version. Example: a
#                             small bin linked against the workspace turso dep,
#                             run over a snapshot, or an app diagnostic endpoint.
# ============================================================================
set -uo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
PASS=0; FAIL=0; NOINPUT=0
report() {
  case "$1" in
    PASS) PASS=$((PASS+1));       printf '  \033[32mPASS\033[0m %-6s %s\n' "$2" "$3";;
    FAIL) FAIL=$((FAIL+1));       printf '  \033[31mFAIL\033[0m %-6s %s\n' "$2" "$3";;
    NONE) NOINPUT=$((NOINPUT+1)); printf '  \033[33mNO-IN\033[0m %-6s %s\n' "$2" "$3";;
  esac
}

echo "== hot-read plan gate (issue 112)  $(date -u +%FT%TZ) =="

# ---------------------------------------------------------------------------
# 0. ESTABLISH THE BUILD — the expectation must be DERIVED from the build that
#    is actually serving, never hand-copied here. A literal list of index names
#    in this file would go green the day an eleventh entry lands in the const,
#    which is the incident that motivated this gate reproduced inside the gate.
#    The rev is ASKED FOR, not assumed (issue 107): the service reports the sha
#    it was built from, and if it cannot, that is no-input.
# ---------------------------------------------------------------------------
REV="${TDB_REV:-}"
if [ -z "$REV" ]; then
  if command -v jq >/dev/null 2>&1 &&
     DASH=$(curl -sS --max-time 20 "$BASE_URL/api/dashboard" 2>/dev/null); then
    REV=$(printf '%s' "$DASH" | jq -r '.system.service_rev // ""' 2>/dev/null)
  fi
fi
case "$REV" in
  ""|dev|null)
    report NONE X0 "cannot establish which build is serving (service_rev='${REV:-unset}'). Set TDB_REV=<sha>. Everything below would be an assumption, so nothing is asserted."
    echo; echo "== $PASS pass, $FAIL fail, $NOINPUT no-input =="
    exit 2;;
esac
if ! git cat-file -e "${REV}^{commit}" 2>/dev/null; then
  report NONE X0 "service reports build '$REV' but this repo has no such commit — cannot derive the expected index set from it."
  echo; echo "== $PASS pass, $FAIL fail, $NOINPUT no-input =="
  exit 2
fi
SRC=$(mktemp); trap 'rm -f "$SRC"' EXIT
if ! git show "${REV}:crates/store/src/canonical.rs" > "$SRC" 2>/dev/null; then
  report NONE X0 "cannot read canonical.rs at $REV — no expectation available."
  echo; echo "== $PASS pass, $FAIL fail, $NOINPUT no-input =="
  exit 2
fi
echo "-- expectation derived from build $REV ($(wc -l < "$SRC") lines of canonical.rs)"

# Every index the DEPLOYED code declares: the DEFERRED_TENDER_INDEXES entries
# plus every `CREATE INDEX IF NOT EXISTS`. Union, so a new index of either kind
# is picked up with no edit here.
#
# `plan_*` indexes are EXCLUDED deliberately: the grouping-plan scratch tables
# are dropped and recreated at both ends of a projection (clear_plan), so their
# indexes are absent by design between runs. Asserting them would produce a
# failure that is correct behaviour — the fastest way to make a gate ignored.
TAB=$(printf '\t')
EXPECT=$(
  {
    # No gawk extensions here on purpose: the box's /usr/bin/awk may be mawk,
    # where a 3-arg match() is a syntax error. That would yield an EMPTY
    # expectation and a section that "passes" having checked nothing — the exact
    # silent-no-check failure this gate exists to catch. sed+grep only, and the
    # emptiness guard below is the backstop.
    sed -n '/const DEFERRED_TENDER_INDEXES/,/^[[:space:]]*\];/p' "$SRC" |
      grep -oE '\("[a-z_0-9]+",[[:space:]]*"[^"]+"\)' |
      sed -E "s/\\(\"([a-z_0-9]+)\",[[:space:]]*\"([^\"]+)\"\\)/\\1${TAB}\\2/"
    tr '\n' ' ' < "$SRC" |
      grep -oE 'CREATE INDEX IF NOT EXISTS [a-z_0-9]+ +ON +[a-z_0-9]+\([^)]*\)' |
      sed -E "s/CREATE INDEX IF NOT EXISTS ([a-z_0-9]+) +ON +/\\1${TAB}/"
  } | grep -v "${TAB}plan_" | sort -u
)
EXPECT_N=$(printf '%s\n' "$EXPECT" | grep -c . )
# An empty or implausibly small expectation means the PARSE broke (a refactor of
# the const, a different awk/sed, a moved file) — not that the code declares no
# indexes. Reporting "0 missing" from a failed parse is the mirror this gate is
# built to avoid, so it is no-input, loudly.
if [ "$EXPECT_N" -lt 5 ]; then
  report NONE X1 "derived only $EXPECT_N indexes from canonical.rs at $REV — the parse almost certainly broke (expected ~20). Refusing to report a verdict from an expectation this thin."
  echo; echo "== $PASS pass, $FAIL fail, $NOINPUT no-input =="
  exit 2
fi
echo "   $EXPECT_N declared indexes to account for (plan_* scratch excluded)"

# ---------------------------------------------------------------------------
# A. PRESENCE + COLUMNS — the necessary half. Observation is sqlite_master on the
#    real DB; metadata only, so stock sqlite3 IS a valid engine for this half
#    (unlike the plan half — reading a catalogue is not planning a query).
# ---------------------------------------------------------------------------
echo "-- A. declared indexes present on the DB, with the declared columns"
if [ -z "${TDB_SNAPSHOT:-}" ]; then
  report NONE A0 "no TDB_SNAPSHOT — cannot read sqlite_master. (/v1/sql is not a fallback: it rejects EXPLAIN and is view-scoped.) Index presence NOT verified."
elif [ ! -r "$TDB_SNAPSHOT" ]; then
  report NONE A0 "TDB_SNAPSHOT=$TDB_SNAPSHOT is not readable — index presence NOT verified."
elif ! command -v sqlite3 >/dev/null 2>&1; then
  report NONE A0 "sqlite3 not found (nix shell nixpkgs#sqlite) — index presence NOT verified."
else
  HAVE=$(sqlite3 -readonly -noheader -separator "$(printf '\t')" \
           "file:${TDB_SNAPSHOT}?immutable=1" \
           "SELECT name, COALESCE(sql,'') FROM sqlite_master WHERE type='index'" 2>/dev/null)
  if [ -z "$HAVE" ]; then
    report NONE A0 "sqlite_master returned nothing — wrong file, or unreadable. NOT verified."
  else
    while IFS=$'\t' read -r name cols; do
      [ -n "$name" ] || continue
      line=$(printf '%s\n' "$HAVE" | awk -F'\t' -v n="$name" '$1==n{print $2; exit}')
      if [ -z "$line" ] && ! printf '%s\n' "$HAVE" | cut -f1 | grep -qx "$name"; then
        report FAIL "A:$name" "DECLARED at $REV but ABSENT from the DB — the read it serves is scanning"
        continue
      fi
      # Compare declared columns against the stored DDL, whitespace-insensitively.
      want=$(printf '%s' "$cols" | tr -d ' ')
      got=$(printf '%s' "$line" | sed -E 's/.* ON //I' | tr -d ' ')
      if [ -z "$line" ]; then
        report PASS "A:$name" "present (implicit/auto index — no DDL to compare)"
      elif [ "$want" = "$got" ]; then
        report PASS "A:$name" "present, columns match $cols"
      else
        report FAIL "A:$name" "present but columns DIFFER — declared $want, on-disk $got (a same-name index built from a stale definition serves the read no better than nothing)"
      fi
    done <<< "$EXPECT"
  fi
fi

# ---------------------------------------------------------------------------
# B. PLANS — the load-bearing half.
#
# Each hot read below is written parameter-free so any engine can compile it.
# The assertion is deliberately "does not SCAN <table>" rather than "uses index
# <name>": the `lots` access is served by an IMPLICIT index
# (sqlite_autoindex_lots_1, from the UNIQUE constraint), so asserting a name
# would be wrong for exactly the read that motivated this gate.
# ---------------------------------------------------------------------------
echo "-- B. hot reads must be served by an index (turso plans only)"

# id | table that must not be scanned | SQL
READS=$(cat <<'SQLS'
B1|lots|SELECT l.id, l.tender_id, l.lot_key FROM lots l JOIN tenders t ON t.id = l.tender_id WHERE l.tender_id = 1 ORDER BY l.id LIMIT 1000
B2|tender_version_bid_parties|SELECT * FROM tender_version_bid_parties WHERE tender_id = 1 AND seq = 1
B3|tenders|SELECT id FROM tenders WHERE procedure_key = 'x'
B4|tenders|SELECT id FROM tenders WHERE source = 'ted' AND island_notice_id = 1
B5|organizations|SELECT id FROM organizations WHERE country = 'DE' AND identifier_kind = 'national' AND identifier = 'x'
B6|tenders|SELECT id FROM tenders ORDER BY current_published_at DESC, id DESC LIMIT 50
SQLS
)

if [ -z "${TDB_PLAN_CMD:-}" ]; then
  report NONE B0 "no TDB_PLAN_CMD — no turso plan source. NOT falling back to sqlite3: measured, stock sqlite3 reports SEARCH ... USING COVERING INDEX for the B1 shape that turso SCANs, so a sqlite3 verdict here would be GREEN over the live defect. Plans NOT verified."
else
  while IFS='|' read -r id tbl sql; do
    [ -n "$id" ] || continue
    if ! plan=$(printf '%s\n' "$sql" | eval "$TDB_PLAN_CMD" 2>/dev/null) || [ -z "$plan" ]; then
      report NONE "$id" "plan source produced nothing for $tbl — NOT verified (is TDB_PLAN_CMD right?)"
      continue
    fi
    # A scan of the target table is the failure. Match SCAN against the table
    # name with or without an alias, case-insensitively.
    if printf '%s\n' "$plan" | grep -qiE "SCAN[[:space:]]+([a-z_0-9]+[[:space:]]+)?\b${tbl}\b|SCAN[[:space:]]+${tbl}\b"; then
      report FAIL "$id" "$tbl is SCANNED — $(printf '%s' "$plan" | tr '\n' ' ' | cut -c1-160)"
    elif printf '%s\n' "$plan" | grep -qiE "SEARCH|USING (COVERING )?INDEX"; then
      report PASS "$id" "$tbl served by an index — $(printf '%s' "$plan" | tr '\n' ' ' | cut -c1-120)"
    else
      report NONE "$id" "plan for $tbl is neither a recognisable SCAN nor SEARCH — read it by hand: $(printf '%s' "$plan" | tr '\n' ' ' | cut -c1-160)"
    fi
  done <<< "$READS"
fi

echo
echo "== $PASS pass, $FAIL fail, $NOINPUT no-input =="
if [ "$FAIL" -ne 0 ]; then
  echo "FAIL — a hot read is scanning, or a declared index is missing/stale."
  exit 1
fi
if [ "$NOINPUT" -ne 0 ]; then
  echo "INCOMPLETE — $NOINPUT check(s) could not be evaluated. This is NOT a pass:"
  echo "the gate could not see its own inputs. Supply TDB_SNAPSHOT / TDB_PLAN_CMD / TDB_REV."
  exit 2
fi
echo "PASS — every declared index is present with its declared columns, and every"
echo "hot read compiles to an index-served plan on the deployed engine."
exit 0
