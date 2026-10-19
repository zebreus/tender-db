#!/bin/bash
# usage: run.sh <name> — runs <name>.sql on the box via /root/sq.sh (token stays on the box), saves raw to
# <name>.out.json, logs to requests.log. Hard cap: refuses once 12 /v1/sql requests are logged (every attempt
# counts, a transport failure included). NEVER retries (a 408 is final).
set -u
d=/home/user/tender-db/.scratch/tender-db/545-triage/u2a
f="$1"
used=$(grep -c ' sql ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 12 ]; then echo "REFUSED: $used /v1/sql requests already used (cap 12)"; exit 3; fi
[ -s "$d/$f.plan.txt" ] || { echo "REFUSED: no local plan for $f"; exit 4; }
if grep -q 'SCAN' "$d/$f.plan.txt"; then echo "REFUSED: plan has a SCAN"; exit 4; fi
scp -q "$d/$f.sql" root@zebreus.click:/root/qtri/545u2a_$f.sql || { echo SCP-FAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/qtri/545u2a_$f.sql" > "$d/$f.out.json"
rc=$?
end=$(date +%s.%N)
echo "$(date -u +%FT%TZ) sql $f rc=$rc secs=$(echo "$end - $start" | bc) bytes=$(wc -c < "$d/$f.out.json")" >> "$d/requests.log"
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
