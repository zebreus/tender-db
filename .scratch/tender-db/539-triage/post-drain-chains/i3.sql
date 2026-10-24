SELECT MIN(t.id) AS min_tid,
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
 WHERE t.id BETWEEN 248001 AND 253000
