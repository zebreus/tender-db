#!/usr/bin/env bash
# Issue 379: re-fold every tender whose head value is a ONE-UNIT TOKEN — an
# elected published 0.01 or 1.00 — so the standing rows catch up with the token
# leg of `sentinel_amount` (bfe7ac6). 112,229 tenders at the time of writing,
# the largest of this family's drains: 4.7x the exact zeros and 7x the negatives.
#
# Same selector discipline as `drain366-zero.sh`, and for the same reasons its
# header gives. The cohort is selected by the OUTCOME — the head value the change
# is meant to remove — not by restating the rule, because this issue family has
# already watched a value-list selector drift from the rule it copied and report
# a confident `0 left` on a corpus with 75.
#
# The published-cents EXISTS is the other half of the same fact, and it is what
# keeps the loop terminating. A head of exactly 1 or 100 EUR cents can also be
# reached by a NON-token amount converting onto it (CZK 25.00 lands near EUR
# 1.00), and the rule does not touch those — so a cohort without the guard would
# re-fold them every round forever.
#
# Backstop the guard cannot cover: a version carrying BOTH a token and another
# amount that converts onto 1 or 100 keeps a qualifying head after the token is
# refused, and its token row still satisfies the EXISTS. So the loop stops the
# moment a round fails to reduce the count and prints the residue, rather than
# spinning. At 1,000 notices per round this is ~113 rounds; the cap is 150.
set -uo pipefail
set -a; . /root/tender-admin-secret; set +a
A=(-s -m 300 -H "x-admin-secret: $TENDER_ADMIN_SECRET")
J=http://127.0.0.1:8080/admin/jobs

WHERE='t.current_value_eur_cents IN (1, 100) AND EXISTS (SELECT 1 FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = t.current_seq AND a.eur_cents = t.current_value_eur_cents AND a.cents IN (1, 100))'

idle() { for _ in $(seq 1 240); do
  c=$(curl "${A[@]}" "$J?limit=1" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("current") or "")' 2>/dev/null)
  [ -z "$c" ] && return 0; sleep 5; done; return 1; }

prev=-1
for round in $(seq 1 150); do
  left=$(echo "SELECT COUNT(*) FROM tenders t WHERE $WHERE" \
         | bash /root/sq.sh | python3 -c 'import json,sys; print(json.load(sys.stdin)["rows"][0][0])')
  echo "round $round: $left one-unit head value(s) left"
  [ "$left" -eq 0 ] && { echo "DRAINED"; break; }
  if [ "$round" -gt 1 ] && [ "$left" -ge "$prev" ]; then
    echo "round did not reduce the count ($prev -> $left) — stopping, residue needs a look"
    break
  fi
  prev=$left
  echo "SELECT DISTINCT v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE $WHERE ORDER BY 1 LIMIT 1000" \
    | bash /root/sq.sh \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); json.dump({"kind":"refold-notices","notices":[r[0] for r in d["rows"]]}, open("/tmp/bt.json","w"))'
  n=$(python3 -c 'import json; print(len(json.load(open("/tmp/bt.json"))["notices"]))')
  [ "$n" -eq 0 ] && { echo "no ids returned though $left remain — stopping"; exit 1; }
  echo "  enqueuing refold of $n notice(s)"
  curl "${A[@]}" -X POST -H 'content-type: application/json' --data-binary @/tmp/bt.json "$J" >/dev/null
  idle || { echo "queue did not drain in 20 min — stopping"; exit 1; }
done
rm -f /tmp/bt.json
echo "SELECT COUNT(*) AS one_unit_left FROM tenders t WHERE $WHERE" | bash /root/sq.sh
