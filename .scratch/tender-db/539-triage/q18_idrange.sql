SELECT n.fetch_id AS f, n.source AS source, n.profile AS profile,
       (n.published_at + ifnull(n.published_offset, 0) * 60) / 86400 AS pcd,
       n.dispatched_at IS NOT NULL AS d, n.published_at IS NULL AS pnull,
       count(*) AS n,
       min((n.dispatched_at + ifnull(n.dispatched_offset, 0) * 60) / 86400) AS dmin,
       max((n.dispatched_at + ifnull(n.dispatched_offset, 0) * 60) / 86400) AS dmax,
       min(n.id) AS min_id, max(n.id) AS max_id
  FROM notices n
 WHERE n.id >= 26100000 AND n.id <= 26110000
 GROUP BY +n.fetch_id, +n.source, +n.profile, pcd, d, pnull
