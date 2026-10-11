#!/bin/bash
# usage: run.sh <n> <tag> <sql>   -- one sequential read, journaled; never retried
n=$1; tag=$2; sql=$3
out=$(printf "v%02d-%s.out" "$n" "$tag")
start=$(date +%s.%N)
echo "$sql" | ssh root@zebreus.click /root/sq.sh > "$out" 2>"$out.err"
rc=$?
end=$(date +%s.%N)
python3 - "$n" "$tag" "$sql" "$rc" "$start" "$end" "$out" <<'PY'
import json,sys
n,tag,sql,rc,s,e,out=sys.argv[1:]
txt=open(out).read()
try:
    d=json.loads(txt); err=d.get('error'); rows=d.get('row_count'); trunc=d.get('truncated')
except Exception as ex:
    err=str(ex)[:100]; rows=None; trunc=None
rec=dict(call=int(n),tag=tag,sql=sql,rc=int(rc),secs=round(float(e)-float(s),2),out=out,rows=rows,truncated=trunc,error=err,is_408=('408' in txt[:300]))
open('journal.jsonl','a').write(json.dumps(rec)+'\n')
print(json.dumps({k:rec[k] for k in ('call','tag','rc','secs','rows','truncated','error','is_408')}))
PY
