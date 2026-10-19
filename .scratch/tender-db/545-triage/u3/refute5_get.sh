#!/bin/bash
# refuter 5: one public GET per path; waits >=3 s after the shared log's last line, then sleeps 2;
# logs to the shared api_requests.log and refute_5_gets.log; caches under bodies/; no retries.
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
for p in "$@"; do
  f="$U3/bodies/$(echo "$p" | sed 's#^/##; s#/#_#g').json"
  if [ -s "$f" ]; then echo "cached $f"; continue; fi
  while true; do
    last=$(tail -1 "$U3/api_requests.log" | cut -d' ' -f1)
    lt=$(date -u -d "$last" +%s 2>/dev/null || echo 0)
    now=$(date -u +%s)
    [ $((now - lt)) -ge 3 ] && break
    sleep 1
  done
  sleep 2
  code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
  line="$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $p rc=$code"
  echo "$line" >> "$U3/api_requests.log"
  echo "$line" >> "$U3/refute_5_gets.log"
  if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
done
