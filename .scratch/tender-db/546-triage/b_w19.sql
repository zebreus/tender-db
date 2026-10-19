SELECT profile, count(*) AS n, min(id) AS lo, max(id) AS hi FROM notices WHERE id BETWEEN 20297001 AND 20298464 GROUP BY +profile LIMIT 50
