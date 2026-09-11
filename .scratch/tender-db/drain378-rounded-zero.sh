#!/usr/bin/env bash
# Issue 378: re-fold the 608 tenders whose head value is a derived zero reached
# by ROUNDING — a published HUF 1.00 or one minor unit converting to under half
# a cent — so they catch up with issue 379's token leg (`bfe7ac6`).
#
# **No published-cents guard, and that is the whole point.** The zero drain
# (`drain366-zero.sh`) carried an `EXISTS … a.cents = 0` guard to keep its loop
# terminating, and these rows were exactly what it excluded: their head is 0 but
# their published figure is not. The token drain (`drain379-token.sh`) selects by
# `current_value_eur_cents IN (1, 100)` and these serve 0. Neither selector is
# wrong; this cohort fell BETWEEN two correct selectors, because the outcome a
# fix produces moves when a later rule changes what gets elected.
#
# Terminating instead on the round-over-round count, which is the guard that
# survives not knowing what the rule refuses: ~602 of the 608 carry a token the
# election now refuses, and the ~6 that do not (CZK 0.10, CZK 0.12, HUF 1.48,
# HUF 0.79, LIT 2.89) will still serve 0 afterwards. The loop stops the moment a
# round changes nothing and prints the residue, which IS the measurement issue
# 378's remaining decision needs.
set -uo pipefail
set -a; . /root/tender-admin-secret; set +a
A=(-s -m 300 -H "x-admin-secret: $TENDER_ADMIN_SECRET")
J=http://127.0.0.1:8080/admin/jobs

WHERE='t.current_value_eur_cents = 0'

idle() { for _ in $(seq 1 240); do
  c=$(curl "${A[@]}" "$J?limit=1" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("current") or "")' 2>/dev/null)
  [ -z "$c" ] && return 0; sleep 5; done; return 1; }

prev=-1
for round in $(seq 1 12); do
  left=$(echo "SELECT COUNT(*) FROM tenders t WHERE $WHERE" \
         | bash /root/sq.sh | python3 -c 'import json,sys; print(json.load(sys.stdin)["rows"][0][0])')
  echo "round $round: $left zero head value(s) left"
  [ "$left" -eq 0 ] && { echo "DRAINED"; break; }
  if [ "$round" -gt 1 ] && [ "$left" -ge "$prev" ]; then
    echo "round changed nothing ($prev -> $left) — the residue is the rounding-only class, which is what issue 378 decides on"
    break
  fi
  prev=$left
  echo "SELECT DISTINCT v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE $WHERE ORDER BY 1 LIMIT 1000" \
    | bash /root/sq.sh \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); json.dump({"kind":"refold-notices","notices":[r[0] for r in d["rows"]]}, open("/tmp/br78.json","w"))'
  n=$(python3 -c 'import json; print(len(json.load(open("/tmp/br78.json"))["notices"]))')
  [ "$n" -eq 0 ] && { echo "no ids returned though $left remain — stopping"; exit 1; }
  echo "  enqueuing refold of $n notice(s)"
  curl "${A[@]}" -X POST -H 'content-type: application/json' --data-binary @/tmp/br78.json "$J" >/dev/null
  idle || { echo "queue did not drain in 20 min — stopping"; exit 1; }
done
rm -f /tmp/br78.json
echo "-- the residue, by published value: what a conversion floor would have to decide about --"
echo "SELECT a.currency, a.cents, COUNT(DISTINCT t.id) AS tenders FROM tenders t JOIN tender_version_amounts a ON a.tender_id = t.id AND a.seq = t.current_seq AND a.eur_cents = 0 WHERE t.current_value_eur_cents = 0 GROUP BY 1,2 ORDER BY 3 DESC LIMIT 20" | bash /root/sq.sh
