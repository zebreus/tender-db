#!/bin/bash
# get.sh <api-path> <outfile> : one serial public GET, >= 3 s after the previous one; never retried.
# Refuses after a 408/429/5xx (stop flag) and past the cap of 15.
set -u
P="$1"; OUT="$2"
D=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u594
LOG=/home/user/tender-db/.scratch/tender-db/594-u1/requests.log
CAP=15
exec 9>"$D/get.lock"
flock 9
if [ -f "$D/get.stop" ]; then echo "REFUSED: stop flag: $(cat $D/get.stop)"; exit 3; fi
n=$(cat "$D/get.count" 2>/dev/null || echo 0)
if [ "$n" -ge "$CAP" ]; then echo "REFUSED: cap $CAP reached"; exit 3; fi
last=$(cat "$D/get.last" 2>/dev/null || echo 0)
now=$(date +%s.%N)
wait=$(python3 -I -c "print(max(0.0, 3.0 - ($now - $last)))")
python3 -I -c "import time; time.sleep($wait)"
code=$(curl -sS -o "$OUT" -w '%{http_code}' --max-time 60 "https://tenders.zebreus.click$P")
rc=$?
date +%s.%N > "$D/get.last"
n=$((n+1)); echo "$n" > "$D/get.count"
sz=$(stat -c %s "$OUT" 2>/dev/null || echo 0)
echo "$(date -u +%FT%TZ) GET #$n $P -> $code rc=$rc bytes=$sz" >> "$LOG"
case "$code" in 408|429|5*) echo "$code on $P" > "$D/get.stop";; esac
echo "$code $sz"
