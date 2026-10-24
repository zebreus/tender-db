#!/bin/bash
# usage: get.sh <name> <path-and-query>; serial, >= 2.5 s after the previous GET (api/last), body to api/raw/<name>.json
set -u
D=/home/user/tender-db/.scratch/tender-db/audit-576-fanout/second-look/api
last=$(cat $D/last 2>/dev/null || echo 0)
now=$(date +%s.%N)
w=$(python3 -c "print(max(0.0, $last + 2.5 - $now))")
python3 -c "import time; time.sleep($w)"
code=$(curl -sS --max-time 60 -o "$D/raw/$1.json" -w '%{http_code}' "https://tenders.zebreus.click$2")
date +%s.%N > $D/last
echo "$(date -u +%FT%TZ) $code $2" >> $D/gets.log
echo "HTTP $code $2"
