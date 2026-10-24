#!/usr/bin/env python3
# Writes i2/i3 .sql: one aggregate row per window — does each stored version instant equal its causing notice's
# stored instant? The fold stamps both with the same resolver (project.rs notice_instants; the version keeps the
# epoch-0 fallback where the notice row is NULL), so after repair-notice-instants (notice rows) and the R3 refolds
# (version rows) a DÖE version that disagrees with its notice row is a version the drain did not re-derive.
WINDOWS = {'i2': (8811001, 8816000), 'i3': (248001, 253000)}
TPL = """SELECT MIN(t.id) AS min_tid,
       MAX(t.id) AS max_tid,
       COUNT(*) AS versions,
       SUM(CASE WHEN na.id IS NULL THEN 1 ELSE 0 END) AS no_notice,
       SUM(CASE WHEN na.source = 'doe' THEN 1 ELSE 0 END) AS doe_versions,
       SUM(CASE WHEN na.source = 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN 1 ELSE 0 END) AS doe_pub_mismatch,
       SUM(CASE WHEN na.source = 'doe' AND a.dispatched_at IS NOT na.dispatched_at THEN 1 ELSE 0 END) AS doe_disp_mismatch,
       SUM(CASE WHEN na.source = 'doe' AND na.published_at IS NULL THEN 1 ELSE 0 END) AS doe_notice_pub_null,
       MIN(CASE WHEN na.source = 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS doe_mm_min_tid,
       MAX(CASE WHEN na.source = 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS doe_mm_max_tid,
       SUM(CASE WHEN na.source <> 'doe' THEN 1 ELSE 0 END) AS non_versions,
       SUM(CASE WHEN na.source <> 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN 1 ELSE 0 END) AS non_pub_mismatch,
       SUM(CASE WHEN na.source <> 'doe' AND a.dispatched_at IS NOT na.dispatched_at THEN 1 ELSE 0 END) AS non_disp_mismatch,
       MIN(CASE WHEN na.source <> 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS non_mm_min_tid,
       MAX(CASE WHEN na.source <> 'doe' AND a.published_at <> COALESCE(na.published_at, 0) THEN t.id END) AS non_mm_max_tid
  FROM tenders t
 CROSS JOIN tender_versions a INDEXED BY sqlite_autoindex_tender_versions_1 ON a.tender_id = t.id
  LEFT JOIN notices na ON na.id = a.caused_by_notice_id
 WHERE t.id BETWEEN {lo} AND {hi}
"""
for name, (lo, hi) in WINDOWS.items():
    assert hi - lo + 1 <= 5000
    open(f'{name}.sql', 'w').write(TPL.format(lo=lo, hi=hi))
    print(name, lo, hi)
