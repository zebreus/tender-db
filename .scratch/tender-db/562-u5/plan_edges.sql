-- name: edge_lo
SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes WHERE cursor > 958810027 AND cursor <= 958810047 ORDER BY cursor LIMIT 30
-- name: edge_hi
SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes WHERE cursor > 958848398 AND cursor <= 958848418 ORDER BY cursor LIMIT 30
