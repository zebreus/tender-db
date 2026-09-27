-- name: s0001
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_fetches'

-- name: s0002
SELECT * FROM sqlite_schema WHERE tbl_name = 'fetches' AND type != 'trigger' OR tbl_name = 'sqlite_sequence'

-- name: s0003
SELECT * FROM sqlite_schema WHERE name = 'fetches_period' AND type = 'index'

-- name: s0004
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_notices'

-- name: s0005
SELECT * FROM sqlite_schema WHERE tbl_name = 'notices' AND type != 'trigger'

-- name: s0006
SELECT * FROM sqlite_schema WHERE name = 'notices_profile' AND type = 'index'

-- name: s0007
SELECT * FROM sqlite_schema WHERE name = 'notices_parse_state' AND type = 'index'

-- name: s0008
SELECT * FROM sqlite_schema WHERE name = 'notices_fetch_id' AND type = 'index'

-- name: s0009
SELECT * FROM sqlite_schema WHERE name = 'notices_member_identity' AND type = 'index'

-- name: s0010
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_quarantine'

-- name: s0011
SELECT * FROM sqlite_schema WHERE tbl_name = 'quarantine' AND type != 'trigger'

-- name: s0012
SELECT * FROM sqlite_schema WHERE name = 'quarantine_reason' AND type = 'index'

-- name: s0013
SELECT * FROM sqlite_schema WHERE name = 'quarantine_notice_id' AND type = 'index'

-- name: s0014
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_sections' AND type != 'trigger'

-- name: s0015
SELECT * FROM sqlite_schema WHERE name = 'notice_sections_kind_notice' AND type = 'index'

-- name: s0016
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_texts' AND type != 'trigger'

-- name: s0017
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_codes' AND type != 'trigger'

-- name: s0018
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_classifications' AND type != 'trigger'

-- name: s0019
SELECT * FROM sqlite_schema WHERE name = 'notice_classifications_code' AND type = 'index'

-- name: s0020
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_amounts' AND type != 'trigger'

-- name: s0021
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_dates' AND type != 'trigger'

-- name: s0022
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_integers' AND type != 'trigger'

-- name: s0023
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_numbers' AND type != 'trigger'

-- name: s0024
SELECT * FROM sqlite_schema WHERE tbl_name = 'notice_ids' AND type != 'trigger'

-- name: s0025
SELECT * FROM sqlite_schema WHERE name = 'notice_ids_target' AND type = 'index'

-- name: s0026
SELECT * FROM sqlite_schema WHERE name = 'notice_withheld_fields'

-- name: s0027
SELECT * FROM sqlite_schema WHERE tbl_name = 'reports' AND type != 'trigger'

-- name: s0028
SELECT * FROM sqlite_schema WHERE tbl_name = 'report_history' AND type != 'trigger'

-- name: s0029
SELECT * FROM sqlite_schema WHERE tbl_name = 'layer_presence' AND type != 'trigger'

-- name: s0030
SELECT * FROM sqlite_schema WHERE tbl_name = 'currency_rates' AND type != 'trigger'

-- name: s0031
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_tenders'

-- name: s0032
SELECT * FROM sqlite_schema WHERE tbl_name = 'tenders' AND type != 'trigger'

-- name: s0033
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_versions' AND type != 'trigger'

-- name: s0034
SELECT * FROM sqlite_schema WHERE name = 'tender_versions_published' AND type = 'index'

-- name: s0035
SELECT * FROM sqlite_schema WHERE name = 'tender_versions_notice' AND type = 'index'

-- name: s0036
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_lots'

-- name: s0037
SELECT * FROM sqlite_schema WHERE tbl_name = 'lots' AND type != 'trigger'

-- name: s0038
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_lots' AND type != 'trigger'

-- name: s0039
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_lot_group_members' AND type != 'trigger'

-- name: s0040
SELECT * FROM sqlite_schema WHERE name = 'tender_version_lot_group_members_member' AND type = 'index'

-- name: s0041
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_texts' AND type != 'trigger'

-- name: s0042
SELECT * FROM sqlite_schema WHERE name = 'tender_version_texts_version' AND type = 'index'

-- name: s0043
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_amounts' AND type != 'trigger'

-- name: s0044
SELECT * FROM sqlite_schema WHERE name = 'tender_version_amounts_version' AND type = 'index'

-- name: s0045
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_currency_presence' AND type != 'trigger'

-- name: s0046
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_dates' AND type != 'trigger'

-- name: s0047
SELECT * FROM sqlite_schema WHERE name = 'tender_version_dates_version' AND type = 'index'

-- name: s0048
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_classifications' AND type != 'trigger'

-- name: s0049
SELECT * FROM sqlite_schema WHERE name = 'tender_version_classifications_code' AND type = 'index'

-- name: s0050
SELECT * FROM sqlite_schema WHERE name = 'tender_version_classifications_version' AND type = 'index'

-- name: s0051
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_parties' AND type != 'trigger'

-- name: s0052
SELECT * FROM sqlite_schema WHERE name = 'tender_version_parties_org' AND type = 'index'

-- name: s0053
SELECT * FROM sqlite_schema WHERE name = 'tender_version_parties_version' AND type = 'index'

-- name: s0054
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_organizations'

-- name: s0055
SELECT * FROM sqlite_schema WHERE tbl_name = 'organizations' AND type != 'trigger'

-- name: s0056
SELECT * FROM sqlite_schema WHERE tbl_name = 'organization_mentions' AND type != 'trigger'

-- name: s0057
SELECT * FROM sqlite_schema WHERE name = 'organization_mentions_org' AND type = 'index'

-- name: s0058
SELECT * FROM sqlite_schema WHERE tbl_name = 'organization_names' AND type != 'trigger'

-- name: s0059
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_merge_log' AND type != 'trigger'

-- name: s0060
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_name_verdicts' AND type != 'trigger'

-- name: s0061
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_case_reviews' AND type != 'trigger'

-- name: s0062
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_mention_rehoming' AND type != 'trigger'

-- name: s0063
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_country_verdicts' AND type != 'trigger'

-- name: s0064
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_merge_verdicts' AND type != 'trigger'

-- name: s0065
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_name_drops' AND type != 'trigger'

-- name: s0066
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_match_keys' AND type != 'trigger'

-- name: s0067
SELECT * FROM sqlite_schema WHERE tbl_name = 'org_candidate_edges' AND type != 'trigger'

-- name: s0068
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_lot_results'

-- name: s0069
SELECT * FROM sqlite_schema WHERE tbl_name = 'lot_results' AND type != 'trigger'

-- name: s0070
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_bids'

-- name: s0071
SELECT * FROM sqlite_schema WHERE tbl_name = 'bids' AND type != 'trigger'

-- name: s0072
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_contracts'

-- name: s0073
SELECT * FROM sqlite_schema WHERE tbl_name = 'contracts' AND type != 'trigger'

-- name: s0074
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_lot_results' AND type != 'trigger'

-- name: s0075
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_result_winners' AND type != 'trigger'

-- name: s0076
SELECT * FROM sqlite_schema WHERE name = 'tender_version_result_winners_org' AND type = 'index'

-- name: s0077
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_result_stats' AND type != 'trigger'

-- name: s0078
SELECT * FROM sqlite_schema WHERE name = 'tender_version_result_stats_version' AND type = 'index'

-- name: s0079
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_bids' AND type != 'trigger'

-- name: s0080
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_bid_parties' AND type != 'trigger'

-- name: s0081
SELECT * FROM sqlite_schema WHERE name = 'tender_version_bid_parties_org' AND type = 'index'

-- name: s0082
SELECT * FROM sqlite_schema WHERE tbl_name = 'tender_version_contracts' AND type != 'trigger'

-- name: s0083
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_changes'

-- name: s0084
SELECT * FROM sqlite_schema WHERE tbl_name = 'changes' AND type != 'trigger'

-- name: s0085
SELECT * FROM sqlite_schema WHERE name = 'changes_entity' AND type = 'index'

-- name: s0086
SELECT * FROM sqlite_schema WHERE tbl_name = 'projection_state' AND type != 'trigger'

-- name: s0087
SELECT * FROM sqlite_schema WHERE tbl_name = 'feed_generation' AND type != 'trigger'

-- name: s0088
SELECT * FROM sqlite_schema WHERE tbl_name = 'legacy_ojs_keys' AND type != 'trigger'

-- name: s0089
SELECT * FROM sqlite_schema WHERE name = 'legacy_ojs_keys_notice' AND type = 'index'

-- name: s0090
SELECT * FROM sqlite_schema WHERE tbl_name = 'legacy_adjacency' AND type != 'trigger'

-- name: s0091
SELECT * FROM sqlite_schema WHERE name = 'v_tender_current'

-- name: s0092
SELECT * FROM sqlite_schema WHERE name = 'v_tenders'

-- name: s0093
SELECT * FROM sqlite_schema WHERE name = 'v_lots'

-- name: s0094
SELECT * FROM sqlite_schema WHERE name = 'v_organizations'

-- name: s0095
SELECT * FROM sqlite_schema WHERE name = 'v_lot_results'

-- name: s0096
SELECT * FROM sqlite_schema WHERE name = 'v_tender_buyers'

-- name: s0097
SELECT * FROM sqlite_schema WHERE name = 'v_awards'

-- name: s0098
SELECT * FROM sqlite_schema WHERE name = 'v_tender_classifications'

-- name: s0099
SELECT * FROM sqlite_schema WHERE name = 'v_tender_amounts'

-- name: s0100
SELECT * FROM sqlite_schema WHERE name = 'v_tender_dates'

-- name: s0101
SELECT * FROM sqlite_schema WHERE name = 'v_tender_notices'

-- name: s0102
SELECT * FROM sqlite_schema WHERE name = 'v_fetches'

-- name: s0103
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_users'

-- name: s0104
SELECT * FROM sqlite_schema WHERE tbl_name = 'users' AND type != 'trigger'

-- name: s0105
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_api_tokens'

-- name: s0106
SELECT * FROM sqlite_schema WHERE tbl_name = 'api_tokens' AND type != 'trigger'

-- name: s0107
SELECT * FROM sqlite_schema WHERE name = 'api_tokens_user' AND type = 'index'

-- name: s0108
SELECT * FROM sqlite_schema WHERE tbl_name = 'sessions' AND type != 'trigger'

-- name: s0109
SELECT * FROM sqlite_schema WHERE name = 'sessions_user' AND type = 'index'

-- name: s0110
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_job_log'

-- name: s0111
SELECT * FROM sqlite_schema WHERE tbl_name = 'job_log' AND type != 'trigger'

-- name: s0112
SELECT * FROM sqlite_schema WHERE tbl_name = 'job_queue' AND type != 'trigger'

-- name: s0113
SELECT * FROM sqlite_schema WHERE tbl_name = 'package_rates' AND type != 'trigger'

-- name: s0114
SELECT * FROM sqlite_schema WHERE name = 'package_rates_walks' AND type = 'index'

-- name: s0115
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_webhook_endpoints'

-- name: s0116
SELECT * FROM sqlite_schema WHERE tbl_name = 'webhook_endpoints' AND type != 'trigger'

-- name: s0117
SELECT * FROM sqlite_schema WHERE name = 'webhook_endpoints_user' AND type = 'index'

-- name: s0118
SELECT * FROM sqlite_schema WHERE name = 'webhook_endpoints_due' AND type = 'index'

-- name: s0119
SELECT * FROM sqlite_schema WHERE name = '__turso_internal_seq___turso_internal_autoincrement_webhook_delivery_log'

-- name: s0120
SELECT * FROM sqlite_schema WHERE tbl_name = 'webhook_delivery_log' AND type != 'trigger'

-- name: s0121
SELECT * FROM sqlite_schema WHERE name = 'webhook_delivery_log_endpoint' AND type = 'index'

-- name: s0122
SELECT 1 FROM tender_version_amounts LIMIT 1

-- name: s0123
UPDATE projection_state SET currency_presence_complete = 1 WHERE id = 0

-- name: s0124
SELECT * FROM sqlite_schema WHERE name = 'tenders_current_published' AND type = 'index'

-- name: s0125
SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'changes'), 0)

-- name: s0126
INSERT INTO fetches(source, kind, period, url, sha256, bytes, fetched_at, path)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)

-- name: s0127
SELECT id, job_id, kind, params, started_at, finished_at, outcome, counts_json
                 FROM job_log ORDER BY id DESC LIMIT ?

-- name: s0128
SELECT body, computed_at FROM reports WHERE kind = ?

-- name: s0129
INSERT INTO reports(kind, computed_at, body) VALUES(?, ?, ?)
               ON CONFLICT(kind) DO UPDATE SET computed_at = excluded.computed_at,
                                               body = excluded.body

-- name: s0130
INSERT OR REPLACE INTO report_history(kind, computed_at, body) VALUES(?, ?, ?)

-- name: s0131
DELETE FROM report_history WHERE kind = ? AND computed_at NOT IN (SELECT computed_at FROM report_history WHERE kind = ? ORDER BY computed_at DESC LIMIT ?)

-- name: s0132
INSERT INTO job_queue(id, kind, params, spec) VALUES(?, ?, ?, ?)

-- name: s0133
SELECT id, period, path FROM fetches
                 WHERE id IN (SELECT MAX(id) FROM fetches
                              WHERE source = ? AND kind = ? AND (? IS NULL OR period = ?)
                              GROUP BY period)
                 ORDER BY period

-- name: s0134
SELECT period, fetch_id, skipped, quarantined FROM package_rates WHERE source = ? AND kind = ? ORDER BY walked_at

-- name: s0135
SELECT member_path FROM quarantine
                  WHERE fetch_id = ?1 AND reason LIKE 'unreadable %'
                    AND reprocessed_at IS NULL AND skipped_at IS NULL
                    AND member_path NOT LIKE '%!%' AND member_path NOT LIKE '%#%'

-- name: s0136
INSERT OR IGNORE INTO notices(source, publication_id, content_hash, profile,
                     declared_version, fetch_id, member_path, ingested_at, published_at, dispatched_at,
                     published_offset, published_has_time, dispatched_offset, dispatched_has_time)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)

-- name: s0137
SELECT id FROM notices
                  WHERE source = ? AND member_path = ? AND content_hash = ?
                    AND publication_id <> ? LIMIT 2

-- name: s0138
SELECT id FROM notices WHERE source = ? AND publication_id = ? AND content_hash = ?

-- name: s0139
INSERT INTO notice_sections(notice_id, section_id, kind, parent_section_id)
                     VALUES(?, ?, ?, ?)
                     ON CONFLICT(notice_id, section_id) DO UPDATE SET
                         kind = excluded.kind, parent_section_id = excluded.parent_section_id

-- name: s0140
INSERT INTO notice_codes(notice_id, section_id, field_id, ordinal, list_name, code)
                         VALUES(?, ?, ?, ?, ?, ?)

-- name: s0141
INSERT INTO notice_texts(notice_id, section_id, field_id, ordinal, lang, value)
                         VALUES(?, ?, ?, ?, ?, ?)

-- name: s0142
INSERT INTO notice_ids(notice_id, section_id, field_id, ordinal, scheme, value, is_ref)
                         VALUES(?, ?, ?, ?, ?, ?, ?)

-- name: s0143
INSERT INTO notice_classifications(notice_id, section_id, field_id, ordinal, scheme, code)
                         VALUES(?, ?, ?, ?, ?, ?)

-- name: s0144
INSERT INTO notice_dates(notice_id, section_id, field_id, ordinal,
                             utc_seconds, offset_minutes, has_time)
                         VALUES(?, ?, ?, ?, ?, ?, ?)

-- name: s0145
INSERT INTO notice_numbers(notice_id, section_id, field_id, ordinal, value, unit)
                         VALUES(?, ?, ?, ?, ?, ?)

-- name: s0146
INSERT INTO notice_integers(notice_id, section_id, field_id, ordinal, value)
                         VALUES(?, ?, ?, ?, ?)

-- name: s0147
UPDATE notices SET parse_state = ?1, projected = projected AND (?1 <> 'parsed')
              WHERE id = ?2

-- name: s0148
UPDATE job_queue SET progress = ? WHERE id = ?

-- name: s0149
SELECT members, seconds FROM package_rates WHERE source = ? AND kind = ? AND notices > 0 AND members >= ? AND seconds > 0 ORDER BY walked_at DESC LIMIT ?

-- name: s0150
INSERT INTO package_rates(source, kind, period, members, notices, duplicates, seconds, walked_at, fetch_id, skipped, quarantined) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)

-- name: s0151
INSERT INTO job_log(job_id, kind, params, started_at, finished_at, outcome, counts_json)
             VALUES(?, ?, ?, ?, ?, ?, ?)

-- name: s0152
DELETE FROM job_queue WHERE id = ?

-- name: s0153
SELECT profile, COUNT(*) FROM notices GROUP BY profile ORDER BY profile

-- name: s0154
SELECT rebuild_in_progress FROM projection_state WHERE id = 0

-- name: s0155
SELECT currency, rate_date, rate_to_eur, source FROM currency_rates
                  ORDER BY currency, rate_date

-- name: s0156
SELECT EXISTS(SELECT 1 FROM tenders)

-- name: s0157
SELECT ever_populated FROM layer_presence WHERE name = 'tenders'

-- name: s0158
SELECT id FROM notices WHERE parse_state = 'parsed' AND projected = 0 ORDER BY id

-- name: s0159
SELECT COUNT(*) FROM notices
                  WHERE parse_state = 'parsed' AND projected = 0
                    AND (profile IN ('text', 'internal-ojs') OR profile LIKE 'ted-export%')

-- name: s0160
SELECT id, source, publication_id, profile FROM notices WHERE id IN (?)

-- name: s0161
SELECT notice_id, section_id, kind, parent_section_id FROM notice_sections
                 WHERE notice_id IN (?)

-- name: s0162
SELECT notice_id, section_id, field_id, ordinal, lang, value FROM notice_texts WHERE notice_id IN (?)

-- name: s0163
SELECT notice_id, section_id, field_id, ordinal, list_name, code FROM notice_codes WHERE notice_id IN (?)

-- name: s0164
SELECT notice_id, section_id, field_id, ordinal, scheme, code FROM notice_classifications WHERE notice_id IN (?)

-- name: s0165
SELECT notice_id, section_id, field_id, ordinal, cents, currency FROM notice_amounts WHERE notice_id IN (?)

-- name: s0166
SELECT notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time FROM notice_dates WHERE notice_id IN (?)

-- name: s0167
SELECT notice_id, section_id, field_id, ordinal, value FROM notice_integers WHERE notice_id IN (?)

-- name: s0168
SELECT notice_id, section_id, field_id, ordinal, value, unit FROM notice_numbers WHERE notice_id IN (?)

-- name: s0169
SELECT notice_id, section_id, field_id, ordinal, scheme, value, is_ref FROM notice_ids WHERE notice_id IN (?)

-- name: s0170
SELECT DISTINCT tender_id FROM tender_versions WHERE caused_by_notice_id IN (?)

-- name: s0171
SELECT id FROM tenders WHERE procedure_key IN (?)

-- name: s0172
SELECT * FROM sqlite_schema WHERE tbl_name = 'plan_notice' AND type != 'trigger'

-- name: s0173
SELECT * FROM sqlite_schema WHERE tbl_name = 'plan_refused_key' AND type != 'trigger'

-- name: s0174
SELECT * FROM sqlite_schema WHERE name = 'plan_notice_fts_key' AND type = 'index'

-- name: s0175
SELECT * FROM sqlite_schema WHERE tbl_name = 'plan_ojs_node' AND type != 'trigger'

-- name: s0176
SELECT * FROM sqlite_schema WHERE tbl_name = 'plan_ojs_edge' AND type != 'trigger'

-- name: s0177
SELECT * FROM sqlite_schema WHERE tbl_name = 'plan_prev_edge' AND type != 'trigger'

-- name: s0178
SELECT * FROM sqlite_schema WHERE tbl_name = 'plan_group_merge' AND type != 'trigger'

-- name: s0179
SELECT id, country, identifier_kind, identifier FROM organizations
                  WHERE identifier IS NOT NULL

-- name: s0180
SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?

-- name: s0181
SELECT org_match_keys_watermark, org_match_keys_epoch FROM projection_state WHERE id = 0

-- name: s0182
INSERT INTO plan_notice(notice_id, procedure_key, legacy, ojs_self, source,
                     source_rank, publication_id, published_at, subtype, group_key, key_shaped,
                     buyer_key, shared_kind)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?)

-- name: s0183
SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN (?)

-- name: s0184
INSERT INTO organizations(country, identifier_kind, identifier, name,
                                 name_norm, provisional, created_at)
                             VALUES(?, ?, ?, ?, ?, 0, ?)

-- name: s0185
INSERT INTO organization_mentions(notice_id, section_id, organization_id, name,
                 country, raw_identifier, scheme)
             VALUES(?, ?, ?, ?, ?, ?, ?)

-- name: s0186
INSERT OR REPLACE INTO organization_names(org_id, lang, name, name_norm)
                 VALUES(?, ?, ?, ?)

-- name: s0187
INSERT INTO changes(entity_kind, entity_id, version_seq, op, changed_at)
         VALUES(?, ?, ?, ?, ?)

-- name: s0188
UPDATE legacy_adjacency SET watermark = MAX(watermark, ?)
              WHERE id = 0 AND watermark > 0

-- name: s0189
SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'plan_notice_fold'

-- name: s0190
INSERT OR IGNORE INTO plan_refused_key(procedure_key)                  SELECT procedure_key FROM plan_notice                   WHERE key_shaped = 1 AND procedure_key IS NOT NULL                   GROUP BY procedure_key                  HAVING COUNT(DISTINCT buyer_key) >= 3

-- name: s0191
SELECT COUNT(*) FROM plan_refused_key

-- name: s0192
INSERT OR IGNORE INTO plan_refused_key(procedure_key)                  SELECT procedure_key FROM plan_notice                   WHERE source = 'fts' AND procedure_key IS NOT NULL AND buyer_key IS NOT NULL                   GROUP BY procedure_key                  HAVING COUNT(DISTINCT buyer_key) >= 2

-- name: s0193
SELECT COUNT(*) FROM (SELECT procedure_key FROM plan_notice                       WHERE source = 'fts' AND procedure_key IS NOT NULL AND buyer_key IS NOT NULL                       GROUP BY procedure_key HAVING COUNT(DISTINCT buyer_key) >= 2)

-- name: s0194
SELECT MIN(notice_id), MAX(notice_id) FROM plan_notice

-- name: s0195
UPDATE plan_notice SET group_key = CASE
                         WHEN procedure_key IS NOT NULL
                              AND procedure_key NOT IN (SELECT procedure_key FROM plan_refused_key)
                              THEN procedure_key
                         -- issue 369 unit 5: a REFUSED key splits per BUYER, not per notice.
                         -- The island fallback was correct but over-split: it turned 82802's
                         -- ~7 real procurements into 42 Tenders and lost the cross-source
                         -- doe/ted merges inside them. Grouping the refused notices by the
                         -- buyer they name recovers that structure while still separating the
                         -- procurements the weld had fused.
                         --
                         -- `procedure_key IS NOT NULL` is load-bearing, not redundant with the
                         -- arm above: without it a notice with NO key and any buyer would group
                         -- by buyer alone, welding every one of that buyer's procurements into a
                         -- single Tender — a far larger weld than the one this issue fixes. The
                         -- explicit `IN` keeps that true even if the arms are ever reordered.
                         --
                         -- Legacy closure still wins: same `NOT (legacy = 1 AND ojs_self ...)`
                         -- guard as the island arm, so an OJS chain remains the stronger identity
                         -- signal and a refused legacy notice falls through to it as designed.
                         WHEN procedure_key IS NOT NULL
                              AND procedure_key IN (SELECT procedure_key FROM plan_refused_key)
                              AND buyer_key IS NOT NULL
                              AND NOT (legacy = 1 AND ojs_self IS NOT NULL)
                              THEN 'refused:' || procedure_key || ':' || buyer_key
                         -- A refused notice naming NO buyer stays an island: grouping every
                         -- buyer-less notice of a key together would be a new weld, smaller but
                         -- the same kind.
                         WHEN NOT (legacy = 1 AND ojs_self IS NOT NULL) THEN 'island:' || notice_id
                         ELSE NULL END
                     WHERE notice_id BETWEEN ? AND ?

-- name: s0196
SELECT ojs_self, shared_kind FROM plan_notice
                      WHERE legacy = 1 AND ojs_self IS NOT NULL AND shared_kind IS NOT NULL

-- name: s0197
SELECT a, b FROM plan_ojs_edge

-- name: s0198
SELECT ojs_self FROM plan_notice WHERE legacy = 1 AND ojs_self IS NOT NULL

-- name: s0199
SELECT notice_id, ojs_self FROM plan_notice
                      WHERE legacy = 1 AND ojs_self IS NOT NULL ORDER BY notice_id

-- name: s0200
SELECT * FROM sqlite_schema WHERE tbl_name = 'sqlite_stat1' AND type != 'trigger'

-- name: s0201
SELECT tbl, idx, stat FROM sqlite_stat1

-- name: s0202
SELECT COUNT(*) FROM plan_prev_edge

-- name: s0203
SELECT a.group_key, a.published_at, a.publication_id, b.group_key, b.published_at, b.publication_id FROM plan_prev_edge e JOIN notices n ON n.source = e.a_source AND n.publication_id = e.b_publication_id JOIN plan_notice a ON a.notice_id = e.a_notice_id JOIN plan_notice b ON b.notice_id = n.id WHERE a.group_key IS NOT NULL AND b.group_key IS NOT NULL AND b.published_at < a.published_at AND a.group_key <> b.group_key

-- name: s0204
SELECT * FROM sqlite_schema WHERE name = 'plan_notice_fold' AND type = 'index'

-- name: s0205
SELECT COUNT(DISTINCT group_key),
                        COUNT(DISTINCT CASE WHEN group_key LIKE 'island:%' THEN group_key END)
                   FROM plan_notice

-- name: s0206
SELECT notice_id, group_key, source, subtype FROM plan_notice
                  WHERE group_key > ?
                  ORDER BY group_key, published_at, source_rank, publication_id, notice_id

-- name: s0207
INSERT INTO tenders(source, procedure_key, island_notice_id, kind, created_at)
                     VALUES(?, ?, ?, ?, ?)

-- name: s0208
INSERT INTO lots(tender_id, lot_key) VALUES(?, ?)

-- name: s0209
UPDATE tenders SET current_seq = ?, current_published_at = ?, projection_epoch = ?, current_deadline = ?, current_title = ?, current_value_eur_cents = ? WHERE id = ?

-- name: s0210
SELECT id FROM lots WHERE tender_id = ? AND lot_key = ?

-- name: s0211
SELECT id FROM lot_results WHERE tender_id = ? AND notice_id = ? AND result_key = ?

-- name: s0212
SELECT id FROM bids WHERE tender_id = ? AND notice_id = ? AND bid_key = ?

-- name: s0213
SELECT id FROM contracts WHERE tender_id = ? AND notice_id = ? AND contract_key = ?

-- name: s0214
SELECT id, source, projection_epoch FROM tenders WHERE procedure_key = ?

-- name: s0215
SELECT caused_by_notice_id FROM tender_versions WHERE tender_id = ? ORDER BY seq

-- name: s0216
INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, dispatched_at, notice_subtype, original_lang, publication_id) VALUES (?,?,?,?,?,?,?,?)

-- name: s0217
INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind) VALUES (?,?,?,?)

-- name: s0218
INSERT INTO tender_version_texts(tender_id, seq, lot_id, field, lang, value) VALUES (?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?)

-- name: s0219
INSERT INTO tender_version_classifications(tender_id, seq, lot_id, field, scheme, code) VALUES (?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?)

-- name: s0220
INSERT INTO tender_version_dates(tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0221
INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0222
SELECT t.id, t.current_seq,
                    (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)
               FROM tenders t
              WHERE t.id IN (?)
                AND t.current_seq IS NOT
                    (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)
              LIMIT 5

-- name: s0223
UPDATE notices SET projected = 1 WHERE id IN (?)

-- name: s0224
UPDATE projection_state SET rebuild_in_progress = 0 WHERE id = 0

-- name: s0225
SELECT * FROM sqlite_schema WHERE name = 'notices_unprojected' AND type = 'index'

-- name: s0226
SELECT * FROM sqlite_schema WHERE name = 'changes_entity_cursor' AND type = 'index'

-- name: s0227
SELECT t.id,
                        COALESCE((SELECT x.value FROM tender_version_texts x
                                   WHERE x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title'
                                   ORDER BY (x.lot_id IS NULL) DESC, (x.lang = 'ENG') DESC
                                   LIMIT 1), '(untitled)')
                   FROM tenders t
                  WHERE t.current_published_at IS NOT NULL
                  ORDER BY t.current_published_at DESC, t.id DESC
                  LIMIT ?

-- name: s0228
SELECT MAX(id) FROM notices WHERE profile = ?

-- name: s0229
SELECT x.field_id, COUNT(*) FROM notice_texts x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0230
SELECT x.field_id, COUNT(*) FROM notice_codes x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0231
SELECT x.field_id, COUNT(*) FROM notice_classifications x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0232
SELECT x.field_id, COUNT(*) FROM notice_amounts x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0233
SELECT x.field_id, COUNT(*) FROM notice_dates x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0234
SELECT x.field_id, COUNT(*) FROM notice_integers x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0235
SELECT x.field_id, COUNT(*) FROM notice_numbers x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0236
SELECT x.field_id, COUNT(*) FROM notice_ids x CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = ? WHERE x.notice_id > ? AND x.notice_id <= ? GROUP BY x.field_id

-- name: s0237
INSERT INTO notice_amounts(notice_id, section_id, field_id, ordinal, cents, currency)
                         VALUES(?, ?, ?, ?, ?, ?)

-- name: s0238
SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed'

-- name: s0239
SELECT id, source, publication_id, profile FROM notices
             WHERE parse_state = 'parsed' AND id > ? AND id <= ? ORDER BY id LIMIT ?

-- name: s0240
SELECT notice_id, section_id, kind, parent_section_id FROM notice_sections
                 WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0241
SELECT notice_id, section_id, field_id, ordinal, lang, value FROM notice_texts WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0242
SELECT notice_id, section_id, field_id, ordinal, list_name, code FROM notice_codes WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0243
SELECT notice_id, section_id, field_id, ordinal, scheme, code FROM notice_classifications WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0244
SELECT notice_id, section_id, field_id, ordinal, cents, currency FROM notice_amounts WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0245
SELECT notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time FROM notice_dates WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0246
SELECT notice_id, section_id, field_id, ordinal, value FROM notice_integers WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0247
SELECT notice_id, section_id, field_id, ordinal, value, unit FROM notice_numbers WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0248
SELECT notice_id, section_id, field_id, ordinal, scheme, value, is_ref FROM notice_ids WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id

-- name: s0249
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1

-- name: s0250
INSERT INTO organizations(country, identifier_kind, identifier, name,
                                         name_norm, provisional, created_at)
                                     VALUES(?, NULL, NULL, ?, ?, 1, ?)

-- name: s0251
UPDATE legacy_adjacency SET watermark = MAX(watermark, ?) WHERE id = 0

-- name: s0252
SELECT group_key FROM plan_notice ORDER BY group_key

-- name: s0253
SELECT COALESCE(MAX(id), 0) FROM notices WHERE parse_state = 'parsed'

-- name: s0254
SELECT COUNT(*) FROM notices
      WHERE +parse_state = 'parsed' AND id > ? AND id <= ?

-- name: s0255
SELECT notice_id, group_key FROM plan_notice
                  WHERE notice_id >= ? AND notice_id <= ? AND group_key IS NOT NULL

-- name: s0256
INSERT INTO tender_version_amounts(tender_id, seq, lot_id, field, cents, currency, tax_basis, eur_cents, quality) VALUES (?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?)

-- name: s0257
INSERT OR IGNORE INTO tender_currency_presence(currency) VALUES(?)

-- name: s0258
INSERT INTO tender_version_classifications(tender_id, seq, lot_id, field, scheme, code) VALUES (?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?)

-- name: s0259
INSERT INTO tender_version_dates(tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0260
INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0261
SELECT id, procedure_key FROM tenders WHERE procedure_key LIKE 'ojs:%'

-- name: s0262
SELECT t.id FROM tenders t
                  WHERE t.procedure_key IS NOT NULL
                    AND t.procedure_key NOT LIKE 'ojs:%'
                    AND NOT EXISTS (SELECT 1 FROM plan_notice p WHERE p.group_key = t.procedure_key)

-- name: s0263
SELECT t.id FROM tenders t
                  WHERE t.procedure_key IS NULL
                    AND NOT EXISTS (
                      SELECT 1 FROM plan_notice p
                       WHERE p.group_key = 'island:' || t.island_notice_id)

-- name: s0264
SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN (?,?)

-- name: s0265
UPDATE notices SET projected = 1 WHERE id IN (?,?)

-- name: s0266
SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN (?,?,?)

-- name: s0267
UPDATE notices SET projected = 1 WHERE id IN (?,?,?)

-- name: s0268
SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN (?,?,?,?)

-- name: s0269
INSERT INTO lot_results(tender_id, notice_id, result_key) VALUES(?, ?, ?)

-- name: s0270
INSERT INTO bids(tender_id, notice_id, bid_key) VALUES(?, ?, ?)

-- name: s0271
INSERT INTO contracts(tender_id, notice_id, contract_key) VALUES(?, ?, ?)

-- name: s0272
INSERT INTO tender_version_amounts(tender_id, seq, lot_id, field, cents, currency, tax_basis, eur_cents, quality) VALUES (?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?)

-- name: s0273
INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0274
INSERT INTO tender_version_lot_results(tender_id, seq, lot_result_id, lot_id, decision, reason, awarded_cents, awarded_currency, decided_utc, decided_offset, decided_has_time, awarded_eur_cents) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)

-- name: s0275
INSERT INTO tender_version_result_winners(tender_id, seq, lot_result_id, organization_id) VALUES (?,?,?,?)

-- name: s0276
INSERT INTO tender_version_result_stats(tender_id, seq, lot_result_id, kind, count, quality) VALUES (?,?,?,?,?,?)

-- name: s0277
INSERT INTO tender_version_bids(tender_id, seq, bid_id, lot_id, cents, currency, eur_cents, quality) VALUES (?,?,?,?,?,?,?,?)

-- name: s0278
INSERT INTO tender_version_bid_parties(tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id) VALUES (?,?,?,?,?,?,?)

-- name: s0279
INSERT INTO tender_version_contracts(tender_id, seq, contract_id, buyer_contract_id, concluded_utc, concluded_offset, concluded_has_time, decided_utc, decided_offset, decided_has_time, cents, currency, eur_cents) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)

-- name: s0280
UPDATE notices SET projected = 1 WHERE id IN (?,?,?,?)

-- name: s0281
SELECT generation FROM feed_generation WHERE id = 0

-- name: s0282
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0283
SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes
                  WHERE entity_kind = ? AND cursor > ? ORDER BY cursor LIMIT ?

-- name: s0284
SELECT 1 FROM tender_version_classifications
              WHERE scheme = ? AND code >= ? AND code < ? LIMIT 1

-- name: s0285
SELECT l.id, l.tender_id, l.lot_key,
                    (SELECT vl.kind FROM tender_version_lots vl
                      WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                        AND vl.lot_id = l.id),
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
               FROM lots l
              WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                             WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                               AND vl.lot_id = l.id) AND l.id > ? ORDER BY l.id LIMIT ?

-- name: s0286
SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE 1 = 1 AND o.id > ? ORDER BY o.id LIMIT ?

-- name: s0287
SELECT id, source, publication_id, content_hash, profile, declared_version,
                member_path, ingested_at, parse_state, published_at, dispatched_at,
                published_offset, published_has_time, dispatched_offset, dispatched_has_time
           FROM notices WHERE 1 = 1 AND id > ? ORDER BY id LIMIT ?

-- name: s0288
INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0289
SELECT (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id), s.field, s.lang, s.value FROM tender_version_texts s
              WHERE s.tender_id = ? AND s.seq = ?

-- name: s0290
SELECT (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id), s.field, s.cents, s.currency, s.quality FROM tender_version_amounts s
              WHERE s.tender_id = ? AND s.seq = ?

-- name: s0291
SELECT (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id), s.field, s.utc_seconds, s.offset_minutes, s.has_time FROM tender_version_dates s
              WHERE s.tender_id = ? AND s.seq = ?

-- name: s0292
SELECT (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id), s.field, s.scheme, s.code FROM tender_version_classifications s
              WHERE s.tender_id = ? AND s.seq = ?

-- name: s0293
SELECT (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id), s.role, s.organization_id, o.name
                   FROM tender_version_parties s
                   JOIN organizations o ON o.id = s.organization_id
                  WHERE s.tender_id = ? AND s.seq = ?

-- name: s0294
SELECT v.seq, v.published_at, v.dispatched_at, v.publication_id, v.notice_subtype,
                    v.caused_by_notice_id, v.original_lang,
                    n.published_offset, n.published_has_time, n.dispatched_offset, n.dispatched_has_time
               FROM tender_versions v LEFT JOIN notices n ON n.id = v.caused_by_notice_id
              WHERE v.tender_id = ? ORDER BY v.seq

-- name: s0295
SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
                   FROM tender_version_lots vl
                   JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id
                   JOIN tenders t ON t.id = vl.tender_id
                   JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq
                  WHERE vl.tender_id = ?
                    AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                   WHERE x.tender_id = ?) AND l.id > ? ORDER BY l.id LIMIT ?

-- name: s0296
SELECT original_lang, published_at FROM tender_versions WHERE tender_id = ? AND seq = ?

-- name: s0297
SELECT s.lot_id, s.lang, s.value FROM tender_version_texts s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.field = 'title'
                    AND s.lot_id IS NOT NULL

-- name: s0298
SELECT s.lot_id, s.cents, s.currency, s.eur_cents FROM tender_version_amounts s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.lot_id IS NOT NULL
                    AND s.quality IS NULL

-- name: s0299
SELECT s.lot_id, s.utc_seconds, s.offset_minutes, s.has_time
                   FROM tender_version_dates s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.field = 'submission_deadline'

-- name: s0300
SELECT s.lot_result_id, r.notice_id, r.result_key, (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id),
                        s.decision, s.reason, s.awarded_cents, s.awarded_currency,
                        s.decided_utc, s.decided_offset, s.decided_has_time
                   FROM tender_version_lot_results s
                   JOIN lot_results r ON r.id = s.lot_result_id
                  WHERE s.tender_id = ? AND s.seq = ? ORDER BY s.lot_result_id

-- name: s0301
SELECT w.lot_result_id, w.organization_id, o.name
               FROM tender_version_result_winners w
               JOIN organizations o ON o.id = w.organization_id
              WHERE w.tender_id = ? AND w.seq = ?

-- name: s0302
SELECT lot_result_id, kind, count, quality FROM tender_version_result_stats
              WHERE tender_id = ? AND seq = ?

-- name: s0303
SELECT s.bid_id, b.notice_id, b.bid_key, (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id), s.cents, s.currency, s.quality
                   FROM tender_version_bids s
                   JOIN bids b ON b.id = s.bid_id
                  WHERE s.tender_id = ? AND s.seq = ? ORDER BY s.bid_id

-- name: s0304
SELECT p.bid_id, p.role, p.organization_id, o.name
               FROM tender_version_bid_parties p
               JOIN organizations o ON o.id = p.organization_id
              WHERE p.tender_id = ? AND p.seq = ?

-- name: s0305
SELECT c.notice_id, c.contract_key, s.buyer_contract_id,
                    s.concluded_utc, s.concluded_offset, s.concluded_has_time,
                    s.decided_utc, s.decided_offset, s.decided_has_time,
                    s.cents, s.currency
               FROM tender_version_contracts s
               JOIN contracts c ON c.id = s.contract_id
              WHERE s.tender_id = ? AND s.seq = ? ORDER BY s.contract_id

-- name: s0306
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_published_at AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_published_at IS NOT NULL AND t.current_published_at >= ? AND t.current_published_at < ? ORDER BY t.current_published_at DESC, t.id DESC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey DESC, t.id DESC

-- name: s0307
UPDATE notices SET published_offset = NULL, published_has_time = NULL,
                                dispatched_offset = NULL, dispatched_has_time = NULL

-- name: s0308
SELECT 1 FROM tenders WHERE kind = ? LIMIT 1

-- name: s0309
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.kind = ? AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0310
SELECT id FROM tenders ORDER BY id DESC LIMIT 1

-- name: s0311
SELECT 1 FROM tenders WHERE source = ? LIMIT 1

-- name: s0312
SELECT l.id, l.tender_id, l.lot_key,
                    (SELECT vl.kind FROM tender_version_lots vl
                      WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                        AND vl.lot_id = l.id),
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
               FROM lots l
              WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                             WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                               AND vl.lot_id = l.id) AND (SELECT tt.source FROM tenders tt WHERE tt.id = l.tender_id) = ? AND l.id > ? AND l.id <= ? ORDER BY l.id LIMIT ?

-- name: s0313
SELECT id FROM lots ORDER BY id DESC LIMIT 1

-- name: s0314
SELECT l.id, l.tender_id, l.lot_key,
                    (SELECT vl.kind FROM tender_version_lots vl
                      WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                        AND vl.lot_id = l.id),
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
               FROM lots l
              WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                             WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                               AND vl.lot_id = l.id AND vl.kind = ?) AND l.id > ? AND l.id <= ? ORDER BY l.id LIMIT ?

-- name: s0315
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.source = ? AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0316
SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN (?,?,?,?,?)

-- name: s0317
UPDATE tender_version_amounts SET cents = 0, eur_cents = 0
              WHERE tender_id = ?1 AND lot_id IS NOT NULL
                AND seq = (SELECT current_seq FROM tenders WHERE id = ?1)

-- name: s0318
UPDATE tenders SET current_value_eur_cents = NULL WHERE id = ?

-- name: s0319
SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
                   FROM tender_version_lots vl
                   JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id
                   JOIN tenders t ON t.id = vl.tender_id
                   JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq
                  WHERE vl.tender_id = ?
                    AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                   WHERE x.tender_id = ?) AND t.current_value_eur_cents <= ? AND l.id > ? ORDER BY l.id LIMIT ?

-- name: s0320
UPDATE notices SET projected = 1 WHERE id IN (?,?,?,?,?)

-- name: s0321
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM tenders t
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ? WHERE 1 = 1 AND t.source = ? AND t.id = ?

-- name: s0322
SELECT sql FROM sqlite_master WHERE tbl_name = ?1 AND sql IS NOT NULL
                     ORDER BY (type <> 'table')

-- name: s0323
DELETE FROM tender_currency_presence

-- name: s0324
SELECT MIN(id), MAX(id) FROM notices WHERE projected <> 0

-- name: s0325
UPDATE feed_generation SET generation = generation + 1 WHERE id = 0

-- name: s0326
UPDATE notices SET projected = 0 WHERE projected <> 0 AND id BETWEEN ? AND ?

-- name: s0327
SELECT procedure_key, island_notice_id FROM tenders WHERE id = ?

-- name: s0328
SELECT 1 FROM plan_notice WHERE group_key = ? LIMIT 1

-- name: s0329
SELECT id FROM lots WHERE tender_id = ?

-- name: s0330
SELECT id FROM lot_results WHERE tender_id = ?

-- name: s0331
SELECT id FROM bids WHERE tender_id = ?

-- name: s0332
SELECT id FROM contracts WHERE tender_id = ?

-- name: s0333
DELETE FROM tender_version_result_winners WHERE tender_id IN (?)

-- name: s0334
DELETE FROM tender_version_result_stats WHERE tender_id IN (?)

-- name: s0335
DELETE FROM tender_version_lot_results WHERE tender_id IN (?)

-- name: s0336
DELETE FROM tender_version_bid_parties WHERE tender_id IN (?)

-- name: s0337
DELETE FROM tender_version_bids WHERE tender_id IN (?)

-- name: s0338
DELETE FROM tender_version_contracts WHERE tender_id IN (?)

-- name: s0339
DELETE FROM tender_version_parties WHERE tender_id IN (?)

-- name: s0340
DELETE FROM tender_version_texts WHERE tender_id IN (?)

-- name: s0341
DELETE FROM tender_version_dates WHERE tender_id IN (?)

-- name: s0342
DELETE FROM tender_version_amounts WHERE tender_id IN (?)

-- name: s0343
DELETE FROM tender_version_classifications WHERE tender_id IN (?)

-- name: s0344
DELETE FROM tender_version_lot_group_members WHERE tender_id IN (?)

-- name: s0345
DELETE FROM tender_version_lots WHERE tender_id IN (?)

-- name: s0346
DELETE FROM tender_versions WHERE tender_id IN (?)

-- name: s0347
DELETE FROM lot_results WHERE tender_id IN (?)

-- name: s0348
DELETE FROM bids WHERE tender_id IN (?)

-- name: s0349
DELETE FROM contracts WHERE tender_id IN (?)

-- name: s0350
DELETE FROM lots WHERE tender_id IN (?)

-- name: s0351
DELETE FROM tenders WHERE id IN (?)

-- name: s0352
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM tenders t
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ? WHERE 1 = 1 AND t.id = ?

-- name: s0353
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_published_at AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_published_at IS NOT NULL ORDER BY t.current_published_at DESC, t.id DESC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey DESC, t.id DESC

-- name: s0354
SELECT current_published_at FROM tenders WHERE id = ?

-- name: s0355
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_published_at AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_published_at IS NOT NULL AND t.current_published_at <= ? AND (t.current_published_at < ? OR t.id < ?) ORDER BY t.current_published_at DESC, t.id DESC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey DESC, t.id DESC

-- name: s0356
SELECT id, source, publication_id, content_hash, profile, declared_version,
                member_path, ingested_at, parse_state, published_at, dispatched_at,
                published_offset, published_has_time, dispatched_offset, dispatched_has_time
           FROM notices WHERE 1 = 1 AND id = ?

-- name: s0357
SELECT reason, detail, profile, first_seen, attempts, last_attempt_at,
                    reprocessed_at, skipped_at, skipped_reason, first_reason, first_detail
               FROM quarantine WHERE notice_id = ? ORDER BY first_seen DESC LIMIT 1

-- name: s0358
SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE 1 = 1 AND o.id = ?

-- name: s0359
DELETE FROM tender_version_dates
              WHERE tender_id = ?1 AND field = 'submission_deadline'
                AND seq = (SELECT current_seq FROM tenders WHERE id = ?1)

-- name: s0360
INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
             SELECT ?1, current_seq, NULL, 'submission_deadline', ?2, 0, 1 FROM tenders WHERE id = ?1

-- name: s0361
UPDATE tenders SET current_deadline = ? WHERE id = ?

-- name: s0362
SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
                   FROM tender_version_lots vl
                   JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id
                   JOIN tenders t ON t.id = vl.tender_id
                   JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq
                  WHERE vl.tender_id = ?
                    AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                   WHERE x.tender_id = ?) AND EXISTS (SELECT 1 FROM tender_version_dates d
                              WHERE d.tender_id = t.id AND d.seq = v.seq
                                AND d.field = 'submission_deadline' AND d.utc_seconds > ?
                                AND d.utc_seconds - (SELECT pv.published_at FROM tender_versions pv
                       WHERE pv.tender_id = t.id AND pv.seq = v.seq) <= 315360000
                                AND (d.lot_id = l.id
                                     OR (d.lot_id IS NULL
                                         AND NOT EXISTS (SELECT 1 FROM tender_version_dates o
                                                          WHERE o.tender_id = t.id AND o.seq = v.seq
                                                            AND o.field = 'submission_deadline'
                                                            AND o.lot_id = l.id
                                                            AND o.utc_seconds >= 631152000
                                                            AND o.utc_seconds - (SELECT pv.published_at FROM tender_versions pv
                       WHERE pv.tender_id = t.id AND pv.seq = v.seq) <= 315360000)))) AND l.id > ? ORDER BY l.id LIMIT ?

-- name: s0363
SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
                   FROM tender_version_lots vl
                   JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id
                   JOIN tenders t ON t.id = vl.tender_id
                   JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq
                  WHERE vl.tender_id = ?
                    AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                   WHERE x.tender_id = ?) AND NOT EXISTS (SELECT 1 FROM tender_version_dates d
                              WHERE d.tender_id = t.id AND d.seq = v.seq
                                AND d.field = 'submission_deadline' AND d.utc_seconds > ?
                                AND d.utc_seconds - (SELECT pv.published_at FROM tender_versions pv
                       WHERE pv.tender_id = t.id AND pv.seq = v.seq) <= 315360000
                                AND (d.lot_id = l.id
                                     OR (d.lot_id IS NULL
                                         AND NOT EXISTS (SELECT 1 FROM tender_version_dates o
                                                          WHERE o.tender_id = t.id AND o.seq = v.seq
                                                            AND o.field = 'submission_deadline'
                                                            AND o.lot_id = l.id
                                                            AND o.utc_seconds >= 631152000
                                                            AND o.utc_seconds - (SELECT pv.published_at FROM tender_versions pv
                       WHERE pv.tender_id = t.id AND pv.seq = v.seq) <= 315360000)))) AND l.id > ? ORDER BY l.id LIMIT ?

-- name: s0364
INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, 'Zzz Merge Sse', 'zzz merge sse', 1, 1700000000)

-- name: s0365
SELECT tender_id, MAX(seq) FROM tender_versions GROUP BY tender_id LIMIT 1

-- name: s0366
INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
         VALUES (?, ?, 9100, 9002)

-- name: s0367
SELECT 1 FROM tender_version_result_winners WHERE organization_id = ? LIMIT 1

-- name: s0368
SELECT tender_id FROM tender_version_result_winners
          WHERE organization_id = ? AND tender_id > ?
          GROUP BY tender_id ORDER BY tender_id LIMIT ?

-- name: s0369
SELECT tt.id FROM tenders tt
              WHERE tt.id IN (?)
                AND EXISTS (SELECT 1 FROM tender_version_result_winners p
                             WHERE p.organization_id = ? AND p.tender_id = tt.id
                               AND p.seq = tt.current_seq)
              ORDER BY tt.id

-- name: s0370
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.id IN (?) AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0371
SELECT id, name_norm, country FROM organizations WHERE identifier IS NULL AND country IS NOT NULL AND name_norm > ? ORDER BY name_norm, country LIMIT ?

-- name: s0372
SELECT tender_id FROM tender_version_parties WHERE organization_id = ? UNION SELECT tender_id FROM tender_version_bid_parties WHERE organization_id = ? UNION SELECT tender_id FROM tender_version_result_winners WHERE organization_id = ?

-- name: s0373
UPDATE organization_mentions SET organization_id = ? WHERE organization_id = ?

-- name: s0374
UPDATE tender_version_parties SET organization_id = ? WHERE organization_id = ?

-- name: s0375
UPDATE tender_version_bid_parties SET organization_id = ? WHERE organization_id = ?

-- name: s0376
DELETE FROM tender_version_result_winners WHERE organization_id = ? AND EXISTS (SELECT 1 FROM tender_version_result_winners w WHERE w.tender_id = tender_version_result_winners.tender_id AND w.seq = tender_version_result_winners.seq AND w.lot_result_id = tender_version_result_winners.lot_result_id AND w.organization_id = ?)

-- name: s0377
UPDATE tender_version_result_winners SET organization_id = ? WHERE organization_id = ?

-- name: s0378
INSERT OR IGNORE INTO organization_names(org_id, lang, name, name_norm) SELECT ?, lang, name, name_norm FROM organization_names WHERE org_id = ?

-- name: s0379
DELETE FROM organization_names WHERE org_id = ?

-- name: s0380
DELETE FROM organizations WHERE id = ?

-- name: s0381
SELECT seq FROM tender_versions WHERE tender_id = ? ORDER BY seq DESC LIMIT 1

-- name: s0382
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT DISTINCT tender_id FROM tender_version_result_winners WHERE organization_id = ?) hits
                   JOIN tenders t ON t.id = hits.tender_id
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ? WHERE 1 = 1 AND EXISTS (SELECT 1 FROM tender_version_result_winners w
                           WHERE w.tender_id = t.id AND w.seq = v.seq
                             AND w.organization_id = ?) AND t.id = ?

-- name: s0383
SELECT w.organization_id, COUNT(*) AS lots
               FROM tender_version_result_winners w
               JOIN tender_version_lots vl ON vl.tender_id = w.tender_id AND vl.seq = w.seq
              WHERE w.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = w.tender_id)
              GROUP BY w.organization_id
              ORDER BY lots DESC, w.organization_id
              LIMIT 1

-- name: s0384
SELECT tender_id FROM tender_version_result_winners
          WHERE organization_id = ? AND tender_id >= ?
          GROUP BY tender_id ORDER BY tender_id LIMIT ?

-- name: s0385
SELECT l.id, l.tender_id, l.lot_key,
                    (SELECT vl.kind FROM tender_version_lots vl
                      WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                        AND vl.lot_id = l.id),
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
               FROM lots l
              WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                             WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                               AND vl.lot_id = l.id) AND l.tender_id IN (?) AND (l.tender_id > ? OR (l.tender_id = ? AND l.id > ?)) ORDER BY l.tender_id, l.id LIMIT ?

-- name: s0386
SELECT id, source, publication_id, content_hash, profile, declared_version,
                member_path, ingested_at, parse_state, published_at, dispatched_at,
                published_offset, published_has_time, dispatched_offset, dispatched_has_time
           FROM notices WHERE 1 = 1 AND publication_id = ? AND id > ? ORDER BY id LIMIT ?

-- name: s0387
SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE 1 = 1 AND o.identifier = ? AND o.id > ? ORDER BY o.id LIMIT ?

-- name: s0388
SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE 1 = 1 AND o.identifier_kind = ? AND o.identifier = ? AND o.id > ? ORDER BY o.id LIMIT ?

-- name: s0389
SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE o.name_norm >= ? AND o.name_norm < ? ORDER BY o.name_norm, o.id LIMIT ?

-- name: s0390
SELECT 1 FROM tender_version_bid_parties WHERE organization_id = ? LIMIT 1

-- name: s0391
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_published_at AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_published_at IS NOT NULL AND t.current_published_at >= ? ORDER BY t.current_published_at DESC, t.id DESC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey DESC, t.id DESC

-- name: s0392
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_deadline AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_deadline IS NOT NULL AND t.current_deadline >= ? ORDER BY t.current_deadline ASC, t.id ASC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey ASC, t.id ASC

-- name: s0393
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_deadline AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_deadline IS NOT NULL ORDER BY t.current_deadline ASC, t.id ASC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey ASC, t.id ASC

-- name: s0394
SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE 1 = 1 AND o.country = ? AND o.id > ? ORDER BY o.id LIMIT ?

-- name: s0395
SELECT id, source, publication_id, content_hash, profile, declared_version,
                member_path, ingested_at, parse_state, published_at, dispatched_at,
                published_offset, published_has_time, dispatched_offset, dispatched_has_time
           FROM notices WHERE 1 = 1 AND source = ? AND id > ? ORDER BY id LIMIT ?

-- name: s0396
SELECT l.id, l.tender_id, l.lot_key,
                    (SELECT vl.kind FROM tender_version_lots vl
                      WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                        AND vl.lot_id = l.id),
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
               FROM lots l
              WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                             WHERE vl.tender_id = l.tender_id AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                               AND vl.lot_id = l.id) AND (SELECT tt.source FROM tenders tt WHERE tt.id = l.tender_id) = ? AND l.tender_id IN (SELECT t.id FROM tenders t
                       WHERE t.current_deadline > ?) AND EXISTS (SELECT 1 FROM tender_version_dates d
                              WHERE d.tender_id = l.tender_id AND d.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                                AND d.field = 'submission_deadline' AND d.utc_seconds > ?
                                AND d.utc_seconds - (SELECT pv.published_at FROM tender_versions pv
                       WHERE pv.tender_id = l.tender_id AND pv.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)) <= 315360000
                                AND (d.lot_id = l.id
                                     OR (d.lot_id IS NULL
                                         AND NOT EXISTS (SELECT 1 FROM tender_version_dates o
                                                          WHERE o.tender_id = l.tender_id AND o.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                                                            AND o.field = 'submission_deadline'
                                                            AND o.lot_id = l.id
                                                            AND o.utc_seconds >= 631152000
                                                            AND o.utc_seconds - (SELECT pv.published_at FROM tender_versions pv
                       WHERE pv.tender_id = l.tender_id AND pv.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)) <= 315360000)))) AND l.id > ? AND l.id <= ? ORDER BY l.id LIMIT ?

-- name: s0397
SELECT organization_id, tender_id FROM tender_version_bid_parties WHERE role = 'tenderer' LIMIT 1

-- name: s0398
SELECT tender_id FROM tender_version_bid_parties
          WHERE organization_id = ? AND tender_id > ?
          GROUP BY tender_id ORDER BY tender_id LIMIT ?

-- name: s0399
SELECT tt.id FROM tenders tt
              WHERE tt.id IN (?)
                AND EXISTS (SELECT 1 FROM tender_version_bid_parties p
                             WHERE p.organization_id = ? AND p.tender_id = tt.id
                               AND p.seq = tt.current_seq AND role = 'tenderer')
              ORDER BY tt.id

-- name: s0400
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM (SELECT DISTINCT tender_id FROM tender_versions WHERE publication_id = ?) hits
               JOIN tenders t ON t.id = hits.tender_id
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0401
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_published_at AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_published_at IS NOT NULL ORDER BY t.current_published_at ASC, t.id ASC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey ASC, t.id ASC

-- name: s0402
SELECT p.organization_id, p.tender_id FROM tender_version_parties p
              WHERE p.role LIKE '%Buyer%'
                AND p.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = p.tender_id)
              LIMIT 1

-- name: s0403
SELECT p.organization_id FROM tender_version_parties p
              WHERE p.organization_id NOT IN
                    (SELECT organization_id FROM tender_version_parties WHERE role LIKE '%Buyer%')
              LIMIT 1

-- name: s0404
SELECT 1 FROM tender_version_parties WHERE organization_id = ? LIMIT 1

-- name: s0405
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM (SELECT DISTINCT tender_id FROM tender_version_parties WHERE organization_id = ? AND role LIKE '%Buyer%') hits
                   JOIN tenders t ON t.id = hits.tender_id
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND EXISTS (SELECT 1 FROM tender_version_parties p
                           WHERE p.tender_id = t.id AND p.seq = v.seq
                             AND p.organization_id = ? AND p.role LIKE '%Buyer%') AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0406
SELECT w.organization_id, w.tender_id, t.source
               FROM tender_version_result_winners w
               JOIN tenders t ON t.id = w.tender_id
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.seq
              WHERE v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              LIMIT 1

-- name: s0407
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.id IN (?) AND t.id > ? AND t.source = ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0408
SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes
                  WHERE cursor > ? ORDER BY cursor LIMIT ?

-- name: s0409
SELECT name, ever_populated, went_empty_at, observed_at FROM layer_presence

-- name: s0410
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND EXISTS (SELECT 1 FROM tender_version_classifications c
                           WHERE c.tender_id = t.id AND c.seq = v.seq
                             AND c.scheme = 'cpv' AND c.code LIKE ?) AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0411
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.current_value_eur_cents >= ? AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0412
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.current_value_eur_cents <= ? AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0413
SELECT 1 FROM projection_state WHERE id = 0 AND currency_presence_complete <> 0

-- name: s0414
SELECT 1 FROM tender_currency_presence WHERE currency = ? LIMIT 1

-- name: s0415
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND EXISTS (SELECT 1 FROM tender_version_amounts a
                           WHERE a.tender_id = t.id AND a.seq = v.seq
                             AND a.currency = ?) AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0416
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND (t.current_deadline IS NULL OR t.current_deadline <= ?) AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0417
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.current_deadline > ? AND t.id > ? AND t.id <= ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0418
SELECT watermark FROM legacy_adjacency WHERE id = 0

-- name: s0419
SELECT kind, computed_at FROM reports

-- name: s0420
SELECT name, type FROM sqlite_schema
              WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%'
              ORDER BY type DESC, name

-- name: s0421
SELECT sql FROM sqlite_schema WHERE type = 'view' AND name = ?

-- name: s0422
SELECT current_deadline FROM tenders WHERE id = ?

-- name: s0423
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ISL') DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0424
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'DEU') DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1 AND t.id > ? ORDER BY t.id LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY t.id

-- name: s0425
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_deadline AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_deadline IS NOT NULL AND t.current_deadline < ? ORDER BY t.current_deadline ASC, t.id ASC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey ASC, t.id ASC

-- name: s0426
SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                (SELECT s.value FROM tender_version_texts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND 1 = 1 AND s.field = 'title'
           ORDER BY (s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value LIMIT 1),
                (SELECT s.cents FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.currency FROM tender_version_amounts s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents
           ORDER BY s.cents DESC, s.currency LIMIT 1),
                (SELECT s.utc_seconds FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.offset_minutes FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1), (SELECT s.has_time FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1),
                (SELECT COUNT(*) FROM tender_version_lots l
                  WHERE l.tender_id = t.id AND l.seq = v.seq),
                v.dispatched_at,
                -- The version's CPV and NUTS codes, echoed so a list row
                -- shows why it matched a cpv/country filter (issue 49). Both
                -- seek by (tender_id, seq) on the classifications index.
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang,
                -- How the causing notice published its two instants (issue 367
                -- unit 3): four PK seeks on notices, so a list row can serve a
                -- date-only publication as the date the source stated.
                (SELECT n.published_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.published_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_offset FROM notices n WHERE n.id = v.caused_by_notice_id),
                (SELECT n.dispatched_has_time FROM notices n WHERE n.id = v.caused_by_notice_id),
                -- The elected deadline's lot_id: NULL for the procedure's own
                -- date, a lot for a lot-level one (issue 370 unit 4's scope).
                (SELECT s.lot_id FROM tender_version_dates s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND s.utc_seconds >= 631152000 AND s.utc_seconds - v.published_at <= 315360000 AND s.field = 'submission_deadline'
           ORDER BY s.utc_seconds DESC, (s.lot_id IS NULL) DESC, s.lot_id LIMIT 1)
           FROM (SELECT t.id AS wid, v.seq AS wseq, t.current_published_at AS wkey
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE t.current_published_at IS NOT NULL AND t.current_published_at < ? ORDER BY t.current_published_at DESC, t.id DESC LIMIT ?) w JOIN tenders t ON t.id = w.wid
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.wseq ORDER BY w.wkey DESC, t.id DESC

-- name: s0427
SELECT COUNT(*) FROM users WHERE created_at > ?

-- name: s0428
INSERT OR IGNORE INTO users(username, password_hash, created_at)
                 SELECT ?, ?, ?
                  WHERE (SELECT COUNT(*) FROM users WHERE created_at > ?) < ?

-- name: s0429
SELECT id, username, created_at FROM users WHERE username = ?

-- name: s0430
INSERT INTO sessions(id_hash, user_id, created_at, expires_at) VALUES(?, ?, ?, ?)

-- name: s0431
INSERT INTO api_tokens(user_id, token_hash, prefix_hint, name, created_at)
             VALUES(?, ?, ?, ?, ?)

-- name: s0432
SELECT id FROM api_tokens WHERE token_hash = ?

-- name: s0433
SELECT u.id, u.username, u.created_at FROM api_tokens k
                   JOIN users u ON u.id = k.user_id
                  WHERE k.token_hash = ? AND k.revoked_at IS NULL

-- name: s0434
UPDATE api_tokens SET last_used_at = ? WHERE token_hash = ?

-- name: s0435
SELECT a.value, (SELECT COUNT(*) FROM generate_series(a.value, a.value + 20000000)) FROM generate_series(1, 2000) a

-- name: s0436
SELECT a.value, (SELECT COUNT(*) FROM generate_series(a.value * 1000000, a.value * 1000000 + 5000000)) FROM generate_series(1, 30) a

-- name: s0437
SELECT COUNT(*) FROM generate_series(1, 7000) a, generate_series(1, 7000) b

-- name: s0438
SELECT 1 AS ok

-- name: s0439
SELECT 'sessions expire' AS note

-- name: s0440
SELECT profile, COUNT(*) AS n FROM notices GROUP BY profile ORDER BY n DESC

-- name: s0441
SELECT o.name, SUM(a.cents) AS total_cents
               FROM tenders t
               JOIN tender_version_parties p
                 ON p.tender_id = t.id AND p.seq = t.current_seq AND p.role = 'buyer'
               JOIN organizations o ON o.id = p.organization_id
               JOIN tender_version_amounts a
                 ON a.tender_id = t.id AND a.seq = t.current_seq
               JOIN tender_version_classifications x
                 ON x.tender_id = t.id AND x.seq = t.current_seq AND x.scheme = 'cpv'
              WHERE x.code LIKE '7%'
              GROUP BY o.id
              ORDER BY total_cents DESC
              LIMIT 10

-- name: s0442
SELECT value FROM generate_series(1, 20000)

-- name: s0443
SELECT value FROM generate_series(1, 5)

-- name: s0444
SELECT t.id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq LIMIT 5

-- name: s0445
SELECT source, COUNT(*) AS n FROM notices GROUP BY source HAVING COUNT(*) >= 0 ORDER BY n DESC LIMIT 5

-- name: s0446
WITH recent AS (SELECT id FROM notices ORDER BY id DESC LIMIT 5) SELECT COUNT(*) FROM recent

-- name: s0447
SELECT strftime('%Y', published_at, 'unixepoch') AS y, COUNT(*) FROM tender_versions GROUP BY y LIMIT 5

-- name: s0448
SELECT CASE WHEN cents < 0 THEN 'neg' WHEN cents = 0 THEN 'zero' ELSE 'pos' END AS b, COUNT(*) FROM tender_version_amounts GROUP BY b

-- name: s0449
SELECT id, (SELECT COUNT(*) FROM tender_versions v WHERE v.tender_id = t.id) FROM tenders t LIMIT 5

-- name: s0450
SELECT COUNT(*) FROM notices WHERE profile LIKE 'eforms%'

-- name: s0451
SELECT COUNT(*) FROM tenders WHERE id IN (SELECT tender_id FROM tender_versions LIMIT 10)

-- name: s0452
SELECT 'a' AS k UNION ALL SELECT 'b' ORDER BY k

-- name: s0453
SELECT COALESCE(CAST(NULL AS INTEGER), 42)

-- name: s0454
SELECT id, row_number() OVER () FROM tenders LIMIT 3

-- name: s0455
SELECT rate_to_eur, source FROM currency_rates WHERE currency = 'DEM' AND rate_date <= '1999-06-01' ORDER BY rate_date DESC LIMIT 1

-- name: s0456
SELECT 1

-- name: s0457
SELECT * FROM generate_series(1, 3)

-- name: s0458
SELECT * FROM v_tender_buyers LIMIT 5

-- name: s0459
SELECT * FROM v_awards LIMIT 5

-- name: s0460
SELECT * FROM v_tender_classifications LIMIT 5

-- name: s0461
SELECT * FROM v_tender_amounts LIMIT 5

-- name: s0462
SELECT * FROM v_tender_dates LIMIT 5

-- name: s0463
SELECT * FROM v_tender_notices LIMIT 5

-- name: s0464
SELECT * FROM v_fetches LIMIT 5

-- name: s0465
SELECT count(*) FROM v_tender_buyers

-- name: s0466
SELECT count(*) FROM tenders t
               JOIN tender_version_classifications x
                 ON x.tender_id = t.id AND x.seq = t.current_seq
              WHERE x.scheme = 'cpv'

-- name: s0467
SELECT count(*) FROM v_tender_notices

-- name: s0468
SELECT * FROM v_fetches LIMIT 1

-- name: s0469
SELECT COUNT(*) AS n FROM notices

-- name: s0470
SELECT id, username, created_at FROM users WHERE id = ?

-- name: s0471
INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, dispatched_at, notice_subtype, original_lang, publication_id) VALUES (?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?)

-- name: s0472
INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind) VALUES (?,?,?,?),(?,?,?,?),(?,?,?,?),(?,?,?,?)

-- name: s0473
INSERT INTO tender_version_texts(tender_id, seq, lot_id, field, lang, value) VALUES (?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?)

-- name: s0474
INSERT INTO tender_version_amounts(tender_id, seq, lot_id, field, cents, currency, tax_basis, eur_cents, quality) VALUES (?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?),(?,?,?,?,?,?,?,?,?)

-- name: s0475
INSERT INTO tender_version_classifications(tender_id, seq, lot_id, field, scheme, code) VALUES (?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?),(?,?,?,?,?,?)

-- name: s0476
INSERT INTO tender_version_dates(tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0477
INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) VALUES (?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?),(?,?,?,?,?,?,?)

-- name: s0478
INSERT INTO webhook_endpoints(user_id, url, secret, created_at, last_delivered_cursor,
                 last_generation, next_attempt_at)
             VALUES(?, ?, ?, ?, ?, ?, 0)

-- name: s0479
SELECT id, user_id, url, secret, created_at, disabled_at,
    last_delivered_cursor, failing_since, next_attempt_at, consecutive_failures, last_generation FROM webhook_endpoints WHERE id = last_insert_rowid()

-- name: s0480
SELECT id, user_id, url, secret, created_at, disabled_at,
    last_delivered_cursor, failing_since, next_attempt_at, consecutive_failures, last_generation FROM webhook_endpoints
                      WHERE disabled_at IS NULL AND next_attempt_at <= ? ORDER BY id

-- name: s0481
UPDATE webhook_endpoints
                SET last_delivered_cursor = ?, last_generation = ?, failing_since = NULL,
                    consecutive_failures = 0, next_attempt_at = 0
              WHERE id = ?

-- name: s0482
INSERT INTO webhook_delivery_log(endpoint_id, attempted_at, cursor_from, cursor_to,
                 events, status, duration_ms, ok, error)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)

-- name: s0483
DELETE FROM webhook_delivery_log WHERE endpoint_id = ? AND id NOT IN (
                 SELECT id FROM webhook_delivery_log WHERE endpoint_id = ? ORDER BY id DESC LIMIT ?)

-- name: s0484
SELECT id, user_id, url, secret, created_at, disabled_at,
    last_delivered_cursor, failing_since, next_attempt_at, consecutive_failures, last_generation FROM webhook_endpoints WHERE id = ? AND user_id = ?

-- name: s0485
UPDATE webhook_endpoints
                SET consecutive_failures = consecutive_failures + 1,
                    failing_since = COALESCE(failing_since, ?),
                    next_attempt_at = ?,
                    disabled_at = CASE WHEN ? = 1 THEN ? ELSE disabled_at END
              WHERE id = ?

-- name: s0486
SELECT attempted_at, cursor_from, cursor_to, events, status, duration_ms, ok, error
                   FROM webhook_delivery_log WHERE endpoint_id = ? ORDER BY id DESC LIMIT ?

-- name: s0487
UPDATE webhook_endpoints
                        SET disabled_at = NULL, failing_since = NULL,
                            consecutive_failures = 0, next_attempt_at = 0
                      WHERE id = ? AND user_id = ?

-- name: s0488
SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'plan_notice'

-- name: s0489
SELECT COUNT(*) FROM plan_notice

-- name: s0490
UPDATE projection_state SET org_match_keys_watermark = 0, org_edge_total = 0, org_match_keys_epoch = '' WHERE id = 0

-- name: s0491
DELETE FROM org_name_drops

-- name: s0492
UPDATE projection_state SET rebuild_in_progress = 1 WHERE id = 0

-- name: s0493
DELETE FROM sqlite_sequence WHERE name IN ('tenders','lots','bids','contracts','lot_results')

-- name: s0494
SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'organizations'

-- name: s0495
SELECT * FROM sqlite_schema WHERE name = 'organizations_identity' AND type = 'index'

-- name: s0496
SELECT MAX(rowid) FROM organization_mentions

-- name: s0497
SELECT * FROM sqlite_schema WHERE name = 'organization_mentions_notice' AND type = 'index'

-- name: s0498
SELECT MAX(rowid) FROM organizations

-- name: s0499
SELECT * FROM sqlite_schema WHERE name = 'organizations_country_id' AND type = 'index'

-- name: s0500
SELECT * FROM sqlite_schema WHERE name = 'organizations_kind_id' AND type = 'index'

-- name: s0501
SELECT * FROM sqlite_schema WHERE name = 'organizations_identifier_id' AND type = 'index'

-- name: s0502
SELECT * FROM sqlite_schema WHERE name = 'organizations_name_norm_id' AND type = 'index'

-- name: s0503
SELECT * FROM sqlite_schema WHERE name = 'organizations_name_country' AND type = 'index'

-- name: s0504
SELECT MAX(rowid) FROM tender_versions

-- name: s0505
SELECT * FROM sqlite_schema WHERE name = 'tender_versions_publication' AND type = 'index'

-- name: s0506
SELECT MAX(rowid) FROM tender_version_classifications

-- name: s0507
SELECT MAX(rowid) FROM tender_version_parties

-- name: s0508
SELECT * FROM sqlite_schema WHERE name = 'tender_version_parties_mention' AND type = 'index'

-- name: s0509
SELECT MAX(rowid) FROM tender_version_bid_parties

-- name: s0510
SELECT * FROM sqlite_schema WHERE name = 'tender_version_bid_parties_mention' AND type = 'index'

-- name: s0511
SELECT * FROM sqlite_schema WHERE name = 'tender_version_parties_org_role' AND type = 'index'

-- name: s0512
SELECT MAX(rowid) FROM tender_version_result_winners

-- name: s0513
SELECT * FROM sqlite_schema WHERE name = 'tender_version_result_winners_org_tender' AND type = 'index'

-- name: s0514
SELECT * FROM sqlite_schema WHERE name = 'tender_version_bid_parties_org_tender' AND type = 'index'

-- name: s0515
SELECT * FROM sqlite_schema WHERE name = 'tender_version_bid_parties_version' AND type = 'index'

-- name: s0516
SELECT MAX(rowid) FROM tenders

-- name: s0517
SELECT * FROM sqlite_schema WHERE name = 'tenders_procedure_key' AND type = 'index'

-- name: s0518
SELECT * FROM sqlite_schema WHERE name = 'tenders_island' AND type = 'index'

-- name: s0519
SELECT * FROM sqlite_schema WHERE name = 'tenders_current_deadline' AND type = 'index'

-- name: s0520
SELECT * FROM sqlite_schema WHERE name = 'tenders_current_value_eur' AND type = 'index'

-- name: s0521
SELECT * FROM sqlite_schema WHERE name = 'tenders_source_id' AND type = 'index'

