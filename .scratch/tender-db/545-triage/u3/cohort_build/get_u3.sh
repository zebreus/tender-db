#!/bin/bash
# unit-3 cohort writer: sequential public GETs, >=2s apart (also after the shared log's last line), logged, cached under bodies/; at most one retry for 5xx, none for 408
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
LOG="$U3/api_requests.log"
mkdir -p "$U3/bodies"
for p in "$@"; do
  f="$U3/bodies/$(echo "$p" | sed 's#^/##; s#/#_#g').json"
  if [ -s "$f" ]; then echo "cached $f"; continue; fi
  tries=0
  while :; do
    sleep 2
    while :; do
      last=$(tail -1 "$LOG" | cut -d' ' -f1)
      lt=$(date -u -d "$last" +%s 2>/dev/null || echo 0)
      now=$(date -u +%s)
      [ $((now - lt)) -ge 2 ] && break
      sleep 1
    done
    code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
    line="$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $p rc=$code"
    echo "$line" >> "$LOG"
    echo "$line" >> "$U3/cohort_build/u3_gets.log"
    if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $p"; break; fi
    tries=$((tries+1))
    case "$code" in 408) mv "$f.tmp" "$f.err.$code"; echo "FAIL 408 (no retry) $p"; break;; 5??) if [ $tries -lt 2 ]; then echo "retry once $code $p"; continue; fi;; esac
    mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; break
  done
done
