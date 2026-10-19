#!/bin/bash
# usage: get.sh <name> <path>  — one public API GET (https://tenders.zebreus.click<path>), body to bodies/<name>.json,
# logged to requests.log as '<UTC iso> get <name> rc=<http code> secs=<s> bytes=<n>'. Cap 10 GETs; sleeps so GETs are >= 2 s apart.
set -u
d=/home/user/tender-db/.scratch/tender-db/539-triage
used=$(grep -c ' get ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 10 ]; then echo "REFUSED: $used GETs already used (cap 10)"; exit 3; fi
sleep 2.1
start=$(date +%s.%N)
code=$(curl -s --max-time 30 -o "$d/bodies/$1.json" -w '%{http_code}' "https://tenders.zebreus.click$2")
end=$(date +%s.%N)
echo "$(date -u +%FT%TZ) get $1 rc=$code secs=$(echo "$end - $start" | bc) bytes=$(wc -c < "$d/bodies/$1.json")" >> "$d/requests.log"
tail -1 "$d/requests.log"
