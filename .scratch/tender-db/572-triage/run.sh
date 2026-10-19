#!/bin/bash
# usage: run.sh <name> — runs <name>.sql on the box via /root/sq.sh (the token stays on the box and is never
# printed), saves the raw body to <name>.out.json, logs one 'sql' line to requests.log. Hard cap: refuses once
# 60 /v1/sql requests are logged (every attempt counts, a transport failure included). NEVER retries (a 408
# is final). Refuses a read with no local plan, or whose plan has a SCAN.
set -u
d=/home/user/tender-db/.scratch/tender-db/572-triage
f="$1"
used=$(grep -c ' sql ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 60 ]; then echo "REFUSED: $used /v1/sql requests already used (cap 60)"; exit 3; fi
[ -s "$d/$f.plan.txt" ] || { echo "REFUSED: no local plan for $f"; exit 4; }
if grep -q 'SCAN' "$d/$f.plan.txt"; then echo "REFUSED: plan has a SCAN"; exit 4; fi
scp -q "$d/$f.sql" root@zebreus.click:/root/qtri/572_$f.sql || { echo "$(date -u +%FT%TZ) sql $f SCP-FAIL" >> "$d/requests.log"; echo SCP-FAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/qtri/572_$f.sql" > "$d/$f.out.json"
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
