#!/bin/bash
# usage: get.sh <tag> /v1/path — one public GET on https://tenders.zebreus.click for 572 unit 2, at least 2.1 s after
# the previous logged request, logged as one line to ../requests.log tagged "(572 u2 <tag>)", body saved under
# u2/bodies/. Refuses once 75 unit-2 GETs are logged (decision.md "Read bars": <= 75 GETs for all of unit 2).
# No retry, ever: a non-200 is recorded and left.
set -u
d=/home/user/tender-db/.scratch/tender-db/572-triage
tag="$1"; p="$2"
mkdir -p "$d/u2/bodies"
f="$d/u2/bodies/$(echo "$p" | sed 's#^/##; s#[/?&=%]#_#g').json"
if [ -e "$f" ]; then echo "EXISTS (not re-read) $f"; exit 0; fi
used=$(grep -c ' api GET .*(572 u2' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 75 ]; then echo "REFUSED: $used unit-2 GETs already used (cap 75)"; exit 3; fi
# spacing: >= 2.1 s since the last GET this script made
last="$d/u2/.last_get"
if [ -s "$last" ]; then
  now=$(date +%s.%N); prev=$(cat "$last"); wait=$(echo "2.2 - ($now - $prev)" | bc)
  if [ "$(echo "$wait > 0" | bc)" = 1 ]; then sleep "$wait"; fi
fi
date +%s.%N > "$last"
code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
echo "$(date -u +%FT%TZ) api GET $p rc=$code (572 u2 $tag)" >> "$d/requests.log"
if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $code $p"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
