-- ============================================================================
--
-- SUPERSEDED ENGINE (ruling 2026-08-05): this reads fixtures with STOCK SQLITE3.
-- The owner ruled turso-only, and the ruling is a CORRECTNESS one, not a matter of
-- consistency: issue 112's whole lesson is that stock-sqlite3 plans are not turso
-- plans. A bed characterising READ PERFORMANCE under sqlite3 measures the wrong
-- engine for exactly the questions 30a/30b ask. When this work is picked up it is
-- to be REBUILT ON TURSO from the start. The DESIGN below carries over — the
-- selectivity bounds, the referential-coverage check, and the corrected anchor
-- (matches must RUN OUT with most of the table still ahead, and the count must
-- exceed LIMIT so pagination reaches an unfillable page). The ENGINE does not.
--
-- fixture_gen.sql — issue 30a. Generate a read bed that is FIT TO CLOCK.
--
-- Companion to fixture_selectivity_gate.sh, which must pass on the output. The
-- gate was written first and deliberately fails the old bed (18/24); this file
-- exists to produce one that passes for the right reasons.
--
-- SCALE is the token __N__ (tenders), substituted by the runner, so the same
-- shape can be generated small for a structural check and large for a clock.
--
-- WHAT THIS FILE IS NOT. The DISTRIBUTION SHAPES here are placeholders with the
-- right STRUCTURE, not prod's measured skew. Prod's `kind` values and their
-- proportions come from the authorised Phase A snapshot read
--   SELECT kind, source, (id*10)/((SELECT MAX(id)+1 FROM tenders)) AS id_decile,
--          COUNT(*), MIN(id), MAX(id) FROM tenders GROUP BY 1,2,3
-- and get substituted here. Inventing a skew is what made issue 117 wrong; this
-- file must not repeat it. Until Phase A lands, output is fit for STRUCTURAL
-- checks (does the pipeline work, does the gate pass) and NOT for timings.
--
-- THE ONE PROPERTY THAT IS NOT A PLACEHOLDER: `registration` is rare AND LATE in
-- id order. Rarity alone does not reproduce the 18.7s read (issue 122) — the read
-- paginates `AND t.id > ? ORDER BY t.id LIMIT ?`, so a rare value sprinkled
-- uniformly lets the walk stop early and the pathology VANISHES while the row
-- counts still look correct. Here every `registration` sits in the last decile,
-- so the walk must traverse ~90% of the table before LIMIT can fill.
--
-- NEVER RUN A BARE `ANALYZE` ON THE OUTPUT. Prod has no stat1 row for any
-- read-path table (measured 2026-08-04: prod's sqlite_stat1 holds exactly one
-- row, for plan_notice). Statistics here would make the bed's planner strictly
-- smarter than prod's, and every plan measured would be one prod cannot produce.
-- ============================================================================

PRAGMA journal_mode=OFF;
PRAGMA synchronous=OFF;
PRAGMA cache_size=-200000;

CREATE TABLE tenders (id INTEGER PRIMARY KEY, source TEXT NOT NULL, procedure_key TEXT, kind TEXT NOT NULL, current_seq INTEGER, current_published_at INTEGER, island_notice_id INTEGER);
CREATE TABLE tender_versions (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, published_at INTEGER NOT NULL, dispatched_at INTEGER, publication_id TEXT NOT NULL, notice_subtype TEXT, caused_by_notice_id INTEGER NOT NULL, UNIQUE (tender_id, seq));
CREATE TABLE lots (id INTEGER PRIMARY KEY, tender_id INTEGER NOT NULL, lot_key TEXT NOT NULL, UNIQUE (tender_id, lot_key));
CREATE TABLE tender_version_lots (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER NOT NULL, kind TEXT NOT NULL, UNIQUE (tender_id, seq, lot_id));
CREATE TABLE organizations (id INTEGER PRIMARY KEY, name TEXT, country TEXT, identifier_kind TEXT);
CREATE TABLE organization_mentions (notice_id INTEGER NOT NULL, section_id TEXT NOT NULL, organization_id INTEGER, PRIMARY KEY (notice_id, section_id));
CREATE TABLE lot_results (id INTEGER PRIMARY KEY, tender_id INTEGER);
CREATE TABLE tender_version_result_winners (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_result_id INTEGER NOT NULL, organization_id INTEGER NOT NULL, PRIMARY KEY (tender_id, seq, lot_result_id, organization_id));
CREATE TABLE tender_version_parties (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, role TEXT NOT NULL, organization_id INTEGER NOT NULL, mention_notice_id INTEGER NOT NULL, mention_section_id TEXT NOT NULL);
CREATE TABLE tender_version_texts (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, lang TEXT, value TEXT NOT NULL);
CREATE TABLE tender_version_dates (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, utc_seconds INTEGER NOT NULL, offset_minutes INTEGER NOT NULL, has_time INTEGER NOT NULL);
CREATE TABLE tender_version_amounts (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, cents INTEGER NOT NULL, currency TEXT NOT NULL);
CREATE TABLE tender_version_classifications (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, scheme TEXT NOT NULL, code TEXT NOT NULL);
CREATE TABLE notices (id INTEGER PRIMARY KEY, source TEXT NOT NULL, publication_id TEXT NOT NULL, content_hash TEXT NOT NULL, profile TEXT NOT NULL, declared_version TEXT, fetch_id INTEGER NOT NULL, member_path TEXT NOT NULL, ingested_at INTEGER NOT NULL, published_at INTEGER, dispatched_at INTEGER, parse_state TEXT);

-- Generated to 3x the tender count because `lots` needs ~3 rows per tender. An
-- earlier version capped this at __N__ and wrote `WHERE n <= __N__ * 3` for lots,
-- which is a NO-OP against an __N__-row source: it produced one lot per THREE
-- tenders, so two thirds of tenders had no lots at all and `?tender=` reads and
-- tender_detail were unrealistically cheap. The selectivity gate passed it —
-- non-emptiness and selectivity were both fine — which is why the gate now also
-- checks REFERENTIAL COVERAGE.
CREATE TEMP TABLE seq_n AS
  WITH RECURSIVE c(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM c WHERE n < __N__ * 3) SELECT n FROM c;

-- tenders. `kind`: registration ONLY in the last decile (the anchor property);
-- a mid-frequency `framework`; the rest `contract`. source ~90/10 ted/doe.
INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, island_notice_id)
SELECT n,
       CASE WHEN n % 10 = 0 THEN 'doe' ELSE 'ted' END,
       'pk-' || n,
       CASE WHEN n > (__N__ * 9) / 10 AND n % 7 = 0 THEN 'registration'
            WHEN n % 13 = 0                          THEN 'framework'
            ELSE 'contract' END,
       1, 1700000000 + n, NULL
FROM seq_n WHERE n <= __N__;

INSERT INTO tender_versions (tender_id, seq, published_at, dispatched_at, publication_id, notice_subtype, caused_by_notice_id)
SELECT id, 1, 1700000000 + id, NULL, 'pub-' || id, '16', id FROM tenders;

INSERT INTO notices (id, source, publication_id, content_hash, profile, declared_version, fetch_id, member_path, ingested_at, published_at, dispatched_at, parse_state)
SELECT id, source, 'pub-' || id, 'h' || id,
       CASE WHEN id % 5 = 0 THEN 'r209' WHEN id % 3 = 0 THEN 'eforms' ELSE 'r208' END,
       '1.0', 1, 'm/' || id || '.xml', 1700000000 + id, 1700000000 + id, NULL, 'parsed'
FROM tenders;

-- organizations: a long tail, plus a country/identifier_kind spread.
INSERT INTO organizations (id, name, country, identifier_kind)
SELECT n, 'Org ' || n,
       CASE WHEN n % 50 = 0 THEN 'FR' WHEN n % 7 = 0 THEN 'DE' ELSE 'IT' END,
       CASE WHEN n % 11 = 0 THEN 'VAT' ELSE 'NATIONAL' END
FROM seq_n WHERE n <= __N__ / 20;

INSERT INTO organization_mentions (notice_id, section_id, organization_id)
SELECT id, 'S1', 1 + (id % (__N__ / 20)) FROM tenders;

-- lots: ~3 per tender, kind skewed Lot / Part / LotsGroup.
INSERT INTO lots (id, tender_id, lot_key)
SELECT n, 1 + ((n - 1) / 3), 'LOT-' || n FROM seq_n WHERE n <= __N__ * 3;

INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind)
SELECT tender_id, 1, id,
       CASE WHEN id % 97 = 0 THEN 'LotsGroup' WHEN id % 11 = 0 THEN 'Part' ELSE 'Lot' END
FROM lots;

-- classifications: nuts + cpv, skewed so the top code is selective but not total,
-- and SCATTERED across the id range (issue 122 requirement 3) — a clustered
-- co-filter would let the walk stop early and hide the hazard.
INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
SELECT id, 1, NULL, 'main', 'nuts',
       CASE WHEN id % 7 = 0 THEN 'DE300' WHEN id % 5 = 0 THEN 'FR101' ELSE 'ITC4' || (id % 9) END
FROM tenders;
INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
SELECT id, 1, NULL, 'main', 'cpv',
       CASE WHEN id % 11 = 0 THEN '45000000' WHEN id % 3 = 0 THEN '72000000' ELSE '3' || (id % 7) || '000000' END
FROM tenders;

-- parties: a dominant buyer plus a long tail.
INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id)
SELECT id, 1, NULL, 'Buyer',
       CASE WHEN id % 23 = 0 THEN 1 ELSE 1 + (id % (__N__ / 20)) END,
       id, 'S1'
FROM tenders;

INSERT INTO lot_results (id, tender_id) SELECT id, id FROM tenders WHERE id % 2 = 0;
INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
SELECT id, 1, id, CASE WHEN id % 29 = 0 THEN 2 ELSE 1 + (id % (__N__ / 20)) END
FROM tenders WHERE id % 2 = 0;

INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
SELECT id, 1, NULL, 'title', CASE WHEN id % 4 = 0 THEN 'ENG' ELSE 'DEU' END, 'Title ' || id FROM tenders;

INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
SELECT id, 1, NULL, 'value', (id % 1000) * 100000 + 5000, 'EUR' FROM tenders;

INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
SELECT id, 1, NULL, 'submission_deadline', 1800000000 + (id % 100000) * 60, 120, 1 FROM tenders;

-- Indexes: the set a healthy prod carries (schema batch + the deferred lists in
-- canonical.rs). NOT a bare ANALYZE — see the header.
CREATE INDEX tender_versions_published ON tender_versions (published_at);
CREATE INDEX tender_versions_notice ON tender_versions (caused_by_notice_id);
CREATE INDEX tender_version_texts_version ON tender_version_texts (tender_id, seq);
CREATE INDEX tender_version_amounts_version ON tender_version_amounts (tender_id, seq);
CREATE INDEX tender_version_dates_version ON tender_version_dates (tender_id, seq);
CREATE INDEX tender_version_classifications_code ON tender_version_classifications (scheme, code);
CREATE INDEX tender_version_classifications_version ON tender_version_classifications (tender_id, seq);
CREATE INDEX tender_version_parties_org ON tender_version_parties (organization_id);
CREATE INDEX tender_version_parties_version ON tender_version_parties (tender_id, seq);
CREATE INDEX tender_version_result_winners_org ON tender_version_result_winners (organization_id);
CREATE INDEX organization_mentions_org ON organization_mentions (organization_id);
CREATE INDEX organizations_country_id ON organizations (country, id);
CREATE INDEX organizations_kind_id ON organizations (identifier_kind, id);
CREATE INDEX notices_source_id ON notices (source, id);
CREATE INDEX tenders_procedure_key ON tenders (procedure_key);
CREATE INDEX tenders_island ON tenders (source, island_notice_id);
CREATE INDEX tenders_current_published ON tenders (current_published_at, id);
