#!/usr/bin/env bash
# STANDING structural gate for the canonical layer (task #28).
#
# Runs COUNT-FREE structural invariants against the daily SNAPSHOT with stock
# sqlite3, read-only. Every check answers "how many rows violate this?" and every
# expectation is 0 — so nothing here depends on corpus size and nothing can go
# stale the way run_light.sh's pinned totals did (1.1/1.2/1.3/1.8: 6961311 /
# 640745 / 6320566, taken seven days before the reclaims; deliberately NOT
# carried over here).
#
#   ./standing_gate.sh                  # Tier A against the newest snapshot
#   TIER=ALL ./standing_gate.sh         # every tier
#   SNAPSHOT=/path/x.db ./standing_gate.sh
#   ./standing_gate.sh --self-test      # no snapshot needed; see SELF-TEST below
#
# WHERE IT MAY RUN. Snapshot-side only, and — because a prod-box read gates on
# HOST, not size (prod-load-safety, 2026-08-04) — on the prod box only inside a
# resource-confined systemd unit (MemoryMax/IOWeight/CPUWeight), never bare. The
# snapshot being immutable buys the live ENDPOINT, not the live BOX: these scans
# would otherwise evict the whole ~2 GB page cache the live service shares.
#
# VACUITY. "Zero rows violate X" is trivially true of an EMPTY table, so a
# violation-count suite alone reports a nuked layer as green — the exact
# catastrophe the prenuke backup exists for. Verified, not assumed: on an empty
# `tenders`, identity_overlap and no_head both return 0. The `present_*` checks
# close that hole and stay count-free: they assert existence, never a pinned
# total, so they cannot go stale as the corpus grows. They use EXISTS rather than
# COUNT(*) > 0 — same answer, O(1) instead of a full scan.
#
# THEIR OWN ASSUMPTION, stated because proj-fix's inverse-vacuity lesson applies
# here too: these are ABSOLUTE assertions ("this table has rows"), and absolute
# assertions cry wolf wherever absence is legitimate. They are correct against a
# PRODUCTION snapshot — taken at the end of a daily fold, where empty always means
# something broke. They would fire spuriously on a fresh install, a pre-first-fold
# instance, or a restore in progress, all of which are legitimately empty.
#
# That is acceptable HERE because the gate's input is pinned to the prod snapshot
# ring and refuses anything stale — but it is an assumption about the deployment,
# not a property of the check, and anyone pointing this gate at a fresh instance
# should expect eleven false alarms rather than a bug report. (proj-fix's rule:
# assert the transition, not the state, when absence is sometimes correct. Here
# absence is never correct, so the absolute form is right — but only because of
# where it is pointed.)
#
# There is ONE present_* per table any check reads, not one for the spine. A
# partial set is a partial hole: run-driver's sweep bed had `tenders` populated
# but every satellite empty, which would sail through orphan_parties,
# orphan_texts and friends on absence alone. The rule is that no check may
# depend on a table whose non-emptiness is unasserted — cheap enough (EXISTS)
# that there is no reason to be selective about it.
#
# WHAT A GREEN DOES NOT MEAN. Several of these invariants are ALSO enforced
# upstream — `head_not_max` is issue #27's projection-time assertion, which
# refuses and rolls back rather than writing a violation. Where a preventer runs
# upstream, a clean snapshot is consistent with BOTH "nothing went wrong" and
# "something went wrong and was prevented", and this gate cannot tell them apart:
# the distinguishing signal is a FAILED JOB carrying the assertion's error, not
# anything visible here. So a green over such an invariant means "no damage
# present", never "the preventer works" — a gate downstream of a guard cannot
# validate that guard, because the guard's success and its absence look identical
# from here. What this gate genuinely adds over the assertion is coverage the
# assertion cannot have: damage that predates it, and Tenders no projection has
# touched since. (proj-fix, 2026-08-04 — sharper than the limit I first wrote.)
#
# And `repeat=no` proves only that the INPUT IS NEW — never that the cycle did
# useful work. A do-nothing projection followed by a successful snapshot yields a
# new file and a correct `repeat=no`. That is not a defect in the check; it is the
# boundary of what it claims. This gate verifies the artifact it is handed, so a
# green is never evidence of pipeline health. (proj-fix, 2026-08-04.)
#
# All three of the above are the same defect wearing different clothes — a result
# that reads as stronger than it is, and none of them visible in the number:
#   vacuity     — 0 because nothing is there
#   preventer   — 0 because something upstream stopped it
#   repeat=no   — new input, not a productive cycle
#
# TIERS bound the daily cost. A = single-table scans. B = joins and grouped
# anti-joins over the version-keyed tables. C = the ~30M organization_mentions
# anti-join. Intended cadence: A daily, B daily once measured, C weekly.
#
# FRESHNESS. The snapshot is produced by the app's daily pipeline (a supervisor
# job, Spec::Snapshot, last in enqueue_daily) — NOT by a timer. If ingestion
# wedges, the file simply stops refreshing and its mtime is the only tell. So the
# age assertion below is part of the gate, not a nicety: verifying a stale
# snapshot green is the failure this guards (issue 107).
#
# SELF-TEST. Each check carries its own `poison` — the minimal edit that must
# make it fire. `--self-test` builds a clean fixture, asserts every check reports
# 0, then re-poisons a fresh copy per check and asserts THAT check reports >0. A
# check with no poison is reported UNEXERCISED rather than silently trusted: a
# detector never seen to fail is decorative (README rule 2 / commit 0bae958).
# What the self-test proves is DETECTOR LOGIC, on a hand-built minimal schema.
# It does NOT prove the SQL binds to the real schema — that is proven by a real
# run, where a wrong table or column makes sqlite3 error and the runner FAILs.
# An errored query is never counted as 0 violations (see `violations`).
set -uo pipefail

TIER="${TIER:-A}"
SNAPSHOT_DIR="${SNAPSHOT_DIR:-/data/db/snapshots}"
MAX_AGE_H="${MAX_AGE_H:-30}"   # daily pipeline runs 09:35 Europe/Berlin; 30h = one missed run
# Where the last verified input's identity is recorded, so a repeat can be seen.
#
# ONE STATE FILE PER CADENCE, and the default derives from GATE_LABEL to make
# accidental sharing hard. Repeat detection keys on "did the input change since
# MY last run", so two schedules sharing a state file answer each other's
# question: a weekly reading the daily's bookkeeping reports repeat=no that looks
# exactly like a correct repeat=no, and FAIL_ON_REPEAT then guards nothing. The
# broken and working versions produce identically-shaped output — the same family
# as unwritable state silently reading as repeat=no. (proj-fix, 2026-08-04, before
# the daily/weekly split gets wired rather than after.)
GATE_LABEL="${GATE_LABEL:-tier$TIER}"
STATE_FILE="${STATE_FILE:-/var/lib/tender-db/standing_gate.$GATE_LABEL.last}"
FAIL_ON_REPEAT="${FAIL_ON_REPEAT:-0}"   # the daily timer sets 1; ad-hoc runs leave 0
SQLITE="${SQLITE:-sqlite3}"

# ---------------------------------------------------------------- the checks
# id|tier|label|sql|poison            (one line each; no '|' inside the fields)
# Every sql returns ONE integer: the number of violating rows. EXPECT 0.
checks() {
cat <<'CHECKS'
present_tenders|A|tenders non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tenders) THEN 0 ELSE 1 END|DELETE FROM tenders
present_versions|A|tender_versions non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_versions) THEN 0 ELSE 1 END|DELETE FROM tender_versions
present_orgs|A|organizations non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM organizations) THEN 0 ELSE 1 END|DELETE FROM organizations
present_mentions|A|organization_mentions non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM organization_mentions) THEN 0 ELSE 1 END|DELETE FROM organization_mentions
present_texts|A|tender_version_texts non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_version_texts) THEN 0 ELSE 1 END|DELETE FROM tender_version_texts
present_amounts|A|tender_version_amounts non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_version_amounts) THEN 0 ELSE 1 END|DELETE FROM tender_version_amounts
present_parties|A|tender_version_parties non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_version_parties) THEN 0 ELSE 1 END|DELETE FROM tender_version_parties
present_winners|A|tender_version_result_winners non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_version_result_winners) THEN 0 ELSE 1 END|DELETE FROM tender_version_result_winners
present_lots|A|lots non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM lots) THEN 0 ELSE 1 END|DELETE FROM lots
present_lot_results|A|lot_results non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM lot_results) THEN 0 ELSE 1 END|DELETE FROM lot_results
present_lot_result_rows|A|tender_version_lot_results non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_version_lot_results) THEN 0 ELSE 1 END|DELETE FROM tender_version_lot_results
present_bid_rows|A|tender_version_bids non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM tender_version_bids) THEN 0 ELSE 1 END|DELETE FROM tender_version_bids
present_changes|A|changes non-empty|SELECT CASE WHEN EXISTS(SELECT 1 FROM changes) THEN 0 ELSE 1 END|DELETE FROM changes
identity_overlap|A|tender identity is exactly one of keyed or island|SELECT COUNT(*) FROM tenders WHERE (procedure_key IS NULL) = (island_notice_id IS NULL)|INSERT INTO tenders(id,source,procedure_key,island_notice_id,kind,created_at,current_seq,current_published_at) VALUES (900,'ted','k900',900,'procedure',1,1,1)
no_head|A|every tender has a head version pointer|SELECT COUNT(*) FROM tenders WHERE current_seq IS NULL|INSERT INTO tenders(id,source,procedure_key,kind,created_at,current_seq) VALUES (901,'ted','k901','procedure',1,NULL)
kind_bad|A|tenders.kind within its domain|SELECT COUNT(*) FROM tenders WHERE kind NOT IN ('procedure','registration')|INSERT INTO tenders(id,source,procedure_key,kind,created_at,current_seq) VALUES (902,'ted','k902','zzz',1,1)
source_bad|A|tenders.source within its domain|SELECT COUNT(*) FROM tenders WHERE source NOT IN ('ted','doe')|INSERT INTO tenders(id,source,procedure_key,kind,created_at,current_seq) VALUES (903,'zz','k903','procedure',1,1)
islands_multi|A|islands are single-notice tenders|SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL AND current_seq <> 1|UPDATE tenders SET current_seq=2 WHERE island_notice_id IS NOT NULL
provisional_identifier|A|provisional org iff it has no identifier|SELECT COUNT(*) FROM organizations WHERE (provisional = 1) <> (identifier IS NULL)|UPDATE organizations SET provisional=1 WHERE identifier IS NOT NULL
identifier_kind_bad|A|organizations.identifier_kind within its domain|SELECT COUNT(*) FROM organizations WHERE identifier_kind IS NOT NULL AND identifier_kind NOT IN ('vat','national')|UPDATE organizations SET identifier_kind='zzz' WHERE identifier IS NOT NULL
absurd_pubdate|A|publication dates inside absolute sane bounds|SELECT COUNT(*) FROM tender_versions WHERE published_at < 631152000 OR published_at > 1800000000|UPDATE tender_versions SET published_at=1 WHERE seq=1
negative_amount|A|negative money only where it is meaningful (result_value)|SELECT COUNT(*) FROM tender_version_amounts WHERE cents < 0 AND field <> 'result_value'|UPDATE tender_version_amounts SET cents=-1 WHERE field='estimated_value'
negative_awarded|A|no negative awarded money|SELECT COUNT(*) FROM tender_version_lot_results WHERE awarded_cents < 0|UPDATE tender_version_lot_results SET awarded_cents=-1
negative_bid|A|no negative bid money|SELECT COUNT(*) FROM tender_version_bids WHERE cents < 0|UPDATE tender_version_bids SET cents=-1
changes_op_bad|A|changes.op within its domain|SELECT COUNT(*) FROM changes WHERE op NOT IN ('added','changed','removed')|UPDATE changes SET op='zzz'
changes_kind_bad|A|changes.entity_kind within its domain|SELECT COUNT(*) FROM changes WHERE entity_kind NOT IN ('tender','lot','organization','lot_result','bid','contract')|UPDATE changes SET entity_kind='zzz'
versions_ge_tenders|A|at least one version per tender|SELECT CASE WHEN (SELECT COUNT(*) FROM tender_versions) >= (SELECT COUNT(*) FROM tenders) THEN 0 ELSE 1 END|DELETE FROM tender_versions
min_seq_is_1|A|version seq is dense from 1|SELECT CASE WHEN (SELECT MIN(seq) FROM tender_versions) = 1 THEN 0 ELSE 1 END|UPDATE tender_versions SET seq=seq+10
head_pub_mismatch|B|head pointer matches its version publication date|SELECT COUNT(*) FROM tenders t JOIN tender_versions v ON v.tender_id=t.id AND v.seq=t.current_seq WHERE t.current_published_at <> v.published_at|UPDATE tenders SET current_published_at=current_published_at+1
head_not_max|B|current_seq is the maximum seq|SELECT COUNT(*) FROM tenders t WHERE EXISTS (SELECT 1 FROM tender_versions v WHERE v.tender_id=t.id AND v.seq > t.current_seq)|UPDATE tenders SET current_seq=current_seq-1 WHERE current_seq > 1
junk_hub|B|one notice causes a version in exactly one tender|SELECT COUNT(*) FROM (SELECT caused_by_notice_id FROM tender_versions GROUP BY caused_by_notice_id HAVING COUNT(*) > 1)|INSERT INTO tender_versions(tender_id,seq,caused_by_notice_id,published_at,publication_id) SELECT tender_id+500,seq,caused_by_notice_id,published_at,publication_id FROM tender_versions WHERE seq=1
org_identity_dupe|B|org identity unique among identified profiles|SELECT COUNT(*) FROM (SELECT 1 FROM organizations WHERE identifier IS NOT NULL GROUP BY country,identifier_kind,identifier HAVING COUNT(*) > 1)|INSERT INTO organizations(id,country,identifier_kind,identifier,name,provisional,created_at) SELECT id+500,country,identifier_kind,identifier,name,provisional,created_at FROM organizations WHERE identifier IS NOT NULL
orphan_versions|B|every version belongs to a tender|SELECT COUNT(*) FROM tender_versions v LEFT JOIN tenders t ON t.id=v.tender_id WHERE t.id IS NULL|INSERT INTO tender_versions(tender_id,seq,caused_by_notice_id,published_at,publication_id) VALUES (9999,1,9999,1000000000,'p9999')
orphan_texts|B|version texts point at a live version|SELECT COUNT(*) FROM tender_version_texts x LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL|INSERT INTO tender_version_texts(tender_id,seq,field,value) VALUES (9999,1,'title','orphan')
orphan_amounts|B|version amounts point at a live version|SELECT COUNT(*) FROM tender_version_amounts x LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL|INSERT INTO tender_version_amounts(tender_id,seq,field,cents,currency) VALUES (9999,1,'value',1,'EUR')
orphan_parties|B|version parties point at a live version|SELECT COUNT(*) FROM tender_version_parties x LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL|INSERT INTO tender_version_parties(tender_id,seq,organization_id,role) VALUES (9999,1,1,'buyer')
orphan_party_orgs|B|version parties point at a live organization|SELECT COUNT(*) FROM tender_version_parties p LEFT JOIN organizations o ON o.id=p.organization_id WHERE o.id IS NULL|UPDATE tender_version_parties SET organization_id=9999
orphan_winner_orgs|B|result winners point at a live organization|SELECT COUNT(*) FROM tender_version_result_winners w LEFT JOIN organizations o ON o.id=w.organization_id WHERE o.id IS NULL|UPDATE tender_version_result_winners SET organization_id=9999
orphan_lots|B|lots belong to a live tender|SELECT COUNT(*) FROM lots l LEFT JOIN tenders t ON t.id=l.tender_id WHERE t.id IS NULL|UPDATE lots SET tender_id=9999
orphan_lot_results|B|lot results belong to a live tender|SELECT COUNT(*) FROM lot_results r LEFT JOIN tenders t ON t.id=r.tender_id WHERE t.id IS NULL|UPDATE lot_results SET tender_id=9999
orphan_mention_orgs|C|every mention points at a live organization|SELECT COUNT(*) FROM organization_mentions m LEFT JOIN organizations o ON o.id=m.organization_id WHERE o.id IS NULL|UPDATE organization_mentions SET organization_id=9999
CHECKS
}

# ------------------------------------------------------------------ plumbing
red() { printf '\033[31m%s\033[0m' "$1"; }
grn() { printf '\033[32m%s\033[0m' "$1"; }

# violations <db> <sql> -> prints the integer, or returns 1 having printed the
# engine error to stderr. A query that ERRORS must never read as "0 violations":
# that is how a broken detector hides (commit 0bae958). Hence the numeric guard.
violations() {
  local out rc
  out=$("$SQLITE" -readonly "file:$1?mode=ro" "$2" 2>&1); rc=$?
  if [ $rc -ne 0 ] || ! [[ "$out" =~ ^[0-9]+$ ]]; then
    echo "    engine: ${out:-exit $rc}" >&2
    return 1
  fi
  echo "$out"
}

# TIER=0 selects ONLY the present_* checks — measured at 34ms for all eleven
# (195ms end-to-end), against 93,040ms for a single full-scan check
# (identity_overlap over 6.96M tenders). ~2,700x.
#
# WHAT THAT DOES *NOT* BUY, because I claimed it did. I argued this moves detection
# of a nuked layer "from within-a-day to within-minutes". FALSE, and proj-fix
# caught it: THIS GATE READS THE SNAPSHOT. Running the presence checks every five
# minutes asks, 288 times a day, whether a photograph taken this morning still has
# rows in it. A live layer emptied at noon stays invisible until tomorrow's
# snapshot. **Detection latency for the live catastrophe is bounded by the SNAPSHOT
# cadence, not the check cadence**, so running the same snapshot-side check more
# often cannot tighten it by even a second.
#
# What a frequent snapshot-side tier 0 *would* catch quickly is a snapshot file
# that is itself truncated or corrupted — real, worth something, and not the case
# the claim named.
#
# It also produces 288 identical PASSes a day: a stream carrying no information,
# which trains a reader to ignore a recurring green. That is the failure this suite
# exists to design out, so frequency here is a cost, not a free win.
#
# The honest claim is **within one snapshot cycle** — which is what it already was
# before the measurement. The 34ms is real; the inference from it was not.
#
# THEY ARE NOT REMOVED FROM A/B/C. Tier 0 is a SUBSET selector, not a partition.
# Moving them out would reopen the vacuity hole this gate was built to close: a
# Tier-A-only run would once again pass on an empty table, since "zero rows violate
# X" is trivially true of nothing. The rule stands — no check may depend on a table
# whose non-emptiness is unasserted in the same run — and at 34ms there is no cost
# argument for weakening it.
# in_tier <tier-of-check> <id-of-check>
in_tier() {
  case "$TIER" in
    ALL) return 0;;
    0)   case "$2" in present_*) return 0;; *) return 1;; esac;;
    *)   [ "$1" = "$TIER" ];;
  esac
}

# ----------------------------------------------------------------- self-test
fixture() { # fixture <db> — minimal CLEAN schema+data; every check must report 0
  rm -f "$1"
  "$SQLITE" "$1" <<'SQL'
CREATE TABLE tenders (id INTEGER PRIMARY KEY, source TEXT, procedure_key TEXT, island_notice_id INTEGER, kind TEXT, created_at INTEGER, current_seq INTEGER, current_published_at INTEGER);
CREATE TABLE tender_versions (tender_id INTEGER, seq INTEGER, caused_by_notice_id INTEGER, published_at INTEGER, publication_id TEXT, notice_subtype TEXT);
CREATE TABLE organizations (id INTEGER PRIMARY KEY, country TEXT, identifier_kind TEXT, identifier TEXT, name TEXT, provisional INTEGER, created_at INTEGER);
CREATE TABLE organization_mentions (notice_id INTEGER, section_id TEXT, organization_id INTEGER);
CREATE TABLE tender_version_texts (tender_id INTEGER, seq INTEGER, field TEXT, value TEXT);
CREATE TABLE tender_version_amounts (tender_id INTEGER, seq INTEGER, field TEXT, cents INTEGER, currency TEXT);
CREATE TABLE tender_version_parties (tender_id INTEGER, seq INTEGER, organization_id INTEGER, role TEXT);
CREATE TABLE tender_version_result_winners (tender_id INTEGER, seq INTEGER, lot_result_id INTEGER, organization_id INTEGER);
CREATE TABLE lots (id INTEGER PRIMARY KEY, tender_id INTEGER, lot_key TEXT);
CREATE TABLE lot_results (id INTEGER PRIMARY KEY, tender_id INTEGER, notice_id INTEGER, result_key TEXT);
CREATE TABLE tender_version_lot_results (tender_id INTEGER, seq INTEGER, lot_result_id INTEGER, lot_id INTEGER, decision TEXT, reason TEXT, awarded_cents INTEGER, awarded_currency TEXT);
CREATE TABLE tender_version_bids (tender_id INTEGER, seq INTEGER, bid_id INTEGER, lot_id INTEGER, cents INTEGER, currency TEXT);
CREATE TABLE changes (cursor INTEGER PRIMARY KEY, entity_kind TEXT, entity_id INTEGER, version_seq INTEGER, op TEXT, changed_at INTEGER);
-- one keyed tender with two versions, one island tender with one
INSERT INTO tenders VALUES (1,'ted','k1',NULL,'procedure',1,2,1000000200),(2,'doe',NULL,77,'registration',1,1,1000000000);
INSERT INTO tender_versions VALUES (1,1,11,1000000100,'p11','cn-standard'),(1,2,12,1000000200,'p12','corrigendum'),(2,1,77,1000000000,'p77',NULL);
INSERT INTO organizations VALUES (1,'DE','vat','DE123','Buyer',0,1),(2,NULL,NULL,NULL,'Prov',1,1);
INSERT INTO organization_mentions VALUES (11,'ORG-0001',1),(12,'ORG-0001',2);
INSERT INTO tender_version_texts VALUES (1,1,'title','T'),(1,2,'title','T2');
INSERT INTO tender_version_amounts VALUES (1,2,'value',5000,'EUR');
-- The real canonical amount field names (project.rs AMOUNTS maps to exactly
-- these three), so a FIELD-SENSITIVE check is exercised against the domain the
-- data actually has rather than against a placeholder. The NEGATIVE result_value
-- is deliberate and load-bearing: it is the permit arm of `negative_amount`
-- (issue 33). If that check ever forbids negatives on result_value again, the
-- CLEAN fixture stops reporting 0 and the self-test fails — so both arms of the
-- re-spec are proven by the existing machinery, with no extra case needed.
INSERT INTO tender_version_amounts VALUES (1,2,'estimated_value',5000,'EUR');
INSERT INTO tender_version_amounts VALUES (1,2,'framework_maximum',9000,'EUR');
INSERT INTO tender_version_amounts VALUES (1,2,'result_value',-250,'EUR');
INSERT INTO tender_version_parties VALUES (1,2,1,'buyer');
INSERT INTO lots VALUES (1,1,'LOT-0001');
INSERT INTO lot_results VALUES (1,1,12,'RES-0001');
INSERT INTO tender_version_result_winners VALUES (1,2,1,1);
INSERT INTO tender_version_lot_results VALUES (1,2,1,1,'selected',NULL,7500,'EUR');
INSERT INTO tender_version_bids VALUES (1,2,1,1,7200,'EUR');
INSERT INTO changes VALUES (1,'tender',1,1,'added',1000000100);
SQL
}

self_test() {
  local tmp; tmp=$(mktemp -d); local clean="$tmp/clean.db" pass=0 fail=0 unex=0
  fixture "$clean" || { echo "fixture build failed"; exit 2; }
  echo "== self-test: does each check actually fire? =="

  echo "-- clean fixture: every check must report 0"
  while IFS='|' read -r id tier label sql poison; do
    [ -z "${id:-}" ] && continue
    local got; got=$(violations "$clean" "$sql") || { printf '  %s %-24s query errored on the clean fixture\n' "$(red FAIL)" "$id"; fail=$((fail+1)); continue; }
    if [ "$got" = 0 ]; then pass=$((pass+1))
    else printf '  %s %-24s clean fixture reports %s violations — the check or the fixture is wrong\n' "$(red FAIL)" "$id" "$got"; fail=$((fail+1)); fi
  done < <(checks)
  echo "   $pass checks reported 0 on the clean fixture"

  echo "-- poisoned fixture: each check must catch its own violation"
  while IFS='|' read -r id tier label sql poison; do
    [ -z "${id:-}" ] && continue
    if [ -z "${poison:-}" ]; then
      printf '  %s %-24s no poison defined — never seen to fail\n' "$(red UNEXERCISED)" "$id"; unex=$((unex+1)); continue
    fi
    local db="$tmp/p.db"; cp "$clean" "$db"
    if ! "$SQLITE" "$db" "$poison" >/dev/null 2>&1; then
      printf '  %s %-24s poison failed to apply\n' "$(red FAIL)" "$id"; fail=$((fail+1)); continue
    fi
    local got; got=$(violations "$db" "$sql") || { printf '  %s %-24s query errored on the poisoned fixture\n' "$(red FAIL)" "$id"; fail=$((fail+1)); continue; }
    if [ "$got" -gt 0 ] 2>/dev/null; then printf '  %s %-24s fires (%s)\n' "$(grn PASS)" "$id" "$got"; pass=$((pass+1))
    else printf '  %s %-24s DID NOT FIRE on its own poison — decorative\n' "$(red FAIL)" "$id"; fail=$((fail+1)); fi
  done < <(checks)

  rm -rf "$tmp"
  echo
  echo "== self-test: $pass passed, $fail failed, $unex unexercised =="
  [ "$fail" -eq 0 ] && [ "$unex" -eq 0 ] || exit 1
  echo "Every check was seen to fail on a violation and pass on a clean layer."
  echo "NOTE: this proves detector LOGIC against a minimal hand-built schema, not"
  echo "that the SQL binds to the real one — a real run proves that (an unbound"
  echo "column errors, and an errored query FAILs rather than reading as 0)."
}

# ---------------------------------------------------------------- real run
main() {
  # PINNED vs NEWEST. The daily integration must pass SNAPSHOT=<the exact path the
  # pipeline just wrote>, not rely on "the newest". If the snapshot step FAILS, the
  # newest file is yesterday's — still inside MAX_AGE_H, so it would verify green
  # and report success for a cycle that produced nothing (proj-fix's catch). Age
  # bounds how stale the input can be; it cannot establish that it is THIS cycle's
  # output. Pinning can, so the mode is carried into the verdict either way and a
  # green from the guessed path never gets to look like a green from a pinned one.
  local snap="${SNAPSHOT:-}" mode=pinned
  if [ -z "$snap" ]; then
    mode=newest
    snap=$(ls -1 "$SNAPSHOT_DIR"/tender-db-*.db 2>/dev/null | sort | tail -1)
  fi
  [ -n "$snap" ] && [ -r "$snap" ] || { echo "$(red FAIL) no readable snapshot in $SNAPSHOT_DIR (set SNAPSHOT=)"; exit 2; }

  # COMPLETENESS, which is a DIFFERENT question from staleness and was missing.
  # The age check below bounds how OLD the input may be. Nothing bounded whether it
  # had finished being WRITTEN. Measured 2026-08-05: a snapshot in progress grew
  # 16 GB in 8 s, and `ls | sort | tail -1` selects exactly that file — so a gate
  # firing during the snapshot step reads a half-copied database. `pinned` mode is
  # not immune either: pinning the path the pipeline is *currently* writing pins an
  # incomplete file just as effectively.
  #
  # WHAT ACTUALLY HAPPENS, tested rather than assumed, and it is milder than I first
  # claimed: sqlite refuses a truncated database with "database disk image is
  # malformed", so the observed failure is a loud ERR -- a false RED, not a false
  # green. I asserted the false-green case before testing it and it did not
  # reproduce. It remains plausible on a 455 GB file where a query touches only
  # pages inside the written prefix, but that is UNDEMONSTRATED and is not the
  # justification for this check.
  #
  # The justification is that a timing problem was reporting as a data problem. With
  # transition alerting, a mid-write run turns every check ERR at once and pages
  # someone about the layer when nothing is wrong with the layer. Refusing by name
  # costs O(1) and says the true thing.
  #
  # O(1): the sqlite header carries page_size @16 and the in-header database size in
  # pages @28, authoritative when the change counter @24 equals version-valid-for
  # @92. A complete file is exactly page_size * pages. Fails CLOSED -- if the header
  # cannot be read or is non-authoritative, this refuses rather than waving through,
  # because a check that cannot tell must never answer "fine".
  local _ps _pages _cc _vv _expect _actual
  _be()  { od -An -tu4 -j"$1" -N4 -v --endian=big "$snap" 2>/dev/null | tr -d ' '; }
  _be2() { od -An -tu2 -j"$1" -N2 -v --endian=big "$snap" 2>/dev/null | tr -d ' '; }
  _ps=$(_be2 16); _pages=$(_be 28); _cc=$(_be 24); _vv=$(_be 92)
  [ "$_ps" = 1 ] && _ps=65536
  _actual=$(stat -c %s "$snap" 2>/dev/null)
  if [ -z "$_ps" ] || [ -z "$_pages" ] || [ -z "$_actual" ] || [ "${_pages:-0}" -eq 0 ] 2>/dev/null; then
    echo "$(red FAIL) cannot read sqlite header of $snap — refusing rather than guessing it is complete"; exit 2
  fi
  # THE AUTHORITATIVE FLAG IS REPORTED, NOT ENFORCED — and that distinction was
  # found by testing against the real artifact rather than the fixture. sqlite
  # documents the in-header size as valid only when change_counter ==
  # version_valid_for, so the first version of this check refused when they differ,
  # on "fail closed" grounds. Measured on prod: BOTH real snapshots have cc != vv,
  # including a verifiably complete one. Fail-closed there would have refused every
  # snapshot forever — a gate that never runs, which is worse than no gate because
  # it also looks installed. My own fail-closed instinct produced a permanent
  # false-red, and only the real file showed it; the fixture said cc == vv because
  # a freshly-created database is not a copy of a live WAL-mode one.
  #
  # The size comparison stands on its own evidence: on the same two real files it
  # discriminated exactly — 209,379,655,680 of a declared 455,205,724,160 while
  # being written, and byte-exact equality on the finished one. It only ever fires
  # when the file is SHORTER than its own header declares, which a merely-stale
  # header does not cause.
  _expect=$(( _ps * _pages ))
  if [ "$_cc" != "$_vv" ]; then
    echo "--   note: in-header size formally non-authoritative (cc=$_cc vv=$_vv) — normal for a snapshot of a live WAL database; size compared anyway, see the comment"
  fi
  if [ "$_actual" -lt "$_expect" ]; then
    echo "$(red FAIL) snapshot INCOMPLETE: $snap is $_actual B, header declares $_expect B ($_pages pages x $_ps) — still being written; this is a TIMING fault, not a layer fault"; exit 2
  fi

  # The input, stated — never assumed. A gate that verifies a stale snapshot
  # green is worse than no gate (issue 107).
  local mtime age_h
  mtime=$(stat -c %Y "$snap")
  age_h=$(( ( $(date +%s) - mtime ) / 3600 ))
  echo "== standing structural gate  tier=$TIER  $(date -u +%FT%TZ) =="
  # DECLARE THE EFFECTIVE CONFIG, not just the input. proj-fix's point: the probe
  # forwards an ENUMERATED list of variables across the systemd-run boundary, so
  # the next variable added here will silently not cross — the same defect as
  # GATE_LABEL, one variable later, failing the same silent way. This cannot
  # prevent the drop; it makes the drop VISIBLE, by printing what the gate is
  # actually using rather than what a caller believes it passed. Declare-your-input
  # discipline applied to the config rather than the snapshot.
  echo "-- config: label=$GATE_LABEL state=$STATE_FILE max_age=${MAX_AGE_H}h fail_on_repeat=$FAIL_ON_REPEAT"
  echo "-- input: $snap  [$mode]"
  echo "--   size $(stat -c %s "$snap") bytes, mtime $(date -u -d "@$mtime" +%FT%TZ), age ${age_h}h"
  [ "$mode" = newest ] && echo "--   NOTE resolved by newest-in-dir, NOT pinned: a failed snapshot step would" \
                       && echo "--        hand this run yesterday's file. Pass SNAPSHOT=<path> from the pipeline."
  if [ "$age_h" -gt "$MAX_AGE_H" ]; then
    echo "$(red FAIL) snapshot is ${age_h}h old (max ${MAX_AGE_H}h) — the daily pipeline is not producing."
    echo "VERDICT stale_input tier=$TIER label=$GATE_LABEL snapshot=$snap age_h=$age_h mode=$mode"
    exit 2
  fi

  # REPEAT DETECTION. Age alone leaves a hole that neither "newest" nor a
  # written-on-success `latest` pointer closes: if today's snapshot step fails,
  # yesterday's file is ~24h old — inside MAX_AGE_H — so the gate verifies it and
  # reports green for a cycle that produced nothing. Both resolutions name the
  # same stale file; the pointer is more trustworthy about *what* it names, not
  # about *when* it was made.
  #
  # What actually distinguishes them is whether the input CHANGED since last run.
  # Under a daily cadence, "identical to the snapshot I verified last time" means
  # the pipeline produced nothing this cycle — which is issue 119's open cadence
  # half, detected at the consumer without threshold-tuning MAX_AGE_H.
  local ident prev="" repeat=unknown
  ident="$(basename "$snap")|$mtime|$(stat -c %s "$snap")"
  [ -r "$STATE_FILE" ] && prev=$(cat "$STATE_FILE" 2>/dev/null)
  if [ -n "$prev" ]; then [ "$prev" = "$ident" ] && repeat=yes || repeat=no; fi
  if [ "$repeat" = yes ]; then
    echo "-- REPEAT INPUT: byte-identical to the last snapshot this gate verified."
    echo "--   On a daily cadence that means NO new snapshot was produced this cycle."
    if [ "$FAIL_ON_REPEAT" = 1 ]; then
      echo "$(red FAIL) refusing to re-report a green for an input already verified."
      echo "VERDICT repeat_input tier=$TIER label=$GATE_LABEL snapshot=$snap age_h=$age_h mode=$mode repeat=yes"
      exit 2
    fi
  fi

  local pass=0 fail=0 err=0 ran=0
  while IFS='|' read -r id tier label sql poison; do
    [ -z "${id:-}" ] && continue
    in_tier "$tier" "$id" || continue
    ran=$((ran+1))
    local got
    if ! got=$(violations "$snap" "$sql"); then
      printf '  %s %-24s %s\n' "$(red ERROR)" "$id" "$label"; err=$((err+1)); continue
    fi
    if [ "$got" = 0 ]; then printf '  %s %-24s %s\n' "$(grn PASS)" "$id" "$label"; pass=$((pass+1))
    else printf '  %s %-24s %s = %s violations\n' "$(red FAIL)" "$id" "$label" "$got"; fail=$((fail+1)); fi
  done < <(checks)

  echo
  echo "== $ran checks in tier $TIER: $pass passed, $fail failed, $err errored =="
  # One machine-readable line for the journal / dashboard.
  # Record what was verified, so the NEXT run can tell whether the input moved.
  # Written after the checks ran, and regardless of their verdict: this records
  # the input, not the outcome.
  if ! { mkdir -p "$(dirname "$STATE_FILE")" 2>/dev/null && printf '%s\n' "$ident" > "$STATE_FILE" 2>/dev/null; }; then
    echo "-- note: could not record input identity at $STATE_FILE — next run cannot"
    echo "--       detect a repeat, so it will report repeat=unknown, not repeat=no."
  fi
  echo "VERDICT $([ $((fail+err)) -eq 0 ] && echo ok || echo BROKEN) tier=$TIER label=$GATE_LABEL ran=$ran pass=$pass fail=$fail err=$err snapshot=$snap age_h=$age_h mode=$mode repeat=$repeat"
  [ $((fail+err)) -eq 0 ] || exit 1
}

# ARGUMENT CONTRACT — FAIL CLOSED. Running this script with no argument performs a
# FULL SNAPSHOT SCAN (455 GB on prod, ~29 min). That must require SAYING NOTHING,
# which is deliberate — never SAYING ANYTHING, which is a typo.
#
# The old default arm was `*) main`, so any unrecognised argument ran the scan.
# That is the exact mechanism of today's incident one level up: a wrapper invoked
# with `--self-test`, not understanding it, and running the payload UNCONFINED
# because preconditions execute before systemd-run. ac8ae13 fixed the wrapper;
# this is the same defect in the file everything delegates TO, which is the more
# dangerous of the two to leave guessing. (run-driver's review, which held the
# relaunch — the instance was fixed while the class stayed open.)
case "${1:-}" in
  --self-test) self_test;;
  # Write a clean minimal fixture — so the runner itself (freshness refusal,
  # error handling, exit codes) can be exercised without touching a real snapshot.
  --fixture)   [ -n "${2:-}" ] || { echo "usage: $0 --fixture <path>"; exit 2; }; fixture "$2";;
  "")          main;;
  *)           echo "refusing unknown argument '$1' — a bare run of this script is a full snapshot scan and it will not guess" >&2; exit 2;;
esac
