SELECT o.id, o.created_at, o.country, o.identifier_kind, o.identifier, o.provisional, o.name, o.name_norm,
 (SELECT count(*) FROM organization_mentions m WHERE m.organization_id = o.id) AS mentions,
 (SELECT count(*) FROM organization_mentions m WHERE m.organization_id = o.id AND m.section_id LIKE 'TPO-%') AS tpo_mentions
FROM organizations o WHERE o.id BETWEEN 31974376 AND 31984375 ORDER BY o.id
