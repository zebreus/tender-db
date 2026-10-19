#!/bin/bash
# refute2_1 (Hounslow x3): sequential public GETs, >=2s apart (also after the shared log's last line),
# logged with a "(refute2)" suffix, cached under bodies/; no retries (budget: 4 new GETs).
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
LOG="$U3/api_requests.log"
exec 9>"$U3/.get.lock"
for p in "$@"; do
  f="$U3/bodies/$(echo "$p" | sed 's#^/##; s#/#_#g; s#[?&=]#_#g').json"
  if [ -s "$f" ]; then echo "cached $f"; continue; fi
  flock 9
  sleep 2
  while :; do
    last=$(tail -1 "$LOG" | cut -d' ' -f1)
    lt=$(date -u -d "$last" +%s 2>/dev/null || echo 0)
    now=$(date -u +%s)
    [ $((now - lt)) -ge 2 ] && break
    sleep 1
  done
  code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
  line="$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $p rc=$code (refute2)"
  echo "$line" >> "$LOG"
  echo "$line" >> "$U3/refute2_1_gets.log"
  flock -u 9
  if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $p -> $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
done
