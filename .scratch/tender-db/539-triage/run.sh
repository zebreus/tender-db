#!/bin/bash
# usage: run.sh <name> — runs <name>.sql on the box via /root/sq.sh (token stays on the box), saves the raw
# body to <name>.out.json, logs to requests.log. Hard cap: refuses once 40 /v1/sql requests are logged
# (every attempt counts, a transport failure included). NEVER retries (a 408 is final).
# Refuses without a local plan, or when the plan SCANs a big table (anything but fetches; a SCAN of a derived table `(subquery-N)` whose own step SEARCHes is allowed).
set -u
d=/home/user/tender-db/.scratch/tender-db/539-triage
f="$1"
used=$(grep -c ' sql ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 40 ]; then echo "REFUSED: $used /v1/sql requests already used (cap 40)"; exit 3; fi
if grep -q ' rc=.* 408\|"status":408\|HTTP 408' "$d/requests.log" 2>/dev/null && [ "${ALLOW_AFTER_408:-}" != 1 ]; then echo "REFUSED: a 408 is logged; SELECT 1 first"; exit 3; fi
[ -s "$d/$f.plan.txt" ] || { echo "REFUSED: no local plan for $f"; exit 4; }
if grep -E 'SCAN' "$d/$f.plan.txt" | grep -vqE 'SCAN (fetches|f)( |\)|$|'"'"')|SCAN \(subquery-[0-9]+\)$'; then echo "REFUSED: plan has a SCAN on a non-fetches table"; grep SCAN "$d/$f.plan.txt"; exit 4; fi
scp -q "$d/$f.sql" root@zebreus.click:/root/q539_$f.sql || { echo SCP-FAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/q539_$f.sql" > "$d/$f.out.json"
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
