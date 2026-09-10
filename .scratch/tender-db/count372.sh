#!/usr/bin/env bash
# Issue 372: how many negative amount rows still carry NO withheld marker?
#
# Windowed on tender_id so each query is an index range scan on
# tender_version_amounts(tender_id, seq) rather than a full-table scan — the
# same reason the DQ report windows, and what keeps every read inside the 10 s
# cap (docs/agents/prod-box-reads.md).
set -uo pipefail
STRIDE=500000
MAXID=8000000
total_unmarked=0
total_marked=0
lo=0
while [ "$lo" -lt "$MAXID" ]; do
  hi=$((lo + STRIDE))
  out=$(printf "SELECT sum(CASE WHEN quality IS NULL THEN 1 ELSE 0 END) AS unmarked, sum(CASE WHEN quality IS NOT NULL THEN 1 ELSE 0 END) AS marked FROM tender_version_amounts WHERE tender_id > %s AND tender_id <= %s AND cents < 0\n" "$lo" "$hi" \
        | bash /root/sq.sh)
  u=$(echo "$out" | python3 -c 'import json,sys; r=json.load(sys.stdin)["rows"][0]; print(r[0] or 0)' 2>/dev/null || echo ERR)
  m=$(echo "$out" | python3 -c 'import json,sys; r=json.load(sys.stdin)["rows"][0]; print(r[1] or 0)' 2>/dev/null || echo ERR)
  if [ "$u" = "ERR" ]; then echo "window $lo-$hi FAILED: $out"; exit 1; fi
  echo "window $lo-$hi: unmarked=$u marked=$m"
  total_unmarked=$((total_unmarked + u))
  total_marked=$((total_marked + m))
  lo=$hi
done
echo "TOTAL negative amount rows: unmarked=$total_unmarked marked=$total_marked"
