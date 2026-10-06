#!/bin/bash
# usage: q.sh LO HI  (eur cents, [LO,HI))
P="1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000, 10000000000, 100000000000, 1000000000000, 10000000000000, 100000000000000, 1000000000000000, 10000000000000000, 100000000000000000, 1000000000000000000"
cat <<SQL
SELECT t.id, t.current_value_eur_cents AS eur, h.field, h.currency, h.cents,
 (SELECT MAX(a.cents) FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.currency = h.currency
   AND a.cents > 1000 AND a.cents < h.cents AND h.cents % a.cents = 0 AND h.cents / a.cents IN ($P)) AS pa,
 (SELECT MAX(r.awarded_cents) FROM tender_version_lot_results r WHERE r.tender_id = t.id AND r.awarded_currency = h.currency
   AND r.awarded_cents > 1000 AND r.awarded_cents < h.cents AND h.cents % r.awarded_cents = 0 AND h.cents / r.awarded_cents IN ($P)) AS pr,
 (SELECT COUNT(DISTINCT s.field) FROM tender_version_amounts s WHERE s.tender_id = t.id AND s.seq = t.current_seq
   AND s.currency = h.currency AND s.cents = h.cents) AS nf
FROM tenders t LEFT JOIN tender_version_amounts h ON h.rowid = (SELECT s.rowid FROM tender_version_amounts s
  WHERE s.tender_id = t.id AND s.seq = t.current_seq AND s.eur_cents = t.current_value_eur_cents ORDER BY s.cents DESC, s.currency LIMIT 1)
WHERE t.current_value_eur_cents >= $1 AND t.current_value_eur_cents < $2
ORDER BY t.current_value_eur_cents DESC, t.id DESC LIMIT 1000
SQL
