#!/bin/bash
# get.sh <api-path> <outfile> : one serial GET to https://tenders.zebreus.click, >= 2.2 s after the previous
# request's END (flock + timestamp file), so never two in flight and never more than one per 2 s.
set -u
P="$1"; OUT="$2"
D=/home/user/tender-db/.scratch/tender-db/586-u3/raw/state
LOG=/home/user/tender-db/.scratch/tender-db/586-u3/raw/gets.log
exec 9>"$D/get.lock"
flock 9
last=$(cat "$D/get.last" 2>/dev/null || echo 0)
now=$(date +%s.%N)
wait=$(python3 -I -c "print(max(0.0, 2.2 - ($now - $last)))")
python3 -I -c "import time; time.sleep($wait)"
start=$(date +%s.%N)
code=$(curl -sS -o "$OUT" -w '%{http_code}' --max-time 60 "https://tenders.zebreus.click$P")
rc=$?
end=$(date +%s.%N)
echo "$end" > "$D/get.last"
sz=$(stat -c %s "$OUT" 2>/dev/null || echo 0)
echo "$(date -u +%FT%TZ) GET $P -> $code rc=$rc bytes=$sz start=$start end=$end" >> "$LOG"
echo "$code $sz"
