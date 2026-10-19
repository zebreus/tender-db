SELECT 433 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19692 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19662 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19661 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19632 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19482 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19297 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19692 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19662 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 433
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 432 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19723 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19692 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19691 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19662 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19512 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19327 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19723 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19692 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 432
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 431 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19754 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19723 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19722 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19693 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19543 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19358 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19754 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19723 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 431
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 430 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19783 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19754 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19753 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19724 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19574 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19389 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19783 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19754 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 430
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 429 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19814 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19783 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19782 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19753 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19603 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19418 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19814 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19783 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 429
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 428 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19844 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19814 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19813 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19784 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19634 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19449 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19844 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19814 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 428
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 427 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19875 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19844 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19843 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19814 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19664 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19479 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19875 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19844 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 427
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 426 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19905 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19875 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19874 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19845 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19695 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19510 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19905 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19875 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 426
 GROUP BY +n.profile, pb, db
