SELECT profile, count(*) AS n, min(id) AS lo, max(id) AS hi FROM notices WHERE id BETWEEN 19394601 AND 19395768 GROUP BY +profile LIMIT 50
