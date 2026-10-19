#!/bin/bash
# usage: run.sh <name> — runs <name>.sql on the box via /root/sq.sh, raw body to <name>.out.json, one line to sql_requests.log
set -u
d=/home/user/tender-db/.scratch/tender-db/548-triage
f="$1"
scp -q "$d/$f.sql" root@zebreus.click:/root/qtri/548_$f.sql || { echo SCP-FAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/qtri/548_$f.sql" > "$d/$f.out.json"
rc=$?
end=$(date +%s.%N)
echo "$(date -u +%FT%TZ) $f rc=$rc secs=$(echo "$end - $start" | bc) bytes=$(wc -c < "$d/$f.out.json")" >> "$d/sql_requests.log"
tail -1 "$d/sql_requests.log"
python3 -I -c "
import json,sys
b=open('$d/$f.out.json').read()
try: j=json.loads(b)
except Exception: print('NOT JSON:', b[:500]); sys.exit()
if 'rows' not in j or 'columns' not in j: print('ERROR BODY:', b[:800]); sys.exit()
print('row_count', j.get('row_count'), 'truncated', j.get('truncated'), 'cols', j['columns'])
"
