SELECT n.id, n.ingested_at, n.source, n.parse_state, n.projected, v.tender_id, v.seq FROM notices n LEFT JOIN tender_versions v ON v.caused_by_notice_id = n.id WHERE n.id >= 47420000 AND n.id < 47420005 ORDER BY n.id
-- plan: SEARCH n USING INTEGER PRIMARY KEY (rowid=?) | SEARCH v USING INDEX tender_versions_notice (caused_by_notice_id=?) LEFT-JOIN
