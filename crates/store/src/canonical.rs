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

use crate::{Db, Parsed, Section, ValueRow, int, opt_int, opt_text, t, text};
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
        UNIQUE(source, procedure_key),
        UNIQUE(island_notice_id)
    ) STRICT;

    -- One version per Notice, ordered by publication. `seq` is dense from 1 and
    -- is recomputed when a late-arriving Notice belongs mid-chain — the change
    -- log, not the version numbering, is the stable spine.
    CREATE TABLE IF NOT EXISTS tender_versions (
        tender_id           INTEGER NOT NULL REFERENCES tenders(id),
        seq                 INTEGER NOT NULL,
        caused_by_notice_id INTEGER NOT NULL REFERENCES notices(id),
        published_at        INTEGER NOT NULL, -- unix seconds
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

    -- The change cursor (docs/architecture.md). Ingestion order, never
    -- renumbered: re-projections append. `op` is diff-derived.
    CREATE TABLE IF NOT EXISTS changes (
        cursor      INTEGER PRIMARY KEY AUTOINCREMENT,
        entity_kind TEXT NOT NULL, -- tender | lot | organization
        entity_id   INTEGER NOT NULL,
        version_seq INTEGER,
        op          TEXT NOT NULL, -- added | changed | removed
        changed_at  INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX IF NOT EXISTS changes_entity ON changes(entity_kind, entity_id);

    -- ---------------------------------------------------------------- views
    -- Current state = the highest seq per Tender.

    CREATE VIEW IF NOT EXISTS v_tender_current AS
    SELECT tender_id, MAX(seq) AS seq FROM tender_versions GROUP BY tender_id;

    CREATE VIEW IF NOT EXISTS v_tenders AS
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

    CREATE VIEW IF NOT EXISTS v_lots AS
    SELECT l.id, l.tender_id, l.lot_key, vl.kind, vl.seq,
           (SELECT x.value FROM tender_version_texts x
             WHERE x.tender_id = l.tender_id AND x.seq = vl.seq
               AND x.lot_id = l.id AND x.field = 'title'
             ORDER BY (x.lang = 'ENG') DESC LIMIT 1) AS title
      FROM lots l
      JOIN v_tender_current c ON c.tender_id = l.tender_id
      JOIN tender_version_lots vl
        ON vl.tender_id = l.tender_id AND vl.seq = c.seq AND vl.lot_id = l.id;

    CREATE VIEW IF NOT EXISTS v_organizations AS
    SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
           (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id) AS mentions
      FROM organizations o;
";

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

/// One Tender version: the Notice that caused it plus the resolved state at
/// that point (this notice's values over the previous version's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenderVersion {
    pub caused_by_notice_id: i64,
    pub published_at: i64,
    pub notice_subtype: Option<String>,
    pub publication_id: String,
    pub facts: BTreeSet<Fact>,
    pub lots: Vec<LotState>,
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

/// A parsed Notice, as the projection addresses it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeRef {
    pub id: i64,
    pub source: String,
    pub publication_id: String,
}

impl Db {
    /// Every notice whose profile parser consumed it — the projection's input.
    pub async fn parsed_notices(&self) -> turso::Result<Vec<NoticeRef>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT id, source, publication_id FROM notices
                 WHERE parse_state = 'parsed' ORDER BY id",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(NoticeRef { id: int(&row, 0), source: text(&row, 1), publication_id: text(&row, 2) });
        }
        Ok(out)
    }

    /// Read one notice's parsed form back out of the notice layer — the exact
    /// [`Parsed`] the profile produced.
    pub async fn parsed_notice(&self, notice_id: i64) -> turso::Result<Parsed> {
        let conn = self.conn().await;
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

    /// Wipe the canonical layer's *content*, leaving the change log intact —
    /// what `project --rebuild` runs before re-deriving everything. The cursor
    /// is never renumbered (docs/architecture.md), so the rebuild appends.
    pub async fn clear_canonical(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        for table in [
            "tender_version_parties",
            "tender_version_texts",
            "tender_version_dates",
            "tender_version_amounts",
            "tender_version_classifications",
            "tender_version_lots",
            "tender_versions",
            "lots",
            "tenders",
            "organization_mentions",
            "organizations",
        ] {
            conn.execute(&format!("DELETE FROM {table}"), ()).await?;
        }
        Ok(())
    }

    /// Resolve a mention onto a canonical Organization, creating one when
    /// needed, and record the mention itself. Merging happens only on an exact
    /// normalised identifier; everything else gets its own provisional profile,
    /// so no mention is ever destroyed by a merge.
    pub async fn resolve_mentions(&self, mentions: &[Mention], now: i64) -> turso::Result<Vec<i64>> {
        if mentions.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let mut result = Ok(Vec::with_capacity(mentions.len()));
        for m in mentions {
            match self.resolve_mention_tx(&conn, m, now).await {
                Ok((id, _)) => result.as_mut().expect("still ok").push(id),
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        match result {
            Ok(ids) => {
                conn.execute("COMMIT", ()).await?;
                Ok(ids)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    async fn resolve_mention_tx(
        &self,
        conn: &Connection,
        m: &Mention,
        now: i64,
    ) -> turso::Result<(i64, bool)> {
        // An already-recorded mention keeps its organization: mentions are
        // immutable evidence, and re-projecting must not renumber them.
        let mut rows = conn
            .query(
                "SELECT organization_id FROM organization_mentions WHERE notice_id = ? AND section_id = ?",
                (Value::Integer(m.notice_id), t(&m.section_id)),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            return Ok((int(&row, 0), false));
        }

        let (org_id, created) = match &m.identifier {
            Some(id) => {
                let mut rows = conn
                    .query(
                        "SELECT id FROM organizations
                          WHERE country IS ? AND identifier_kind = ? AND identifier = ?",
                        (opt_text(id.country.as_deref()), t(&id.kind), t(&id.value)),
                    )
                    .await?;
                match rows.next().await? {
                    Some(row) => (int(&row, 0), false),
                    None => {
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
                        (last_insert_rowid(conn).await?, true)
                    }
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
        if created {
            append_change(conn, "organization", org_id, None, "added", now).await?;
        }
        Ok((org_id, created))
    }

    /// Reconcile one Tender's computed chain against what is stored, appending
    /// change rows for whatever actually differs. Re-running an unchanged
    /// projection writes nothing at all — that is the idempotency guarantee.
    pub async fn apply_tender(&self, p: &TenderProjection, now: i64) -> turso::Result<Applied> {
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = self.apply_tender_tx(&conn, p, now).await;
        match result {
            Ok(applied) => {
                conn.execute("COMMIT", ()).await?;
                Ok(applied)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
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
        let (sql, params): (&str, (Value, Value)) = match (&p.procedure_key, p.island_notice_id) {
            (Some(key), _) => (
                "SELECT id FROM tenders WHERE source = ? AND procedure_key = ?",
                (t(&p.source), t(key)),
            ),
            (None, Some(notice_id)) => (
                "SELECT id FROM tenders WHERE source = ? AND island_notice_id = ?",
                (t(&p.source), Value::Integer(notice_id)),
            ),
            (None, None) => unreachable!("a Tender is keyed by its procedure or by its island notice"),
        };
        let mut rows = conn.query(sql, params).await?;
        if let Some(row) = rows.next().await? {
            return Ok((int(&row, 0), false));
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
                 notice_subtype, publication_id)
             VALUES(?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(tender_id),
                Value::Integer(seq),
                Value::Integer(v.caused_by_notice_id),
                Value::Integer(v.published_at),
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
        Ok(())
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
            Some(prev) if prev.facts != v.facts => {
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
        Ok(count)
    }

    /// Apply many Tenders, summing what each one did.
    pub async fn apply_tenders(
        &self,
        projections: &[TenderProjection],
        now: i64,
    ) -> turso::Result<Applied> {
        let mut total = Applied::default();
        for p in projections {
            total.add(self.apply_tender(p, now).await?);
        }
        Ok(total)
    }

    /// `(rows, )` counts for the canonical layer — what the CLI and the
    /// dashboard report.
    pub async fn canonical_counts(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.conn().await;
        let mut out = Vec::new();
        for (label, sql) in [
            ("tenders", "SELECT COUNT(*) FROM tenders"),
            ("tenders (island)", "SELECT COUNT(*) FROM tenders WHERE procedure_key IS NULL"),
            ("tender_versions", "SELECT COUNT(*) FROM tender_versions"),
            ("lots", "SELECT COUNT(*) FROM lots"),
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
        let conn = self.conn().await;
        let mut rows = conn.query(sql, ()).await?;
        Ok(match rows.next().await? {
            Some(row) => row.get_value(0).ok(),
            None => None,
        })
    }

    /// The change log from a cursor position — the poll/SSE/webhook feed.
    pub async fn changes_since(&self, cursor: i64, limit: i64) -> turso::Result<Vec<Change>> {
        let conn = self.conn().await;
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
    let mut rows = conn.query("SELECT last_insert_rowid()", ()).await?;
    Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
}

fn opt_text_of(row: &turso::Row, idx: usize) -> Option<String> {
    match row.get_value(idx) {
        Ok(Value::Text(s)) => Some(s),
        _ => None,
    }
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
