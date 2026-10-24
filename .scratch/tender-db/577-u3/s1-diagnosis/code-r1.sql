SELECT 'p' AS t, p.rowid AS rid, p.tender_id AS tid, p.seq AS seq, p.lot_id AS x, p.role AS role, p.organization_id AS org, p.mention_notice_id AS nid, p.mention_section_id AS sid, n.profile AS profile
  FROM tender_version_parties p JOIN notices n ON n.id = p.mention_notice_id
 WHERE p.tender_id IN (2, 26)
UNION ALL
SELECT 'b', b.rowid, b.tender_id, b.seq, b.bid_id, b.role, b.organization_id, b.mention_notice_id, b.mention_section_id, n.profile
  FROM tender_version_bid_parties b JOIN notices n ON n.id = b.mention_notice_id
 WHERE b.tender_id IN (2, 26)
UNION ALL
SELECT 'v', v.rowid, v.tender_id, v.seq, NULL, NULL, NULL, v.caused_by_notice_id, NULL, n.profile
  FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id
 WHERE v.tender_id IN (2, 26)
UNION ALL
SELECT 'm', m.rowid, NULL, NULL, NULL, NULL, m.organization_id, m.notice_id, m.section_id, NULL
  FROM organization_mentions m
 WHERE m.notice_id IN (26213590, 26198825, 26290799)
LIMIT 800
