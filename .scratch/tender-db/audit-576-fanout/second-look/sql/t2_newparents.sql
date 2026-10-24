SELECT o.id, m.notice_id, m.section_id, m.country AS tp_cc,
  (SELECT k.code FROM notice_codes k WHERE k.notice_id = m.notice_id AND k.section_id = m.section_id
     AND k.field_id = 'BT-514-Organization-TouchPoint' LIMIT 1) AS tp_own_cc,
  s.parent_section_id AS psec, b.organization_id AS porg, b.name AS p_printed, b.country AS p_printed_cc,
  p.name AS porg_name, p.country AS porg_cc, p.identifier_kind AS pkind, p.provisional AS pprov, p.created_at AS pcreated
FROM organizations o
CROSS JOIN organization_mentions m ON m.rowid = (SELECT min(y.rowid) FROM organization_mentions y WHERE y.organization_id = o.id)
CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
LEFT JOIN organizations p ON p.id = b.organization_id
WHERE o.id BETWEEN 31974972 AND 31978722
