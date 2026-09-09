#!/usr/bin/env bash
# Issue 366 unit 3: re-fold every tender whose head deadline is beyond
# DEADLINE_HORIZON_SECS (published_at + 10y) AND still in the future, so the
# standing rows catch up with the horizon filter head_deadline already applies.
# The future half is what causes the visible harm: those rows are served as
# status=open and own `sort=deadline&order=desc`. Same shape as drain366.sh.
set -uo pipefail
set -a; . /root/tender-admin-secret; set +a
A=(-s -m 300 -H "x-admin-secret: $TENDER_ADMIN_SECRET")
J=http://127.0.0.1:8080/admin/jobs
HORIZON=315360000

idle() { for _ in $(seq 1 240); do
  c=$(curl "${A[@]}" "$J?limit=1" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("current") or "")' 2>/dev/null)
  [ -z "$c" ] && return 0; sleep 5; done; return 1; }

for round in $(seq 1 10); do
  now=$(date +%s)
  left=$(printf 'SELECT COUNT(*) FROM tenders WHERE current_deadline > %s AND current_deadline - current_published_at > %s\n' "$now" "$HORIZON" \
         | bash /root/sq.sh | python3 -c 'import json,sys; print(json.load(sys.stdin)["rows"][0][0])')
  echo "round $round: $left beyond-horizon future deadline(s) left"
  [ "$left" -eq 0 ] && { echo "DRAINED"; break; }
  printf 'SELECT DISTINCT v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE t.current_deadline > %s AND t.current_deadline - t.current_published_at > %s ORDER BY 1 LIMIT 1000\n' "$now" "$HORIZON" \
    | bash /root/sq.sh \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); json.dump({"kind":"refold-notices","notices":[r[0] for r in d["rows"]]}, open("/tmp/bd.json","w"))'
  n=$(python3 -c 'import json; print(len(json.load(open("/tmp/bd.json"))["notices"]))')
  [ "$n" -eq 0 ] && { echo "no ids returned though $left remain — stopping"; exit 1; }
  echo "  enqueuing refold of $n notice(s)"
  curl "${A[@]}" -X POST -H 'content-type: application/json' --data-binary @/tmp/bd.json "$J" >/dev/null
  idle || { echo "queue did not drain in 20 min — stopping"; exit 1; }
done
rm -f /tmp/bd.json
now=$(date +%s)
printf 'SELECT COUNT(*) AS beyond_horizon_future_left FROM tenders WHERE current_deadline > %s AND current_deadline - current_published_at > %s\n' "$now" "$HORIZON" | bash /root/sq.sh
