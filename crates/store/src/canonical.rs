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
use crate::{Db, Parsed, Section, ValueRow, int, opt_int, opt_int_of, opt_text, opt_text_of, t, text};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use turso::{Connection, Statement, Value};

/// Peak RSS of a bulk `CREATE INDEX`, per row of the table being indexed — the
/// LARGEST measured, not the typical one.
///
/// Four points on the deployed turso 0.7.0, from run-driver:
///
/// | rows | index | B/row |
/// |---|---|---|
/// | 8,132,478 | `organizations(country, id)` | 45 |
/// | 25,289,344 | `organizations(country, id)` | 45 |
/// | (tenders) | `tenders(source, id)` | 41 |
/// | 14,240,000 | `notices(source, id)` | **48** |
///
/// This constant was first set to 45, which two consecutive measurements agreed on —
/// and 45 turned out to be CENTRAL rather than an upper bound. Taking a
/// repeated sample for a bound is the same error as taking a measurement to license a
/// conclusion about something it did not vary; the fourth point exceeded it and would
/// have made every cap derived from it optimistic by 7%.
///
/// So it is now the measured maximum, and it must be RAISED — never averaged — the
/// moment a wider key measures above it. The margin deliberately lives in
/// [`AUTO_INDEX_MEMORY_BUDGET`] (half the ceiling) rather than being padded into this
/// number, so the two stay separable: this is an empirical fact about turso, that is
/// a policy choice about how much of the box a build may use.
const INDEX_BUILD_BYTES_PER_ROW: i64 = 48;

/// What one auto-triggered index build may consume. This is the POLICY half of the
/// cap (how much of the box a build may use); [`INDEX_BUILD_BYTES_PER_ROW`] is the
/// empirical half and never moves without a turso measurement.
///
/// Raised 2026-08-15 from 2 GB to 12 GB after the box was measured at **62 GB RAM,
/// no cgroup limit, `TMPDIR=/data/tmp` on a 1.7 TB disk** (issue 205). The old 2 GB
/// was "half the project's ~4 GB ceiling on a box carrying issue 57's swap
/// band-aid" — a relic of the smaller box (that box is gone; this one has 59 GB free
/// and no swap band-aid, and the external-sort spill lands on disk, not tmpfs). At
/// 12 GB the cap covers the corpus's largest satellite (`tender_version_classifications`,
/// 138.9M rows → 6.67 GB at 48 B/row) with headroom, while a single build stays under
/// ~19% of the box — builds are sequential (see the `Reindex` job), so peaks never sum.
/// This is the flag that was silently dropping six read-path indexes at the 41M cap
/// and, worse, forcing the schema batch to rebuild five of them blocking at the next
/// `Db::open` — a ~22-minute unserved boot after every rebuild (issue 205 P1/P2).
const AUTO_INDEX_MEMORY_BUDGET: i64 = 12_000_000_000;

// The one part of the size cap that IS a compile-time fact, and so is checked as one.
//
// The cap itself must stay a RUNTIME check: the hazard is a row count, and row counts
// are not known at compile time — a static allowlist of "small" tables would encode
// today's judgement and go stale the moment one grew, which is exactly how
// `notices(source, id)` ended up in the schema batch on the strength of a comment
// written when that table was 8x smaller.
//
// But the cap's DERIVATION is static: raising `MAX_AUTO_INDEX_ROWS` without
// re-deriving it against the measured bytes-per-row is a compile error, not a
// production discovery. That is the half that never leaves the branch.
const _: () = assert!(
    Db::MAX_AUTO_INDEX_ROWS * INDEX_BUILD_BYTES_PER_ROW <= AUTO_INDEX_MEMORY_BUDGET,
    "MAX_AUTO_INDEX_ROWS exceeds the auto-build memory budget at the measured \
     bytes-per-row: a build at that size would sort past the bounded-memory ceiling. \
     Re-derive the cap against INDEX_BUILD_BYTES_PER_ROW, or raise the budget only \
     with a new measurement behind it."
);

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
        -- The head version's submission deadline (issue 216, deadline half) —
        -- fold-maintained like the published pointer above, backfilled once
        -- (job 706). Also ALTERed in by migrate() for pre-216 files; it lives
        -- here too so a fresh file and reset_tender_layer's recreate agree.
        current_deadline     INTEGER,
        -- The head version's title (issue 239), fold-maintained like the pointers
        -- above. It exists so `v_tenders` can SELECT a column instead of running a
        -- correlated subquery per row: that subquery blocked view flattening, so the
        -- planner could not push a filter down and the view materialised the whole
        -- corpus for every query — a primary-key point read measured >10s.
        current_title        TEXT,
        -- Which PROJECTION LOGIC this Tender's content was last folded under
        -- (issue 99). The fold's early-return keys on the chain of causing
        -- notices, which is a state key only while the logic is fixed: a mapping
        -- change makes the same chain yield different content, and the unchanged
        -- chain then skips it. Issue 85 left 2,185 factless shells that way and
        -- issue 98 would have written zero parties. Stamped on every rewrite; a
        -- mismatch forces one.
        projection_epoch     INTEGER NOT NULL DEFAULT 0
        -- A procedure key is globally unique across Sources: a TED eForms
        -- procedure and its DÖE twin share one BT-04 UUID and must collapse into
        -- one Tender (ADR-0003), and legacy `ojs:` keys are TED-only, so the key
        -- alone identifies the Tender. `source` is the primary Source label
        -- (TED where the procedure appears on both).
        --
        -- Identity is served by PLAIN (non-unique) named indexes, not inline
        -- UNIQUE auto-indexes: uniqueness is guaranteed by construction — a
        -- rebuild assigns one distinct group_key per group (the identity probe is
        -- skipped, reset_tender_layer having emptied the layer first), and the
        -- incremental path probes these indexes before inserting — while turso
        -- hangs building UNIQUE indexes at scale (issue 62). The inline UNIQUEs
        -- were also the random-key probe+maintenance storm that steepened the
        -- Phase-2 fold (issue 60/62, here on the tender side).
        --
        -- Those named indexes (`tenders_procedure_key`, `tenders_island`) are NOT
        -- created here: a schema-batch CREATE INDEX runs at every Db::open and would
        -- build over prod's millions of existing tenders on open (the hang that hit
        -- organizations_identity). They live only in DEFERRED_TENDER_INDEXES, built
        -- once by build_tender_indexes at a rebuild's end; the incremental probe
        -- uses whatever the last rebuild left in place.
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

    -- Which lots a LotsGroup contains, as this version publishes it (issue 237).
    --
    -- Version-keyed like every other satellite, because composition is a published
    -- fact that a corrigendum can restate: the group is a Lot row of kind 'LotsGroup'
    -- and each member an ordinary Lot, so both ends are `lots` ids and this table adds
    -- no identity of its own.
    --
    -- Why it exists: eForms gives a Bid exactly ONE lot reference (BT-13714-Tender),
    -- pointing at either a Lot or a LotsGroup, so a bid covering several lots names the
    -- GROUP. Without this table we know a bid covers GLO-0002 and not which lots that
    -- is, and every per-lot rollup silently omits combined-award bids.
    CREATE TABLE IF NOT EXISTS tender_version_lot_group_members (
        tender_id     INTEGER NOT NULL,
        seq           INTEGER NOT NULL,
        group_lot_id  INTEGER NOT NULL REFERENCES lots(id),
        member_lot_id INTEGER NOT NULL REFERENCES lots(id),
        PRIMARY KEY (tender_id, seq, group_lot_id, member_lot_id),
        FOREIGN KEY (tender_id, seq) REFERENCES tender_versions(tender_id, seq)
    ) STRICT;
    -- Answers the reverse question (which groups is this lot in), the direction a
    -- per-lot rollup needs.
    CREATE INDEX IF NOT EXISTS tender_version_lot_group_members_member
        ON tender_version_lot_group_members(member_lot_id);

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
        -- Whether the figure includes tax: 'incl' | 'excl' | NULL when the source did
        -- not say (issue 251). Nullable and unlabelled-by-default on purpose: the
        -- corpus has always mixed the two bases in this column, because the text era
        -- states its basis in prose and the r208/r209 eras publish a VAT indicator that
        -- is not mapped. Before this column a reader could not tell an inclusive figure
        -- from an exclusive one, and a sum over the column was meaningless in a way
        -- nothing warned about. NULL still means unknown — but now unknown reads as
        -- unknown instead of as agreement.
        tax_basis TEXT,
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
        -- Unicode-lowercased `name` (issue 217-B): the case-insensitive
        -- name-prefix search seeks `(name_norm, id)`, because turso 0.7 accepts
        -- COLLATE NOCASE index DDL but only ever SCANS such an index (measured,
        -- name_prefix_probe.rs). Written by the resolver in Rust (`to_lowercase`
        -- — SQL lower() is ASCII-only and would break umlauts); backfilled by the
        -- batched `backfill-org-names` job on a pre-existing DB.
        name_norm       TEXT,
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
        -- When the buyer DECIDED (issue 255, slice 2). The legacy form eras put
        -- this on the award block itself (`CONTRACT_AWARD_DATE` inside
        -- `AWARD_OF_CONTRACT`) and have no contract graph to hang it on, so it
        -- lives here rather than beside eForms' contract-scoped BT-1451.
        decided_utc      INTEGER,
        decided_offset   INTEGER,
        decided_has_time INTEGER,
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
    -- date (BT-145), the winner-decision date (BT-1451), and the value of the
    -- winning Bid(s) it settled (BT-3202 → BT-720 — eForms contracts carry no
    -- value of their own).
    --
    -- The two dates are different facts and both are published: BT-1451 is when
    -- the buyer DECIDED, BT-145 is when the contract was signed. Every committed
    -- eForms CAN fixture carries both, days apart (issue 255).
    CREATE TABLE IF NOT EXISTS tender_version_contracts (
        tender_id          INTEGER NOT NULL,
        seq                INTEGER NOT NULL,
        contract_id        INTEGER NOT NULL REFERENCES contracts(id),
        buyer_contract_id  TEXT,
        concluded_utc      INTEGER,
        concluded_offset   INTEGER,
        concluded_has_time INTEGER,
        decided_utc        INTEGER,
        decided_offset     INTEGER,
        decided_has_time   INTEGER,
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

    -- Durable projection control (salvage-loop fix). `rebuild_in_progress` is 1
    -- while a full rebuild's Phase-2 is mid-flight: set together with
    -- `reset_tender_layer` (the moment a rebuild commits to emptying the layer),
    -- cleared with `clear_plan` on clean completion. The resume-from-plan salvage
    -- keys on THIS flag, NOT on a complete grouping plan being present on disk --
    -- a complete plan is ALSO left by a finished build (whose plan was retired) or an
    -- interrupted rebuild=false full-fallback over an intact layer, and resuming
    -- those re-nukes a good 6.96M-tender layer on every restart (a livelock).
    -- Single row, id pinned to 0.
    CREATE TABLE IF NOT EXISTS projection_state (
        id                  INTEGER PRIMARY KEY CHECK (id = 0),
        rebuild_in_progress INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    INSERT OR IGNORE INTO projection_state(id, rebuild_in_progress) VALUES (0, 0);

    -- The change feed's generation (issue 46). Bumped whenever the canonical
    -- layer or the change log is wiped for a rebuild: events across a bump do
    -- not compose into one coherent state (entity ids are reissued, `added`
    -- rows for dead ids linger with no `removed`), so a client that sees the
    -- generation move must drop its state and re-snapshot. Single row, id 0.
    CREATE TABLE IF NOT EXISTS feed_generation (
        id         INTEGER PRIMARY KEY CHECK (id = 0),
        generation INTEGER NOT NULL DEFAULT 1
    ) STRICT;
    INSERT OR IGNORE INTO feed_generation(id, generation) VALUES (0, 1);

    -- issue 58 v2: the DURABLE legacy OJS adjacency — every OJS key a legacy
    -- notice touches (its own number and every REF_OJS edge, unified: the
    -- closure treats them identically). Written where the plan already has them
    -- (insert_plan), so it cannot drift from the grouping's own reading; parse-
    -- derived, so a canonical rebuild does not invalidate it. The incremental
    -- fold may replace its issue-58-v1 full-corpus fallback with a closure walk
    -- over this table ONLY when legacy_adjacency.watermark attests coverage
    -- (every parsed notice with id <= watermark has its rows here); 0 = never
    -- established, and the fallback then behaves exactly as v1.
    CREATE TABLE IF NOT EXISTS legacy_ojs_keys (
        ojs_key   INTEGER NOT NULL,
        notice_id INTEGER NOT NULL,
        PRIMARY KEY (ojs_key, notice_id)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS legacy_ojs_keys_notice ON legacy_ojs_keys(notice_id);
    CREATE TABLE IF NOT EXISTS legacy_adjacency (
        id        INTEGER PRIMARY KEY CHECK (id = 0),
        watermark INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    INSERT OR IGNORE INTO legacy_adjacency(id, watermark) VALUES (0, 0);

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
    -- `title` reads a COLUMN, not a per-row correlated subquery (issue 239). The old
    -- shape computed it inline with an ORDER BY over two computed expressions, which
    -- blocked view flattening: the planner could not merge the view into the caller's
    -- query, so it could not push the caller's filter down, so every query — even
    -- `WHERE id = <pk>` — materialised essentially the whole corpus. Measured on prod:
    -- >10 s for a point read, and identical cost for `id < 100` and `id < 5000`, which
    -- is what proved the filter was doing nothing.
    --
    -- The precedence that subquery encoded now lives in `head_title`, applied by the
    -- fold when it writes the head pointer.
    CREATE VIEW v_tenders AS
    SELECT t.id, t.source, t.procedure_key, t.kind,
           v.seq, v.published_at, v.caused_by_notice_id, v.notice_subtype, v.publication_id,
           t.current_title AS title
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
           s.decided_utc, s.decided_offset, s.decided_has_time,
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
           r.decided_utc, r.decided_offset, r.decided_has_time,
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
    SELECT t.id AS tender_id, a.lot_id, a.field, a.cents, a.currency, a.tax_basis
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
/// The version of the projection LOGIC that produced a Tender's stored content
/// (issue 99).
///
/// `apply_tender_tx` skips a Tender whose chain of causing notices is unchanged,
/// on the stated assumption that "the projection is deterministic, so the sequence
/// of causing notices is the state key". That holds only while the logic is fixed.
/// A mapping change makes the SAME chain yield different content, so the chain
/// stops being a state key and the new content is silently discarded — issue 85
/// left 2,185 factless shells exactly that way, and issue 98 would have written
/// zero party rows at all. Every rewrite stamps this; a stored value that differs
/// forces a rewrite the chain alone would have skipped.
///
/// **Bump this on any change to the projection's mapping or fold logic** — the
/// alias tables, `canonical_name`, `NoticeState::read`, `read_results`, `fold`.
/// Not for changes that only affect grouping (those change the chain, which the
/// existing check already catches) and not for read-path or performance work.
///
/// **A bump MUST be paired with a SCOPED refold.** The rewrite set is the
/// `projected = 0` marking, never the epoch, so a profile-scoped `refold` rewrites
/// only that cohort. A bump plus a whole-corpus refold would re-emit version change
/// events for all ~8.1M Tenders — the feed is append-only, so that noise is
/// permanent. Scope the refold to the profiles the logic change actually touched.
///
/// Bumping is human discipline, and the coupling that partly guards it has a known
/// hole: `project_golden` turns red when fold output moves, but its corpus contains
/// no eForms-DE 1.x notice, so a DE-only change — issue 98 exactly — would not have
/// tripped it. Closing that needs a DE-1.x golden fixture (see the issue-99
/// follow-up), which is also what would give the DE path multi-notice fold-order
/// coverage.
///
/// | epoch | change |
/// |---|---|
/// | 1 | issue 98 — DE-1.x organization references (`is_ref` + 25 role aliases) |
/// | 2 | issue 174 — r208 `RECEIPT_LIMIT_DATE` maps to `submission_deadline`, so the 2011–2016 era re-folds with deadlines |
/// | 3 | issue 177 — r208 `VALUE_COST` routes by context (CN estimate / award value / CAN final total), so the era re-folds with `estimated_value`/`result_value` |
///
/// **Bump this ONLY for a logic change that crosses profiles** (issue 179). A
/// profile-scoped mapping fix — the 174/177 class, historically every bump —
/// must instead ship WITHOUT touching this constant and ride the `refold` job,
/// which now stamps scoped staleness ([`Db::stamp_stale_for_profiles`]): the
/// cohort's tenders rewrite under the new logic, every other tender keeps its
/// early-return. A global bump declares 7.9M tenders stale to fix one era and
/// owes the whole corpus a rewrite on the next full walk — measured at
/// 6h02m / 14.2M version writes for a 2.69M-notice cohort (issue 179).
///
/// NOT bumped for issue 234's identifier-less mention merge, and the reasoning
/// is worth keeping because the bump was made and then REVERTED after its first
/// live no-op: the resolver's `(notice, section)` idempotency preload returns
/// the already-recorded Organization for every existing mention, so re-folding
/// a stored chain produces byte-identical output — stored chains remain valid
/// state keys, which is precisely the condition under which this constant must
/// NOT move. The merge changes only mentions that have never been recorded
/// (fresh ingests, re-parses — which is also why the fresh-DB golden legitimately
/// changed). Collapsing the EXISTING provisional rows is a separate backfill
/// concern on issue 234, not a fold concern.
pub const PROJECTION_EPOCH: i64 = 3;

const NODE_WRITE_BATCH: usize = 20_000;

/// How often the ADR-0011 previous-notice pass reports how far it has read (issue 256).
///
/// A DURATION, not a row count. On a full-corpus plan this step has gone silent for
/// over two hours with no way — short of `ps -o pcpu` — to tell a grinding join from an
/// empty edge set, which is what the heartbeat was added for. But it counted rows the
/// query RETURNS, not references in the plan, and the survivor count is not knowable in
/// advance: the first 50,000-row interval printed nothing at all on the 245,955-edge run
/// of 2026-08-20, so the silence still looked exactly like a hang and I read it as one
/// for ten minutes. A clock cannot be silent.
const PREV_EDGE_HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(15);

/// The ADR-0011 previous-notice join, named so its query plan can be asserted against
/// the statement the fold actually runs rather than a copy that drifts from it
/// (`the_previous_notice_join_seeks_notices_rather_than_scanning_it`). `plan_prev_edge`
/// carries `a_source` precisely so this lookup can seek `notices` by the leading
/// columns of its `UNIQUE(source, publication_id, content_hash)` index; that was an
/// assumption in a comment until the test made it checkable. It is the step that pins
/// one core for the better part of an hour on a full re-projection (issue 256 part 2).
pub(crate) const PREV_EDGE_JOIN_SQL: &str = "SELECT a.group_key, a.published_at, a.publication_id, \
        b.group_key, b.published_at, b.publication_id \
   FROM plan_prev_edge e \
   JOIN notices n ON n.source = e.a_source \
                 AND n.publication_id = e.b_publication_id \
   JOIN plan_notice a ON a.notice_id = e.a_notice_id \
   JOIN plan_notice b ON b.notice_id = n.id \
  WHERE a.group_key IS NOT NULL AND b.group_key IS NOT NULL \
    AND b.published_at < a.published_at \
    AND a.group_key <> b.group_key";

/// The org-merge backfill's batch scan (issue 234's second half). The resolver
/// merge only PREVENTS new duplicate identifier-less Organizations; this scan
/// feeds the job that collapses the standing stock. Ordered by the
/// `organizations_name_country` index so duplicate `(name_norm, country)`
/// groups arrive adjacent and the whole walk is ONE pass over the index — the
/// grouping happens in Rust on the streamed rows, never in a sorter. The
/// cursor is `name_norm > ?`: `''` both starts the walk and excludes empty
/// names, which (with `country IS NOT NULL`) is exactly the resolver probe's
/// scope. Gated by an EXPLAIN QUERY PLAN test against THIS constant, per the
/// planner's record on composite-index access (issues 239/248/256).
pub(crate) const ORG_MERGE_SCAN_SQL: &str = "SELECT id, name_norm, country FROM organizations \
  WHERE identifier IS NULL AND country IS NOT NULL AND name_norm > ? \
  ORDER BY name_norm, country LIMIT ?";

/// `notice_id`-range width for the batched keyed/island `group_key` UPDATE. A
/// single whole-corpus UPDATE writes a WAL frame PER ROW (turso has no truncate
/// optimisation), one uncheckpointable statement that balloons the in-RAM
/// WAL-index to OOM at 14M rows (issue 63). Splitting it into `notice_id` ranges
/// with a TRUNCATE between keeps the WAL bounded; each row's key is a pure
/// function of its own columns, so the split is byte-identical to the one-shot.
const GROUP_KEY_UPDATE_BATCH: i64 = 200_000;

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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Fact {
    Text { field: String, lang: Option<String>, value: String },
    Amount { field: String, cents: i64, currency: String, tax_basis: Option<String> },
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

/// The head version's submission deadline (issue 216): the LATEST
/// `submission_deadline` date across the version's tender-level AND lot-level
/// facts — exactly the value the list row's `pick` reads back out of
/// `tender_version_dates` (`ORDER BY utc_seconds DESC LIMIT 1` over all rows of
/// the version, lot rows included), computed here from the in-memory facts so the
/// head update pays no extra query. `None` when the version publishes no deadline
/// (award notices), which the read layer treats as "not in the deadline
/// ordering".
pub fn head_deadline(head: &TenderVersion) -> Option<i64> {
    head.facts
        .iter()
        .chain(head.lots.iter().flat_map(|l| l.facts.iter()))
        .filter_map(|f| match f {
            Fact::Date { field, utc_seconds, .. } if field == "submission_deadline" => {
                Some(*utc_seconds)
            }
            _ => None,
        })
        .max()
}

/// The title to denormalise onto `tenders.current_title` (issue 239).
///
/// Reproduces exactly the precedence `v_tenders` used to compute per row with a
/// correlated subquery: **the Tender's own title wins over a lot's**, and within a
/// scope an `ENG` variant wins over another language. The lot fallback is not a nicety
/// — many notices title their lots and never the procedure, so without it those
/// Tenders would read as untitled (the reason the subquery had that ORDER BY at all).
///
/// Ties beyond that are resolved by first-seen, which is stable because `facts` is a
/// `BTreeSet`: two ENG titles at the same scope is not a shape the corpus should
/// publish, and picking deterministically beats picking arbitrarily.
pub fn head_title(head: &TenderVersion) -> Option<String> {
    let pick = |facts: &mut dyn Iterator<Item = &Fact>| -> Option<(bool, String)> {
        facts
            .filter_map(|f| match f {
                Fact::Text { field, lang, value } if field == "title" => {
                    Some((lang.as_deref() == Some("ENG"), value.clone()))
                }
                _ => None,
            })
            .max_by_key(|(is_eng, _)| *is_eng)
    };
    pick(&mut head.facts.iter())
        .or_else(|| pick(&mut head.lots.iter().flat_map(|l| l.facts.iter())))
        .map(|(_, value)| value)
}

/// A Lot as one version publishes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotState {
    pub key: String,
    pub kind: String,
    pub facts: BTreeSet<Fact>,
}

/// The entity ids one Tender's rewritten chain references — fed by
/// [`Db::write_version`] as it resolves them, consumed by the shrinking-rewrite
/// sweep (issue 103). Collected in memory rather than re-derived from the
/// satellites afterwards, because the satellite probes are not indexed for that
/// shape: `tender_version_bids`' primary key is `(tender_id, seq, bid_id)` — the
/// two columns a per-entity probe needs are separated by `seq`, so they are not
/// a usable prefix (the trap issue 248 already caught this engine in) — and
/// `tender_version_bid_parties` carries no index that could serve it at all.
#[derive(Default)]
struct WrittenEntities {
    lots: std::collections::HashSet<i64>,
    lot_results: std::collections::HashSet<i64>,
    bids: std::collections::HashSet<i64>,
    contracts: std::collections::HashSet<i64>,
}

/// One result notice's contribution to the results layer — a "round" in the
/// framework/DPS sense (ted-empirical-checks.md §3). Rounds are *additive*:
/// a later version never supersedes an earlier round's results (the verified
/// tranche pattern), except that a correction — a change notice republishing
/// the same logical notice — replaces the round it corrects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotResultState {
    pub key: String,
    /// The referenced lot id as published (BT-13713) — resolved defensively,
    /// because FA/DPS rounds relabel lots per round.
    pub lot_key: Option<String>,
    pub decision: Option<String>,
    pub reason: Option<String>,
    pub awarded_cents: Option<i64>,
    pub awarded_currency: Option<String>,
    /// When the buyer decided: (utc seconds, offset minutes, has_time). The
    /// legacy eras' `CONTRACT_AWARD_DATE`, which sits on the award block itself
    /// (issue 255). eForms states the same fact on its settled contract, where
    /// it lands on [`ContractState::decided`] instead.
    pub decided: Option<(i64, i64, bool)>,
    /// Winning Organization ids, resolved through the notice's own graph.
    pub winners: Vec<i64>,
    /// (received-submission-type code, count), from BT-759/BT-760.
    pub statistics: Vec<(String, i64)>,
}

/// A Bid (eForms LotTender, TEN-): one offer on one Lot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BidState {
    pub key: String,
    pub lot_key: Option<String>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
    pub parties: Vec<BidParty>,
}

/// One Organization behind a Bid — the eForms TenderingParty flattened.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BidParty {
    pub role: String, // tenderer | subcontractor
    pub organization_id: i64,
    /// The ORG- section in the origin notice — the mention evidence.
    pub section_id: String,
}

/// A settled Contract (eForms SettledContract, CON-).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractState {
    pub key: String,
    pub buyer_contract_id: Option<String>,
    /// BT-145 conclusion date: (utc seconds, offset minutes, has_time).
    pub concluded: Option<(i64, i64, bool)>,
    /// BT-1451 winner-decision date, in the same shape (issue 255). A separate
    /// fact from [`ContractState::concluded`] — the decision precedes the
    /// signature, by 1 day in `can-maximal-sdk17` and by 47 in
    /// `can-subdesc-00570953-2025`.
    pub decided: Option<(i64, i64, bool)>,
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
    /// `(group lot key, member lot key)` pairs this version publishes (issue 237),
    /// read from the notice's `GroupComposition` sections. Empty for the vast majority
    /// of notices, which declare no lots group at all.
    pub group_members: Vec<(String, String)>,
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

/// One batch of the issue-234 org-merge backfill: what moved, and where the
/// name-cursor walk stands. Totals are summed across batches by the job.
#[derive(Debug, Default, Clone)]
pub struct OrgMergeBatch {
    /// In-scope org rows this batch actually processed (post name-alignment).
    pub scanned: u64,
    /// Duplicate `(name_norm, country)` groups collapsed.
    pub groups: u64,
    /// Loser Organization rows deleted.
    pub removed: u64,
    /// `organization_mentions` rows repointed to survivors.
    pub mentions: u64,
    /// `tender_version_parties` rows repointed.
    pub parties: u64,
    /// `tender_version_bid_parties` rows repointed.
    pub bid_parties: u64,
    /// `tender_version_result_winners` rows repointed.
    pub winners: u64,
    /// Winner rows DELETED because their lot_result already named the survivor
    /// (the PK ends in `organization_id`, so the repoint would collide).
    pub winner_dups: u64,
    /// Distinct Tenders that had a party/bid-party/winner row repointed and so got
    /// a `tender` `changed` change-feed row (issue 286): the merge rewrites org
    /// references in place on current-version rows, changing what the `buyer` /
    /// `winner` / `bidder` org-role filters return, and this is the event that tells
    /// a `/v1/changes` or SSE subscriber to re-evaluate the Tender.
    pub tender_changes: u64,
    /// `name_norm` watermark: pass as `after` to continue the walk.
    pub cursor: String,
    /// The scan ran out of rows — the walk is complete.
    pub done: bool,
}

/// What applying a projection did — the numbers the CLI reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub tenders_created: u64,
    pub versions_written: u64,
    pub versions_removed: u64,
    pub changes: u64,
    /// Tenders whose chain CHANGED — the write path ran (issue 108). Together
    /// with `tenders_unchanged` this makes the fold's two exits countable:
    /// `projected = 1` means "considered by a fold", and these two say which
    /// way each consideration went. A G2 breach reads differently under
    /// `written 0 / unchanged N` (the watermark over-claims — issue 105's
    /// marked-without-a-row route) than under a large `written` (the fold
    /// itself misbehaved), and before this split that diagnosis needed someone
    /// who knew both mechanisms reading code under time pressure.
    pub tenders_written: u64,
    /// Entity rows (lots/lot_results/bids/contracts) deleted by the shrinking-
    /// rewrite sweep (issue 103): rows no surviving version references after a
    /// full rewrite produced FEWER entities than the stored chain had. Zero on
    /// every run whose projection logic did not narrow a mapping.
    pub entities_swept: u64,
    /// Tenders verified current by the unchanged-chain early return (same
    /// epoch, same causing-notice sequence) — considered, decided, zero writes.
    pub tenders_unchanged: u64,
}

impl Applied {
    /// Accumulate another batch's tally — the streaming projection applies in
    /// bounded batches and sums their reports (issue 57).
    pub fn add(&mut self, other: Applied) {
        self.tenders_created += other.tenders_created;
        self.versions_written += other.versions_written;
        self.versions_removed += other.versions_removed;
        self.changes += other.changes;
        self.tenders_written += other.tenders_written;
        self.tenders_unchanged += other.tenders_unchanged;
        self.entities_swept += other.entities_swept;
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
    /// Issue 234: identifier-LESS mentions used to mint a fresh provisional
    /// Organization every time — measured at 95.30 % of a 24.6M-row table, with
    /// one review body alone holding 51,388 rows in a single 200k-id window. Now
    /// a mention with a non-empty name AND a country reuses the first Organization
    /// with that `(name_norm, country)`. This map is the RUN's cache of those
    /// lookups, filled lazily (a probe or an insert adds one entry), so a name
    /// seen a thousand times in one run costs one indexed SELECT — it cannot be
    /// preloaded like `org_of`, because 24.6M keys do not sit in RAM (issue 57).
    name_of: std::collections::HashMap<(String, String), i64>,
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
    /// Normalised `publication_id`s this notice names as its predecessors
    /// (`OPP-090-Procedure`, ADR-0011). Stored as an edge per reference in
    /// `plan_prev_edge`, resolved to a group after the keyed/island and legacy
    /// passes have given every notice a `group_key`.
    pub prev_refs: Vec<String>,
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
        Self::parsed_chunk_on(&conn, after_id, i64::MAX, limit).await
    }

    /// As [`Db::parsed_chunk`], but through an explicit connection and bounded to
    /// notice ids `≤ hi`. The bound lets the sharded Phase-2 pre-pass (issue 66)
    /// give each worker its own reader connection and its own contiguous id stripe
    /// `(after_id, hi]`; the un-sharded [`Db::parsed_chunk`] passes `hi = i64::MAX`.
    pub async fn parsed_chunk_on(
        conn: &Connection,
        after_id: i64,
        hi: i64,
        limit: i64,
    ) -> turso::Result<Vec<(NoticeRef, Parsed)>> {
        Self::parsed_chunk_inner(conn, after_id, hi, limit, false).await
    }

    /// As [`Db::parsed_chunk`], restricted to LEGACY-profile notices — the
    /// issue-58-v2 adjacency backfill's read: it sweeps only the notices whose
    /// keys the choke-point writer would record, skipping the (larger) eForms
    /// half of the corpus entirely.
    /// `hi` caps the ID RANGE the query may scan, not just the rows it returns
    /// (issue 228). The legacy pre-filter means `limit` alone cannot stop a query
    /// early in a legacy-free range — it scans to the end of the table to prove
    /// no match remains — so the caller windows the range and the two bounds
    /// together keep both memory and per-query I/O bounded.
    pub async fn legacy_parsed_chunk(
        &self,
        after_id: i64,
        hi: i64,
        limit: i64,
    ) -> turso::Result<Vec<(NoticeRef, Parsed)>> {
        let conn = self.reader().await?;
        Self::parsed_chunk_inner(&conn, after_id, hi, limit, true).await
    }

    async fn parsed_chunk_inner(
        conn: &Connection,
        after_id: i64,
        hi: i64,
        limit: i64,
        legacy_only: bool,
    ) -> turso::Result<Vec<(NoticeRef, Parsed)>> {
        let mut out: Vec<(NoticeRef, Parsed)> = Vec::new();
        let mut slot: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

        // The legacy predicate mirrors `is_legacy_profile` (project.rs) exactly;
        // it is a pre-filter only — the sweep re-derives legacy per notice in
        // Rust, so an over-match costs a read, never a wrong row.
        let legacy = if legacy_only {
            " AND (profile = 'text' OR profile = 'internal-ojs' OR profile LIKE 'ted-export%')"
        } else {
            ""
        };
        let sql = format!(
            "SELECT id, source, publication_id, profile FROM notices
             WHERE parse_state = 'parsed' AND id > ? AND id <= ?{legacy} ORDER BY id LIMIT ?"
        );
        let mut rows = conn
            .query(&sql, (Value::Integer(after_id), Value::Integer(hi), Value::Integer(limit)))
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
        // The chunk's contiguous id window: a ranged scan of each value table
        // over [lo, hi] yields every member's rows (plus, under the legacy
        // filter, interlopers' rows — dropped by the `slot` lookup below;
        // legacy ids sit in dense historical ranges, so the waste is small).
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
        let conn = self.reader().await?;
        Self::mentions_by_ids_on(&conn, ids).await
    }

    /// As [`Db::mentions_by_ids`], but through an explicit connection — the sharded
    /// Phase-2 pre-pass (issue 66) resolves each worker's mentions on the worker's
    /// own reader.
    pub async fn mentions_by_ids_on(
        conn: &Connection,
        ids: &[i64],
    ) -> turso::Result<std::collections::HashMap<i64, std::collections::HashMap<String, i64>>> {
        use std::collections::HashMap;
        let mut out: HashMap<i64, HashMap<String, i64>> = HashMap::new();
        if ids.is_empty() {
            return Ok(out);
        }
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
    ///
    /// `organizations`/`organization_mentions` are NOT cleared here: both callers
    /// (the `project` rebuild and `project_plan_only`) invoke
    /// [`Db::strip_organization_indexes`] immediately after, which DROPs+recreates
    /// those two tables bare. A DROP is O(1) WAL; a `DELETE FROM` over the ~28M-row
    /// org layer writes a WAL frame PER ROW (turso has no truncate optimisation,
    /// measured ~240 B/row → multi-GB), a single uncheckpointed statement that
    /// balloons the in-RAM WAL-index to OOM on a full-corpus rebuild (issue 63).
    /// Clearing them here was pure redundant work in front of the DROP.
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
            "tender_version_lot_group_members",
        "tender_version_lots",
            "tender_versions",
            "lot_results",
            "bids",
            "contracts",
            "lots",
            "tenders",
        ] {
            // DROP+recreate, NOT DELETE (issue 63): over the tens-of-millions-of-row
            // tender-content tables a per-row DELETE balloons the WAL to OOM. Same
            // fix as reset_tender_layer; the fresh path must not balloon either.
            Self::drop_and_recreate(&conn, table).await?;
        }
        // Reset the incremental-projection watermark (issue 58): a full rebuild
        // re-derives everything, so every parsed notice must be re-folded. Over a
        // fully-projected corpus this touches ~14M rows, so batch it by id range
        // with a TRUNCATE between (issue 63) — a single whole-corpus UPDATE writes
        // a WAL frame per row and balloons the in-RAM WAL-index to OOM.
        let (min_id, max_id) = {
            let mut r = conn
                .query("SELECT MIN(id), MAX(id) FROM notices WHERE projected <> 0", ())
                .await?;
            match r.next().await? {
                Some(row) => (opt_int_of(&row, 0), opt_int_of(&row, 1)),
                None => (None, None),
            }
        };
        // The wipe makes every previously-issued entity id and change event
        // incomposable with what the rebuild will emit — new generation
        // (issue 46). Bumped BEFORE the re-derivation starts, so a client that
        // polls mid-rebuild already sees the move.
        conn.execute("UPDATE feed_generation SET generation = generation + 1 WHERE id = 0", ())
            .await?;
        if let (Some(min_id), Some(max_id)) = (min_id, max_id) {
            let mut lo = min_id;
            while lo <= max_id {
                let hi = lo.saturating_add(GROUP_KEY_UPDATE_BATCH - 1).min(max_id);
                conn.execute(
                    "UPDATE notices SET projected = 0 WHERE projected <> 0 AND id BETWEEN ? AND ?",
                    (Value::Integer(lo), Value::Integer(hi)),
                )
                .await?;
                let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
                lo = hi.saturating_add(1);
            }
        }
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

    /// The `(entity_kind, cursor)` index that lets an entity-filtered change query
    /// (`/v1/changes?entity=…`, the SSE diff loop) seek to its kind's rows in cursor
    /// order instead of walking the whole `changes` table (issue 61 finding 2).
    /// Built here — at the END of a projection, alongside `ensure_unprojected_index`
    /// — NOT in the schema batch, deliberately: a `CREATE INDEX` over the 80M-row
    /// `changes` table at `Db::open` would re-introduce the very multi-minute slow
    /// boot issue 61 just removed. `IF NOT EXISTS`, so it builds once then no-ops.
    pub async fn ensure_changes_entity_cursor_index(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS changes_entity_cursor ON changes(entity_kind, cursor)",
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

    /// Parsed notices in `profiles` still marked projected — exactly the set
    /// [`Db::unmark_projected_for_profiles`] would re-queue. Counted first so a caller
    /// can refuse a cohort that is not the size it expected BEFORE anything is
    /// written: the failure mode being guarded is a mistyped profile string matching a
    /// far larger set, whose re-fold would be a corpus-wide surprise (issue 85).
    pub async fn projected_notice_count_for_profiles(&self, profiles: &[&str]) -> turso::Result<u64> {
        if profiles.is_empty() {
            return Ok(0);
        }
        let conn = self.conn().await;
        let sql = format!(
            "SELECT COUNT(*) FROM notices
              WHERE parse_state = 'parsed' AND projected <> 0 AND profile IN ({})",
            placeholders(profiles.len())
        );
        let params: Vec<Value> = profiles.iter().map(|p| t(*p)).collect();
        let mut rows = conn.query(&sql, params).await?;
        Ok(match rows.next().await? {
            Some(row) => int(&row, 0).max(0) as u64,
            None => 0,
        })
    }

    /// Re-queue a profile cohort for the incremental fold (issue 85): clear the
    /// `projected` watermark for its parsed notices so the next `project rebuild=false`
    /// re-derives just those Tenders. The parsed layer is untouched — this is for a
    /// cohort the projection MIS-READ (unmapped field ids), not one that was parsed
    /// wrong, so no re-parse is needed.
    ///
    /// Batched by id range with a TRUNCATE between, like [`Db::clear_canonical`]: a
    /// single cohort-wide UPDATE writes a WAL frame per row and balloons the in-RAM
    /// WAL-index (issue 63). Returns the number of notices re-queued.
    /// Age every Tender's stored projection epoch — **tests only** (issue 99).
    ///
    /// Production never needs this: bumping [`PROJECTION_EPOCH`] in code makes every
    /// stored value stale by definition. The epoch-invariance gates need to drive the
    /// branch directly, and forcing them through a runtime-injectable epoch would
    /// mean making a compile-time constant configurable in production purely to be
    /// testable — a worse trade than one narrow, clearly-labelled writer.
    pub async fn set_projection_epoch_for_test(&self, epoch: i64) -> turso::Result<u64> {
        let conn = self.conn().await;
        conn.execute("UPDATE tenders SET projection_epoch = ?", (Value::Integer(epoch),)).await
    }

    /// Stamp the Tenders whose chains a profile cohort caused as EPOCH-STALE
    /// (issue 179): `projection_epoch = 0`, the pre-epoch "oldest" value, so the
    /// next fold's chain-compare keeps nothing for THEM and rewrites them under
    /// the current logic — without bumping the global [`PROJECTION_EPOCH`], which
    /// declares every unrelated Tender stale too and owes the whole corpus a
    /// rewrite on the next full walk (the issue-179 half-day, measured: 7.9M
    /// tenders / 14.2M versions rewritten for a 2.69M-notice cohort).
    ///
    /// The requeue alone ([`Db::unmark_projected_for_profiles`]) cannot do this:
    /// re-queueing clears the watermark but leaves each Tender's CHAIN identical,
    /// and an unchanged chain with a current epoch early-returns — the mapping fix
    /// would silently never land (the issue-85 shells, issue 99's motivation). So
    /// the refold job runs both: requeue puts the cohort in the delta, this stamp
    /// makes the fold rewrite it. A tender with a MIXED chain (cohort + other
    /// profiles) is stamped too — its whole chain refolds, which is what planning
    /// does with it anyway.
    ///
    /// Batched with checkpoints like every bulk UPDATE (issue 63); idempotent, so
    /// a job re-run after a crash heals. Returns the number of Tenders stamped.
    pub async fn stamp_stale_for_profiles(&self, profiles: &[&str]) -> turso::Result<u64> {
        if profiles.is_empty() {
            return Ok(0);
        }
        let conn = self.conn().await;
        // The cohort's tender ids: notices by profile (notices_profile) resolved
        // through the causing edge (tender_versions_notice). Deduped in Rust — a
        // SQL DISTINCT over millions of rows would sort server-side for nothing.
        let sql = format!(
            "SELECT v.tender_id FROM notices n
               JOIN tender_versions v ON v.caused_by_notice_id = n.id
              WHERE n.profile IN ({})",
            placeholders(profiles.len())
        );
        let params: Vec<Value> = profiles.iter().map(|p| t(*p)).collect();
        let mut rows = conn.query(&sql, params).await?;
        let mut ids: Vec<i64> = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(int(&row, 0));
        }
        ids.sort_unstable();
        ids.dedup();

        Self::stamp_tenders_stale(&conn, ids).await
    }

    /// The shared batched stamp: `projection_epoch = 0` over a deduped tender-id
    /// list, checkpointed per the issue-63 WAL discipline. Callers differ only in
    /// how they derive the cohort (profile join, notice-id join).
    async fn stamp_tenders_stale(conn: &Connection, mut ids: Vec<i64>) -> turso::Result<u64> {
        ids.sort_unstable();
        ids.dedup();
        const STAMP_BATCH: usize = 500;
        for (i, chunk) in ids.chunks(STAMP_BATCH).enumerate() {
            let sql = format!(
                "UPDATE tenders SET projection_epoch = 0 WHERE id IN ({})",
                placeholders(chunk.len())
            );
            let params: Vec<Value> = chunk.iter().map(|id| Value::Integer(*id)).collect();
            conn.execute(&sql, params).await?;
            if i % 100 == 99 {
                let _ = checkpoint_on(conn, CheckpointMode::Truncate).await;
            }
        }
        let _ = checkpoint_on(conn, CheckpointMode::Truncate).await;
        Ok(ids.len() as u64)
    }

    /// The parsed notices whose value layer carries any of `field_ids` — the
    /// FIELD-scoped cohort a mapping fix affects (issue 88's follow-up), where the
    /// profile-scoped refold would over-fold whole profiles for a handful of
    /// carriers. Sweeps `notice_amounts` and `notice_texts` (the two channels the
    /// issue-88 mappings use — extend the table list when a mapped id needs
    /// another channel) in notice-id windows that ride each table's
    /// `(notice_id, …)` primary key, so each statement is bounded and progress is
    /// visible. `field_id` itself is unindexed — this is a full pass over both
    /// tables by design; run it as a queued job in a quiet window, never inline.
    pub async fn notice_ids_carrying_fields(&self, field_ids: &[&str]) -> turso::Result<Vec<i64>> {
        if field_ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn().await;
        let max_id = {
            let mut rows = conn.query("SELECT MAX(id) FROM notices", ()).await?;
            match rows.next().await? {
                Some(row) => opt_int_of(&row, 0).unwrap_or(0),
                None => 0,
            }
        };
        const WINDOW: i64 = 500_000;
        let list = placeholders(field_ids.len());
        let mut ids: Vec<i64> = Vec::new();
        for table in ["notice_amounts", "notice_texts"] {
            let sql = format!(
                "SELECT DISTINCT notice_id FROM {table}
                  WHERE notice_id >= ? AND notice_id < ? AND field_id IN ({list})"
            );
            let mut lo = 0i64;
            while lo <= max_id {
                let hi = lo.saturating_add(WINDOW);
                let mut params: Vec<Value> = vec![Value::Integer(lo), Value::Integer(hi)];
                params.extend(field_ids.iter().map(|f| t(*f)));
                let mut rows = conn.query(&sql, params).await?;
                while let Some(row) = rows.next().await? {
                    ids.push(int(&row, 0));
                }
                if lo % 5_000_000 == 0 {
                    eprintln!("[store] field sweep {table}: {lo}/{max_id}, {} carriers", ids.len());
                }
                lo = hi;
            }
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    /// The parsed notices carrying a section of any of these KINDS — the cohort a
    /// mapping keyed on a section kind affects (issue 237).
    ///
    /// The cheap sibling of [`Db::notice_ids_carrying_fields`], and the reason it exists:
    /// that one sweeps whole value tables because `field_id` is unindexed, which is
    /// minutes of I/O. `notice_sections` has `notice_sections_kind`, so a kind-scoped
    /// cohort is an index range read instead — 11,416 `GroupComposition` sections came
    /// back in 15 ms on prod, against a full pass over `notice_ids` for the same answer.
    ///
    /// Prefer this whenever the mapping's trigger is a section kind rather than a field
    /// id: several past cohorts (the sibling-mount classes, sdk-0.1's party kinds) were
    /// exactly that shape.
    pub async fn notice_ids_with_section_kind(&self, kinds: &[&str]) -> turso::Result<Vec<i64>> {
        if kinds.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn().await;
        let sql = format!(
            "SELECT DISTINCT s.notice_id FROM notice_sections s
               JOIN notices n ON n.id = s.notice_id
              WHERE s.kind IN ({}) AND n.parse_state = 'parsed'",
            placeholders(kinds.len())
        );
        let params: Vec<Value> = kinds.iter().map(|k| t(*k)).collect();
        let mut rows = conn.query(&sql, params).await?;
        let mut ids: Vec<i64> = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(int(&row, 0));
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    /// Re-queue an explicit notice-id cohort for the incremental fold — the
    /// by-ids twin of [`Db::unmark_projected_for_profiles`], for cohorts a
    /// profile cannot name (issue 88's field carriers). Only parsed, currently
    /// projected rows are touched; returns how many actually re-queued.
    pub async fn unmark_projected_by_ids(&self, ids: &[i64]) -> turso::Result<u64> {
        let conn = self.conn().await;
        let mut requeued = 0u64;
        for (i, chunk) in ids.chunks(500).enumerate() {
            let sql = format!(
                "UPDATE notices SET projected = 0
                  WHERE parse_state = 'parsed' AND projected <> 0 AND id IN ({})",
                placeholders(chunk.len())
            );
            let params: Vec<Value> = chunk.iter().map(|id| Value::Integer(*id)).collect();
            requeued += conn.execute(&sql, params).await?;
            if i % 100 == 99 {
                let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
            }
        }
        let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
        Ok(requeued)
    }

    /// Stamp the Tenders whose chains a NOTICE-ID cohort caused as epoch-stale —
    /// the by-ids twin of [`Db::stamp_stale_for_profiles`], same rationale: the
    /// requeue alone leaves chains identical and the fold early-returns, so the
    /// mapping fix never lands (issues 85/99/179).
    pub async fn stamp_stale_for_notices(&self, notice_ids: &[i64]) -> turso::Result<u64> {
        if notice_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn().await;
        // tender_versions_notice (caused_by_notice_id) serves each chunk's probe.
        let mut ids: Vec<i64> = Vec::new();
        for chunk in notice_ids.chunks(500) {
            let sql = format!(
                "SELECT tender_id FROM tender_versions WHERE caused_by_notice_id IN ({})",
                placeholders(chunk.len())
            );
            let params: Vec<Value> = chunk.iter().map(|id| Value::Integer(*id)).collect();
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                ids.push(int(&row, 0));
            }
        }
        Self::stamp_tenders_stale(&conn, ids).await
    }

    pub async fn unmark_projected_for_profiles(&self, profiles: &[&str]) -> turso::Result<u64> {
        let requeued = self.projected_notice_count_for_profiles(profiles).await?;
        if requeued == 0 {
            return Ok(0);
        }
        let conn = self.conn().await;
        let list = placeholders(profiles.len());
        // Bound the id walk to the cohort itself, so a cohort clustered in one id
        // range costs a few batches rather than a walk over the whole notices table.
        let (min_id, max_id) = {
            let sql = format!(
                "SELECT MIN(id), MAX(id) FROM notices
                  WHERE parse_state = 'parsed' AND projected <> 0 AND profile IN ({list})"
            );
            let params: Vec<Value> = profiles.iter().map(|p| t(*p)).collect();
            let mut rows = conn.query(&sql, params).await?;
            match rows.next().await? {
                Some(row) => (opt_int_of(&row, 0), opt_int_of(&row, 1)),
                None => (None, None),
            }
        };
        if let (Some(min_id), Some(max_id)) = (min_id, max_id) {
            let sql = format!(
                "UPDATE notices SET projected = 0
                  WHERE parse_state = 'parsed' AND projected <> 0 AND profile IN ({list})
                    AND id BETWEEN ? AND ?"
            );
            let mut lo = min_id;
            while lo <= max_id {
                let hi = lo.saturating_add(GROUP_KEY_UPDATE_BATCH - 1).min(max_id);
                let mut params: Vec<Value> = profiles.iter().map(|p| t(*p)).collect();
                params.push(Value::Integer(lo));
                params.push(Value::Integer(hi));
                conn.execute(&sql, params).await?;
                let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
                lo = hi.saturating_add(1);
            }
        }
        Ok(requeued)
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
                 identifier TEXT, name TEXT NOT NULL, name_norm TEXT,
                 provisional INTEGER NOT NULL, created_at INTEGER NOT NULL
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
    /// Builds `organizations_identity` as a PLAIN (non-unique) index. Nothing
    /// relies on a DB UNIQUE constraint for org identity: the dedup is the in-RAM
    /// `org_of` map (the run's authority) and there is no `INSERT … ON CONFLICT` on
    /// organizations anywhere, so a plain index serves every lookup identically —
    /// while a UNIQUE build over the NULLable identity columns is the cross-corpus
    /// uniqueness-CHECK that HUNG prod startup at ~30M orgs (issue 62, the same
    /// random-key CREATE-INDEX-at-scale pathology Task-1 removed from the tenders
    /// identity keys). CRITICAL for the resume cutover: `strip_organization_indexes`
    /// recreates `organizations` BARE (no inline UNIQUE) and the salvage loads it
    /// full in Phase-1 but only builds indexes at a completed fold's end — so a
    /// resume whose Phase-2 never finished reaches here with a bare-but-FULL table.
    /// A unique build there would wedge; a plain one is bulk-load-safe (measured).
    /// Still skipped when the table carries a pre-issue-62 inline
    /// `UNIQUE(country, identifier_kind, identifier)` — its auto-index already
    /// serves the lookup, so a redundant named index is pointless.
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
                "CREATE INDEX IF NOT EXISTS organizations_identity
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
        // Issue 117: `(filter, id)` indexes for the paginated `/v1/organizations`
        // reads, so `WHERE <filter> = ? AND id > ? ORDER BY id LIMIT ?` uses BOTH the
        // filter and the cursor as index bounds and no sorter runs.
        //
        // `organizations_identity(country, identifier_kind, identifier)` cannot serve
        // either read: its trailing column is not `id`, so seeking `country` yields the
        // slice in identifier order and `ORDER BY id` must sort all of it before
        // `LIMIT` — measured at 15.16s against 0.0001s for `?country=DE&limit=50` over
        // prod's 3.85M-row DE slice. And `identifier_kind` is its SECOND column, so a
        // kind-only listing cannot seek it at all (the 99.08s case). Both are fixed by
        // an index that leads with the filter and ends with `id`; the reads are
        // unchanged. Measured in `crates/store/tests/paginated_index_probe.rs`: the
        // dense case is unchanged and the absent case collapses to nothing.
        //
        // ISSUE 111 APPLIES: this function runs at a rebuild's end or via the `Reindex`
        // admin job, and at no other time. Deploying the code does not create them.
        for (name, cols) in Self::DEFERRED_ORG_INDEXES {
            if let Some(rows) = Self::too_large_to_build(&conn, cols).await? {
                eprintln!(
                    "store: REFUSING to auto-build {name}: {cols} has ~{rows} rows, over the                      {} row cap — a bulk CREATE INDEX there would sort ~{} GB (issue 111).                      Build it index-first at a rebuild, or in a maintenance window.",
                    Self::MAX_AUTO_INDEX_ROWS,
                    rows * 45 / 1_000_000_000,
                );
                continue;
            }
            conn.execute(&format!("CREATE INDEX IF NOT EXISTS {name} ON {cols}"), ()).await?;
        }
        Ok(())
    }

    /// The organization indexes a healthy database always has, as ONE list shared by
    /// the builder above and [`Db::missing_deferred_indexes`]. Two copies of this
    /// list would be a detector that can silently stop matching what is built — the
    /// artifact-versus-proxy failure of issues 110 and 102, in a new place.
    ///
    /// `organizations_identity` is deliberately absent: it is built only when the
    /// table lacks the inline UNIQUE, so a database that HAS the inline constraint
    /// legitimately lacks the index and must not be reported as missing.
    const DEFERRED_ORG_INDEXES: [(&'static str, &'static str); 7] = [
        ("organization_mentions_org", "organization_mentions(organization_id)"),
        // Issue 247: the re-parse's clear deletes a notice's mentions, and that single
        // statement was 99.6% of a re-parse's writer time — 153 ms per notice, measured
        // by `tender_db_reparse_clear_statement_seconds_total` on prod, against
        // microseconds for every other statement in the same clear.
        //
        // `organization_mentions` already declares `PRIMARY KEY (notice_id, section_id)`,
        // so this index looks redundant. It is not, and the measurement says why: the
        // DELETE does not use the implicit composite-PK index and scans all 41.78M rows
        // instead — including for a notice with ZERO mentions, which is the case that
        // proves it is scanning rather than deleting. A SELECT with the identical
        // predicate answers in 0.8 ms, so reads seek and this write did not.
        //
        // The two party tables' deletes in that same clear cost 14 ms and 8 ms IN TOTAL
        // over 555 notices, and the only thing they have that this lacked is an explicit
        // single-column index on the predicate (`tender_version_parties_mention`, added
        // by issue 100 for exactly this reason). Do not "tidy" this away as duplicated
        // by the primary key.
        ("organization_mentions_notice", "organization_mentions(notice_id)"),
        ("organizations_country_id", "organizations(country, id)"),
        ("organizations_kind_id", "organizations(identifier_kind, id)"),
        // Issue 217: look up an Organization by its official identifier value (a VAT
        // number, say). Leads with `identifier` and ends with `id` so the paginated
        // `WHERE identifier=? AND id>? ORDER BY id LIMIT` seeks the value and rides the
        // cursor on one index with no sorter — the same (filter, id) shape the country
        // and kind listings use. `missing_deferred_indexes` will report it absent until
        // a `Reindex` builds it (auto-enqueued on open, or run on demand).
        ("organizations_identifier_id", "organizations(identifier, id)"),
        // Issue 217-B: the case-insensitive name-prefix search's index. Leads with
        // the normalised name so the prefix range seeks; ends with `id` so the
        // (name_norm, id) keyset cursor rides the same index (the 216 shape).
        ("organizations_name_norm_id", "organizations(name_norm, id)"),
        // Issue 234: the identifier-less mention merge's probe. `resolve_one_mention`
        // looks up `(name_norm, country)` before minting a provisional Organization;
        // the existing `(name_norm, id)` index can seek the name but then filters
        // country row by row, which is fine for a rare name and a scan for
        // `tribunal administratif` at 1,500 rows. Leads with the name (the selective
        // column); country completes the key.
        ("organizations_name_country", "organizations(name_norm, country)"),
    ];

    /// The notice indexes that are deferred rather than schema-batch.
    ///
    /// `notices(source, id)` (issue 117) started in the schema batch, next to
    /// `notices_fetch_id`, because `notices` is never dropped by a rebuild so a
    /// schema-batch index is durable and needs no operator step. Moved here once the
    /// build was measured: run-driver clocked `CREATE INDEX` at **413 s over 25.3M
    /// rows on the real 441 GB file**, so ~27.4M notices is ~7 minutes — and the
    /// schema batch runs inside `Db::open`, which would make that a SEVEN-MINUTE
    /// BLOCKING BOOT on the deploy restart. That is precisely the start-up regression
    /// issues 82/83 removed, and `IF NOT EXISTS` only makes it once rather than never.
    ///
    /// `notices_fetch_id`'s comment estimates "tens of seconds" for its own build on
    /// 3.5M rows, which is where the schema-batch placement was reasonable; at 27.4M
    /// it no longer is. The estimate did not scale, and nobody re-checked it — the
    /// reason this one was measured instead of reasoned by analogy.
    const DEFERRED_NOTICE_INDEXES: [(&'static str, &'static str); 2] = [
        ("notices_source_id", "notices(source, id)"),
        // Issue 217-A: the official notice number is the primary external key a
        // consumer holds, and without this index its lookup walks (the `ORDER BY id
        // LIMIT` pagination drives off the id PK and FILTERs — ~10 s measured on
        // prod, isolated pool). Leads with `publication_id` and ends with `id`, the
        // same (filter, id) shape as `notices_source_id` and the org listing indexes,
        // so `WHERE publication_id = ? AND id > ? ORDER BY id LIMIT ?` seeks the value
        // and rides the cursor with no sorter. Routing stays ISOLATED until the built
        // index is measured serving on prod — de-isolating on the assumption an index
        // helps is exactly the 88d876a regression (issue 217's first deploy).
        ("notices_publication_id_id", "notices(publication_id, id)"),
    ];

    /// Build the deferred notice indexes. Separate from the org and tender builders
    /// because `notices` has neither's lifecycle: it is never dropped by a rebuild, so
    /// these are build-once rather than rebuild-after-fold.
    pub async fn build_notice_indexes(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        for (name, cols) in Self::DEFERRED_NOTICE_INDEXES {
            if let Some(rows) = Self::too_large_to_build(&conn, cols).await? {
                eprintln!(
                    "store: REFUSING to auto-build {name}: {cols} has ~{rows} rows, over the                      {} row cap — a bulk CREATE INDEX there would sort ~{} GB (issue 111).                      Build it index-first at a rebuild, or in a maintenance window.",
                    Self::MAX_AUTO_INDEX_ROWS,
                    rows * 45 / 1_000_000_000,
                );
                continue;
            }
            conn.execute(&format!("CREATE INDEX IF NOT EXISTS {name} ON {cols}"), ()).await?;
        }
        Ok(())
    }

    /// The largest table an index may be auto-built over, in rows.
    ///
    /// A bulk `CREATE INDEX` over a populated table sorts the whole table and its peak
    /// RSS is LINEAR in row count — run-driver measured ~45 bytes/row on the deployed
    /// turso 0.7.0 (366 MiB at 8.13M rows, 1.07 GB at 25.3M, no spill threshold
    /// between them). It cannot be batched: `CREATE INDEX` has no range knob, and N
    /// partial indexes over disjoint id ranges do not compose into one usable index
    /// (a partial index serves only queries whose `WHERE` implies its predicate, so a
    /// filter with no id bound would use none of them).
    ///
    /// So the only protection is not to start a build that will not fit. At ~45 B/row
    /// this cap is ~2 GB of peak RSS — half the project's ~4 GB bounded-memory ceiling,
    /// leaving room for the rest of the process on a box still carrying issue 57's
    /// swap band-aid. Today's largest member, `organization_mentions` at 40.9M rows,
    /// passes; `changes` at 93.6M (~4 GB alone) would not, and is deliberately in
    /// neither deferred list — its indexes are built index-first in the schema DDL or
    /// at projection end, which is bounded (24 MiB measured) because maintaining an
    /// index during insert never sorts the whole table.
    ///
    /// Enforced at build time rather than as a compile-time assertion because the
    /// hazard is a ROW COUNT, and row counts are not compile-time facts. A static
    /// allowlist of table names would encode today's judgement about which tables are
    /// small, and go stale silently the moment one grows — which is precisely the
    /// mistake that put `notices(source, id)` in the schema batch on the strength of a
    /// comment written when the table was 8x smaller.
    /// 240M x 48 B/row = 11.52 GB, inside the 12 GB budget. The compile-time assertion
    /// below forces this recalculation whenever either number moves, instead of leaving
    /// the two to drift.
    ///
    /// Raised 2026-08-15 from 41M (issue 205). The old 41M cap's own WATCH ITEM had
    /// called this: `organization_mentions` was at 40.9M — 99.8% of the cap — and "any
    /// growth trips it, and then its index refuses to auto-build ... a decision someone
    /// should make deliberately rather than discover from a stderr line during a
    /// rebuild." It grew to 41.27M and did exactly that, and it dragged FIVE more
    /// satellites over with it (classifications 138.9M, result_winners 127.8M, parties
    /// 77.3M, bid_parties 66.8M ×2). Five of those are also in the schema batch, so the
    /// next `Db::open` rebuilt them BLOCKING — a ~22-minute unserved boot. Raising the
    /// budget to match the real 62 GB box (see [`AUTO_INDEX_MEMORY_BUDGET`]) lets the
    /// rebuild's own end-of-fold builder build them all instead — a sequential
    /// background job, memory-safe — so nothing is left for the boot to rebuild and no
    /// read-path index is left permanently unbuildable. 240M covers today's largest
    /// (138.9M) with 1.7x headroom.
    const MAX_AUTO_INDEX_ROWS: i64 = 240_000_000;

    /// Refuse to bulk-build an index over a table too large to sort within the memory
    /// budget. Returns the offending row estimate, or `None` if the build may proceed.
    ///
    /// Uses `MAX(rowid)`, which is an O(1) index seek to the end rather than a
    /// `COUNT(*)` scan — and an OVER-estimate once rows have been deleted, so it errs
    /// toward refusing. Erring toward refusing is right: a refused index leaves a read
    /// slow and says so, while an accepted one that does not fit takes the process
    /// down mid-build, and turso has no way to interrupt a running statement.
    async fn too_large_to_build(conn: &Connection, cols: &str) -> turso::Result<Option<i64>> {
        let Some(table) = cols.split('(').next().map(str::trim) else { return Ok(None) };
        let mut rows = conn.query(&format!("SELECT MAX(rowid) FROM {table}"), ()).await?;
        let estimate = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        Ok((estimate > Self::MAX_AUTO_INDEX_ROWS).then_some(estimate))
    }

    /// Which deferred indexes are absent from this database.
    ///
    /// Issue 111: the deferred indexes have no guaranteed builder. They are created at
    /// a rebuild's end or by the `Reindex` admin job and at no other time, so a deploy
    /// that ADDS one leaves it uncreated — the read it exists for stays slow until
    /// somebody notices and fires a job by hand. That is how the issue-117 DoS fix
    /// would ship without taking effect.
    ///
    /// This is the detection half: cheap enough to run at every boot (a scan of
    /// `sqlite_master`, which holds one row per object, not per row of data), so the
    /// caller can enqueue the existing `Reindex` job in the BACKGROUND rather than
    /// building anything inline. Building at boot is what issues 82/83 removed, and
    /// this must not bring it back.
    pub async fn missing_deferred_indexes(&self) -> turso::Result<Vec<String>> {
        let conn = self.conn().await;
        let mut present = std::collections::HashSet::new();
        let mut rows = conn
            .query("SELECT name FROM sqlite_master WHERE type = 'index'", ())
            .await?;
        while let Some(row) = rows.next().await? {
            present.insert(text(&row, 0));
        }
        Ok(Self::DEFERRED_TENDER_INDEXES
            .iter()
            .chain(Self::DEFERRED_ORG_INDEXES.iter())
            .chain(Self::DEFERRED_NOTICE_INDEXES.iter())
            .map(|(name, _)| *name)
            .filter(|name| !present.contains(*name))
            .map(str::to_owned)
            .collect())
    }

    /// The tender satellite indexes whose keys are RANDOM across the corpus —
    /// `organization_id` (millions of orgs), CPV `code`, `published_at`, the
    /// causing `notice_id`, and the two `tenders` identity keys (a rebuild inserts
    /// tenders in fold order, so `procedure_key`/`island_notice_id` land at random
    /// b-tree positions) — as opposed to the append-mostly `(tender_id, seq)`
    /// indexes. Maintaining these live during a from-scratch Phase-2 fold is a
    /// random-position b-tree write storm past the page cache (the issue-60/62
    /// thrash, here on the tender side — the `tenders` identity probe+maintenance is
    /// exactly what steepened the fold). They are dropped before the fold and
    /// rebuilt once, sorted, at the end. All PLAIN (non-unique), so the bulk build
    /// is the measured-safe kind — not the org-identity NULL-unique hang (issue 62);
    /// the identity indexes are non-unique because a rebuild's group_keys are
    /// distinct by construction and the incremental probe guards otherwise.
    const DEFERRED_TENDER_INDEXES: [(&'static str, &'static str); 16] = [
        ("tender_versions_published", "tender_versions(published_at)"),
        ("tender_versions_notice", "tender_versions(caused_by_notice_id)"),
        // Issue 217-A: `/v1/tenders?publication_id=` seeds its FROM with "the
        // tenders whose versions this number caused" (`tender_from`); this makes
        // that seed an index-only seek. `tender_id` last covers the seed's SELECT.
        ("tender_versions_publication", "tender_versions(publication_id, tender_id)"),
        ("tender_version_classifications_code", "tender_version_classifications(scheme, code)"),
        ("tender_version_parties_org", "tender_version_parties(organization_id)"),
        // Issue 100: the RE-PARSE path's delete key. Clearing a notice's parsed layer
        // has to clear the party rows anchored to its mentions first (they carry an FK
        // onto `organization_mentions`), and without an index on the mention's notice
        // that DELETE is a full scan of this table PER NOTICE — survivable for a
        // 31-notice cohort, fatal for the 218,876-notice one the re-parse exists to
        // serve. Leads with `mention_notice_id` because that is the whole predicate.
        ("tender_version_parties_mention", "tender_version_parties(mention_notice_id)"),
        ("tender_version_bid_parties_mention", "tender_version_bid_parties(mention_notice_id)"),
        // Issue 225: the buyer reverse-lookup's covering index. The seed
        // (`participation_seed`) narrows `tender_version_parties` by organization AND
        // buyer role; this index serves that whole shape index-only, so a ubiquitous
        // NON-buyer party (org 2: 112k service-provider rows; org 3: 1.79M review-body
        // rows, 3 actual buyer rows — measured) collapses to its buyer slice instead of
        // hauling every mention through per-row table lookups (~17-19 s, measured).
        // `role` is mid-index so the org's slice scans index-only even though the
        // `%Buyer%` match is not a prefix; `tender_id` last makes the seed covered.
        ("tender_version_parties_org_role", "tender_version_parties(organization_id, role, tender_id)"),
        ("tender_version_result_winners_org", "tender_version_result_winners(organization_id)"),
        ("tender_version_bid_parties_org", "tender_version_bid_parties(organization_id)"),
        // The by-version index every other satellite carries — bid_parties was the
        // one left without one when `tender_version_parties_version` closed the same
        // gap for `parties`. `tender_detail` reads it by `(tender_id, seq)` like all
        // its siblings (read.rs:794), and the table has NO primary key and no other
        // index, so without this every `/v1/tenders/{id}` full-scans it. Deferred
        // rather than added to the schema batch: a schema-batch CREATE INDEX would
        // build it over the whole table at every `Db::open`, which is the multi-hour
        // boot issue 82/83 just removed. `(tender_id, seq)` is append-mostly rather
        // than random-key, so it is here for the boot-path reason, not the issue-60 one.
        ("tender_version_bid_parties_version", "tender_version_bid_parties(tender_id, seq)"),
        ("tenders_procedure_key", "tenders(procedure_key)"),
        ("tenders_island", "tenders(source, island_notice_id)"),
        // The newest-Tenders list's covering index (issue 25; issue 82). `migrate()`
        // creates it too, but only at process open — and `reset_tender_layer` DROPs the
        // `tenders` table, so a rebuild loses it and the list falls to a full scan until
        // the next boot rebuilds it. `current_published_at` is random in fold order, so it
        // belongs here — dropped before the fold, rebuilt once sorted at the end.
        ("tenders_current_published", "tenders(current_published_at, id)"),
        // Issue 216 (deadline half): the twin of tenders_current_published for the
        // deadline ordering — "what closes soon" seeks this, range + cursor +
        // ORDER BY on one index. The column is fold-maintained and was backfilled
        // by job 706 (7.92M rows), so the index is correct from its first build.
        ("tenders_current_deadline", "tenders(current_deadline, id)"),
        // Issue 117: the paginated reads' `(filter, id)` indexes. Each one exists so
        // that `WHERE <filter> = ? AND id > ? ORDER BY id LIMIT ?` can use BOTH the
        // filter and the cursor as index bounds — the shape `tenders_current_published`
        // above and `changes_entity_cursor` (issue 61) already have.
        //
        // Without them the read has no good plan for both densities, only a choice of
        // which density to be bad at. The plain cursor walks in rowid order and stops
        // at `LIMIT` matches: fast when the filter is dense, a full table walk when it
        // matches nothing or only late — measured on prod at 22.0s (`?country=ZZ`),
        // 99.08s (`?kind=`) and 226s+ (`?source=`), unauthenticated. Rewriting the
        // cursor as a row value inverts it: turso then seeks the EXISTING index, whose
        // trailing column is not `id`, so `ORDER BY id` sorts the whole matched slice
        // before `LIMIT` — measured at 15.16s against 0.0001s for `?country=DE&limit=50`
        // over prod's 3.85M-row DE slice, a 151,648x regression on the ORDINARY query.
        // These indexes end in `id`, so the seek yields id order, `LIMIT` truncates
        // immediately, and no sorter runs. Measured: the dense case is unchanged and
        // the absent case collapses to nothing (crates/store/tests/paginated_index_probe.rs,
        // org_cursor_probe.rs). The reads themselves are NOT changed — that is the point.
        //
        // Only the `tenders` one lives here, because only `tenders` is dropped and
        // refolded: `reset_tender_layer` DROPs the table, so a schema-batch index would
        // be lost by a rebuild and not return until the next process open. The
        // `organizations` pair belongs to [`Db::build_organization_indexes`] and the
        // `notices` one to the schema batch, each for the same reason — the builder
        // that owns the table's lifecycle owns its indexes.
        //
        // ISSUE 111 APPLIES TO THIS ONE: it materialises at a rebuild's end or via the
        // `Reindex` admin job, and at no other time. Deploying the code does not create
        // it, so `?source=` stays slow until one of those runs.
        //
        // And note there is no partition size at which the rejected row-value cursor is
        // merely harmless: measured on prod's real slices it costs 71,000x at DE
        // (3.85M rows) and still 14x at MT (24,911 rows) for a 50-row page. Only a
        // `lots`-sized partition — single digits — makes its sort free, which is why
        // `1830d50` got away with it and why nothing else should copy it.
        ("tenders_source_id", "tenders(source, id)"),
    ];

    /// DROP+recreate `table` from its own captured DDL (table + any named indexes),
    /// emptying it at O(1) WAL. A `DELETE FROM` writes a WAL frame PER ROW (turso has
    /// no truncate optimisation) — over the tens-of-millions-of-row tender-content
    /// tables that is a single un-checkpointable statement that balloons the in-RAM
    /// WAL-index to OOM (issue 63). Recreating from `sqlite_master.sql` keeps the exact
    /// schema (no DDL duplication) and, by dropping the table, resets its
    /// `sqlite_sequence` high-water — the same reset the callers already intend. Runs
    /// under the rebuild's FK-off teardown, so drop order is unconstrained.
    async fn drop_and_recreate(conn: &Connection, table: &str) -> turso::Result<()> {
        let mut ddls: Vec<String> = Vec::new();
        {
            let mut rows = conn
                .query(
                    "SELECT sql FROM sqlite_master WHERE tbl_name = ?1 AND sql IS NOT NULL
                     ORDER BY (type <> 'table')",
                    (Value::Text(table.to_string()),),
                )
                .await?;
            while let Some(row) = rows.next().await? {
                ddls.push(text(&row, 0));
            }
        }
        conn.execute(&format!("DROP TABLE IF EXISTS {table}"), ()).await?;
        for ddl in ddls {
            conn.execute(&ddl, ()).await?;
        }
        Ok(())
    }

    /// Empty and schema-migrate the tender-CONTENT layer for a from-scratch Phase-2
    /// fold — a fresh rebuild OR a resume (both fold from an empty tender layer).
    /// DROP+recreate `tenders` BARE (no inline `UNIQUE`) both strips the inline
    /// auto-indexes that steepened the fold — the issue-60 fix finally applied to
    /// tenders — and, by dropping the table, clears its `sqlite_sequence` row so ids
    /// restart at 1 in fold order (making a resume byte-identical to a fresh run).
    /// The other AUTOINCREMENT content tables (`lots`/`bids`/`contracts`/
    /// `lot_results`) are emptied and their `sqlite_sequence` rows reset likewise.
    ///
    /// Preserves everything Phase-1 produced or the resume relies on:
    /// `organizations`, `organization_mentions`, the `plan_*` tables, the notice /
    /// value layer, and the `changes` cursor. Unlike [`Db::clear_canonical`] it does
    /// NOT reset the `notices.projected` watermark — the projection marks notices
    /// projected as it folds them.
    pub async fn reset_tender_layer(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        // Drop the table (removing its inline-UNIQUE auto-indexes AND its
        // sqlite_sequence row) and recreate it bare — identity is served by the
        // deferred plain named indexes instead.
        conn.execute("DROP TABLE IF EXISTS tenders", ()).await?;
        conn.execute(
            // Must carry EVERY column the fold and the deferred indexes touch:
            // `migrate()`'s ALTERs run at process open only, so a column missing
            // here is missing for the whole rebuild — the head UPDATE errors on
            // its first write and `build_tender_indexes` on its CREATE. That is
            // how `current_deadline` (issue 216) broke this path when it landed
            // via ALTER alone.
            "CREATE TABLE tenders (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, source TEXT NOT NULL,
                 procedure_key TEXT, island_notice_id INTEGER REFERENCES notices(id),
                 kind TEXT NOT NULL, created_at INTEGER NOT NULL,
                 current_seq INTEGER, current_published_at INTEGER,
                 current_deadline INTEGER, current_title TEXT,
                 projection_epoch INTEGER NOT NULL DEFAULT 0
             ) STRICT",
            (),
        )
        .await?;
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
            "tender_version_lot_group_members",
        "tender_version_lots",
            "tender_versions",
            "lot_results",
            "bids",
            "contracts",
            "lots",
        ] {
            // DROP+recreate, NOT DELETE: these tables are still full from the prior
            // build (each attempt's per-row DELETE ballooned the WAL and was rolled
            // back on kill, so they never cleared — the deadlock, issue 63).
            Self::drop_and_recreate(&conn, table).await?;
        }
        // Note: `changes` (the CDC cursor spine) is deliberately NOT cleared here.
        // A rebuild re-derives identical deterministic surrogate ids (ADR-0001), so
        // existing change rows stay valid; re-projection appends a fresh set and the
        // cursor is never renumbered (docs/architecture.md; covered by
        // `the_change_log_reads_added_then_changed`).
        //
        // Also clear tenders' high-water row explicitly: the DROP above already
        // removes it on turso, so this is belt-and-suspenders — correctness no longer
        // depends on turso's DROP-TABLE-clears-sqlite_sequence behavior. A no-op if
        // the DROP cleared it; the reset if it did not. Either way the next tenders
        // INSERT gets id 1.
        conn.execute(
            "DELETE FROM sqlite_sequence WHERE name IN ('tenders','lots','bids','contracts','lot_results')",
            (),
        )
        .await?;
        Ok(())
    }

    /// One-time reset of the CDC `changes` table for the recovered baseline (issue
    /// 81 / bulk-recovery). DROP+recreate it — dropping its `sqlite_sequence`
    /// high-water so the cursor restarts at 0 — and reset the in-memory cursor watch
    /// to match, so a paired `rebuild:true` re-emits ONE clean generation reflecting
    /// the final layer instead of appending onto the accumulated feed. `changes` is
    /// derived data (the rebuild regenerates it), so this loses nothing
    /// reconstructable; only meaningful WITH a rebuild that re-emits (clearing alone
    /// would leave the feed empty against an intact layer). No external consumers
    /// yet; the internal coverage refresher uses the cursor only as a change-detector
    /// (its `HeavyKey`), so it re-measures on the reset rather than breaking.
    pub async fn clear_changes(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("DROP TABLE IF EXISTS changes", ()).await?;
        conn.execute(
            "CREATE TABLE changes (
                 cursor      INTEGER PRIMARY KEY AUTOINCREMENT,
                 entity_kind TEXT NOT NULL,
                 entity_id   INTEGER NOT NULL,
                 version_seq INTEGER,
                 op          TEXT NOT NULL,
                 changed_at  INTEGER NOT NULL
             ) STRICT",
            (),
        )
        .await?;
        conn.execute("CREATE INDEX IF NOT EXISTS changes_entity ON changes(entity_kind, entity_id)", ())
            .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS changes_entity_cursor ON changes(entity_kind, cursor)",
            (),
        )
        .await?;
        // A dropped feed re-issues cursors from 1 — an old cursor would "resume"
        // inside the new feed and silently merge two generations (issue 46).
        conn.execute("UPDATE feed_generation SET generation = generation + 1 WHERE id = 0", ())
            .await?;
        // The watch was seeded from the OLD high-water; reset it to the new empty
        // table's max (0) so /health and the coverage refresher see the reset at once.
        self.publish_cursor(&conn).await?;
        Ok(())
    }

    /// Drop the random-key tender satellite indexes before a full Phase-2 (issue
    /// 60/62). Runs for a fresh rebuild AND a resume — both do a from-scratch fold.
    pub async fn strip_tender_indexes(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        for (name, _) in Self::DEFERRED_TENDER_INDEXES {
            conn.execute(&format!("DROP INDEX IF EXISTS {name}"), ()).await?;
        }
        Ok(())
    }

    /// Rebuild the deferred tender satellite indexes after the fold — one sorted
    /// build each, over the now-complete tables.
    pub async fn build_tender_indexes(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        for (name, cols) in Self::DEFERRED_TENDER_INDEXES {
            if let Some(rows) = Self::too_large_to_build(&conn, cols).await? {
                eprintln!(
                    "store: REFUSING to auto-build {name}: {cols} has ~{rows} rows, over the \
                     {} row cap — a bulk CREATE INDEX there would sort ~{} GB (issue 111). \
                     Build it index-first at a rebuild, or in a maintenance window.",
                    Self::MAX_AUTO_INDEX_ROWS,
                    rows * 45 / 1_000_000_000,
                );
                continue;
            }
            conn.execute(&format!("CREATE INDEX IF NOT EXISTS {name} ON {cols}"), ()).await?;
        }
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
        // DROP+recreate the plan tables bare (O(1) WAL) — see clear_plan_on. Was
        // CREATE-IF-NOT-EXISTS then a whole-table DELETE, whose DELETE ballooned the
        // WAL over a partial plan left by a killed run (issue 63).
        self.clear_plan_on(&conn).await
    }

    /// Empty the grouping-plan scratch tables (and drop the Phase-2 fold index so
    /// the next run rebuilds it). Called at the start and the end of a run — the
    /// durable DB never carries a projection's transient plan between runs.
    pub async fn clear_plan(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        // Retire the plan AND clear the rebuild watermark ATOMICALLY (one
        // transaction): the only dangerous end state is "plan retired but still
        // marked rebuilding" (→ the salvage would re-nuke a fully-built layer on the
        // next restart). A single transaction makes that state unreachable regardless
        // of crash timing. A no-op on the incremental path (flag already 0). Kept out
        // of `clear_plan_on`, which the START-of-run `reset_plan` uses and must NOT
        // touch the flag. Per the turso ROLLBACK-on-dropped-write discipline
        // (CONTEXT.md), any failure rolls back before propagating.
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = async {
            conn.execute("UPDATE projection_state SET rebuild_in_progress = 0 WHERE id = 0", ()).await?;
            self.clear_plan_on(&conn).await
        }
        .await;
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

    /// Mark that a full rebuild's Phase-2 is mid-flight (salvage-loop fix). Set the
    /// instant a rebuild commits to emptying the layer (with `reset_tender_layer`),
    /// so an interruption is resumable; cleared by `clear_plan` on clean completion.
    pub async fn set_rebuild_in_progress(&self) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("UPDATE projection_state SET rebuild_in_progress = 1 WHERE id = 0", ()).await?;
        Ok(())
    }

    /// Whether a full rebuild's Phase-2 was interrupted and should be resumed — the
    /// resume-from-plan salvage signal. This, NOT `plan_is_complete`, is what the
    /// supervisor keys the salvage on: a complete plan on disk is also the resting
    /// state of a FINISHED build and of an interrupted rebuild=false full-fallback,
    /// neither of which must trigger a layer-nuking resume.
    pub async fn rebuild_in_progress(&self) -> turso::Result<bool> {
        let conn = self.conn().await;
        let mut r = conn
            .query("SELECT rebuild_in_progress FROM projection_state WHERE id = 0", ())
            .await?;
        match r.next().await? {
            Some(row) => Ok(int(&row, 0) != 0),
            None => Ok(false),
        }
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

    /// Reset the transient plan tables to present-but-empty by DROP+recreate — the
    /// single home of the plan DDL, used by both `reset_plan` (start of a run) and
    /// `clear_plan` (end). DROP (not `DELETE FROM`) because a DELETE of the ~14M-row
    /// plan_notice writes a WAL frame PER ROW and balloons the in-RAM WAL-index to
    /// OOM (issue 63; turso has no truncate optimisation), whereas DROP is O(1).
    /// DROP TABLE cascades the fold/edge indexes, so no explicit DROP INDEX is needed.
    async fn clear_plan_on(&self, conn: &Connection) -> turso::Result<()> {
        for table in ["plan_notice", "plan_ojs_node", "plan_ojs_edge", "plan_prev_edge", "plan_group_merge"] {
            conn.execute(&format!("DROP TABLE IF EXISTS {table}"), ()).await?;
        }
        conn.execute(
            "CREATE TABLE plan_notice (
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
            "CREATE TABLE plan_ojs_node (key INTEGER PRIMARY KEY, label INTEGER NOT NULL) STRICT",
            (),
        )
        .await?;
        conn.execute("CREATE TABLE plan_ojs_edge (a INTEGER NOT NULL, b INTEGER NOT NULL) STRICT", ())
            .await?;
        // ADR-0011: the publisher-declared previous-notice edges. `a_source` rides along
        // so resolution can require the target to be the SAME Source without joining
        // plan_notice for it — and so the lookup hits `notices`'
        // UNIQUE(source, publication_id, …) index by its leading columns.
        conn.execute(
            "CREATE TABLE plan_prev_edge (
                 a_notice_id      INTEGER NOT NULL,
                 a_source         TEXT NOT NULL,
                 b_publication_id TEXT NOT NULL
             ) STRICT",
            (),
        )
        .await?;
        // The `from_key -> to_key` relabel map the previous-notice pass builds, kept as a
        // table so the relabel is one indexed pass over plan_notice instead of a
        // statement per merged component.
        conn.execute(
            "CREATE TABLE plan_group_merge (from_key TEXT PRIMARY KEY, to_key TEXT NOT NULL) STRICT",
            (),
        )
        .await?;
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

    /// The legacy-adjacency coverage attestation (issue 58 v2): every parsed
    /// notice with id ≤ watermark has its `legacy_ojs_keys` rows. 0 = never
    /// established — the incremental fold must keep the v1 full fallback.
    pub async fn legacy_adjacency_watermark(&self) -> turso::Result<i64> {
        let conn = self.conn().await;
        let mut rows = conn.query("SELECT watermark FROM legacy_adjacency WHERE id = 0", ()).await?;
        Ok(match rows.next().await? {
            Some(row) => int(&row, 0),
            None => 0,
        })
    }

    /// [`Db::legacy_adjacency_watermark`] over the READER POOL — for observers
    /// (`/metrics`) rather than the projection. The projection wants the writer's
    /// own view, since it raises the watermark in the same transaction sequence
    /// it writes key rows; an observer must never take the writer mutex, or a
    /// scrape during a projection would queue behind a multi-hour fold. Serving
    /// over WAL, this can read a watermark one commit stale, which is harmless
    /// for a gauge and never for a gate (the gate uses the writer view above).
    pub async fn legacy_adjacency_watermark_observed(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        let mut rows = conn.query("SELECT watermark FROM legacy_adjacency WHERE id = 0", ()).await?;
        Ok(match rows.next().await? {
            Some(row) => int(&row, 0),
            None => 0,
        })
    }

    /// ESTABLISH coverage after a pass that visited EVERY parsed notice (a full
    /// plan build, or the backfill job): watermark := max(current, `to`). Only
    /// such a pass may make the first raise — the incremental advance below
    /// refuses to move a 0 watermark, so induction always starts from a base
    /// that actually covered the corpus.
    pub async fn establish_legacy_adjacency(&self, to: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE legacy_adjacency SET watermark = MAX(watermark, ?) WHERE id = 0",
            (Value::Integer(to),),
        )
        .await?;
        Ok(())
    }

    /// ADVANCE coverage after an incremental plan build. Sound because the delta
    /// is ALL unprojected parsed notices: any parsed notice above the old
    /// watermark has never been folded (projected is only ever set by a fold that
    /// planned it, which wrote its rows), so it is in this delta and its rows
    /// were just written; everything at or below is covered by induction. A
    /// reclaimed notice below the watermark re-enters the next delta and rewrites
    /// its rows before any closure runs. No-op while the base was never
    /// established (watermark 0).
    pub async fn advance_legacy_adjacency(&self, to: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE legacy_adjacency SET watermark = MAX(watermark, ?)
              WHERE id = 0 AND watermark > 0",
            (Value::Integer(to),),
        )
        .await?;
        Ok(())
    }

    /// One backfill batch of durable adjacency rows (issue 58 v2, step 2) in a
    /// single transaction. INSERT OR IGNORE — re-sweeping a notice the plan
    /// build (or an interrupted earlier backfill) already recorded is free.
    pub async fn insert_legacy_ojs_keys(&self, rows: &[(i64, i64)]) -> turso::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let conn = self.conn().await;
        conn.execute("BEGIN", ()).await?;
        for &(key, notice_id) in rows {
            conn.execute(
                "INSERT OR IGNORE INTO legacy_ojs_keys(ojs_key, notice_id) VALUES(?, ?)",
                (Value::Integer(key), Value::Integer(notice_id)),
            )
            .await?;
        }
        conn.execute("COMMIT", ()).await?;
        Ok(())
    }

    /// One key→notices hop of the legacy closure walk (issue 58 v2, step 3):
    /// every notice carrying ANY of the given OJS keys. Sorted, deduplicated.
    pub async fn legacy_notices_for_keys(&self, keys: &[i64]) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut set = std::collections::BTreeSet::new();
        for chunk in keys.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&k| Value::Integer(k)).collect();
            let sql = format!(
                "SELECT DISTINCT notice_id FROM legacy_ojs_keys WHERE ojs_key IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                set.insert(int(&row, 0));
            }
        }
        Ok(set.into_iter().collect())
    }

    /// The notices→keys hop: every OJS key carried by ANY of the given notices
    /// (self and edge rows alike — the closure treats them identically).
    pub async fn legacy_keys_for_notices(&self, notice_ids: &[i64]) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut set = std::collections::BTreeSet::new();
        for chunk in notice_ids.chunks(IN_CHUNK) {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let sql = format!(
                "SELECT DISTINCT ojs_key FROM legacy_ojs_keys WHERE notice_id IN ({})",
                placeholders(chunk.len())
            );
            let mut rows = conn.query(&sql, params).await?;
            while let Some(row) = rows.next().await? {
                set.insert(int(&row, 0));
            }
        }
        Ok(set.into_iter().collect())
    }

    /// The Tenders the given notices currently fold into — the closure walk pulls
    /// each hop's tender memberships so those Tenders re-derive IN FULL. Same
    /// `caused_by_notice_id` lookup [`Db::touched_existing_tender_ids`] runs over
    /// the changed set.
    pub async fn tenders_for_notice_ids(&self, notice_ids: &[i64]) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut set = std::collections::BTreeSet::new();
        for chunk in notice_ids.chunks(IN_CHUNK) {
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
        Ok(set.into_iter().collect())
    }

    /// Coverage-gap verify for the closure gate (issue 58 v2, step 3): the highest
    /// PROJECTED parsed notice above the watermark, if any. The advance induction
    /// (see [`Db::advance_legacy_adjacency`]) makes this impossible under the
    /// current binary — a hit means a fold ran WITHOUT the choke-point writer (a
    /// rollback binary projected notices and never advanced), so the table has
    /// silent holes and the closure must fall back to the full path, which
    /// re-establishes coverage. Cheap: the id range above the watermark is
    /// normally just the current (unprojected) delta.
    pub async fn projected_parsed_above(&self, watermark: i64) -> turso::Result<Option<i64>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT MAX(id) FROM notices
                  WHERE id > ? AND parse_state = 'parsed' AND projected = 1",
                (Value::Integer(watermark),),
            )
            .await?;
        Ok(match rows.next().await? {
            Some(row) => match row.get_value(0)? {
                Value::Integer(id) => Some(id),
                _ => None,
            },
            None => None,
        })
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
            // issue 58 v2: the DURABLE adjacency rows, in the same transaction —
            // every key this legacy notice touches, self and edges alike (a
            // SUPERSET of what the grouping unions below, which is the safe
            // direction for the closure: over-collecting widens the scoped plan,
            // under-collecting would mis-merge). INSERT OR IGNORE: re-planning
            // the same notice is idempotent.
            if r.legacy {
                for key in r.ojs_self.iter().chain(&r.ojs_edges) {
                    conn.execute(
                        "INSERT OR IGNORE INTO legacy_ojs_keys(ojs_key, notice_id) VALUES(?, ?)",
                        (Value::Integer(*key), Value::Integer(r.notice_id)),
                    )
                    .await?;
                }
            }
            // ADR-0011: one row per predecessor this notice names. Resolution happens at
            // grouping time, against the plan — a reference to a notice we do not hold
            // creates nothing, and a reference into another Source is not followed.
            for pub_id in &r.prev_refs {
                conn.execute(
                    "INSERT INTO plan_prev_edge(a_notice_id, a_source, b_publication_id)
                     VALUES(?, ?, ?)",
                    (Value::Integer(r.notice_id), t(&r.source), t(pub_id)),
                )
                .await?;
            }
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
        // Resumable grouping: the fold index is this method's LAST write, so its
        // presence means a prior run already assigned every group_key from the SAME
        // plan with the SAME logic (grouping is a pure function of the plan). Reuse it
        // rather than redo the ~25 min at 12.4M — the resume salvage, extended past
        // Phase-1 to grouping. A fresh rebuild's build_plan → reset_plan drops the
        // fold index (and clears the plan) first, so this only fires on a resume/retry
        // where grouping genuinely completed on disk. (If grouping logic ever changes,
        // drop plan_notice_fold to force a recompute.)
        {
            let mut r = conn
                .query("SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'plan_notice_fold'", ())
                .await?;
            if r.next().await?.is_some() {
                drop(r);
                eprintln!("[project] group: reusing complete on-disk grouping (fold index present)");
                return Ok(());
            }
        }
        // Fresh grouping (the fold index is absent — reset_plan dropped it). The
        // group_key UPDATEs run index-free so they don't pay per-row index
        // maintenance on ~22M writes; the fold index is rebuilt once at the end.
        // Keyed and island in one pass; legacy left NULL for the closure below.
        // BATCHED by notice_id range with a TRUNCATE between (issue 63): a single
        // whole-corpus UPDATE writes a WAL frame per row and cannot be checkpointed
        // mid-statement, ballooning the in-RAM WAL-index to OOM at 14M. Byte-identical
        // to the one-shot — each row's key is a pure function of its own columns.
        let t = std::time::Instant::now();
        let (min_id, max_id) = {
            let mut r = conn.query("SELECT MIN(notice_id), MAX(notice_id) FROM plan_notice", ()).await?;
            match r.next().await? {
                Some(row) => (opt_int_of(&row, 0), opt_int_of(&row, 1)),
                None => (None, None),
            }
        };
        if let (Some(min_id), Some(max_id)) = (min_id, max_id) {
            let mut lo = min_id;
            while lo <= max_id {
                let hi = lo.saturating_add(GROUP_KEY_UPDATE_BATCH - 1).min(max_id);
                conn.execute(
                    "UPDATE plan_notice SET group_key = CASE
                         WHEN procedure_key IS NOT NULL THEN procedure_key
                         WHEN NOT (legacy = 1 AND ojs_self IS NOT NULL) THEN 'island:' || notice_id
                         ELSE NULL END
                     WHERE notice_id BETWEEN ? AND ?",
                    (Value::Integer(lo), Value::Integer(hi)),
                )
                .await?;
                let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
                lo = hi.saturating_add(1);
            }
        }
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
        // TRUNCATE between chunks so the accumulated legacy UPDATEs (millions of
        // rows across all chunks) don't balloon the WAL as one un-checkpointed run
        // (issue 63) — the per-chunk BEGIN/COMMIT alone never reclaimed it.
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
            let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
        }
        eprintln!("[project] group step legacy-update: {:.1}s ({} legacy)", t.elapsed().as_secs_f64(), legacy.len());

        // ADR-0011: the publisher-declared previous-notice edge. It runs HERE — after the
        // keyed and legacy passes, before the fold index — for two reasons: it unions
        // COMPONENTS, so every notice must already carry a group_key, and relabelling
        // before the index exists means the UPDATE pays no index maintenance.
        //
        // EU eForms does not keep BT-04 stable across a procedure's notices (issue 236:
        // 27–39 % of EU award Tenders are single-notice islands whose contract notice sits
        // in the corpus under a different BT-04). `OPP-090-Procedure` is the source's own
        // statement that two publications are one procedure, so joining them is reading a
        // published fact, not inference (ADR-0003's "never heuristic" line).
        //
        // Guards, all measured on prod before being written (ADR-0011): the target must
        // exist, must be the same Source, and must be strictly EARLIER — 563 of 563
        // resolvable references pointed backwards, so a forward or self reference is not a
        // shape the corpus has, and refusing it rules out a cycle for free.
        //
        // On an INCREMENTAL run the plan holds only the touched notices, so a reference to
        // an untouched notice finds no `plan_notice b` row and unions nothing: the award
        // stays an island until a run whose plan holds both. That is the intended
        // degradation — a partial merge would be far worse than a late one — and it is why
        // the corpus repair needs the rebuild, not a daily projection.
        // `started`, because the earlier passes in this function have already bound `t` to
        // an Instant — which shadows the crate's `t()` text helper for the rest of the
        // body, so the INSERTs below spell their text values out.
        // Row stats BEFORE the join, not only after the fold index (issue 256).
        //
        // turso keeps none unless asked, and this file already says of the LATER `ANALYZE
        // plan_notice` that "its young planner has mis-planned at scale". The join below
        // is exactly the shape that needs stats to get right: `plan_prev_edge` is tiny
        // (563 resolvable references corpus-wide when ADR-0011 landed) and `plan_notice`
        // is 14.3M rows on a full-corpus plan, and driving from the wrong one turns a few
        // hundred index seeks into a walk of the whole plan. With no stats the planner has
        // no way to tell which is which — and this step has twice failed to finish.
        //
        // A HYPOTHESIS, not a proven fix: the verification is whether the step completes
        // on the next full re-projection, which is also the first run whose heartbeats say
        // how many edges there were. Non-fatal like the later ANALYZE, and timed, so its
        // own cost at 14.3M rows is on the record rather than assumed.
        for table in ["plan_prev_edge", "plan_notice"] {
            let t = std::time::Instant::now();
            match conn.execute(&format!("ANALYZE {table}"), ()).await {
                Ok(_) => eprintln!(
                    "[project] group step analyze {table}: {:.1}s",
                    t.elapsed().as_secs_f64()
                ),
                Err(e) => eprintln!("[project] ANALYZE {table} failed (non-fatal): {e}"),
            }
        }

        let started = std::time::Instant::now();
        // The input size, printed BEFORE the join runs (issue 256). On a full-corpus
        // re-projection this step has twice gone quiet for over two hours, and from
        // outside the process there was no way to tell an empty edge set from a join that
        // is grinding: the step's only line prints when it FINISHES. One count over a plan
        // table costs nothing next to what follows it.
        let mut edge_rows = conn.query("SELECT COUNT(*) FROM plan_prev_edge", ()).await?;
        let planned_edges = match edge_rows.next().await? {
            Some(row) => int(&row, 0),
            None => 0,
        };
        eprintln!("[project] group step previous-notice: {planned_edges} edge(s) in the plan");
        let mut edges: Vec<(String, String)> = Vec::new();
        let mut rank: std::collections::HashMap<String, (i64, String)> =
            std::collections::HashMap::new();
        let t_read = std::time::Instant::now();
        {
            let mut read = 0u64;
            // Heartbeat on the CLOCK, not on a row count. The count was rows the query
            // RETURNS, and the query filters hard (same-Source, strictly-earlier, different
            // key), so a run that matched fewer than the interval printed nothing at all and
            // looked identical to a hang — which is exactly how I misread 2026-08-20's run
            // for ten minutes (issue 256 part 2). A clock cannot be silent.
            let mut beat = std::time::Instant::now();
            let mut rows = conn.query(PREV_EDGE_JOIN_SQL, ()).await?;
            while let Some(row) = rows.next().await? {
                read += 1;
                if beat.elapsed() >= PREV_EDGE_HEARTBEAT {
                    beat = std::time::Instant::now();
                    eprintln!(
                        "[project] group step previous-notice: {read}/{planned_edges} edge(s) \
                         matched in {:.1}s",
                        started.elapsed().as_secs_f64()
                    );
                }
                let (a, b) = (text(&row, 0), text(&row, 3));
                // A key's rank is the earliest publication it is seen carrying, so the
                // representative below is the procedure's first appearance — the same rule
                // the legacy closure applies with MIN(ojs).
                for (key, at, pub_id) in
                    [(&a, int(&row, 1), text(&row, 2)), (&b, int(&row, 4), text(&row, 5))]
                {
                    let entry = rank.entry(key.clone()).or_insert((at, pub_id.clone()));
                    if (at, pub_id.clone()) < *entry {
                        *entry = (at, pub_id);
                    }
                }
                edges.push((a, b));
            }
        }
        eprintln!(
            "[project] group step previous-notice read: {:.1}s ({} of {planned_edges} edge(s) \
             matched, {} key(s) involved)",
            t_read.elapsed().as_secs_f64(),
            edges.len(),
            rank.len()
        );
        let t_uf = std::time::Instant::now();
        let merges: Vec<(String, String)> = if edges.is_empty() {
            Vec::new()
        } else {
            // Order the involved keys by (earliest publication, publication id, key) and
            // union over their POSITIONS: [`MinUnionFind`] unions to the minimum, so the
            // component root is by construction the earliest-published key. Reusing it
            // this way is why there is no second union-find here.
            let mut keys: Vec<String> = rank.keys().cloned().collect();
            keys.sort_by(|x, y| rank[x].cmp(&rank[y]).then_with(|| x.cmp(y)));
            let position: std::collections::HashMap<&str, i64> =
                keys.iter().enumerate().map(|(i, k)| (k.as_str(), i as i64)).collect();
            let mut uf = MinUnionFind::default();
            for (a, b) in &edges {
                uf.union(position[a.as_str()], position[b.as_str()]);
            }
            keys.iter()
                .enumerate()
                .filter_map(|(i, key)| {
                    let root = uf.find(i as i64) as usize;
                    (root != i).then(|| (key.clone(), keys[root].clone()))
                })
                .collect()
        };
        eprintln!(
            "[project] group step previous-notice union: {:.1}s ({} key(s) to relabel)",
            t_uf.elapsed().as_secs_f64(),
            merges.len()
        );
        if !merges.is_empty() {
            let t_write = std::time::Instant::now();
            for chunk in merges.chunks(NODE_WRITE_BATCH) {
                conn.execute("BEGIN IMMEDIATE", ()).await?;
                for (from, to) in chunk {
                    conn.execute(
                        "INSERT OR REPLACE INTO plan_group_merge(from_key, to_key) VALUES(?, ?)",
                        (Value::Text(from.clone()), Value::Text(to.clone())),
                    )
                    .await?;
                }
                conn.execute("COMMIT", ()).await?;
            }
            eprintln!(
                "[project] group step previous-notice merge-write: {:.1}s",
                t_write.elapsed().as_secs_f64()
            );
            // Batched by notice_id like the keyed/island pass, and for the same reason
            // (issue 63: one whole-plan UPDATE cannot be checkpointed mid-statement).
            let t_relabel = std::time::Instant::now();
            let mut beat_relabel = std::time::Instant::now();
            let mut batches = 0u64;
            if let (Some(min_id), Some(max_id)) = (min_id, max_id) {
                let mut lo = min_id;
                while lo <= max_id {
                    batches += 1;
                    let hi = lo.saturating_add(GROUP_KEY_UPDATE_BATCH - 1).min(max_id);
                    // `EXISTS`, not `IN (SELECT …)` — measured, not stylistic. turso plans
                    // the IN form as a LIST SUBQUERY it re-evaluates by SCANNING
                    // plan_group_merge for every candidate row (EXPLAIN shows the scan
                    // inside the loop), which at 24,528 merge keys × 200,000 rows per
                    // batch is ~5 billion comparisons a batch — measured on the bench at
                    // ~147 s per 20k rows, and live on job 289 at 187 s per batch × 143
                    // batches ≈ 6 HOURS, which was issue 256's "silent hours" all along.
                    // The EXISTS form is an indexed probe of plan_group_merge's PRIMARY
                    // KEY per row; the same bench batch completes in well under a second.
                    conn.execute(
                        "UPDATE plan_notice SET group_key = \
                             (SELECT to_key FROM plan_group_merge m WHERE m.from_key = plan_notice.group_key) \
                          WHERE notice_id BETWEEN ? AND ? \
                            AND EXISTS (SELECT 1 FROM plan_group_merge x \
                                         WHERE x.from_key = plan_notice.group_key)",
                        (Value::Integer(lo), Value::Integer(hi)),
                    )
                    .await?;
                    let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
                    if beat_relabel.elapsed() >= PREV_EDGE_HEARTBEAT {
                        beat_relabel = std::time::Instant::now();
                        eprintln!(
                            "[project] group step previous-notice relabel: batch {batches}, \
                             notice_id {lo}..{hi} of {max_id}, {:.1}s",
                            t_relabel.elapsed().as_secs_f64()
                        );
                    }
                    lo = hi.saturating_add(1);
                }
            }
            eprintln!(
                "[project] group step previous-notice relabel: {:.1}s ({batches} batch(es))",
                t_relabel.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "[project] group step previous-notice: {:.1}s ({} edge(s) resolved, {} key(s) merged)",
            started.elapsed().as_secs_f64(),
            edges.len(),
            merges.len()
        );

        // The fold order Phase 2 streams by: rows arrive grouped by Tender and, within
        // a Tender, in supersession order (ADR-0003 tiebreak), so no in-RAM sort.
        // A single CREATE INDEX over the whole plan is one bounded statement; reclaim
        // its WAL immediately after so it doesn't sit as a multi-GB tail (issue 63).
        let t = std::time::Instant::now();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS plan_notice_fold
                 ON plan_notice(group_key, published_at, source_rank, publication_id, notice_id)",
            (),
        )
        .await?;
        let _ = checkpoint_on(&conn, CheckpointMode::Truncate).await;
        eprintln!("[project] group step fold-index: {:.1}s", t.elapsed().as_secs_f64());
        // Give turso's planner row stats so plan_summary and Phase-2's next_plan_batch
        // stream via plan_notice_fold instead of sorting the group_key tail (turso
        // keeps no stats otherwise, and its young planner has mis-planned at scale).
        // Non-fatal — without stats the planner should still match the fold index to
        // the range+order; this just removes the risk it doesn't. Cheap here: the
        // canonical layer is empty at grouping time, so only the notice tables scan.
        let t = std::time::Instant::now();
        if let Err(e) = conn.execute("ANALYZE plan_notice", ()).await {
            eprintln!("[project] ANALYZE plan_notice failed (non-fatal): {e}");
        }
        eprintln!("[project] group step analyze: {:.1}s", t.elapsed().as_secs_f64());
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

    /// The `notice_id → group_key` map for a contiguous notice-id range — the
    /// Phase-2 bucketed fold's routing lookup (issue 62). The fold's pre-pass reads
    /// the parsed layer in id-order chunks ([`Db::parsed_chunk`]) and needs each
    /// notice's grouping key to route it to its order-preserving bucket; a ranged
    /// scan over `plan_notice`'s `notice_id` PK matches that burst-sequential
    /// pattern (one range scan per chunk, not an `IN(…)` per-notice probe). Notices
    /// outside the plan — a resume's post-Phase-1 suffix — are simply absent from
    /// the map and skipped (left for the incremental projection).
    pub async fn plan_group_keys(
        &self,
        lo: i64,
        hi: i64,
    ) -> turso::Result<std::collections::HashMap<i64, String>> {
        let conn = self.reader().await?;
        Self::plan_group_keys_on(&conn, lo, hi).await
    }

    /// As [`Db::plan_group_keys`], but through an explicit connection — each sharded
    /// Phase-2 pre-pass worker (issue 66) routes its stripe on its own reader.
    pub async fn plan_group_keys_on(
        conn: &Connection,
        lo: i64,
        hi: i64,
    ) -> turso::Result<std::collections::HashMap<i64, String>> {
        let mut out = std::collections::HashMap::new();
        let mut rows = conn
            .query(
                "SELECT notice_id, group_key FROM plan_notice
                  WHERE notice_id >= ? AND notice_id <= ? AND group_key IS NOT NULL",
                (Value::Integer(lo), Value::Integer(hi)),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            out.insert(int(&row, 0), text(&row, 1));
        }
        Ok(out)
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
        Ok(MentionResolver { org_of, name_of: std::collections::HashMap::new(), created_any: false })
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
                    .resolve_one_mention(
                        &conn,
                        m,
                        now,
                        &mut resolver.org_of,
                        &mut resolver.name_of,
                        &mut mention_of,
                    )
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
        name_of: &mut std::collections::HashMap<(String, String), i64>,
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
                             name_norm, provisional, created_at)
                         VALUES(?, ?, ?, ?, ?, 0, ?)",
                        (
                            opt_text(id.country.as_deref()),
                            t(&id.kind),
                            t(&id.value),
                            t(&m.name),
                            Value::Text(m.name.to_lowercase()),
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
                // Issue 234: reuse before minting. Two notices naming the same
                // authority — byte-identical normalised name, same country — are one
                // Organization, not two; without this the table measured 95.30 %
                // provisional (23.46M of 24.6M rows), which makes "canonical
                // Organization" describe 4.7 % of its own table and every legacy
                // buyer rollup mostly fragments.
                //
                // Scoped DELIBERATELY narrowly, per the over-merge evidence on the
                // issue: only a NON-EMPTY name (nameless rows are distinct unknown
                // parties — 2,411 of them are awarded contractors, and merging every
                // nameless org in a country into one row would be corpus-scale
                // cross-linking) and only WITH a country (same-name-no-country is
                // where platform strings like `tendsign` concentrate, and a
                // country-less merge has no scope to err inside). The row STAYS
                // `provisional = 1`, so a later identifier can still split or
                // canonicalise it — the merge is a reuse policy, not a promotion.
                let name_norm = m.name.to_lowercase();
                let scope = (!name_norm.is_empty()).then_some(()).and(m.country.clone());
                if let Some(country) = &scope {
                    let key = (name_norm.clone(), country.clone());
                    if let Some(&org_id) = name_of.get(&key) {
                        (org_id, false)
                    } else {
                        let mut rows = conn
                            .query(
                                "SELECT id FROM organizations \
                                  WHERE name_norm = ? AND country = ? AND identifier IS NULL \
                                  LIMIT 1",
                                (Value::Text(name_norm.clone()), t(country)),
                            )
                            .await?;
                        let hit = match rows.next().await? {
                            Some(row) => Some(int(&row, 0)),
                            None => None,
                        };
                        drop(rows);
                        match hit {
                            Some(org_id) => {
                                name_of.insert(key, org_id);
                                (org_id, false)
                            }
                            None => {
                                conn.execute(
                                    "INSERT INTO organizations(country, identifier_kind, identifier, name,
                                         name_norm, provisional, created_at)
                                     VALUES(?, NULL, NULL, ?, ?, 1, ?)",
                                    (
                                        opt_text(m.country.as_deref()),
                                        t(&m.name),
                                        Value::Text(name_norm.clone()),
                                        Value::Integer(now),
                                    ),
                                )
                                .await?;
                                let org_id = last_insert_rowid(conn).await?;
                                name_of.insert(key, org_id);
                                (org_id, true)
                            }
                        }
                    }
                } else {
                    conn.execute(
                        "INSERT INTO organizations(country, identifier_kind, identifier, name,
                             name_norm, provisional, created_at)
                         VALUES(?, NULL, NULL, ?, ?, 1, ?)",
                        (
                            opt_text(m.country.as_deref()),
                            t(&m.name),
                            Value::Text(name_norm),
                            Value::Integer(now),
                        ),
                    )
                    .await?;
                    (last_insert_rowid(conn).await?, true)
                }
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
    pub async fn apply_tenders(&self, projections: &[TenderProjection], now: i64, rebuild: bool) -> turso::Result<Applied> {
        let conn = self.conn().await;
        // Prepare the hot per-row inserts once for this connection and reuse the
        // handles across every row of every batch (task #3): turso re-parses the
        // SQL on each `conn.execute`, so a reused prepared statement is ~2.87x on
        // the fold's uniform inserts. The handles outlive the per-batch
        // BEGIN/COMMIT and the between-batch checkpoints (they carry their own
        // connection clone) and no DDL runs during the fold, so no plan goes stale.
        let mut stmts = TenderInserts::prepare(&conn).await?;
        let mut total = Applied::default();
        let mut changed_any = false;
        for (batch, chunk) in projections.chunks(WRITE_BATCH).enumerate() {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            // Accumulate the batchable leaf/satellite rows of the whole
            // transaction (issue 67) and flush them as chunked multi-row INSERTs
            // just before COMMIT — one statement per ~150 rows instead of one per
            // row. Only tables whose row order is unobservable go here (no emitted
            // or referenced surrogate id); the identity tables and `changes` keep
            // inserting in place, so their ids/cursor stay in fold order.
            let mut pending = Pending::default();
            let mut applied = Applied::default();
            let mut error = None;
            // The Tenders whose version chain this batch rewrote — the set whose
            // head pointer this run is answerable for (see `assert_heads_match`).
            let mut rewrote = Vec::new();
            for p in chunk {
                match self.apply_tender_tx(&conn, p, now, rebuild, &mut stmts, &mut pending).await {
                    Ok((a, head)) => {
                        applied.add(a);
                        rewrote.extend(head);
                    }
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }
            match error {
                None => {
                    pending.flush(&conn).await?;
                    // Integrity gate (task #27), inside the transaction: a head
                    // that is not the last version never reaches disk.
                    if let Err(e) = Self::assert_heads_match(&conn, &rewrote).await {
                        let _ = conn.execute("ROLLBACK", ()).await;
                        return Err(e);
                    }
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
        rebuild: bool,
        stmts: &mut TenderInserts,
        pending: &mut Pending,
    ) -> turso::Result<(Applied, Option<i64>)> {
        let mut applied = Applied::default();
        let (tender_id, created, stored_epoch) =
            self.tender_identity(conn, p, now, rebuild, stmts).await?;
        applied.tenders_created += u64::from(created);

        // The projection is deterministic, so the sequence of causing notices
        // is the state key: an unchanged sequence means an unchanged chain, and
        // a changed one is repaired from the first differing position (a
        // late-arriving notice that belongs mid-chain rewrites the tail).
        //
        // On a rebuild the tender id was just minted by tender_identity's rebuild
        // fast-path, so its version chain is empty by construction — skip the
        // SELECT (issue 67, ~one per tender). keep stays 0 and every version is
        // written: byte-identical to today's empty-`stored` path.
        let stored = if rebuild { Vec::new() } else { self.stored_chain(conn, tender_id).await? };
        // A stored epoch from older projection LOGIC makes the chain meaningless as a
        // state key: the same notices now fold to different content (issue 99). Force
        // a full rewrite by keeping nothing — which re-uses the repair path below
        // unchanged, so there is no second write mechanism to keep correct. Merely
        // skipping the early return would do NOTHING: with the chain unchanged,
        // `keep == stored.len() == p.versions.len()`, so both loops are no-ops.
        let stale = stored_epoch != PROJECTION_EPOCH;
        let keep = if stale {
            0
        } else {
            stored
                .iter()
                .zip(&p.versions)
                .take_while(|(a, b)| **a == b.caused_by_notice_id)
                .count()
        };
        if !stale && keep == stored.len() && keep == p.versions.len() {
            // Considered and verified current — counted, so a fold that skips
            // everything is distinguishable from one that wrote everything by
            // its report alone (issue 108).
            applied.tenders_unchanged += 1;
            return Ok((applied, None));
        }
        applied.tenders_written += 1;

        // A note for whoever reconciles a re-fold's numbers, because the obvious
        // reading is wrong and it reconciles anyway.
        //
        // `applied.versions_written` below counts WRITE OPERATIONS, not rows in
        // `tender_versions`. A forced rewrite (`keep = 0`) deletes and re-writes the
        // SAME versions, so it inflates `versions_written` while leaving the net row
        // count untouched. Comparing `versions_written` against the planned-notice
        // count therefore yields a "shortfall" that is really the set of Tenders that
        // early-returned — and after an epoch bump that shortfall collapses, which
        // looks exactly like the version count having GROWN. It has not.
        //
        // The net count cannot grow through this path: the early return above
        // requires `stored.len() == p.versions.len()`, so a Tender with a SHORT
        // stored chain could never have been skipped — it falls through here and the
        // write loop appends the missing versions on the spot. Chains are repaired by
        // the ordinary path, never left truncated for an epoch bump to find.
        //
        // If a re-fold really does change the net `COUNT(*) FROM tender_versions`,
        // the cause is upstream of this function — a notice planned but absent from
        // the fold chain (`parse_state` no longer 'parsed', so the pre-pass never
        // spilled it; see issue 105) — and it means the grouping moved. Treat it as a
        // stop, not as expected growth.
        for seq in (keep + 1..=stored.len()).rev() {
            self.delete_version(conn, tender_id, seq as i64).await?;
            applied.versions_removed += 1;
        }

        let mut written = WrittenEntities::default();
        for (i, version) in p.versions.iter().enumerate().skip(keep) {
            let seq = i as i64 + 1;
            let previous = i.checked_sub(1).map(|j| &p.versions[j]);
            self.write_version(conn, tender_id, seq, version, stmts, pending, &mut written).await?;
            applied.versions_written += 1;
            applied.changes += self
                .append_version_changes(conn, tender_id, seq, version, previous, now, stmts)
                .await?;
        }

        // The shrinking-rewrite sweep (issue 103). A full rewrite that produces
        // FEWER entities than the stored chain had — the first one was issue 100's
        // fix, which stopped three DE-1.x reference carriers minting phantom
        // bids/contracts — leaves the extra rows in the entity tables with nothing
        // referencing them: `delete_version` clears the version-keyed satellites
        // only, deliberately, so an unchanged rewrite reuses the same surrogate
        // ids and stays byte-identical.
        //
        // Run whenever the chain SHRANK — `keep < stored.len()` — so a partial
        // rewrite that kept a prefix (`keep > 0`) sweeps its dropped tail too
        // (issue 279), not just the `keep == 0` full rewrite (issue 103). The
        // sweep deletes a Tender's entity rows absent from the keep-set, so that
        // set must be COMPLETE: for `keep > 0` the kept prefix's rows were never
        // re-written into `written`, so augment it with them first — otherwise the
        // sweep would delete entities the surviving versions still point at.
        // `!stored.is_empty()` skips the paths that cannot have orphans and would
        // pay SELECTs for nothing — a fresh Tender, and every Tender of a rebuild
        // (empty chain by construction after `reset_tender_layer`). `keep == 0`
        // adds nothing (empty prefix) and stays byte-identical to issue 103, so
        // issue 99's forced-rewrite byte-identity holds by arithmetic as before.
        if !stored.is_empty() && keep < stored.len() {
            if keep > 0 {
                let kept_notices: Vec<i64> =
                    p.versions[..keep].iter().map(|v| v.caused_by_notice_id).collect();
                self.extend_keep_with_kept_prefix(conn, tender_id, &kept_notices, &mut written)
                    .await?;
            }
            let (swept, sweep_changes) =
                self.sweep_orphaned_entities(conn, tender_id, &written, now).await?;
            applied.entities_swept += swept;
            applied.changes += sweep_changes;
        }

        // Record the new head (issue 25): the current version is the last of the
        // chain, its `published_at` the date the "newest Tenders" list orders by.
        // Reached only when the chain changed — the unchanged early-return above
        // leaves an already-correct pointer (set when those versions were written,
        // or by the one-time backfill at open for pre-issue-25 rows).
        if let Some(head) = p.versions.last() {
            stmts
                .head_update
                .execute((
                    Value::Integer(p.versions.len() as i64),
                    Value::Integer(head.published_at),
                    Value::Integer(PROJECTION_EPOCH),
                    head_deadline(head).map(Value::Integer).unwrap_or(Value::Null),
                    head_title(head).map(Value::Text).unwrap_or(Value::Null),
                    Value::Integer(tender_id),
                ))
                .await?;
        }
        // The chain changed, so this run owns this Tender's head pointer — the
        // batch checks it before committing (see `assert_heads_match`).
        Ok((applied, Some(tender_id)))
    }

    /// Issue 27 / task #27. Every Tender this batch rewrote must leave
    /// `tenders.current_seq` equal to `MAX(tender_versions.seq)` — the head
    /// pointer is what `v_tenders` and every satellite view join on
    /// (`canonical.rs` view definitions), so a head that points at a version
    /// that is not the last one silently serves a stale or missing reading of
    /// the Tender, with no error anywhere.
    ///
    /// Run INSIDE the batch transaction, before COMMIT, so a violation rolls the
    /// batch back instead of landing: the projection is re-runnable, a wrong head
    /// is not self-healing.
    ///
    /// **Scoped to the Tenders this run modified**, deliberately. A pre-existing
    /// violation on a Tender the run never touched is the standing snapshot gate's
    /// business (task #28's `head_not_max`); failing the projection for it would
    /// wedge the whole pipeline on damage the projection cannot repair, and a
    /// verifier that stops the daily cycle over old damage gets turned off.
    ///
    /// **This check and that gate are NOT independent confirmations of each
    /// other** (sdk-vendor's observation, sharpened): this one *prevents*, so a
    /// violation it catches never commits, so the snapshot stays clean and the
    /// gate sees nothing. Once this is live, `head_not_max` reporting green means
    /// "no damage **or** damage prevented" — the two are indistinguishable from
    /// the snapshot side. The signal that a violation actually happened is a
    /// FAILED PROJECTION JOB carrying the error below, nowhere else. Reading
    /// gate-green as end-to-end verification of this invariant is the decorative
    /// reading; the gate's real job here is the damage this check cannot see.
    ///
    /// One statement per batch (~512 Tenders), both sides seeking by
    /// `tender_id` — bounded by the work actually done, so a small daily pays a
    /// small price. `IS NOT` rather than `<>` because both sides are nullable and
    /// the dangerous case is exactly a NULL one: a Tender whose versions were all
    /// removed keeps its old `current_seq` (the head update is skipped when the
    /// new chain is empty), leaving a pointer into versions that no longer exist.
    async fn assert_heads_match(conn: &Connection, ids: &[i64]) -> turso::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let places = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT t.id, t.current_seq,
                    (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)
               FROM tenders t
              WHERE t.id IN ({places})
                AND t.current_seq IS NOT
                    (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)
              LIMIT 5"
        );
        let params: Vec<Value> = ids.iter().map(|id| Value::Integer(*id)).collect();
        let mut rows = conn.query(&sql, params).await?;
        let mut bad = Vec::new();
        while let Some(row) = rows.next().await? {
            bad.push(format!(
                "tender {} head current_seq={:?} but MAX(seq)={:?}",
                int(&row, 0),
                row.get_value(1).ok(),
                row.get_value(2).ok(),
            ));
        }
        if bad.is_empty() {
            return Ok(());
        }
        Err(turso::Error::Corrupt(format!(
            "projection would commit a Tender head that is not the last version: {}",
            bad.join("; ")
        )))
    }

    async fn tender_identity(
        &self,
        conn: &Connection,
        p: &TenderProjection,
        now: i64,
        rebuild: bool,
        stmts: &mut TenderInserts,
    ) -> turso::Result<(i64, bool, i64)> {
        // On a rebuild the tender-content layer was just emptied by
        // [`Db::reset_tender_layer`] and every group_key is distinct, so identity is
        // ALWAYS a fresh insert — skip the random-position probe into the (now
        // deferred) identity indexes that steepened the Phase-2 fold (issue 60/62).
        // Byte-identical: clear_canonical never reset sqlite_sequence, so a fresh
        // rebuild already re-inserts at ever-climbing ids and always missed this
        // probe; the reset-to-1 just makes it deterministic (resume == fresh). The
        // incremental/non-rebuild path keeps the probe — it folds touched Tenders
        // against a populated layer.
        if !rebuild {
            // A keyed Tender is found by its procedure key alone — that is what
            // merges a procedure's TED and DÖE readings into one Tender (ADR-0003).
            // Islands stay per-notice.
            let mut rows = match (&p.procedure_key, p.island_notice_id) {
                (Some(key), _) => {
                    conn.query(
                        "SELECT id, source, projection_epoch FROM tenders WHERE procedure_key = ?",
                        (t(key),),
                    )
                    .await?
                }
                (None, Some(notice_id)) => {
                    conn.query(
                        "SELECT id, source, projection_epoch FROM tenders
                          WHERE source = ? AND island_notice_id = ?",
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
                return Ok((id, false, int(&row, 2)));
            }
        }
        stmts
            .tenders
            .execute((
                t(&p.source),
                opt_text(p.procedure_key.as_deref()),
                opt_int(p.island_notice_id),
                t(&p.kind),
                Value::Integer(now),
            ))
            .await?;
        Ok((last_insert_rowid(conn).await?, true, PROJECTION_EPOCH))
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
            "tender_version_lot_group_members",
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

    /// Add the KEPT prefix's still-valid entity ids to `written`, so the orphan
    /// sweep can run on a partial rewrite (`keep > 0`) without deleting rows the
    /// surviving versions still reference (issue 279). `write_version` only records
    /// the versions it re-wrote (seq `keep+1..`), so on a pure tail-drop `written`
    /// is empty and a sweep against it would delete the whole Tender.
    ///
    /// `lots` accumulate and are keyed by `(tender_id, lot_key)`, not by notice, so
    /// a surviving `tender_version_lots` reference — kept prefix OR freshly written —
    /// is the ground truth for which lots are still valid. `lot_results`/`bids`/
    /// `contracts` are keyed by their origin notice, so the kept prefix's are exactly
    /// those under a kept notice (the prefix's content is unchanged — that is why the
    /// chain-as-state-key match kept it).
    async fn extend_keep_with_kept_prefix(
        &self,
        conn: &Connection,
        tender_id: i64,
        kept_notices: &[i64],
        written: &mut WrittenEntities,
    ) -> turso::Result<()> {
        {
            let mut rows = conn
                .query(
                    "SELECT DISTINCT lot_id FROM tender_version_lots WHERE tender_id = ?",
                    (Value::Integer(tender_id),),
                )
                .await?;
            while let Some(row) = rows.next().await? {
                written.lots.insert(int(&row, 0));
            }
        }
        for &notice in kept_notices {
            for (table, set) in [
                ("lot_results", &mut written.lot_results),
                ("bids", &mut written.bids),
                ("contracts", &mut written.contracts),
            ] {
                let mut rows = conn
                    .query(
                        &format!("SELECT id FROM {table} WHERE tender_id = ? AND notice_id = ?"),
                        (Value::Integer(tender_id), Value::Integer(notice)),
                    )
                    .await?;
                while let Some(row) = rows.next().await? {
                    set.insert(int(&row, 0));
                }
            }
        }
        Ok(())
    }

    /// Delete this Tender's entity rows that the just-rewritten chain no longer
    /// references, announcing each on the change feed (issue 103; the feed
    /// discipline is issue 164's — a removal a subscriber cannot see leaves them
    /// holding ghosts). Children first, `lots` last: nothing in the entity
    /// tables references `lots`, but the order costs nothing and reads as intent.
    ///
    /// The per-table read rides each entity table's `tender_id`-leading UNIQUE
    /// index; the orphan set is computed in memory against [`WrittenEntities`]
    /// rather than via `NOT EXISTS` probes into the satellites, which are not
    /// indexed for that shape (see the type's doc).
    async fn sweep_orphaned_entities(
        &self,
        conn: &Connection,
        tender_id: i64,
        written: &WrittenEntities,
        now: i64,
    ) -> turso::Result<(u64, u64)> {
        let (mut swept, mut changes) = (0u64, 0u64);
        for (table, kind, keep) in [
            ("lot_results", "lot_result", &written.lot_results),
            ("bids", "bid", &written.bids),
            ("contracts", "contract", &written.contracts),
            ("lots", "lot", &written.lots),
        ] {
            let mut orphans: Vec<i64> = Vec::new();
            let mut rows = conn
                .query(
                    &format!("SELECT id FROM {table} WHERE tender_id = ?"),
                    (Value::Integer(tender_id),),
                )
                .await?;
            while let Some(row) = rows.next().await? {
                let id = int(&row, 0);
                if !keep.contains(&id) {
                    orphans.push(id);
                }
            }
            drop(rows);
            for id in orphans {
                conn.execute(
                    &format!("DELETE FROM {table} WHERE id = ?"),
                    (Value::Integer(id),),
                )
                .await?;
                append_change(conn, kind, id, None, "removed", now).await?;
                swept += 1;
                changes += 1;
            }
        }
        Ok((swept, changes))
    }

    /// One batch of the issue-234 backfill: collapse duplicate identifier-less
    /// provisional Organizations into their group's minimum id and repoint every
    /// referencing row. Scans up to `batch_rows` in-scope org rows past the
    /// `after` name cursor in `(name_norm, country)` index order and merges the
    /// complete groups it saw; a batch cut mid-name drops that whole trailing
    /// name (its country-groups may be incomplete) and the next batch resumes
    /// there — except when ONE name fills the batch alone, which is refetched
    /// unbounded so it can never stall the walk. Restart-safe without a durable
    /// cursor: merged groups leave the scan's scope as singletons, so a rescan
    /// from `''` redoes nothing.
    ///
    /// The `tender_version_result_winners` PK ends in `organization_id`, so a
    /// lot_result already naming the survivor collides with a repointed loser
    /// row; those loser rows are deleted (`winner_dups`) before the repoint.
    ///
    /// Change feed: each loser gets a `removed` row and each group's survivor a
    /// `changed` one (issue 285 — its merged mention set changes what reads return).
    /// AND every Tender whose party/bid/winner row was repointed gets a `tender`
    /// `changed` row (issue 286): the repoints hit current-version rows and move what
    /// the `buyer`/`winner`/`bidder` org-role filters return, so a subscriber must be
    /// told to re-evaluate the Tender. Without it, a `/v1/tenders?winner=<survivor>`
    /// subscriber never learns a merged-in Tender now matches, and a
    /// `winner=<loser>` one keeps a Tender that no longer does.
    pub async fn merge_provisional_organizations_batch(
        &self,
        batch_rows: i64,
        after: &str,
        dry_run: bool,
    ) -> turso::Result<OrgMergeBatch> {
        let conn = self.conn().await;
        let now = crate::now_unix();
        let mut scanned: Vec<(i64, String, String)> = Vec::new();
        {
            let mut rows =
                conn.query(ORG_MERGE_SCAN_SQL, (t(after), Value::Integer(batch_rows))).await?;
            while let Some(row) = rows.next().await? {
                scanned.push((int(&row, 0), text(&row, 1), text(&row, 2)));
            }
        }
        let full = scanned.len() as i64 == batch_rows;
        let mut report = OrgMergeBatch { done: !full, cursor: after.to_owned(), ..Default::default() };
        if scanned.is_empty() {
            return Ok(report);
        }
        if full {
            let last_name = scanned.last().expect("non-empty").1.clone();
            let cut = scanned.iter().position(|r| r.1 == last_name).expect("last row's name");
            if cut == 0 {
                // The whole batch is one name: refetch it complete, unbounded.
                scanned.clear();
                let mut rows = conn
                    .query(
                        "SELECT id, country FROM organizations \
                          WHERE identifier IS NULL AND country IS NOT NULL AND name_norm = ? \
                          ORDER BY country",
                        (t(&last_name),),
                    )
                    .await?;
                while let Some(row) = rows.next().await? {
                    scanned.push((int(&row, 0), last_name.clone(), text(&row, 1)));
                }
            } else {
                scanned.truncate(cut);
            }
        }
        report.scanned = scanned.len() as u64;
        report.cursor = scanned.last().expect("non-empty").1.clone();

        // The rows arrive in (name_norm, country) order, so groups are adjacent runs.
        let mut groups: Vec<(i64, Vec<i64>)> = Vec::new();
        let mut i = 0;
        while i < scanned.len() {
            let run = scanned[i..]
                .iter()
                .take_while(|r| r.1 == scanned[i].1 && r.2 == scanned[i].2)
                .count();
            if run > 1 {
                let mut ids: Vec<i64> = scanned[i..i + run].iter().map(|r| r.0).collect();
                ids.sort_unstable();
                groups.push((ids[0], ids[1..].to_vec()));
            }
            i += run;
        }
        report.groups = groups.len() as u64;
        report.removed = groups.iter().map(|(_, l)| l.len() as u64).sum();
        if dry_run || groups.is_empty() {
            return Ok(report);
        }

        conn.execute("BEGIN IMMEDIATE", ()).await?;
        // Tenders whose party/bid/winner rows this batch repoints — collected so the
        // change feed learns of the in-place rewrite (issue 286). A BTreeSet across
        // the whole batch dedups a Tender referenced by several losers of one group.
        let mut touched: BTreeSet<i64> = BTreeSet::new();
        for (keep, losers) in &groups {
            for &loser in losers {
                // BEFORE the repoints, while these rows still point at the loser:
                // which Tenders does it touch? Each leg is an index probe on the
                // loser's `*_org` index (bounded by the loser's few references), never
                // a table scan — so this stays cheap on the batched merge path.
                {
                    let mut trows = conn
                        .query(
                            "SELECT tender_id FROM tender_version_parties WHERE organization_id = ? \
                       UNION SELECT tender_id FROM tender_version_bid_parties WHERE organization_id = ? \
                       UNION SELECT tender_id FROM tender_version_result_winners WHERE organization_id = ?",
                            (
                                Value::Integer(loser),
                                Value::Integer(loser),
                                Value::Integer(loser),
                            ),
                        )
                        .await?;
                    while let Some(row) = trows.next().await? {
                        touched.insert(int(&row, 0));
                    }
                }
                report.mentions += conn
                    .execute(
                        "UPDATE organization_mentions SET organization_id = ? \
                          WHERE organization_id = ?",
                        (Value::Integer(*keep), Value::Integer(loser)),
                    )
                    .await?;
                report.parties += conn
                    .execute(
                        "UPDATE tender_version_parties SET organization_id = ? \
                          WHERE organization_id = ?",
                        (Value::Integer(*keep), Value::Integer(loser)),
                    )
                    .await?;
                report.bid_parties += conn
                    .execute(
                        "UPDATE tender_version_bid_parties SET organization_id = ? \
                          WHERE organization_id = ?",
                        (Value::Integer(*keep), Value::Integer(loser)),
                    )
                    .await?;
                report.winner_dups += conn
                    .execute(
                        "DELETE FROM tender_version_result_winners \
                          WHERE organization_id = ? \
                            AND EXISTS (SELECT 1 FROM tender_version_result_winners w \
                                         WHERE w.tender_id = tender_version_result_winners.tender_id \
                                           AND w.seq = tender_version_result_winners.seq \
                                           AND w.lot_result_id = tender_version_result_winners.lot_result_id \
                                           AND w.organization_id = ?)",
                        (Value::Integer(loser), Value::Integer(*keep)),
                    )
                    .await?;
                report.winners += conn
                    .execute(
                        "UPDATE tender_version_result_winners SET organization_id = ? \
                          WHERE organization_id = ?",
                        (Value::Integer(*keep), Value::Integer(loser)),
                    )
                    .await?;
                conn.execute("DELETE FROM organizations WHERE id = ?", (Value::Integer(loser),))
                    .await?;
                append_change(&conn, "organization", loser, None, "removed", now).await?;
            }
            // Issue 285: the survivor's mention set grew, which is a `changed` — NOT
            // "updated", an op outside the documented `added | changed | removed` enum
            // (schema comment above; docs.rs public contract). SSE reclassifies by
            // match-state so it was unaffected, but /v1/changes poll and webhooks pass
            // the raw op through, so an "updated" reached those two transports while SSE
            // showed "added" — three feeds disagreeing on one event. `changed` is the
            // documented, correct value and makes all three agree.
            append_change(&conn, "organization", *keep, None, "changed", now).await?;
        }
        // The Tender-side events (issue 286): one `changed` per touched Tender, seq
        // NULL like the retirement path's in-place tender change — it is not tied to a
        // new version, it re-points existing ones. It tells a `buyer`/`winner`/`bidder`
        // subscriber to re-evaluate the Tender (add it if it now matches the survivor,
        // drop it if it no longer matches the deleted loser).
        for &tid in &touched {
            append_change(&conn, "tender", tid, None, "changed", now).await?;
        }
        report.tender_changes = touched.len() as u64;
        conn.execute("COMMIT", ()).await?;
        // Ring the doorbell for the just-committed change rows (issue 287): without
        // it an SSE subscriber only learns of the merge's events when some LATER
        // write publishes — retirement's `publish_cursor` after-COMMIT pattern.
        self.publish_cursor(&conn).await?;
        Ok(report)
    }

    /// How many org rows are in the merge walk's scope — the progress total for
    /// the backfill job. One full scan; run it once at job start, not per batch.
    pub async fn org_merge_scope_count(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM organizations \
                  WHERE identifier IS NULL AND country IS NOT NULL AND name_norm > ''",
                (),
            )
            .await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// Whether a named index exists — the backfill job's refusal gate: without
    /// `organizations_name_country` (deferred, issues 62/111; built by `reindex`)
    /// the merge scan would sort 24.6M rows per batch instead of walking an index.
    pub async fn has_index(&self, name: &str) -> turso::Result<bool> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?", (t(name),))
            .await?;
        Ok(rows.next().await?.is_some())
    }

    async fn write_version(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        v: &TenderVersion,
        stmts: &mut TenderInserts,
        pending: &mut Pending,
        written: &mut WrittenEntities,
    ) -> turso::Result<()> {
        pending.versions.extend([
            Value::Integer(tender_id),
            Value::Integer(seq),
            Value::Integer(v.caused_by_notice_id),
            Value::Integer(v.published_at),
            opt_int(v.dispatched_at),
            opt_text(v.notice_subtype.as_deref()),
            t(&v.publication_id),
        ]);
        self.write_facts(tender_id, seq, None, &v.facts, pending);
        for lot in &v.lots {
            let lot_id = self.lot_identity(conn, tender_id, &lot.key, stmts).await?;
            written.lots.insert(lot_id);
            pending.version_lots.extend([
                Value::Integer(tender_id),
                Value::Integer(seq),
                Value::Integer(lot_id),
                t(&lot.kind),
            ]);
            self.write_facts(tender_id, seq, Some(lot_id), &lot.facts, pending);
        }
        // After the lots loop on purpose: both ends of a membership pair are lots this
        // same version publishes, so by here they are already in `lots` and
        // `lot_identity` is a lookup rather than an insert. A pair naming a lot the
        // notice does NOT publish would create the row — that shape is not in the
        // corpus we have seen, and creating it is better than dropping a published
        // composition silently, which is the failure this table exists to end.
        for (group_key, member_key) in &v.group_members {
            let group_lot_id = self.lot_identity(conn, tender_id, group_key, stmts).await?;
            let member_lot_id = self.lot_identity(conn, tender_id, member_key, stmts).await?;
            written.lots.insert(group_lot_id);
            written.lots.insert(member_lot_id);
            pending.lot_group_members.extend([
                Value::Integer(tender_id),
                Value::Integer(seq),
                Value::Integer(group_lot_id),
                Value::Integer(member_lot_id),
            ]);
        }
        for round in &v.rounds {
            self.write_round(conn, tender_id, seq, round, stmts, pending, written).await?;
        }
        Ok(())
    }

    async fn write_round(
        &self,
        conn: &Connection,
        tender_id: i64,
        seq: i64,
        round: &Round,
        stmts: &mut TenderInserts,
        pending: &mut Pending,
        written: &mut WrittenEntities,
    ) -> turso::Result<()> {
        let scope = || (Value::Integer(tender_id), Value::Integer(seq));
        for result in &round.lot_results {
            let id = self
                .result_identity(conn, "lot_results", "result_key", tender_id, round.notice_id, &result.key, stmts)
                .await?;
            let lot_id = self.result_lot(conn, tender_id, result.lot_key.as_deref(), stmts).await?;
            written.lot_results.insert(id);
            written.lots.extend(lot_id);
            let (a, b) = scope();
            pending.lot_results.extend([
                a,
                b,
                Value::Integer(id),
                opt_int(lot_id),
                opt_text(result.decision.as_deref()),
                opt_text(result.reason.as_deref()),
                opt_int(result.awarded_cents),
                opt_text(result.awarded_currency.as_deref()),
                opt_int(result.decided.map(|(utc, _, _)| utc)),
                opt_int(result.decided.map(|(_, offset, _)| offset)),
                opt_int(result.decided.map(|(_, _, has_time)| i64::from(has_time))),
            ]);
            for organization_id in &result.winners {
                let (a, b) = scope();
                pending
                    .result_winners
                    .extend([a, b, Value::Integer(id), Value::Integer(*organization_id)]);
            }
            for (kind, count) in &result.statistics {
                let (a, b) = scope();
                pending
                    .result_stats
                    .extend([a, b, Value::Integer(id), t(kind), Value::Integer(*count)]);
            }
        }
        for bid in &round.bids {
            let id = self
                .result_identity(conn, "bids", "bid_key", tender_id, round.notice_id, &bid.key, stmts)
                .await?;
            let lot_id = self.result_lot(conn, tender_id, bid.lot_key.as_deref(), stmts).await?;
            written.bids.insert(id);
            written.lots.extend(lot_id);
            let (a, b) = scope();
            pending.bids.extend([
                a,
                b,
                Value::Integer(id),
                opt_int(lot_id),
                opt_int(bid.cents),
                opt_text(bid.currency.as_deref()),
            ]);
            for party in &bid.parties {
                let (a, b) = scope();
                pending.bid_parties.extend([
                    a,
                    b,
                    Value::Integer(id),
                    t(&party.role),
                    Value::Integer(party.organization_id),
                    Value::Integer(round.notice_id),
                    t(&party.section_id),
                ]);
            }
        }
        for contract in &round.contracts {
            let id = self
                .result_identity(conn, "contracts", "contract_key", tender_id, round.notice_id, &contract.key, stmts)
                .await?;
            written.contracts.insert(id);
            let (a, b) = scope();
            let stamp = |at: Option<(i64, i64, bool)>| match at {
                Some((utc, offset, has_time)) => {
                    (Some(utc), Some(offset), Some(i64::from(has_time)))
                }
                None => (None, None, None),
            };
            let (utc, offset, has_time) = stamp(contract.concluded);
            let (d_utc, d_offset, d_has_time) = stamp(contract.decided);
            pending.contracts.extend([
                a,
                b,
                Value::Integer(id),
                opt_text(contract.buyer_contract_id.as_deref()),
                opt_int(utc),
                opt_int(offset),
                opt_int(has_time),
                opt_int(d_utc),
                opt_int(d_offset),
                opt_int(d_has_time),
                opt_int(contract.cents),
                opt_text(contract.currency.as_deref()),
            ]);
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
        stmts: &mut TenderInserts,
    ) -> turso::Result<i64> {
        // The dedup SELECT is one of three fixed shapes (lot_results/bids/
        // contracts), so it is prepared once and reused (issue 67) rather than
        // re-planned per call. It is load-bearing even on a rebuild — it dedupes a
        // result across a tender's re-published rounds. The INSERT stays a
        // dynamic-table statement (low volume, miss-only).
        let lookup = match table {
            "lot_results" => &mut stmts.lot_result_lookup,
            "bids" => &mut stmts.bid_lookup,
            "contracts" => &mut stmts.contract_lookup,
            _ => unreachable!("a results entity is one of the three fixed shapes"),
        };
        let mut rows = lookup
            .query((Value::Integer(tender_id), Value::Integer(notice_id), t(key)))
            .await?;
        if let Some(row) = rows.next().await? {
            return Ok(int(&row, 0));
        }
        drop(rows);
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
        stmts: &mut TenderInserts,
    ) -> turso::Result<Option<i64>> {
        Ok(match lot_key {
            Some(key) => Some(self.lot_identity(conn, tender_id, key, stmts).await?),
            None => None,
        })
    }

    /// Buffer a version's facts into the batchable satellites (issue 67). These
    /// tables carry no emitted/referenced surrogate id, so their row order is
    /// unobservable; the rows are flushed as multi-row INSERTs at end of batch.
    /// Pure accumulation — no SQL runs here, so it is synchronous.
    fn write_facts(
        &self,
        tender_id: i64,
        seq: i64,
        lot_id: Option<i64>,
        facts: &BTreeSet<Fact>,
        pending: &mut Pending,
    ) {
        let scope = || (Value::Integer(tender_id), Value::Integer(seq), opt_int(lot_id));
        for fact in facts {
            let (a, b, c) = scope();
            match fact {
                Fact::Text { field, lang, value } => {
                    pending.texts.extend([a, b, c, t(field), opt_text(lang.as_deref()), t(value)]);
                }
                Fact::Amount { field, cents, currency, tax_basis } => {
                    pending.amounts.extend([
                        a,
                        b,
                        c,
                        t(field),
                        Value::Integer(*cents),
                        t(currency),
                        opt_text(tax_basis.as_deref()),
                    ]);
                }
                Fact::Classification { field, scheme, code } => {
                    pending.classifications.extend([a, b, c, t(field), t(scheme), t(code)]);
                }
                Fact::Date { field, utc_seconds, offset_minutes, has_time } => {
                    pending.dates.extend([
                        a,
                        b,
                        c,
                        t(field),
                        Value::Integer(*utc_seconds),
                        Value::Integer(*offset_minutes),
                        Value::Integer(i64::from(*has_time)),
                    ]);
                }
                Fact::Party { role, organization_id, notice_id, section_id } => {
                    pending.parties.extend([
                        a,
                        b,
                        c,
                        t(role),
                        Value::Integer(*organization_id),
                        Value::Integer(*notice_id),
                        t(section_id),
                    ]);
                }
            }
        }
    }

    async fn lot_identity(
        &self,
        conn: &Connection,
        tender_id: i64,
        key: &str,
        stmts: &mut TenderInserts,
    ) -> turso::Result<i64> {
        let mut rows = stmts
            .lot_lookup
            .query((Value::Integer(tender_id), t(key)))
            .await?;
        if let Some(row) = rows.next().await? {
            return Ok(int(&row, 0));
        }
        drop(rows);
        stmts.lots.execute((Value::Integer(tender_id), t(key))).await?;
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
        stmts: &mut TenderInserts,
    ) -> turso::Result<u64> {
        let mut count = 0;
        match previous {
            None => {
                append_change(conn, "tender", tender_id, Some(seq), "added", now).await?;
                count += 1;
            }
            // group_members too (issue 283): a version whose only delta is
            // lots-group composition writes new membership rows but was otherwise
            // invisible to the change feed. The fold keeps group_members in
            // canonical sorted order (project.rs sort_unstable+dedup; silent
            // versions clone the already-sorted prev), so a Vec compare is a set
            // compare — no spurious change on a byte-identical re-fold.
            Some(prev)
                if prev.facts != v.facts
                    || prev.rounds != v.rounds
                    || prev.group_members != v.group_members =>
            {
                append_change(conn, "tender", tender_id, Some(seq), "changed", now).await?;
                count += 1;
            }
            Some(_) => {}
        }

        let previous_lots = previous.map(|p| p.lots.as_slice()).unwrap_or_default();
        for lot in &v.lots {
            let lot_id = self.lot_identity(conn, tender_id, &lot.key, stmts).await?;
            let op = match previous_lots.iter().find(|l| l.key == lot.key) {
                None => "added",
                Some(before) if before.facts != lot.facts || before.kind != lot.kind => "changed",
                Some(_) => continue,
            };
            append_change(conn, "lot", lot_id, Some(seq), op, now).await?;
            count += 1;
        }
        for gone in previous_lots.iter().filter(|l| !v.lots.iter().any(|n| n.key == l.key)) {
            let lot_id = self.lot_identity(conn, tender_id, &gone.key, stmts).await?;
            append_change(conn, "lot", lot_id, Some(seq), "removed", now).await?;
            count += 1;
        }

        let previous_rounds = previous.map(|p| p.rounds.as_slice()).unwrap_or_default();
        count += self
            .append_round_changes(conn, tender_id, seq, previous_rounds, &v.rounds, now, stmts)
            .await?;
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
        stmts: &mut TenderInserts,
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
                let id = self.result_identity(conn, table, column, tender_id, *notice_id, key, stmts).await?;
                append_change(conn, kind, id, Some(seq), op, now).await?;
                count += 1;
            }
            for (notice_id, key, _) in &prev {
                if curr.iter().any(|(n, k, _)| n == notice_id && k == key) {
                    continue;
                }
                let id = self.result_identity(conn, table, column, tender_id, *notice_id, key, stmts).await?;
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

        self.retire_tenders_chunked(&conn, &absorbed, now, NODE_WRITE_BATCH, "absorbed legacy").await?;
        self.publish_cursor(&conn).await?;
        Ok(absorbed.len() as u64)
    }

    /// Every content table a retired Tender's rows must be deleted from, in
    /// dependency order (satellites before the entities they reference).
    const RETIRE_TABLES: [&'static str; 18] = [
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
        "tender_version_lot_group_members",
        "tender_version_lots",
        "tender_versions",
        "lot_results",
        "bids",
        "contracts",
        "lots",
    ];

    /// Retire `ids` in BOUNDED chunks — the shared body of both retirement paths
    /// (issue 93).
    ///
    /// Retirement used to run as one unbounded `BEGIN IMMEDIATE`, ~23 statements per
    /// Tender, with no checkpoint and no output. On the eForms-DE 1.x re-fold that
    /// was 216,450 Tenders → ~5M statements in a single transaction taking 5m45s,
    /// which read as a wedge and came within minutes of being killed. It is the one
    /// bulk writer here that issue 63 never chunked; every other one already commits
    /// and TRUNCATE-checkpoints in batches.
    ///
    /// Each chunk is independently consistent — a retirement is a whole Tender's
    /// removal plus its `removed` events, and a chunk never splits one — so an
    /// interruption leaves earlier chunks retired and later ones simply still
    /// orphaned, which the next run re-derives and finishes. That is strictly better
    /// than the previous all-or-nothing transaction, whose failure mode at scale was
    /// a WAL/RAM balloon and then losing every retirement to the rollback.
    async fn retire_tenders_chunked(
        &self,
        conn: &Connection,
        ids: &[i64],
        now: i64,
        batch: usize,
        what: &str,
    ) -> turso::Result<()> {
        let total = ids.len();
        let mut done = 0usize;
        for chunk in ids.chunks(batch.max(1)) {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            match self.retire_chunk_tx(conn, chunk, now).await {
                Ok(()) => conn.execute("COMMIT", ()).await?,
                Err(e) => {
                    let _ = conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
            };
            let _ = checkpoint_on(conn, CheckpointMode::Truncate).await;
            done += chunk.len();
            eprintln!("[project] retire {what}: {done}/{total} Tenders retired");
        }
        Ok(())
    }

    /// One chunk's retirement, inside the caller's transaction.
    ///
    /// The `removed` change events are emitted **per Tender, in the caller's id
    /// order**, and within a Tender in the same kind order as before — the `changes`
    /// feed is ordered by its autoincrement cursor and is part of the projection's
    /// byte-identity surface, so that sequence is not free to change. Only the
    /// DELETEs are batched: they are ~88% of the per-Tender cost (17 statements at
    /// ~0.09 ms against 4 entity reads at ~0.013 ms), and batching them is invisible
    /// to the feed because a DELETE writes no change row. Doing all reads before any
    /// delete is equivalent — a Tender's deletes never touch another Tender's rows.
    async fn retire_chunk_tx(&self, conn: &Connection, ids: &[i64], now: i64) -> turso::Result<()> {
        for &tender_id in ids {
            append_change(conn, "tender", tender_id, None, "removed", now).await?;
            for (kind, table) in [
                ("lot", "lots"),
                ("lot_result", "lot_results"),
                ("bid", "bids"),
                ("contract", "contracts"),
            ] {
                let mut rows = conn
                    .query(
                        &format!("SELECT id FROM {table} WHERE tender_id = ?"),
                        (Value::Integer(tender_id),),
                    )
                    .await?;
                let mut entities = Vec::new();
                while let Some(row) = rows.next().await? {
                    entities.push(int(&row, 0));
                }
                drop(rows);
                for entity in entities {
                    append_change(conn, kind, entity, None, "removed", now).await?;
                }
            }
        }
        for table in Self::RETIRE_TABLES {
            for part in ids.chunks(IN_CHUNK) {
                let sql =
                    format!("DELETE FROM {table} WHERE tender_id IN ({})", placeholders(part.len()));
                let params: Vec<Value> = part.iter().map(|&i| Value::Integer(i)).collect();
                conn.execute(&sql, params).await?;
            }
        }
        for part in ids.chunks(IN_CHUNK) {
            let sql = format!("DELETE FROM tenders WHERE id IN ({})", placeholders(part.len()));
            let params: Vec<Value> = part.iter().map(|&i| Value::Integer(i)).collect();
            conn.execute(&sql, params).await?;
        }
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
    pub async fn retire_regrouped_tenders(&self, touched: &[i64], now: i64) -> turso::Result<u64> {
        self.retire_regrouped_tenders_chunked(touched, now, NODE_WRITE_BATCH).await
    }

    /// As [`Db::retire_regrouped_tenders`], with an explicit retirement chunk size.
    /// Exposed so the chunk-invariance test can drive a tiny chunk and prove the
    /// canonical layer — the cursor-ordered `changes` feed included — is identical
    /// however the retirement is split (issue 93); production uses
    /// [`NODE_WRITE_BATCH`].
    pub async fn retire_regrouped_tenders_chunked(
        &self,
        touched: &[i64],
        now: i64,
        chunk: usize,
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
        self.retire_tenders_chunked(&conn, &orphaned, now, chunk, "regrouped").await?;
        self.publish_cursor(&conn).await?;
        Ok(orphaned.len() as u64)
    }

    /// The full-projection counterpart of [`Db::retire_regrouped_tenders`], for the
    /// two shapes [`Db::retire_absorbed_legacy_tenders`] does NOT cover (issue 278):
    /// a **uuid-keyed** tender (`procedure_key` not `ojs:%`) and an **island**
    /// tender (`procedure_key IS NULL`) whose group_key the freshly-built plan no
    /// longer produces — a notice that regrouped to another key (BT-04 instability,
    /// issue 236) leaving its old tender behind. The full non-rebuild path only
    /// retired `ojs:%` legacy keys, so every reparse-driven regroup that tripped the
    /// incremental→full fallback left a ghost; the snapshot measured ~45k of them.
    ///
    /// Corpus-wide and set-based (NOT the scoped per-tender loop — that would be one
    /// query per tender over ~8M rows): two `NOT EXISTS` anti-joins seeking
    /// `plan_notice(group_key, …)` by its leading column. Legacy `ojs:%` is left to
    /// `retire_absorbed_legacy_tenders`, whose proven behaviour this does not touch;
    /// the two passes partition the key space (`ojs:%` vs uuid vs NULL). Byte-identity
    /// with a from-scratch rebuild holds by construction: a rebuild resets the layer
    /// so a survivor's key is always produced, making this a no-op there, and on the
    /// non-rebuild path the surviving set becomes exactly {keys in plan} — the rebuild
    /// set. Retires chunked with `removed` change events, like both siblings.
    pub async fn retire_regrouped_nonlegacy_tenders(&self, now: i64) -> turso::Result<u64> {
        let conn = self.conn().await;
        let mut orphaned = Vec::new();
        // uuid-keyed (not legacy ojs:%) whose key the plan no longer produces.
        let mut rows = conn
            .query(
                "SELECT t.id FROM tenders t
                  WHERE t.procedure_key IS NOT NULL
                    AND t.procedure_key NOT LIKE 'ojs:%'
                    AND NOT EXISTS (SELECT 1 FROM plan_notice p WHERE p.group_key = t.procedure_key)",
                (),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            orphaned.push(int(&row, 0));
        }
        drop(rows);
        // island tenders (no key) whose `island:<notice>` key the plan no longer produces.
        let mut rows = conn
            .query(
                "SELECT t.id FROM tenders t
                  WHERE t.procedure_key IS NULL
                    AND NOT EXISTS (
                      SELECT 1 FROM plan_notice p
                       WHERE p.group_key = 'island:' || t.island_notice_id)",
                (),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            orphaned.push(int(&row, 0));
        }
        drop(rows);
        if orphaned.is_empty() {
            return Ok(0);
        }
        self.retire_tenders_chunked(&conn, &orphaned, now, NODE_WRITE_BATCH, "regrouped non-legacy").await?;
        self.publish_cursor(&conn).await?;
        Ok(orphaned.len() as u64)
    }

    /// The notices whose `caused_by_notice_id` appears under 2+ distinct Tenders —
    /// the issue-278 ghost signature. `plan_notice.notice_id` is a PK, so a notice
    /// keys to exactly ONE group; a notice under two Tenders means one of them is a
    /// stale ghost the full non-rebuild path failed to retire (the ~45k measured on
    /// prod). The cleanup (`sweep-regrouped-ghosts`) marks these unprojected so an
    /// ordinary incremental fold re-derives each under its single current key and
    /// its `retire_regrouped_tenders` drops whichever member is no longer produced.
    /// One `GROUP BY` over `tender_versions` — bounded result (~45k), ~6s at prod
    /// scale (measured); run once per sweep, not on any hot path.
    pub async fn regrouped_dup_notice_ids(&self) -> turso::Result<Vec<i64>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT caused_by_notice_id FROM tender_versions
                  WHERE caused_by_notice_id IS NOT NULL
                  GROUP BY caused_by_notice_id
                 HAVING COUNT(DISTINCT tender_id) > 1",
                (),
            )
            .await?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(int(&row, 0));
        }
        Ok(ids)
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

    /// Execute one write statement outside every guarded path — for TESTS that
    /// need surgical corpus damage no real API produces (issue 109's shell
    /// reproduction strips a cohort's satellites while its version rows
    /// survive, which is the incident's exact shape). Sibling of
    /// [`Db::set_projection_epoch_for_test`]; not an API surface — the guarded
    /// public SQL endpoint is issue 07.
    pub async fn execute_for_test(&self, sql: &str) -> turso::Result<u64> {
        let conn = self.conn().await;
        conn.execute(sql, ()).await
    }

    /// How many notices are parsed — the projection's Phase-1 total, read once up
    /// front so it can log a progress fraction over a run that spans many minutes.
    pub async fn parsed_notice_count(&self) -> turso::Result<u64> {
        let conn = self.reader().await?;
        let mut rows =
            conn.query("SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed'", ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0) as u64))
    }

    /// The largest parsed notice id — the top of the id space the sharded Phase-2
    /// pre-pass (issue 66) partitions into contiguous worker stripes. `0` when no
    /// notices are parsed.
    pub async fn max_parsed_notice_id(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT COALESCE(MAX(id), 0) FROM notices WHERE parse_state = 'parsed'", ())
            .await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// Observe whether each canonical table the standing gate's `present_*`
    /// checks cover currently holds any rows, and fold that against what was
    /// observed before (issue 133 / task #38).
    ///
    /// ## Why this exists at all
    ///
    /// The #28 verification scheme is moving from whole-corpus counting to
    /// assertions made incrementally, as the projection writes. That covers
    /// every invariant ABOUT rows, and it is structurally blind to exactly one
    /// thing: **there being no rows**. An assertion scoped to what a batch
    /// wrote cannot notice an empty table, and the catastrophic case — a
    /// rebuild killed after `reset_tender_layer` has already committed its
    /// DROP, which is what happened on 2026-07-30 — writes no batch at all, so
    /// no assertion ever runs. Every other check then reports clean over the
    /// wreckage, because vacuously true is still true.
    ///
    /// ## Why the verdict is a transition, not "is it empty"
    ///
    /// Asserting non-emptiness outright would fire on states that are correct:
    /// a fresh install has never had a canonical layer, and a rebuild empties
    /// the layer at its start by design. A check that cries wolf on a normal
    /// rebuild is a check someone turns off.
    ///
    /// So the question asked is *"was this populated, and is it now empty"* —
    /// which needs the one fact the database cannot tell you about itself, and
    /// is why `layer_presence` is stored rather than derived.
    ///
    /// ## What this deliberately does NOT decide
    ///
    /// It does not decide whether an empty-having-been-populated table is an
    /// emergency. It cannot: a rebuild in flight produces the identical
    /// observation, and whether one is in flight is the supervisor's knowledge,
    /// not the store's. The caller joins this with that (see
    /// `Supervisor::heavy_write_in_progress`) and classifies. Reporting the
    /// state and classifying it are kept apart on purpose — a detector that
    /// suppressed its own signal whenever a projection was running would have
    /// been silent for the entire hours-long window that mattered on 07-30,
    /// since a killed job is recovered and re-run at boot (issue 21).
    ///
    /// Cost is O(1) per table: `EXISTS` stops at the first row, so this is 13
    /// index/table seeks and no scan, safe to run against the live database on
    /// any cadence. Verdict rows are rewritten only when a table's state
    /// actually CHANGES, but every observation stamps `observed_at`: the
    /// staleness clause in `/health/deep` reads that stamp as "somebody is
    /// still looking", so an observation that leaves no trace is
    /// indistinguishable from an observer that has died. The first cut wrote
    /// only on transitions — a quiet box froze at the last heavy-write touch
    /// and tripped the staleness alarm six hours later, every day (issue 161).
    pub async fn observe_layer_presence(&self, now: i64) -> turso::Result<Vec<LayerPresence>> {
        let conn = self.conn().await;
        let mut out = Vec::with_capacity(PRESENCE_TABLES.len());
        for name in PRESENCE_TABLES {
            // EXISTS, not COUNT: the point is to stop at the first row rather
            // than to know how many there are, and on a 12M-row table those are
            // very different queries.
            let non_empty = {
                let mut rows = conn
                    .query(&format!("SELECT EXISTS(SELECT 1 FROM {name})"), ())
                    .await?;
                rows.next().await?.is_some_and(|row| int(&row, 0) != 0)
            };

            let prior = {
                let mut rows = conn
                    .query(
                        "SELECT ever_populated, went_empty_at FROM layer_presence WHERE name = ?1",
                        turso::params::Params::Positional(vec![Value::Text(name.to_string())]),
                    )
                    .await?;
                rows.next().await?.map(|row| (int(&row, 0) != 0, opt_int_of(&row, 1)))
            };
            let (was_populated, went_empty_at) = prior.unwrap_or((false, None));

            let state = match (non_empty, was_populated) {
                (true, _) => LayerState::Populated,
                // Empty and never seen otherwise: a fresh or not-yet-projected
                // database. Silent by construction — this is the false alarm
                // the whole design is arranged to avoid.
                (false, false) => LayerState::NeverPopulated,
                // The one that matters: it held rows, and now it does not.
                (false, true) => LayerState::WentEmpty { at: went_empty_at.unwrap_or(now) },
            };

            // Write only on a real change of state, so a probe running every
            // minute against a healthy layer performs no writes at all.
            let next_went_empty = match state {
                LayerState::WentEmpty { at } => Some(at),
                _ => None,
            };
            let next_ever = was_populated || non_empty;
            let changed = prior.is_none()
                || next_ever != was_populated
                || next_went_empty != went_empty_at;
            if changed {
                conn.execute(
                    "INSERT INTO layer_presence(name, ever_populated, went_empty_at, observed_at)
                          VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(name) DO UPDATE SET
                         ever_populated = excluded.ever_populated,
                         went_empty_at  = excluded.went_empty_at,
                         observed_at    = excluded.observed_at",
                    turso::params::Params::Positional(vec![
                        Value::Text(name.to_string()),
                        Value::Integer(i64::from(next_ever)),
                        next_went_empty.map_or(Value::Null, Value::Integer),
                        Value::Integer(now),
                    ]),
                )
                .await?;
            }

            out.push(LayerPresence { name: name.to_string(), state });
        }

        // The observation IS the heartbeat. Rows exist for every table after
        // the loop above (a first observation inserts them), so this blanket
        // stamp is what keeps a healthy quiet box out of the staleness alarm.
        conn.execute(
            "UPDATE layer_presence SET observed_at = ?1",
            turso::params::Params::Positional(vec![Value::Integer(now)]),
        )
        .await?;

        Ok(out)
    }

    /// Mark the presence observations as still current WITHOUT re-evaluating
    /// them (issue 133 / task #38).
    ///
    /// Used while a rebuild is in flight. The layer is legitimately empty then,
    /// so observing would record a wipe that did not happen — but simply not
    /// observing would let `observed_at` age past the staleness threshold and
    /// raise the other alarm instead. Both are false positives; this is the
    /// narrow path between them, and it says exactly what is true: nobody has
    /// re-checked, and that is on purpose.
    ///
    /// Deliberately does NOT create rows. On a box that has never observed
    /// there is nothing to keep fresh, and inventing `NeverPopulated` rows here
    /// would manufacture an observation that never happened.
    pub async fn touch_layer_presence(&self, now: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE layer_presence SET observed_at = ?1",
            turso::params::Params::Positional(vec![Value::Integer(now)]),
        )
        .await?;
        Ok(())
    }

    /// The two facts the projection's wipe guards need (issue 133): does
    /// `tenders` hold rows NOW, and has it ever held rows. The witness lives in
    /// `layer_presence`, its own table, so it survives `reset_tender_layer`'s
    /// DROP — which is the point: empty-but-ever-populated is the wipe
    /// signature. Absent witness row means never observed, and reads as
    /// never-populated: the guards stay silent on a box nobody watches rather
    /// than inventing an observation (same doctrine as
    /// [`Db::touch_layer_presence`]).
    pub async fn tender_layer_state(&self) -> turso::Result<(bool, bool)> {
        let conn = self.conn().await;
        let mut rows = conn.query("SELECT EXISTS(SELECT 1 FROM tenders)", ()).await?;
        let populated = rows.next().await?.is_some_and(|row| int(&row, 0) != 0);
        drop(rows);
        let mut rows = conn
            .query("SELECT ever_populated FROM layer_presence WHERE name = 'tenders'", ())
            .await?;
        let ever = rows.next().await?.is_some_and(|row| int(&row, 0) != 0);
        Ok((populated, ever))
    }

    /// The stored presence verdicts, WITHOUT observing (issue 133 / task #38).
    ///
    /// [`Db::observe_layer_presence`] takes the writer connection, because on a
    /// transition it has to record one. That makes it wrong for `/health/deep`:
    /// a probe that wants the writer blocks behind a running projection, times
    /// out, and reports the service unhealthy for the sin of being busy — an
    /// alarm caused by the alarm. So observing is a supervisor job, and the
    /// probe reads what it left, on the reader pool.
    ///
    /// The cost is that this answer is only as fresh as the last observation,
    /// which is why `observed_at` comes back with it and the caller must judge
    /// staleness. A presence check whose observer has silently stopped reports
    /// a confident, plausible, permanently-green answer — the exact shape of
    /// failure this whole layer exists to catch, so it must not be reproduced
    /// here.
    /// Store a rendered operator report under `kind`, replacing any previous one
    /// (issue 230). One row per kind: the latest measurement is the only one worth
    /// keeping, and a history of multi-minute scans is not worth its storage.
    pub async fn put_report(&self, kind: &str, body: &str, computed_at: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO reports(kind, computed_at, body) VALUES(?, ?, ?)
               ON CONFLICT(kind) DO UPDATE SET computed_at = excluded.computed_at,
                                               body = excluded.body",
            (t(kind), Value::Integer(computed_at), t(body)),
        )
        .await?;
        Ok(())
    }

    /// The latest stored report of `kind` and when it was computed, or `None` if
    /// no run has produced one yet. Reader pool: this is a point lookup a request
    /// path may serve, unlike the measurement that produced it.
    pub async fn latest_report(&self, kind: &str) -> turso::Result<Option<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT body, computed_at FROM reports WHERE kind = ?", (t(kind),))
            .await?;
        Ok(rows.next().await?.map(|row| (text(&row, 0), int(&row, 1))))
    }

    /// Every stored report's kind and when it was computed — the stamps without the
    /// bodies (issue 230).
    ///
    /// `reports` holds one row per kind (newest wins), so this is bounded by how
    /// many kinds the code writes, not by the corpus. Separate from
    /// [`Db::latest_report`] so a `/metrics` scrape can expose report freshness
    /// without dragging a multi-kilobyte body across on every scrape.
    pub async fn report_stamps(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn.query("SELECT kind, computed_at FROM reports", ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1)));
        }
        Ok(out)
    }

    /// Run one read-only aggregate on the READER POOL and return its rows as
    /// JSON, for a measurement job that would not survive the `/v1/sql` deadline
    /// (issue 230: the data-quality report's eleven queries each exceed the 10 s
    /// cap at full-corpus scale).
    ///
    /// No sandbox, no allow-list, no time limit — so this is NOT reachable from a
    /// request path and must never become so: the caller is a supervisor job,
    /// which is serialized against other jobs and visible in the job log. It runs
    /// on the reader pool rather than the writer, so a multi-minute scan cannot
    /// block ingestion; it can still pin the WAL, which is why the caller gates
    /// on the queue being its own (jobs run one at a time) rather than racing a
    /// projection.
    /// Rows come back as turso values, not JSON: `store` carries no JSON
    /// dependency and the only consumer already has one, so the mapping belongs
    /// at the call site.
    pub async fn measure_rows(&self, sql: &str) -> turso::Result<Vec<Vec<Value>>> {
        let conn = self.reader().await?;
        let mut rows = conn.query(sql, ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let mut cells = Vec::new();
            let mut i = 0;
            while let Ok(v) = row.get_value(i) {
                cells.push(v);
                i += 1;
            }
            out.push(cells);
        }
        Ok(out)
    }

    pub async fn read_layer_presence(&self) -> turso::Result<Vec<(LayerPresence, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT name, ever_populated, went_empty_at, observed_at FROM layer_presence",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let name = text(&row, 0);
            let ever = int(&row, 1) != 0;
            let went_empty_at = opt_int_of(&row, 2);
            let observed_at = int(&row, 3);
            let state = match (went_empty_at, ever) {
                (Some(at), _) => LayerState::WentEmpty { at },
                (None, true) => LayerState::Populated,
                (None, false) => LayerState::NeverPopulated,
            };
            out.push((LayerPresence { name, state }, observed_at));
        }
        Ok(out)
    }


    /// The `(min, max)` notice id the current grouping plan covers, or `None` when
    /// the plan is empty. The Phase-2 pre-pass sweeps notice ids in order and skips
    /// anything absent from the plan, so ids outside this range are pure waste —
    /// this is what bounds the sweep to the part of the id space that can produce a
    /// row (issue 94). On a full rebuild the plan covers the whole corpus and the
    /// range degenerates to the whole id space, which is exactly right.
    pub async fn plan_notice_id_range(&self) -> turso::Result<Option<(i64, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn.query("SELECT MIN(notice_id), MAX(notice_id) FROM plan_notice", ()).await?;
        let Some(row) = rows.next().await? else { return Ok(None) };
        Ok(match (opt_int_of(&row, 0), opt_int_of(&row, 1)) {
            (Some(lo), Some(hi)) => Some((lo, hi)),
            _ => None,
        })
    }

    /// Split `(lo, hi]` into at most `k` contiguous notice-id stripes holding ~the
    /// same number of PARSED notices each — the Phase-2 pre-pass's work partition
    /// (issue 94).
    ///
    /// The pre-pass used to stripe by equal id WIDTH, which silently assumes notices
    /// are uniformly dense across id space. They are not: `MAX(id)` sits far above
    /// the dense region and bulk reclaims append late, so equal-width stripes put
    /// nearly all the work in one worker while the others finish instantly on empty
    /// id space — the sharded sweep then runs at ~1× however many workers it has.
    /// Striping by parsed-notice COUNT makes every worker's stripe genuinely
    /// equal-cost whatever the id distribution.
    ///
    /// **Streams; never sorts.** `notices.id` is `INTEGER PRIMARY KEY`, i.e. the
    /// rowid, so `ORDER BY id` is free — the scan is already in that order and no
    /// sort is materialised. Verified against turso rather than assumed, because a
    /// filesort here would buffer ~14.2M ids (~100 MB+) and quietly break the
    /// bounded-memory guarantee this whole path exists to keep:
    ///
    /// ```text
    /// EXPLAIN QUERY PLAN → SEARCH notices USING INTEGER PRIMARY KEY (rowid=?)
    /// 800k rows: time-to-first-row 0.0000s, time-to-last-row 0.272s
    /// ```
    ///
    /// The first-row latency is the load-bearing half: a sort cannot emit row 1
    /// until it has consumed every row, so a ratio of 0.00015 is proof of streaming
    /// independent of how the plan text is worded.
    ///
    /// Note the planner picks the **rowid range seek**, not `notices_parse_state`,
    /// and that is the better plan here: it seeks straight to `id > lo` and stops at
    /// `id <= hi`, whereas forcing `INDEXED BY notices_parse_state` walks every
    /// `parsed` entry and filters (measured: `SEARCH … USING INDEX
    /// notices_parse_state (parse_state=?)`, no id range applied). Since this is
    /// normally called over a SCOPED range, forcing the compact index would be a
    /// pessimisation. It reads table rows rather than index entries, so the two
    /// scans (count, then split points) are not free — but they are one bounded pass
    /// over a range the sweep is about to read anyway, and they warm it.
    ///
    /// Returns `(lo, hi]`-style half-open-below stripes covering exactly `(lo, hi]`,
    /// in ascending order, with no gaps; a single stripe when the range holds fewer
    /// parsed notices than `k`. The last stripe always ends at `hi`.
    pub async fn parsed_id_stripes(
        &self,
        lo: i64,
        hi: i64,
        k: usize,
    ) -> turso::Result<Vec<(i64, i64)>> {
        let one = vec![(lo, hi)];
        if k <= 1 || lo >= hi {
            return Ok(one);
        }
        let conn = self.reader().await?;
        let total = {
            let mut r = conn
                .query(
                    "SELECT COUNT(*) FROM notices
                      WHERE parse_state = 'parsed' AND id > ? AND id <= ?",
                    (Value::Integer(lo), Value::Integer(hi)),
                )
                .await?;
            r.next().await?.map_or(0, |row| int(&row, 0))
        };
        let per = total / k as i64;
        if per == 0 {
            return Ok(one);
        }
        // One index-only pass, taking every `per`-th id as a split point. Splitting
        // AFTER the n-th row (not at it) keeps the stripes half-open-below, matching
        // the `id > lo AND id <= hi` window the pre-pass reads with.
        let mut splits: Vec<i64> = Vec::with_capacity(k - 1);
        let mut seen = 0i64;
        let mut rows = conn
            .query(
                "SELECT id FROM notices
                  WHERE parse_state = 'parsed' AND id > ? AND id <= ? ORDER BY id",
                (Value::Integer(lo), Value::Integer(hi)),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            seen += 1;
            if seen % per == 0 && splits.len() < k - 1 {
                let id = int(&row, 0);
                if id < hi {
                    splits.push(id);
                }
            }
        }
        drop(rows);
        splits.dedup();
        let mut stripes = Vec::with_capacity(splits.len() + 1);
        let mut start = lo;
        for s in splits {
            stripes.push((start, s));
            start = s;
        }
        stripes.push((start, hi));
        Ok(stripes)
    }

}

/// The canonical-layer tables whose emptiness means a nuked or externally
/// damaged layer — the standing gate's `present_*` set (issue 133 / task #28).
/// One per table any check reads, and each is created unconditionally at every
/// `Db::open` by `SCHEMA` above, so an `EXISTS` against any of them is always a
/// seek, never a "no such table". Keep this in lockstep with those checks:
/// `.scratch/tender-db/canonical-verify/standing_gate.sh`.
const PRESENCE_TABLES: &[&str] = &[
    "tenders",
    "tender_versions",
    "organizations",
    "organization_mentions",
    "tender_version_texts",
    "tender_version_amounts",
    "tender_version_parties",
    "tender_version_result_winners",
    "lots",
    "lot_results",
    "tender_version_lot_results",
    "tender_version_bids",
    "changes",
];

/// What `observe_layer_presence` found for one presence table. The verdict is a
/// transition, not "is it empty": an empty table is only alarming if it was
/// once populated (a fresh install and a rebuild-in-flight are both legitimately
/// empty), so the discriminating state is `WentEmpty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerState {
    /// Holds rows now.
    Populated,
    /// Empty, and never observed otherwise — a fresh or not-yet-projected DB.
    /// Silent by construction; this is the false alarm the design avoids.
    NeverPopulated,
    /// It held rows and now does not. `at` is when that transition was first
    /// observed — set once and preserved, so it dates the damage.
    WentEmpty { at: i64 },
}

/// One presence table and its observed state (issue 133 / task #38).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPresence {
    pub name: String,
    pub state: LayerState,
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

/// The per-tender identity/dedup/head statements of the Phase-2 fold apply path,
/// prepared ONCE per [`Db::apply_tenders`] connection and reused across every
/// tender of every batch. turso re-parses the SQL on each `conn.execute`/
/// `conn.query`; a reused handle skips the re-parse (docs/research/turso-scale.md).
/// Every [`Statement::execute`]/[`Statement::query`] resets the statement and
/// rebinds all positional parameters, so a reused handle is a drop-in for the old
/// `conn.execute` — same rows, same order, same AUTOINCREMENT surrogate ids.
/// Handles hold their own connection clone, so they survive the per-batch
/// BEGIN/COMMIT and the between-batch checkpoints; no DDL runs during the fold, so
/// no compiled plan goes stale.
///
/// Only the id-then-use tables live here: `tenders`/`lots` inserts (whose
/// AUTOINCREMENT id is captured and referenced), their dedup lookups, the head
/// pointer UPDATE, and the three fixed result-identity dedup SELECTs. The
/// order-independent leaf/satellite tables are batched instead (see [`Pending`]),
/// so they need no single-row handle; the dynamic-table result-identity INSERT
/// (miss-only, low volume) stays on `conn.execute`.
struct TenderInserts {
    tenders: Statement,
    lots: Statement,
    head_update: Statement,
    lot_lookup: Statement,
    lot_result_lookup: Statement,
    bid_lookup: Statement,
    contract_lookup: Statement,
}

impl TenderInserts {
    async fn prepare(conn: &Connection) -> turso::Result<Self> {
        Ok(Self {
            tenders: conn
                .prepare(
                    "INSERT INTO tenders(source, procedure_key, island_notice_id, kind, created_at)
                     VALUES(?, ?, ?, ?, ?)",
                )
                .await?,
            lots: conn
                .prepare("INSERT INTO lots(tender_id, lot_key) VALUES(?, ?)")
                .await?,
            head_update: conn
                .prepare("UPDATE tenders SET current_seq = ?, current_published_at = ?, projection_epoch = ?, current_deadline = ?, current_title = ? WHERE id = ?")
                .await?,
            lot_lookup: conn
                .prepare("SELECT id FROM lots WHERE tender_id = ? AND lot_key = ?")
                .await?,
            lot_result_lookup: conn
                .prepare("SELECT id FROM lot_results WHERE tender_id = ? AND notice_id = ? AND result_key = ?")
                .await?,
            bid_lookup: conn
                .prepare("SELECT id FROM bids WHERE tender_id = ? AND notice_id = ? AND bid_key = ?")
                .await?,
            contract_lookup: conn
                .prepare("SELECT id FROM contracts WHERE tender_id = ? AND notice_id = ? AND contract_key = ?")
                .await?,
        })
    }
}

/// Accumulated rows of the batchable leaf/satellite tables for one WRITE_BATCH
/// transaction (issue 67), flushed as chunked multi-row INSERTs just before
/// COMMIT. Each `Vec<Value>` is row-major (`cols` values per row). Only tables
/// whose row order is unobservable belong here — no emitted or referenced
/// surrogate id (their implicit rowid is never joined on, and every snapshot
/// digest ORDERs BY content columns), so collapsing N per-row INSERTs into one
/// N-row INSERT is byte-identical while cutting the serial writer's per-statement
/// floor ~N-fold. The identity tables (`tenders`/`lots`/`lot_results`/`bids`/
/// `contracts`) and `changes` are NOT here: their AUTOINCREMENT id/cursor is
/// emitted or observed, so they keep inserting in place, in fold order.
///
/// Bounded by WRITE_BATCH (~512 tenders × ~30 facts ≈ 15k rows), so peak RAM is
/// flat vs corpus size.
#[derive(Default)]
struct Pending {
    versions: Vec<Value>,
    version_lots: Vec<Value>,
    /// Issue 237: `(tender_id, seq, group_lot_id, member_lot_id)` quadruples.
    lot_group_members: Vec<Value>,
    texts: Vec<Value>,
    amounts: Vec<Value>,
    classifications: Vec<Value>,
    dates: Vec<Value>,
    parties: Vec<Value>,
    lot_results: Vec<Value>,
    result_winners: Vec<Value>,
    result_stats: Vec<Value>,
    bids: Vec<Value>,
    bid_parties: Vec<Value>,
    contracts: Vec<Value>,
}

impl Pending {
    /// Flush every buffered table as chunked multi-row INSERTs, then clear.
    /// Parents before children so a foreign-keys-ON caller (some store unit tests)
    /// still finds each referenced `(tender_id, seq)` present at insert time; the
    /// projection itself runs with FK off.
    async fn flush(&mut self, conn: &Connection) -> turso::Result<()> {
        flush_rows(conn, "INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, dispatched_at, notice_subtype, publication_id) VALUES ", 7, &mut self.versions).await?;
        flush_rows(conn, "INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind) VALUES ", 4, &mut self.version_lots).await?;
        flush_rows(conn, "INSERT INTO tender_version_lot_group_members(tender_id, seq, group_lot_id, member_lot_id) VALUES ", 4, &mut self.lot_group_members).await?;
        flush_rows(conn, "INSERT INTO tender_version_texts(tender_id, seq, lot_id, field, lang, value) VALUES ", 6, &mut self.texts).await?;
        flush_rows(conn, "INSERT INTO tender_version_amounts(tender_id, seq, lot_id, field, cents, currency, tax_basis) VALUES ", 7, &mut self.amounts).await?;
        flush_rows(conn, "INSERT INTO tender_version_classifications(tender_id, seq, lot_id, field, scheme, code) VALUES ", 6, &mut self.classifications).await?;
        flush_rows(conn, "INSERT INTO tender_version_dates(tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time) VALUES ", 7, &mut self.dates).await?;
        flush_rows(conn, "INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) VALUES ", 7, &mut self.parties).await?;
        flush_rows(conn, "INSERT INTO tender_version_lot_results(tender_id, seq, lot_result_id, lot_id, decision, reason, awarded_cents, awarded_currency, decided_utc, decided_offset, decided_has_time) VALUES ", 11, &mut self.lot_results).await?;
        flush_rows(conn, "INSERT INTO tender_version_result_winners(tender_id, seq, lot_result_id, organization_id) VALUES ", 4, &mut self.result_winners).await?;
        flush_rows(conn, "INSERT INTO tender_version_result_stats(tender_id, seq, lot_result_id, kind, count) VALUES ", 5, &mut self.result_stats).await?;
        flush_rows(conn, "INSERT INTO tender_version_bids(tender_id, seq, bid_id, lot_id, cents, currency) VALUES ", 6, &mut self.bids).await?;
        flush_rows(conn, "INSERT INTO tender_version_bid_parties(tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id) VALUES ", 7, &mut self.bid_parties).await?;
        flush_rows(conn, "INSERT INTO tender_version_contracts(tender_id, seq, contract_id, buyer_contract_id, concluded_utc, concluded_offset, concluded_has_time, decided_utc, decided_offset, decided_has_time, cents, currency) VALUES ", 12, &mut self.contracts).await?;
        Ok(())
    }
}

/// Bind-parameter budget for one multi-row INSERT (issue 67): rows-per-statement
/// is `MAX_BATCH_BIND / cols`. Kept well under turso's ceiling — a version rarely
/// exceeds it for one fact type, but per-batch accumulation does, so the chunk
/// guard is required.
const MAX_BATCH_BIND: usize = 900;

/// Emit `rows` (row-major, `cols` values per row) as chunked multi-row INSERTs
/// with `prefix` (`INSERT INTO t(cols) VALUES `), then clear the buffer.
async fn flush_rows(conn: &Connection, prefix: &str, cols: usize, rows: &mut Vec<Value>) -> turso::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let per_stmt = (MAX_BATCH_BIND / cols).max(1);
    let total = rows.len() / cols;
    let mut done = 0;
    while done < total {
        let n = (total - done).min(per_stmt);
        let mut sql = String::with_capacity(prefix.len() + n * (cols * 2 + 2));
        sql.push_str(prefix);
        for r in 0..n {
            if r > 0 {
                sql.push(',');
            }
            sql.push('(');
            for c in 0..cols {
                if c > 0 {
                    sql.push(',');
                }
                sql.push('?');
            }
            sql.push(')');
        }
        let params: Vec<Value> = rows[done * cols..(done + n) * cols].to_vec();
        conn.execute(&sql, params).await?;
        done += n;
    }
    rows.clear();
    Ok(())
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
pub(crate) fn placeholders(n: usize) -> String {
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
    use super::{Fact, LotState, MinUnionFind, TenderVersion, head_title};
    use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

    fn text(field: &str, lang: Option<&str>, value: &str) -> Fact {
        Fact::Text {
            field: field.into(),
            lang: lang.map(str::to_owned),
            value: value.into(),
        }
    }

    fn head(facts: Vec<Fact>, lot_facts: Vec<Fact>) -> TenderVersion {
        TenderVersion {
            caused_by_notice_id: 1,
            published_at: 0,
            dispatched_at: None,
            notice_subtype: None,
            publication_id: "pub-1".into(),
            facts: facts.into_iter().collect::<BTreeSet<_>>(),
            lots: vec![LotState {
                key: "LOT-0001".into(),
                kind: "Lot".into(),
                facts: lot_facts.into_iter().collect::<BTreeSet<_>>(),
            }],
            rounds: Vec::new(),
            group_members: Vec::new(),
        }
    }

    /// Issue 239: the fold's title writer must agree with the SQL backfill's, because
    /// they populate the same column and `v_tenders` now trusts it. The precedence is
    /// the one the old per-row subquery encoded — Tender over lot, ENG over other — and
    /// `crates/store/tests/title_backfill.rs` asserts the SQL side of the same cases.
    #[test]
    fn head_title_prefers_the_tender_then_english() {
        // A Tender's own title beats a lot's.
        assert_eq!(
            head_title(&head(vec![text("title", None, "tender")], vec![text("title", None, "lot")]))
                .as_deref(),
            Some("tender")
        );
        // Lot-only is the fallback that exists because many notices title only lots.
        assert_eq!(
            head_title(&head(Vec::new(), vec![text("title", None, "lot")])).as_deref(),
            Some("lot")
        );
        // ENG wins within a scope, whichever order the facts arrive in.
        assert_eq!(
            head_title(&head(
                vec![text("title", Some("DEU"), "deutsch"), text("title", Some("ENG"), "english")],
                Vec::new(),
            ))
            .as_deref(),
            Some("english")
        );
        // Other fields are not titles, and no title at all is None rather than "".
        assert_eq!(head_title(&head(vec![text("description", None, "d")], Vec::new())), None);
        assert_eq!(head_title(&head(Vec::new(), Vec::new())), None);
    }

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

    use super::{Db, LayerPresence, LayerState};

    async fn scratch_db(name: &str) -> (Db, String) {
        let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).await.expect("open scratch db"), path)
    }

    fn state_of<'a>(presence: &'a [LayerPresence], name: &str) -> &'a LayerState {
        &presence.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no presence row for {name}")).state
    }

    /// The detector's whole point is a TRANSITION, not "is it empty": a table
    /// still holding rows is `Populated`, one never seen with rows is silent
    /// (`NeverPopulated`), and only a table that HELD rows and is now empty
    /// raises `WentEmpty` — dated once at the transition and preserved on every
    /// later observation so it measures the age of the damage (issue 133).
    #[tokio::test]
    async fn observe_layer_presence_flags_only_the_populated_then_emptied_transition() {
        let (db, path) = scratch_db("layer-presence").await;

        // Every connection handle is SCOPED and dropped before the next
        // `observe_layer_presence` call. Holding one across it DEADLOCKS: the
        // observer takes the writer connection itself, and there is one. This
        // is not hypothetical — the first cut of this test hung here, and so
        // did an unrelated probe the same day, which is why it is spelled out.
        async fn seed(db: &Db, sql: &str) {
            let conn = db.conn().await;
            conn.execute(sql, ()).await.expect("seed");
        }

        // `tenders` gets a row; `changes` never does.
        seed(&db, "INSERT INTO tenders(source, kind, created_at) VALUES('ted','procedure',0)").await;

        // t1: tenders populated, changes never populated.
        let first = db.observe_layer_presence(100).await.expect("observe t1");
        assert_eq!(state_of(&first, "tenders"), &LayerState::Populated);
        assert_eq!(state_of(&first, "changes"), &LayerState::NeverPopulated);

        // The layer is emptied out from under us (a re-nuke), then observed at
        // t2: this is the one case that must alarm, dated at t2.
        seed(&db, "DELETE FROM tenders").await;
        let second = db.observe_layer_presence(200).await.expect("observe t2");
        assert_eq!(state_of(&second, "tenders"), &LayerState::WentEmpty { at: 200 });
        // Still-empty-and-never-populated stays silent.
        assert_eq!(state_of(&second, "changes"), &LayerState::NeverPopulated);

        // t3: still empty. The transition time is preserved, NOT re-stamped to
        // t3 — the damage keeps its original age.
        let third = db.observe_layer_presence(300).await.expect("observe t3");
        assert_eq!(state_of(&third, "tenders"), &LayerState::WentEmpty { at: 200 });

        // t4: REPOPULATED — the alarm must clear itself.
        //
        // This is the arm that decides whether the detector is usable rather
        // than merely correct. A `WentEmpty` that latched forever would pass
        // every assertion above, keep reporting damage over a layer that has
        // been rebuilt and is serving fine, and get muted by whoever is on the
        // other end — at which point the real wipe it exists for goes unread.
        // A rebuild legitimately empties and then refills the layer, so this is
        // not an edge case: it is the normal path, and it must end silent.
        seed(&db, "INSERT INTO tenders(source, kind, created_at) VALUES('ted','procedure',0)").await;
        let fourth = db.observe_layer_presence(400).await.expect("observe t4");
        assert_eq!(
            state_of(&fourth, "tenders"),
            &LayerState::Populated,
            "the detector latched: a repopulated table still reports damage, so the alarm never clears"
        );

        // And the clearing must be DURABLE, not just this call's return value —
        // the stored `went_empty_at` has to be gone, or the next observation
        // resurrects an alarm for damage that has been repaired.
        let fifth = db.observe_layer_presence(500).await.expect("observe t5");
        assert_eq!(state_of(&fifth, "tenders"), &LayerState::Populated);

        let _ = std::fs::remove_file(&path);
    }

    /// While a rebuild runs, the layer is legitimately empty, so the observer
    /// stops evaluating and only keeps the observation FRESH. That has to
    /// preserve the verdicts exactly — and it must not invent rows on a box
    /// that has never observed, which would fabricate an observation that
    /// never happened (issue 133).
    #[tokio::test]
    async fn touching_presence_refreshes_freshness_without_inventing_a_verdict() {
        let (db, path) = scratch_db("layer-presence-touch").await;

        async fn seed(db: &Db, sql: &str) {
            let conn = db.conn().await;
            conn.execute(sql, ()).await.expect("seed");
        }

        // Nothing observed yet: touching must create nothing at all.
        db.touch_layer_presence(50).await.expect("touch on a fresh box");
        assert!(
            db.read_layer_presence().await.expect("read").is_empty(),
            "touch fabricated presence rows on a box that has never observed"
        );

        // Observe for real, then let the layer go empty and observe again so
        // there is a verdict worth preserving.
        seed(&db, "INSERT INTO tenders(source, kind, created_at) VALUES('ted','procedure',0)").await;
        db.observe_layer_presence(100).await.expect("observe");
        seed(&db, "DELETE FROM tenders").await;
        db.observe_layer_presence(200).await.expect("observe empty");

        // Touch: freshness moves, the verdict does not.
        db.touch_layer_presence(900).await.expect("touch");
        let after = db.read_layer_presence().await.expect("read");
        let (tenders, observed_at) = after
            .iter()
            .find(|(p, _)| p.name == "tenders")
            .expect("tenders row");
        assert_eq!(
            tenders.state,
            LayerState::WentEmpty { at: 200 },
            "touch overwrote a real verdict — a rebuild would erase the evidence of a wipe"
        );
        assert_eq!(*observed_at, 900, "touch did not refresh freshness, so a long rebuild would age into the staleness alarm");

        let _ = std::fs::remove_file(&path);
    }

    /// A steady-state observation must refresh `observed_at` even though it
    /// changes no verdict. `/health/deep` reads staleness off that stamp, so
    /// an observation that leaves no trace is indistinguishable from an
    /// observer that has died — and a quiet box would freeze at the last
    /// heavy-write touch and trip the staleness alarm six hours later, every
    /// day (issue 161).
    #[tokio::test]
    async fn a_steady_state_observation_refreshes_observed_at() {
        let (db, path) = scratch_db("layer-presence-steady").await;

        async fn seed(db: &Db, sql: &str) {
            let conn = db.conn().await;
            conn.execute(sql, ()).await.expect("seed");
        }

        seed(&db, "INSERT INTO tenders(source, kind, created_at) VALUES('ted','procedure',0)").await;
        db.observe_layer_presence(100).await.expect("observe t1");

        // Nothing changes between the observations — the pure steady state.
        db.observe_layer_presence(200).await.expect("observe t2");

        let after = db.read_layer_presence().await.expect("read");
        assert!(!after.is_empty(), "first observation created no presence rows");
        for (p, observed_at) in &after {
            assert_eq!(
                *observed_at, 200,
                "`{}` still carries the previous observation's timestamp: a steady-state \
                 observation left no heartbeat, so a healthy quiet box ages into the \
                 staleness alarm",
                p.name
            );
        }

        let _ = std::fs::remove_file(&path);
    }
}
