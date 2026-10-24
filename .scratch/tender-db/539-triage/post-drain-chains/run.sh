#!/bin/bash
# usage: run.sh <name> — runs <name>.sql on the box via /root/sq.sh (the token stays on the box), saves the raw body
# to <name>.out.json, logs one line to requests.log. Every attempt counts; NEVER retries (a 408 or transport failure is final).
# Refusals: 10 /v1/sql attempts already logged; a 408/err already logged; no local plan; a SCAN in the plan; the plan
# file older than the .sql; the name already sent; the job queue not idle (current null, queued []); < 4 s since the last.
set -u
d=/home/user/tender-db/.scratch/tender-db/539-triage/post-drain-chains
f="$1"
touch "$d/requests.log"
used=$(grep -c ' sql ' "$d/requests.log" || true); used=${used:-0}
if [ "$used" -ge 10 ]; then echo "REFUSED: $used /v1/sql attempts already used (cap 10)"; exit 3; fi
if grep -qE ' err |rc=[1-9]|"status": ?408' "$d/requests.log"; then echo "REFUSED: an error/408 is logged; stop and report"; exit 3; fi
[ -s "$d/$f.sql" ] || { echo "REFUSED: no $f.sql"; exit 4; }
[ -s "$d/$f.plan.txt" ] || { echo "REFUSED: no local plan for $f"; exit 4; }
[ "$d/$f.plan.txt" -nt "$d/$f.sql" ] || { echo "REFUSED: plan older than sql"; exit 4; }
if grep -q 'SCAN' "$d/$f.plan.txt"; then echo "REFUSED: plan has a SCAN"; grep SCAN "$d/$f.plan.txt"; exit 4; fi
grep -q '^PLAN' "$d/$f.plan.txt" || { echo "REFUSED: no turso plan lines"; exit 4; }
if grep -qE " (sql|err) $f( |$)" "$d/requests.log"; then echo "REFUSED: $f was already sent once (no retry)"; exit 3; fi
aj=$(ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=2"' 2>/dev/null)
if ! printf '%s' "$aj" | python3 -I -c "
import json,sys
j=json.loads(sys.stdin.read())
c,q=j.get('current','missing'),j.get('queued','missing')
print('jobs: current', json.dumps(c)[:200], 'queued', json.dumps(q)[:200])
sys.exit(0 if c is None and q == [] else 1)
"; then echo "$(date -u +%FT%TZ) queue-not-idle $f (not sent)" >> "$d/requests.log"; echo "REFUSED: job queue not idle (or unreadable)"; exit 7; fi
echo "$(date -u +%FT%TZ) queue-idle-before $f" >> "$d/requests.log"
last=$(grep ' sql ' "$d/requests.log" | tail -1 | cut -d' ' -f1)
if [ -n "$last" ]; then
  wait=$(( $(date -d "$last" +%s) + 5 - $(date +%s) )); [ "$wait" -gt 0 ] && sleep "$wait"
fi
scp -q "$d/$f.sql" "root@zebreus.click:/root/q539c_$f.sql" || { echo "$(date -u +%FT%TZ) scp-fail $f (not sent)" >> "$d/requests.log"; echo SCP-FAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/q539c_$f.sql" > "$d/$f.out.json"
rc=$?
end=$(date +%s.%N)
echo "$(date -u -d @${start%.*} +%FT%TZ) sql $f rc=$rc secs=$(echo "$end - $start" | bc) bytes=$(wc -c < "$d/$f.out.json") sql_sha256=$(sha256sum < "$d/$f.sql" | cut -c1-16)" >> "$d/requests.log"
tail -1 "$d/requests.log"
python3 -I -c "
import json,sys
b=open('$d/$f.out.json').read()
try: j=json.loads(b)
except Exception: print('NOT JSON:', b[:500]); sys.exit(5)
if 'rows' not in j or 'columns' not in j: print('ERROR BODY:', b[:800]); sys.exit(5)
print('row_count', j.get('row_count'), 'truncated', j.get('truncated'), 'cols', j['columns'])
if j.get('truncated'): print('TRUNCATED'); sys.exit(6)
"
prc=$?
if [ "$prc" = 5 ]; then echo "$(date -u +%FT%TZ) err $f $(head -c 300 "$d/$f.out.json" | tr '\n' ' ')" >> "$d/requests.log"; fi
exit $prc
