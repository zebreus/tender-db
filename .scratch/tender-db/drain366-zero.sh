#!/usr/bin/env bash
# Issue 366: re-fold every tender whose head value is exactly 0 BECAUSE it
# elected a published 0, so the standing rows catch up with the zero leg of
# `sentinel_amount` (55d239d). 24,039 tenders at the time of writing.
#
# The cohort is selected by the OUTCOME, not by restating the rule. That is
# deliberate: this issue's own drain already learned that "a cohort selector
# which restates a rule will drift from it, and its failure mode is a confident
# zero" — the repdigit drain's value list encoded the pre-widening rule and
# reported `0 left` on a corpus with 75. `current_value_eur_cents = 0` cannot
# drift, because it names the served number the change is meant to remove.
#
# The EXISTS is not a second rule either — it is the published-cents half of the
# same fact, and it is what keeps the loop terminating. 608 tenders serve a head
# of 0 from a published amount that is NOT zero: HUF 1.00 and friends, which
# convert to under half a cent (HUF 549, RON 29, CZK 10, PLN 9, NOK 4, DKK 4,
# SEK 1, LTL 1, LIT 1). The zero leg does not touch those — they are a ROUNDING
# floor, not an election — so a cohort without the guard would re-fold them
# every round forever.
#
# Backstop for the case the guard cannot see: a version carrying BOTH a
# published 0 and a sub-cent amount stays at head 0 after the 0 is refused, and
# its published-0 row still satisfies the EXISTS. So the loop stops the moment a
# round fails to reduce the count, and prints the residue rather than spinning.
set -uo pipefail
set -a; . /root/tender-admin-secret; set +a
A=(-s -m 300 -H "x-admin-secret: $TENDER_ADMIN_SECRET")
J=http://127.0.0.1:8080/admin/jobs

WHERE='t.current_value_eur_cents = 0 AND EXISTS (SELECT 1 FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = t.current_seq AND a.eur_cents = 0 AND a.cents = 0)'

idle() { for _ in $(seq 1 240); do
  c=$(curl "${A[@]}" "$J?limit=1" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("current") or "")' 2>/dev/null)
  [ -z "$c" ] && return 0; sleep 5; done; return 1; }

prev=-1
for round in $(seq 1 40); do
  left=$(echo "SELECT COUNT(*) FROM tenders t WHERE $WHERE" \
         | bash /root/sq.sh | python3 -c 'import json,sys; print(json.load(sys.stdin)["rows"][0][0])')
  echo "round $round: $left zero head value(s) left"
  [ "$left" -eq 0 ] && { echo "DRAINED"; break; }
  if [ "$round" -gt 1 ] && [ "$left" -ge "$prev" ]; then
    echo "round did not reduce the count ($prev -> $left) — stopping, residue needs a look"
    break
  fi
  prev=$left
  echo "SELECT DISTINCT v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE $WHERE ORDER BY 1 LIMIT 1000" \
    | bash /root/sq.sh \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); json.dump({"kind":"refold-notices","notices":[r[0] for r in d["rows"]]}, open("/tmp/bz.json","w"))'
  n=$(python3 -c 'import json; print(len(json.load(open("/tmp/bz.json"))["notices"]))')
  [ "$n" -eq 0 ] && { echo "no ids returned though $left remain — stopping"; exit 1; }
  echo "  enqueuing refold of $n notice(s)"
  curl "${A[@]}" -X POST -H 'content-type: application/json' --data-binary @/tmp/bz.json "$J" >/dev/null
  idle || { echo "queue did not drain in 20 min — stopping"; exit 1; }
done
rm -f /tmp/bz.json
echo "SELECT COUNT(*) AS zero_left FROM tenders t WHERE $WHERE" | bash /root/sq.sh
