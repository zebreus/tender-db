#!/bin/bash
# usage: tplan.sh <name>... — turso EXPLAIN QUERY PLAN (target/debug/plan-probe plan) of each <name>.sql
# on the local plan.db copy; appends the turso plan to <name>.plan.txt.
d=/home/user/tender-db/.scratch/tender-db/572-triage
for n in "$@"; do
  { echo "-- name: $n"; cat "$d/$n.sql"; } > "$d/.tplan_$n.stmts"
  out=$(timeout 60 /home/user/tender-db/target/debug/plan-probe plan "$d/plan.db" "$d/.tplan_$n.stmts" 2>&1); rc=$?
  { echo "-- turso plan-probe (rc=$rc)"; echo "$out"; } >> "$d/$n.plan.txt"
  echo "== $n rc=$rc"; echo "$out" | head -40
  rm -f "$d/.tplan_$n.stmts"
done
