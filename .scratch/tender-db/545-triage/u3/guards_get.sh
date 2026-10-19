#!/bin/bash
# Unit 3 guards review: one public GET per call. Reuses any saved body (bodies/, shared with the sibling reviewer).
# Spacing: waits until >=3 s after the LAST line of the shared api_requests.log (any agent), then GETs once.
# Cap: 20 GETs of this review (counted in guards_gets.log). No retries at all (never a 408, never a 5xx).
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
P="$1"
F="$U3/bodies/$(echo "$P" | sed 's#^/##; s#/#_#g').json"
if [ -s "$F" ]; then echo "REUSE $F"; exit 0; fi
N=$(cat "$U3/guards_gets.log" 2>/dev/null | wc -l)
if [ "$N" -ge 20 ]; then echo "REFUSED: 20 guards GETs used"; exit 3; fi
while :; do
  L=$(tail -1 "$U3/api_requests.log" 2>/dev/null | cut -d' ' -f1)
  if [ -z "$L" ]; then break; fi
  D=$(( $(date -u +%s) - $(date -u -d "$L" +%s) ))
  if [ "$D" -ge 3 ]; then break; fi
  sleep 1
done
RC=$(curl -s --max-time 30 -o "$F.tmp" -w '%{http_code}' "https://tenders.zebreus.click$P")
TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "$TS GET $P rc=$RC" >> "$U3/api_requests.log"
echo "$TS GET $P rc=$RC" >> "$U3/guards_gets.log"
if [ "$RC" = "200" ]; then mv "$F.tmp" "$F"; else mv "$F.tmp" "$F.err.$RC"; fi
echo "rc=$RC -> $F"
