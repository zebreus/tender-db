//! The versioned canonical layer (ADR-0001) and the change cursor.
//!
//! The notice-parsed layer is the source of truth; everything here is a
//! deterministic function of it (`ingest::project`). Shape:
//!
//! - `tenders` — identity only. A TED eForms procedure is keyed by BT-04
//!   (`ContractFolderID`); a notice publishing none becomes a single-notice
//!   *island* Tender (CONTEXT.md), keyed by that notice.
//! - `tender_versions(tender_id, seq, caused_by_notice_id, …)` — one row per
//!   Notice of the Tender, in publication order. Current state is `MAX(seq)`,
//!   exposed as the `v_*` views; time travel filters on `published_at`.
//! - version-keyed satellites — lots, texts, amounts, classifications,
//!   parties. Each row is scoped either to the Tender (`lot_id IS NULL`) or to
//!   one of its Lots.
//! - `lots` — Lot identity within a Tender, and *only* the published lot id.
//!   Framework/DPS call-off rounds relabel lots per round, so two versions
//!   share a Lot exactly when they publish the same id; nothing is guessed
//!   across versions.
//! - `organizations` ← `organization_mentions` — a mention per (notice,
//!   Organization section) always; mentions collapse into one canonical
//!   profile only on an exact normalised official identifier.
//! - `changes` — the cursor spine, appended in the same transaction as the
//!   canonical write, never renumbered.
//!
//! Change scoping is diff-based (ADR-0001 amendment): `op` comes from
//! comparing consecutive version payloads, never from a notice's own
//! declaration of what it changed (BT-13716 covers only ~58% of real change
//! notices).

use crate::checkpoint::{CheckpointMode, checkpoint_on};
use crate::{Db, Parsed, Section, ValueRow, int, opt_int, opt_text, opt_text_of, t, text};
use std::collections::BTreeSet;
use turso::{Connection, Value};

pub(crate) const SCHEMA: &str = "
    -- A Tender: one procurement opportunity, independent of how many Notices
    -- documented it. `procedure_key` is the source's procedure identity (BT-04
    -- for TED eForms). Where a Notice publishes none, `island_notice_id` names
    -- that single notice instead — the island rule; such a Tender upgrades by
    -- re-projection if linkage ever appears. Exactly one of the two is set, and
    -- UNIQUE over a NULL column is vacuous in SQLite, which is what lets both
    -- kinds live in one table.
    CREATE TABLE IF NOT EXISTS tenders (
        id               INTEGER PRIMARY KEY AUTOINCREMENT,
        source           TEXT NOT NULL,
        procedure_key    TEXT,
        island_notice_id INTEGER REFERENCES notices(id),
        -- 'procedure' | 'registration' (BRIN notices are minimal Tenders of a
        -- distinct kind, CONTEXT.md).
        kind             TEXT NOT NULL,
        created_at       INTEGER NOT NULL,
        -- Denormalised pointer to the current (highest-seq) version, maintained by
        -- the projection (issue 25). MAX(seq) GROUP BY tender_id over the whole
        -- tender_versions table was O(all versions) on every cold list; the
        -- projection already knows the head when it writes a version, so it records
        -- it here. current_published_at is that version's publication date, so a
        -- newest-current-Tenders list is an index range scan, not a full sort.
        -- tender_versions stays fully append-only -- this is a derived head pointer
        -- (ADR-0001 allows validity-range writes on the canonical layer).
        current_seq          INTEGER,
        current_published_at INTEGER,
        -- A procedure key is globally unique across Sources: a TED eForms
        -- procedure and its DÖE twin share one BT-04 UUID and must collapse into
        -- one Tender (ADR-0003), and legacy `ojs:` keys are TED-only, so the key
        -- alone identifies the Tender. `source` is the primary Source label
        -- (TED where the procedure appears on both).
        UNIQUE(procedure_key),
        UNIQUE(island_notice_id)
    ) STRICT;
    -- The index that serves the newest-current-Tenders list (issue 25) is
    -- created in migrate(), not here: on a pre-issue-25 database the
    -- current_published_at column it covers is added by an ALTER that runs
    -- after this schema batch, so the CREATE INDEX must follow it.

    -- One version per Notice, ordered by publication. `seq` is dense from 1 and
    -- is recomputed when a late-arriving Notice belongs mid-chain — the change
    -- log, not the version numbering, is the stable spine.
    CREATE TABLE IF NOT EXISTS tender_versions (
        tender_id           INTEGER NOT NULL REFERENCES tenders(id),
        seq                 INTEGER NOT NULL,
        caused_by_notice_id INTEGER NOT NULL REFERENCES notices(id),
        published_at        INTEGER NOT NULL, -- unix seconds: the OJ/portal publication date
        dispatched_at       INTEGER,          -- unix seconds: when the notice was sent (issue 18)
        notice_subtype      TEXT,
        publication_id      TEXT NOT NULL,
        PRIMARY KEY (tender_id, seq),
        -- One version per Notice: re-projecting a package can never duplicate a
        -- version, which is what makes re-processing a no-op.
        UNIQUE (tender_id, caused_by_notice_id)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_versions_published ON tender_versions(published_at);
    CREATE INDEX IF NOT EXISTS tender_versions_notice ON tender_versions(caused_by_notice_id);

    -- Lot identity inside a Tender: the published id and nothing else.
    CREATE TABLE IF NOT EXISTS lots (
        id        INTEGER PRIMARY KEY AUTOINCREMENT,
        tender_id INTEGER NOT NULL REFERENCES tenders(id),
        lot_key   TEXT NOT NULL,
        UNIQUE(tender_id, lot_key)
    ) STRICT;

    -- Which Lots a version publishes, and as what: Parts are Lots with a kind
    -- flag (CONTEXT.md), LotsGroups likewise.
    CREATE TABLE IF NOT EXISTS tender_version_lots (
        tender_id INTEGER NOT NULL,
        seq       INTEGER NOT NULL,
        lot_id    INTEGER NOT NULL REFERENCES lots(id),
        kind      TEXT NOT NULL, -- Lot | LotsGroup | Part
        PRIMARY KEY (tender_id, seq, lot_id),
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;

    -- The version-keyed satellites. `lot_id IS NULL` means the value is the
    -- Tender's own; otherwise it belongs to that Lot. They carry no uniqueness
    -- constraint of their own: the projection writes each (tender, seq) exactly
    -- once, guarded by tender_versions' UNIQUE on the causing notice.
    CREATE TABLE IF NOT EXISTS tender_version_texts (
        tender_id INTEGER NOT NULL,
        seq       INTEGER NOT NULL,
        lot_id    INTEGER REFERENCES lots(id),
        field     TEXT NOT NULL, -- title | description
        lang      TEXT,          -- ISO 639-2, as published (EN + original)
        value     TEXT NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_texts_version ON tender_version_texts(tender_id, seq);

    CREATE TABLE IF NOT EXISTS tender_version_amounts (
        tender_id INTEGER NOT NULL,
        seq       INTEGER NOT NULL,
        lot_id    INTEGER REFERENCES lots(id),
        field     TEXT NOT NULL,
        cents     INTEGER NOT NULL,
        currency  TEXT NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_amounts_version ON tender_version_amounts(tender_id, seq);

    -- Deadlines and planned periods: UTC instant plus the buyer's own offset,
    -- because the local wall-clock deadline is the meaningful one. This is the
    -- satellite ADR-0001's motivating question reads (\"how did the deadline
    -- move?\") — a corrigendum typically changes nothing else.
    CREATE TABLE IF NOT EXISTS tender_version_dates (
        tender_id      INTEGER NOT NULL,
        seq            INTEGER NOT NULL,
        lot_id         INTEGER REFERENCES lots(id),
        field          TEXT NOT NULL,
        utc_seconds    INTEGER NOT NULL,
        offset_minutes INTEGER NOT NULL,
        has_time       INTEGER NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_dates_version ON tender_version_dates(tender_id, seq);

    CREATE TABLE IF NOT EXISTS tender_version_classifications (
        tender_id INTEGER NOT NULL,
        seq       INTEGER NOT NULL,
        lot_id    INTEGER REFERENCES lots(id),
        field     TEXT NOT NULL, -- main | additional | place
        scheme    TEXT NOT NULL, -- cpv | nuts
        code      TEXT NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_classifications_code
        ON tender_version_classifications(scheme, code);
    -- The by-version index its siblings (texts/amounts/dates) already carry: the
    -- /v1/tenders list echoes a row's CPV+NUTS codes with a correlated subquery
    -- per page row, keyed by (tender_id, seq) — without this it would seek the
    -- (scheme, code) index and scan half the table per row (issue 49 / the
    -- issue-25 scan pathology). Idempotent; builds once on first open after
    -- deploy on an existing table, like notices_fetch_id.
    CREATE INDEX IF NOT EXISTS tender_version_classifications_version
        ON tender_version_classifications(tender_id, seq);

    -- Who participates, in which role, in this version. The mention columns
    -- keep the row anchored to its evidence in the notice layer.
    CREATE TABLE IF NOT EXISTS tender_version_parties (
        tender_id          INTEGER NOT NULL,
        seq                INTEGER NOT NULL,
        lot_id             INTEGER REFERENCES lots(id),
        role               TEXT NOT NULL,
        organization_id    INTEGER NOT NULL REFERENCES organizations(id),
        mention_notice_id  INTEGER NOT NULL,
        mention_section_id TEXT NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq),
        FOREIGN KEY (mention_notice_id, mention_section_id)
            REFERENCES organization_mentions(notice_id, section_id)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_parties_org ON tender_version_parties(organization_id);
    -- The by-version index its siblings (texts/amounts/dates/classifications) all
    -- carry: parties was the lone satellite without one. It lets a (tender_id,
    -- seq) lookup — the analyst views' access pattern — seek rather than scan once
    -- the planner has row stats. Idempotent. (turso does not push a predicate
    -- through a view, so an unanalysed DB still materialises v_tender_buyers, like
    -- every v_* view; the index is what makes the seek reachable on real data.)
    CREATE INDEX IF NOT EXISTS tender_version_parties_version
        ON tender_version_parties(tender_id, seq);

    -- A canonical Organization profile. `identifier` is the normalised official
    -- id that merged its mentions; a profile without one is `provisional` — it
    -- represents exactly one mention and never absorbs another, because
    -- name-only matching would merge distinct companies (CONTEXT.md).
    -- The org-identity uniqueness is a NAMED index (`organizations_identity`),
    -- not an inline `UNIQUE` constraint, so a full-rebuild projection can DROP it,
    -- bulk-load organizations by sequential id, and rebuild it once at the end
    -- (issue 60): each new org otherwise did a random-position uniqueness *probe*
    -- into this index, which — once it outgrew the page cache at millions of orgs
    -- — became a random-seek storm. The in-RAM org_of map is the run's
    -- authoritative dedup, so the deferred rebuild never finds a conflict.
    --
    -- The index is built by [`Db::build_organization_indexes`] (the end of a
    -- rebuild), NOT here in the open-time schema batch: on an existing prod DB the
    -- organizations table is already populated (tens of millions, from a prior
    -- run's Phase-1) but lacks this named index, so a `CREATE UNIQUE INDEX` at
    -- open would build it over all of them at once — a pathological
    -- CREATE-INDEX-at-scale that HUNG prod startup. A fresh DB's table is empty, so
    -- the first rebuild builds it instantly.
    CREATE TABLE IF NOT EXISTS organizations (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        country         TEXT,
        identifier_kind TEXT, -- vat | national
        identifier      TEXT,
        name            TEXT NOT NULL,
        provisional     INTEGER NOT NULL,
        created_at      INTEGER NOT NULL
    ) STRICT;

    -- One appearance of an Organization in one Notice — the immutable evidence
    -- canonical profiles are built from (CONTEXT.md). (notice_id, section_id)
    -- is the issue-03 Organization section it came from.
    CREATE TABLE IF NOT EXISTS organization_mentions (
        notice_id       INTEGER NOT NULL REFERENCES notices(id),
        section_id      TEXT NOT NULL,
        organization_id INTEGER NOT NULL REFERENCES organizations(id),
        name            TEXT,
        country         TEXT,
        raw_identifier  TEXT,
        scheme          TEXT,
        PRIMARY KEY (notice_id, section_id),
        FOREIGN KEY (notice_id, section_id) REFERENCES notice_sections(notice_id, section_id)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS organization_mentions_org ON organization_mentions(organization_id);

    -- ------------------------------------------------------------- results
    -- The tendering-results half (issue 13): what happened after submission.
    -- eForms publishes results as notice-local sections (RES-/TEN-/CON-), and
    -- framework/DPS award rounds are repeated result notices under one Tender
    -- whose lot ids can be round-local labels (ted-empirical-checks.md §1/§3).
    -- A results entity is therefore identified by its origin notice plus its
    -- section id there: rounds accumulate, and two rounds can never collide.

    CREATE TABLE IF NOT EXISTS lot_results (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        tender_id  INTEGER NOT NULL REFERENCES tenders(id),
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        result_key TEXT NOT NULL, -- RES-XXXX in the origin notice
        UNIQUE(tender_id, notice_id, result_key)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS bids (
        id        INTEGER PRIMARY KEY AUTOINCREMENT,
        tender_id INTEGER NOT NULL REFERENCES tenders(id),
        notice_id INTEGER NOT NULL REFERENCES notices(id),
        bid_key   TEXT NOT NULL, -- TEN-XXXX (the eForms LotTender)
        UNIQUE(tender_id, notice_id, bid_key)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS contracts (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        tender_id    INTEGER NOT NULL REFERENCES tenders(id),
        notice_id    INTEGER NOT NULL REFERENCES notices(id),
        contract_key TEXT NOT NULL, -- CON-XXXX (the eForms SettledContract)
        UNIQUE(tender_id, notice_id, contract_key)
    ) STRICT;

    -- The award decision for one Lot as of this version (BT-142/BT-144). The
    -- awarded value and the winners are resolved through the origin notice's
    -- own result graph (LotResult → SettledContract → LotTender →
    -- TenderingParty → Organization).
    CREATE TABLE IF NOT EXISTS tender_version_lot_results (
        tender_id        INTEGER NOT NULL,
        seq              INTEGER NOT NULL,
        lot_result_id    INTEGER NOT NULL REFERENCES lot_results(id),
        lot_id           INTEGER REFERENCES lots(id),
        decision         TEXT, -- BT-142 winner-selection-status
        reason           TEXT, -- BT-144 non-award justification
        awarded_cents    INTEGER,
        awarded_currency TEXT,
        PRIMARY KEY (tender_id, seq, lot_result_id),
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS tender_version_result_winners (
        tender_id       INTEGER NOT NULL,
        seq             INTEGER NOT NULL,
        lot_result_id   INTEGER NOT NULL REFERENCES lot_results(id),
        organization_id INTEGER NOT NULL REFERENCES organizations(id),
        PRIMARY KEY (tender_id, seq, lot_result_id, organization_id),
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_result_winners_org
        ON tender_version_result_winners(organization_id);

    -- Received-submission statistics (BT-759/BT-760): `kind` is the published
    -- received-submission-type code (tenders, t-sme, t-eea, …).
    CREATE TABLE IF NOT EXISTS tender_version_result_stats (
        tender_id     INTEGER NOT NULL,
        seq           INTEGER NOT NULL,
        lot_result_id INTEGER NOT NULL REFERENCES lot_results(id),
        kind          TEXT NOT NULL,
        count         INTEGER NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_result_stats_version
        ON tender_version_result_stats(tender_id, seq);

    -- A Bid: one offer on one Lot, with its value (BT-720). CONTEXT.md: the
    -- eForms TEN- entity is a Bid — the word tender never means an offer here.
    CREATE TABLE IF NOT EXISTS tender_version_bids (
        tender_id INTEGER NOT NULL,
        seq       INTEGER NOT NULL,
        bid_id    INTEGER NOT NULL REFERENCES bids(id),
        lot_id    INTEGER REFERENCES lots(id),
        cents     INTEGER,
        currency  TEXT,
        PRIMARY KEY (tender_id, seq, bid_id),
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;

    -- The eForms TenderingParty flattened onto the Bid: who stands behind the
    -- offer (role 'tenderer') and who they subcontract to ('subcontractor'),
    -- each link anchored to its mention evidence.
    CREATE TABLE IF NOT EXISTS tender_version_bid_parties (
        tender_id          INTEGER NOT NULL,
        seq                INTEGER NOT NULL,
        bid_id             INTEGER NOT NULL REFERENCES bids(id),
        role               TEXT NOT NULL, -- tenderer | subcontractor
        organization_id    INTEGER NOT NULL REFERENCES organizations(id),
        mention_notice_id  INTEGER NOT NULL,
        mention_section_id TEXT NOT NULL,
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq),
        FOREIGN KEY (mention_notice_id, mention_section_id)
            REFERENCES organization_mentions(notice_id, section_id)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS tender_version_bid_parties_org
        ON tender_version_bid_parties(organization_id);

    -- A settled Contract: the buyer's contract id (BT-150), the conclusion
    -- date (BT-145), and the value of the winning Bid(s) it settled
    -- (BT-3202 → BT-720 — eForms contracts carry no value of their own).
    CREATE TABLE IF NOT EXISTS tender_version_contracts (
        tender_id          INTEGER NOT NULL,
        seq                INTEGER NOT NULL,
        contract_id        INTEGER NOT NULL REFERENCES contracts(id),
        buyer_contract_id  TEXT,
        concluded_utc      INTEGER,
        concluded_offset   INTEGER,
        concluded_has_time INTEGER,
        cents              INTEGER,
        currency           TEXT,
        PRIMARY KEY (tender_id, seq, contract_id),
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;

    -- The change cursor (docs/architecture.md). Ingestion order, never
    -- renumbered: re-projections append. `op` is diff-derived.
    CREATE TABLE IF NOT EXISTS changes (
        cursor      INTEGER PRIMARY KEY AUTOINCREMENT,
        entity_kind TEXT NOT NULL, -- tender | lot | organization | lot_result | bid | contract
        entity_id   INTEGER NOT NULL,
        version_seq INTEGER,
        op          TEXT NOT NULL, -- added | changed | removed
        changed_at  INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX IF NOT EXISTS changes_entity ON changes(entity_kind, entity_id);

    -- ---------------------------------------------------------------- views
    -- Current state = the highest seq per Tender.

    -- The current-version pointer, read from the maintained head column instead
    -- of `MAX(seq) GROUP BY tender_id` over every version (issue 25). Same
    -- `(tender_id, seq)` shape, so v_lots/v_lot_results and the public `/v1/sql`
    -- queries that join it are unchanged — just O(tenders), not O(all versions).
    DROP VIEW IF EXISTS v_tender_current;
    CREATE VIEW v_tender_current AS
    SELECT id AS tender_id, current_seq AS seq FROM tenders WHERE current_seq IS NOT NULL;

    DROP VIEW IF EXISTS v_tenders;
    CREATE VIEW v_tenders AS
    SELECT t.id, t.source, t.procedure_key, t.kind,
           v.seq, v.published_at, v.caused_by_notice_id, v.notice_subtype, v.publication_id,
           (SELECT x.value FROM tender_version_texts x
             WHERE x.tender_id = t.id AND x.seq = v.seq AND x.field = 'title'
             -- The Tender's own title wins; a lot-only title stands in for the
             -- many notices that title their lots and not the procedure.
             ORDER BY (x.lot_id IS NULL) DESC, (x.lang = 'ENG') DESC
             LIMIT 1) AS title
      FROM tenders t
      JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq;

    DROP VIEW IF EXISTS v_lots;
    CREATE VIEW v_lots AS
    SELECT l.id, l.tender_id, l.lot_key, vl.kind, vl.seq,
           (SELECT x.value FROM tender_version_texts x
             WHERE x.tender_id = l.tender_id AND x.seq = vl.seq
               AND x.lot_id = l.id AND x.field = 'title'
             ORDER BY (x.lang = 'ENG') DESC LIMIT 1) AS title
      FROM lots l
      JOIN v_tender_current c ON c.tender_id = l.tender_id
      JOIN tender_version_lots vl
        ON vl.tender_id = l.tender_id AND vl.seq = c.seq AND vl.lot_id = l.id;

    DROP VIEW IF EXISTS v_organizations;
    CREATE VIEW v_organizations AS
    SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
           (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id) AS mentions
      FROM organizations o;

    -- Current lot results with their winner — one row per (result, winning
    -- organization); a consortium yields one row per member, an unresolved or
    -- withheld winner yields NULL columns. The spec's competitor question
    -- (org → won lots → values) is a GROUP BY over this view.
    DROP VIEW IF EXISTS v_lot_results;
    CREATE VIEW v_lot_results AS
    SELECT r.id, r.tender_id, r.notice_id, r.result_key,
           s.lot_id, (SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id) AS lot_key,
           s.decision, s.reason, s.awarded_cents, s.awarded_currency,
           w.organization_id AS winner_organization_id,
           o.name AS winner_name, o.provisional AS winner_provisional
      FROM lot_results r
      JOIN v_tender_current c ON c.tender_id = r.tender_id
      JOIN tender_version_lot_results s
        ON s.tender_id = r.tender_id AND s.seq = c.seq AND s.lot_result_id = r.id
      LEFT JOIN tender_version_result_winners w
        ON w.tender_id = r.tender_id AND w.seq = c.seq AND w.lot_result_id = r.id
      LEFT JOIN organizations o ON o.id = w.organization_id;

    -- ----------------------------------------------------- analyst views
    -- Convenience views for the common questions (issue 50), so an analyst does
    -- not reverse-engineer the version satellites and magic role strings. Every
    -- one drives off the v_tender_current pointer (or a PK/`_version` index), so
    -- it stays O(matched) — never a fresh MAX(seq) or full scan (issue 25).

    -- Buyers of each current Tender. Buyer roles are era-dependent ('buyer' or
    -- 'Procedure-Buyer'), matched with LIKE; one row per buyer party. Joins
    -- tenders directly on current_seq (like v_tenders) so a per-tender lookup
    -- seeks parties by the version index instead of scanning (issue 25).
    DROP VIEW IF EXISTS v_tender_buyers;
    CREATE VIEW v_tender_buyers AS
    SELECT t.id AS tender_id,
           p.organization_id AS buyer_organization_id,
           o.name            AS buyer_name,
           o.country         AS buyer_country,
           o.provisional     AS buyer_provisional
      FROM tenders t
      JOIN tender_version_parties p
        ON p.tender_id = t.id AND p.seq = t.current_seq AND p.role LIKE '%uyer%'
      JOIN organizations o ON o.id = p.organization_id
     WHERE t.current_seq IS NOT NULL;

    -- Current award decisions with their winner (from v_lot_results) plus a
    -- representative buyer — a scalar lookup, so the view keeps v_lot_results'
    -- one-row-per-winner grain and does not multiply awarded_cents by buyer count.
    DROP VIEW IF EXISTS v_awards;
    CREATE VIEW v_awards AS
    SELECT r.tender_id, r.notice_id, r.result_key, r.lot_id, r.lot_key,
           r.decision, r.reason, r.awarded_cents, r.awarded_currency,
           r.winner_organization_id, r.winner_name,
           (SELECT b.buyer_organization_id FROM v_tender_buyers b
             WHERE b.tender_id = r.tender_id LIMIT 1) AS buyer_organization_id,
           (SELECT b.buyer_name FROM v_tender_buyers b
             WHERE b.tender_id = r.tender_id LIMIT 1) AS buyer_name
      FROM v_lot_results r;

    -- CPV and NUTS codes of each current Tender (scheme in ('cpv','nuts')). Joins
    -- tenders on current_seq directly so a per-tender lookup seeks the satellite's
    -- `_version` index (issue 25); same shape for amounts and dates below.
    DROP VIEW IF EXISTS v_tender_classifications;
    CREATE VIEW v_tender_classifications AS
    SELECT t.id AS tender_id, x.lot_id, x.field, x.scheme, x.code
      FROM tenders t
      JOIN tender_version_classifications x ON x.tender_id = t.id AND x.seq = t.current_seq
     WHERE t.current_seq IS NOT NULL;

    -- Money amounts of each current Tender (field names the amount; cents+currency).
    DROP VIEW IF EXISTS v_tender_amounts;
    CREATE VIEW v_tender_amounts AS
    SELECT t.id AS tender_id, a.lot_id, a.field, a.cents, a.currency
      FROM tenders t
      JOIN tender_version_amounts a ON a.tender_id = t.id AND a.seq = t.current_seq
     WHERE t.current_seq IS NOT NULL;

    -- Dates of each current Tender: utc_seconds is epoch seconds in the buyer's
    -- own offset_minutes; has_time = 0 means the source gave a date only.
    DROP VIEW IF EXISTS v_tender_dates;
    CREATE VIEW v_tender_dates AS
    SELECT t.id AS tender_id, d.lot_id, d.field, d.utc_seconds, d.offset_minutes, d.has_time
      FROM tenders t
      JOIN tender_version_dates d ON d.tender_id = t.id AND d.seq = t.current_seq
     WHERE t.current_seq IS NOT NULL;

    -- The Notices that caused each Tender version — the ADR-0001 chain as a join,
    -- across all versions (not just current), so the whole history is reachable.
    DROP VIEW IF EXISTS v_tender_notices;
    CREATE VIEW v_tender_notices AS
    SELECT v.tender_id, v.seq, v.caused_by_notice_id AS notice_id,
           v.notice_subtype, v.published_at,
           n.source, n.publication_id, n.profile, n.parse_state
      FROM tender_versions v
      JOIN notices n ON n.id = v.caused_by_notice_id;

    -- Path-free fetch provenance (issue 45): which source package/period a notice
    -- came from, without the raw-fetch registry's server filesystem `path`.
    DROP VIEW IF EXISTS v_fetches;
    CREATE VIEW v_fetches AS
    SELECT id, source, kind, period, url, sha256, bytes, fetched_at
      FROM fetches;
";

/// How many Tenders (or Organization mentions) a single projection write
/// transaction covers (issue 19). Batching amortises per-transaction overhead —
/// the projection bottleneck — while keeping each batch small enough that its
/// change rows stay atomic with their canonical writes and a failure rolls back
/// only a bounded slice. A batch is one BEGIN IMMEDIATE … COMMIT.
const WRITE_BATCH: usize = 512;

/// Checkpoint the WAL every this many committed batches during a projection
/// (issue 42). 32 batches ≈ 16k tenders between folds — frequent enough to keep
/// the WAL bounded through a full-backfill projection, rare enough that the
/// checkpoint cost is noise against the writes it follows.
const CHECKPOINT_EVERY_BATCHES: usize = 32;

/// Legacy `plan_ojs_node` rows written per transaction when materialising the
/// union-find result (path-B). Chunked so the sorted bulk load keeps the WAL
/// bounded, like the plan bulk load (issue 60).
const NODE_WRITE_BATCH: usize = 20_000;

/// An in-memory union-find over legacy OJS keys where each component's root is its
/// MINIMUM key (union-TO-MIN), so `find(k)` returns the component's earliest OJS
/// number — exactly the representative the former SQL label-propagation converged
/// to (the MIN label). Replaces that O(diameter) full-table-rewrite loop with one
/// near-linear in-memory pass (path-B, issue 60). Bounded by the count of distinct
/// legacy OJS numbers, not the corpus.
#[derive(Default)]
struct MinUnionFind {
    parent: std::collections::HashMap<i64, i64>,
}

impl MinUnionFind {
    /// Register `k` as a (possibly singleton) node.
    fn add(&mut self, k: i64) {
        self.parent.entry(k).or_insert(k);
    }

    /// The component representative of `k` — its minimum key — with path
    /// compression. An unregistered key is its own representative.
    fn find(&mut self, k: i64) -> i64 {
        let mut root = k;
        while let Some(&p) = self.parent.get(&root) {
            if p == root {
                break;
            }
            root = p;
        }
        let mut cur = k;
        while let Some(&p) = self.parent.get(&cur).filter(|&&p| p != cur) {
            self.parent.insert(cur, root);
            cur = p;
        }
        root
    }

    /// Merge the components of `a` and `b`; the smaller key becomes the root, so
    /// the representative is always the component MIN.
    fn union(&mut self, a: i64, b: i64) {
        self.add(a);
        self.add(b);
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (root, child) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent.insert(child, root);
        }
    }
}

/// One canonical value of a Tender version, in its scope. The satellites of
/// docs/architecture.md, as one comparable type — diffing versions is set
/// comparison over these.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fact {
    Text { field: String, lang: Option<String>, value: String },
    Amount { field: String, cents: i64, currency: String },
    Classification { field: String, scheme: String, code: String },
    Date { field: String, utc_seconds: i64, offset_minutes: i64, has_time: bool },
    Party { role: String, organization_id: i64, notice_id: i64, section_id: String },
}

impl Fact {
    /// What "a later notice supersedes this field" replaces as a unit. All
    /// language variants of a title go together, as do all CPV codes of one
    /// classification role: a notice that republishes one republishes the set.
    pub fn key(&self) -> (&'static str, &str) {
        match self {
            Fact::Text { field, .. } => ("text", field),
            Fact::Amount { field, .. } => ("amount", field),
            Fact::Classification { field, .. } => ("classification", field),
            Fact::Date { field, .. } => ("date", field),
            Fact::Party { role, .. } => ("party", role),
        }
    }
}

/// A Lot as one version publishes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LotState {
    pub key: String,
    pub kind: String,
    pub facts: BTreeSet<Fact>,
}

/// One result notice's contribution to the results layer — a "round" in the
/// framework/DPS sense (ted-empirical-checks.md §3). Rounds are *additive*:
/// a later version never supersedes an earlier round's results (the verified
/// tranche pattern), except that a correction — a change notice republishing
/// the same logical notice — replaces the round it corrects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    /// The result notice this round came from; results keys are local to it.
    pub notice_id: i64,
    /// BT-701 of the origin notice — the correction-replacement key.
    pub logical_notice_id: Option<String>,
    pub lot_results: Vec<LotResultState>,
    pub bids: Vec<BidState>,
    pub contracts: Vec<ContractState>,
}

/// The award decision for one Lot (eForms LotResult, RES-).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LotResultState {
    pub key: String,
    /// The referenced lot id as published (BT-13713) — resolved defensively,
    /// because FA/DPS rounds relabel lots per round.
    pub lot_key: Option<String>,
    pub decision: Option<String>,
    pub reason: Option<String>,
    pub awarded_cents: Option<i64>,
    pub awarded_currency: Option<String>,
    /// Winning Organization ids, resolved through the notice's own graph.
    pub winners: Vec<i64>,
    /// (received-submission-type code, count), from BT-759/BT-760.
    pub statistics: Vec<(String, i64)>,
}

/// A Bid (eForms LotTender, TEN-): one offer on one Lot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BidState {
    pub key: String,
    pub lot_key: Option<String>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
    pub parties: Vec<BidParty>,
}

/// One Organization behind a Bid — the eForms TenderingParty flattened.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BidParty {
    pub role: String, // tenderer | subcontractor
    pub organization_id: i64,
    /// The ORG- section in the origin notice — the mention evidence.
    pub section_id: String,
}

/// A settled Contract (eForms SettledContract, CON-).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractState {
    pub key: String,
    pub buyer_contract_id: Option<String>,
    /// BT-145 conclusion date: (utc seconds, offset minutes, has_time).
    pub concluded: Option<(i64, i64, bool)>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
}

/// One Tender version: the Notice that caused it plus the resolved state at
/// that point (this notice's values over the previous version's, and every
/// results round published so far).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenderVersion {
    pub caused_by_notice_id: i64,
    pub published_at: i64,
    /// When the notice was dispatched, where the era records it (issue 18).
    pub dispatched_at: Option<i64>,
    pub notice_subtype: Option<String>,
    pub publication_id: String,
    pub facts: BTreeSet<Fact>,
    pub lots: Vec<LotState>,
    pub rounds: Vec<Round>,
}

/// A whole Tender as the projection computed it, ready to reconcile against
/// what is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenderProjection {
    pub source: String,
    pub procedure_key: Option<String>,
    pub island_notice_id: Option<i64>,
    pub kind: String,
    pub versions: Vec<TenderVersion>,
}

/// An Organization mention as the projection read it out of a notice, before
/// any merging decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub notice_id: i64,
    pub section_id: String,
    pub name: String,
    pub country: Option<String>,
    pub raw_identifier: Option<String>,
    pub scheme: Option<String>,
    /// The normalised official identifier, if it passed the plausibility gate.
    /// `None` ⇒ a provisional profile of this mention alone.
    pub identifier: Option<Identifier>,
}

/// A normalised official identifier that is allowed to merge mentions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier {
    pub country: Option<String>,
    pub kind: String,
    pub value: String,
}

/// What applying a projection did — the numbers the CLI reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub tenders_created: u64,
    pub versions_written: u64,
    pub versions_removed: u64,
    pub changes: u64,
}

impl Applied {
    /// Accumulate another batch's tally — the streaming projection applies in
    /// bounded batches and sums their reports (issue 57).
    pub fn add(&mut self, other: Applied) {
        self.tenders_created += other.tenders_created;
        self.versions_written += other.versions_written;
        self.versions_removed += other.versions_removed;
        self.changes += other.changes;
    }
}

/// A parsed Notice, as the projection addresses it. `profile` selects the era
/// vocabulary — eForms notices key on BT-04; legacy TED profiles chain by
/// transitive OJS-number closure instead (docs/research/ted-legacy-mapping.md).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeRef {
    pub id: i64,
    pub source: String,
    pub publication_id: String,
    pub profile: String,
}

/// Streaming Organization dedup state, held across a projection's mention
/// batches so the same official identifier resolves to one canonical
/// Organization wherever in the corpus it appears — without holding either the
/// whole corpus's mentions or the whole `(notice, section)` idempotency map in
/// RAM at once (issue 57). Open with [`Db::mention_resolver`], drive with
/// [`Db::resolve_mentions`], close with [`Db::finish_mention_resolver`].
pub struct MentionResolver {
    org_of: std::collections::HashMap<(Option<String>, String, String), i64>,
    created_any: bool,
}

/// One notice's grouping identity, as written to the on-disk plan (issue 59).
/// `ojs_self`/`ojs_edges` are the OJS keys **encoded** `year*1e9 + number`, so a
/// SQL `MIN` over a component gives its earliest publication.
pub struct PlanRow {
    pub notice_id: i64,
    pub procedure_key: Option<String>,
    pub legacy: bool,
    pub ojs_self: Option<i64>,
    pub source: String,
    pub source_rank: i64,
    pub publication_id: String,
    pub published_at: i64,
    pub subtype: Option<String>,
    pub ojs_edges: Vec<i64>,
}

/// One Tender's notices, streamed from the plan in fold order (issue 59). Carries
/// only what folding needs beyond the parsed layer: the notice ids in order, each
/// one's Source (for the ADR-0003 primary-Source rule), and the fold-first
/// notice's subtype (for the Tender kind).
pub struct PlanGroup {
    pub group_key: String,
    pub notice_ids: Vec<i64>,
    pub sources: Vec<String>,
    pub first_subtype: Option<String>,
}

impl Db {
    /// The next chunk of parsed notices with `id > after_id` (up to `limit`),
    /// each with its full [`Parsed`] form, read in a fixed handful of scans over
    /// the chunk's id window rather than the ~9 queries **per notice** the
    /// original per-notice read path cost — the projection's batched input
    /// (issue 19). Empty when no more parsed notices follow `after_id`.
    ///
    /// Chunking (rather than one all-notices read) is what bounds memory: the
    /// projection keeps only the compact per-notice *states* for the whole
    /// period in RAM, never the whole raw notice layer at once. The query count
    /// is O(tables) per chunk, which is what removes the read storm.
    pub async fn parsed_chunk(&self, after_id: i64, limit: i64) -> turso::Result<Vec<(NoticeRef, Parsed)>> {
        let conn = self.reader().await?;
        let mut out: Vec<(NoticeRef, Parsed)> = Vec::new();
        let mut slot: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

        let mut rows = conn
            .query(
                "SELECT id, source, publication_id, profile FROM notices
                 WHERE parse_state = 'parsed' AND id > ? ORDER BY id LIMIT ?",
                (Value::Integer(after_id), Value::Integer(limit)),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            let id = int(&row, 0);
            slot.insert(id, out.len());
            out.push((
                NoticeRef {
                    id,
                    source: text(&row, 1),
                    publication_id: text(&row, 2),
                    profile: text(&row, 3),
                },
                Parsed::default(),
            ));
        }
        drop(rows);
        if out.is_empty() {
            return Ok(out);
        }
        // The chunk's contiguous id window: parsed ids in (after_id, hi] are
        // exactly this chunk (ORDER BY id LIMIT), so a ranged scan of each value
        // table over [lo, hi] yields precisely their rows.
        let lo = out.first().expect("non-empty").0.id;
        let hi = out.last().expect("non-empty").0.id;
        let window = (Value::Integer(lo), Value::Integer(hi));

        let mut rows = conn
            .query(
                "SELECT notice_id, section_id, kind, parent_section_id FROM notice_sections
                 WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
                window.clone(),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            if let Some(&i) = slot.get(&int(&row, 0)) {
                out[i].1.sections.push(Section {
                    id: text(&row, 1),
                    kind: text(&row, 2),
                    parent: opt_text_of(&row, 3),
                });
            }
        }
        drop(rows);

        for (table, cols, build) in value_sources() {
            let sql = format!(
                "SELECT {cols} FROM {table} WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id"
            );
            let mut rows = conn.query(&sql, window.clone()).await?;
            while let Some(row) = rows.next().await? {
                if let Some(&i) = slot.get(&int(&row, 0)) {
                    out[i].1.values.push(ValueRow {
                        section_id: text(&row, 1),
                        field_id: text(&row, 2),
                        ordinal: int(&row, 3),
                        value: build(&row),
                    });
                }
            }
            drop(rows);
        }
        Ok(out)
    }

    /// Read the full [`Parsed`] form of an explicit set of notice ids — the
    /// projection's Phase 2 read, where a bounded batch of whole Tenders (their
    /// notices scattered across the id space by publication history) is folded at
    /// once. Unlike [`Db::parsed_chunk`]'s contiguous id window, the ids here are
    /// arbitrary, so each satellite is read with an `IN (…)` over the batch —
    /// index seeks, chunked under the bind-variable ceiling. The returned notices
    /// are ordered by id; a caller that needs another order re-orders by id.
    pub async fn parsed_by_ids(&self, ids: &[i64]) -> turso::Result<Vec<(NoticeRef, Parsed)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.reader().await?;
        let mut out: Vec<(NoticeRef, Parsed)> = Vec::new();
        let mut slot: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

        for chunk in ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT id, source, publication_id, profile FROM notices WHERE id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                let id = int(&row, 0);
                slot.insert(id, out.len());
                out.push((
                    NoticeRef {
                        id,
                        source: text(&row, 1),
                        publication_id: text(&row, 2),
                        profile: text(&row, 3),
                    },
                    Parsed::default(),
                ));
            }
            drop(rows);
        }
        if out.is_empty() {
            return Ok(out);
        }
        // Keep the id order the caller-facing contract promises, independent of
        // the order the `IN` scans returned rows in.
        out.sort_by_key(|(n, _)| n.id);
        for (i, (n, _)) in out.iter().enumerate() {
            slot.insert(n.id, i);
        }

        for chunk in ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT notice_id, section_id, kind, parent_section_id FROM notice_sections
                 WHERE notice_id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                if let Some(&i) = slot.get(&int(&row, 0)) {
                    out[i].1.sections.push(Section {
                        id: text(&row, 1),
                        kind: text(&row, 2),
                        parent: opt_text_of(&row, 3),
                    });
                }
            }
            drop(rows);
        }

        for (table, cols, build) in value_sources() {
            for chunk in ids.chunks(IN_CHUNK) {
                let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
                let sql = format!(
                    "SELECT {cols} FROM {table} WHERE notice_id IN ({})",
                    placeholders(chunk.len())
                );
                let mut rows = conn.query(&sql, params).await?;
                while let Some(row) = rows.next().await? {
                    if let Some(&i) = slot.get(&int(&row, 0)) {
                        out[i].1.values.push(ValueRow {
                            section_id: text(&row, 1),
                            field_id: text(&row, 2),
                            ordinal: int(&row, 3),
                            value: build(&row),
                        });
                    }
                }
                drop(rows);
            }
        }
        Ok(out)
    }

    /// The resolved canonical Organization of every mention of the given notices,
    /// as `notice_id → (section_id → organization_id)`. Phase 2 uses it to bind a
    /// rebuilt notice's roles and results back to the Organizations that Phase 1
    /// already resolved and recorded, without re-resolving.
    pub async fn mentions_by_ids(
        &self,
        ids: &[i64],
    ) -> turso::Result<std::collections::HashMap<i64, std::collections::HashMap<String, i64>>> {
        use std::collections::HashMap;
        let mut out: HashMap<i64, HashMap<String, i64>> = HashMap::new();
        if ids.is_empty() {
            return Ok(out);
        }
        let conn = self.reader().await?;
        for chunk in ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                out.entry(int(&row, 0)).or_default().insert(text(&row, 1), int(&row, 2));
            }
            drop(rows);
        }
        Ok(out)
    }

    /// Wipe the canonical layer's *content*, leaving the change log intact —
    /// what `project --rebuild` runs before re-deriving everything. The cursor
    /// is never renumbered (docs/architecture.md), so the rebuild appends.
    pub async fn clear_canonical(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        for table in [
            "tender_version_result_winners",
            "tender_version_result_stats",
            "tender_version_lot_results",
            "tender_version_bid_parties",
            "tender_version_bids",
            "tender_version_contracts",
            "tender_version_parties",
            "tender_version_texts",
            "tender_version_dates",
            "tender_version_amounts",
            "tender_version_classifications",
            "tender_version_lots",
            "tender_versions",
            "lot_results",
            "bids",
            "contracts",
            "lots",
            "tenders",
            "organization_mentions",
            "organizations",
        ] {
            conn.execute(&format!("DELETE FROM {table}"), ()).await?;
        }
        // Reset the incremental-projection watermark (issue 58): a full rebuild
        // re-derives everything, so every parsed notice must be re-folded.
        conn.execute("UPDATE notices SET projected = 0 WHERE projected <> 0", ()).await?;
        Ok(())
    }

    /// The incremental-projection change-set (issue 58): parsed notices not yet
    /// folded into the canonical layer since their last (re)parse. Bounded by the
    /// daily delta via the `notices_unprojected` partial index, not the corpus.
    pub async fn unprojected_parsed_notice_ids(&self) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut out = Vec::new();
        let mut rows = conn
            .query("SELECT id FROM notices WHERE parse_state = 'parsed' AND projected = 0 ORDER BY id", ())
            .await?;
        while let Some(row) = rows.next().await? {
            out.push(int(&row, 0));
        }
        Ok(out)
    }

    /// Build the `notices_unprojected` partial index if absent (issue 58) — called
    /// at the END of a projection, when nearly every parsed notice is projected=1,
    /// so the partial index (over `projected = 0` rows only) is near-empty and
    /// builds instantly. Deferred to here rather than the schema/migration so a
    /// large existing DB — where a freshly-added `projected` column leaves ALL rows
    /// 0 until Phase-2 runs — never indexes the whole corpus at open.
    pub async fn ensure_unprojected_index(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS notices_unprojected ON notices(id)
                 WHERE parse_state = 'parsed' AND projected = 0",
            (),
        )
        .await?;
        Ok(())
    }

    /// Mark notices as folded into the canonical layer (issue 58). Called by
    /// Phase 2 for every notice in an applied batch — whether or not its Tender's
    /// content changed — so the next incremental run's change-set excludes them.
    pub async fn mark_projected(&self, ids: &[i64]) -> turso::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn().await;
        for chunk in ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "UPDATE notices SET projected = 1 WHERE id IN ({})",
                placeholders(chunk.len())
            );
            conn.execute(&sql, params).await?;
        }
        Ok(())
    }

    /// Drop and recreate the Organization tables as BARE tables — no uniqueness
    /// or org-id index — for a full-rebuild bulk load (issue 60). Every org and
    /// mention insert is then a sequential PK append instead of a random-position
    /// index probe; the in-RAM `org_of` dedup map is the run's authority, so no
    /// duplicate identifiers are ever emitted, and the indexes are rebuilt once by
    /// [`Db::build_organization_indexes`] at the end. Runs with FK off (the
    /// projection disables it), and drops the whole table so it works whether the
    /// db still has the old inline-`UNIQUE` auto-index or the new named index.
    pub async fn strip_organization_indexes(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("DROP TABLE IF EXISTS organization_mentions", ()).await?;
        conn.execute("DROP TABLE IF EXISTS organizations", ()).await?;
        conn.execute(
            "CREATE TABLE organizations (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, country TEXT, identifier_kind TEXT,
                 identifier TEXT, name TEXT NOT NULL, provisional INTEGER NOT NULL,
                 created_at INTEGER NOT NULL
             ) STRICT",
            (),
        )
        .await?;
        conn.execute(
            "CREATE TABLE organization_mentions (
                 notice_id INTEGER NOT NULL REFERENCES notices(id), section_id TEXT NOT NULL,
                 organization_id INTEGER NOT NULL REFERENCES organizations(id), name TEXT,
                 country TEXT, raw_identifier TEXT, scheme TEXT,
                 PRIMARY KEY (notice_id, section_id),
                 FOREIGN KEY (notice_id, section_id) REFERENCES notice_sections(notice_id, section_id)
             ) STRICT",
            (),
        )
        .await?;
        Ok(())
    }

    /// Rebuild the Organization indexes after a full-rebuild bulk load — one
    /// sorted build each, instead of the millions of random-position inserts they
    /// replace (issue 60). The `org_of` map guaranteed no duplicate identifiers,
    /// so the unique index builds without conflict.
    ///
    /// Skips `organizations_identity` when the table already carries an inline
    /// `UNIQUE(country, identifier_kind, identifier)` — a pre-issue-62 (existing
    /// prod) table does. Building a redundant named unique index over its millions
    /// of existing orgs is a pathological CREATE-INDEX-at-scale (it HUNG prod
    /// startup) AND pointless, since the inline constraint already enforces the
    /// uniqueness. On a resume the org tables were never stripped, so this is the
    /// path that runs there; a fresh issue-62 bare table has no inline UNIQUE and
    /// gets the named index built (cheaply, on an empty/small table).
    pub async fn build_organization_indexes(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        let inline_unique = {
            let mut rows = conn
                .query("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'organizations'", ())
                .await?;
            match rows.next().await? {
                Some(row) => text(&row, 0).to_uppercase().contains("UNIQUE"),
                None => false,
            }
        };
        if !inline_unique {
            conn.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS organizations_identity
                     ON organizations(country, identifier_kind, identifier)",
                (),
            )
            .await?;
        }
        conn.execute(
            "CREATE INDEX IF NOT EXISTS organization_mentions_org
                 ON organization_mentions(organization_id)",
            (),
        )
        .await?;
        Ok(())
    }

    // --------------------------------------------------------- grouping plan
    //
    // The projection's grouping plan lives on disk in scratch tables, not in a
    // RAM `Vec` (issue 59): at 7.5M+ notices an in-RAM plan is ~1.5 GB and grows
    // with the corpus. Here the whole grouping — keyed chains, the legacy OJS
    // transitive-closure union-find, islands — runs in SQL over these tables, and
    // Phase 2 streams whole-Tender batches out of them, so projection peak RAM is
    // one batch's working set regardless of corpus size. The tables are real
    // (not TEMP — turso's temp store is unconfigured) and cleared at both ends of
    // a run so nothing transient persists into a snapshot.

    /// (Re)create the empty grouping-plan scratch tables — dropped-clean at the
    /// start of a projection.
    pub async fn reset_plan(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS plan_notice (
                 notice_id      INTEGER PRIMARY KEY,
                 procedure_key  TEXT,
                 legacy         INTEGER NOT NULL,
                 ojs_self       INTEGER,
                 source         TEXT NOT NULL,
                 source_rank    INTEGER NOT NULL,
                 publication_id TEXT NOT NULL,
                 published_at   INTEGER NOT NULL,
                 subtype        TEXT,
                 group_key      TEXT
             ) STRICT",
            (),
        )
        .await?;
        // The legacy OJS graph: one node per OJS number (including not-yet-ingested
        // edge targets, so identity is stable as backfill deepens), symmetric edges.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS plan_ojs_node (key INTEGER PRIMARY KEY, label INTEGER NOT NULL) STRICT",
            (),
        )
        .await?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS plan_ojs_edge (a INTEGER NOT NULL, b INTEGER NOT NULL) STRICT",
            (),
        )
        .await?;
        self.clear_plan_on(&conn).await
    }

    /// Empty the grouping-plan scratch tables (and drop the Phase-2 fold index so
    /// the next run rebuilds it). Called at the start and the end of a run — the
    /// durable DB never carries a projection's transient plan between runs.
    pub async fn clear_plan(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        self.clear_plan_on(&conn).await
    }

    /// Whether a grouping plan from a finished Phase-1 is on disk that can be
    /// RESUMED — the resume-from-plan salvage signal (issue 60). Phase-1 streams the
    /// parsed notices in id order and appends one `plan_notice` row each (mentions
    /// resolved first, so a planned notice implies its mentions are complete), so a
    /// finished Phase-1 leaves a gapless PREFIX: every parsed notice up to the
    /// highest planned id is in the plan. The signal is therefore "the plan covers
    /// its whole prefix", i.e. `COUNT(plan_notice) == COUNT(parsed notices with id ≤
    /// MAX(planned id))` — NOT `planned == parsed`.
    ///
    /// The distinction matters because daily ingestion continues between the plan
    /// build and the resume (issue 60 originally assumed it did not): those notices
    /// are strictly newer, get higher ids, fall OUTSIDE the prefix, and are folded
    /// by the next incremental projection — so they no longer defeat the salvage.
    /// False on a never-projected DB (the scratch table may be absent) or an empty
    /// plan, so a normal rebuild — whose prior run cleared the plan — rebuilds from
    /// scratch.
    pub async fn plan_is_complete(&self) -> turso::Result<bool> {
        let conn = self.conn().await;
        let mut exists = conn
            .query("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'plan_notice'", ())
            .await?;
        if exists.next().await?.is_none() {
            return Ok(false);
        }
        drop(exists);
        let planned = {
            let mut r = conn.query("SELECT COUNT(*) FROM plan_notice", ()).await?;
            int(&r.next().await?.expect("count row"), 0)
        };
        if planned == 0 {
            return Ok(false);
        }
        // Parsed notices in the plan's prefix (id ≤ the highest planned notice).
        // Equal to `planned` exactly when Phase-1 planned every parsed notice up to
        // where it finished — a complete, resumable prefix. Notices parsed since
        // (higher id) are excluded and left for the incremental projection.
        let prefix_parsed = {
            let mut r = conn
                .query(
                    "SELECT COUNT(*) FROM notices
                     WHERE parse_state = 'parsed'
                       AND id <= (SELECT MAX(notice_id) FROM plan_notice)",
                    (),
                )
                .await?;
            int(&r.next().await?.expect("count row"), 0)
        };
        Ok(planned == prefix_parsed)
    }

    async fn clear_plan_on(&self, conn: &Connection) -> turso::Result<()> {
        conn.execute("DROP INDEX IF EXISTS plan_notice_fold", ()).await?;
        conn.execute("DROP INDEX IF EXISTS plan_ojs_edge_a", ()).await?;
        for table in ["plan_notice", "plan_ojs_node", "plan_ojs_edge"] {
            conn.execute(&format!("DELETE FROM {table}"), ()).await?;
        }
        Ok(())
    }

    /// Insert one batch of the grouping plan (issue 59) in a single transaction.
    /// Every write here is a **sequential append** so the bulk load stays flat as
    /// the plan grows (issue 60 regression): `plan_notice` by its `notice_id` PK
    /// (Phase 1 streams in id order) and `plan_ojs_edge` by rowid. The legacy
    /// nodes are NOT inserted here — a node's key is a random OJS number, so
    /// per-row `INSERT OR IGNORE` into `plan_ojs_node`'s PK was a random-position
    /// probe that thrashed once the node table outgrew the page cache (the
    /// superlinear Phase-1 at 12.4M). They are built once, sorted, in
    /// [`Db::build_plan_groups`] instead.
    pub async fn insert_plan(&self, rows: &[PlanRow]) -> turso::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = self.insert_plan_tx(&conn, rows).await;
        match result {
            Ok(()) => {
                conn.execute("COMMIT", ()).await?;
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    async fn insert_plan_tx(&self, conn: &Connection, rows: &[PlanRow]) -> turso::Result<()> {
        for r in rows {
            conn.execute(
                "INSERT INTO plan_notice(notice_id, procedure_key, legacy, ojs_self, source,
                     source_rank, publication_id, published_at, subtype, group_key)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
                (
                    Value::Integer(r.notice_id),
                    opt_text(r.procedure_key.as_deref()),
                    Value::Integer(i64::from(r.legacy)),
                    r.ojs_self.map_or(Value::Null, Value::Integer),
                    t(&r.source),
                    Value::Integer(r.source_rank),
                    t(&r.publication_id),
                    Value::Integer(r.published_at),
                    opt_text(r.subtype.as_deref()),
                ),
            )
            .await?;
            // A legacy notice with its own OJS number seeds the union-find graph:
            // append its symmetric edges (sequential). Its node — and every edge
            // target's node — is materialised later from these rows and
            // plan_notice.ojs_self, so nothing random is written here.
            if let Some(own) = r.ojs_self.filter(|_| r.legacy) {
                for &edge in &r.ojs_edges {
                    conn.execute(
                        "INSERT INTO plan_ojs_edge(a, b) VALUES(?, ?)",
                        (Value::Integer(own), Value::Integer(edge)),
                    )
                    .await?;
                    conn.execute(
                        "INSERT INTO plan_ojs_edge(a, b) VALUES(?, ?)",
                        (Value::Integer(edge), Value::Integer(own)),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    /// Assign every notice its Tender `group_key` — the grouping, done in SQL so it
    /// never pulls the corpus into RAM (issue 59):
    ///
    /// - **keyed** → the procedure key itself (BT-04 / sdk-0.1 uuid), which is what
    ///   `tenders.procedure_key` stores; a shared key merges across Sources.
    /// - **island** → `island:<notice_id>`, a unique non-colliding handle.
    /// - **legacy** → `ojs:<earliest year>-<number>`, the transitive component's
    ///   minimum OJS number, found by iterative label propagation over the edge
    ///   graph (turso has no `WITH RECURSIVE`): each pass sets a node's label to the
    ///   min of itself and its neighbours until the labels stop moving. OJS keys are
    ///   encoded `year*1e9 + number` so a single `MIN` gives the earliest.
    ///
    /// Finally builds the fold-order index Phase 2 streams by.
    pub async fn build_plan_groups(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        // Drop the fold index before the group_key UPDATEs so they don't pay per-row
        // index maintenance on every one of ~22M writes. It persists across an
        // interrupted resume (grouping re-runs over an already-populated plan), and
        // is rebuilt once at the end.
        conn.execute("DROP INDEX IF EXISTS plan_notice_fold", ()).await?;
        // Keyed and island in one pass; legacy left NULL for the closure below.
        let t = std::time::Instant::now();
        conn.execute(
            "UPDATE plan_notice SET group_key = CASE
                 WHEN procedure_key IS NOT NULL THEN procedure_key
                 WHEN NOT (legacy = 1 AND ojs_self IS NOT NULL) THEN 'island:' || notice_id
                 ELSE NULL END",
            (),
        )
        .await?;
        eprintln!("[project] group step keyed/island: {:.1}s", t.elapsed().as_secs_f64());

        // Legacy transitive components via an in-memory union-find (path-B). Load
        // the legacy adjacency once (bounded by distinct OJS numbers, not the
        // corpus) and union with union-TO-MIN so each component's root IS its
        // minimum OJS key — byte-for-byte the representative the former SQL
        // label-propagation's converged MIN label produced.
        //
        // The node set is every edge endpoint plus every legacy notice's own OJS
        // number (a legacy notice with no refs and no referrer is its own
        // singleton component). Edges are symmetric, so one side covers endpoints.
        let t = std::time::Instant::now();
        let mut uf = MinUnionFind::default();
        {
            let mut rows = conn.query("SELECT a, b FROM plan_ojs_edge", ()).await?;
            while let Some(row) = rows.next().await? {
                uf.union(int(&row, 0), int(&row, 1));
            }
        }
        {
            let mut rows = conn
                .query("SELECT ojs_self FROM plan_notice WHERE legacy = 1 AND ojs_self IS NOT NULL", ())
                .await?;
            while let Some(row) = rows.next().await? {
                uf.add(int(&row, 0));
            }
        }
        eprintln!("[project] group step union-load: {:.1}s ({} nodes)", t.elapsed().as_secs_f64(), uf.parent.len());

        // Assign each legacy notice its component's earliest-OJS key, formatted to
        // match `ojs_procedure_key`: `ojs:{year}-{number:06}`, computed in Rust from
        // the union-find. The former path materialised the reps into `plan_ojs_node`
        // and then ran a per-row correlated-subquery UPDATE (`SELECT … FROM
        // plan_ojs_node WHERE key = plan_notice.ojs_self`). turso 0.7's planner did
        // not lower that inner PK-equality to a rowid seek, so it re-scanned the node
        // table for every one of millions of legacy rows — 100 % CPU, ~0 disk I/O,
        // hours at 12.4M scale. Here `rep = uf.find(ojs_self)` and the same format
        // give byte-identical group keys with no per-row subquery; writes are point
        // UPDATEs by `notice_id` PK in ascending order. `plan_ojs_node` has no other
        // reader in the codebase, so it is no longer materialised.
        let t = std::time::Instant::now();
        let mut legacy: Vec<(i64, i64)> = Vec::new();
        {
            let mut rows = conn
                .query(
                    "SELECT notice_id, ojs_self FROM plan_notice
                      WHERE legacy = 1 AND ojs_self IS NOT NULL ORDER BY notice_id",
                    (),
                )
                .await?;
            while let Some(row) = rows.next().await? {
                legacy.push((int(&row, 0), int(&row, 1)));
            }
        }
        for chunk in legacy.chunks(NODE_WRITE_BATCH) {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            for (notice_id, ojs_self) in chunk {
                let rep = uf.find(*ojs_self);
                let group_key = format!("ojs:{}-{:06}", rep / 1_000_000_000, rep % 1_000_000_000);
                conn.execute(
                    "UPDATE plan_notice SET group_key = ? WHERE notice_id = ?",
                    (Value::Text(group_key), Value::Integer(*notice_id)),
                )
                .await?;
            }
            conn.execute("COMMIT", ()).await?;
        }
        eprintln!("[project] group step legacy-update: {:.1}s ({} legacy)", t.elapsed().as_secs_f64(), legacy.len());

        // The fold order Phase 2 streams by: rows arrive grouped by Tender and, within
        // a Tender, in supersession order (ADR-0003 tiebreak), so no in-RAM sort.
        let t = std::time::Instant::now();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS plan_notice_fold
                 ON plan_notice(group_key, published_at, source_rank, publication_id, notice_id)",
            (),
        )
        .await?;
        eprintln!("[project] group step fold-index: {:.1}s", t.elapsed().as_secs_f64());
        Ok(())
    }

    /// `(tenders, islands)` in the built plan — distinct group keys, and those that
    /// are single-notice islands.
    pub async fn plan_counts(&self) -> turso::Result<(u64, u64)> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT COUNT(DISTINCT group_key),
                        COUNT(DISTINCT CASE WHEN group_key LIKE 'island:%' THEN group_key END)
                   FROM plan_notice",
                (),
            )
            .await?;
        let row = rows.next().await?.expect("counts row");
        Ok((int(&row, 0) as u64, int(&row, 1) as u64))
    }

    /// `(tenders, islands, legacy_keys)` from ONE index-ordered pass over the plan —
    /// the resume/rebuild grouping summary. Streams `group_key` in fold-index order
    /// (so distinct keys arrive consecutively) and folds distinctness in Rust. turso
    /// builds an in-memory hash/sort for `COUNT(DISTINCT)` / `SELECT DISTINCT`, which
    /// is O(distinct) in RAM and spun for many minutes at 12.4M scale — islands have
    /// unique keys, so a DISTINCT over all group keys is ~corpus-wide. Here islands
    /// and tenders are plain counters; only the legacy (`ojs:`) key set is
    /// materialised — bounded by the legacy Tender count (which retirement needs),
    /// not the corpus. `legacy_keys` is what [`Db::retire_absorbed_legacy_tenders`]
    /// checks a late merge against.
    pub async fn plan_summary(&self) -> turso::Result<(u64, u64, BTreeSet<String>)> {
        let conn = self.conn().await;
        let mut rows = conn.query("SELECT group_key FROM plan_notice ORDER BY group_key", ()).await?;
        let (mut tenders, mut islands) = (0u64, 0u64);
        let mut legacy = BTreeSet::new();
        let mut prev: Option<String> = None;
        while let Some(row) = rows.next().await? {
            let gk = text(&row, 0);
            if prev.as_deref() != Some(gk.as_str()) {
                tenders += 1;
                if gk.starts_with("island:") {
                    islands += 1;
                } else if gk.starts_with("ojs:") {
                    legacy.insert(gk.clone());
                }
                prev = Some(gk);
            }
        }
        Ok((tenders, islands, legacy))
    }

    /// Stream the next bounded batch of WHOLE Tenders from the plan, in fold order,
    /// starting after `after_group_key` (`""` for the first batch). Accumulates
    /// whole groups until the notice budget is reached — a Tender is never split
    /// across a batch — so Phase 2 holds one batch, not the corpus (issue 59).
    /// Empty when the plan is exhausted.
    pub async fn next_plan_batch(
        &self,
        after_group_key: &str,
        notice_budget: usize,
    ) -> turso::Result<Vec<PlanGroup>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT notice_id, group_key, source, subtype FROM plan_notice
                  WHERE group_key > ?
                  ORDER BY group_key, published_at, source_rank, publication_id, notice_id",
                (t(after_group_key),),
            )
            .await?;
        let mut groups: Vec<PlanGroup> = Vec::new();
        let mut total = 0usize;
        while let Some(row) = rows.next().await? {
            let notice_id = int(&row, 0);
            let group_key = text(&row, 1);
            let source = text(&row, 2);
            let subtype = opt_text_of(&row, 3);
            match groups.last_mut() {
                Some(g) if g.group_key == group_key => {
                    g.notice_ids.push(notice_id);
                    g.sources.push(source);
                }
                _ => {
                    // A new group begins. Stop *before* opening it if the budget is
                    // already met, so groups are never split across a batch.
                    if total >= notice_budget {
                        break;
                    }
                    groups.push(PlanGroup {
                        group_key,
                        first_subtype: subtype,
                        notice_ids: vec![notice_id],
                        sources: vec![source],
                    });
                }
            }
            total += 1;
        }
        Ok(groups)
    }

    /// Open a streaming mention resolver, preloading the Organization dedup key
    /// `(country, identifier_kind, identifier) → id` **once** for the whole run.
    /// On `--rebuild` the organizations table was just cleared, so the scan is
    /// empty and the map fills as the run proceeds. See [`MentionResolver`].
    pub async fn mention_resolver(&self) -> turso::Result<MentionResolver> {
        use std::collections::HashMap;
        let conn = self.conn().await;
        let mut org_of: HashMap<(Option<String>, String, String), i64> = HashMap::new();
        let mut rows = conn
            .query(
                "SELECT id, country, identifier_kind, identifier FROM organizations
                  WHERE identifier IS NOT NULL",
                (),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            org_of.insert((opt_text_of(&row, 1), text(&row, 2), text(&row, 3)), int(&row, 0));
        }
        Ok(MentionResolver { org_of, created_any: false })
    }

    /// Resolve one bounded batch of mentions onto canonical Organizations,
    /// creating ones as needed, and record the mentions — returning one id per
    /// input mention. Merging happens only on an exact normalised identifier;
    /// everything else gets its own provisional profile, so no mention is ever
    /// destroyed by a merge.
    ///
    /// The dedup runs in memory, not with a per-mention `SELECT` (issue 19): the
    /// old org lookup `WHERE country IS ? AND identifier_kind = ? AND identifier
    /// = ?` scanned the growing organizations table for *every* mention — O(n²),
    /// the projection's original bottleneck (44 min of a 54 min month at ~350k
    /// mentions). The `org_of` map (held across batches on the resolver) makes it
    /// O(1). The idempotency map `(notice, section) → org` — which keeps an
    /// already-recorded mention on its Organization on a re-projection — is
    /// preloaded **for this batch's notices only** (not the whole corpus, which
    /// at 7.5M+ notices is gigabytes; issue 57), because each notice is resolved
    /// exactly once per run. Writes commit in [`WRITE_BATCH`]-sized transactions;
    /// the change-feed doorbell rings once, from [`Db::finish_mention_resolver`].
    pub async fn resolve_mentions(
        &self,
        resolver: &mut MentionResolver,
        mentions: &[Mention],
        now: i64,
    ) -> turso::Result<Vec<i64>> {
        use std::collections::HashMap;
        if mentions.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn().await;

        // Idempotency preload, scoped to this batch's notices: an already-recorded
        // (notice, section) keeps its Organization. Empty on `--rebuild`.
        let mut notice_ids: Vec<i64> = mentions.iter().map(|m| m.notice_id).collect();
        notice_ids.sort_unstable();
        notice_ids.dedup();
        let mut mention_of: HashMap<(i64, String), i64> = HashMap::new();
        for chunk in notice_ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT notice_id, section_id, organization_id FROM organization_mentions
                 WHERE notice_id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                mention_of.insert((int(&row, 0), text(&row, 1)), int(&row, 2));
            }
            drop(rows);
        }

        let mut ids = Vec::with_capacity(mentions.len());
        for chunk in mentions.chunks(WRITE_BATCH) {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            let mut chunk_ids = Vec::with_capacity(chunk.len());
            let mut error = None;
            for m in chunk {
                // The maps are updated as we go, so a later mention in the same
                // chunk reuses an Organization an earlier one just created.
                match self
                    .resolve_one_mention(&conn, m, now, &mut resolver.org_of, &mut mention_of)
                    .await
                {
                    Ok((id, created)) => {
                        chunk_ids.push(id);
                        resolver.created_any |= created;
                    }
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }
            match error {
                None => {
                    conn.execute("COMMIT", ()).await?;
                    ids.extend(chunk_ids);
                }
                Some(e) => {
                    // The in-memory maps may now hold rolled-back entries, but
                    // the run aborts on error, so they are never read again.
                    let _ = conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
            }
        }
        Ok(ids)
    }

    /// Ring the change-cursor doorbell once if the resolver created any
    /// Organization over its lifetime, so a change-feed consumer sees every new
    /// Organization exactly once.
    pub async fn finish_mention_resolver(&self, resolver: MentionResolver) -> turso::Result<()> {
        if resolver.created_any {
            let conn = self.conn().await;
            self.publish_cursor(&conn).await?;
        }
        Ok(())
    }

    async fn resolve_one_mention(
        &self,
        conn: &Connection,
        m: &Mention,
        now: i64,
        org_of: &mut std::collections::HashMap<(Option<String>, String, String), i64>,
        mention_of: &mut std::collections::HashMap<(i64, String), i64>,
    ) -> turso::Result<(i64, bool)> {
        if let Some(&org_id) = mention_of.get(&(m.notice_id, m.section_id.clone())) {
            return Ok((org_id, false));
        }

        let (org_id, created) = match &m.identifier {
            Some(id) => {
                let key = (id.country.clone(), id.kind.clone(), id.value.clone());
                if let Some(&org_id) = org_of.get(&key) {
                    (org_id, false)
                } else {
                    conn.execute(
                        "INSERT INTO organizations(country, identifier_kind, identifier, name,
                             provisional, created_at)
                         VALUES(?, ?, ?, ?, 0, ?)",
                        (
                            opt_text(id.country.as_deref()),
                            t(&id.kind),
                            t(&id.value),
                            t(&m.name),
                            Value::Integer(now),
                        ),
                    )
                    .await?;
                    let org_id = last_insert_rowid(conn).await?;
                    org_of.insert(key, org_id);
                    (org_id, true)
                }
            }
            None => {
                conn.execute(
                    "INSERT INTO organizations(country, identifier_kind, identifier, name,
                         provisional, created_at)
                     VALUES(?, NULL, NULL, ?, 1, ?)",
                    (opt_text(m.country.as_deref()), t(&m.name), Value::Integer(now)),
                )
                .await?;
                (last_insert_rowid(conn).await?, true)
            }
        };

        conn.execute(
            "INSERT INTO organization_mentions(notice_id, section_id, organization_id, name,
                 country, raw_identifier, scheme)
             VALUES(?, ?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(m.notice_id),
                t(&m.section_id),
                Value::Integer(org_id),
                t(&m.name),
                opt_text(m.country.as_deref()),
                opt_text(m.raw_identifier.as_deref()),
                opt_text(m.scheme.as_deref()),
            ),
        )
        .await?;
        mention_of.insert((m.notice_id, m.section_id.clone()), org_id);
        if created {
            append_change(conn, "organization", org_id, None, "added", now).await?;
        }
        Ok((org_id, created))
    }

    /// Reconcile many Tenders, batching [`WRITE_BATCH`] of them per write
    /// transaction (issue 19) instead of one transaction each — a month's ~90k
    /// per-Tender BEGIN/COMMIT round-trips were the projection bottleneck. The
    /// invariants are preserved exactly: each Tender's canonical writes and its
    /// change rows commit together in the batch transaction (a batch is
    /// all-or-nothing); the per-Tender reconcile in [`apply_tender_tx`] is
    /// unchanged, so an unchanged projection still writes nothing; and the
    /// cursor doorbell rings once at the end, so a change-feed consumer sees
    /// every version exactly once — only the batching, not the set, changes.
    pub async fn apply_tenders(&self, projections: &[TenderProjection], now: i64) -> turso::Result<Applied> {
        let conn = self.conn().await;
        let mut total = Applied::default();
        let mut changed_any = false;
        for (batch, chunk) in projections.chunks(WRITE_BATCH).enumerate() {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            let mut applied = Applied::default();
            let mut error = None;
            for p in chunk {
                match self.apply_tender_tx(&conn, p, now).await {
                    Ok(a) => applied.add(a),
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }
            match error {
                None => {
                    conn.execute("COMMIT", ()).await?;
                    changed_any |= applied.changes > 0;
                    total.add(applied);
                }
                Some(e) => {
                    let _ = conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
            }
            // Bound the WAL during the projection burst (issue 42): turso
            // autocheckpoints PASSIVE but reuses the -wal file in place (never
            // shrinks it) and stalls behind any long reader snapshot, so a
            // multi-thousand-batch projection lets the file balloon. TRUNCATE
            // every `CHECKPOINT_EVERY_BATCHES` at the clean point between
            // committed batches returns the space. Best-effort: a checkpoint
            // failure only delays reclaim, never the projection's correctness.
            if (batch + 1).is_multiple_of(CHECKPOINT_EVERY_BATCHES)
                && let Err(e) = checkpoint_on(&conn, CheckpointMode::Truncate).await
            {
                eprintln!("[project] checkpoint after batch {batch}: {e}");
            }
        }
        if changed_any {
            self.publish_cursor(&conn).await?;
        }
        Ok(total)
    }

    async fn apply_tender_tx(
        &self,
        conn: &Connection,
        p: &TenderProjection,
        now: i64,
    ) -> turso::Result<Applied> {
        let mut applied = Applied::default();
        let (tender_id, created) = self.tender_identity(conn, p, now).await?;
        applied.tenders_created += u64::from(created);

        // The projection is deterministic, so the sequence of causing notices
        // is the state key: an unchanged sequence means an unchanged chain, and
        // a changed one is repaired from the first differing position (a
        // late-arriving notice that belongs mid-chain rewrites the tail).
        let stored = self.stored_chain(conn, tender_id).await?;
        let keep = stored
            .iter()
            .zip(&p.versions)
            .take_while(|(a, b)| **a == b.caused_by_notice_id)
            .count();
        if keep == stored.len() && keep == p.versions.len() {
            return Ok(applied);
        }

        for seq in (keep + 1..=stored.len()).rev() {
            self.delete_version(conn, tender_id, seq as i64).await?;
            applied.versions_removed += 1;
        }

        for (i, version) in p.versions.iter().enumerate().skip(keep) {
            let seq = i as i64 + 1;
            let previous = i.checked_sub(1).map(|j| &p.versions[j]);
            self.write_version(conn, tender_id, seq, version).await?;
            applied.versions_written += 1;
            applied.changes +=
                self.append_version_changes(conn, tender_id, seq, version, previous, now).await?;
        }

        // Record the new head (issue 25): the current version is the last of the
        // chain, its `published_at` the date the "newest Tenders" list orders by.
        // Reached only when the chain changed — the unchanged early-return above
        // leaves an already-correct pointer (set when those versions were written,
        // or by the one-time backfill at open for pre-issue-25 rows).
        if let Some(head) = p.versions.last() {
            conn.execute(
                "UPDATE tenders SET current_seq = ?, current_published_at = ? WHERE id = ?",
                (
                    Value::Integer(p.versions.len() as i64),
                    Value::Integer(head.published_at),
                    Value::Integer(tender_id),
                ),
            )
            .await?;
        }
        Ok(applied)
    }

    async fn tender_identity(
        &self,
        conn: &Connection,
        p: &TenderProjection,
        now: i64,
    ) -> turso::Result<(i64, bool)> {
        // A keyed Tender is found by its procedure key alone — that is what
        // merges a procedure's TED and DÖE readings into one Tender (ADR-0003).
        // Islands stay per-notice.
        let mut rows = match (&p.procedure_key, p.island_notice_id) {
            (Some(key), _) => {
                conn.query("SELECT id, source FROM tenders WHERE procedure_key = ?", (t(key),)).await?
            }
            (None, Some(notice_id)) => {
                conn.query(
                    "SELECT id, source FROM tenders WHERE source = ? AND island_notice_id = ?",
                    (t(&p.source), Value::Integer(notice_id)),
                )
                .await?
            }
            (None, None) => unreachable!("a Tender is keyed by its procedure or by its island notice"),
        };
        if let Some(row) = rows.next().await? {
            let id = int(&row, 0);
            // As backfill deepens, a DÖE-first procedure gains its TED twin and
            // the primary Source flips to TED (ADR-0003); keep the label current.
            if text(&row, 1) != p.source {
                conn.execute("UPDATE tenders SET source = ? WHERE id = ?", (t(&p.source), Value::Integer(id)))
                    .await?;
            }
            return Ok((id, false));
        }
        conn.execute(
            "INSERT INTO tenders(source, procedure_key, island_notice_id, kind, created_at)
             VALUES(?, ?, ?, ?, ?)",
            (
                t(&p.source),
                opt_text(p.procedure_key.as_deref()),
                opt_int(p.island_notice_id),
                t(&p.kind),
                Value::Integer(now),
            ),
        )
        .await?;
        Ok((last_insert_rowid(conn).await?, true))
    }

    async fn stored_chain(&self, conn: &Connection, tender_id: i64) -> turso::Result<Vec<i64>> {
        let mut rows = conn
            .query(
                "SELECT caused_by_notice_id FROM tender_versions WHERE tender_id = ? ORDER BY seq",
                (Value::Integer(tender_id),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(int(&row, 0));
        }
        Ok(out)
    }

    async fn delete_version(&self, conn: &Connection, tender_id: i64, seq: i64) -> turso::Result<()> {
        for table in [
            "tender_version_result_winners",
            "tender_version_result_stats",
            "tender_version_lot_results",
            "tender_version_bid_parties",
            "tender_version_bids",
            "tender_version_contracts",
            "tender_version_parties",
            "tender_version_texts",
            "tender_version_dates",
            "tender_version_amounts",
            "tender_version_classifications",
            "tender_version_lots",
            "tender_versions",
        ] {
            conn.execute(
                &format!("DELETE FROM {table} WHERE tender_id = ? AND seq = ?"),
                (Value::Integer(tender_id), Value::Integer(seq)),
            )
            .await?;
        }
        Ok(())
    }

    async fn write_version(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        v: &TenderVersion,
    ) -> turso::Result<()> {
        conn.execute(
            "INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at,
                 dispatched_at, notice_subtype, publication_id)
             VALUES(?, ?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(tender_id),
                Value::Integer(seq),
                Value::Integer(v.caused_by_notice_id),
                Value::Integer(v.published_at),
                opt_int(v.dispatched_at),
                opt_text(v.notice_subtype.as_deref()),
                t(&v.publication_id),
            ),
        )
        .await?;
        self.write_facts(conn, tender_id, seq, None, &v.facts).await?;
        for lot in &v.lots {
            let lot_id = self.lot_identity(conn, tender_id, &lot.key).await?;
            conn.execute(
                "INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind) VALUES(?, ?, ?, ?)",
                (
                    Value::Integer(tender_id),
                    Value::Integer(seq),
                    Value::Integer(lot_id),
                    t(&lot.kind),
                ),
            )
            .await?;
            self.write_facts(conn, tender_id, seq, Some(lot_id), &lot.facts).await?;
        }
        for round in &v.rounds {
            self.write_round(conn, tender_id, seq, round).await?;
        }
        Ok(())
    }

    async fn write_round(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        round: &Round,
    ) -> turso::Result<()> {
        let scope = || (Value::Integer(tender_id), Value::Integer(seq));
        for result in &round.lot_results {
            let id = self
                .result_identity(conn, "lot_results", "result_key", tender_id, round.notice_id, &result.key)
                .await?;
            let lot_id = self.result_lot(conn, tender_id, result.lot_key.as_deref()).await?;
            let (a, b) = scope();
            conn.execute(
                "INSERT INTO tender_version_lot_results(tender_id, seq, lot_result_id, lot_id,
                     decision, reason, awarded_cents, awarded_currency)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    a,
                    b,
                    Value::Integer(id),
                    opt_int(lot_id),
                    opt_text(result.decision.as_deref()),
                    opt_text(result.reason.as_deref()),
                    opt_int(result.awarded_cents),
                    opt_text(result.awarded_currency.as_deref()),
                ),
            )
            .await?;
            for organization_id in &result.winners {
                let (a, b) = scope();
                conn.execute(
                    "INSERT INTO tender_version_result_winners(tender_id, seq, lot_result_id,
                         organization_id)
                     VALUES(?, ?, ?, ?)",
                    (a, b, Value::Integer(id), Value::Integer(*organization_id)),
                )
                .await?;
            }
            for (kind, count) in &result.statistics {
                let (a, b) = scope();
                conn.execute(
                    "INSERT INTO tender_version_result_stats(tender_id, seq, lot_result_id, kind, count)
                     VALUES(?, ?, ?, ?, ?)",
                    (a, b, Value::Integer(id), t(kind), Value::Integer(*count)),
                )
                .await?;
            }
        }
        for bid in &round.bids {
            let id = self
                .result_identity(conn, "bids", "bid_key", tender_id, round.notice_id, &bid.key)
                .await?;
            let lot_id = self.result_lot(conn, tender_id, bid.lot_key.as_deref()).await?;
            let (a, b) = scope();
            conn.execute(
                "INSERT INTO tender_version_bids(tender_id, seq, bid_id, lot_id, cents, currency)
                 VALUES(?, ?, ?, ?, ?, ?)",
                (
                    a,
                    b,
                    Value::Integer(id),
                    opt_int(lot_id),
                    opt_int(bid.cents),
                    opt_text(bid.currency.as_deref()),
                ),
            )
            .await?;
            for party in &bid.parties {
                let (a, b) = scope();
                conn.execute(
                    "INSERT INTO tender_version_bid_parties(tender_id, seq, bid_id, role,
                         organization_id, mention_notice_id, mention_section_id)
                     VALUES(?, ?, ?, ?, ?, ?, ?)",
                    (
                        a,
                        b,
                        Value::Integer(id),
                        t(&party.role),
                        Value::Integer(party.organization_id),
                        Value::Integer(round.notice_id),
                        t(&party.section_id),
                    ),
                )
                .await?;
            }
        }
        for contract in &round.contracts {
            let id = self
                .result_identity(conn, "contracts", "contract_key", tender_id, round.notice_id, &contract.key)
                .await?;
            let (a, b) = scope();
            let (utc, offset, has_time) = match contract.concluded {
                Some((utc, offset, has_time)) => {
                    (Some(utc), Some(offset), Some(i64::from(has_time)))
                }
                None => (None, None, None),
            };
            conn.execute(
                "INSERT INTO tender_version_contracts(tender_id, seq, contract_id, buyer_contract_id,
                     concluded_utc, concluded_offset, concluded_has_time, cents, currency)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    a,
                    b,
                    Value::Integer(id),
                    opt_text(contract.buyer_contract_id.as_deref()),
                    opt_int(utc),
                    opt_int(offset),
                    opt_int(has_time),
                    opt_int(contract.cents),
                    opt_text(contract.currency.as_deref()),
                ),
            )
            .await?;
        }
        Ok(())
    }

    /// A results entity's identity row: (tender, origin notice, section key).
    async fn result_identity(
        &self,
        conn: &Connection,
        table: &str,
        key_column: &str,
        tender_id: i64,
        notice_id: i64,
        key: &str,
    ) -> turso::Result<i64> {
        let mut rows = conn
            .query(
                &format!("SELECT id FROM {table} WHERE tender_id = ? AND notice_id = ? AND {key_column} = ?"),
                (Value::Integer(tender_id), Value::Integer(notice_id), t(key)),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            return Ok(int(&row, 0));
        }
        conn.execute(
            &format!("INSERT INTO {table}(tender_id, notice_id, {key_column}) VALUES(?, ?, ?)"),
            (Value::Integer(tender_id), Value::Integer(notice_id), t(key)),
        )
        .await?;
        last_insert_rowid(conn).await
    }

    /// Resolve a result's published lot reference to a Lot row — creating the
    /// identity if the result notice referenced a lot no notice sectioned,
    /// because the published id is a fact and there is nowhere to record a guess.
    async fn result_lot(
        &self,
        conn: &Connection,
        tender_id: i64,
        lot_key: Option<&str>,
    ) -> turso::Result<Option<i64>> {
        Ok(match lot_key {
            Some(key) => Some(self.lot_identity(conn, tender_id, key).await?),
            None => None,
        })
    }

    async fn write_facts(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        lot_id: Option<i64>,
        facts: &BTreeSet<Fact>,
    ) -> turso::Result<()> {
        let scope = || (Value::Integer(tender_id), Value::Integer(seq), opt_int(lot_id));
        for fact in facts {
            let (a, b, c) = scope();
            match fact {
                Fact::Text { field, lang, value } => {
                    conn.execute(
                        "INSERT INTO tender_version_texts(tender_id, seq, lot_id, field, lang, value)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (a, b, c, t(field), opt_text(lang.as_deref()), t(value)),
                    )
                    .await?;
                }
                Fact::Amount { field, cents, currency } => {
                    conn.execute(
                        "INSERT INTO tender_version_amounts(tender_id, seq, lot_id, field, cents, currency)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (a, b, c, t(field), Value::Integer(*cents), t(currency)),
                    )
                    .await?;
                }
                Fact::Classification { field, scheme, code } => {
                    conn.execute(
                        "INSERT INTO tender_version_classifications(tender_id, seq, lot_id, field,
                             scheme, code)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (a, b, c, t(field), t(scheme), t(code)),
                    )
                    .await?;
                }
                Fact::Date { field, utc_seconds, offset_minutes, has_time } => {
                    conn.execute(
                        "INSERT INTO tender_version_dates(tender_id, seq, lot_id, field,
                             utc_seconds, offset_minutes, has_time)
                         VALUES(?, ?, ?, ?, ?, ?, ?)",
                        (
                            a,
                            b,
                            c,
                            t(field),
                            Value::Integer(*utc_seconds),
                            Value::Integer(*offset_minutes),
                            Value::Integer(i64::from(*has_time)),
                        ),
                    )
                    .await?;
                }
                Fact::Party { role, organization_id, notice_id, section_id } => {
                    conn.execute(
                        "INSERT INTO tender_version_parties(tender_id, seq, lot_id, role,
                             organization_id, mention_notice_id, mention_section_id)
                         VALUES(?, ?, ?, ?, ?, ?, ?)",
                        (
                            a,
                            b,
                            c,
                            t(role),
                            Value::Integer(*organization_id),
                            Value::Integer(*notice_id),
                            t(section_id),
                        ),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn lot_identity(&self, conn: &Connection, tender_id: i64, key: &str) -> turso::Result<i64> {
        let mut rows = conn
            .query(
                "SELECT id FROM lots WHERE tender_id = ? AND lot_key = ?",
                (Value::Integer(tender_id), t(key)),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            return Ok(int(&row, 0));
        }
        conn.execute(
            "INSERT INTO lots(tender_id, lot_key) VALUES(?, ?)",
            (Value::Integer(tender_id), t(key)),
        )
        .await?;
        last_insert_rowid(conn).await
    }

    /// The diff-based change scoping of ADR-0001's amendment: what this version
    /// changed is what differs from the previous one, per entity.
    async fn append_version_changes(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        v: &TenderVersion,
        previous: Option<&TenderVersion>,
        now: i64,
    ) -> turso::Result<u64> {
        let mut count = 0;
        match previous {
            None => {
                append_change(conn, "tender", tender_id, Some(seq), "added", now).await?;
                count += 1;
            }
            Some(prev) if prev.facts != v.facts || prev.rounds != v.rounds => {
                append_change(conn, "tender", tender_id, Some(seq), "changed", now).await?;
                count += 1;
            }
            Some(_) => {}
        }

        let previous_lots = previous.map(|p| p.lots.as_slice()).unwrap_or_default();
        for lot in &v.lots {
            let lot_id = self.lot_identity(conn, tender_id, &lot.key).await?;
            let op = match previous_lots.iter().find(|l| l.key == lot.key) {
                None => "added",
                Some(before) if before.facts != lot.facts || before.kind != lot.kind => "changed",
                Some(_) => continue,
            };
            append_change(conn, "lot", lot_id, Some(seq), op, now).await?;
            count += 1;
        }
        for gone in previous_lots.iter().filter(|l| !v.lots.iter().any(|n| n.key == l.key)) {
            let lot_id = self.lot_identity(conn, tender_id, &gone.key).await?;
            append_change(conn, "lot", lot_id, Some(seq), "removed", now).await?;
            count += 1;
        }

        let previous_rounds = previous.map(|p| p.rounds.as_slice()).unwrap_or_default();
        count += self.append_round_changes(conn, tender_id, seq, previous_rounds, &v.rounds, now).await?;
        Ok(count)
    }

    /// Diff the results layer of two consecutive versions. Rounds accumulate,
    /// so almost every diff is a new round's `added` rows; a correction that
    /// replaced a round reads as `removed` + `added`, because the corrected
    /// entities have a different origin notice and are different entities.
    async fn append_round_changes(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        previous: &[Round],
        current: &[Round],
        now: i64,
    ) -> turso::Result<u64> {
        let mut count = 0;
        for (kind, table, column) in [
            ("lot_result", "lot_results", "result_key"),
            ("bid", "bids", "bid_key"),
            ("contract", "contracts", "contract_key"),
        ] {
            let curr = flatten(current, kind);
            let prev = flatten(previous, kind);
            for (notice_id, key, state) in &curr {
                let op = match prev.iter().find(|(n, k, _)| n == notice_id && k == key) {
                    None => "added",
                    Some((_, _, before)) if before != state => "changed",
                    Some(_) => continue,
                };
                let id = self.result_identity(conn, table, column, tender_id, *notice_id, key).await?;
                append_change(conn, kind, id, Some(seq), op, now).await?;
                count += 1;
            }
            for (notice_id, key, _) in &prev {
                if curr.iter().any(|(n, k, _)| n == notice_id && k == key) {
                    continue;
                }
                let id = self.result_identity(conn, table, column, tender_id, *notice_id, key).await?;
                append_change(conn, kind, id, Some(seq), "removed", now).await?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Retire legacy Tenders that no producing group claims any more — the
    /// absorbed side of an ADR-0003-style merge. When a late edge joins two OJS
    /// components, every member re-projects under the component's surviving
    /// earliest-OJS key; the old key's identity is then orphaned. Its notices'
    /// versions were re-added under the survivor by [`Self::apply_tenders`], so
    /// here we only emit the `removed` change events and delete the orphan's
    /// rows. Organization mentions are left untouched — they are immutable
    /// evidence and now feed the survivor's parties.
    pub async fn retire_absorbed_legacy_tenders(
        &self,
        produced: &BTreeSet<String>,
        now: i64,
    ) -> turso::Result<u64> {
        let conn = self.conn().await;
        let mut rows = conn
            .query("SELECT id, procedure_key FROM tenders WHERE procedure_key LIKE 'ojs:%'", ())
            .await?;
        let mut absorbed = Vec::new();
        while let Some(row) = rows.next().await? {
            let key = text(&row, 1);
            if !produced.contains(&key) {
                absorbed.push(int(&row, 0));
            }
        }
        drop(rows);
        if absorbed.is_empty() {
            return Ok(0);
        }

        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let mut result = Ok(());
        for id in &absorbed {
            if let Err(e) = self.retire_tender_tx(&conn, *id, now).await {
                result = Err(e);
                break;
            }
        }
        match result {
            Ok(()) => {
                conn.execute("COMMIT", ()).await?;
                self.publish_cursor(&conn).await?;
                Ok(absorbed.len() as u64)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    async fn retire_tender_tx(&self, conn: &Connection, tender_id: i64, now: i64) -> turso::Result<()> {
        let id = Value::Integer(tender_id);
        append_change(conn, "tender", tender_id, None, "removed", now).await?;
        for (kind, table) in
            [("lot", "lots"), ("lot_result", "lot_results"), ("bid", "bids"), ("contract", "contracts")]
        {
            let mut rows =
                conn.query(&format!("SELECT id FROM {table} WHERE tender_id = ?"), (id.clone(),)).await?;
            let mut ids = Vec::new();
            while let Some(row) = rows.next().await? {
                ids.push(int(&row, 0));
            }
            drop(rows);
            for entity in ids {
                append_change(conn, kind, entity, None, "removed", now).await?;
            }
        }
        for table in [
            "tender_version_result_winners",
            "tender_version_result_stats",
            "tender_version_lot_results",
            "tender_version_bid_parties",
            "tender_version_bids",
            "tender_version_contracts",
            "tender_version_parties",
            "tender_version_texts",
            "tender_version_dates",
            "tender_version_amounts",
            "tender_version_classifications",
            "tender_version_lots",
            "tender_versions",
            "lot_results",
            "bids",
            "contracts",
            "lots",
        ] {
            conn.execute(&format!("DELETE FROM {table} WHERE tender_id = ?"), (id.clone(),)).await?;
        }
        conn.execute("DELETE FROM tenders WHERE id = ?", (id,)).await?;
        Ok(())
    }

    // ---------------------------------------------------- incremental (issue 58)

    /// The EXISTING Tenders touched by an incremental change-set: every Tender
    /// that already contains one of the `changed` notices (old membership — a
    /// correction/award attaching to an old Tender), plus every Tender whose
    /// `procedure_key` is a `changed` notice's NEW identity key (a notice joining
    /// or regrouping into an already-existing keyed Tender). These are the only
    /// Tenders an incremental run may rewrite or retire; untouched Tenders are
    /// never read. Bounded by the delta, not the corpus.
    pub async fn touched_existing_tender_ids(
        &self,
        changed: &[i64],
        new_keyed_keys: &[String],
    ) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut set = std::collections::BTreeSet::new();
        for chunk in changed.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT DISTINCT tender_id FROM tender_versions WHERE caused_by_notice_id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                set.insert(int(&row, 0));
            }
        }
        for chunk in new_keyed_keys.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|k| t(k)).collect();
            let sql = format!(
                "SELECT id FROM tenders WHERE procedure_key IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                set.insert(int(&row, 0));
            }
        }
        Ok(set.into_iter().collect())
    }

    /// The full notice-id set of the given Tenders — every notice that folds into
    /// them (`tender_versions.caused_by_notice_id`). An incremental run seeds its
    /// scoped plan with these (∪ the changed notices) so each touched Tender is
    /// re-derived IN FULL, exactly as a full projection would.
    pub async fn notice_ids_for_tenders(&self, tender_ids: &[i64]) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut set = std::collections::BTreeSet::new();
        for chunk in tender_ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT caused_by_notice_id FROM tender_versions WHERE tender_id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                set.insert(int(&row, 0));
            }
        }
        Ok(set.into_iter().collect())
    }

    /// Retire any touched Tender whose identity key the freshly-built plan did NOT
    /// reproduce (issue 58) — a notice regrouped away (island → keyed upgrade), or
    /// a legacy component merged. The plan's `group_key` set is authoritative: a
    /// touched Tender absent from it has no notices left and gets `removed` change
    /// events, exactly like [`Db::retire_absorbed_legacy_tenders`] but scoped to
    /// the touched set instead of the whole corpus. Returns the count retired.
    pub async fn retire_regrouped_tenders(
        &self,
        touched: &[i64],
        now: i64,
    ) -> turso::Result<u64> {
        if touched.is_empty() {
            return Ok(0);
        }
        let conn = self.conn().await;
        let mut orphaned = Vec::new();
        for &id in touched {
            let mut rows = conn
                .query("SELECT procedure_key, island_notice_id FROM tenders WHERE id = ?", (Value::Integer(id),))
                .await?;
            let Some(row) = rows.next().await? else { continue };
            let key = match opt_text_of(&row, 0) {
                Some(k) => k,
                None => format!("island:{}", int(&row, 1)),
            };
            drop(rows);
            let mut hit = conn
                .query("SELECT 1 FROM plan_notice WHERE group_key = ? LIMIT 1", (t(&key),))
                .await?;
            if hit.next().await?.is_none() {
                orphaned.push(id);
            }
        }
        if orphaned.is_empty() {
            return Ok(0);
        }
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let mut result = Ok(());
        for id in &orphaned {
            if let Err(e) = self.retire_tender_tx(&conn, *id, now).await {
                result = Err(e);
                break;
            }
        }
        match result {
            Ok(()) => {
                conn.execute("COMMIT", ()).await?;
                self.publish_cursor(&conn).await?;
                Ok(orphaned.len() as u64)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    /// Award-linkage per era (docs/research/ted-legacy-mapping.md §3): of the
    /// Tenders that carry an award (any `lot_results`), how many are a single
    /// notice — an award that never chained to its contract notice. The era is
    /// the profile of the Tender's first version's notice. Returns
    /// `(profile, award_tenders, unchained)` rows.
    ///
    /// A deliberate mirror of `ingest::data_quality::LINKAGE_SQL` — the same
    /// metric, kept as two copies because this is the dashboard's typed query
    /// while that is a raw string in the CLI report's `(label, sql)` catalog;
    /// folding one into the other would couple that self-contained catalog to
    /// store internals. Both use the indexed-`EXISTS` formulation for a reason:
    /// wrapping `lot_results`/`tender_versions` in inline `(SELECT … ) JOIN`
    /// derived tables makes turso re-evaluate them per outer row and times the
    /// query out at a few thousand awards (verified against prod). Keep the two
    /// in sync.
    pub async fn award_linkage(&self) -> turso::Result<Vec<(String, i64, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT n.profile,
                        COUNT(*) AS awards,
                        SUM(CASE WHEN NOT EXISTS(
                              SELECT 1 FROM tender_versions tv
                               WHERE tv.tender_id = v1.tender_id AND tv.seq > 1
                            ) THEN 1 ELSE 0 END) AS unchained
                   FROM tender_versions v1
                   JOIN notices n ON n.id = v1.caused_by_notice_id
                  WHERE v1.seq = 1
                    AND EXISTS(SELECT 1 FROM lot_results lr WHERE lr.tender_id = v1.tender_id)
                  GROUP BY n.profile
                  ORDER BY n.profile",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1), int(&row, 2)));
        }
        Ok(out)
    }

    /// `(rows, )` counts for the canonical layer — what the CLI and the
    /// dashboard report.
    pub async fn canonical_counts(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut out = Vec::new();
        for (label, sql) in [
            ("tenders", "SELECT COUNT(*) FROM tenders"),
            ("tenders (island)", "SELECT COUNT(*) FROM tenders WHERE procedure_key IS NULL"),
            ("tender_versions", "SELECT COUNT(*) FROM tender_versions"),
            ("lots", "SELECT COUNT(*) FROM lots"),
            ("lot_results", "SELECT COUNT(*) FROM lot_results"),
            ("bids", "SELECT COUNT(*) FROM bids"),
            ("contracts", "SELECT COUNT(*) FROM contracts"),
            ("organizations", "SELECT COUNT(*) FROM organizations"),
            ("organizations (canonical)", "SELECT COUNT(*) FROM organizations WHERE provisional = 0"),
            ("organizations (provisional)", "SELECT COUNT(*) FROM organizations WHERE provisional = 1"),
            ("organization_mentions", "SELECT COUNT(*) FROM organization_mentions"),
            ("changes", "SELECT COUNT(*) FROM changes"),
        ] {
            let mut rows = conn.query(sql, ()).await?;
            let count = rows.next().await?.map_or(0, |row| int(&row, 0));
            out.push((label.to_owned(), count));
        }
        Ok(out)
    }

    /// The first column of the first row of a read query. The canonical layer's
    /// contract is "queryable with plain SQL" (ADR-0001), and this is how tests
    /// and operational spot-checks hold it to that; the guarded public SQL
    /// endpoint is issue 07.
    pub async fn scalar(&self, sql: &str) -> turso::Result<Option<Value>> {
        let conn = self.reader().await?;
        let mut rows = conn.query(sql, ()).await?;
        Ok(match rows.next().await? {
            Some(row) => row.get_value(0).ok(),
            None => None,
        })
    }

    /// How many notices are parsed — the projection's Phase-1 total, read once up
    /// front so it can log a progress fraction over a run that spans many minutes.
    pub async fn parsed_notice_count(&self) -> turso::Result<u64> {
        let conn = self.reader().await?;
        let mut rows =
            conn.query("SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed'", ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0) as u64))
    }

}

/// One entry of the change log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub cursor: i64,
    pub entity_kind: String,
    pub entity_id: i64,
    pub version_seq: Option<i64>,
    pub op: String,
    pub changed_at: i64,
}

/// One results entity flattened out of a round set, for diffing across
/// versions. Entities are keyed by (origin notice, section key), which is why
/// accumulating rounds can never alias each other.
#[derive(PartialEq)]
enum RoundEntity<'a> {
    LotResult(&'a LotResultState),
    Bid(&'a BidState),
    Contract(&'a ContractState),
}

fn flatten<'a>(rounds: &'a [Round], kind: &str) -> Vec<(i64, &'a str, RoundEntity<'a>)> {
    let mut out = Vec::new();
    for r in rounds {
        match kind {
            "lot_result" => out.extend(
                r.lot_results.iter().map(|e| (r.notice_id, e.key.as_str(), RoundEntity::LotResult(e))),
            ),
            "bid" => {
                out.extend(r.bids.iter().map(|e| (r.notice_id, e.key.as_str(), RoundEntity::Bid(e))));
            }
            _ => out.extend(
                r.contracts.iter().map(|e| (r.notice_id, e.key.as_str(), RoundEntity::Contract(e))),
            ),
        }
    }
    out
}

async fn append_change(
    conn: &Connection,
    entity_kind: &str,
    entity_id: i64,
    version_seq: Option<i64>,
    op: &str,
    now: i64,
) -> turso::Result<()> {
    conn.execute(
        "INSERT INTO changes(entity_kind, entity_id, version_seq, op, changed_at)
         VALUES(?, ?, ?, ?, ?)",
        (
            t(entity_kind),
            Value::Integer(entity_id),
            opt_int(version_seq),
            t(op),
            Value::Integer(now),
        ),
    )
    .await?;
    Ok(())
}

async fn last_insert_rowid(conn: &Connection) -> turso::Result<i64> {
    // turso exposes the last rowid in-memory; a `SELECT last_insert_rowid()`
    // round-trip after every insert was a large slice of the projection's query
    // volume (issue 19). Kept async so the call sites are unchanged.
    Ok(conn.last_insert_rowid())
}

type ValueBuilder = fn(&turso::Row) -> crate::NoticeValue;

/// Every value table read back into its [`crate::NoticeValue`] variant, over a
/// `notice_id` window (`>= ?1 AND <= ?2`), in `notice_id` order, with
/// `notice_id` as column 0 and the payload shifted one column right. Used by
/// [`Db::parsed_chunk`] so the projection reads the notice layer in a handful of
/// scans per chunk rather than a per-notice query storm (issue 19).
/// The value satellite tables of the parsed layer, each as `(table, columns,
/// builder)`. The columns are `notice_id, section_id, field_id, ordinal` then the
/// table's value columns, so a row's positions are the same whichever `WHERE`
/// selects it — that is what lets both the id-window read ([`Db::parsed_chunk`])
/// and the id-set read ([`Db::parsed_by_ids`]) share one builder set.
fn value_sources() -> Vec<(&'static str, &'static str, ValueBuilder)> {
    use crate::NoticeValue as V;
    vec![
        ("notice_texts", "notice_id, section_id, field_id, ordinal, lang, value",
            (|r| V::Text { lang: opt_text_of(r, 4), value: text(r, 5) }) as ValueBuilder),
        ("notice_codes", "notice_id, section_id, field_id, ordinal, list_name, code",
            |r| V::Code { list: opt_text_of(r, 4), code: text(r, 5) }),
        ("notice_classifications", "notice_id, section_id, field_id, ordinal, scheme, code",
            |r| V::Classification { scheme: text(r, 4), code: text(r, 5) }),
        ("notice_amounts", "notice_id, section_id, field_id, ordinal, cents, currency",
            |r| V::Amount { cents: int(r, 4), currency: text(r, 5) }),
        ("notice_dates", "notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time",
            |r| V::Date { utc_seconds: int(r, 4), offset_minutes: int(r, 5), has_time: int(r, 6) != 0 }),
        ("notice_integers", "notice_id, section_id, field_id, ordinal, value",
            |r| V::Integer(int(r, 4))),
        ("notice_numbers", "notice_id, section_id, field_id, ordinal, value, unit",
            |r| V::Number {
                value: match r.get_value(4) {
                    Ok(Value::Real(f)) => f,
                    Ok(Value::Integer(i)) => i as f64,
                    _ => 0.0,
                },
                unit: opt_text_of(r, 5),
            }),
        ("notice_ids", "notice_id, section_id, field_id, ordinal, scheme, value, is_ref",
            |r| V::Id { scheme: opt_text_of(r, 4), value: text(r, 5), is_ref: int(r, 6) != 0 }),
    ]
}

/// A `?,?,…` placeholder list of `n` bind slots for an `IN (…)` clause.
fn placeholders(n: usize) -> String {
    let mut s = String::with_capacity(n * 2);
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push('?');
    }
    s
}

/// SQLite's default bind-variable ceiling is 999; stay well under it so an
/// `IN (…)` read of a batch's ids never overflows a single statement.
const IN_CHUNK: usize = 512;

#[cfg(test)]
mod tests {
    use super::MinUnionFind;
    use std::collections::{HashMap, HashSet, VecDeque};

    /// A deterministic LCG so the random graphs are reproducible without
    /// `Math.random`/`rand` (Numerical Recipes constants).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, n: u64) -> u64 {
            self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % n
        }
    }

    /// Brute-force reference: each node's component MINIMUM key, by BFS over the
    /// undirected adjacency — the invariant the label-propagation converged to and
    /// the union-find must reproduce exactly.
    fn component_mins(nodes: &[i64], edges: &[(i64, i64)]) -> HashMap<i64, i64> {
        let mut adj: HashMap<i64, Vec<i64>> = HashMap::new();
        for &n in nodes {
            adj.entry(n).or_default();
        }
        for &(a, b) in edges {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }
        let mut min_of: HashMap<i64, i64> = HashMap::new();
        let mut seen: HashSet<i64> = HashSet::new();
        for &start in adj.keys() {
            if !seen.insert(start) {
                continue;
            }
            // Collect the whole component, then stamp its min on every member.
            let mut queue = VecDeque::from([start]);
            let mut members = vec![start];
            let mut min = start;
            while let Some(cur) = queue.pop_front() {
                for &nb in &adj[&cur] {
                    if seen.insert(nb) {
                        members.push(nb);
                        min = min.min(nb);
                        queue.push_back(nb);
                    }
                }
                min = min.min(cur);
            }
            for m in members {
                min_of.insert(m, min);
            }
        }
        min_of
    }

    /// The union-find representative MUST equal the component minimum for every
    /// node, across many random graphs (chains, bridges, isolated singletons,
    /// dense clusters) — the byte-identical-grouping guarantee for legacy Tenders.
    #[test]
    fn union_find_rep_is_the_component_min_like_label_propagation() {
        let mut lcg = Lcg(0x9E37_79B9_7F4A_7C15);
        for trial in 0..500 {
            // A pool of scattered OJS-like keys (non-monotonic, like real numbers).
            let node_count = 1 + lcg.next(40) as usize;
            let nodes: Vec<i64> =
                (0..node_count).map(|_| (lcg.next(5_000) as i64) * 1_000_000_000 + lcg.next(999_999) as i64).collect();
            let mut nodes = nodes;
            nodes.sort_unstable();
            nodes.dedup();

            let edge_count = lcg.next(3 * nodes.len() as u64 + 1) as usize;
            let mut edges = Vec::new();
            for _ in 0..edge_count {
                let a = nodes[lcg.next(nodes.len() as u64) as usize];
                let b = nodes[lcg.next(nodes.len() as u64) as usize];
                edges.push((a, b));
            }

            let mut uf = MinUnionFind::default();
            for &(a, b) in &edges {
                uf.union(a, b);
            }
            for &n in &nodes {
                uf.add(n);
            }

            let reference = component_mins(&nodes, &edges);
            for &n in &nodes {
                assert_eq!(
                    uf.find(n),
                    reference[&n],
                    "trial {trial}: union-find rep for {n} != component min"
                );
            }
        }
    }
}
