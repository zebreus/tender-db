SELECT m.organization_id AS org, o.created_at AS ocreated, o.country AS occ, o.identifier_kind AS okind, o.name AS oname,
  b.organization_id AS porg, p.identifier_kind AS pkind, p.name AS porg_name,
  count(*) AS n, count(DISTINCT s.notice_id) AS notices, min(m.name) AS tp_printed, min(b.name) AS p_printed,
  sum(k.code IS NULL) AS no_own_cc
FROM notice_sections s
CROSS JOIN organization_mentions m ON m.notice_id = s.notice_id AND m.section_id = s.section_id
CROSS JOIN organizations o ON o.id = m.organization_id
LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
LEFT JOIN organizations p ON p.id = b.organization_id
LEFT JOIN notice_codes k ON k.notice_id = s.notice_id AND k.section_id = s.section_id
  AND k.field_id = 'BT-514-Organization-TouchPoint' AND k.ordinal = 0
WHERE s.kind = 'Touchpoint' AND s.notice_id BETWEEN 47420000 AND 47439999
GROUP BY +m.organization_id, +b.organization_id
