"""mkcompact.py NAME ID... : one compact-shape query: the T row, the HEAD version's amounts/lots/results in full,
and the whole chain (every version) reduced to what ScalePartners reads from it:
  CA = DISTINCT (currency, cents, is_procedure) over every amount above the 10.00 floor, where is_procedure =
       tender scope AND no award of the same version has the same (currency, cents)  (add_version_figures' rule)
  CR = DISTINCT (currency, cents) over every award above the floor."""
import sys
name, arms = sys.argv[1], sys.argv[2].split(",")
ids = [int(x) for x in sys.argv[3:]]
L = ",".join(map(str, ids))
parts = [f"SELECT 'T' AS k, t.id AS tid, t.current_seq AS seq, NULL AS lot_id, t.current_value_eur_cents AS n1, NULL AS s1, NULL AS s2, NULL AS s3, NULL AS n2, NULL AS n3 FROM tenders t WHERE t.id IN ({L})",
       f"SELECT 'A', a.tender_id, a.seq, a.lot_id, a.cents, a.field, a.currency, a.tax_basis, a.eur_cents, a.quality FROM tenders t CROSS JOIN tender_version_amounts a ON a.tender_id = t.id AND a.seq = t.current_seq WHERE t.id IN ({L})",
       f"SELECT 'L', l.tender_id, l.seq, l.lot_id, NULL, l.kind, k.lot_key, NULL, NULL, NULL FROM tenders t CROSS JOIN tender_version_lots l ON l.tender_id = t.id AND l.seq = t.current_seq CROSS JOIN lots k ON k.id = l.lot_id WHERE t.id IN ({L})",
       f"SELECT 'R', r.tender_id, r.seq, r.lot_id, r.awarded_cents, r.awarded_currency, lr.result_key, NULL, r.lot_result_id, lr.notice_id FROM tenders t CROSS JOIN tender_version_lot_results r ON r.tender_id = t.id AND r.seq = t.current_seq CROSS JOIN lot_results lr ON lr.id = r.lot_result_id WHERE t.id IN ({L})",
       f"SELECT DISTINCT 'CA', a.tender_id, NULL, NULL, a.cents, NULL, a.currency, NULL, (a.lot_id IS NULL AND NOT EXISTS (SELECT 1 FROM tender_version_lot_results x WHERE x.tender_id = a.tender_id AND x.seq = a.seq AND x.awarded_currency = a.currency AND x.awarded_cents = a.cents)), NULL FROM tenders t CROSS JOIN tender_version_amounts a ON a.tender_id = t.id WHERE t.id IN ({L}) AND a.cents > 1000",
       f"SELECT DISTINCT 'CR', r.tender_id, NULL, NULL, r.awarded_cents, r.awarded_currency, NULL, NULL, NULL, NULL FROM tenders t CROSS JOIN tender_version_lot_results r ON r.tender_id = t.id WHERE t.id IN ({L}) AND r.awarded_cents > 1000 AND r.awarded_currency IS NOT NULL"]
KEYS = ["T", "A", "L", "R", "CA", "CR"]
sql = " UNION ALL ".join(p for k, p in zip(KEYS, parts) if k in arms)
open(f"sql/{name}.sql", "w").write(sql + "\n")
print(name, len(ids), len(sql))
