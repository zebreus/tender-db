SELECT 425 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19936 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19905 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19904 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19875 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19725 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19540 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19936 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19905 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 425
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 424 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19967 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19936 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19935 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19906 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19756 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19571 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19967 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19936 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 424
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 423 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19997 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19967 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19966 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19937 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19787 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19602 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19997 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19967 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 423
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 422 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20028 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19997 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19996 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19967 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19817 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19632 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20028 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19997 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 422
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 421 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20058 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20028 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20027 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19998 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19848 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19663 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20058 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20028 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 421
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 420 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20089 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20058 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20057 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20028 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19878 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19693 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20089 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20058 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 420
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 419 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20120 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20089 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20088 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20059 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19909 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19724 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20120 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20089 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 419
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 418 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20148 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20120 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20119 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20090 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19940 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19755 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20148 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20120 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 418
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 417 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20179 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20148 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20147 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20118 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19968 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19783 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20179 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20148 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 417
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 416 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20209 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20179 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20178 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20149 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19999 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19814 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20209 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20179 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 416
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 415 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20240 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20209 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20208 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20179 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20029 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19844 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20240 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20209 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 415
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 414 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20270 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20240 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20239 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20210 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20060 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19875 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20270 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20240 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 414
 GROUP BY +n.profile, pb, db
