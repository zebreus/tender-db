-- name: r1_overseas_rows
SELECT id, country, identifier_kind, identifier, provisional, created_at, name_norm FROM organizations WHERE country IN ('GP','MQ','GF','RE','YT','PM','BL','MF','WF','AX','GL','SJ') LIMIT 300
-- name: r3_register_twins
SELECT id, name_norm, country, provisional, created_at FROM organizations WHERE name_norm IN ('a', 'b') AND country IN ('FR', 'FI') AND identifier IS NULL LIMIT 300
-- name: r4_changes_eq
SELECT entity_id, op, changed_at, cursor FROM changes WHERE entity_kind = 'organization' AND entity_id = 5 LIMIT 300
-- name: r4_changes_plus
SELECT entity_id, op, changed_at, cursor FROM changes WHERE +entity_kind = 'organization' AND entity_id IN (1, 2, 3) LIMIT 300
-- name: r4_changes_indexed
SELECT entity_id, op, changed_at, cursor FROM changes INDEXED BY changes_entity WHERE entity_kind = 'organization' AND entity_id IN (1, 2, 3) LIMIT 300
