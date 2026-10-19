# Generates the unit-5 statements (issue 562). One file per (query, chunk).
# Range (958810037, 958848408], chunks of <= 5,000 change rows.
import os, sys
D = os.path.dirname(os.path.abspath(__file__))
LO, HI = 958810037, 958848408
chunks = []
a = LO
while a < HI:
    b = min(a + 5000, HI)
    chunks.append((a, b)); a = b

SUM = """SELECT c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi, MIN(c.changed_at) AS at_lo, MAX(c.changed_at) AS at_hi, COUNT(DISTINCT c.changed_at) AS ats FROM changes c WHERE c.cursor > {a} AND c.cursor <= {b} GROUP BY +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3 LIMIT 50"""

# every seq-less tender 'changed' row in the chunk, with its Tender and the seq-carrying rows paired to it
SEQLESS = """SELECT c.cursor, c.entity_id AS tender_id, c.changed_at, t.source, t.kind, t.created_at, t.current_seq, (SELECT COUNT(*) FROM changes c2 WHERE c2.entity_kind = 'tender' AND c2.entity_id = c.entity_id AND c2.version_seq IS NOT NULL AND c2.changed_at = c.changed_at AND c2.cursor > {LO} AND c2.cursor < c.cursor) AS paired_seq_rows, (SELECT MIN(c2.version_seq) FROM changes c2 WHERE c2.entity_kind = 'tender' AND c2.entity_id = c.entity_id AND c2.version_seq IS NOT NULL AND c2.changed_at = c.changed_at AND c2.cursor > {LO} AND c2.cursor < c.cursor) AS paired_min_seq, (SELECT COUNT(*) FROM tender_version_lots l WHERE l.tender_id = c.entity_id AND l.seq = t.current_seq) AS head_lots FROM changes c CROSS JOIN tenders t ON t.id = c.entity_id WHERE c.cursor > {a} AND c.cursor <= {b} AND +c.entity_kind = 'tender' AND c.version_seq IS NULL AND +c.op = 'changed' ORDER BY c.cursor LIMIT 1000"""

# every seq-carrying tender row whose version is caused by a TED notice, on a Tender whose chain holds a DOE notice
TED = """SELECT c.cursor, c.entity_id AS tender_id, c.version_seq AS seq, c.op, c.changed_at, n.id AS nid, n.ingested_at AS n_ingested_at, t.created_at, t.source, t.current_seq, EXISTS (SELECT 1 FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = c.entity_id AND n2.id < n.id AND +n2.source = 'doe') AS doe_before, EXISTS (SELECT 1 FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = c.entity_id AND n2.id < n.id AND +n2.source = 'ted') AS ted_before, (SELECT MIN(n2.id) FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = c.entity_id AND +n2.source = 'doe') AS doe_min_nid, (SELECT MIN(c2.cursor) FROM changes c2 WHERE c2.entity_kind = 'tender' AND c2.entity_id = c.entity_id AND c2.version_seq IS NULL AND c2.op = 'changed' AND c2.changed_at = c.changed_at AND c2.cursor > c.cursor AND c2.cursor <= {HI}) AS seqless_cursor FROM changes c CROSS JOIN tender_versions v ON v.tender_id = c.entity_id AND v.seq = c.version_seq CROSS JOIN notices n ON n.id = v.caused_by_notice_id CROSS JOIN tenders t ON t.id = c.entity_id WHERE c.cursor > {a} AND c.cursor <= {b} AND +c.entity_kind = 'tender' AND c.version_seq IS NOT NULL AND +n.source = 'ted' AND EXISTS (SELECT 1 FROM tender_versions v3 CROSS JOIN notices n3 ON n3.id = v3.caused_by_notice_id WHERE v3.tender_id = c.entity_id AND +n3.source = 'doe') ORDER BY c.cursor LIMIT 2000"""

plan = []
for i, (a, b) in enumerate(chunks):
    for name, tpl in (("sum", SUM), ("seqless", SEQLESS), ("ted", TED)):
        sql = tpl.format(a=a, b=b, LO=LO, HI=HI)
        fn = f"{name}_{i}.sql"
        open(os.path.join(D, "sql", fn), "w").write(sql + "\n")
        plan.append(f"-- name: {name}_{i}\n{sql}\n")
open(os.path.join(D, "plan_statements.sql"), "w").write("\n".join(plan))
print(chunks)
