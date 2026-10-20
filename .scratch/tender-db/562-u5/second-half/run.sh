#!/bin/bash
# usage: run.sh <name>  — scp sql/<name>.sql to the box, run it through /root/sq.sh (token stays on the box),
# save out/<name>.out, print whether the body is an answer (row_count + columns) or an error.
D=/home/user/tender-db/.scratch/tender-db/562-u5/second-half
mkdir -p $D/out
scp -q -o BatchMode=yes $D/sql/$1.sql root@zebreus.click:/root/qtri/u5b_$1.sql || { echo SCP-FAIL; exit 2; }
start=$(date +%s.%N)
ssh -o BatchMode=yes root@zebreus.click "/root/sq.sh < /root/qtri/u5b_$1.sql" > $D/out/$1.out 2>&1
rc=$?
end=$(date +%s.%N)
echo "$(date -u +%FT%TZ) $1 rc=$rc" >> $D/out/requests.log
python3 - "$D/out/$1.out" "$rc" "$start" "$end" <<'PY'
import json,sys
p,rc,s,e=sys.argv[1:]
t=open(p).read()
try:
    j=json.loads(t)
    ok = 'rows' in j and 'columns' in j and 'row_count' in j
except Exception:
    ok=False; j=None
print(f"{p.split('/')[-1]} ssh_rc={rc} secs={float(e)-float(s):.2f} ok={ok} rows={j.get('row_count') if ok else None} truncated={j.get('truncated') if ok else None}")
if not ok: print("BODY:", t[:500])
PY
