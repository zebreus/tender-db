SELECT c.op, count(*) AS n, count(DISTINCT c.entity_id) AS ids, min(c.entity_id) AS lo, max(c.entity_id) AS hi,
  min(c.changed_at) AS t0, max(c.changed_at) AS t1,
  (SELECT o.id FROM organizations o WHERE o.id >= 31974972 ORDER BY o.id DESC LIMIT 1) AS max_org_now
FROM changes c
WHERE c.entity_kind = 'organization' AND c.entity_id >= 31974972
GROUP BY c.op
