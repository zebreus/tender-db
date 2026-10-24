SELECT o.id,
 (SELECT count(*) FROM organizations e WHERE e.name_norm = o.name_norm AND +e.id < 31974972) AS pre_same_name,
 (SELECT count(*) FROM organizations e WHERE e.name_norm = o.name_norm AND e.country = o.country AND +e.id < 31974972) AS pre_same_name_cc,
 (SELECT count(*) FROM organizations e WHERE e.name_norm = o.name_norm AND +e.id < 31974972 AND +e.country IS NULL) AS pre_same_name_nocc,
 (SELECT min(e.id) FROM organizations e WHERE e.name_norm = o.name_norm AND +e.id < 31974972) AS pre_min_id
FROM organizations o WHERE o.id BETWEEN 31974972 AND 31978722
