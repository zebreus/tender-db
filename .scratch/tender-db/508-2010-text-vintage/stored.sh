#!/bin/bash
# usage: stored.sh <from> <to> — award bodies, favour-heading bodies, stored winners and values in one text notice-id window
a=$1; b=$2
q() { ssh root@zebreus.click /root/sq.sh | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["rows"][0] if "rows" in d else "ERR "+str(d.get("error")))'; }
x=$(cat <<SQL | q
SELECT SUM(value LIKE 'CONTRACT AWARD NOTICE%'), SUM(value LIKE '%IN FAVOUR OF WHOM%'), SUM(value LIKE '%IN FAVOUR OF WHOM%' AND value NOT LIKE '%HAS BEEN TAKEN:%'),
       SUM(value LIKE '%TOTAL FINAL VALUE%' OR value LIKE '%Total final value%')
  FROM notice_texts WHERE notice_id BETWEEN $a AND $b AND section_id='PROCEDURE' AND field_id='TXT-TX' AND ordinal=0
SQL
)
w=$(echo "SELECT COUNT(DISTINCT notice_id) FROM notice_texts WHERE notice_id BETWEEN $a AND $b AND field_id='TED-OFFICIALNAME'" | q)
v=$(echo "SELECT COUNT(DISTINCT notice_id) FROM notice_amounts WHERE notice_id BETWEEN $a AND $b AND field_id='TED-VAL_TOTAL'" | q)
echo "$a can,favour,favour_nocolon,tfv=$x winners=$w values=$v"
