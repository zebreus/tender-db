SELECT profile, count(*) AS n, min(id) AS lo, max(id) AS hi FROM notices WHERE id BETWEEN 22470601 AND 22472192 GROUP BY +profile LIMIT 50
