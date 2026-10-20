# Issue 562 unit 5, second half: the 2026-10-20 daily fold (project job 2669).
# Same method and SQL shape as ../gen.py (first half), with one extra step first:
# the chain's cursor range (958848784, 958904203] also holds rederive-eur-recent 2668's
# D5 rows (before the fold) and reveal-recheck 2670's rows (after it), so phase 1 maps
# changed_at -> cursor over the whole range, and phase 2 reads the paired queries over the
# project's own sub-range (PLO, PHI] only.
#
#   python3 gen.py p1                 -> sql/sum_<i>.sql over the whole range
#   python3 gen.py edge <name> <lo> <hi>   -> sql/<name>.sql (rows (lo, hi], <= 30)
#   python3 gen.py p2 <PLO> <PHI>     -> sql/seqless_<i>.sql, sql/ted_<i>.sql over (PLO, PHI]
# Every statement is also appended to plan_<phase>.sql for the local plan read.
import os, sys
D = os.path.dirname(os.path.abspath(__file__))
LO, HI = 958848784, 958904203          # the whole 10-20 chain (task-supplied cursors)
STEP = 5000                             # <= 5,000 change rows per statement, as in the first half


def chunks(lo, hi):
    out, a = [], lo
    while a < hi:
        b = min(a + STEP, hi)
        out.append((a, b)); a = b
    return out


# Phase 1: the first half's SUM, grouped also by changed_at so the writers' sub-ranges show.
SUM = """SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > {a} AND c.cursor <= {b} GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400"""

EDGE = """SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes WHERE cursor > {a} AND cursor <= {b} ORDER BY cursor LIMIT 30"""

# Phase 2: the first half's SEQLESS, plus d5_rows (the Tender has a seq-less tender row from
# rederive-eur-recent 2668 in (LO, PLO], i.e. it is one of the 913 stamped epoch-stale and
# refolded by this fold under compare On: ADR-0017 D3's acting set, counted as 'corrected').
SEQLESS = """SELECT c.cursor, c.entity_id AS tender_id, c.changed_at, t.source, t.kind, t.created_at, t.current_seq, (SELECT COUNT(*) FROM changes c2 WHERE c2.entity_kind = 'tender' AND c2.entity_id = c.entity_id AND c2.version_seq IS NOT NULL AND c2.changed_at = c.changed_at AND c2.cursor > {PLO} AND c2.cursor < c.cursor) AS paired_seq_rows, (SELECT MIN(c2.version_seq) FROM changes c2 WHERE c2.entity_kind = 'tender' AND c2.entity_id = c.entity_id AND c2.version_seq IS NOT NULL AND c2.changed_at = c.changed_at AND c2.cursor > {PLO} AND c2.cursor < c.cursor) AS paired_min_seq, (SELECT COUNT(*) FROM tender_version_lots l WHERE l.tender_id = c.entity_id AND l.seq = t.current_seq) AS head_lots, (SELECT COUNT(*) FROM changes c3 WHERE c3.entity_kind = 'tender' AND c3.entity_id = c.entity_id AND c3.version_seq IS NULL AND c3.cursor > {LO} AND c3.cursor <= {PLO}) AS d5_rows FROM changes c CROSS JOIN tenders t ON t.id = c.entity_id WHERE c.cursor > {a} AND c.cursor <= {b} AND +c.entity_kind = 'tender' AND c.version_seq IS NULL AND +c.op = 'changed' ORDER BY c.cursor LIMIT 1000"""

# The first half's TED, with the seq-less pairing bounded by PHI and the same d5_rows column.
TED = """SELECT c.cursor, c.entity_id AS tender_id, c.version_seq AS seq, c.op, c.changed_at, n.id AS nid, n.ingested_at AS n_ingested_at, t.created_at, t.source, t.current_seq, EXISTS (SELECT 1 FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = c.entity_id AND n2.id < n.id AND +n2.source = 'doe') AS doe_before, EXISTS (SELECT 1 FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = c.entity_id AND n2.id < n.id AND +n2.source = 'ted') AS ted_before, (SELECT MIN(n2.id) FROM tender_versions v2 CROSS JOIN notices n2 ON n2.id = v2.caused_by_notice_id WHERE v2.tender_id = c.entity_id AND +n2.source = 'doe') AS doe_min_nid, (SELECT MIN(c2.cursor) FROM changes c2 WHERE c2.entity_kind = 'tender' AND c2.entity_id = c.entity_id AND c2.version_seq IS NULL AND c2.op = 'changed' AND c2.changed_at = c.changed_at AND c2.cursor > c.cursor AND c2.cursor <= {PHI}) AS seqless_cursor, (SELECT COUNT(*) FROM changes c3 WHERE c3.entity_kind = 'tender' AND c3.entity_id = c.entity_id AND c3.version_seq IS NULL AND c3.cursor > {LO} AND c3.cursor <= {PLO}) AS d5_rows FROM changes c CROSS JOIN tender_versions v ON v.tender_id = c.entity_id AND v.seq = c.version_seq CROSS JOIN notices n ON n.id = v.caused_by_notice_id CROSS JOIN tenders t ON t.id = c.entity_id WHERE c.cursor > {a} AND c.cursor <= {b} AND +c.entity_kind = 'tender' AND c.version_seq IS NOT NULL AND +n.source = 'ted' AND EXISTS (SELECT 1 FROM tender_versions v3 CROSS JOIN notices n3 ON n3.id = v3.caused_by_notice_id WHERE v3.tender_id = c.entity_id AND +n3.source = 'doe') ORDER BY c.cursor LIMIT 2000"""


def emit(phase, name, sql):
    open(os.path.join(D, "sql", f"{name}.sql"), "w").write(sql + "\n")
    with open(os.path.join(D, f"plan_{phase}.sql"), "a") as f:
        f.write(f"-- name: {name}\n{sql}\n")


cmd = sys.argv[1]
if cmd == "p1":
    open(os.path.join(D, "plan_p1.sql"), "w").close()
    cs = chunks(LO, HI)
    for i, (a, b) in enumerate(cs):
        emit("p1", f"sum_{i}", SUM.format(a=a, b=b))
    print(len(cs), cs)
elif cmd == "edge":
    name, a, b = sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    assert 0 < b - a <= 30
    emit("edge", name, EDGE.format(a=a, b=b))
    print(name, a, b)
elif cmd == "p2":
    PLO, PHI = int(sys.argv[2]), int(sys.argv[3])
    assert LO <= PLO < PHI <= HI
    open(os.path.join(D, "plan_p2.sql"), "w").close()
    cs = chunks(PLO, PHI)
    for i, (a, b) in enumerate(cs):
        for name, tpl in (("seqless", SEQLESS), ("ted", TED)):
            emit("p2", f"{name}_{i}", tpl.format(a=a, b=b, LO=LO, HI=HI, PLO=PLO, PHI=PHI))
    print(len(cs), cs)
