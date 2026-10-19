SELECT 444 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19358 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19327 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19326 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19297 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19147 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 18962 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19358 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19327 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 444
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 443 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19389 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19358 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19357 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19328 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19178 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 18993 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19389 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19358 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 443
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 442 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19417 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19389 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19388 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19359 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19209 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19024 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19417 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19389 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 442
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 441 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19448 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19417 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19416 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19387 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19237 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19052 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19448 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19417 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 441
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 440 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19478 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19448 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19447 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19418 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19268 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19083 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19478 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19448 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 440
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 439 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19509 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19478 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19477 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19448 THEN 'b002'
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
UNION ALL
SELECT 438 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19539 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19509 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19508 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19479 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19329 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19144 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19539 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19509 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 438
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 437 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19570 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19539 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19538 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19509 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19359 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19174 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19570 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19539 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 437
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 436 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19601 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19570 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19569 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19540 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19390 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19205 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19601 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19570 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 436
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 435 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19631 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19601 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19600 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19571 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19421 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19236 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19631 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19601 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 435
 GROUP BY +n.profile, pb, db
UNION ALL
SELECT 434 AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19662 THEN 'after'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) >= 19631 THEN 'in'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) = 19630 THEN 'b001'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19601 THEN 'b002'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19451 THEN 'b030'
       WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 19266 THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19662 THEN 'after'
       WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) >= 19631 THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400) > 20654 THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min(((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = 434
 GROUP BY +n.profile, pb, db
