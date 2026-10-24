#!/bin/bash
# fget.sh <api-path> <outfile> : finalizer's serial public GET; continues the workflow's global count
# (u584c/get.count). sleep 3 before, never retried, stop flag on 408/429/5xx, global cap 40.
set -u
P="$1"; OUT="$2"
D=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u584c
LOG=/home/user/tender-db/.scratch/tender-db/584-u2/requests.log
CAP=40
exec 9>"$D/get.lock"; flock 9
if [ -f "$D/get.stop" ]; then echo "REFUSED: stop flag: $(cat $D/get.stop)"; exit 3; fi
n=$(cat "$D/get.count")
if [ "$n" -ge "$CAP" ]; then echo "REFUSED: cap $CAP reached"; exit 3; fi
sleep 3
code=$(curl -sS -o "$OUT" -w '%{http_code}' --max-time 60 "https://tenders.zebreus.click$P")
rc=$?
n=$((n+1)); echo "$n" > "$D/get.count"
sz=$(stat -c %s "$OUT" 2>/dev/null || echo 0)
echo "$(date -u +%FT%TZ) GET #$n (finalizer) $P -> $code rc=$rc bytes=$sz" >> "$LOG"
case "$code" in 408|429|5*) echo "$code on $P" > "$D/get.stop";; esac
echo "$code $sz"
