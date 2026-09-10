#!/usr/bin/env bash
# Issue 372, standing rows: re-fold the tenders whose negative amounts still
# carry no withheld marker, so the fold stamps `quality` the way it does for
# every row folded since `ce6d2c2`.
#
# Same route as issue 366's drains — aim the fold, do not reimplement it. The
# marker is conditioned on the NOTICE's BT-195 declaration, which only the fold
# reads, so a SQL repair could not do this even in principle.
#
# Windowed on tender_id because tender_version_amounts has no index on `cents`;
# the (tender_id, seq) index makes each window a range scan. Only ids below
# 1.5M and above 7.5M hold any negatives (measured), but the walk covers the
# whole range so a new cluster is not silently skipped.
set -uo pipefail
set -a; . /root/tender-admin-secret; set +a
A=(-s -m 300 -H "x-admin-secret: $TENDER_ADMIN_SECRET")
J=http://127.0.0.1:8080/admin/jobs
STRIDE=500000
MAXID=8000000

idle() { for _ in $(seq 1 240); do
  c=$(curl "${A[@]}" "$J?limit=1" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("current") or "")' 2>/dev/null)
  [ -z "$c" ] && return 0; sleep 5; done; return 1; }

lo=0
while [ "$lo" -lt "$MAXID" ]; do
  hi=$((lo + STRIDE))
  prev=-1
  for round in $(seq 1 8); do
    printf 'SELECT DISTINCT v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE t.id IN (SELECT tender_id FROM tender_version_amounts WHERE tender_id > %s AND tender_id <= %s AND cents < 0 AND quality IS NULL) ORDER BY 1 LIMIT 1000\n' "$lo" "$hi" \
      | bash /root/sq.sh \
      | python3 -c 'import json,sys; d=json.load(sys.stdin); json.dump({"kind":"refold-notices","notices":[r[0] for r in d["rows"]]}, open("/tmp/bw.json","w"))'
    n=$(python3 -c 'import json; print(len(json.load(open("/tmp/bw.json"))["notices"]))')
    [ "$n" -eq 0 ] && break
    # STOP WHEN THE COUNT STOPS FALLING, not when it reaches zero. Not every
    # unmarked row is markable: issue 372 unit 5 established that a negative
    # amount whose notice carries NO BT-195 declaration is a publisher-invented
    # sentinel, and the fold is RIGHT to leave it unmarked. The first run of this
    # script had `n == 0` as its only exit and burned every remaining round
    # re-folding the same 28 notices per window to no effect — harmless, but it
    # is the tell of a loop whose success condition disagrees with the code it
    # is driving.
    if [ "$n" -eq "$prev" ]; then
      echo "window $lo-$hi: $n notice(s) will not clear — the undeclared residue, correctly unmarked"
      break
    fi
    prev=$n
    echo "window $lo-$hi round $round: refolding $n notice(s)"
    curl "${A[@]}" -X POST -H 'content-type: application/json' --data-binary @/tmp/bw.json "$J" >/dev/null
    idle || { echo "queue did not drain in 20 min — stopping"; exit 1; }
  done
  lo=$hi
done
rm -f /tmp/bw.json
echo "DRAINED — recounting"
bash /tmp/count372.sh | tail -3
