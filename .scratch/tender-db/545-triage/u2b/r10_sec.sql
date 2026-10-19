SELECT 'j|30913587|0' AS k, m.notice_id AS a, m.section_id AS b, m.name AS c, m.country AS d, p.tender_id AS e, p.seq AS f, p.lot_id AS g, p.role AS h FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30913587 AND +m.name = 'Royal College of Psychiatrists'
UNION ALL
SELECT 'k|30913587|0', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30913587 AND +m.name = 'Royal College of Psychiatrists'
UNION ALL
SELECT 'j|30913851|1', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30913851 AND +m.name = 'London Borough of Hounslow'
UNION ALL
SELECT 'k|30913851|1', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30913851 AND +m.name = 'London Borough of Hounslow'
UNION ALL
SELECT 'j|30914001|2', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914001 AND +m.name = 'University of Liverpool'
UNION ALL
SELECT 'k|30914001|2', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914001 AND +m.name = 'University of Liverpool'
UNION ALL
SELECT 'j|30914115|3', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914115 AND +m.name = 'Stockton on Tees Borough Council'
UNION ALL
SELECT 'k|30914115|3', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914115 AND +m.name = 'Stockton on Tees Borough Council'
UNION ALL
SELECT 'j|30914436|4', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914436 AND +m.name = 'GLEEDS COST MANAGEMENT LIMITED'
UNION ALL
SELECT 'k|30914436|4', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914436 AND +m.name = 'GLEEDS COST MANAGEMENT LIMITED'
UNION ALL
SELECT 'j|30914453|5', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914453 AND +m.name = 'Ballicom Limited'
UNION ALL
SELECT 'k|30914453|5', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914453 AND +m.name = 'Ballicom Limited'
UNION ALL
SELECT 'j|30914469|6', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914469 AND +m.name = 'UK Centre for Ecology & Hydrology'
UNION ALL
SELECT 'k|30914469|6', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30914469 AND +m.name = 'UK Centre for Ecology & Hydrology'
UNION ALL
SELECT 'j|30915240|7', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30915240 AND +m.name = 'Prosci UK Limited'
UNION ALL
SELECT 'k|30915240|7', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30915240 AND +m.name = 'Prosci UK Limited'
UNION ALL
SELECT 'j|30915302|8', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30915302 AND +m.name = 'Eastern Counties Scaffodling Services'
UNION ALL
SELECT 'k|30915302|8', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30915302 AND +m.name = 'Eastern Counties Scaffodling Services'
UNION ALL
SELECT 'j|30915848|9', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30915848 AND +m.name = 'Pullman Rail Ltd'
UNION ALL
SELECT 'k|30915848|9', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30915848 AND +m.name = 'Pullman Rail Ltd'
UNION ALL
SELECT 'j|30916544|10', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30916544 AND +m.name = 'TSI Instruments Ltd'
UNION ALL
SELECT 'k|30916544|10', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30916544 AND +m.name = 'TSI Instruments Ltd'
UNION ALL
SELECT 'j|30916932|11', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30916932 AND +m.name = 'PwC LLP'
UNION ALL
SELECT 'k|30916932|11', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30916932 AND +m.name = 'PwC LLP'
UNION ALL
SELECT 'j|30916935|12', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30916935 AND +m.name = 'CAPITA RESOURCING LTD (T/A VEREDUS)'
UNION ALL
SELECT 'k|30916935|12', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30916935 AND +m.name = 'CAPITA RESOURCING LTD (T/A VEREDUS)'
UNION ALL
SELECT 'j|30917637|13', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30917637 AND +m.name = 'Create Partnership Trust '
UNION ALL
SELECT 'k|30917637|13', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30917637 AND +m.name = 'Create Partnership Trust '
UNION ALL
SELECT 'j|30918064|14', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30918064 AND +m.name = 'COUNTY DURHAM AND DARLINGTON NHS FOUNDATION TRUST'
UNION ALL
SELECT 'k|30918064|14', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30918064 AND +m.name = 'COUNTY DURHAM AND DARLINGTON NHS FOUNDATION TRUST'
UNION ALL
SELECT 'j|30918064|15', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30918064 AND +m.name = 'County Durham and Darlington NHS Foundation Trust'
UNION ALL
SELECT 'k|30918064|15', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30918064 AND +m.name = 'County Durham and Darlington NHS Foundation Trust'
UNION ALL
SELECT 'j|30926702|16', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30926702 AND +m.name = 'Diabetes UK'
UNION ALL
SELECT 'k|30926702|16', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30926702 AND +m.name = 'Diabetes UK'
UNION ALL
SELECT 'j|30926961|17', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30926961 AND +m.name = 'HOSPITAL INNOVATIONS LIMITED'
UNION ALL
SELECT 'k|30926961|17', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30926961 AND +m.name = 'HOSPITAL INNOVATIONS LIMITED'
UNION ALL
SELECT 'j|30930397|18', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30930397 AND +m.name = 'Don Whitley Scientific'
UNION ALL
SELECT 'k|30930397|18', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30930397 AND +m.name = 'Don Whitley Scientific'
UNION ALL
SELECT 'j|30932866|19', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30932866 AND +m.name = 'London Cactus'
UNION ALL
SELECT 'k|30932866|19', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30932866 AND +m.name = 'London Cactus'
UNION ALL
SELECT 'j|30953724|20', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30953724 AND +m.name = 'Newcastle City Council'
UNION ALL
SELECT 'k|30953724|20', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30953724 AND +m.name = 'Newcastle City Council'
UNION ALL
SELECT 'j|30968615|21', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30968615 AND +m.name = 'Bedford Borough Council'
UNION ALL
SELECT 'k|30968615|21', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30968615 AND +m.name = 'Bedford Borough Council'
UNION ALL
SELECT 'j|30968919|22', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30968919 AND +m.name = 'Autocross Euroshell Ltd'
UNION ALL
SELECT 'k|30968919|22', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30968919 AND +m.name = 'Autocross Euroshell Ltd'
UNION ALL
SELECT 'j|30974855|23', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30974855 AND +m.name = 'BMBC'
UNION ALL
SELECT 'k|30974855|23', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30974855 AND +m.name = 'BMBC'
UNION ALL
SELECT 'j|30975996|24', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30975996 AND +m.name = 'Kingston Upon Hull City Council'
UNION ALL
SELECT 'k|30975996|24', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 30975996 AND +m.name = 'Kingston Upon Hull City Council'
UNION ALL
SELECT 'j|31535435|25', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31535435 AND +m.name = 'Big Life'
UNION ALL
SELECT 'k|31535435|25', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31535435 AND +m.name = 'Big Life'
UNION ALL
SELECT 'j|31537076|26', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31537076 AND +m.name = 'Monthind Clean Ltd'
UNION ALL
SELECT 'k|31537076|26', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31537076 AND +m.name = 'Monthind Clean Ltd'
UNION ALL
SELECT 'j|31537241|27', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31537241 AND +m.name = 'LONDON BOROUGH OF HARINGEY'
UNION ALL
SELECT 'k|31537241|27', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31537241 AND +m.name = 'LONDON BOROUGH OF HARINGEY'
UNION ALL
SELECT 'j|31537611|28', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31537611 AND +m.name = 'Accenture (UK) Ltd'
UNION ALL
SELECT 'k|31537611|28', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31537611 AND +m.name = 'Accenture (UK) Ltd'
UNION ALL
SELECT 'j|31540233|29', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31540233 AND +m.name = 'TPS Fruit & Veg Ltd'
UNION ALL
SELECT 'k|31540233|29', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31540233 AND +m.name = 'TPS Fruit & Veg Ltd'
UNION ALL
SELECT 'j|31545104|30', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31545104 AND +m.name = 'LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED'
UNION ALL
SELECT 'k|31545104|30', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31545104 AND +m.name = 'LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED'
UNION ALL
SELECT 'j|31548301|31', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31548301 AND +m.name = 'West Yorkshire Combined Authority'
UNION ALL
SELECT 'k|31548301|31', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31548301 AND +m.name = 'West Yorkshire Combined Authority'
UNION ALL
SELECT 'j|31549084|32', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31549084 AND +m.name = 'Action Deafness'
UNION ALL
SELECT 'k|31549084|32', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31549084 AND +m.name = 'Action Deafness'
UNION ALL
SELECT 'j|31551788|33', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31551788 AND +m.name = 'LAND USE CONSULTANTS LIMITED'
UNION ALL
SELECT 'k|31551788|33', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31551788 AND +m.name = 'LAND USE CONSULTANTS LIMITED'
UNION ALL
SELECT 'j|31552420|34', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31552420 AND +m.name = 'Jeremy Benn Associates Limited'
UNION ALL
SELECT 'k|31552420|34', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31552420 AND +m.name = 'Jeremy Benn Associates Limited'
UNION ALL
SELECT 'j|31553151|35', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31553151 AND +m.name = 'Oteca Ltd'
UNION ALL
SELECT 'k|31553151|35', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31553151 AND +m.name = 'Oteca Ltd'
UNION ALL
SELECT 'j|31553275|36', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31553275 AND +m.name = 'BMBC'
UNION ALL
SELECT 'k|31553275|36', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31553275 AND +m.name = 'BMBC'
UNION ALL
SELECT 'j|31557867|37', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31557867 AND +m.name = 'BTME GROUP LTD TA MEDTREE'
UNION ALL
SELECT 'k|31557867|37', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31557867 AND +m.name = 'BTME GROUP LTD TA MEDTREE'
UNION ALL
SELECT 'j|31560408|38', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31560408 AND +m.name = 'Wyre Forest District Council'
UNION ALL
SELECT 'k|31560408|38', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31560408 AND +m.name = 'Wyre Forest District Council'
UNION ALL
SELECT 'j|31568585|39', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31568585 AND +m.name = 'University of Northumbria at Newcastle'
UNION ALL
SELECT 'k|31568585|39', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31568585 AND +m.name = 'University of Northumbria at Newcastle'
UNION ALL
SELECT 'j|31568872|40', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31568872 AND +m.name = 'DDM Health Ltd'
UNION ALL
SELECT 'k|31568872|40', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31568872 AND +m.name = 'DDM Health Ltd'
UNION ALL
SELECT 'j|31577532|41', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31577532 AND +m.name = 'The Bolton Impact Trust'
UNION ALL
SELECT 'k|31577532|41', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31577532 AND +m.name = 'The Bolton Impact Trust'
UNION ALL
SELECT 'j|31577532|42', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31577532 AND +m.name = 'University of Essex'
UNION ALL
SELECT 'k|31577532|42', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31577532 AND +m.name = 'University of Essex'
UNION ALL
SELECT 'j|31580163|43', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31580163 AND +m.name = 'Generator Power Ltd'
UNION ALL
SELECT 'k|31580163|43', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31580163 AND +m.name = 'Generator Power Ltd'
UNION ALL
SELECT 'j|31582801|44', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31582801 AND +m.name = 'Dell Corporation'
UNION ALL
SELECT 'k|31582801|44', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31582801 AND +m.name = 'Dell Corporation'
UNION ALL
SELECT 'j|31594200|45', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31594200 AND +m.name = 'Ingeus UK Ltd'
UNION ALL
SELECT 'k|31594200|45', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31594200 AND +m.name = 'Ingeus UK Ltd'
UNION ALL
SELECT 'j|31597239|46', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31597239 AND +m.name = 'PULMONX UK LIMITED'
UNION ALL
SELECT 'k|31597239|46', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31597239 AND +m.name = 'PULMONX UK LIMITED'
UNION ALL
SELECT 'j|31597395|47', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31597395 AND +m.name = 'Crown Paints Ltd'
UNION ALL
SELECT 'k|31597395|47', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31597395 AND +m.name = 'Crown Paints Ltd'
UNION ALL
SELECT 'j|31606224|48', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31606224 AND +m.name = 'Board of Trustees of the Royal Botanic Gardens Kew'
UNION ALL
SELECT 'k|31606224|48', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31606224 AND +m.name = 'Board of Trustees of the Royal Botanic Gardens Kew'
UNION ALL
SELECT 'j|31607235|49', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31607235 AND +m.name = 'University of Suffolk Dental Community Interest Company'
UNION ALL
SELECT 'k|31607235|49', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31607235 AND +m.name = 'University of Suffolk Dental Community Interest Company'
UNION ALL
SELECT 'j|31614234|50', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31614234 AND +m.name = 'FCC WASTE MANAGEMENT LIMITED'
UNION ALL
SELECT 'k|31614234|50', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31614234 AND +m.name = 'FCC WASTE MANAGEMENT LIMITED'
UNION ALL
SELECT 'j|31632003|51', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.lot_id, p.role FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31632003 AND +m.name = 'Thomas Bow Limited'
UNION ALL
SELECT 'k|31632003|51', m.notice_id, m.section_id, m.name, m.country, p.tender_id, p.seq, p.bid_id, p.role FROM organization_mentions m LEFT JOIN tender_version_bid_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = 31632003 AND +m.name = 'Thomas Bow Limited'
