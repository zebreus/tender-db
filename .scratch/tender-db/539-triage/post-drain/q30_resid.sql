SELECT n.id, n.publication_id, n.profile, n.published_at, n.published_offset, n.published_has_time,
       n.dispatched_at, n.dispatched_offset, n.ingested_at
  FROM notices n
 WHERE n.fetch_id = 431
   AND +n.profile = 'eforms:eforms-de-1.1'
   AND ((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400) > ((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)
 LIMIT 5
