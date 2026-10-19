#!/bin/bash
# usage: tplan.sh <name>... — EXPLAIN QUERY PLAN of each <name>.sql on the local plan.db copy, twice:
# python sqlite3 and turso (target/debug/plan-probe plan). Writes <name>.plan.txt.
d=/home/user/tender-db/.scratch/tender-db/539-triage
for n in "$@"; do
  python3 -I - "$d" "$n" <<'PY'
import sqlite3, sys
d, n = sys.argv[1], sys.argv[2]
sql = open(f'{d}/{n}.sql').read()
c = sqlite3.connect(f'file:{d}/plan.db?mode=ro', uri=True)
rows = list(c.execute('EXPLAIN QUERY PLAN ' + sql))
with open(f'{d}/{n}.plan.txt', 'w') as f:
    f.write('-- EXPLAIN QUERY PLAN, python sqlite3 %s on plan.db (copy of 545-triage/u2b/plan.db; notices_fetch_id, tender_versions_notice present)\n' % sqlite3.sqlite_version)
    for r in rows: f.write(repr(r) + '\n')
PY
  { echo "-- name: $n"; cat "$d/$n.sql"; } > "$d/.tplan_$n.stmts"
  out=$(timeout 60 /home/user/tender-db/target/debug/plan-probe plan "$d/plan.db" "$d/.tplan_$n.stmts" 2>&1); rc=$?
  { echo "-- turso plan-probe (rc=$rc)"; echo "$out"; } >> "$d/$n.plan.txt"
  echo "== $n rc=$rc"; cat "$d/$n.plan.txt" | head -60
  rm -f "$d/.tplan_$n.stmts"
done
