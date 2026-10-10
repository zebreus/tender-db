#!/bin/bash
# usage: shape.sh <from> <to> — colon vs colonless sectioned award bodies in one text notice-id window
a=$1; b=$2
cat <<SQL | ssh root@zebreus.click /root/sq.sh
SELECT COUNT(*) AS bodies,
  SUM(value LIKE '%of contract(s)' || char(10) || 'Value %') AS agg_nocolon,
  SUM(value LIKE '%of contract(s):%') AS agg_colon,
  SUM(value LIKE '%Total final value of the contract' || char(10) || 'Value %') AS v4_nocolon,
  SUM(value LIKE '%IN FAVOUR OF WHOM A CONTRACT%') AS favour,
  SUM(value LIKE '%TO WHOM THE CONTRACT HAS BEEN%') AS to_whom,
  SUM(value LIKE 'CONTRACT AWARD NOTICE%') AS can
FROM notice_texts WHERE notice_id BETWEEN $a AND $b AND section_id='PROCEDURE' AND field_id='TXT-TX' AND ordinal=0
SQL
