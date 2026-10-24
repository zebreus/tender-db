SELECT t.id AS tid,
       t.current_seq AS cur_seq,
       COUNT(*) AS nv,
       MIN(a.seq) AS min_seq,
       MAX(a.seq) AS max_seq,
       SUM(CASE WHEN na.id IS NULL THEN 1 ELSE 0 END) AS n_nonotice,
       SUM(CASE WHEN na.source = 'doe' THEN 1 ELSE 0 END) AS n_doe,
       SUM(CASE WHEN na.source = 'ted' THEN 1 ELSE 0 END) AS n_ted,
       MAX(CASE WHEN a.seq = t.current_seq THEN a.published_at END) - t.current_published_at AS head_pub_delta,
       COUNT(b.seq) AS pairs,
       SUM(CASE WHEN b.seq IS NOT NULL AND (na.source = 'doe' OR nb.source = 'doe') THEN 1 ELSE 0 END) AS pairs_doe,
       SUM(CASE WHEN b.seq IS NOT NULL AND a.published_at = b.published_at THEN 1 ELSE 0 END) AS pairs_same_instant,
       SUM(CASE WHEN b.seq IS NOT NULL AND na.source <> nb.source THEN 1 ELSE 0 END) AS pairs_cross_source,
       SUM(CASE WHEN b.seq IS NOT NULL AND (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at > b.published_at THEN 1 ELSE 0 END) AS vd_doe,
       SUM(CASE WHEN b.seq IS NOT NULL AND (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at = b.published_at
                 AND ((na.source = 'ted') > (nb.source = 'ted')
                      OR ((na.source = 'ted') = (nb.source = 'ted')
                          AND (a.publication_id > b.publication_id
                               OR (a.publication_id = b.publication_id AND a.caused_by_notice_id > b.caused_by_notice_id))))
                THEN 1 ELSE 0 END) AS vt_doe,
       SUM(CASE WHEN b.seq IS NOT NULL AND NOT (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at > b.published_at THEN 1 ELSE 0 END) AS vd_non,
       SUM(CASE WHEN b.seq IS NOT NULL AND NOT (na.source = 'doe' OR nb.source = 'doe')
                 AND a.published_at = b.published_at
                 AND ((na.source = 'ted') > (nb.source = 'ted')
                      OR ((na.source = 'ted') = (nb.source = 'ted')
                          AND (a.publication_id > b.publication_id
                               OR (a.publication_id = b.publication_id AND a.caused_by_notice_id > b.caused_by_notice_id))))
                THEN 1 ELSE 0 END) AS vt_non,
       MIN(CASE WHEN b.seq IS NOT NULL
                 AND (a.published_at > b.published_at
                      OR (a.published_at = b.published_at
                          AND ((na.source = 'ted') > (nb.source = 'ted')
                               OR ((na.source = 'ted') = (nb.source = 'ted')
                                   AND (a.publication_id > b.publication_id
                                        OR (a.publication_id = b.publication_id AND a.caused_by_notice_id > b.caused_by_notice_id))))))
                THEN a.seq END) AS first_viol_seq
  FROM tenders t
 CROSS JOIN tender_versions a INDEXED BY sqlite_autoindex_tender_versions_1 ON a.tender_id = t.id
  LEFT JOIN notices na ON na.id = a.caused_by_notice_id
  LEFT JOIN tender_versions b INDEXED BY sqlite_autoindex_tender_versions_1 ON b.tender_id = a.tender_id AND b.seq = a.seq + 1
  LEFT JOIN notices nb ON nb.id = b.caused_by_notice_id
 WHERE t.id BETWEEN 178001 AND 183000
 GROUP BY t.id
 LIMIT 6000
