SELECT profile, count(*) AS n, min(id) AS lo, max(id) AS hi FROM notices WHERE id BETWEEN 17300001 AND 17305000 GROUP BY +profile LIMIT 50
