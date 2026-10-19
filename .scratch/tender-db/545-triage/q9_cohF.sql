SELECT 'org:bedford' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e FROM organizations WHERE name_norm >= 'bedford borough council' AND name_norm < 'bedford borough councim'
UNION ALL
SELECT 'org:ddm' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e FROM organizations WHERE name_norm >= 'ddm health' AND name_norm < 'ddm healti'
UNION ALL
SELECT 'org:diabetes' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e FROM organizations WHERE name_norm >= 'diabetes uk' AND name_norm < 'diabetes ul'
UNION ALL
SELECT 'org:autocross' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e FROM organizations WHERE name_norm >= 'autocross euroshell' AND name_norm < 'autocross euroshelm'
UNION ALL
SELECT 'org:luc' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e FROM organizations WHERE name_norm >= 'land use consultants' AND name_norm < 'land use consultantt'
UNION ALL
SELECT 'org:cactus' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e FROM organizations WHERE name_norm >= 'london cactus' AND name_norm < 'london cactut'
UNION ALL
SELECT 'role:bedford' AS k, tender_id AS a, seq AS b, role AS c, lot_id AS d, mention_section_id AS e FROM tender_version_parties WHERE organization_id = 30968615 AND mention_notice_id = 47300761
UNION ALL
SELECT 'role:ddm' AS k, tender_id AS a, seq AS b, role AS c, lot_id AS d, mention_section_id AS e FROM tender_version_parties WHERE organization_id = 31568872 AND mention_notice_id = 46890437
UNION ALL
SELECT 'role:diabetes' AS k, tender_id AS a, seq AS b, role AS c, lot_id AS d, mention_section_id AS e FROM tender_version_parties WHERE organization_id = 30926702 AND mention_notice_id = 46786666
UNION ALL
SELECT 'role:autocross' AS k, tender_id AS a, seq AS b, role AS c, lot_id AS d, mention_section_id AS e FROM tender_version_parties WHERE organization_id = 30968919 AND mention_notice_id = 46933272
UNION ALL
SELECT 'role:luc' AS k, tender_id AS a, seq AS b, role AS c, lot_id AS d, mention_section_id AS e FROM tender_version_parties WHERE organization_id = 31551788 AND mention_notice_id = 46762813
UNION ALL
SELECT 'role:cactus' AS k, tender_id AS a, seq AS b, role AS c, lot_id AS d, mention_section_id AS e FROM tender_version_parties WHERE organization_id = 30932866 AND mention_notice_id = 46842859
