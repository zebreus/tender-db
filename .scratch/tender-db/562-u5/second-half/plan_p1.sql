-- name: sum_0
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958848784 AND c.cursor <= 958853784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_1
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958853784 AND c.cursor <= 958858784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_2
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958858784 AND c.cursor <= 958863784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_3
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958863784 AND c.cursor <= 958868784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_4
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958868784 AND c.cursor <= 958873784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_5
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958873784 AND c.cursor <= 958878784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_6
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958878784 AND c.cursor <= 958883784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_7
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958883784 AND c.cursor <= 958888784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_8
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958888784 AND c.cursor <= 958893784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_9
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958893784 AND c.cursor <= 958898784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_10
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958898784 AND c.cursor <= 958903784 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
-- name: sum_11
SELECT c.changed_at AS at, c.entity_kind AS kind, c.version_seq IS NULL AS seqless, c.op AS op, COUNT(*) AS n, MIN(c.cursor) AS lo, MAX(c.cursor) AS hi FROM changes c WHERE c.cursor > 958903784 AND c.cursor <= 958904203 GROUP BY +c.changed_at, +c.entity_kind, c.version_seq IS NULL, +c.op ORDER BY 1, 2, 3, 4 LIMIT 400
