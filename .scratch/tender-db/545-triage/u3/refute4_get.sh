#!/bin/bash
# refute_4: one public GET per path, under the shared flock (.get.lock), waiting >=2 s after the last line of the
# shared api_requests.log AND sleeping 2 s; logged to api_requests.log and refute4_gets.log; body saved under bodies/;
# a saved body is reused. A 408/5xx is retried at most once.
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
mkdir -p "$U3/bodies"
for P in "$@"; do
  F="$U3/bodies/$(echo "$P" | sed 's#^/##; s#/#_#g').json"
  if [ -s "$F" ]; then echo "REUSE $F"; continue; fi
  tries=0
  while :; do
    RC=$(
      (
        flock 9
        while :; do
          L=$(tail -1 "$U3/api_requests.log" 2>/dev/null | cut -d' ' -f1)
          [ -z "$L" ] && break
          D=$(( $(date -u +%s) - $(date -u -d "$L" +%s) ))
          [ "$D" -ge 2 ] && break
          sleep 1
        done
        sleep 2
        RC=$(curl -s --max-time 30 -o "$F.tmp" -w '%{http_code}' "https://tenders.zebreus.click$P")
        echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $P rc=$RC" >> "$U3/api_requests.log"
        echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $P rc=$RC" >> "$U3/refute4_gets.log"
        echo "$RC"
      ) 9>"$U3/.get.lock"
    )
    if [ "$RC" = "200" ]; then mv "$F.tmp" "$F"; echo "ok $P"; break; fi
    tries=$((tries+1))
    case "$RC" in 408|5??) if [ $tries -lt 2 ]; then echo "retry once $RC $P"; continue; fi;; esac
    mv "$F.tmp" "$F.rc$RC" 2>/dev/null; echo "FAIL rc=$RC $P"; break
  done
done
