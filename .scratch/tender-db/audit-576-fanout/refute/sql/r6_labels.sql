WITH probe(x) AS (VALUES (31975060),(31976915),(31975176),(31975059),(31975783),(31975920),(31975556),(31975859),(31977749),(31976920))
SELECT p.x AS org, b.organization_id AS parent_org, count(*) AS n, min(b.name) AS parent_name,
 min((SELECT min(t.value) FROM notice_texts t WHERE t.notice_id = m.notice_id AND t.section_id = m.section_id AND t.field_id = 'BT-513-Organization-TouchPoint')) AS city
FROM probe p
CROSS JOIN organization_mentions m ON m.organization_id = p.x
CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
GROUP BY +p.x, +b.organization_id
