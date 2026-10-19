#!/bin/bash
# reviewer 6: sequential public GETs, >=2s apart, logged, cached under bodies/; no retries beyond one for 408/5xx
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
mkdir -p "$U3/bodies"
for p in "$@"; do
  f="$U3/bodies/$(echo "$p" | sed 's#^/##; s#/#_#g').json"
  if [ -s "$f" ]; then echo "cached $f"; continue; fi
  tries=0
  while :; do
    sleep 2
    code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $p rc=$code" >> "$U3/api_requests.log"
    if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $p"; break; fi
    tries=$((tries+1))
    case "$code" in 408|5??) if [ $tries -lt 2 ]; then echo "retry once $code $p"; continue; fi;; esac
    mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; break
  done
done
