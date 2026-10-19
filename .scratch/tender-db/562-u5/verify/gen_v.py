# Verifier's confirmation reads (issue 562 unit 5, 10-19). Writes verify/sql/v1.sql, v2.sql.
import json, os
D = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
rows = []
for i in range(8):
    j = json.load(open(f'{D}/out/seqless_{i}.out'))
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
# v1: each seq-less tender row X and the k+1 rows after it (k = its head lots): which entities, whose lots.
arms = []
for r in rows:
    x, k = r['cursor'], r['head_lots']
    arms.append(f"SELECT {x} AS anchor, c.cursor, c.entity_kind, c.entity_id, c.version_seq, c.op, c.changed_at, "
                f"(SELECT l.tender_id FROM lots l WHERE l.id = c.entity_id) AS lot_tender FROM changes c "
                f"WHERE c.cursor >= {x} AND c.cursor <= {x + k + 1}")
open(f'{D}/verify/sql/v1.sql', 'w').write(" UNION ALL ".join(arms) + "\n")
# v2: q1's own version-driven form over the TED notices ingested 2026-10-19 (day 20745) in a notice-id window,
# classified and checked against the fold's change rows (changed_at 1792395355, cursor (958810037, 958848408]).
V2 = """SELECT pre, doe_before, ted_before, seq1, fold_seq_row, fold_seqless, COUNT(*) AS versions, COUNT(DISTINCT tid) AS tenders, MIN(nid) AS nid_lo, MAX(nid) AS nid_hi
FROM (SELECT n.id AS nid, v.tender_id AS tid, v.seq = 1 AS seq1, t.created_at < n.ingested_at AS pre,
   EXISTS (SELECT 1 FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = v.tender_id AND n2.id < n.id AND +n2.source = 'doe') AS doe_before,
   EXISTS (SELECT 1 FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = v.tender_id AND n2.id < n.id AND +n2.source = 'ted') AS ted_before,
   EXISTS (SELECT 1 FROM changes c WHERE c.entity_kind = 'tender' AND c.entity_id = v.tender_id AND c.version_seq = v.seq AND c.changed_at = 1792395355 AND c.cursor > 958810037 AND c.cursor <= 958848408) AS fold_seq_row,
   EXISTS (SELECT 1 FROM changes c WHERE c.entity_kind = 'tender' AND c.entity_id = v.tender_id AND c.version_seq IS NULL AND +c.op = 'changed' AND c.changed_at = 1792395355 AND c.cursor > 958810037 AND c.cursor <= 958848408) AS fold_seqless
  FROM notices n CROSS JOIN tender_versions v ON v.caused_by_notice_id = n.id CROSS JOIN tenders t ON t.id = v.tender_id
  WHERE n.id > 47409900 AND n.id <= 47414900 AND +n.source = 'ted' AND n.ingested_at >= 1792368000 AND n.ingested_at < 1792454400) AS w
GROUP BY pre, doe_before, ted_before, seq1, fold_seq_row, fold_seqless ORDER BY 1, 2, 3, 4, 5, 6 LIMIT 100"""
open(f'{D}/verify/sql/v2.sql', 'w').write(V2.replace("\n", " ") + "\n")
with open(f'{D}/verify/plan_v.sql', 'w') as f:
    f.write("-- name: v1\n" + open(f'{D}/verify/sql/v1.sql').read() + "\n-- name: v2\n" + open(f'{D}/verify/sql/v2.sql').read())
print(len(arms), os.path.getsize(f'{D}/verify/sql/v1.sql'))
