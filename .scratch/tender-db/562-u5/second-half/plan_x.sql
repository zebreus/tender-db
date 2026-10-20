-- name: x_8853588_versions
SELECT v.seq, v.caused_by_notice_id AS nid, n.source AS n_source, n.ingested_at, n.published_at, n.parse_state, n.projected, t.created_at, t.source, t.kind, t.current_seq, t.island_notice_id, t.procedure_key FROM tenders t CROSS JOIN tender_versions v ON v.tender_id = t.id CROSS JOIN notices n ON n.id = v.caused_by_notice_id WHERE t.id = 8853588 ORDER BY v.seq LIMIT 20
-- name: x_8853588_changes
SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes WHERE entity_kind = 'tender' AND entity_id = 8853588 ORDER BY cursor LIMIT 20
-- name: x_removed_tenders
SELECT c.cursor, c.entity_id, c.op, c.changed_at, t.id AS t_id, t.source, t.island_notice_id, t.procedure_key, t.created_at, t.current_seq FROM changes c LEFT JOIN tenders t ON t.id = c.entity_id WHERE c.cursor > 958853608 AND c.cursor <= 958858608 AND +c.entity_kind = 'tender' AND c.version_seq IS NULL AND +c.op = 'removed' ORDER BY c.cursor LIMIT 50
-- name: x_8852015_changes
SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes WHERE entity_kind = 'tender' AND entity_id = 8852015 ORDER BY cursor LIMIT 20
