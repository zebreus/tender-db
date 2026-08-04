#!/usr/bin/env bash
# ============================================================================
# fixture_selectivity_gate.sh — issue 30a. Is this bed fit to be CLOCKED on?
#
# WHY THIS EXISTS. On 2026-08-04 the read-sweep bed (/tmp/bed16/bed40m.db) had
# 40.6M tender_version_lots, 13.2M lots, 4.26M tenders, and a prod-matching index
# set. It looked right. Every other satellite was EMPTY, and `tenders.kind` held a
# single value. So every filter selecting on an empty satellite — country, cpv,
# buyer, winner, status, min_value, max_value — would have returned nothing,
# INSTANTLY, and been recorded as fast. Including `tenders?kind=registration`,
# the one read we already know takes 18.7s.
#
# An empty table reads EXACTLY like a fixed one. A clock cannot tell them apart:
# both return in milliseconds. So the fixture needs a gate that runs BEFORE any
# timing, and the property it must assert is not "is there data" but:
#
#     EVERY FILTER MUST SELECT A NON-TRIVIAL, NON-TOTAL ROW SET.
#
# Selecting nothing measures absence. Selecting everything measures the unfiltered
# scan and calls it a filter. Both produce a number, neither measures the filter.
#
# THIS IS THE ANSWER/PATH/COST TAXONOMY ONE STEP EARLIER (see hot_read_plans.sh).
# That file asks "which instrument for this property?". This one asks the question
# that comes first and was skipped: DOES THE SUBJECT EXIST IN THIS BED AT ALL? A
# perfect instrument on an absent subject reports a confident wrong answer.
#
# ---------------------------------------------------------------------------
# ON `sqlite_stat1` — MEASURED, NOT ASSUMED (2026-08-04, issue 122)
#
# proj-fix's requirement was "the fixture must have NO sqlite_stat1; prod has none."
# Prod was checked instead of taken: `sqlite_stat1` EXISTS on prod (both the serving
# DB and the snapshot) and holds exactly ONE row —
#     plan_notice | plan_notice_fold | 12265 4 2 2 1 1
# a projection-internal table. No READ-PATH table appears in it.
#
# So the conclusion held (the planner has no stats for tenders.kind or any read-path
# index) but the requirement as stated was wrong in both directions: building the
# fixture via the real projection creates that plan_notice row, so a "no stat1 at
# all" check would REJECT the most faithful bed; and "no stat1 table" is not what
# prod looks like anyway. The load-bearing property is narrower and is what is
# enforced below: NO READ-PATH TABLE HAS A stat1 ROW.
#
# The single named way this breaks is a human running a bare `ANALYZE` on the
# fixture or on prod. Verified by grep across the workspace: the only ANALYZE in
# the codebase is the table-scoped `ANALYZE plan_notice` (canonical.rs:2273), so
# the property is maintained by construction — but a bare ANALYZE would silently
# invalidate every number taken here and leave no trace anyone would look for.
#
# ---------------------------------------------------------------------------
# READ-ONLY. Opens the fixture with `immutable=1` and only ever counts. It writes
# nothing, so it can never be the reason a bed changed between gate and clock.
#
# usage: fixture_selectivity_gate.sh <fixture.db>
#   exit 0 = FIT TO CLOCK      exit 1 = NOT FIT      exit 2 = could not run
# ============================================================================
set -uo pipefail

DB="${1:-}"
if [[ -z "$DB" ]]; then echo "usage: $0 <fixture.db>" >&2; exit 2; fi
if [[ ! -f "$DB" ]]; then echo "FAIL: no such fixture: $DB" >&2; exit 2; fi
if [[ -s "$DB-wal" ]]; then
  echo "FAIL: $DB has a non-empty -wal. Counts taken now may not be the counts a" >&2
  echo "      clock sees. Checkpoint the fixture first." >&2
  exit 2
fi
command -v sqlite3 >/dev/null || { echo "FAIL: sqlite3 not on PATH" >&2; exit 2; }

Q() { sqlite3 -readonly "file:$DB?immutable=1" "$1" 2>/dev/null; }

# A filter is fit if some value of it selects a share of the collection strictly
# inside these bounds. Wide on purpose: this gate rejects DEGENERATE beds, it does
# not impose a distribution. Prod-likeness of the skew is a separate question and
# is derived from the snapshot, not asserted here.
MIN_SHARE_PCT=0.01     # below this, a filter is measuring near-absence
MAX_SHARE_PCT=90       # above this, a filter is measuring the unfiltered scan

fails=0; checks=0
report() { # status label detail
  printf '%-5s %-34s %s\n' "$1" "$2" "$3"
  checks=$((checks+1)); [[ "$1" == FAIL ]] && fails=$((fails+1))
  return 0
}

echo "fixture: $DB"
echo "bytes:   $(stat -c %s "$DB" 2>/dev/null)"
echo

# --- 1. every table a read touches must be non-empty -------------------------
# EXISTS, not COUNT(*): O(1), so there is no cost argument for checking only some
# of them, and therefore no excuse for a partial set (sdk-vendor, ca1a9b8).
echo "--- presence: a check that reads an empty table certifies nothing ---"
for t in tenders tender_versions lots tender_version_lots tender_version_texts \
         tender_version_amounts tender_version_dates tender_version_classifications \
         tender_version_parties tender_version_result_winners organizations \
         organization_mentions notices; do
  if [[ -z "$(Q "SELECT name FROM sqlite_master WHERE type='table' AND name='$t';")" ]]; then
    report FAIL "present:$t" "TABLE ABSENT — any filter selecting on it measures nothing"
  elif [[ -z "$(Q "SELECT 1 FROM $t LIMIT 1;")" ]]; then
    report FAIL "present:$t" "EMPTY — selects nothing, reads as fast, indistinguishable from fixed"
  else
    report ok "present:$t" "non-empty"
  fi
done

# --- 2. no read-path table may carry planner statistics ----------------------
echo
echo "--- planner state: the bed's planner must be no smarter than prod's ---"
if [[ -z "$(Q "SELECT name FROM sqlite_master WHERE name='sqlite_stat1';")" ]]; then
  report ok "stat1:read-path-clean" "no sqlite_stat1 at all (prod has one row, for plan_notice; this is acceptable)"
else
  n=$(Q "SELECT COUNT(*) FROM sqlite_stat1 WHERE tbl IN
          ('tenders','lots','tender_versions','tender_version_lots',
           'tender_version_classifications','tender_version_parties',
           'tender_version_amounts','tender_version_dates','tender_version_texts',
           'tender_version_result_winners','organizations','organization_mentions','notices');")
  if [[ "${n:-0}" -gt 0 ]]; then
    report FAIL "stat1:read-path-clean" "$n read-path table(s) have stats — planner is SMARTER than prod's; every plan measured here is one prod cannot produce. Did someone run a bare ANALYZE?"
  else
    report ok "stat1:read-path-clean" "stat1 present but no read-path rows (prod-faithful)"
  fi
fi

# --- 3. every filter must select non-trivially and non-totally ---------------
# Each probe finds the BEST case available in this bed: if even the most balanced
# value of a filter is degenerate, no choice of parameter can make it measurable.
echo
echo "--- selectivity: bounds ${MIN_SHARE_PCT}% .. ${MAX_SHARE_PCT}% of the collection ---"

probe() { # label total_sql best_matching_sql
  local label="$1" total sel share
  total=$(Q "$2"); sel=$(Q "$3")
  if [[ -z "$total" || -z "$sel" || "$total" == "0" ]]; then
    report FAIL "select:$label" "no rows to select from (total=${total:-?})"; return
  fi
  share=$(Q "SELECT ROUND(100.0 * $sel / $total, 4);")
  if awk -v s="$share" -v lo="$MIN_SHARE_PCT" 'BEGIN{exit !(s<lo)}'; then
    report FAIL "select:$label" "best value selects ${share}% of ${total} — measures near-absence, not the filter"
  elif awk -v s="$share" -v hi="$MAX_SHARE_PCT" 'BEGIN{exit !(s>hi)}'; then
    report FAIL "select:$label" "best value selects ${share}% of ${total} — measures the unfiltered scan"
  else
    report ok "select:$label" "best value selects ${share}% of ${total}"
  fi
}

T="SELECT COUNT(*) FROM tenders;"
L="SELECT COUNT(*) FROM lots;"

probe "tenders.kind"    "$T" "SELECT COUNT(*) FROM tenders WHERE kind=(SELECT kind FROM tenders GROUP BY kind ORDER BY COUNT(*) ASC LIMIT 1);"
probe "tenders.source"  "$T" "SELECT COUNT(*) FROM tenders WHERE source=(SELECT source FROM tenders GROUP BY source ORDER BY COUNT(*) ASC LIMIT 1);"
probe "lots.kind"       "$L" "SELECT COUNT(*) FROM lots l WHERE EXISTS (SELECT 1 FROM tender_version_lots vl WHERE vl.lot_id=l.id AND vl.kind=(SELECT kind FROM tender_version_lots GROUP BY kind ORDER BY COUNT(*) ASC LIMIT 1));"
probe "country(nuts)"   "$T" "SELECT COUNT(DISTINCT tender_id) FROM tender_version_classifications WHERE scheme='nuts' AND code=(SELECT code FROM tender_version_classifications WHERE scheme='nuts' GROUP BY code ORDER BY COUNT(*) DESC LIMIT 1);"
probe "cpv"             "$T" "SELECT COUNT(DISTINCT tender_id) FROM tender_version_classifications WHERE scheme='cpv' AND code=(SELECT code FROM tender_version_classifications WHERE scheme='cpv' GROUP BY code ORDER BY COUNT(*) DESC LIMIT 1);"
probe "buyer"           "$T" "SELECT COUNT(DISTINCT tender_id) FROM tender_version_parties WHERE role LIKE '%Buyer%' AND organization_id=(SELECT organization_id FROM tender_version_parties WHERE role LIKE '%Buyer%' GROUP BY organization_id ORDER BY COUNT(*) DESC LIMIT 1);"
probe "winner"          "$T" "SELECT COUNT(DISTINCT tender_id) FROM tender_version_result_winners WHERE organization_id=(SELECT organization_id FROM tender_version_result_winners GROUP BY organization_id ORDER BY COUNT(*) DESC LIMIT 1);"
probe "status(deadline)" "$T" "SELECT COUNT(DISTINCT tender_id) FROM tender_version_dates WHERE field='submission_deadline' AND utc_seconds > (SELECT AVG(utc_seconds) FROM tender_version_dates WHERE field='submission_deadline');"
probe "value(amounts)"  "$T" "SELECT COUNT(DISTINCT tender_id) FROM tender_version_amounts WHERE cents > (SELECT AVG(cents) FROM tender_version_amounts);"

# --- 4. the anchor: a rare value must also sit LATE in id order --------------
# Rarity alone does not reproduce the 18.7s kind=registration read (issue 122).
# The read paginates `AND t.id > ? ORDER BY t.id LIMIT ?`, so what makes it slow
# is that matches sit late enough that the walk runs nearly to completion before
# LIMIT fills. A rare value sprinkled uniformly terminates early and the pathology
# VANISHES WHILE THE ROW COUNTS STILL LOOK CORRECT. Matches-late is the property.
echo
echo "--- anchor: the rarest kind must sit LATE in id order, not be sprinkled ---"
rare=$(Q "SELECT kind FROM tenders GROUP BY kind ORDER BY COUNT(*) ASC LIMIT 1;")
if [[ -z "$rare" ]]; then
  report FAIL "anchor:late-in-id" "no kind values at all"
else
  # Share of the id range that must be walked before the first match is found.
  pos=$(Q "SELECT ROUND(100.0 * (SELECT MIN(id) FROM tenders WHERE kind='$rare')
                        / (SELECT MAX(id) FROM tenders), 2);")
  if awk -v p="${pos:-0}" 'BEGIN{exit !(p<50)}'; then
    report FAIL "anchor:late-in-id" "rarest kind '$rare' first matches at ${pos}% of the id range — the walk stops early and the pathology is absent"
  else
    report ok "anchor:late-in-id" "rarest kind '$rare' first matches at ${pos}% of the id range"
  fi
fi

echo
if [[ "$fails" -gt 0 ]]; then
  echo "VERDICT: NOT FIT TO CLOCK — $fails of $checks checks failed."
  echo "A clock run on this bed would report absence as speed."
  exit 1
fi
echo "VERDICT: FIT TO CLOCK — $checks checks passed."
exit 0
