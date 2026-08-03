#!/usr/bin/env bash
# ============================================================================
# hot_read_plans.sh — issue 112. Do the hot reads actually USE an index?
#
# READ-ONLY and METADATA-ONLY. Reads `sqlite_master` and compiles query plans.
# It never executes a data query, so it touches no data pages — unlike every other
# gate in this directory it is safe to run against the serving DB.
#
# ONE PRECISION, because the original claim here was "takes no lock" and that is
# no longer literally true. Section A opens the DB NON-immutable when a non-empty
# `-wal` exists (see section A for why: immutable=1 silently reads a stale
# catalogue). In WAL mode that registers a read mark in the `-shm`; it does NOT
# take the writer's lock and does NOT block writers.
# Measured rather than asserted, against a live writer committing continuously:
#   writer alone             p50 5.355 ms   347 commits / 2 s
#   writer + catalogue reads p50 5.732 ms   334 commits / 2 s   (-3.7%)
#   289 catalogue reads completed concurrently, 0 failures
# and that is with reads in a tight loop — one run of this gate does ONE such read,
# so the effect is not measurable. The read is a single `sqlite_master` scan, so it
# also cannot hold back a WAL checkpoint in any meaningful way (a long-lived reader
# could; this is milliseconds).
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
# "IF EQP CAN LIE, WHY DOES THIS GATE TRUST IT?"  — the asymmetry
#   Both things are true at once, because they are two different jobs for one
#   instrument:
#     * As a REGRESSION DETECTOR, EQP is sound. A plan that names a real index
#       and later names a rowid walk is a real, specific signal, and it is
#       available cheaply, standing, without executing anything. That is this
#       gate.
#     * As PROOF OF A SPEEDUP, EQP is not sound — the very text that misleads
#       (`SEARCH … USING INTEGER PRIMARY KEY (rowid=?)`) is the one that claims
#       to be fast while walking 13.2M rows. Proving a fix made something faster
#       needs a clock, not a plan.
#   So this gate is built on plans, AND a fix to a scanning read must still be
#   validated by timing it. Neither position contradicts the other; do not
#   "simplify" one into the other.
#
# THE FIVE RULES THIS FILE LEARNED THE HARD WAY (2026-08-03)
#   Each was paid for with a real false verdict. Detail lives at the point of use and
#   in issues 112/114; this is the index, so a future editor meets them before the code.
#
#   1. ASSERT THE ARTIFACT, NEVER A PARAPHRASE OF IT.
#      B1 was hand-written to "match" `read.rs`. Measured later: it returned SEARCH …
#      USING INDEX — green — with OR without the fix, because the paraphrase had
#      dropped the cursor predicate and the cursor predicate WAS the defect. It could
#      never have failed. Extract statements from the builder. (114 part 2)
#
#   2. A CHECK THAT HAS NOT BEEN SEEN TO FAIL IS NOT EVIDENCE.
#      Every check here has been run against a plan DB with its own index removed and
#      observed to go red — and to stay green when a DIFFERENT index is removed.
#      Sensitivity without specificity would only report "something changed".
#      (the can-fail matrix, above)
#
#   3. NEVER FAIL CORRECT CODE.
#      `SCAN … USING COVERING INDEX` is the RIGHT plan for `ORDER BY … LIMIT`; the
#      13.2M-row walk says SEARCH. `plan_*` indexes are absent by design between
#      projections. A gate that cries wolf about correct code is switched off within
#      a week, and then catches nothing at all.
#
#   4. A CHECK NEEDS AN ACHIEVABLE PASS STATE.
#      B7 is red today with a green available (a row-value cursor gives it a seek), so
#      it belongs here. The kind-only organizations read is slower — 99.08s — and does
#      NOT belong here yet, because no index on the table can serve it: red with no
#      reachable green is a permanent alarm, not a test, and could never tell "still
#      broken" from "broken in a new way". Severity does not decide this; achievability
#      does.
#
#   5. DERIVE WHAT IS CHECKED, NOT ONLY HOW.
#      The costliest miss of the day was not a wrong check but a MISSING one:
#      `read::organizations` walked 25.3M rows on a public endpoint (22.0s / 99.08s)
#      while B5, on the same table, asserted a read that issue 19 had deleted. The
#      target set was enumerated from memory, so it inherited the blind spots of
#      whoever wrote it. Every `Scope::Page` read in `read.rs` is a paginated hot read
#      by construction — the SET is derivable, and until it is derived this gate's
#      coverage is one person's recall. (114 part 2)
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
#   TDB_PLAN_BIN=…/turso-bench/target-0.7.0/release/plan   the on-box turso probe
#                             (pinned turso ="=0.7.0"), invoked `plan <db> eqp
#                             <sqlfile>`. NOTE the path is the built BINARY, not
#                             the crate directory `…/turso-bench/plan` — an earlier
#                             version of this header named the crate dir, which
#                             fails as "not executable".
#   TDB_PLAN_DB=/path/schema.db   the DB whose CATALOGUE the plans are compiled
#                             against. Setting both satisfies the plan-source
#                             contract — no build needed. TDB_PLAN_DB is ALSO the
#                             stats precondition's input, so it is required even
#                             with a custom TDB_PLAN_CMD.
#
#   THE PLAN DB MUST CARRY THE WHOLE CATALOGUE, and this is a real trap:
#   a partial scratch DB (e.g. one holding only `lots`/`tenders`/`tender_version_*`)
#   makes B2/B5 no-input and B3/B4/B6 FAIL — from a MISSING FIXTURE, not from a
#   defect. A gate whose red means "I was pointed at the wrong file" trains its
#   readers to discount reds. Use a schema-only clone of prod's catalogue.
#   Building one (run-driver, 2026-08-03): read prod's `sqlite_master` DDL with
#   `immutable=1` and replay it through TURSO's own exec mode. A sqlite3-built
#   clone is NOT readable by turso 0.7.0 — it demands its
#   `__turso_internal_autoincrement_*` shadow tables.
#
#   VERIFY THE CLONE BY: every index named in a B-check being present, and
#   `sqlite_stat1` carrying no rows for the tables under test. Both are stable.
#   Do NOT verify by index count against prod — an earlier version of this note
#   said to, and it is wrong: the count was 62 one hour and 61 the next, because
#   `plan_*` scratch indexes come and go with `clear_plan` at each end of a
#   projection. That is the same reason `plan_*` is excluded from section A, and
#   asserting on a number that correctly varies would have someone conclude their
#   plan DB was broken when it was fine.
#   TDB_PLAN_CMD='…'          alternative plan source: any command reading SQL on
#                             stdin and writing a turso-produced plan on stdout,
#                             at the DEPLOYED turso version.
#
# THE STATS PRECONDITION (not a caveat — enforced)
#   These schema-only plans are representative ONLY while `sqlite_stat1` carries
#   no rows for the tables under test. Run ANALYZE on prod and the planner may
#   choose differently, at which point a plan derived here silently stops
#   describing production. The gate checks TDB_PLAN_DB for stat rows and reports
#   no-input rather than a verdict if any exist, or if the stats state cannot be
#   established at all.
#
# THE RECORDED PRE-FIX ARTIFACT (run-driver-2) — AND A CORRECTION TO IT
#   B2 GREEN:  SEARCH tender_version_bid_parties USING INDEX
#                     tender_version_bid_parties_version
#   B1 RED:    SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
#   The walk was real. But this header used to imply the RED came from the B1
#   statement as committed, and MEASUREMENT SAYS IT DID NOT: run-driver planned
#   that old B1 text on turso 0.7.0 against two independent DBs and it comes back
#   `SEARCH l USING INDEX sqlite_autoindex_lots_1` — GREEN — with or without the
#   fix. The paraphrase had dropped the cursor predicate, and the cursor predicate
#   IS the defect, so that B1 could not have produced this RED and could never
#   have failed. The gate's own recorded falsifier did not falsify.
#   Left in the record rather than deleted: a gate that quietly rewrites its
#   history is worse than one with a wrong entry, and this is the cleanest example
#   in the file of why an artifact must be extracted rather than restated.
#   Section C now carries this weight properly — it establishes the RED in-run,
#   from a statement of known provenance, every time.
#
# EVERY CHECK IS DEMONSTRATED TO BE ABLE TO FAIL (run-driver, 2026-08-03)
#   A green from a check that CANNOT go red is worth nothing — B5 was reported PASS
#   in a canonical run while planning a statement the application does not issue.
#   So each check was run against a variant plan DB with exactly ONE index removed:
#
#     plan DB                  B1   B1b  B2   B3   B4   B6   C1
#     planschema.db (intact)   P    P    P    P    P    P    P(red)
#     nx_lots                  F    F    P    P    P    P    P
#     nx_bidp                  P    P    F    P    P    P    P
#     nx_pkey                  P    P    P    F    P    P    P
#     nx_isl                   P    P    P    P    F    P    P
#     nx_curpub                P    P    P    P    P    F    P
#
#   Sensitivity AND specificity: every diagonal fails, every off-diagonal passes. A
#   check that went red on ANY index removal would be as useless as one that never
#   went red — it would report "something changed", not "this read regressed".
#   C1 is unperturbed throughout, which is what a control should do.
#
#   Variants live at /data/scratch-lots/nx_{lots,bidp,pkey,isl,curpub}.db (~528 KB
#   each), built by the recipe above with one `sed` on the DDL. For B1/B1b the
#   removal is dropping `UNIQUE (tender_id, lot_key)` from the `lots` DDL, since
#   `sqlite_autoindex_lots_1` cannot be `DROP INDEX`ed.
#   RE-RUN THIS whenever turso is bumped — the version bump is this gate's stated
#   trigger, and a planner change can make a check unfalsifiable as easily as it can
#   make a read regress.
#
# THE POST-FIX RUN (run-driver, 2026-08-03, on-box turso 0.7.0, prod catalogue)
#   28 pass, 1 fail, 0 no-input — the single fail was the partial-index parse bug
#   below, since fixed.
#   B1  GREEN:  lots served by sqlite_autoindex_lots_1
#   B1b GREEN:  same, on an after=49377 cursor page
#   C1  RED:    SEARCH l USING INTEGER PRIMARY KEY (rowid=?)   <- control held
#   B1/B1b green WITH C1 red, same probe and same DB, is the result: the cursor
#   predicate is the only difference between them.
#
#   READ THIS BEFORE TREATING A POST-FIX B1 GREEN AS THE FLIP OF THAT RED.
#   The B1 statement was REPLACED after that run (see PROVENANCE below): the RED
#   was produced by a hand-written paraphrase, the GREEN by text extracted from
#   the deployed builder. Different probe, so red→green is NOT by itself a
#   controlled comparison — a green could mean "the fix works" or "the new
#   statement is one this planner happens to like". Section C exists to close
#   exactly that hole: it plans the PRE-FIX builder's own SQL (extracted the same
#   way, from 1830d50^) in the same run, on the same DB, through the same probe,
#   and requires it to still come back a rowid walk. Both extracted texts differ
#   in one place only — `l.id > ?` versus `(l.tender_id, l.id) > (?, ?)` — so with
#   C RED and B1 GREEN in one run, the cursor predicate is the only thing that
#   changed, and the fix is the only thing that can explain the difference.
#   If C ever goes GREEN, every B verdict in that run is void: the probe has
#   stopped discriminating (wrong DB, wrong engine, stats appeared), and a B1
#   green would be the fourth false-green in this issue's story, not the fix.
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
      grep -oE 'CREATE INDEX IF NOT EXISTS [a-z_0-9]+ +ON +[a-z_0-9]+ *\([^)]*\)( +WHERE [^";]*)?' |
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
  # ---- WHICH CATALOGUE AM I ACTUALLY READING? ----------------------------------
  # `immutable=1` is what makes this section safe against the SERVING DB: no lock,
  # no contention. But it also makes sqlite IGNORE the `-wal` sibling, and on prod
  # that WAL is gigabytes and live. So the main file alone is a STALE VIEW of the
  # catalogue, and the staleness is invisible: an index created recently enough to
  # live only in WAL frames reads as ABSENT ("the read it serves is scanning") on a
  # perfectly healthy DB, and an index DROPPED in WAL frames still reads as
  # present. Both directions wrong, both silent.
  #
  # This did not bite the 2026-08-03 run only by timing — job 534 built the
  # deferred indexes the day before, so they had reached the main file. Hours
  # later and the gate would have printed a wall of A-FAILs about a healthy DB.
  #
  # In WAL mode a read-only connection does NOT block the writer, so the merged
  # view is available at no cost to live traffic — the no-lock property that
  # motivated `immutable=1` is not actually needed to stay safe. Prefer the merged
  # read whenever a non-empty WAL exists; fall back to `immutable=1` only when
  # there are no frames to miss; and if the merged read cannot be had, say so
  # rather than quietly reporting a main-file view as the truth.
  A_VIEW=""
  if [ -s "${TDB_SNAPSHOT}-wal" ]; then
    HAVE=$(sqlite3 -readonly -noheader -separator "$(printf '\t')" \
             "file:${TDB_SNAPSHOT}?mode=ro" \
             "SELECT name, COALESCE(sql,'') FROM sqlite_master WHERE type='index'" 2>/dev/null)
    if [ -n "$HAVE" ]; then
      A_VIEW="WAL-merged ($(wc -c < "${TDB_SNAPSHOT}-wal") byte -wal included)"
    else
      report NONE A0 "TDB_SNAPSHOT has a NON-EMPTY -wal ($(wc -c < "${TDB_SNAPSHOT}-wal") bytes) and the WAL-merged read failed. An immutable=1 read would IGNORE those frames, so an index living only in the WAL would report as ABSENT on a healthy DB — and one dropped in the WAL would report as present. Index presence NOT verified. Checkpoint (TRUNCATE) and re-run, or point TDB_SNAPSHOT at a checkpointed snapshot."
      HAVE=""
    fi
  else
    HAVE=$(sqlite3 -readonly -noheader -separator "$(printf '\t')" \
             "file:${TDB_SNAPSHOT}?immutable=1" \
             "SELECT name, COALESCE(sql,'') FROM sqlite_master WHERE type='index'" 2>/dev/null)
    [ -n "$HAVE" ] && A_VIEW="main file, immutable=1 (no -wal frames to miss)"
  fi
  if [ -z "$HAVE" ]; then
    [ -s "${TDB_SNAPSHOT}-wal" ] || report NONE A0 "sqlite_master returned nothing — wrong file, or unreadable. NOT verified."
  else
    # State the provenance of the observation, not just its verdict. A reader who
    # cannot tell which catalogue was read cannot tell what a PASS is worth.
    echo "   catalogue read: $A_VIEW"

    while IFS=$'\t' read -r name cols; do
      [ -n "$name" ] || continue
      line=$(printf '%s\n' "$HAVE" | awk -F'\t' -v n="$name" '$1==n{print $2; exit}')
      if [ -z "$line" ] && ! printf '%s\n' "$HAVE" | cut -f1 | grep -qx "$name"; then
        report FAIL "A:$name" "DECLARED at $REV but ABSENT from the DB — the read it serves is scanning"
        continue
      fi
      # Compare declared columns against the stored DDL, whitespace-insensitively.
      #
      # The trailing `WHERE …` of a PARTIAL index is part of the definition, not
      # decoration: `notices(id) WHERE parse_state='parsed' AND projected=0` and
      # `notices(id) WHERE projected=1` are different indexes serving different
      # reads. The capture above therefore takes the predicate too. It used to
      # stop at the first `)`, which broke this comparison BOTH ways: the
      # expectation lost the predicate while the on-disk DDL kept it, so every
      # partial index failed with a bogus mismatch (`notices_unprojected` did) —
      # and, far worse, an index REBUILT WITH THE WRONG PREDICATE would have
      # compared equal and PASSED. A silent no-check of exactly the kind this
      # file exists to prevent, hiding behind a visible false alarm.
      #
      # Strip the `CREATE … ON ` prefix by consuming only text BEFORE the first
      # `(`. A greedy `.* ON ` would eat into the predicate the moment one
      # contains the letters " ON " (a column named `on_hold`, a nested table).
      want=$(printf '%s' "$cols" | tr -d ' ')
      got=$(printf '%s' "$line" | sed -E 's/^[^(]* ON +//I' | tr -d ' ')
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
# Each hot read is parameter-free so any engine can compile it.
#
# WHAT IS ASSERTED, AND WHY NOT "SEARCH NOT SCAN"
#   "SEARCH rather than SCAN" is ITSELF a correlate, and turso's plan text makes
#   it a dangerous one: for the `lots` full walk turso prints
#       SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
#   which READS like a point lookup and is in fact a forward walk of all 13.2M
#   rows. A naive SEARCH-vs-SCAN gate passes the very defect it was built for —
#   the third false-green in this story, after "the index exists" and "sqlite3
#   says SEARCH".
#
#   So the assertion is on the ACCESS PATH: the target table's plan line must
#   name a real INDEX. A rowid/INTEGER PRIMARY KEY access on the target is RED,
#   the same as a SCAN. Where a specific index is the point of the read, its name
#   is required too (the EXPECT column below) — `*` means "any real index", used
#   where more than one index would legitimately serve.
#
#   Note the rowid rule is scoped to the TARGET table only. In B1,
#   `SEARCH t USING INTEGER PRIMARY KEY (rowid=?)` for the joined `tenders` is a
#   correct point lookup on the primary key and must NOT be flagged; only the
#   `lots` access is under test.
#
# PROVENANCE OF THE SQL — B1/B1b ARE EXTRACTED, B2-B6 ARE STILL PARAPHRASES
#   B1 and B1b are the literal text the DEPLOYED `lots_of` builder emits at
#   rev 1830d50 (the row-value-cursor fix). They were not retyped. Extraction,
#   reproducible from this repo:
#     git worktree add /tmp/w 1830d50
#     # in /tmp/w: env-gate an eprintln of `self.sql`/`self.params` inside
#     # read.rs `Query::rows`, and add a test that calls `lots_of(&conn, 424242)`
#     TDB_DUMP_SQL=1 cargo test -p store --lib <that test> -- --nocapture
#   The ONLY edits applied to that output are (a) newline/indent collapse to one
#   line, because this table is newline-delimited, and (b) substituting each `?`
#   with the value the builder ITSELF bound in the same dump
#   ([424242, 424242, 0, 1000] — i.e. tender, tender, after=0, limit=MAX_PAGE),
#   because the probe compiles parameter-free statements. No token was rewritten.
#
#   B1b is the same extracted text with the cursor moved off the first page
#   (after=49377). It is not redundant: `lots_of` only ever asks after=0, but the
#   public `/v1/lots?tender=&after=N` runs the identical builder on a real cursor
#   page, and 1830d50 rejected an `after == 0`-only fix precisely because that
#   variant stayed at a measured 2237ms. A first-page-only assertion would certify
#   a half-fix as whole.
#
#   B2-B6 REMAIN HAND-WRITTEN APPROXIMATIONS of the shapes `read.rs` emits. They
#   can DRIFT: change the query and the string here keeps planning the OLD shape,
#   staying green while the read that actually runs regresses — the same
#   artifact-vs-proxy error as issues 110 and 102. Extending the extraction above
#   to cover them is issue 114's point 1; until then this comment is the only
#   thing standing between those five checks and a stale paraphrase.
# ---------------------------------------------------------------------------
# ---- reading one access line out of a plan, whatever shape the probe prints ----
#
# Plan text is NOT one format. run-driver-2's probe prints EQP's raw columns:
#     1 | 0 | 0 | SCAN tenders USING COVERING INDEX tenders_current_published
# while the tree rendering (and the recorded pre-fix artifact) prints:
#     |--SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
# The old test anchored SCAN to the START of the line, so against the column form
# it NEVER MATCHED: a real `SCAN` fell through to the index-name branch and was
# reported PASS. The gate's headline check — "must not scan" — was inoperative
# against the very probe this issue standardised on, and B6 passed only because
# its scan happens to be the correct plan. Right verdict, no working check.
#
# So: take the DETAIL as the text after the last `|`, then strip the tree glyphs.
detail_of() { printf '%s' "$1" | sed -E 's/.*\|//; s/^[[:space:]-]+//'; }

# ---- SCAN is not the discriminator; NAMING NO INDEX is -------------------------
# "SCAN = bad" is too crude in both directions, and each error is load-bearing:
#   * `SEARCH l USING INTEGER PRIMARY KEY (rowid=?)` is the 13.2M-row WALK that
#     motivated this issue, and it says SEARCH.
#   * `SCAN tenders USING COVERING INDEX tenders_current_published` (B6) is the
#     RIGHT plan for `ORDER BY current_published_at DESC, id DESC LIMIT 50` — an
#     ordered index traversal that stops after 50 rows. Failing it would make the
#     gate cry wolf about correct code, which is how gates get switched off.
# The real question is whether the access reaches rows THROUGH AN INDEX at all.
# A SCAN that names an index is still index-served — but it is a full ordered
# traversal, not a seek, so the report SAYS SO rather than blurring the two.
echo "-- B. hot reads must be served by a real index (turso plans only)"

# B7 IS EXPECTED TO BE **RED** UNTIL `read::organizations` IS FIXED — that is the
# point of it, not a bug in it.
#   `/v1/organizations?country=` was measured at 22.0s cold on prod (run-driver,
#   2026-08-03) and `?kind=` at 99.08s, unauthenticated and user-reachable. The cause
#   is the SAME defect this whole gate was built for: `read::organizations` carries a
#   plain `o.id > ?` cursor under `ORDER BY o.id`, so the planner drives from the
#   25.3M-row table in rowid order — exactly as `l.id > ?` did for `lots`.
#   This statement is EXTRACTED from the deployed builder at 1830d50, not written to
#   match it. A row-value cursor gives it `SEARCH o USING INDEX organizations_identity
#   (country=?)`, so the RED here has a reachable GREEN and will flip when the fix
#   lands.
#
#   THE KIND-ONLY VARIANT IS DELIBERATELY NOT A CHECK HERE, and the distinction is
#   not squeamishness about a red report:
#     `organizations_identity` is `(country, identifier_kind, identifier)`. It is the
#     ONLY index on the table. A filter on `identifier_kind` alone has no leading
#     `country`, so NO INDEX CAN SERVE IT — a row-value cursor does not help either.
#     A check demanding a plan that cannot exist has no achievable pass state; it is a
#     permanent alarm, not a test, and it would never distinguish "still broken" from
#     "broken in a new way". `?kind=` at 99.08s is a real, worse defect — it is
#     tracked in 112, and it gets a check once there is a schema or access-path answer
#     it could pass.
#   The difference is achievability, not severity: B7 is red with a green available,
#   the kind-only case is red with none.

# B5 WAS DELETED, NOT FIXED (2026-08-03) — and the reason generalises
#   B5 planned `SELECT id FROM organizations WHERE country = ? AND identifier_kind
#   = ? AND identifier = ?`, justified as "the Phase-1 mention resolver". Issue 19
#   DELETED that read: the per-mention probe was the O(n²) projection bottleneck and
#   was replaced by an in-memory `org_of` map. The deployed resolver issues, once,
#   at construction:
#       SELECT id, country, identifier_kind, identifier FROM organizations
#        WHERE identifier IS NOT NULL          -- an intentional ONE-TIME FULL SCAN
#   So B5 planned a statement the application never issues, and went GREEN in a
#   canonical run while protecting nothing.
#
#   It was DELETED rather than repaired because repairing it makes things worse:
#   pointed at the real resolver query it goes RED, over a full scan that is CORRECT
#   BY DESIGN — the same cry-wolf trap as failing `SCAN … USING COVERING INDEX`.
#   The index `organizations_identity` is still declared and still checked for
#   presence by section A; what is gone is the false claim that a hot indexed read
#   depends on it.
#
#   If `organizations` ever warrants a plan check again (read.rs `organizations()`
#   does filter by country/kind), it must be EXTRACTED from that builder. Writing
#   another statement by hand here is how B5 came to exist.

# Fields separated by `~` (NOT `|`, which appears inside the expected-index
# alternations below). id ~ table ~ alias ~ expected-index-regex ~ SQL
# The expected-index regex is matched against the index name turso reports; `*`
# means "any real index is acceptable", used where more than one would serve.
READS=$(cat <<'SQLS'
B1~lots~l~sqlite_autoindex_lots_1|lots_[a-z_]+~SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq, (SELECT s.value FROM tender_version_texts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'title' ORDER BY (s.lang = 'ENG') DESC LIMIT 1), (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id), (SELECT s.currency FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id ORDER BY s.cents DESC LIMIT 1), (SELECT s.utc_seconds FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.has_time FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1) FROM lots l JOIN tenders t ON t.id = l.tender_id JOIN tender_versions v ON v.tender_id = t.id AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id WHERE 1 = 1 AND l.tender_id = 424242 AND (l.tender_id, l.id) > (424242, 0) ORDER BY l.id LIMIT 1000
B1b~lots~l~sqlite_autoindex_lots_1|lots_[a-z_]+~SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq, (SELECT s.value FROM tender_version_texts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'title' ORDER BY (s.lang = 'ENG') DESC LIMIT 1), (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id), (SELECT s.currency FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id ORDER BY s.cents DESC LIMIT 1), (SELECT s.utc_seconds FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.has_time FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1) FROM lots l JOIN tenders t ON t.id = l.tender_id JOIN tender_versions v ON v.tender_id = t.id AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id WHERE 1 = 1 AND l.tender_id = 424242 AND (l.tender_id, l.id) > (424242, 49377) ORDER BY l.id LIMIT 1000
B2~tender_version_bid_parties~tender_version_bid_parties~tender_version_bid_parties_version~SELECT * FROM tender_version_bid_parties WHERE tender_id = 1 AND seq = 1
B3~tenders~tenders~tenders_procedure_key~SELECT id FROM tenders WHERE procedure_key = 'x'
B4~tenders~tenders~tenders_island~SELECT id FROM tenders WHERE source = 'ted' AND island_notice_id = 1
B6~tenders~tenders~tenders_current_published~SELECT id FROM tenders ORDER BY current_published_at DESC, id DESC LIMIT 50
B7~organizations~o~organizations_identity|organizations_[a-z_]+~SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional, (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id) FROM organizations o WHERE 1 = 1 AND o.country = 'ZZ' AND o.id > 0 ORDER BY o.id LIMIT 1000
SQLS
)

# Adapter for run-driver-2's on-box turso probe, which takes a FILE rather than
# stdin: `plan <db> eqp <sqlfile>`, built against a pinned turso ="=0.7.0". Set
# TDB_PLAN_BIN + TDB_PLAN_DB and the stdin contract is satisfied for you.
#   TDB_PLAN_BIN=/opt/tender-db/turso-bench/plan TDB_PLAN_DB=/path/snapshot.db
probe_plan() {
  local f rc
  f=$(mktemp) || return 1
  cat > "$f"
  "$TDB_PLAN_BIN" "$TDB_PLAN_DB" eqp "$f"
  rc=$?
  rm -f "$f"
  return $rc
}
if [ -z "${TDB_PLAN_CMD:-}" ] && [ -n "${TDB_PLAN_BIN:-}" ] && [ -n "${TDB_PLAN_DB:-}" ]; then
  if [ -x "$TDB_PLAN_BIN" ]; then
    TDB_PLAN_CMD='probe_plan'
  else
    report NONE B0 "TDB_PLAN_BIN=$TDB_PLAN_BIN is not executable — no plan source."
  fi
fi

# ---- PRECONDITION: the plan DB's STATS state must be known and match prod ----
# A schema-only EQP is representative ONLY because `sqlite_stat1` carries no rows
# for these tables: with stats present the planner can choose differently, so a
# plan derived from a stats-carrying DB (or from a stats-free one when prod has
# stats) silently stops describing production. This is a precondition, not a
# caveat in a comment — if it cannot be established, section B does not run.
PLAN_STATS_OK=no
if [ -n "${TDB_PLAN_CMD:-}" ]; then
  if [ -z "${TDB_PLAN_DB:-}" ]; then
    report NONE B0 "a plan source is set but the DB it reads is not identified (TDB_PLAN_DB) — the ANALYZE/sqlite_stat1 state of the planning input cannot be established, so no plan verdict is trustworthy. Plans NOT verified."
  elif [ ! -r "$TDB_PLAN_DB" ] || ! command -v sqlite3 >/dev/null 2>&1; then
    report NONE B0 "cannot inspect TDB_PLAN_DB=$TDB_PLAN_DB for sqlite_stat1 (unreadable, or sqlite3 absent) — stats state unknown. Plans NOT verified."
  else
    has_stat=$(sqlite3 -readonly -noheader "file:${TDB_PLAN_DB}?immutable=1" \
      "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sqlite_stat1'" 2>/dev/null)
    if [ -z "$has_stat" ]; then
      report NONE B0 "could not read sqlite_master on TDB_PLAN_DB — stats state unknown. Plans NOT verified."
    elif [ "$has_stat" = 0 ]; then
      PLAN_STATS_OK=yes
    else
      n=$(sqlite3 -readonly -noheader "file:${TDB_PLAN_DB}?immutable=1" \
        "SELECT COUNT(*) FROM sqlite_stat1 WHERE tbl IN ('lots','tenders','organizations','tender_version_bid_parties')" 2>/dev/null)
      if [ "${n:-0}" = 0 ]; then
        PLAN_STATS_OK=yes
      else
        report NONE B0 "TDB_PLAN_DB carries ANALYZE stats ($n sqlite_stat1 rows for the tables under test). The plans below would be derived under different statistics than the schema-only assumption this gate was validated against. Re-derive against a DB whose stats state matches prod's before trusting any plan verdict. Plans NOT verified."
      fi
    fi
  fi
fi

if [ -z "${TDB_PLAN_CMD:-}" ]; then
  report NONE B0 "no plan source. Set TDB_PLAN_BIN+TDB_PLAN_DB (the on-box turso probe) or TDB_PLAN_CMD. NOT falling back to sqlite3: measured, stock sqlite3 reports SEARCH ... USING COVERING INDEX for the B1 shape that turso walks, so a sqlite3 verdict here would be GREEN over the live defect. Plans NOT verified."
elif [ "$PLAN_STATS_OK" != yes ]; then
  : # the precondition already reported why
else
  while IFS='~' read -r id tbl alias want sql; do
    [ -n "$id" ] || continue
    if ! plan=$(printf '%s\n' "$sql" | eval "$TDB_PLAN_CMD" 2>/dev/null) || [ -z "$plan" ]; then
      report NONE "$id" "plan source produced nothing for $tbl — NOT verified (is TDB_PLAN_CMD right?)"
      continue
    fi
    # Isolate the TARGET table's access line. Anchoring on "(SCAN|SEARCH) <alias|table>"
    # keeps a single-letter alias from matching stray text, and keeps the rowid
    # rule below scoped to the table under test — a rowid lookup on a JOINED
    # table (B1's `t`) is correct and must not be flagged.
    line=$(printf '%s\n' "$plan" | grep -iE "(SCAN|SEARCH)[[:space:]]+(${alias}|${tbl})([[:space:]]|\$)" | head -1)
    if [ -z "$line" ]; then
      report NONE "$id" "no access line for $tbl in the plan — cannot tell how it is read: $(printf '%s' "$plan" | tr '\n' ' ' | cut -c1-160)"
      continue
    fi
    det=$(detail_of "$line")
    idx=$(printf '%s' "$det" | sed -nE 's/.*USING (COVERING )?INDEX ([a-z_0-9]+).*/\2/Ip')
    mode=$(printf '%s' "$det" | grep -oiE '^(SCAN|SEARCH)' | tr '[:lower:]' '[:upper:]')
    if printf '%s' "$det" | grep -qiE 'INTEGER PRIMARY KEY|rowid='; then
      # The trap this gate exists to survive: turso prints a full forward walk of
      # `lots` as "SEARCH l USING INTEGER PRIMARY KEY (rowid=?)", which reads like
      # a point lookup. On the target table that is a walk, not an index seek.
      report FAIL "$id" "$tbl is read by ROWID, not by an index — this is a full walk that PRINTS like a seek: $line"
    elif [ -z "$idx" ] && [ "$mode" = SCAN ]; then
      report FAIL "$id" "$tbl is SCANNED and the plan names NO index — every row is visited: $line"
    elif [ -z "$idx" ]; then
      report NONE "$id" "$tbl access names no index and is not a recognisable scan — read it by hand: $line"
    elif [ "$want" != '*' ] && ! printf '%s' "$idx" | grep -qE "^(${want})\$"; then
      report FAIL "$id" "$tbl is served by index '$idx', not the expected ${want} — a different index can still be the wrong access path for this read: $line"
    elif [ "$mode" = SCAN ]; then
      report PASS "$id" "$tbl read by an ordered FULL TRAVERSAL of index $idx (SCAN, not a seek) — correct only while the read's ORDER BY matches that index and a LIMIT stops it early: $line"
    else
      report PASS "$id" "$tbl served by index $idx (seek)"
    fi
  done <<< "$READS"
fi

# ---------------------------------------------------------------------------
# C. THE NEGATIVE CONTROL — can this probe still produce a RED at all?
#
# A gate that has never been observed to fail on the run that matters is not
# evidence. Section B's greens are only meaningful if the same probe, same DB,
# same engine, in the SAME RUN, still calls the KNOWN-BAD shape bad.
#
# The statement below is the PRE-FIX builder's own SQL, extracted from 1830d50^
# by the identical method used for B1 (see PROVENANCE). It differs from B1 in
# exactly one place: `l.id > 0` where B1 has `(l.tender_id, l.id) > (424242, 0)`.
# Nothing else. So:
#   C RED + B1 GREEN  -> the cursor predicate is the only variable, and the fix
#                        is the only available explanation. This is the result.
#   C GREEN           -> the probe no longer discriminates. Section B's greens
#                        say nothing, and this run is NOT a pass — it is a
#                        broken instrument reporting good news, which is the
#                        single most expensive outcome in this issue's history.
# Note this is NOT a copy of B1 with a predicate edited by hand: both texts came
# out of the builder at their respective revisions. Do not "tidy" one into the
# other.
#
# AND DO NOT DELETE THIS SECTION WHEN THE APP STOPS ISSUING THIS QUERY.
#   Rule 5 above, and the B5 deletion, both say "a check for a read nobody issues
#   asserts nothing". That reasoning does NOT apply here, and it is easy to get
#   backwards because both sit in this same file.
#     B5 was an ASSERTION — it claimed a hot read was index-served, so its value
#     depended on the app issuing that read; once it didn't, it asserted nothing.
#     THIS is a DELIBERATE KNOWN-BAD, whose only job is to show that the probe can
#     still tell a walk from a seek IN THIS RUN. That needs the shape to still WALK.
#     It does not need anyone to run it in production.
#   When the read it mirrors changes (issue 115 moves the driving table from `lots`
#   to `tender_version_lots`), RE-BASE this control onto the new shape's known-bad
#   predecessor. Retire it only if the old shape stops compiling or stops walking —
#   i.e. only if it has stopped being a known-bad, which is the one thing that would
#   actually invalidate it.
#   The real hazard with a stale control is MISLABELLING, not invalidity: someone
#   reading `C1 PASS` as "the lots_of read is fine" rather than "the probe
#   discriminates". Keep the report wording that explicit through any re-base.
# ---------------------------------------------------------------------------
CONTROL=unknown
echo "-- C. negative control: the pre-fix shape must still come back RED"
if [ -z "${TDB_PLAN_CMD:-}" ] || [ "$PLAN_STATS_OK" != yes ]; then
  report NONE C1 "no usable plan source — the control did not run, so section B's results are UNCONTROLLED and must not be read as a confirmed fix."
else
  # Quoted heredoc, NOT a '…' assignment: this SQL contains its own single
  # quotes ('title', 'ENG', 'submission_deadline'). In a single-quoted string
  # those close and reopen it, so `'title'` silently degrades to the bare
  # identifier `title` — a different statement, planned without comment.
  CONTROL_SQL=$(cat <<'CTLSQL'
SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq, (SELECT s.value FROM tender_version_texts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'title' ORDER BY (s.lang = 'ENG') DESC LIMIT 1), (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id), (SELECT s.currency FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id ORDER BY s.cents DESC LIMIT 1), (SELECT s.utc_seconds FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.has_time FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1) FROM lots l JOIN tenders t ON t.id = l.tender_id JOIN tender_versions v ON v.tender_id = t.id AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id WHERE 1 = 1 AND l.tender_id = 424242 AND l.id > 0 ORDER BY l.id LIMIT 1000
CTLSQL
)
  if ! cplan=$(printf '%s\n' "$CONTROL_SQL" | eval "$TDB_PLAN_CMD" 2>/dev/null) || [ -z "$cplan" ]; then
    report NONE C1 "the control statement produced no plan — the probe cannot be shown to discriminate, so section B is UNCONTROLLED."
  else
    cline=$(printf '%s\n' "$cplan" | grep -iE "(SCAN|SEARCH)[[:space:]]+(l|lots)([[:space:]]|\$)" | head -1)
    cdet=$(detail_of "$cline")
    # Judged by the SAME parser and the SAME rule as section B. A control scored
    # by different logic could agree with B for the wrong reason and would not be
    # a control at all: "still walks" here must mean exactly what "walks" means
    # there — reaches rows without going through an index.
    if [ -z "$cline" ]; then
      report NONE C1 "no access line for lots in the control plan — cannot establish that the probe discriminates: $(printf '%s' "$cplan" | tr '\n' ' ' | cut -c1-160)"
    elif printf '%s' "$cdet" | grep -qiE 'INTEGER PRIMARY KEY|rowid=' ||
         ! printf '%s' "$cdet" | grep -qiE 'USING (COVERING )?INDEX'; then
      CONTROL=discriminates
      report PASS C1 "control still RED (pre-fix shape walks lots) — the probe discriminates, so B1's verdict is meaningful: $cline"
    else
      CONTROL=void
      report FAIL C1 "CONTROL WENT GREEN. The pre-fix shape — the one measured at ~2.2s on prod — now plans as an index seek through this probe: $cline. The probe is not discriminating (wrong DB, wrong engine version, or ANALYZE stats appeared), so EVERY section B verdict in this run is void. Do not report B1 as a confirmed fix."
    fi
  fi
fi

echo
echo "== $PASS pass, $FAIL fail, $NOINPUT no-input =="
if [ "$CONTROL" = void ]; then
  echo "VOID — the negative control passed, so this probe cannot be shown to tell a"
  echo "walk from a seek. Any GREEN above is uninterpretable, not good news. Fix the"
  echo "probe (wrong DB? wrong turso version? ANALYZE stats?) and re-run before"
  echo "reporting anything about the hot reads."
  exit 1
fi
if [ "$FAIL" -ne 0 ]; then
  echo "FAIL — a hot read is scanning, or a declared index is missing/stale."
  exit 1
fi
if [ "$NOINPUT" -ne 0 ]; then
  echo "INCOMPLETE — $NOINPUT check(s) could not be evaluated. This is NOT a pass:"
  echo "the gate could not see its own inputs. Supply TDB_SNAPSHOT / TDB_PLAN_CMD / TDB_REV."
  exit 2
fi
echo "PASS — every declared index is present with its declared columns, every hot"
echo "read compiles to an index-served plan on the deployed engine, and the"
echo "negative control still fails, so the probe was shown to discriminate."
exit 0
