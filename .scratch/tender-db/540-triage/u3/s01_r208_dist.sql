SELECT id / 250000 AS b, count(*) AS n, min(id) AS lo, max(id) AS hi FROM notices WHERE profile = 'ted-export-r208' AND id > 18579247 GROUP BY 1 LIMIT 100
