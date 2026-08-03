-- Post-build canonical-layer verification — read-only. See README.md for pass
-- criteria and the LIGHT/HEAVY execution guidance.
--
-- [LIGHT] = safe on the live server via /v1/sql (single SELECT) or /admin.
-- [HEAVY] = run with stock sqlite3 (real SQLite, no turso hash cliff) against the
--           idle DB or a post-build snapshot — NOT through turso.
--
-- Expected: tenders = 6,961,311 ; islands = 640,745 ; keyed = 6,320,566.

-- ============================================================ 1. COUNT invariants

-- 1.1 [LIGHT] total tenders → EXPECT 6961311
SELECT COUNT(*) AS tenders_total FROM tenders;

-- 1.2 [LIGHT] islands → EXPECT 640745
SELECT COUNT(*) AS islands FROM tenders WHERE island_notice_id IS NOT NULL;

-- 1.3 [LIGHT] keyed → EXPECT 6320566
SELECT COUNT(*) AS keyed FROM tenders WHERE procedure_key IS NOT NULL;

-- 1.4 [LIGHT] identity mutual-exclusivity → EXPECT 0
--     counts rows where BOTH are null or BOTH are set (either = a broken identity)
SELECT COUNT(*) AS identity_violations
  FROM tenders WHERE (procedure_key IS NULL) = (island_notice_id IS NULL);

-- 1.5 [LIGHT] every tender has a maintained head version → EXPECT 0
SELECT COUNT(*) AS tenders_without_head FROM tenders WHERE current_seq IS NULL;

-- 1.6 [LIGHT] kind domain → EXPECT only 'procedure' and 'registration'
SELECT kind, COUNT(*) AS n FROM tenders GROUP BY kind ORDER BY n DESC;

-- 1.7 [LIGHT] source domain → EXPECT only 'ted' and 'doe'
SELECT source, COUNT(*) AS n FROM tenders GROUP BY source ORDER BY n DESC;

-- 1.8 [LIGHT] total versions → EXPECT ≈ 12.36M and ≥ 6,961,311
SELECT COUNT(*) AS versions_total FROM tender_versions;

-- 1.9 [LIGHT] organizations + mentions non-empty; mentions ≥ orgs
SELECT (SELECT COUNT(*) FROM organizations)         AS organizations,
       (SELECT COUNT(*) FROM organization_mentions) AS mentions;

-- 1.10 [LIGHT] provisional split (0 = merged/identified, 1 = one-mention profile)
SELECT provisional, COUNT(*) AS n FROM organizations GROUP BY provisional;

-- ========================================================= 2. Domain invariants

-- 2.1 [LIGHT] islands are single-notice tenders → EXPECT 0
SELECT COUNT(*) AS islands_multi_version
  FROM tenders WHERE island_notice_id IS NOT NULL AND current_seq <> 1;

-- 2.2 [LIGHT] version seq is dense from 1 → EXPECT min_seq = 1
SELECT MIN(seq) AS min_seq, MAX(seq) AS max_seq FROM tender_versions;

-- 2.3 [HEAVY] a notice must cause a version in exactly ONE tender → EXPECT 0 rows
--     high-cardinality GROUP BY over ~12.36M — sqlite3 only.
SELECT caused_by_notice_id, COUNT(*) AS in_n_tenders
  FROM tender_versions GROUP BY caused_by_notice_id HAVING COUNT(*) > 1 LIMIT 50;

-- 2.4 [HEAVY] head pointer matches its version's publication date → EXPECT 0
--     join on tender_versions PK — millions of indexed probes; sqlite3.
SELECT COUNT(*) AS head_pub_mismatch
  FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq
 WHERE t.current_published_at <> v.published_at;

-- 2.5 [HEAVY] current_seq is the maximum seq → EXPECT 0
SELECT COUNT(*) AS head_not_max
  FROM tenders t
 WHERE EXISTS (SELECT 1 FROM tender_versions v
                WHERE v.tender_id = t.id AND v.seq > t.current_seq);

-- 2.6 [LIGHT] provisional ⟺ no normalised identifier → EXPECT 0
SELECT COUNT(*) AS provisional_identifier_violations
  FROM organizations WHERE (provisional = 1) <> (identifier IS NULL);

-- 2.7 [LIGHT] identifier_kind domain → EXPECT only NULL / 'vat' / 'national'
SELECT identifier_kind, COUNT(*) AS n FROM organizations GROUP BY identifier_kind;

-- 2.8 [HEAVY] org identity uniqueness among identified profiles → EXPECT 0 rows
--     high-cardinality GROUP BY — sqlite3. (A dup ⇒ organizations_identity failed.)
SELECT country, identifier_kind, identifier, COUNT(*) AS n
  FROM organizations WHERE identifier IS NOT NULL
 GROUP BY country, identifier_kind, identifier HAVING COUNT(*) > 1 LIMIT 50;

-- 2.9 [LIGHT] changes domains
SELECT op, COUNT(*) AS n FROM changes GROUP BY op;
SELECT entity_kind, COUNT(*) AS n FROM changes GROUP BY entity_kind;

-- ==================================================== 3. Data-quality / weirdness

-- 3.1 [LIGHT] mega-tender tail (JUNK-HUB red flag) — thresholds on version count.
--     Expect a thin decaying tail; a fat >1000 bucket ⇒ bad legacy-OJS merge.
SELECT
  (SELECT COUNT(*) FROM tenders WHERE current_seq >   50) AS gt_50,
  (SELECT COUNT(*) FROM tenders WHERE current_seq >  100) AS gt_100,
  (SELECT COUNT(*) FROM tenders WHERE current_seq >  500) AS gt_500,
  (SELECT COUNT(*) FROM tenders WHERE current_seq > 1000) AS gt_1000;

-- 3.2 [MED] the biggest chains — eyeball legit-framework vs junk-hub.
--     Full sort of tenders for top-30 (bounded output). Fine via sqlite3; may be
--     heavy via /v1/sql (no index on current_seq) — run there only if it returns.
SELECT id, source, kind, procedure_key, island_notice_id, current_seq
  FROM tenders ORDER BY current_seq DESC LIMIT 30;

-- 3.3 [LIGHT] notices-per-tender histogram (thresholds; islands are exactly 1)
SELECT
  (SELECT COUNT(*) FROM tenders WHERE current_seq  = 1)             AS v1,
  (SELECT COUNT(*) FROM tenders WHERE current_seq  = 2)             AS v2,
  (SELECT COUNT(*) FROM tenders WHERE current_seq BETWEEN 3 AND 5)  AS v3_5,
  (SELECT COUNT(*) FROM tenders WHERE current_seq BETWEEN 6 AND 10) AS v6_10,
  (SELECT COUNT(*) FROM tenders WHERE current_seq BETWEEN 11 AND 50)AS v11_50,
  (SELECT COUNT(*) FROM tenders WHERE current_seq > 50)             AS v_gt50;

-- 3.4 [HEAVY] orgs by mention count — over-merge check. sqlite3 only (30M-row
--     high-cardinality GROUP BY). Eyeball: a huge count = real big buyer OR a
--     junk identifier merging distinct companies.
SELECT organization_id, COUNT(*) AS mentions
  FROM organization_mentions GROUP BY organization_id ORDER BY mentions DESC LIMIT 30;

-- 3.5a [LIGHT] version subtype-null rate (high for legacy is expected)
SELECT COUNT(*) AS versions, SUM(notice_subtype IS NULL) AS subtype_null
  FROM tender_versions;

-- 3.5b [HEAVY] tenders whose CURRENT version carries no title → count, eyeball.
--     correlated existence over the by-version text index; sqlite3.
SELECT COUNT(*) AS current_versions_without_title
  FROM tenders t
 WHERE NOT EXISTS (SELECT 1 FROM tender_version_texts x
                    WHERE x.tender_id = t.id AND x.seq = t.current_seq
                      AND x.field = 'title');

-- 3.6 [LIGHT] publication-date sanity → EXPECT 0 (before 1990-01-01 or after ~2027-01-15)
SELECT COUNT(*) AS absurd_pub_dates
  FROM tender_versions WHERE published_at < 631152000 OR published_at > 1800000000;

-- 3.7 [LIGHT] negative money → EXPECT 0 for each
SELECT (SELECT COUNT(*) FROM tender_version_amounts     WHERE cents < 0)         AS neg_amounts,
       (SELECT COUNT(*) FROM tender_version_lot_results WHERE awarded_cents < 0) AS neg_awards,
       (SELECT COUNT(*) FROM tender_version_bids        WHERE cents < 0)         AS neg_bids;

-- 3.8 [LIGHT] results-layer presence → EXPECT all > 0
SELECT (SELECT COUNT(*) FROM lot_results) AS lot_results,
       (SELECT COUNT(*) FROM bids)        AS bids,
       (SELECT COUNT(*) FROM contracts)   AS contracts;

-- ======================================================== 4. changes feed sanity

-- 4.1 [LIGHT] cursor monotonic (AUTOINCREMENT) — EXPECT max_cursor ≥ rows
SELECT MIN(cursor) AS min_cursor, MAX(cursor) AS max_cursor, COUNT(*) AS rows
  FROM changes;

-- 4.3 [LIGHT] fallback-prefix note — NOT a failure. `changes` is not cleared on
--     rebuild, so tender/added ≥ 6,961,311 (fallback prefix + full run). Harmless.
SELECT COUNT(*) AS tender_added_events
  FROM changes WHERE entity_kind = 'tender' AND op = 'added';

-- ================================= 6. Incremental watermark / post-plan suffix

-- 6.1 [LIGHT] the ~14k notices parsed AFTER the plan was built are left
--     UNPROJECTED for the incremental fold (NOT silently dropped) → EXPECT ≈ 14k.
--     Uses the notices_unprojected partial index — instant.
SELECT COUNT(*) AS unprojected_parsed
  FROM notices WHERE parse_state = 'parsed' AND projected = 0;

-- 6.2 [LIGHT] the unprojected set is a CLEAN CONTIGUOUS SUFFIX (nothing dropped
--     mid-corpus): every parsed notice at/above the first unprojected id is itself
--     unprojected → EXPECT the two counts EQUAL.
SELECT
  (SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=0) AS unprojected,
  (SELECT COUNT(*) FROM notices WHERE parse_state='parsed'
     AND id >= (SELECT MIN(id) FROM notices WHERE parse_state='parsed' AND projected=0)
   ) AS parsed_at_or_above_first_unprojected;

-- 6.3 [LIGHT] cross-check: folded (projected=1) notices ≈ versions (one version each)
SELECT (SELECT COUNT(*) FROM notices WHERE parse_state='parsed' AND projected=1) AS projected,
       (SELECT COUNT(*) FROM tender_versions) AS versions;

-- ============================= 5. Referential integrity (orphans) — [HEAVY] sqlite3

-- 5.0 Authoritative, whole-DB (sqlite3 idle; may take a while at 254GB):
--     PRAGMA foreign_key_check;
--   or per-table, e.g.:  PRAGMA foreign_key_check(tender_version_texts);

-- 5.1 Portable anti-joins — EXPECT 0 each. Satellites → tender_versions(tender_id,seq):
SELECT COUNT(*) AS orphan_texts
  FROM tender_version_texts x
  LEFT JOIN tender_versions v ON v.tender_id = x.tender_id AND v.seq = x.seq
 WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_parties
  FROM tender_version_parties x
  LEFT JOIN tender_versions v ON v.tender_id = x.tender_id AND v.seq = x.seq
 WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_lot_results
  FROM tender_version_lot_results x
  LEFT JOIN tender_versions v ON v.tender_id = x.tender_id AND v.seq = x.seq
 WHERE v.tender_id IS NULL;

-- 5.2 tender_versions → tenders → EXPECT 0
SELECT COUNT(*) AS orphan_versions
  FROM tender_versions v LEFT JOIN tenders t ON t.id = v.tender_id
 WHERE t.id IS NULL;

-- 5.3 parties/winners → organizations → EXPECT 0
SELECT COUNT(*) AS orphan_party_orgs
  FROM tender_version_parties p LEFT JOIN organizations o ON o.id = p.organization_id
 WHERE o.id IS NULL;

-- 5.4 lots → tenders, and lot_results → tenders → EXPECT 0
SELECT COUNT(*) AS orphan_lots
  FROM lots l LEFT JOIN tenders t ON t.id = l.tender_id WHERE t.id IS NULL;
SELECT COUNT(*) AS orphan_lotresults
  FROM lot_results r LEFT JOIN tenders t ON t.id = r.tender_id WHERE t.id IS NULL;
