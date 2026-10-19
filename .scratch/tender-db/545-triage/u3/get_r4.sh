#!/bin/bash
# reviewer 4: usage get_r4.sh /v1/path ... -- one public GET per path, under the shared flock (.get.lock),
# waiting >=2 s after the last line of the shared api_requests.log AND sleeping 2 s; logged; body saved under bodies/;
# a saved body is reused. No retries (a 408/5xx is recorded and left).
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
mkdir -p "$U3/bodies"
for P in "$@"; do
  F="$U3/bodies/$(echo "$P" | sed 's#^/##; s#/#_#g').json"
  if [ -s "$F" ]; then echo "REUSE $F"; continue; fi
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
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $P rc=$RC" >> "$U3/r4_gets.log"
    if [ "$RC" = "200" ]; then mv "$F.tmp" "$F"; echo "ok $P"; else mv "$F.tmp" "$F.rc$RC"; echo "FAIL rc=$RC $P"; fi
  ) 9>"$U3/.get.lock"
done
