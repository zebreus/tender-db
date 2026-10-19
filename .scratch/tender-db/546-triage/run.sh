#!/bin/bash
# usage: run.sh <name>  — runs <name>.sql on the box via /root/sq.sh, saves raw to <name>.out.json, logs it
set -u
d=/home/user/tender-db/.scratch/tender-db/546-triage
f="$1"
scp -q "$d/$f.sql" root@zebreus.click:/root/qtri/546_$f.sql || { echo SCP-FAIL; exit 1; }
start=$(date +%s.%N)
ssh root@zebreus.click "/root/sq.sh < /root/qtri/546_$f.sql" > "$d/$f.out.json"
rc=$?
end=$(date +%s.%N)
echo "$(date -u +%FT%TZ) $f rc=$rc secs=$(echo "$end - $start" | bc) bytes=$(wc -c < "$d/$f.out.json")" >> "$d/requests.log"
tail -1 "$d/requests.log"
python3 -c "
import json,sys
b=open('$d/$f.out.json').read()
try: j=json.loads(b)
except Exception: print('NOT JSON:', b[:500]); sys.exit()
if 'rows' not in j or 'columns' not in j: print('ERROR BODY:', b[:800]); sys.exit()
print('row_count', j.get('row_count'), 'truncated', j.get('truncated'), 'cols', j['columns'])
"
