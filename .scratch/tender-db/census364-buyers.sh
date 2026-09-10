#!/usr/bin/env bash
# Issue 364 unit 3, corrected: count DISTINCT buyer organizations per tender
# across BOTH buyer vocabularies.
#
# The issue's calibration says role='Procedure-Buyer' only. That is right about
# excluding Tenderer/ReviewOrg noise and wrong about the corpus: the legacy era
# maps its buyer to role='buyer', and tender 2816628 — this issue's own flagship
# 127-buyer weld — has 2,983 'buyer' rows and ZERO 'Procedure-Buyer'. The gauge
# as specified would report 0 buyers for it.
#
# Counted across ALL versions, not just the head: a weld shows up along the
# version chain, and it avoids a per-row join to tenders.current_seq that put the
# 500k-window form over the 10s cap.
set -uo pipefail
STRIDE=100000
MAXID=8000000
tot3=0; tot5=0; tot10=0; tot50=0
lo=0
while [ "$lo" -lt "$MAXID" ]; do
  hi=$((lo + STRIDE))
  out=$(printf "SELECT sum(CASE WHEN n >= 3 THEN 1 ELSE 0 END), sum(CASE WHEN n >= 5 THEN 1 ELSE 0 END), sum(CASE WHEN n >= 10 THEN 1 ELSE 0 END), sum(CASE WHEN n >= 50 THEN 1 ELSE 0 END) FROM (SELECT tender_id, count(DISTINCT organization_id) AS n FROM tender_version_parties WHERE tender_id > %s AND tender_id <= %s AND role IN ('buyer','Procedure-Buyer') GROUP BY tender_id)\n" "$lo" "$hi" | bash /root/sq.sh)
  vals=$(echo "$out" | python3 -c 'import json,sys
r=json.load(sys.stdin)["rows"][0]
print((r[0] or 0),(r[1] or 0),(r[2] or 0),(r[3] or 0))' 2>/dev/null)
  if [ -z "$vals" ]; then echo "window $lo-$hi FAILED: $(echo "$out" | head -c 150)"; exit 1; fi
  set -- $vals
  tot3=$((tot3+$1)); tot5=$((tot5+$2)); tot10=$((tot10+$3)); tot50=$((tot50+$4))
  [ "$1" -gt 0 ] && echo "window $lo-$hi: >=3:$1 >=5:$2 >=10:$3 >=50:$4"
  lo=$hi
done
echo "TOTAL distinct buyer orgs per tender: >=3: $tot3   >=5: $tot5   >=10: $tot10   >=50: $tot50"

