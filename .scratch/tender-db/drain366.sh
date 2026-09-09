#!/usr/bin/env bash
# Issue 366 unit 1: re-fold every tender whose head value is a negative
# sentinel, so the standing rows catch up with the election guard that already
# refuses them. Batched at the refold-notices cap of 1000, one batch at a time,
# waiting for the queue to drain between rounds so nothing piles up.
set -uo pipefail
set -a; . /root/tender-admin-secret; set +a
A=(-s -m 300 -H "x-admin-secret: $TENDER_ADMIN_SECRET")
J=http://127.0.0.1:8080/admin/jobs

idle() { for _ in $(seq 1 240); do
  c=$(curl "${A[@]}" "$J?limit=1" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("current") or "")' 2>/dev/null)
  [ -z "$c" ] && return 0; sleep 5; done; return 1; }

for round in $(seq 1 20); do
  left=$(echo 'SELECT COUNT(*) FROM tenders WHERE current_value_eur_cents < 0' \
         | bash /root/sq.sh | python3 -c 'import json,sys; print(json.load(sys.stdin)["rows"][0][0])')
  echo "round $round: $left negative head(s) left"
  [ "$left" -eq 0 ] && { echo "DRAINED"; break; }
  echo 'SELECT DISTINCT v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE t.current_value_eur_cents < 0 ORDER BY 1 LIMIT 1000' \
    | bash /root/sq.sh \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); json.dump({"kind":"refold-notices","notices":[r[0] for r in d["rows"]]}, open("/tmp/b.json","w"))'
  n=$(python3 -c 'import json; print(len(json.load(open("/tmp/b.json"))["notices"]))')
  [ "$n" -eq 0 ] && { echo "no ids returned though $left remain — stopping"; exit 1; }
  curl "${A[@]}" -X POST -H 'content-type: application/json' --data-binary @/tmp/b.json "$J" >/dev/null
  idle || { echo "queue did not drain in 20 min — stopping"; exit 1; }
done
rm -f /tmp/b.json
echo 'SELECT COUNT(*) AS negative_left FROM tenders WHERE current_value_eur_cents < 0' | bash /root/sq.sh
