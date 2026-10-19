#!/bin/bash
# usage: get_amend.sh (get.sh, log lines tagged "(572 amend)") /v1/path — one public GET on https://tenders.zebreus.click, >= 2 s after the previous one,
# logged to requests.log ('api' lines), body cached under bodies/. Refuses after 150 logged GETs. No retry.
set -u
d=/home/user/tender-db/.scratch/tender-db/572-triage
p="$1"
mkdir -p "$d/bodies"
f="$d/bodies/$(echo "$p" | sed 's#^/##; s#[/?&=]#_#g').json"
if [ -s "$f" ]; then echo "cached $f"; exit 0; fi
used=$(grep -c ' api GET ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 150 ]; then echo "REFUSED: $used public GETs already used (cap 150)"; exit 3; fi
sleep 2.2
code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
echo "$(date -u +%FT%TZ) api GET $p rc=$code (572 amend)" >> "$d/requests.log"
if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
