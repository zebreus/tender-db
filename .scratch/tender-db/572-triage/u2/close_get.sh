#!/bin/bash
# usage: cget.sh <tag> <path> — one public GET for 572 unit 2's close; >= 6 s after the previous one (another agent reads too);
# logged to 572-triage/requests.log "(572 u2 close <tag>)"; body saved to u2/close/; cap 21 close GETs; never retries.
set -u
d=/home/user/tender-db/.scratch/tender-db/572-triage
tag="$1"; p="$2"
f="$d/u2/close/$(echo "$p" | sed 's#^/##; s#[/?&=%]#_#g; s#^$#root#').body"
used=$(grep -c ' api GET .*(572 u2 close' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 21 ]; then echo "REFUSED: $used close GETs used (cap 21)"; exit 3; fi
last=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/bin/.last_get
if [ -s "$last" ]; then
  now=$(date +%s.%N); prev=$(cat "$last"); wait=$(echo "6.2 - ($now - $prev)" | bc)
  if [ "$(echo "$wait > 0" | bc)" = 1 ]; then sleep "$wait"; fi
fi
date +%s.%N > "$last"
code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
echo "$(date -u +%FT%TZ) api GET $p rc=$code (572 u2 close $tag)" >> "$d/requests.log"
if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $code $p -> $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
