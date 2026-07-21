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
        -- A procedure key is globally unique across Sources: a TED eForms
        -- procedure and its DÖE twin share one BT-04 UUID and must collapse into
        -- one Tender (ADR-0003), and legacy `ojs:` keys are TED-only, so the key
        -- alone identifies the Tender. `source` is the primary Source label
        -- (TED where the procedure appears on both).
        UNIQUE(procedure_key),
        UNIQUE(island_notice_id)
    ) STRICT;

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

    -- A canonical Organization profile. `identifier` is the normalised official
    -- id that merged its mentions; a profile without one is `provisional` — it
    -- represents exactly one mention and never absorbs another, because
    -- name-only matching would merge distinct companies (CONTEXT.md).
    CREATE TABLE IF NOT EXISTS organizations (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        country         TEXT,
        identifier_kind TEXT, -- vat | national
        identifier      TEXT,
        name            TEXT NOT NULL,
        provisional     INTEGER NOT NULL,
        created_at      INTEGER NOT NULL,
        UNIQUE(country, identifier_kind, identifier)
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

    DROP VIEW IF EXISTS v_tender_current;
    CREATE VIEW v_tender_current AS
    SELECT tender_id, MAX(seq) AS seq FROM tender_versions GROUP BY tender_id;

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
      JOIN v_tender_current c ON c.tender_id = t.id
      JOIN tender_versions v ON v.tender_id = t.id AND v.seq = c.seq;

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
";

/// How many Tenders (or Organization mentions) a single projection write
/// transaction covers (issue 19). Batching amortises per-transaction overhead —
/// the projection bottleneck — while keeping each batch small enough that its
/// change rows stay atomic with their canonical writes and a failure rolls back
/// only a bounded slice. A batch is one BEGIN IMMEDIATE … COMMIT.
const WRITE_BATCH: usize = 512;

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

impl Db {
    /// Every notice whose profile parser consumed it — the projection's input.
    pub async fn parsed_notices(&self) -> turso::Result<Vec<NoticeRef>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT id, source, publication_id, profile FROM notices
                 WHERE parse_state = 'parsed' ORDER BY id",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(NoticeRef {
                id: int(&row, 0),
                source: text(&row, 1),
                publication_id: text(&row, 2),
                profile: text(&row, 3),
            });
        }
        Ok(out)
    }

    /// Read one notice's parsed form back out of the notice layer — the exact
    /// [`Parsed`] the profile produced.
    pub async fn parsed_notice(&self, notice_id: i64) -> turso::Result<Parsed> {
        let conn = self.reader().await?;
        let id = Value::Integer(notice_id);
        let mut parsed = Parsed::default();

        let mut rows = conn
            .query(
                "SELECT section_id, kind, parent_section_id FROM notice_sections WHERE notice_id = ?",
                (id.clone(),),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            parsed.sections.push(Section {
                id: text(&row, 0),
                kind: text(&row, 1),
                parent: opt_text_of(&row, 2),
            });
        }

        for (sql, build) in value_queries() {
            let mut rows = conn.query(sql, (id.clone(),)).await?;
            while let Some(row) = rows.next().await? {
                parsed.values.push(ValueRow {
                    section_id: text(&row, 0),
                    field_id: text(&row, 1),
                    ordinal: int(&row, 2),
                    value: build(&row),
                });
            }
        }
        Ok(parsed)
    }

    /// The next chunk of parsed notices with `id > after_id` (up to `limit`),
    /// each with its full [`Parsed`] form, read in a fixed handful of scans over
    /// the chunk's id window rather than the ~9 queries **per notice** that
    /// [`parsed_notice`](Self::parsed_notice) costs — the projection's batched
    /// input (issue 19). Empty when no more parsed notices follow `after_id`.
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

        for (sql, build) in all_value_queries() {
            let mut rows = conn.query(sql, window.clone()).await?;
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
        Ok(())
    }

    /// Resolve every mention onto a canonical Organization, creating ones as
    /// needed, and record the mentions. Merging happens only on an exact
    /// normalised identifier; everything else gets its own provisional profile,
    /// so no mention is ever destroyed by a merge.
    ///
    /// The dedup runs in memory, not with a per-mention `SELECT` (issue 19): the
    /// old org lookup `WHERE country IS ? AND identifier_kind = ? AND identifier
    /// = ?` scanned the growing organizations table for *every* mention — O(n²),
    /// the projection's real bottleneck (44 min of a 54 min month at ~350k
    /// mentions). We preload the two lookup tables once, then each mention is an
    /// O(1) map hit and only genuinely new rows are written. Writes still commit
    /// in [`WRITE_BATCH`]-sized transactions with one doorbell at the end, so a
    /// change-feed consumer sees every new Organization exactly once; and the
    /// (notice, section) key keeps it idempotent (an already-recorded mention
    /// keeps its Organization — immutable evidence).
    pub async fn resolve_mentions(&self, mentions: &[Mention], now: i64) -> turso::Result<Vec<i64>> {
        use std::collections::HashMap;
        if mentions.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn().await;

        // Preload: the org dedup key (country, identifier_kind, identifier) → id,
        // and (notice_id, section_id) → org id for already-recorded mentions.
        // On `--rebuild` both tables were just cleared, so these scans are empty.
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
        drop(rows);
        let mut mention_of: HashMap<(i64, String), i64> = HashMap::new();
        let mut rows = conn
            .query("SELECT notice_id, section_id, organization_id FROM organization_mentions", ())
            .await?;
        while let Some(row) = rows.next().await? {
            mention_of.insert((int(&row, 0), text(&row, 1)), int(&row, 2));
        }
        drop(rows);

        let mut ids = Vec::with_capacity(mentions.len());
        let mut created_any = false;
        for chunk in mentions.chunks(WRITE_BATCH) {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            let mut chunk_ids = Vec::with_capacity(chunk.len());
            let mut error = None;
            for m in chunk {
                // The maps are updated as we go, so a later mention in the same
                // chunk reuses an Organization an earlier one just created.
                match self.resolve_one_mention(&conn, m, now, &mut org_of, &mut mention_of).await {
                    Ok((id, created)) => {
                        chunk_ids.push(id);
                        created_any |= created;
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
        if created_any {
            self.publish_cursor(&conn).await?;
        }
        Ok(ids)
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

    /// Reconcile one Tender's computed chain against what is stored, appending
    /// change rows for whatever actually differs. Re-running an unchanged
    /// projection writes nothing at all — that is the idempotency guarantee.
    pub async fn apply_tender(&self, p: &TenderProjection, now: i64) -> turso::Result<Applied> {
        self.apply_tenders(std::slice::from_ref(p), now).await
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
        for chunk in projections.chunks(WRITE_BATCH) {
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
    /// versions were re-added under the survivor by [`Self::apply_tender`], so
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

    /// Award-linkage per era (docs/research/ted-legacy-mapping.md §3): of the
    /// Tenders that carry an award (any `lot_results`), how many are a single
    /// notice — an award that never chained to its contract notice. The era is
    /// the profile of the Tender's first version's notice. Returns
    /// `(profile, award_tenders, unchained)` rows.
    pub async fn award_linkage(&self) -> turso::Result<Vec<(String, i64, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT n.profile,
                        COUNT(DISTINCT t.id) AS awards,
                        COUNT(DISTINCT CASE WHEN vc.versions = 1 THEN t.id END) AS unchained
                   FROM tenders t
                   JOIN (SELECT DISTINCT tender_id FROM lot_results) ar ON ar.tender_id = t.id
                   JOIN tender_versions v1 ON v1.tender_id = t.id AND v1.seq = 1
                   JOIN notices n ON n.id = v1.caused_by_notice_id
                   JOIN (SELECT tender_id, COUNT(*) AS versions FROM tender_versions GROUP BY tender_id) vc
                     ON vc.tender_id = t.id
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

    /// The change log from a cursor position — the poll/SSE/webhook feed.
    pub async fn changes_since(&self, cursor: i64, limit: i64) -> turso::Result<Vec<Change>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes
                  WHERE cursor > ? ORDER BY cursor LIMIT ?",
                (Value::Integer(cursor), Value::Integer(limit)),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(Change {
                cursor: int(&row, 0),
                entity_kind: text(&row, 1),
                entity_id: int(&row, 2),
                version_seq: match row.get_value(3) {
                    Ok(Value::Integer(i)) => Some(i),
                    _ => None,
                },
                op: text(&row, 4),
                changed_at: int(&row, 5),
            });
        }
        Ok(out)
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

/// The eight value tables, read back into their [`crate::NoticeValue`]
/// variants. Column order is (section_id, field_id, ordinal, …payload).
fn value_queries() -> Vec<(&'static str, ValueBuilder)> {
    use crate::NoticeValue as V;
    vec![
        (
            "SELECT section_id, field_id, ordinal, lang, value FROM notice_texts WHERE notice_id = ?",
            (|r| V::Text { lang: opt_text_of(r, 3), value: text(r, 4) }) as ValueBuilder,
        ),
        (
            "SELECT section_id, field_id, ordinal, list_name, code FROM notice_codes WHERE notice_id = ?",
            |r| V::Code { list: opt_text_of(r, 3), code: text(r, 4) },
        ),
        (
            "SELECT section_id, field_id, ordinal, scheme, code FROM notice_classifications WHERE notice_id = ?",
            |r| V::Classification { scheme: text(r, 3), code: text(r, 4) },
        ),
        (
            "SELECT section_id, field_id, ordinal, cents, currency FROM notice_amounts WHERE notice_id = ?",
            |r| V::Amount { cents: int(r, 3), currency: text(r, 4) },
        ),
        (
            "SELECT section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time
               FROM notice_dates WHERE notice_id = ?",
            |r| V::Date {
                utc_seconds: int(r, 3),
                offset_minutes: int(r, 4),
                has_time: int(r, 5) != 0,
            },
        ),
        (
            "SELECT section_id, field_id, ordinal, value FROM notice_integers WHERE notice_id = ?",
            |r| V::Integer(int(r, 3)),
        ),
        (
            "SELECT section_id, field_id, ordinal, value, unit FROM notice_numbers WHERE notice_id = ?",
            |r| V::Number {
                value: match r.get_value(3) {
                    Ok(Value::Real(f)) => f,
                    Ok(Value::Integer(i)) => i as f64,
                    _ => 0.0,
                },
                unit: opt_text_of(r, 4),
            },
        ),
        (
            "SELECT section_id, field_id, ordinal, scheme, value, is_ref FROM notice_ids WHERE notice_id = ?",
            |r| V::Id { scheme: opt_text_of(r, 3), value: text(r, 4), is_ref: int(r, 5) != 0 },
        ),
    ]
}

/// The batched form of [`value_queries`]: every value table read over a
/// `notice_id` window (`>= ?1 AND <= ?2`), in `notice_id` order, with
/// `notice_id` as column 0 and the payload shifted one column right. Used by
/// [`Db::parsed_chunk`] so the projection reads the notice layer in a handful of
/// scans per chunk rather than a per-notice query storm (issue 19).
fn all_value_queries() -> Vec<(&'static str, ValueBuilder)> {
    use crate::NoticeValue as V;
    vec![
        (
            "SELECT notice_id, section_id, field_id, ordinal, lang, value FROM notice_texts WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            (|r| V::Text { lang: opt_text_of(r, 4), value: text(r, 5) }) as ValueBuilder,
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, list_name, code FROM notice_codes WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Code { list: opt_text_of(r, 4), code: text(r, 5) },
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, scheme, code FROM notice_classifications WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Classification { scheme: text(r, 4), code: text(r, 5) },
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, cents, currency FROM notice_amounts WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Amount { cents: int(r, 4), currency: text(r, 5) },
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time
               FROM notice_dates WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Date { utc_seconds: int(r, 4), offset_minutes: int(r, 5), has_time: int(r, 6) != 0 },
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, value FROM notice_integers WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Integer(int(r, 4)),
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, value, unit FROM notice_numbers WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Number {
                value: match r.get_value(4) {
                    Ok(Value::Real(f)) => f,
                    Ok(Value::Integer(i)) => i as f64,
                    _ => 0.0,
                },
                unit: opt_text_of(r, 5),
            },
        ),
        (
            "SELECT notice_id, section_id, field_id, ordinal, scheme, value, is_ref FROM notice_ids WHERE notice_id >= ? AND notice_id <= ? ORDER BY notice_id",
            |r| V::Id { scheme: opt_text_of(r, 4), value: text(r, 5), is_ref: int(r, 6) != 0 },
        ),
    ]
}
