#!/bin/bash
# Post-drain copy of ../run.sh (issue 539 drain audit, 2026-10-24).
# usage: run.sh <name> — runs <name>.sql on the box via /root/sq.sh (token stays on the box), saves the raw
# body to post-drain/<name>.out.json, logs to post-drain/requests.log. Hard cap: refuses once 20 /v1/sql requests are logged
# (every attempt counts, a transport failure included). NEVER retries (a 408 is final).
# Refuses without a local plan, or when the plan SCANs a big table (anything but fetches; a SCAN of a derived table `(subquery-N)` whose own step SEARCHes is allowed).
# Added refusals (post-drain): the .sql and .plan.txt must be byte-identical (sha256) to the pre-drain originals in ../;
# a name already logged is never re-sent (no retry); the box job queue must be idle (current null, queued []);
# statements are spaced >= 3 s apart. An error body is logged as an `err` line so the 408 refusal can see it.
set -u
src=/home/user/tender-db/.scratch/tender-db/539-triage
d=$src/post-drain
f="$1"
used=$(grep -c ' sql ' "$d/requests.log" 2>/dev/null || true); used=${used:-0}
if [ "$used" -ge 20 ]; then echo "REFUSED: $used /v1/sql requests already used (cap 20)"; exit 3; fi
if grep -q ' rc=.* 408\|"status":408\|"status": 408\|HTTP 408' "$d/requests.log" 2>/dev/null && [ "${ALLOW_AFTER_408:-}" != 1 ]; then echo "REFUSED: a 408 is logged; SELECT 1 first"; exit 3; fi
[ -s "$d/$f.plan.txt" ] || { echo "REFUSED: no local plan for $f"; exit 4; }
if grep -E 'SCAN' "$d/$f.plan.txt" | grep -vqE 'SCAN (fetches|f)( |\)|$|'"'"')|SCAN \(subquery-[0-9]+\)$'; then echo "REFUSED: plan has a SCAN on a non-fetches table"; grep SCAN "$d/$f.plan.txt"; exit 4; fi
for ext in sql plan.txt; do
  [ -s "$src/$f.$ext" ] || { echo "REFUSED: no pre-drain original $src/$f.$ext"; exit 4; }
  a=$(sha256sum < "$src/$f.$ext" | cut -d' ' -f1); b=$(sha256sum < "$d/$f.$ext" | cut -d' ' -f1)
  [ "$a" = "$b" ] || { echo "REFUSED: $f.$ext differs from the pre-drain original ($b vs $a)"; exit 4; }
done
if grep -qE " (sql|err) $f( |$)" "$d/requests.log" 2>/dev/null; then echo "REFUSED: $f was already sent once (no retry)"; exit 3; fi
aj=$(ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=2"' 2>/dev/null)
if ! printf '%s' "$aj" | python3 -I -c "
import json,sys
j=json.loads(sys.stdin.read())
c,q=j.get('current','missing'),j.get('queued','missing')
print('jobs: current', json.dumps(c)[:200], 'queued', json.dumps(q)[:200])
sys.exit(0 if c is None and q == [] else 1)
"; then echo "REFUSED: job queue not idle (or unreadable)"; exit 7; fi
last=$(grep ' sql ' "$d/requests.log" 2>/dev/null | tail -1 | cut -d' ' -f1)
if [ -n "$last" ]; then
  wait=$(( $(date -d "$last" +%s) + 4 - $(date +%s) )); [ "$wait" -gt 0 ] && sleep "$wait"
fi
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
prc=$?
if [ "$prc" = 5 ]; then echo "$(date -u +%FT%TZ) err $f $(head -c 300 "$d/$f.out.json" | tr '\n' ' ')" >> "$d/requests.log"; fi
exit $prc
