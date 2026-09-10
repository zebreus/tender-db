#!/usr/bin/env bash
# Issue 372 unit 5: pull every negative amount row that carries no withheld
# marker, so the residue can be read rather than sampled. Windowed on tender_id
# for the same index reason as count372.sh.
set -uo pipefail
STRIDE=500000
MAXID=8000000
lo=0
echo "["
first=1
while [ "$lo" -lt "$MAXID" ]; do
  hi=$((lo + STRIDE))
  out=$(printf "SELECT a.tender_id, a.seq, a.field, a.cents, a.currency, substr(t.current_title,1,60) FROM tender_version_amounts a JOIN tenders t ON t.id = a.tender_id WHERE a.tender_id > %s AND a.tender_id <= %s AND a.cents < 0 AND a.quality IS NULL ORDER BY a.tender_id, a.seq\n" "$lo" "$hi" | bash /root/sq.sh)
  echo "$out" | python3 -c '
import json,sys
d=json.load(sys.stdin)
for r in d["rows"]:
    print(json.dumps(r))
' 2>/dev/null
  lo=$hi
done
echo "]"
