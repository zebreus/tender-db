#!/usr/bin/env bash
# ============================================================================
# V1 — eForms-DE 1.x post-fold facts verification (issue 85).
#
# READ-ONLY. Runs only SELECTs through POST /v1/sql plus unauthenticated GETs
# on /v1. Never enqueues a job, never writes. Re-runnable.
#
#   BASE_URL   default http://127.0.0.1:8080 (prod app; nginx may still be down)
#   TDB_TOKEN  API token (tdb_…) — REQUIRED, /v1/sql is account-gated
#   SAMPLE     notices per window (default 100); 5 windows per minor
#
#   # from the workstation, through a tunnel:
#   ssh -N -L 8080:127.0.0.1:8080 root@zebreus.click &
#   TDB_TOKEN=tdb_… ./de1x_verify.sh
#
#   # or on the box (needs curl+jq there):
#   ssh root@zebreus.click 'TDB_TOKEN=tdb_… bash -s' < de1x_verify.sh
#
# WHY THESE QUERIES AND NOT OTHERS
#   * turso cliffs on high-cardinality GROUP BY / DISTINCT and re-evaluates a
#     joined derived table (CTE with LIMIT/OFFSET) once per outer row, so every
#     fact query here is a LITERAL rowid window (`n.id BETWEEN a AND b`) with
#     correlated EXISTS on the satellites' (tender_id, seq) indexes. No CTE
#     joins, no DISTINCT, no COUNT(DISTINCT). Each call touches ~SAMPLE rows.
#   * /v1/sql caps at 10 s per query, 10k rows. Everything below is far inside.
#   * /v1/tenders/{id} and /v1/notices?tender= are NOT used: both go through
#     read::tender_detail, which full-scans tender_version_bid_parties on prod
#     rev 484e9204 (issue 89). The fix (a572544) adds a DEFERRED_TENDER_INDEXES
#     entry, and that const is only built by build_tender_indexes at a REBUILD's
#     end — the re-fold is a scoped incremental, so it does NOT auto-build; the
#     index needs the reindex op after the deploy. Until that op runs, the detail
#     facts come through /v1/sql and the list endpoints instead.
#
# EXPECTED RATES come from the empirical full-corpus scan of all 218,876 DE-1.x
# payloads (.scratch/de1x-scan.json, issue 75) — the share of notices whose XML
# actually carries the element. A projected rate far BELOW the source rate means
# the fold dropped facts; a rate at/above it means the mapping landed.
# ============================================================================
set -uo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
SAMPLE="${SAMPLE:-100}"

# TWO READ PATHS, one set of queries.
#
#   TDB_SNAPSHOT=/path/post-refold.db   stock sqlite3 on an immutable snapshot
#                                       (the preferred path: no token, no live
#                                       contention, real SQLite so no turso hash
#                                       cliff and no 10 s cap — which is what
#                                       unlocks the exhaustive section H)
#   TDB_TOKEN=tdb_…                     POST /v1/sql against the live app
#
# Every check below is a plain SELECT, so both paths run the identical SQL and
# must agree. Snapshot mode cannot serve the REST rendering checks — those live
# in de1x_spotcheck.sh and need the live app.
if [ -n "${TDB_SNAPSHOT:-}" ]; then
  [ -r "$TDB_SNAPSHOT" ] || { echo "TDB_SNAPSHOT=$TDB_SNAPSHOT is not readable" >&2; exit 2; }
  # immutable=1 skips locking entirely — but it also IGNORES a -wal sibling, so a
  # snapshot copied mid-write would read stale. Refuse rather than report numbers
  # from a half-copied file.
  # Refuse on a NON-EMPTY -wal only. Every snapshot in this deployment ships a
  # 0-byte -wal sibling (a checkpointed WAL that was never removed); an empty WAL
  # carries no frames, so immutable=1 ignoring it loses nothing. Guarding on mere
  # existence would have refused every legitimate snapshot on the box.
  if [ -s "${TDB_SNAPSHOT}-wal" ]; then
    echo "REFUSING: ${TDB_SNAPSHOT}-wal is non-empty ($(wc -c < "${TDB_SNAPSHOT}-wal") bytes) —" >&2
    echo "immutable=1 would ignore those frames and read stale data. Checkpoint (TRUNCATE) and re-run." >&2
    exit 2
  fi
  command -v sqlite3 >/dev/null || { echo "sqlite3 not found (try: nix shell nixpkgs#sqlite --command …)" >&2; exit 2; }
else
  : "${TDB_TOKEN:?set TDB_TOKEN (live /v1/sql) or TDB_SNAPSHOT (a post-refold snapshot file)}"
fi

MINORS=("eforms:eforms-de-1.1" "eforms:eforms-de-1.2" "eforms:eforms-de-1.0")

# Pre-fold baseline (snapshot 531): 0 of 110 sampled DE-1.x notices had ANY
# text / classification / party / amount / lot. Every non-zero below is new.
PASS=0; FAIL=0; EYE=0; HARDFAIL=0
# Conservation gates are counted SEPARATELY and never touch HARDFAIL, so the exit
# code answers exactly one question: did the fold land correct facts without
# double-counting? The only pre-fold baseline available is 2026-08-01 12:00 —
# ~13h and jobs 532/533/534 before the fold's retire — so a Δ mismatch there is
# at least as likely to be baseline staleness as a fold defect. Reporting that as
# NO-GO would hold the site down on evidence that cannot distinguish the two.
CONS_OK=0; CONS_UNK=0
cons() { # cons <OK|??> <id> <text>
  if [ "$1" = OK ]; then CONS_OK=$((CONS_OK+1)); printf '  \033[32mCONS\033[0m %-6s %s\n' "$2" "$3"
  else CONS_UNK=$((CONS_UNK+1)); printf '  \033[33mCONS?\033[0m %-6s %s\n' "$2" "$3"; fi
}

# Where `--baseline` writes the pre-fold top-line, for section G's conservation
# check. The re-fold is a SCOPED INCREMENTAL (`project rebuild=false`), so ids
# outside the cohort survive and the baseline is not perishable the way it would
# be under a renumbering rebuild — but it is still the only thing the Δ
# arithmetic can be checked against. run_light.sh's pinned literals (6,961,311 /
# 640,745 / 6,320,566) predate the reclaims and are NOT a substitute.
BASELINE="${BASELINE:-$(dirname "$0")/de1x-prefold-baseline.env}"

TAB=$(printf '\t')
if [ -n "${TDB_SNAPSHOT:-}" ]; then
  q() {
    local out
    # 256 MB page cache per query. The box has ~4 GB free against a 441 GB file,
    # and each q() is its own process, so nothing persists between queries except
    # the OS page cache — the heavy gates re-walk the same index pages 218k times,
    # which is exactly what this keeps resident. Peak is one process at a time.
    if ! out=$(sqlite3 -readonly -noheader -separator "$TAB" \
                 -cmd "PRAGMA cache_size=-262144" \
                 "file:${TDB_SNAPSHOT}?immutable=1" "$1" 2>&1); then
      echo "  sqlite3: $(printf '%s' "$out" | head -1)" >&2
      return 1
    fi
    printf '%s\n' "$out"
  }
else
  q() {
    local resp
    resp=$(curl -sS --max-time 120 -X POST "$BASE_URL/v1/sql" \
      -H "Authorization: Bearer $TDB_TOKEN" -H "Content-Type: text/plain" \
      --data-binary "$1") || { echo "  curl failed" >&2; return 1; }
    if ! echo "$resp" | jq -e '.rows' >/dev/null 2>&1; then
      echo "  server: $(echo "$resp" | jq -r '.error // .' 2>/dev/null | head -1)" >&2
      return 1
    fi
    echo "$resp" | jq -r '.rows[] | @tsv'
  }
fi
scalar() { q "$1" | head -1 | cut -f1; }
row()    { q "$1" | head -1; }

report() {
  case "$1" in
    PASS) PASS=$((PASS+1)); printf '  \033[32mPASS\033[0m %-6s %s\n' "$2" "$3";;
    FAIL) FAIL=$((FAIL+1)); printf '  \033[31mFAIL\033[0m %-6s %s\n' "$2" "$3";;
    EYE)  EYE=$((EYE+1));   printf '  \033[36mEYE \033[0m %-6s %s\n' "$2" "$3";;
  esac
}
eq()   { local g; g=$(scalar "$2"); [ "$g" = "$3" ] && report PASS "$1" "$5 = $g" \
         || { report FAIL "$1" "$5: got '$g', expected '$3'"; [ "$4" = hard ] && HARDFAIL=$((HARDFAIL+1)); }; }
zero() { eq "$1" "$2" 0 "$3" "$4"; }
info() { report EYE "$1" "$3: $(q "$2" | tr '\t' '/' | tr '\n' ' ')"; }
# rate <id> <have> <of> <min-percent> <hard|soft> <label>
rate() {
  local pct; pct=$(awk -v a="$2" -v b="$3" 'BEGIN{printf "%.1f", (b>0? 100*a/b : 0)}')
  if awk -v p="$pct" -v m="$4" 'BEGIN{exit !(p>=m)}'; then
    report PASS "$1" "$6 = $2/$3 (${pct}%, ≥ $4%)"
  else
    report FAIL "$1" "$6 = $2/$3 (${pct}%, expected ≥ $4%)"
    [ "$5" = hard ] && HARDFAIL=$((HARDFAIL+1))
  fi
}

# --------------------------------------------------------------------------
# `--baseline` — run this ONCE before the fold starts.
#
# The scoped incremental regroups the DE-1.x cohort out of islands into
# uuid-keyed procedures and touches the TED tenders those keys merge into —
# and NOTHING else. Without the pre-fold numbers "the totals moved" is
# unfalsifiable, which is how a regrouping bug hides. Eight scalars.
#
# CONTROL BAND: a fixed 10k-wide tender id range, plus how many of its rows are
# DE-1.x islands. Because the fold is scoped (no renumber), the band's tender
# count may only fall by tenders the cohort can explain — that is the direct
# test of "nothing outside the cohort regrouped", and it is only possible
# because this is NOT a rebuild.
# --------------------------------------------------------------------------
if [ "${1:-}" = "--baseline" ]; then
  echo "== pre-fold baseline  ($BASE_URL)  $(date -u +%FT%TZ) =="
  max_id=$(scalar 'SELECT COALESCE(MAX(id),0) FROM tenders')
  lo=$(( max_id / 3 )); hi=$(( lo + 10000 ))
  {
    echo "# pre-fold baseline captured $(date -u +%FT%TZ) from $BASE_URL"
    echo "PRE_TENDERS=$(scalar 'SELECT COUNT(*) FROM tenders')"
    echo "PRE_ISLANDS=$(scalar 'SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL')"
    echo "PRE_KEYED=$(scalar 'SELECT COUNT(*) FROM tenders WHERE procedure_key IS NOT NULL')"
    echo "PRE_VERSIONS=$(scalar 'SELECT COUNT(*) FROM tender_versions')"
    echo "PRE_PROJECTED=$(scalar "SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=1")"
    echo "PRE_DE1X_VERSIONS=$(scalar "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')")"
    echo "PRE_REMOVED_EVENTS=$(scalar "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='removed'")"
    echo "SPOT_LO=$lo"
    echo "SPOT_HI=$hi"
    echo "PRE_SPOT_TENDERS=$(scalar "SELECT COUNT(*) FROM tenders WHERE id BETWEEN $lo AND $hi")"
    echo "PRE_SPOT_DE_ISLANDS=$(scalar "SELECT COUNT(*) FROM tenders t JOIN notices n ON n.id=t.island_notice_id WHERE t.id BETWEEN $lo AND $hi AND n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')")"
  } | tee "$BASELINE"
  echo "written to $BASELINE — keep it; section G needs it after the fold."
  exit 0
fi

if [ -n "${TDB_SNAPSHOT:-}" ]; then
  echo "== V1 eForms-DE 1.x post-fold verification  (snapshot $TDB_SNAPSHOT)  $(date -u +%FT%TZ) =="
else
  echo "== V1 eForms-DE 1.x post-fold verification  ($BASE_URL)  $(date -u +%FT%TZ) =="
  echo "-- preflight"
  curl -sS --max-time 10 "$BASE_URL/health" | head -c 200; echo
fi

# ---------------------------------------------------------------------------
# A. Parse layer must be UNTOUCHED by a projection-only re-fold.
#    218,635 parsed + 241 held = 218,876 (issue 85 / ADR-0009 reclaim).
# ---------------------------------------------------------------------------
echo "-- A. parse layer unchanged (the re-fold must not re-parse anything)"
eq A1 "SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.1' AND parse_state='parsed'" 145717 hard "de-1.1 parsed"
eq A2 "SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.2' AND parse_state='parsed'" 72887 hard "de-1.2 parsed"
eq A3 "SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.0' AND parse_state='parsed'" 31 hard "de-1.0 parsed"
info A4 "SELECT (SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.1' AND parse_state<>'parsed'),(SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.2' AND parse_state<>'parsed')" "not-parsed 1.1/1.2 (the 241 residual)"
# projected watermark: after a successful fold every parsed cohort notice is 1.
zero A5 "SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.1' AND parse_state='parsed' AND projected=0" hard "de-1.1 unprojected leftovers"
zero A6 "SELECT COUNT(*) FROM notices WHERE profile='eforms:eforms-de-1.2' AND parse_state='parsed' AND projected=0" hard "de-1.2 unprojected leftovers"

# ---------------------------------------------------------------------------
# B. Every parsed notice caused exactly one version (store/lib.rs:417) — the
#    invariant issue 85's "109/110" left open once the parse_state filter is
#    applied. Windowed per minor to stay bounded.
# ---------------------------------------------------------------------------
echo "-- B. one version per parsed notice"
# Mechanical, not an eyeball comparison: the invariant is exact. 218,635 parsed
# cohort notices ⇒ exactly 218,635 cohort-caused versions. Fewer = notices that
# folded to nothing; more = the double-count G6/H4 chase. (A merge into a TED
# twin still yields one version — it just hangs off a different Tender.)
eq B1 "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id
        WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')
          AND n.parse_state='parsed'" 218635 hard "cohort versions (== parsed notices)"
for P in "${MINORS[@]}"; do
  n=$(scalar "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id WHERE n.profile='$P'")
  report EYE "B:$P" "versions caused = $n"
done

# ---------------------------------------------------------------------------
# C. THE FACTS CHECK — 5 rowid windows per minor.
#    Source rates (all 218,876 payloads, de1x-scan.json):
#      title 100.0% · description 100.0% · main CPV 100.0% · NUTS 98.1%
#      lots 99.99% (up to 404/notice) · buyer ref 100.0% · subtype 100.0%
#      any deadline ~47.7% · tender-scope estimate 17.6% · lot estimate 10.7%
#      payable (award) 18.8% · LotResult 42.8%
# ---------------------------------------------------------------------------
echo "-- C. facts on the folded versions (pre-fold baseline: 0 of 110 had ANY)"
facts_sql() { # $1 profile  $2 id-lo  $3 id-hi
cat <<SQL
SELECT COUNT(*),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_texts x WHERE x.tender_id=v.tender_id AND x.seq=v.seq AND x.field='title') THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_texts x WHERE x.tender_id=v.tender_id AND x.seq=v.seq AND x.field='description') THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_classifications c WHERE c.tender_id=v.tender_id AND c.seq=v.seq AND c.scheme='cpv') THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_classifications c WHERE c.tender_id=v.tender_id AND c.seq=v.seq AND c.scheme='nuts') THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_amounts a WHERE a.tender_id=v.tender_id AND a.seq=v.seq) THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_dates d WHERE d.tender_id=v.tender_id AND d.seq=v.seq) THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_lots l WHERE l.tender_id=v.tender_id AND l.seq=v.seq) THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_parties p WHERE p.tender_id=v.tender_id AND p.seq=v.seq AND p.role LIKE '%uyer%') THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN v.notice_subtype IS NOT NULL THEN 1 ELSE 0 END),0),
 COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_texts x WHERE x.tender_id=v.tender_id AND x.seq=v.seq AND x.lot_id IS NOT NULL) THEN 1 ELSE 0 END),0)
FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id
WHERE n.profile='$1' AND n.parse_state='parsed' AND n.id BETWEEN $2 AND $3
SQL
}
declare -A T=( [n]=0 [title]=0 [desc]=0 [cpv]=0 [nuts]=0 [amt]=0 [date]=0 [lots]=0 [buyer]=0 [sub]=0 [lottext]=0 )
for P in "${MINORS[@]}"; do
  total=$(scalar "SELECT COUNT(*) FROM notices WHERE profile='$P'")
  [ -z "$total" ] && continue
  for frac in 0 20 40 60 80; do
    off=$(( total * frac / 100 ))
    lo=$(scalar "SELECT id FROM notices WHERE profile='$P' ORDER BY id LIMIT 1 OFFSET $off")
    hi=$(scalar "SELECT id FROM notices WHERE profile='$P' ORDER BY id LIMIT 1 OFFSET $(( off + SAMPLE - 1 ))")
    [ -z "$lo" ] && continue
    [ -z "$hi" ] && hi=$(scalar "SELECT MAX(id) FROM notices WHERE profile='$P'")
    r=$(row "$(facts_sql "$P" "$lo" "$hi")") || continue
    IFS=$'\t' read -r c ti de cp nu am da lo_ bu su lt <<<"$r"
    printf '   %-24s ids %-10s..%-10s  n=%-4s title=%-4s desc=%-4s cpv=%-4s nuts=%-4s amt=%-4s date=%-4s LOTS=%-4s buyer=%-4s subtype=%-4s lot-text=%s\n' \
      "$P" "$lo" "$hi" "$c" "$ti" "$de" "$cp" "$nu" "$am" "$da" "$lo_" "$bu" "$su" "$lt"
    T[n]=$((T[n]+c)); T[title]=$((T[title]+ti)); T[desc]=$((T[desc]+de)); T[cpv]=$((T[cpv]+cp))
    T[nuts]=$((T[nuts]+nu)); T[amt]=$((T[amt]+am)); T[date]=$((T[date]+da)); T[lots]=$((T[lots]+lo_))
    T[buyer]=$((T[buyer]+bu)); T[sub]=$((T[sub]+su)); T[lottext]=$((T[lottext]+lt))
  done
done
echo "   -- pooled over all windows (n=${T[n]}):"
rate C1  "${T[title]}"   "${T[n]}" 95 hard "title        (source 100.0%)"
rate C2  "${T[desc]}"    "${T[n]}" 90 hard "description  (source 100.0%)"
rate C3  "${T[cpv]}"     "${T[n]}" 95 hard "CPV          (source 100.0%)"
rate C4  "${T[nuts]}"    "${T[n]}" 90 hard "NUTS         (source  98.1%)"
rate C5  "${T[lots]}"    "${T[n]}" 95 hard "LOTS         (source 100.0%)  <-- the second fix"
rate C6  "${T[buyer]}"   "${T[n]}" 90 hard "buyer party  (source 100.0%)"
rate C7  "${T[sub]}"     "${T[n]}" 95 hard "notice_subtype (was NULL pre-fold)"
rate C8  "${T[amt]}"     "${T[n]}" 15 soft "amounts      (source ~35-45% union)"
rate C9  "${T[date]}"    "${T[n]}" 35 soft "dates        (source ~47.7% deadline)"
rate C10 "${T[lottext]}" "${T[n]}" 90 soft "lot-scoped text (BT-21/24-Lot landed)"

# ---------------------------------------------------------------------------
# D. Lots in detail — the Lot/LotsGroup/Part section-id fix (de1_lot_kind).
#    Pre-fix EVERY DE-1.x lot was dropped (kind 'ProcurementProjectLot' matched
#    no LOT_KIND), so a field-only fix would leave all of these at zero.
# ---------------------------------------------------------------------------
echo "-- D. lots: kinds, key prefixes, per-notice count parity"
P=eforms:eforms-de-1.1
total=$(scalar "SELECT COUNT(*) FROM notices WHERE profile='$P'")
lo=$(scalar "SELECT id FROM notices WHERE profile='$P' ORDER BY id LIMIT 1 OFFSET $(( total/2 ))")
hi=$(scalar "SELECT id FROM notices WHERE profile='$P' ORDER BY id LIMIT 1 OFFSET $(( total/2 + SAMPLE - 1 ))")
[ -z "$lo" ] && lo=0
[ -z "$hi" ] && hi=$(scalar "SELECT COALESCE(MAX(id),0) FROM notices WHERE profile='$P'")
info D1 "SELECT vl.kind, COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id JOIN tender_version_lots vl ON vl.tender_id=v.tender_id AND vl.seq=v.seq WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi GROUP BY vl.kind" \
  "lot kinds in window (expect Lot dominant; LotsGroup/Part only if GLO-/PAR- ids exist)"
zero D2 "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id JOIN tender_version_lots vl ON vl.tender_id=v.tender_id AND vl.seq=v.seq JOIN lots l ON l.id=vl.lot_id WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi AND vl.kind NOT IN ('Lot','LotsGroup','Part')" hard "lot kind outside the domain"
zero D3 "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id JOIN tender_version_lots vl ON vl.tender_id=v.tender_id AND vl.seq=v.seq JOIN lots l ON l.id=vl.lot_id WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi AND ((substr(l.lot_key,1,4)='LOT-' AND vl.kind<>'Lot') OR (substr(l.lot_key,1,4)='GLO-' AND vl.kind<>'LotsGroup') OR (substr(l.lot_key,1,4)='PAR-' AND vl.kind<>'Part'))" hard "lot_key prefix disagrees with kind (de1_lot_kind)"
# Parity: parse-layer ProcurementProjectLot sections vs projected lots, same notices.
info D4 "SELECT (SELECT COUNT(*) FROM notices n JOIN notice_sections s ON s.notice_id=n.id WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi AND s.kind='ProcurementProjectLot'),(SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id JOIN tender_version_lots vl ON vl.tender_id=v.tender_id AND vl.seq=v.seq WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi)" \
  "parse-layer lot sections / projected lot rows (should match)"

# ---------------------------------------------------------------------------
# E. Grouping & identity — what the is_uuid folder-id gate actually did.
#    Pre-fold the whole cohort was keyless islands. A uuid-shaped
#    DE1-ContractFolderID (99.0% of payloads carry SOME folder id) now keys a
#    procedure and may merge with a TED twin under ADR-0003.
# ---------------------------------------------------------------------------
echo "-- E. identity: island vs keyed, cross-source merges (eyeball)"
info E1 "SELECT CASE WHEN t.procedure_key IS NULL THEN 'island' ELSE 'keyed' END, COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id JOIN tenders t ON t.id=v.tender_id WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi GROUP BY 1" "island/keyed split in window"
info E2 "SELECT t.source, COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id JOIN tenders t ON t.id=v.tender_id WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi GROUP BY t.source" "tender source (any 'ted' = a DÖE↔TED merge, ADR-0003)"
info E3 "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id WHERE n.profile='$P' AND n.id BETWEEN $lo AND $hi AND v.seq>1" "versions with seq>1 (regrouping happened)"

# ---------------------------------------------------------------------------
# F. Quarantine — NOT a "new reason bucket" check.
#    Issue 87: a failed reclaim leaves the FIRST-INGEST reason in place and
#    never stamps reprocessed_at, so "did a new reason bucket appear?" is
#    STRUCTURALLY UNABLE TO FAIL for this cohort. What can fail: the counts.
# ---------------------------------------------------------------------------
echo "-- F. quarantine ledger counts (see the issue-87 caveat below)"
# HARD, because this pair IS the dashboard's user-facing "Resolved" claim, and
# nginx coming up is what publishes it. The ledger says eForms-DE 1.x resolved
# 2026-07-29; that only becomes true when the cohort renders facts.
eq F1a "SELECT COALESCE(SUM(CASE WHEN reprocessed_at IS NOT NULL THEN 1 ELSE 0 END),0) FROM quarantine WHERE reason='unknown-customization' AND detail LIKE '%eforms-de-1.%'" \
  218635 hard "ledger: resolved"
eq F1b "SELECT COALESCE(SUM(CASE WHEN reprocessed_at IS NULL THEN 1 ELSE 0 END),0) FROM quarantine WHERE reason='unknown-customization' AND detail LIKE '%eforms-de-1.%'" \
  241 hard "ledger: still held"
info F2 "SELECT reason, COUNT(*) FROM quarantine WHERE profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2') AND reprocessed_at IS NULL GROUP BY reason" \
  "held DE-1.x rows by reason (expect ONLY the stale unknown-customization bucket)"
cat <<'CAVEAT'
   NOTE (issue 87): F2 showing "no new bucket" proves NOTHING — Reclaim::StillHeld
   never rewrites reason/detail. The 241 residual still claim
   "no vendored SDK metadata for eforms-de-1.1", which is false since issue 75.
   The pre-reclaim prediction was ~293 malformed-money (unrepresentable-value,
   >2 fraction digits); the actual residual is 241 under a stale label, and the
   52-row gap CANNOT be explained from the database. Settling it needs the
   issue-87 fix or an offline re-parse of those members from the archive.
   Member list for that offline pass (read-only):
     SELECT q.fetch_id, q.member_path, q.detail FROM quarantine q
      WHERE q.reason='unknown-customization' AND q.detail LIKE '%eforms-de-1.%'
        AND q.reprocessed_at IS NULL ORDER BY q.fetch_id LIMIT 300;
CAVEAT

# ---------------------------------------------------------------------------
# G. Conservation across the regrouping (scoped incremental, `rebuild=false`).
#
#    What must hold:
#      * one version per projected parsed notice (the corpus is conserved),
#      * islands + keyed == tenders (identity is exclusive and total),
#      * the DE-1.x move closes: islands retired − new keyed groups == tenders lost,
#      * nothing outside the cohort regrouped (the control band).
#
#    G6 IS THE CHECK THIS MECHANISM NEEDS AND A REBUILD WOULD NOT HAVE.
#    A rebuild empties the layer first, so a notice can only land once. The
#    incremental path instead UPGRADES a notice from its island Tender to a
#    uuid-keyed one, and the old island is removed by `retire_regrouped_tenders`
#    (project.rs:931) — which retires a touched Tender only if the freshly built
#    plan did not reproduce its group_key. `tender_versions`' UNIQUE is
#    (tender_id, caused_by_notice_id): it is per-Tender, so it CANNOT catch the
#    same notice holding a version in both the retired-but-surviving island and
#    the new keyed Tender. If retirement misses anything, the corpus silently
#    double-counts and every density rate above is inflated. G6 is the only
#    check that sees it; G7 catches the other half (a Tender left with no
#    versions at all), G9 cross-checks the retirement count from the change feed.
# ---------------------------------------------------------------------------
echo "-- G. conservation across the regrouping"
# G6/G7 need no baseline — run them unconditionally.
gdup=0
for P in "${MINORS[@]}"; do
  total=$(scalar "SELECT COUNT(*) FROM notices WHERE profile='$P'")
  [ -z "$total" ] || [ "$total" -eq 0 ] && continue
  for frac in 10 50 90; do
    off=$(( total * frac / 100 ))
    a=$(scalar "SELECT id FROM notices WHERE profile='$P' ORDER BY id LIMIT 1 OFFSET $off")
    b=$(scalar "SELECT id FROM notices WHERE profile='$P' ORDER BY id LIMIT 1 OFFSET $(( off + 500 ))")
    [ -z "$a" ] && continue
    [ -z "$b" ] && b=$(scalar "SELECT COALESCE(MAX(id),0) FROM notices WHERE profile='$P'")
    d=$(scalar "SELECT COUNT(*) FROM (SELECT caused_by_notice_id FROM tender_versions WHERE caused_by_notice_id BETWEEN $a AND $b GROUP BY caused_by_notice_id HAVING COUNT(*) > 1)")
    gdup=$(( gdup + ${d:-0} ))
  done
done
if [ "$gdup" -eq 0 ]; then
  report PASS G6 "no notice holds versions in two Tenders (retirement complete, sampled)"
else
  report FAIL G6 "$gdup sampled notices hold >1 version — retire_regrouped_tenders missed islands"
  HARDFAIL=$((HARDFAIL+1))
fi
zero G7 "SELECT COUNT(*) FROM tenders WHERE current_seq IS NULL" hard "Tenders left with no head version (empty shells)"

# G1/G2 are CORE, not conservation: both are self-consistency of the post-fold
# layer and need no baseline at all. They were previously trapped inside the
# `if [ -f "$BASELINE" ]` block, so a missing baseline silently skipped two real
# gates. Moved out — they run always and hard-fail always.
eq G1 "SELECT CASE WHEN (SELECT COUNT(*) FROM tenders) = (SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL) + (SELECT COUNT(*) FROM tenders WHERE procedure_key IS NOT NULL) THEN 1 ELSE 0 END" 1 hard "islands + keyed == tenders"
# Components printed alongside so a failure is diagnosable on sight rather than
# a bare 0/1 — a notice re-parsed between fold and snapshot moves the right side.
info G2a "SELECT (SELECT COUNT(*) FROM tender_versions), (SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=1)" "versions / projected-parsed notices"
eq G2 "SELECT CASE WHEN (SELECT COUNT(*) FROM tender_versions) = (SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=1) THEN 1 ELSE 0 END" 1 hard "one version per projected parsed notice"
if [ -f "$BASELINE" ]; then
  # shellcheck disable=SC1090
  . "$BASELINE"
  post_t=$(scalar "SELECT COUNT(*) FROM tenders")
  post_i=$(scalar "SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL")
  post_k=$(scalar "SELECT COUNT(*) FROM tenders WHERE procedure_key IS NOT NULL")
  post_v=$(scalar "SELECT COUNT(*) FROM tender_versions")
  post_p=$(scalar "SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=1")
  echo "   pre : tenders=$PRE_TENDERS islands=$PRE_ISLANDS keyed=$PRE_KEYED versions=$PRE_VERSIONS projected=$PRE_PROJECTED"
  echo "   post: tenders=$post_t islands=$post_i keyed=$post_k versions=$post_v projected=$post_p"
  di=$((PRE_ISLANDS - post_i)); dk=$((post_k - PRE_KEYED)); dt=$((PRE_TENDERS - post_t))
  echo "   Δ islands retired=$di · new keyed groups=$dk · net tenders lost=$dt"
  if [ "$dt" -eq $((di - dk)) ]; then
    cons OK G3 "regrouping arithmetic closes ($dt == $di - $dk)"
  else
    cons ?? G3 "regrouping arithmetic does not close: $dt != $di - $dk (baseline is 2026-08-01 12:00, ~13h and jobs 532/533/534 before the fold — staleness is the likely cause, not the fold)"
  fi
  if [ "$di" -ge 0 ] && [ "$di" -le 218635 ]; then
    cons OK G4 "islands retired within what the DE-1.x cohort can explain ($di ≤ 218,635)"
  else
    cons ?? G4 "islands retired = $di — outside the cohort, OR the baseline predates jobs 532/533/534"
  fi
  report EYE G5 "uuid-gate yield ≈ $di of 218,635 cohort notices left island status"

  # G8 — the control band. Only possible because this is a scoped incremental:
  # ids outside the cohort survive, so the band's count may fall by AT MOST the
  # DE-1.x islands it held. A bigger drop means something else regrouped.
  post_spot=$(scalar "SELECT COUNT(*) FROM tenders WHERE id BETWEEN $SPOT_LO AND $SPOT_HI")
  floor=$((PRE_SPOT_TENDERS - PRE_SPOT_DE_ISLANDS))
  if [ "$post_spot" -le "$PRE_SPOT_TENDERS" ] && [ "$post_spot" -ge "$floor" ]; then
    cons OK G8 "control band ids $SPOT_LO-$SPOT_HI: $PRE_SPOT_TENDERS → $post_spot (within [$floor, $PRE_SPOT_TENDERS])"
  else
    cons ?? G8 "control band ids $SPOT_LO-$SPOT_HI: $PRE_SPOT_TENDERS → $post_spot, outside [$floor, $PRE_SPOT_TENDERS] — non-cohort Tenders moved, OR the baseline predates jobs 532/533/534"
  fi

  # G9 — independent cross-check of G3's `islands retired`: retire_tender_tx
  # appends a `removed` tender change per retirement (canonical.rs:2915).
  post_rm=$(scalar "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='removed'")
  report EYE G9 "removed-tender events this fold = $((post_rm - PRE_REMOVED_EVENTS)) (should track islands retired = $di)"
else
  cons ?? G0 "no baseline at $BASELINE — the Δ arithmetic (G3/G4/G5/G8/G9) is UNVERIFIED. Core gates are unaffected."
fi

# ---------------------------------------------------------------------------
# H. EXHAUSTIVE — snapshot mode only. Real SQLite has no turso hash cliff and no
#    10 s request cap, so the whole-cohort forms of C and G6 run for real instead
#    of being sampled. This is the difference between "500 sampled notices all
#    had facts" and "zero of 218,635 lack them" — issue 85's actual acceptance
#    wording. Minutes, not seconds; every lookup is an index seek.
# ---------------------------------------------------------------------------
# --------------------------------------------------------------------------
# INTERIM PROGRESS — everything above (A, B1, C, D, F, G1/G2/G6/G7) is the cheap
# core. It is an early health signal ONLY. It is NOT the verdict and must never
# be read as one: H1 is strictly stronger than C's sampled ≥95% — a sample can
# pass while H1 finds a real pocket of factless versions, which is precisely the
# failure the fold exists to prevent. The exit code comes after H.
# --------------------------------------------------------------------------
echo
echo "  ==============================================================="
if [ "$HARDFAIL" -eq 0 ]; then
  echo "  INTERIM — NOT THE VERDICT. Cheap core gates: $PASS passed, 0 hard-fail."
  echo "  On track, but H can still fail: a sampled rate can pass while the"
  echo "  exhaustive pass finds factless versions the sample never touched."
else
  echo "  INTERIM — NOT THE VERDICT. Cheap core gates: $HARDFAIL HARD-FAIL already."
  echo "  This is heading to NO-GO; H runs anyway so the report is complete."
fi
echo "  Exhaustive H next (~30-60 min on 441 GB). Exit code comes after it."
echo "  ==============================================================="
echo

COHORT="n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2')"
# Heartbeat for the long phase — a silent 30-60 min terminal is indistinguishable
# from a hung one.
hstep() { printf '   [%s] running %s …\n' "$(date -u +%H:%M:%SZ)" "$1"; }
if [ -n "${TDB_SNAPSHOT:-}" ]; then
  echo "-- H. exhaustive whole-cohort checks (snapshot) — started $(date -u +%H:%M:%SZ)"
  hstep "H1 (cohort versions with no text)"
  zero H1 "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id
            WHERE $COHORT AND n.parse_state='parsed'
              AND NOT EXISTS (SELECT 1 FROM tender_version_texts x
                               WHERE x.tender_id=v.tender_id AND x.seq=v.seq)" \
       hard "cohort versions with NO text at all (issue 85's symptom, exhaustively)"
  hstep "H2 (cohort versions with no CPV)"
  zero H2 "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id
            WHERE $COHORT AND n.parse_state='parsed'
              AND NOT EXISTS (SELECT 1 FROM tender_version_classifications c
                               WHERE c.tender_id=v.tender_id AND c.seq=v.seq AND c.scheme='cpv')" \
       hard "cohort versions with no CPV (source rate is 100.0%)"
  # 218,853 of 218,876 payloads carry a ProcurementProjectLot — ~23 genuinely have
  # none, so this is a small-threshold gate, not a zero gate. Anything larger is
  # the lot fix failing, not the source.
  hstep "H3 (cohort versions with no lots)"
  lotless=$(scalar "SELECT COUNT(*) FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id
                     WHERE $COHORT AND n.parse_state='parsed'
                       AND NOT EXISTS (SELECT 1 FROM tender_version_lots l
                                        WHERE l.tender_id=v.tender_id AND l.seq=v.seq)")
  if [ "${lotless:-0}" -le 50 ]; then
    report PASS H3 "cohort versions with NO lots = ${lotless:-?} (≤ 50; ~23 payloads genuinely carry none)"
  else
    report FAIL H3 "cohort versions with NO lots = $lotless (> 50) — the Lot/LotsGroup/Part fix is not landing"
    HARDFAIL=$((HARDFAIL+1))
  fi
  hstep "H4 (double-count, exhaustive)"
  zero H4 "SELECT COUNT(*) FROM (
             SELECT v.caused_by_notice_id FROM notices n JOIN tender_versions v
               ON v.caused_by_notice_id=n.id WHERE $COHORT
              GROUP BY v.caused_by_notice_id HAVING COUNT(*) > 1)" \
       hard "cohort notices holding versions in >1 Tender (G6, exhaustively)"
  hstep "H5 (exact per-fact rates — the slowest, 6 probes per notice)"
  info H5 "SELECT COUNT(*),
             SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_texts x WHERE x.tender_id=v.tender_id AND x.seq=v.seq AND x.field='title') THEN 1 ELSE 0 END),
             SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_classifications c WHERE c.tender_id=v.tender_id AND c.seq=v.seq AND c.scheme='nuts') THEN 1 ELSE 0 END),
             SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_amounts a WHERE a.tender_id=v.tender_id AND a.seq=v.seq) THEN 1 ELSE 0 END),
             SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_dates d WHERE d.tender_id=v.tender_id AND d.seq=v.seq) THEN 1 ELSE 0 END),
             SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_parties p WHERE p.tender_id=v.tender_id AND p.seq=v.seq AND p.role LIKE '%uyer%') THEN 1 ELSE 0 END)
           FROM notices n JOIN tender_versions v ON v.caused_by_notice_id=n.id
            WHERE $COHORT AND n.parse_state='parsed'" \
    "exact cohort rates: versions/title/nuts/amount/date/buyer (source: 100/98.1/~35-45/~47.7/100 %)"
  hstep "H6 (exact lot-kind split)"
  info H6 "SELECT vl.kind, COUNT(*) FROM notices n
             JOIN tender_versions v ON v.caused_by_notice_id=n.id
             JOIN tender_version_lots vl ON vl.tender_id=v.tender_id AND vl.seq=v.seq
            WHERE $COHORT GROUP BY vl.kind" "exact lot-kind split across the cohort"
else
  report EYE H0 "exhaustive checks skipped — set TDB_SNAPSHOT=/path/post-refold.db to run them (they are the actual acceptance wording of issue 85)"
fi

echo
echo "== CORE (drives the exit code): $PASS passed, $FAIL failed ($HARDFAIL hard-fail), $EYE eyeball =="
echo "== CONSERVATION (never blocks): $CONS_OK confirmed, $CONS_UNK unverified =="
echo "Named spot-checks — run ./de1x_spotcheck.sh"
if [ "$HARDFAIL" -ne 0 ]; then
  echo
  echo "NO-GO — a core gate failed: the DE-1.x cohort did not fold correctly. Hold nginx."
  exit 1
fi
echo
if [ "$CONS_UNK" -gt 0 ]; then
  echo "PASS (core) — facts land, no double-count, parse layer intact. $CONS_UNK conservation"
  echo "gate(s) UNVERIFIED against a stale baseline; report says unverified, not verified."
else
  echo "PASS — core gates and conservation arithmetic both hold."
fi
exit 0
