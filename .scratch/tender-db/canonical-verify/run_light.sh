#!/usr/bin/env bash
# LIGHT canonical-layer checks against the LIVE service via POST /v1/sql.
# Read-only. Re-runnable. Exits nonzero if any HARD-FAIL gate misses.
#
#   BASE_URL   default https://tenders.zebreus.click
#   TDB_TOKEN  an API token (tdb_…) created on the dashboard (required)
#
#   TDB_TOKEN=tdb_xxx ./run_light.sh
#
# Only the [LIGHT] checks run here (bounded — no turso hash cliff). The [HEAVY]
# checks are in heavy.sql (stock sqlite3, service stopped). Needs: bash, curl, jq.
set -uo pipefail

BASE_URL="${BASE_URL:-https://tenders.zebreus.click}"
: "${TDB_TOKEN:?set TDB_TOKEN to an API token (tdb_…) from the dashboard}"

PASS=0; FAIL=0; EYE=0; HARDFAIL=0

# q "<sql>" -> prints rows as TSV (one row per line, cells tab-separated), or the
# HTTP/engine error to stderr and nothing to stdout.
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

scalar() { q "$1" | head -1 | cut -f1; }   # first cell of first row

report() { # report <PASS|FAIL|EYE> <id> <text>
  case "$1" in
    PASS) PASS=$((PASS+1)); printf '  \033[32mPASS\033[0m %-5s %s\n' "$2" "$3";;
    FAIL) FAIL=$((FAIL+1)); printf '  \033[31mFAIL\033[0m %-5s %s\n' "$2" "$3";;
    EYE)  EYE=$((EYE+1));   printf '  \033[36mEYE \033[0m %-5s %s\n' "$2" "$3";;
  esac
}

# eq <id> <sql> <expected> <hard|soft> <label>
eq() {
  local got; got=$(scalar "$2")
  if [ "$got" = "$3" ]; then report PASS "$1" "$5 = $got"
  else report FAIL "$1" "$5: got '$got', expected '$3'"; [ "$4" = hard ] && HARDFAIL=$((HARDFAIL+1)); fi
}
# zero <id> <sql> <hard|soft> <label>  (expect 0 violations)
zero() { eq "$1" "$2" 0 "$3" "$4"; }
# ge <id> <sql> <min> <hard|soft> <label>
ge() {
  local got; got=$(scalar "$2")
  if [ -n "$got" ] && [ "$got" -ge "$3" ] 2>/dev/null; then report PASS "$1" "$5 = $got (≥ $3)"
  else report FAIL "$1" "$5: got '$got', expected ≥ $3"; [ "$4" = hard ] && HARDFAIL=$((HARDFAIL+1)); fi
}
# subset <id> <sql> "<allowed space-separated>" <hard|soft> <label>  (col0 of each row ∈ allowed)
# Split with parameter expansion, not `read -r v _`: tab is IFS-whitespace, so read
# would TRIM a leading empty field and a NULL first column (e.g. identifier_kind)
# would wrongly read as its count. `${line%%<tab>*}` preserves the empty field.
subset() {
  local bad="" line v
  while IFS= read -r line; do
    v="${line%%$'\t'*}"
    [ -z "$v" ] && v="(null)"
    case " $3 " in *" $v "*) ;; *) bad="$bad $v";; esac
  done < <(q "$2")
  if [ -z "$bad" ]; then report PASS "$1" "$5 ⊆ {$3}"
  else report FAIL "$1" "$5: unexpected value(s):$bad"; [ "$4" = hard ] && HARDFAIL=$((HARDFAIL+1)); fi
}
# info <id> <sql> <label>  (eyeball — prints the row(s), never fails)
info() { report EYE "$1" "$3: $(q "$2" | tr '\t' '/' | tr '\n' ' ')"; }

echo "== LIGHT canonical-layer checks  ($BASE_URL)  $(date -u +%FT%TZ) =="

echo "-- 1. COUNT invariants"
eq   1.1 "SELECT COUNT(*) FROM tenders" 6961311 hard "tenders"
eq   1.2 "SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL" 640745 hard "islands"
eq   1.3 "SELECT COUNT(*) FROM tenders WHERE procedure_key IS NOT NULL" 6320566 hard "keyed"
zero 1.4 "SELECT COUNT(*) FROM tenders WHERE (procedure_key IS NULL) = (island_notice_id IS NULL)" hard "identity mutual-exclusivity"
zero 1.5 "SELECT COUNT(*) FROM tenders WHERE current_seq IS NULL" hard "every tender has a head version"
subset 1.6 "SELECT kind, COUNT(*) FROM tenders GROUP BY kind" "procedure registration" hard "kind"
subset 1.7 "SELECT source, COUNT(*) FROM tenders GROUP BY source" "ted doe" hard "source"
ge   1.8 "SELECT COUNT(*) FROM tender_versions" 6961311 hard "versions ≥ tenders"
info 1.9 "SELECT (SELECT COUNT(*) FROM organizations), (SELECT COUNT(*) FROM organization_mentions)" "orgs / mentions"
info 1.10 "SELECT provisional, COUNT(*) FROM organizations GROUP BY provisional" "provisional split"

echo "-- 2. Domain invariants"
zero 2.1 "SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL AND current_seq <> 1" hard "islands single-notice"
eq   2.2 "SELECT MIN(seq) FROM tender_versions" 1 hard "seq dense from 1 (min_seq)"
zero 2.6 "SELECT COUNT(*) FROM organizations WHERE (provisional=1) <> (identifier IS NULL)" hard "provisional ⟺ no identifier"
subset 2.7 "SELECT identifier_kind, COUNT(*) FROM organizations GROUP BY identifier_kind" "(null) vat national" hard "identifier_kind"
subset 2.9a "SELECT op, COUNT(*) FROM changes GROUP BY op" "added changed removed" hard "changes.op"
subset 2.9b "SELECT entity_kind, COUNT(*) FROM changes GROUP BY entity_kind" "tender lot organization lot_result bid contract" hard "changes.entity_kind"

echo "-- 3. Weirdness (eyeball) + hard sanity"
info 3.1 "SELECT (SELECT COUNT(*) FROM tenders WHERE current_seq>50),(SELECT COUNT(*) FROM tenders WHERE current_seq>100),(SELECT COUNT(*) FROM tenders WHERE current_seq>500),(SELECT COUNT(*) FROM tenders WHERE current_seq>1000)" "mega-tail >50/100/500/1000"
info 3.3 "SELECT (SELECT COUNT(*) FROM tenders WHERE current_seq=1),(SELECT COUNT(*) FROM tenders WHERE current_seq=2),(SELECT COUNT(*) FROM tenders WHERE current_seq BETWEEN 3 AND 5),(SELECT COUNT(*) FROM tenders WHERE current_seq BETWEEN 6 AND 10),(SELECT COUNT(*) FROM tenders WHERE current_seq BETWEEN 11 AND 50),(SELECT COUNT(*) FROM tenders WHERE current_seq>50)" "notices/tender 1/2/3-5/6-10/11-50/>50"
zero 3.6 "SELECT COUNT(*) FROM tender_versions WHERE published_at < 631152000 OR published_at > 1800000000" hard "no absurd publication dates"
# Issue 33: negatives are legitimate on `result_value` (award/result adjustments —
# 17,687 of 17,738 measured, 99.6% between EUR 1 and 100) and implausible on an
# estimated value or a framework maximum. Written as an ALLOW-LIST (`<> 'result_value'`)
# rather than a deny-list of the two forbidden names, so a future fourth amount field
# is forbidden by default: the check fails closed on a name nobody has thought about yet.
zero 3.7 "SELECT COUNT(*) FROM tender_version_amounts WHERE cents < 0 AND field <> 'result_value'" hard "no negative amounts outside result_value"
info 3.8 "SELECT (SELECT COUNT(*) FROM lot_results),(SELECT COUNT(*) FROM bids),(SELECT COUNT(*) FROM contracts)" "results lot_results/bids/contracts (all should be > 0)"

echo "-- 4. changes feed"
info 4.1 "SELECT MIN(cursor), MAX(cursor), COUNT(*) FROM changes" "cursor min/max/rows"
info 4.3 "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='added'" "tender/added (≥6.96M incl. harmless fallback prefix)"

echo "-- 6. Incremental watermark / post-plan suffix"
info 6.1 "SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=0" "unprojected suffix (≈14k, waiting for incremental)"
eq   6.2 "SELECT CASE WHEN (SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=0) = (SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND id >= (SELECT MIN(id) FROM notices WHERE parse_state='parsed' AND projected=0)) THEN 1 ELSE 0 END" 1 hard "suffix is a clean contiguous tail (nothing dropped mid-corpus)"

echo
echo "== $PASS passed, $FAIL failed ($HARDFAIL hard-fail), $EYE to eyeball =="
[ "$HARDFAIL" -eq 0 ] || { echo "HARD-FAIL gate(s) missed — do NOT trust the layer."; exit 1; }
echo "All hard-fail gates passed. Review the EYE lines + run heavy.sql in the stop window."
