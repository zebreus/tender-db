#!/bin/bash
# get.sh <api-path> <outfile> : one serial GET, >= 2 s after the previous one (flock + timestamp file).
set -u
P="$1"; OUT="$2"
D=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u4bin
LOG=/home/user/tender-db/.scratch/tender-db/576-u4/raw/gets.log
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
echo "$(date -u +%FT%TZ) GET $P -> $code rc=$rc bytes=$sz start=$start" >> "$LOG"
echo "$code $sz"
