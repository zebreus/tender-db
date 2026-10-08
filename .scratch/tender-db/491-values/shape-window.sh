#!/bin/bash
# usage: q491.sh <from> <to>  — run-together min/max shape among text-era notice amounts in a notice-id window
a=$1; b=$2
cat <<SQL | ssh root@zebreus.click /root/sq.sh
SELECT a.currency, COUNT(*) AS n,
       SUM(CASE WHEN (a.cents % 100) = 0
                 AND ((a.cents / 100) % 1000000) >= 100000
                 AND ((a.cents / 100) % 1000000) > ((a.cents / 100) / 1000000)
                 AND ((a.cents / 100) % 1000000) <= 4 * ((a.cents / 100) / 1000000)
                THEN 1 ELSE 0 END) AS shape
  FROM notice_amounts a JOIN notices n ON n.id = a.notice_id
 WHERE a.notice_id BETWEEN $a AND $b AND n.profile = 'text'
 GROUP BY a.currency ORDER BY n DESC
SQL
