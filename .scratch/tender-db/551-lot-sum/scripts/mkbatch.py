"""mkbatch.py PREFIX SIZE IDFILE : write sql/PREFIX_NN.sql batches of the one union shape."""
import sys
prefix, size, idfile = sys.argv[1], int(sys.argv[2]), sys.argv[3]
ids = [int(x) for x in open(idfile).read().split()]
SHAPE = ("SELECT 'T' AS k, t.id AS tid, t.current_seq AS seq, NULL AS lot_id, t.current_value_eur_cents AS n1, NULL AS s1, NULL AS s2, NULL AS s3, NULL AS n2, NULL AS n3 FROM tenders t WHERE t.id IN ({L})"
         " UNION ALL SELECT 'A', a.tender_id, a.seq, a.lot_id, a.cents, a.field, a.currency, a.tax_basis, a.eur_cents, a.quality FROM tenders t CROSS JOIN tender_version_amounts a ON a.tender_id = t.id WHERE t.id IN ({L})"
         " UNION ALL SELECT 'L', l.tender_id, l.seq, l.lot_id, NULL, l.kind, k.lot_key, NULL, NULL, NULL FROM tenders t CROSS JOIN tender_version_lots l ON l.tender_id = t.id AND l.seq = t.current_seq CROSS JOIN lots k ON k.id = l.lot_id WHERE t.id IN ({L})"
         " UNION ALL SELECT 'R', r.tender_id, r.seq, r.lot_id, r.awarded_cents, r.awarded_currency, lr.result_key, NULL, r.lot_result_id, lr.notice_id FROM tenders t CROSS JOIN tender_version_lot_results r ON r.tender_id = t.id CROSS JOIN lot_results lr ON lr.id = r.lot_result_id WHERE t.id IN ({L})")
n = 0
for i in range(0, len(ids), size):
    chunk = ids[i:i + size]
    n += 1
    open(f"sql/{prefix}_{n:02d}.sql", "w").write(SHAPE.format(L=",".join(map(str, chunk))) + "\n")
    print(f"{prefix}_{n:02d} {len(chunk)}")
