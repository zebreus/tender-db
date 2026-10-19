#!/bin/bash
# reviewer 2: usage get_r2.sh /v1/path ... -- one public GET per path, >=2 s apart (sleep 2 under a shared flock),
# logged to api_requests.log, body saved under bodies/, a saved body is reused. A 408/5xx is retried at most once (here: never).
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
for P in "$@"; do
  F="$U3/bodies/$(echo "$P" | sed 's#^/##; s#/#_#g').json"
  if [ -s "$F" ]; then echo "REUSE $F"; continue; fi
  (
    flock 9
    sleep 2
    RC=$(curl -s --max-time 30 -o "$F.tmp" -w '%{http_code}' "https://tenders.zebreus.click$P")
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $P rc=$RC" >> "$U3/api_requests.log"
    if [ "$RC" = "200" ]; then mv "$F.tmp" "$F"; echo "ok $P"; else mv "$F.tmp" "$F.rc$RC"; echo "FAIL rc=$RC $P"; fi
  ) 9>"$U3/.get.lock"
done
