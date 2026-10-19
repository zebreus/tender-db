#!/bin/bash
# usage: get1.sh /v1/path   -- one public GET, rate-limited, logged, body cached under bodies/
set -u
U3=/home/user/tender-db/.scratch/tender-db/545-triage/u3
p="$1"
f="$U3/bodies/$(echo "$p" | sed 's#^/##; s#/#_#g').json"
if [ -s "$f" ]; then echo "cached $f"; exit 0; fi
sleep 2
code=$(curl -s --max-time 30 -o "$f.tmp" -w '%{http_code}' "https://tenders.zebreus.click$p")
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) GET $p rc=$code" >> "$U3/api_requests.log"
if [ "$code" = "200" ]; then mv "$f.tmp" "$f"; echo "ok $f"; else mv "$f.tmp" "$f.err.$code"; echo "FAIL $code $p"; fi
