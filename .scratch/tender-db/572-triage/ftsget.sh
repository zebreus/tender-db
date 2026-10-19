#!/bin/bash
# usage: ftsget.sh <publication_id> — the published OCDS release package from Find a Tender itself
# (https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/<id>, public, OGL v3.0), the way
# issue 519's m7 read it: to see the source's own party roles (buyer vs procuringEntity) and
# details.classifications, which the FTS parser does not carry. >= 2 s apart, logged to requests.log
# ('fts' lines), cached under fts/. Refuses after 15. No retry. Not the production box.
set -u
d=/home/user/tender-db/.scratch/tender-db/572-triage
p="$1"
mkdir -p "$d/fts"
f="$d/fts/$p.json"
if [ -s "$f" ]; then echo "cached $f"; exit 0; fi
used=$(grep -c ' fts GET ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 15 ]; then echo "REFUSED: $used FTS GETs already used (cap 15)"; exit 3; fi
sleep 2.2
code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/$p")
echo "$(date -u +%FT%TZ) fts GET /api/1.0/ocdsReleasePackages/$p rc=$code" >> "$d/requests.log"
if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
