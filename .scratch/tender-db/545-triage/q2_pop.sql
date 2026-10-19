SELECT substr(identifier,1,8) AS p, count(*) AS n, min(id) AS min_id, max(id) AS max_id FROM organizations WHERE identifier >= 'GBPPON' AND identifier < 'GBPPOO' GROUP BY 1
