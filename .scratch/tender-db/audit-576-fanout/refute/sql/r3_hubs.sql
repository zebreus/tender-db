WITH probe(x) AS (VALUES (31974991),(31975006),(31975008))
SELECT p.x AS org, b.organization_id AS parent_org, count(*) AS n, count(DISTINCT m.notice_id) AS notices,
 min(b.name) AS parent_name_min, max(b.name) AS parent_name_max,
 (SELECT po.identifier IS NOT NULL FROM organizations po WHERE po.id = b.organization_id) AS parent_identified,
 min((SELECT min(t.value) FROM notice_texts t WHERE t.notice_id = m.notice_id AND t.section_id = m.section_id AND t.field_id = 'BT-513-Organization-TouchPoint')) AS city_min,
 max((SELECT max(t.value) FROM notice_texts t WHERE t.notice_id = m.notice_id AND t.section_id = m.section_id AND t.field_id = 'BT-513-Organization-TouchPoint')) AS city_max
FROM probe p
CROSS JOIN organization_mentions m ON m.organization_id = p.x
CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
GROUP BY +p.x, +b.organization_id
