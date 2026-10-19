SELECT substr(identifier,1,8) AS p, count(*) AS n FROM organizations WHERE identifier >= 'GBCOH' AND identifier < 'GBCOI' GROUP BY 1
