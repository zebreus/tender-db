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
# WHAT THIS GATE CANNOT DETECT, AT ALL, EVER (rule 6, and the boundary of the file)
#   A plan says which ACCESS PATH was chosen. It does not say how many rows that path
#   touches, and the cost of a read is rows x work-per-row. So an entire class of
#   regression is invisible here, and it is not a gap to be closed by a better check —
#   the information is not in the input.
#
#   Measured, twice, in one day:
#     * issue 115 — the tender-scoped lots read planned ENTIRELY index-served, every
#       line green, while taking 248.8s. Seven correlated subqueries per lot; the plan
#       shows that each is a seek and not how many run.
#     * issue 117 — the proposed organizations fix turns
#           SEARCH o USING INTEGER PRIMARY KEY (rowid=?)     (0.0003s dense)
#       into
#           SEARCH o USING INDEX organizations_identity      (1.1989s dense)
#           USE SORTER FOR ORDER BY
#       i.e. a walk becomes a seek — the transition this gate REWARDS — while getting
#       4,209x slower, because the seek's order is not the ORDER BY and everything
#       matching must be sorted before LIMIT. Which path is faster depends on
#       SELECTIVITY, which no plan reports.
#
#   Consequences, all of them load-bearing:
#     * A green here means "the access path is X". It NEVER means "this read is
#       healthy". Every PASS says so in its own text; do not paraphrase it away.
#     * A plan change in the IMPROVING direction (walk -> seek) is exactly when a
#       sorter can appear. That is a moment to reach for a CLOCK, not to record a win.
#     * Do not add a check whose pass condition is "the plan got better". This gate is
#       sound as a REGRESSION DETECTOR for an access path and unsound as proof of a
#       speedup — the asymmetry documented below. B7 was added in violation of it and
#       withdrawn; the note at section B is the worked example.
#     * Cost-per-row and rows-touched belong to a timing instrument (issue 115's
#       before/after: a scaling RATIO across a size spread, with a control that must
#       NOT improve). Nothing in this file substitutes for it.
#
# THREE STATES, NEVER TWO
#   pass     the read is served by an index / the declared index is present
#   fail     the plan SCANs, or a declared index is missing or has wrong columns
#   no-input the expectation or the observation could not be established at all
#            (unknown build, no plan engine, unreachable DB) — LOUD, never a pass
#   (n/a is not a fourth verdict: it says a row does not describe the BUILD that is
#    serving — see `applies-when`. It is printed and counted, never silent, because
#    a check that quietly disappears looks exactly like one that never existed.)
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
#
# THE 115 RE-BASELINE — B1 NOW ASSERTS THE DRIVING TABLE (sdk-vendor, 2026-08-03)
#   Issue 115 moved the tender-scoped lots read off `lots` and onto
#   `tender_version_lots` (the containment shape). B1 was re-based onto it, and the
#   re-base is NOT a string swap:
#
#     old B1: target `lots` / `l`,                 expect sqlite_autoindex_lots_1
#     new B1: target `tender_version_lots` / `vl`, expect its PK autoindex,
#             AND `drives-before = l|lots`
#
#   The `drives-before` half is the load-bearing one. Measured, both plans below are
#   from the SAME probe and the SAME DB:
#
#     post-115 statement   SEARCH vl USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id=?)   <- line 3
#                          SEARCH l  USING INTEGER PRIMARY KEY (rowid=?)                                <- line 4
#     pre-115 statement    SEARCH l  USING INTEGER PRIMARY KEY (rowid=?)                                <- line 1
#                          SEARCH vl USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id=?)   <- line 3
#
#   `vl` is index-served in BOTH. An index-name check alone would call the 13.2M-row
#   walk GREEN. Only the join order separates them, which is why the row asserts it.
#
#   FALSIFIED IN BOTH DIRECTIONS, locally, before the box run:
#     1. JOIN-ORDER SENSITIVITY. Feed the new B1 row the PRE-115 statement:
#          FAIL B1 "JOIN ORDER INVERTED: 'l|lots' is read FIRST (plan line 1),
#                   tender_version_lots only at line 3"
#        while B2/B3/B4/B6 stayed green — sensitivity WITH specificity.
#     2. INDEX SENSITIVITY. Plan against a DB whose `tender_version_lots` DDL has no
#        `PRIMARY KEY (tender_id, seq, lot_id)`:
#          FAIL B1 "is SCANNED and the plan names NO index"
#        again with B2/B3/B4/B6 green.
#     3. HEALTHY: post-115 statement on the intact catalogue -> PASS, "drives the
#        join ahead of 'l|lots' (line 3 before 4)".
#
#   REPRODUCING IT WITHOUT THE BOX (this is also 114 part 1's dry run)
#   The engine was the WORKSPACE's pinned turso — `turso ="=0.7.0"`, the same version
#   the on-box probe pins — driven by a `#[cfg(test)] local_eqp_probe` in the store
#   crate that plans statements from a file. Catalogue: a DB from `Db::open` (the real
#   schema) plus the 10 `DEFERRED_TENDER_INDEXES` created from canonical.rs by the same
#   derivation section A uses. Two traps worth recording, both cost time:
#     * a mutant catalogue must be built by executing DDL THROUGH TURSO. A DB built by
#       sqlite3 does not open here at all ("internal sequence backing table … is empty"),
#       so the falsifier silently becomes a no-input rather than a red.
#     * `.schema` output carries `sqlite_sequence` and `__turso_internal_*` objects that
#       turso refuses to create by name. Filter them or the DDL aborts halfway and you
#       are planning against a HALF-BUILT catalogue — which fails checks for a reason
#       that has nothing to do with the read.
#   This is a local cross-check, not a replacement for the box: the box run is what
#   plans against the deployed engine and the prod catalogue. Two sources that agree
#   are the point (114 part 1); if they ever disagree, the box wins and the disagreement
#   is itself the finding.
#
# SECTION C NEEDED NO RE-BASE, AND THAT WAS CHECKED RATHER THAN ASSUMED
#   The plan was to re-base the negative control onto the new shape's predecessor when
#   115 landed. Measured instead: the pre-115 `lots_of` statement STILL COMPILES and
#   STILL WALKS on the post-115 catalogue —
#       SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
#   which is exactly and only what the control's retirement rule asks of it ("retire it
#   only if the old shape stops compiling or stops walking"). A control's job is to be a
#   known-bad this probe can still call bad; it does not need to be a query anyone runs.
#   So it stays as it is. Re-basing it "to match 115" would have coupled the control to
#   the thing it is supposed to be independent of.
# ============================================================================
set -uo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
PASS=0; FAIL=0; NOINPUT=0; NA=0
report() {
  case "$1" in
    PASS) PASS=$((PASS+1));       printf '  \033[32mPASS\033[0m %-6s %s\n' "$2" "$3";;
    FAIL) FAIL=$((FAIL+1));       printf '  \033[31mFAIL\033[0m %-6s %s\n' "$2" "$3";;
    NONE) NOINPUT=$((NOINPUT+1)); printf '  \033[33mNO-IN\033[0m %-6s %s\n' "$2" "$3";;
    # n/a is NOT a fourth verdict — it says the row does not describe the build
    # that is serving (see `applies-when`). It is printed and counted rather than
    # skipped, because a check that silently vanishes cannot be told apart from one
    # that was never written, and that is how B5 survived.
    NA)   NA=$((NA+1));           printf '  \033[2mn/a  \033[0m %-6s %s\n' "$2" "$3";;
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
#   There are TWO B1/B1b pairs, one per side of issue 115's deploy, selected by the
#   `applies-when` guard. Both are extracted; neither is retyped.
#
#   THE PRE-115 PAIR (`-2751ce3`) is the literal text the `lots_of` builder emits at
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
#   THE POST-115 PAIR (`+2751ce3`) IS EXTRACTED THE SAME WAY, ONE SEAM BETTER.
#   `2ea1b23` added `#[cfg(test)] read::lots_statement(filter, scope)`, which returns
#   the SQL and params of the very `Query` that `lots()` runs — so the extraction is
#   now a supported seam rather than a temporary patch in a throwaway worktree:
#     git worktree add --detach /tmp/w 2ea1b23
#     # in /tmp/w: a #[test] that calls
#     #   read::lots_statement(&Filter { tender: Some(424_242), ..default() },
#     #                        Scope::Page { after, limit: 1000 })
#     # for after in [0, 49_377], inlining each `?` with the value it returned
#     cargo test -p store --lib <that test> -- --nocapture
#   Same two edits as above and no others: whitespace collapsed to one line, and each
#   `?` replaced by the value THE BUILDER ITSELF bound ([424242, 424242, 0|49377, 1000]).
#   Note the post-115 statement is shorter because 115 moved the six per-lot summary
#   subqueries out of the row query into `summarise` — that is the fix, not a trim.
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

# B7 / B8 / B9 ARE EXPECTED **RED** TODAY. THAT IS THE POINT OF THEM.
#   These three are the 117 class: a plain `<pk> > ?` cursor under `ORDER BY <pk>`, so
#   the planner drives from the table in rowid order and filters per row — the same
#   defect as `lots_of`, measured live at 22.0s (`/v1/organizations?country=`) and
#   99.08s (`?kind=`), unauthenticated and user-reachable.
#
#   THE PROPERTY IS `cursor-bound`, NOT AN INDEX NAME, and that is what makes them
#   sound where the first attempt at B7 was not. An earlier B7 asserted merely
#   "index-served"; proj-fix then measured the proposed row-value fix at 4,209x SLOWER
#   (151,648x at prod scale) while plumping the plan from a walk to a seek — so that B7
#   would have flipped GREEN over the regression. `cursor-bound` demands the seek carry
#   `id>?`, which is red for the rowid walk AND red for the row-value shape, because the
#   row-value form loses the `id` bound and re-enters the partition from its start.
#   `expected-index` is `*` deliberately: naming the future index would couple this gate
#   to whatever proj-fix calls it, and the property is what matters, not the name.
#
#   NO `applies-when` GUARD, deliberately. Guarding them on the fix's commit would mean
#   somebody must add them in the window between that commit and its deploy, or the
#   class ships unchecked — the same "nobody remembered" failure that produced B5, just
#   relocated. Unguarded they are red today (truthfully), and they go GREEN BY
#   THEMSELVES the moment an index giving both the seek and the id ordering is the
#   serving rev. Nobody has to act at any particular minute.
#
#   So a red run is expected until 117 lands. If that red is unwelcome, the remedy is
#   the fix; silencing the check is how the 22.0s read survived unnoticed in the first
#   place.

# B7 WAS ADDED, THEN SUSPENDED (2026-08-03). ITS EXPECTATION WAS UNSOUND.
#   Kept as the worked example, because the mistake is subtle and mine.
#   `/v1/organizations?country=` was measured at 22.0s cold on prod (run-driver,
#   2026-08-03) and `?kind=` at 99.08s, unauthenticated and user-reachable. The cause
#   is the SAME defect this whole gate was built for: `read::organizations` carries a
#   plain `o.id > ?` cursor under `ORDER BY o.id`, so the planner drives from the
#   25.3M-row table in rowid order — exactly as `l.id > ?` did for `lots`.
#   I added B7 asserting that this read must be INDEX-SERVED rather than a rowid walk.
#   proj-fix then measured the proposed row-value fix (issue 117): on a dense filter it
#   is **4,209x SLOWER** than the walk B7 calls broken —
#       plain cursor (today)   0.0003s   SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
#       row value  (proposed)  1.1989s   SEARCH o USING INDEX organizations_identity
#                                        USE SORTER FOR ORDER BY
#   **B7 would have flipped RED -> GREEN over that.** A gate built to catch read
#   regressions would have certified a three-order-of-magnitude one.
#
#   The reason is not a bug in B7's wiring; the EXPECTATION is unsound.
#   `organizations_identity` is `(country, identifier_kind, identifier)`, so a seek on
#   `country` yields rows ordered by `identifier_kind`, NOT by `o.id`. The query asks
#   `ORDER BY o.id`, so every matching row must be materialised and sorted before
#   `LIMIT` can apply. Which access path is FASTER therefore depends on SELECTIVITY:
#       ?country=ZZ (sparse)  walk = 25.3M rows for nothing (22.0s) | seek = instant
#       ?country=DE (dense)   walk = LIMIT stops early (fast)       | seek+sort all DE
#   Neither plan is right for all inputs, and the thing that decides — how many rows
#   match — IS NOT IN THE PLAN TEXT. A plan check cannot express this read's health,
#   so it must not pretend to. Suspended rather than left to go green on the fix.
#
#   It becomes assertable again the moment there is an index that gives BOTH the seek
#   and the `o.id` ordering — `organizations(country, id)`, the same shape as
#   `tenders_current_published(current_published_at, id)`. Then the plan is a seek with
#   NO top-level sorter, `LIMIT` truncates early, and both densities are fast. At that
#   point B7 returns with the new index NAMED, and `organizations_identity` explicitly
#   NOT accepted — because accepting it is precisely the false green above.
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

# Fields separated by `~` (NOT `|`, which appears inside the alternations below).
#   id ~ table ~ alias ~ expected-index-regex ~ drives-before ~ cursor-bound ~ applies-when ~ SQL
# The expected-index regex is matched against the index name turso reports; `*`
# means "any real index is acceptable", used where more than one would serve.
#
# `applies-when` — WHY A ROW CAN BE BUILD-CONDITIONAL, AND WHY THAT IS NOT A HEDGE
#   Section A derives its expectation from the DEPLOYED build's canonical.rs. This
#   table did not: it was a flat literal, so it silently asserted ONE build's shape
#   whatever was serving. That is fine until a read's correct plan CHANGES, and then
#   it is the same drift 114 is about, pointed the other way — the gate reporting RED
#   against a deployment that is healthy for its own rev, or GREEN because it is still
#   planning last week's statement.
#
#   Issue 115 is exactly that event: it moved the tender-scoped lots read from driving
#   off `lots` to driving off `tender_version_lots`, so the CORRECT plan before and
#   after 115 are different plans. A re-baseline that just overwrote B1 would have been
#   wrong for whichever rev was not serving, with no way for the gate to say so.
#
#   So a row may name a commit it depends on:
#       +<sha>  the row applies only if the serving build CONTAINS that commit
#       -<sha>  the row applies only if it does NOT
#       (empty) the row always applies
#   Evaluated with `git merge-base --is-ancestor` against the rev section 0 already
#   established from the service itself. A sha this repo does not know is NO-INPUT for
#   that row, never a skip. Rows that do not apply are PRINTED as n/a with the reason —
#   a check that quietly vanishes is indistinguishable from one that was never written,
#   and that is precisely how B5 survived.
#
#   Note what is and is not restated here. A commit sha is a FACT ABOUT HISTORY and
#   cannot drift; the SQL beside it is still extracted from that build's own builder.
#   The end state remains 114 part 2 — a fixture generated FROM the deployed rev, so
#   even the pairing is derived — and this is the honest interim, not a substitute.
#
# `cursor-bound` (usually empty) names the CURSOR COLUMN, and asserts that the target's
# index seek carries it as a BOUND — `(country=? AND id>?)`, not `(country=?)`. It is
# issue 117's rule ("the cursor column must participate in the index being sought"), and
# it exists because "names a real index" turned out to be a correlate one more time:
#
#     SEARCH o USING INDEX organizations_country_id (country=? AND id>?)   <- O(page)
#     SEARCH o USING INDEX organizations_identity  (country=?)             <- O(partition)
#
# Both name a real index; both pass every other check in this file. In the second the
# seek does not carry the cursor, so rows do not arrive in `id` order, the plan needs a
# top-level `USE SORTER FOR ORDER BY`, and LIMIT cannot stop early — every row matching
# the filter is visited on every request, whatever page was asked for. Measured, both
# shapes on the same catalogue: adding `(filter, id)` removes the sorter and the plain
# cursor then seeks on both columns; a row-value cursor without that index seeks the
# filter only and keeps the sorter.
# Which matters because the two errors point opposite ways: for a filter matching
# NOTHING the sorter version is still a huge win (`?country=ZZ`, 22.0s of walking), and
# for a DENSE filter it is a loss — `?country=DE` is 19 ms TODAY precisely because the
# current walk stops as soon as it has LIMIT matches. A gate that cannot tell the two
# fixes apart would certify the one that regresses the fast path.
#
# `drives-before` (usually empty) is an alias|table alternation that the target must
# be read BEFORE. It exists because of a trap proj-fix measured on the 115 shape:
#
#     SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
#
# appears in BOTH the pre-115 and post-115 plans, meaning OPPOSITE things.
#   pre-115 : `l` is the OUTER loop      -> a 13.2M-row walk        (quadratic)
#   post-115: `l` is joined from `vl` on `l.id = vl.lot_id`
#                                        -> a genuine one-row lookup (linear)
# Identical string, opposite verdicts; the ONLY difference is position in the join
# order. So `!contains("INTEGER PRIMARY KEY")` rejects the correct shape and
# `!contains("SCAN")` accepts the broken one — the rowid rule below is right for a
# table that DRIVES and wrong for one that is driven. What actually separates linear
# from quadratic is which table is the outer loop, and that is what this field
# asserts. (Same discriminator proj-fix's store-crate tests now use.)
READS=$(cat <<'SQLS'
B1~lots~l~sqlite_autoindex_lots_1|lots_[a-z_]+~~~-2751ce3~SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq, (SELECT s.value FROM tender_version_texts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'title' ORDER BY (s.lang = 'ENG') DESC LIMIT 1), (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id), (SELECT s.currency FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id ORDER BY s.cents DESC LIMIT 1), (SELECT s.utc_seconds FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.has_time FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1) FROM lots l JOIN tenders t ON t.id = l.tender_id JOIN tender_versions v ON v.tender_id = t.id AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id WHERE 1 = 1 AND l.tender_id = 424242 AND (l.tender_id, l.id) > (424242, 0) ORDER BY l.id LIMIT 1000
B1b~lots~l~sqlite_autoindex_lots_1|lots_[a-z_]+~~~-2751ce3~SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq, (SELECT s.value FROM tender_version_texts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'title' ORDER BY (s.lang = 'ENG') DESC LIMIT 1), (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id), (SELECT s.currency FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id ORDER BY s.cents DESC LIMIT 1), (SELECT s.utc_seconds FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.has_time FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1) FROM lots l JOIN tenders t ON t.id = l.tender_id JOIN tender_versions v ON v.tender_id = t.id AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id WHERE 1 = 1 AND l.tender_id = 424242 AND (l.tender_id, l.id) > (424242, 49377) ORDER BY l.id LIMIT 1000
B1~tender_version_lots~vl~sqlite_autoindex_tender_version_lots_1|tender_version_lots_[a-z_]+~l|lots~~+2751ce3~SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq FROM tender_version_lots vl JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id JOIN tenders t ON t.id = vl.tender_id JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq WHERE vl.tender_id = 424242 AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = 424242) AND l.id > 0 ORDER BY l.id LIMIT 1000
B1b~tender_version_lots~vl~sqlite_autoindex_tender_version_lots_1|tender_version_lots_[a-z_]+~l|lots~~+2751ce3~SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq FROM tender_version_lots vl JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id JOIN tenders t ON t.id = vl.tender_id JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq WHERE vl.tender_id = 424242 AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = 424242) AND l.id > 49377 ORDER BY l.id LIMIT 1000
B2~tender_version_bid_parties~tender_version_bid_parties~tender_version_bid_parties_version~~~~SELECT * FROM tender_version_bid_parties WHERE tender_id = 1 AND seq = 1
B3~tenders~tenders~tenders_procedure_key~~~~SELECT id FROM tenders WHERE procedure_key = 'x'
B4~tenders~tenders~tenders_island~~~~SELECT id FROM tenders WHERE source = 'ted' AND island_notice_id = 1
B6~tenders~tenders~tenders_current_published~~~~SELECT id FROM tenders ORDER BY current_published_at DESC, id DESC LIMIT 50
B7~organizations~o~*~~id~~SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional, (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id) FROM organizations o WHERE 1 = 1 AND o.country = 'ZZ' AND o.id > 0 ORDER BY o.id LIMIT 1000
B8~notices~notices~*~~id~~SELECT id, source, publication_id, content_hash, profile, declared_version, member_path, ingested_at, parse_state, published_at, dispatched_at FROM notices WHERE 1 = 1 AND source = 'ted' AND id > 0 ORDER BY id LIMIT 1000
B9~tenders~t~*~~id~~SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at, v.publication_id, v.notice_subtype, (SELECT s.value FROM tender_version_texts s WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title' ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC LIMIT 1), (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = v.seq), (SELECT s.currency FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 ORDER BY s.cents DESC LIMIT 1), (SELECT s.utc_seconds FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT s.has_time FROM tender_version_dates s WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1), (SELECT COUNT(*) FROM tender_version_lots l WHERE l.tender_id = t.id AND l.seq = v.seq), v.dispatched_at, (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'), (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts') FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) WHERE 1 = 1 AND t.source = 'ted' AND t.id > 0 ORDER BY t.id LIMIT 1000
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
  while IFS='~' read -r id tbl alias want drives bound when sql; do
    [ -n "$id" ] || continue
    # ---- BUILD GUARD (see `applies-when` above) --------------------------------
    # The correct plan for a read can CHANGE with a deploy (issue 115 moved the
    # tender-scoped lots read onto a different driving table). A row may therefore
    # name the commit it depends on, and it is evaluated against the rev section 0
    # established FROM THE SERVICE — not against this checkout, which is routinely
    # ahead of what is deployed.
    if [ -n "$when" ]; then
      sha=${when#[+-]}
      if ! git cat-file -e "${sha}^{commit}" 2>/dev/null; then
        report NONE "$id" "row is conditioned on commit $sha, which this repo does not contain — whether it describes build $REV cannot be established, so it is NOT verified (rather than assumed to apply)."
        continue
      fi
      if git merge-base --is-ancestor "$sha" "$REV" 2>/dev/null; then has=yes; else has=no; fi
      case "$when" in
        +*) [ "$has" = yes ] || { report NA "$id" "build $REV does NOT contain $sha — this row states the post-$sha shape, so it does not describe what is serving."; continue; };;
        -*) [ "$has" = no  ] || { report NA "$id" "build $REV contains $sha — this row states the pre-$sha shape, so it does not describe what is serving."; continue; };;
      esac
    fi
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
    # A TOP-LEVEL sorter after an index seek is the shape that hides a regression:
    # the seek reorders rows away from the ORDER BY, so everything matching must be
    # materialised and sorted before LIMIT applies. Whether that is cheap depends on
    # how many rows match — which is NOT in the plan. Reported, never a verdict:
    # issue 115's fixed lots read also ends in a sorter and is correct, because its
    # slices are ~2 rows. Failing on a sorter would fail correct code (rule 3).
    sorter=""
    if printf '%s\n' "$plan" | grep -qiE '(^|\|)[[:space:]]*[0-9]+[[:space:]]*\|[[:space:]]*0[[:space:]]*\|.*USE SORTER FOR ORDER BY|^[|`-]*[[:space:]]*USE SORTER FOR ORDER BY'; then
      sorter="  [!] plan also has a TOP-LEVEL SORTER after the seek — cost depends on how many rows match, which no plan shows. VERIFY BY TIMING before reading this pass as an improvement."
    fi
    # Position of the target's access line, and of the table it must precede.
    pos_t=$(printf '%s\n' "$plan" | grep -niE "(SCAN|SEARCH)[[:space:]]+(${alias}|${tbl})([[:space:]]|\$)" | head -1 | cut -d: -f1)
    pos_d=""
    [ -n "$drives" ] && pos_d=$(printf '%s\n' "$plan" | grep -niE "(SCAN|SEARCH)[[:space:]]+(${drives})([[:space:]]|\$)" | head -1 | cut -d: -f1)
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
    elif [ -n "$bound" ] && ! printf '%s' "$det" | grep -qE "(^|[^a-z_])${bound}[[:space:]]*>"; then
      # ---- THE CURSOR COLUMN MUST BE A BOUND OF THE SEEK, NOT JUST THE FILTER ----
      # Issue 117's rule, and the ONE thing that separates its two candidate fixes.
      # Both of these name a real index and both would pass every other check here:
      #   SEARCH o USING INDEX organizations_country_id (country=? AND id>?)   O(page)
      #   SEARCH o USING INDEX organizations_identity  (country=?)             O(partition)
      # In the second the seek does not carry the cursor, so the rows do not arrive in
      # `id` order, the plan needs `USE SORTER FOR ORDER BY`, and LIMIT CANNOT TRUNCATE
      # EARLY — every row of the filter partition is visited on every request, whatever
      # page was asked for. That is an enormous win over walking the whole table when the
      # partition is empty (`?country=ZZ`, 22.0s) and a LOSS when it is large: the dense
      # filters are the ones that are fast today (`?country=DE`, 19ms) precisely because
      # the current walk stops at LIMIT matches.
      # So "names an index" is a CORRELATE again, one level in from where this gate
      # started, and without this field the weaker fix reports green.
      report FAIL "$id" "$tbl is served by index $idx but the seek does NOT bound the cursor column '$bound' — so rows do not arrive in cursor order, LIMIT cannot stop early, and every row matching the filter is visited on every request (O(partition), not O(page)): $line"
    elif [ -n "$drives" ] && [ -z "$pos_d" ]; then
      report NONE "$id" "$tbl is served by index $idx, but no access line for '$drives' was found, so the JOIN ORDER could not be established — and join order is what separates a linear read from a quadratic one here. NOT verified."
    elif [ -n "$drives" ] && [ "${pos_t:-0}" -ge "${pos_d:-0}" ]; then
      report FAIL "$id" "JOIN ORDER INVERTED: '$drives' is read FIRST (plan line $pos_d), $tbl only at line $pos_t — $tbl must be the outer loop and drive the join. The index on $tbl is fine; the DRIVING TABLE is wrong, which is the difference between one lookup per row and a full walk: $line"
    elif [ "$mode" = SCAN ]; then
      report PASS "$id" "ACCESS PATH ONLY: $tbl read by an ordered FULL TRAVERSAL of index $idx (SCAN, not a seek) — correct only while the read's ORDER BY matches that index and a LIMIT stops it early. Says nothing about rows scanned, rows sorted or latency.$sorter"
    elif [ -n "$drives" ]; then
      report PASS "$id" "ACCESS PATH ONLY: $tbl served by index $idx (seek), driving the join ahead of '$drives' (line $pos_t before $pos_d). Says nothing about rows scanned, rows sorted or latency.$sorter"
    else
      report PASS "$id" "ACCESS PATH ONLY: $tbl served by index $idx (seek). Says nothing about rows scanned, rows sorted or latency.$sorter"
    fi
  done <<< "$READS"
fi

# ---------------------------------------------------------------------------
# E. IS THE CHECKED SET STILL THE WHOLE SET?  (issue 114 part 2)
#
# Every check above answers "is this read served properly". None of them answers
# "are these the right reads to be checking" — and on 2026-08-03 that was the more
# expensive question. The READS table was hand-written from the reads someone
# remembered being hot, so it carried B5, a check for a read issue 19 had DELETED,
# while NOTHING checked `read::organizations`, which was walking 25.3M rows on a
# public endpoint at 22.0s. Not a wrong check: a missing one. Coverage chosen by
# recall is the artifact-vs-paraphrase error applied to WHICH reads get checked.
#
# So the set is DERIVED: `checked-set.tsv` is emitted by a generator that walks every
# Collection `read_items` dispatches to, crossed with every `Filter` field, through the
# statement seams — so a new collection, a new filter or a changed builder all change
# it with no edit here. `checked-set-triage.tsv` gives each one a disposition. A key in
# the set with no disposition is a FAILURE, loudly, because that is exactly the state
# `read::organizations` was in for months.
#
# THE FIXTURE IS CHECKED IN, WHICH MEANS IT CAN GO STALE, so staleness is tested
# rather than hoped for: the generator records the rev it ran at, and this section
# compares `read.rs` between that rev and the SERVING rev. Any difference and the
# audit is no-input — a fixture describing a build that is not running is exactly the
# paraphrase problem it was built to solve.
# ---------------------------------------------------------------------------
echo "-- E. the checked SET is derived from the code, not from memory"
E_DIR=$(dirname "$0")
E_SET="$E_DIR/checked-set.tsv"
E_TRI="$E_DIR/checked-set-triage.tsv"
if [ ! -r "$E_SET" ] || [ ! -r "$E_TRI" ]; then
  report NONE E0 "checked-set.tsv or checked-set-triage.tsv missing — the coverage audit did NOT run, so nothing here says the checked set is complete."
else
  E_REV=$(grep -m1 '^# generated-from-rev' "$E_SET" | awk '{print $3}')
  if [ -z "$E_REV" ]; then
    report NONE E0 "checked-set.tsv does not record the rev it was generated from, so it cannot be shown to describe the serving build. NOT verified."
  elif ! git cat-file -e "${E_REV}^{commit}" 2>/dev/null; then
    report NONE E0 "checked-set.tsv was generated at '$E_REV', which this repo does not contain — cannot establish whether it still describes the serving build."
  elif ! git diff --quiet "$E_REV" "$REV" -- crates/store/src/read.rs 2>/dev/null; then
    report NONE E0 "STALE SET: read.rs differs between the fixture's rev ($E_REV) and the serving build ($REV), so the enumeration may no longer match the reads that run. Regenerate checked-set.tsv. NOT verified."
  else
    e_missing=0; e_n=0
    while IFS=$'\t' read -r c f _sql; do
      case "$c" in ''|'#'*) continue;; esac
      e_n=$((e_n+1))
      if ! awk -F'\t' -v c="$c" -v f="$f" '$1==c && $2==f {found=1} END{exit !found}' "$E_TRI"; then
        report FAIL "E:$c/$f" "the code can emit this read and NOTHING dispositions it — neither asserted, excluded, nor explained. This is the state read::organizations was in while serving 22.0s pages."
        e_missing=$((e_missing+1))
      fi
    done < "$E_SET"
    if [ "$e_n" -lt 20 ]; then
      report NONE E1 "only $e_n reads in checked-set.tsv — the fixture is implausibly thin (expected ~44), so the enumeration probably failed rather than the code having shrunk. NOT verified."
    elif [ "$e_missing" = 0 ]; then
      report PASS E1 "all $e_n derived reads carry a disposition ($(awk -F'\t' '!/^#/ && NF{print $3}' "$E_TRI" | sort | uniq -c | tr '\n' ' ' | sed 's/  */ /g'))"
    fi
  fi
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
#   When the read it mirrors changes, ASK WHETHER THE OLD SHAPE STILL WALKS before
#   touching this section — do not re-base reflexively. Issue 115 moved the driving
#   table from `lots` to `tender_version_lots`, and the answer was measured on the
#   post-115 catalogue: the statement below still compiles and still comes back
#   `SEARCH l USING INTEGER PRIMARY KEY (rowid=?)`. It is therefore still a known-bad
#   and still does its job, so it was left ALONE. Retire it only if the old shape stops
#   compiling or stops walking — i.e. only if it has stopped being a known-bad, which is
#   the one thing that would actually invalidate it. Re-basing a control every time the
#   real read moves quietly couples it to the thing it exists to be independent of.
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
echo "== $PASS pass, $FAIL fail, $NOINPUT no-input, $NA n/a (wrong build) =="
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
