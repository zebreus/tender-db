-- name: legacy440_lookup
-- params: ["sdis de la reunion", "RE"]
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- name: resolver_lookup
-- params: ["sdis de la reunion", "FR"]
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL ORDER BY id LIMIT 1
-- name: r1_overseas_rows
SELECT id, country, identifier_kind, identifier, provisional, created_at, name_norm FROM organizations WHERE country IN ('GP','MQ','GF','RE','YT','PM','BL','MF','WF','AX','GL','SJ') ORDER BY country, id LIMIT 300
-- name: r2_mentions
SELECT organization_id, COUNT(*), MIN(notice_id), MAX(notice_id), group_concat(DISTINCT country) FROM organization_mentions WHERE organization_id IN (1, 2, 3) GROUP BY organization_id
-- name: r3_register_twins
SELECT id, name_norm, country, provisional, created_at FROM organizations WHERE name_norm IN ('a', 'b') AND country IN ('FR', 'FI') AND identifier IS NULL ORDER BY name_norm, id LIMIT 300
-- name: r4_changes
SELECT entity_id, op, changed_at, cursor FROM changes WHERE entity_kind = 'organization' AND entity_id IN (1, 2, 3) LIMIT 300
