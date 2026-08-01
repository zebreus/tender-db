#!/usr/bin/env bash
# ============================================================================
# V1 spot-check — two NAMED eForms-DE 1.x tenders, rendered end to end.
# READ-ONLY (SELECTs + GETs). Companion to de1x_verify.sh (the statistical half).
#
#   TDB_TOKEN=tdb_… [BASE_URL=http://127.0.0.1:8080] ./de1x_spotcheck.sh
#
# Both notices are the real archive payloads vendored as test fixtures for
# issue 75 (crates/ingest/tests/fixtures/doe/), so every expected value below is
# read off the publisher's own XML — not off the projection being tested.
#
#   A) eForms-DE 1.1 contract notice   subtype 16
#      notice        7d69b0f7-2605-448f-9495-676458dcddc2  (publication id …-01)
#      folder/key    990acec0-88c0-48e2-86d5-92abb2293542   <- uuid: passes is_uuid,
#                                                              so this Tender is KEYED
#      title         "Erschließung Starkstrom"
#      lot           LOT-0000 (kind Lot)          CPV 45310000    NUTS DED2D
#      deadline      2024-04-02 (+02:00)
#      buyer         "Städtisches Klinikum Görlitz gGmbH"
#      review body   "1. Vergabekammer des Freistaates Sachsen"
#
#   B) eForms-DE 1.2 contract award notice   subtype 38
#      notice        799811c4-2454-414e-b3cc-1d5d55c69690  (publication id …-01)
#      folder/key    469efb81-40de-4d7e-817f-8dd45659eaee
#      title         "NA 26a+b Dachabdichtungs- und Spenglerarbeiten"
#      lot           LOT-0001                     CPV 45000000/45261410  NUTS DE212
#      awarded       73332.89 EUR  (= 7333289 cents)
#      buyer         "MRG Münchner Raumentwicklungsgesellschaft mbH"
#      winner        "Gebrüder Schneller GmbH & Co. KG"
#
# NOT USED: /v1/tenders/{id} and /v1/notices?tender= — both route through
# read::tender_detail, which full-scans tender_version_bid_parties on prod rev
# 484e9204 (issue 89; fix a572544 undeployed). A single tender row is fetched
# from the LIST endpoint instead, via `?cursor=<id-1>&limit=1` (the cursor is
# just the last row id), and its lots via /v1/lots?tender=<id>.
# ============================================================================
set -uo pipefail
BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"

# Same dual read path as de1x_verify.sh: TDB_SNAPSHOT=/path/post-refold.db reads
# an immutable snapshot with stock sqlite3 (no token, no live contention), else
# TDB_TOKEN drives POST /v1/sql. The REST rendering half needs the LIVE app and
# is skipped in snapshot mode — set BOTH to get the fact tables from the snapshot
# and the rendering from the app.
TAB=$(printf '\t')
if [ -n "${TDB_SNAPSHOT:-}" ]; then
  [ -r "$TDB_SNAPSHOT" ] || { echo "TDB_SNAPSHOT=$TDB_SNAPSHOT is not readable" >&2; exit 2; }
  [ -e "${TDB_SNAPSHOT}-wal" ] && { echo "REFUSING: ${TDB_SNAPSHOT}-wal exists — immutable=1 would read stale data." >&2; exit 2; }
  command -v sqlite3 >/dev/null || { echo "sqlite3 not found (try: nix shell nixpkgs#sqlite --command …)" >&2; exit 2; }
  q() {
    local out
    if ! out=$(sqlite3 -readonly -noheader -separator "$TAB" \
                 "file:${TDB_SNAPSHOT}?immutable=1" "$1" 2>&1); then
      echo "  sqlite3: $(printf '%s' "$out" | head -1)" >&2; return 1
    fi
    printf '%s\n' "$out"
  }
else
  : "${TDB_TOKEN:?set TDB_TOKEN (live /v1/sql) or TDB_SNAPSHOT (a post-refold snapshot file)}"
  q() {
    local resp
    resp=$(curl -sS --max-time 60 -X POST "$BASE_URL/v1/sql" \
      -H "Authorization: Bearer $TDB_TOKEN" -H "Content-Type: text/plain" --data-binary "$1") \
      || { echo "  curl failed" >&2; return 1; }
    echo "$resp" | jq -e '.rows' >/dev/null 2>&1 \
      || { echo "  server: $(echo "$resp" | jq -r '.error // .' | head -1)" >&2; return 1; }
    echo "$resp" | jq -r '.rows[] | @tsv'
  }
fi
scalar() { q "$1" | head -1 | cut -f1; }
show()   { printf '\n  \033[1m%s\033[0m\n' "$1"; q "$2" | sed 's/^/    /'; }

check() { # $1 label  $2 publication_id  $3 procedure uuid
  echo; echo "=============================================================="
  echo "== $1"
  echo "=============================================================="
  local nid tid seq
  nid=$(scalar "SELECT id FROM notices WHERE source='doe' AND publication_id='$2'")
  if [ -z "$nid" ]; then
    echo "  !! notice '$2' not found (seeks the UNIQUE(source,publication_id,…) index)"
    echo "     fallback: the Tender is still reachable by its procedure key below."
  else
    show "notice (parse layer)" \
      "SELECT id, publication_id, profile, parse_state, projected, published_at, dispatched_at
         FROM notices WHERE id=$nid"
  fi

  # Identity: the uuid folder id must have keyed a Tender (the is_uuid gate,
  # commit 029d2a7). Pre-fix the whole cohort was keyless islands.
  show "tender by procedure key (is_uuid gate worked ⟺ this returns a row)" \
    "SELECT id, source, procedure_key, kind, current_seq, current_published_at
       FROM tenders WHERE procedure_key='$3'"
  tid=$(scalar "SELECT id FROM tenders WHERE procedure_key='$3'")
  if [ -z "$tid" ] && [ -n "$nid" ]; then
    tid=$(scalar "SELECT tender_id FROM tender_versions WHERE caused_by_notice_id=$nid")
    echo "    (no keyed Tender — still an island; tender_id via the notice = ${tid:-none})"
  fi
  [ -z "$tid" ] && { echo "  !! no Tender at all — the fold did not land this notice"; return; }
  seq=$(scalar "SELECT current_seq FROM tenders WHERE id=$tid")

  show "versions (ADR-0001 chain)" \
    "SELECT seq, caused_by_notice_id, published_at, dispatched_at, notice_subtype, publication_id
       FROM tender_versions WHERE tender_id=$tid ORDER BY seq"
  show "texts — German title/description, tender + lot scope" \
    "SELECT field, lang, lot_id, substr(value,1,90)
       FROM tender_version_texts WHERE tender_id=$tid AND seq=$seq ORDER BY field, lot_id"
  show "classifications — CPV + NUTS" \
    "SELECT scheme, field, lot_id, code
       FROM tender_version_classifications WHERE tender_id=$tid AND seq=$seq ORDER BY scheme, code"
  show "amounts (cents + currency)" \
    "SELECT field, lot_id, cents, currency
       FROM tender_version_amounts WHERE tender_id=$tid AND seq=$seq"
  show "dates (utc_seconds + offset_minutes + has_time)" \
    "SELECT field, lot_id, utc_seconds, offset_minutes, has_time, datetime(utc_seconds,'unixepoch')
       FROM tender_version_dates WHERE tender_id=$tid AND seq=$seq"
  show "LOTS — the Lot/LotsGroup/Part fix (kind read back from the LOT-/GLO-/PAR- id)" \
    "SELECT vl.kind, l.lot_key, l.id
       FROM tender_version_lots vl JOIN lots l ON l.id=vl.lot_id
      WHERE vl.tender_id=$tid AND vl.seq=$seq ORDER BY l.lot_key"
  show "parties — role → canonical Organization (buyer must resolve to its org)" \
    "SELECT p.role, o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional
       FROM tender_version_parties p JOIN organizations o ON o.id=p.organization_id
      WHERE p.tender_id=$tid AND p.seq=$seq ORDER BY p.role"
  show "award results + winner (CAN only)" \
    "SELECT r.result_key, s.decision, s.awarded_cents, s.awarded_currency, o.name
       FROM lot_results r
       JOIN tender_version_lot_results s ON s.tender_id=r.tender_id AND s.seq=$seq AND s.lot_result_id=r.id
       LEFT JOIN tender_version_result_winners w ON w.tender_id=r.tender_id AND w.seq=$seq AND w.lot_result_id=r.id
       LEFT JOIN organizations o ON o.id=w.organization_id
      WHERE r.tender_id=$tid"

  # The strongest single assertion: the canonical title is byte-identical to the
  # value the publisher put in DE1-ProcurementProject-Name. Both sides are PK
  # seeks; nothing here can pass on a coincidence.
  if [ -n "$nid" ]; then
    show "parse ↔ canonical equality (DE1-ProcurementProject-Name vs title)" \
      "SELECT (SELECT substr(value,1,90) FROM notice_texts
                WHERE notice_id=$nid AND field_id='DE1-ProcurementProject-Name' AND ordinal=0),
              (SELECT substr(value,1,90) FROM tender_version_texts
                WHERE tender_id=$tid AND seq=$seq AND field='title' AND lot_id IS NULL LIMIT 1)"
    show "parse ↔ canonical lot parity (sections vs projected lots)" \
      "SELECT (SELECT COUNT(*) FROM notice_sections WHERE notice_id=$nid AND kind='ProcurementProjectLot'),
              (SELECT COUNT(*) FROM tender_version_lots WHERE tender_id=$tid AND seq=$seq)"
    show "parse-layer buyer name (DE1-Organizations-…-PartyName-Name) — compare with parties above" \
      "SELECT section_id, substr(value,1,60) FROM notice_texts
        WHERE notice_id=$nid AND field_id='DE1-Organizations-Organization-Company-PartyName-Name'
        ORDER BY section_id LIMIT 6"
  fi

  # ---- REST rendering, unauthenticated, detail endpoint avoided -------------
  if [ -n "${TDB_SNAPSHOT:-}" ] && [ -z "${TDB_TOKEN:-}" ]; then
    echo; echo "  -- REST rendering skipped (snapshot mode; needs the live app) --"
    return
  fi
  echo; echo "  -- REST (what a user sees) --"
  echo "  GET /v1/tenders?cursor=$((tid-1))&limit=1"
  curl -sS --max-time 30 "$BASE_URL/v1/tenders?cursor=$((tid-1))&limit=1" \
    | jq -C '.items[0] | {id,source,procedure_key,kind,title,version,published_at,notice_subtype,value,submission_deadline,lots,cpv,country}' 2>/dev/null | sed 's/^/    /'
  echo "  GET /v1/lots?tender=$tid&limit=20"
  curl -sS --max-time 30 "$BASE_URL/v1/lots?tender=$tid&limit=20" \
    | jq -C '.items[] | {id,tender_id,lot_key,kind,title,value,submission_deadline}' 2>/dev/null | sed 's/^/    /'
  local org
  org=$(scalar "SELECT organization_id FROM tender_version_parties
                 WHERE tender_id=$tid AND seq=$seq AND role LIKE '%uyer%' LIMIT 1")
  if [ -n "$org" ]; then
    echo "  GET /v1/organizations/$org   (buyer)"
    curl -sS --max-time 30 "$BASE_URL/v1/organizations/$org" | jq -C . 2>/dev/null | sed 's/^/    /'
  fi
  [ -n "$nid" ] && { echo "  GET /v1/notices/$nid"; curl -sS --max-time 30 "$BASE_URL/v1/notices/$nid" | jq -C . 2>/dev/null | sed 's/^/    /'; }
}

check "A) eForms-DE 1.1 CN — Erschließung Starkstrom / Städtisches Klinikum Görlitz gGmbH" \
      "7d69b0f7-2605-448f-9495-676458dcddc2-01" "990acec0-88c0-48e2-86d5-92abb2293542"
check "B) eForms-DE 1.2 CAN — NA 26a+b Dachabdichtungs- und Spenglerarbeiten / Gebrüder Schneller" \
      "799811c4-2454-414e-b3cc-1d5d55c69690-01" "469efb81-40de-4d7e-817f-8dd45659eaee"

# ---- filter cross-check: the new facts must be reachable through the API ----
echo; echo "=============================================================="
echo "== C) the folded facts are FILTERABLE (not just stored)"
echo "=============================================================="
echo "  GET /v1/tenders?country=DE&cpv=4531&limit=3   (NUTS DE* + CPV 4531* — CN's codes)"
curl -sS --max-time 60 "$BASE_URL/v1/tenders?country=DE&cpv=4531&limit=3" \
  | jq -C '.items[] | {id,source,title,cpv,country,lots,value}' 2>/dev/null | sed 's/^/    /'
echo "  GET /v1/notices?kind=eforms:eforms-de-1.1&limit=3   (cohort is addressable by profile)"
curl -sS --max-time 30 "$BASE_URL/v1/notices?kind=eforms:eforms-de-1.1&limit=3" \
  | jq -C '.items[] | {id,publication_id,profile,parse_state,published_at}' 2>/dev/null | sed 's/^/    /'
echo
echo "Dashboard cross-check (browser, no token): the quarantine 'Resolved' section must show"
echo "eForms-DE 1.x at 218,635 resolved / 241 held — the ledger claim that only becomes true"
echo "once this fold lands (crates/app/data/quarantine-ledger.json, resolved 2026-07-29)."
