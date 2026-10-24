#!/bin/bash
# usage: tplan.sh <name>... — EXPLAIN QUERY PLAN of each <name>.sql on post-drain-chains/plan.db (a copy of
# 539-triage/plan.db, schema only, no rows, no indexes added), twice: python sqlite3 and turso (b6u6kit plan-probe plan).
# Writes <name>.plan.txt.
d=/home/user/tender-db/.scratch/tender-db/539-triage/post-drain-chains
pp=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b6u6kit/plan-probe
for n in "$@"; do
  python3 -I - "$d" "$n" <<'PY'
import sqlite3, sys
d, n = sys.argv[1], sys.argv[2]
sql = open(f'{d}/{n}.sql').read()
c = sqlite3.connect(f'file:{d}/plan.db?mode=ro', uri=True)
rows = list(c.execute('EXPLAIN QUERY PLAN ' + sql))
with open(f'{d}/{n}.plan.txt', 'w') as f:
    f.write('-- EXPLAIN QUERY PLAN, python sqlite3 %s on post-drain-chains/plan.db (copy of 539-triage/plan.db)\n' % sqlite3.sqlite_version)
    for r in rows: f.write(repr(r) + '\n')
PY
  { echo "-- name: $n"; cat "$d/$n.sql"; } > "$d/.tplan_$n.stmts"
  out=$(timeout 60 "$pp" plan "$d/plan.db" "$d/.tplan_$n.stmts" 2>&1); rc=$?
  { echo "-- turso plan-probe (rc=$rc) $(sha256sum < "$pp" | cut -c1-16)"; echo "$out"; } >> "$d/$n.plan.txt"
  echo "== $n rc=$rc"; cat "$d/$n.plan.txt"
  rm -f "$d/.tplan_$n.stmts"
done
