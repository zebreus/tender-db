SELECT 439 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19509 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19478 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19448 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19298 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19113 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19509 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19478 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 439
 GROUP BY +n.profile, pb, db
