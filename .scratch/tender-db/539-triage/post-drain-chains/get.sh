#!/bin/bash
# usage: get.sh <tender_id> — one public GET /v1/tenders/<id>, raw body to t<id>.json, one line in requests.log.
# Refuses after 6 logged GETs; spaces GETs >= 3 s apart; never retries.
set -u
d=/home/user/tender-db/.scratch/tender-db/539-triage/post-drain-chains
id="$1"
touch "$d/requests.log"
used=$(grep -c ' get ' "$d/requests.log" || true); used=${used:-0}
[ "$used" -ge 6 ] && { echo "REFUSED: 6 GETs used"; exit 3; }
grep -qE " get /v1/tenders/$id " "$d/requests.log" && { echo "REFUSED: already fetched (no retry)"; exit 3; }
last=$(grep ' get ' "$d/requests.log" | tail -1 | cut -d' ' -f1)
if [ -n "$last" ]; then w=$(( $(date -d "$last" +%s) + 3 - $(date +%s) )); [ "$w" -gt 0 ] && sleep "$w"; fi
ts=$(date -u +%FT%TZ)
code=$(curl -sS --max-time 30 -o "$d/t$id.json" -w '%{http_code}' "https://tenders.zebreus.click/v1/tenders/$id"); rc=$?
echo "$ts get /v1/tenders/$id rc=$rc http=$code bytes=$(wc -c < "$d/t$id.json")" >> "$d/requests.log"
tail -1 "$d/requests.log"
