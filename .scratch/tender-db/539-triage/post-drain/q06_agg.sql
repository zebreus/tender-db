SELECT 413 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20301 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20270 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20269 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20240 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20090 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19905 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20301 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20270 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 413
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 412 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20332 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20301 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20300 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20271 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20121 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19936 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20332 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20301 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 412
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 411 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20362 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20332 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20331 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20302 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20152 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19967 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20362 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20332 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 411
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 410 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20393 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20362 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20361 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20332 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20182 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19997 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20393 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20362 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 410
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 409 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20423 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20393 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20392 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20363 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20213 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20028 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20423 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20393 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 409
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 408 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20454 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20423 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20422 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20393 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20243 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20058 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20454 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20423 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 408
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 407 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20485 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20454 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20453 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20424 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20274 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20089 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20485 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20454 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 407
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 406 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20513 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20485 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20484 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20455 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20305 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20120 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20513 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20485 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 406
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 405 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20544 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20513 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20512 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20483 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20333 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20148 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20544 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20513 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 405
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 404 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20574 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20544 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20543 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20514 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20364 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20179 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20574 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20544 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 404
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 403 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20605 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20574 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20573 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20544 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20394 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20209 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20605 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20574 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 403
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 4 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20635 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20605 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20604 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20575 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20425 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20240 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20635 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20605 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20653 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 4
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 402 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20666 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 20635 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 20634 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20605 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20455 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20270 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20666 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 20635 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 402
 GROUP BY +n.profile, pb, db
