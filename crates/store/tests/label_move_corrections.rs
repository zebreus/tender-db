//! Issue 562: a fold that rewrites a Tender's `source` or `kind` in place (a DÖE Tender turning
//! `ted` when its TED twin joins) was invisible to an SSE subscription on the old label: every
//! transition row reads the one rewritten label on both sides, so a `source=doe` tape heard
//! nothing and kept a ghost. The decision (B, 2026-10-17, amendments adopted): every fold that
//! is not `acting` and has `StoredIdentity.moved` set writes ADR-0017 D3's correction set —
//! rule T's seq-less `tender changed`, then one seq-less `lot changed` per lot of the head
//! version — last for the Tender, counted in `Applied.changes` and `Applied.label_corrections`.
//! - The head lots never depend on the write loop: a head version the call did not rewrite (a
//!   chain cut back to its prefix) is read from disk, a written one by its lot keys.
//! - Not guarded by `!stale`: Off and Shadow stale refolds that move the label are in scope;
//!   the acting compare (`On`) already writes one set of its own and gets no second.
//! - A Tender minted TED-first, or co-minted with both notices, never moves its label.
//!
//! The unchanged-chain early return is pinned through the ingest pipeline, where a stored
//! label can disagree with the derivation (`a_label_moved_on_an_unchanged_chain_writes_the_
//! correction` and `a_refold_corrects_a_stored_tender_kind` in `crates/ingest/tests/project.rs`).
//!
//! Every case here goes through the FOLD (`apply_tenders`), in the style of
//! `value_move_change_rows.rs`.

use std::collections::BTreeMap;

use store::RefoldCompare;
use store::canonical::{Applied, Fact, LotState, TenderProjection, TenderVersion};
use store::turso::{self, Value};

const DAY: i64 = 86_400;
/// 2026-01-02T12:00:00Z: every chain's first version.
const T0: i64 = 1_767_355_200;
/// The fold's `now`, after every publication.
const NOW: i64 = T0 + 100 * DAY;

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-562-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    (db, conn)
}

fn lot(key: &str) -> LotState {
    LotState { key: key.into(), kind: "Lot".into(), facts: Default::default() }
}

fn deadline(at: i64) -> Fact {
    Fact::Date { field: "submission_deadline".into(), utc_seconds: at, offset_minutes: 0, has_time: true }
}

/// One version as the fold carries it; each version moves the deadline, so each is the
/// Tender's own `changed` and no lot's.
fn version(notice: i64, published_at: i64, lots: &[&str]) -> TenderVersion {
    TenderVersion {
        caused_by_notice_id: notice,
        published_at,
        dispatched_at: None,
        notice_subtype: None,
        original_lang: None,
        publication_id: format!("{notice}-2026"),
        facts: [deadline(published_at + 30 * DAY)].into_iter().collect(),
        lots: lots.iter().copied().map(lot).collect(),
        rounds: Vec::new(),
        group_members: Vec::new(),
    }
}

/// The one keyed Tender of every case, under the label `source`.
fn projection(source: &str, versions: Vec<TenderVersion>) -> TenderProjection {
    TenderProjection {
        source: source.into(),
        procedure_key: Some("pk-562".into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions,
    }
}

async fn scalar(conn: &turso::Connection, sql: &str) -> Option<i64> {
    let mut rows = conn.query(sql, ()).await.unwrap_or_else(|e| panic!("{e}: {sql}"));
    let row = rows.next().await.unwrap()?;
    row.get_value(0).unwrap().as_integer().copied()
}

async fn last_cursor(conn: &turso::Connection) -> i64 {
    scalar(conn, "SELECT COALESCE(MAX(cursor), 0) FROM changes").await.unwrap_or(0)
}

async fn label(conn: &turso::Connection) -> String {
    let mut rows = conn.query("SELECT source FROM tenders WHERE procedure_key = 'pk-562'", ()).await.unwrap();
    let row = rows.next().await.unwrap().expect("the case's Tender");
    match row.get_value(0).unwrap() {
        Value::Text(s) => s,
        other => panic!("source: {other:?}"),
    }
}

/// Every lot id of the database with its key: taken before a fold that sweeps a lot, so the
/// swept lot's `removed` row still reads by its key.
async fn lot_keys(conn: &turso::Connection) -> BTreeMap<i64, String> {
    let mut rows = conn.query("SELECT id, lot_key FROM lots", ()).await.unwrap();
    let mut out = BTreeMap::new();
    while let Some(row) = rows.next().await.unwrap() {
        let id = row.get_value(0).unwrap().as_integer().copied().unwrap();
        let Value::Text(key) = row.get_value(1).unwrap() else { panic!("lot_key") };
        out.insert(id, key);
    }
    out
}

/// The change log's Tender and lot rows after `cursor`, as `tender@2:changed` and
/// `lot:LOT-1@-:changed` (a seq-less row reads `@-`), in cursor order. One Tender per database,
/// so every row belongs to the case. A lot no longer stored is named from `known`.
async fn feed(conn: &turso::Connection, cursor: i64, known: &BTreeMap<i64, String>) -> Vec<String> {
    let mut current = lot_keys(conn).await;
    for (id, key) in known {
        current.entry(*id).or_insert_with(|| key.clone());
    }
    let mut rows = conn
        .query(
            "SELECT entity_kind, entity_id, version_seq, op FROM changes
              WHERE cursor > ? AND entity_kind IN ('tender', 'lot') ORDER BY cursor",
            (Value::Integer(cursor),),
        )
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let Value::Text(kind) = row.get_value(0).unwrap() else { panic!("entity_kind") };
        let id = row.get_value(1).unwrap().as_integer().copied().unwrap();
        let seq = match row.get_value(2).unwrap() {
            Value::Integer(n) => n.to_string(),
            _ => "-".to_owned(),
        };
        let Value::Text(op) = row.get_value(3).unwrap() else { panic!("op") };
        let entity = match kind.as_str() {
            "lot" => format!("lot:{}", current.get(&id).map_or("?", String::as_str)),
            other => other.to_owned(),
        };
        out.push(format!("{entity}@{seq}:{op}"));
    }
    out
}

fn corrections(feed: &[String]) -> usize {
    feed.iter().filter(|row| row.as_str() == "tender@-:changed").count()
}

/// The decision's headline case: a DÖE fold mints the Tender, then its TED twin arrives on the
/// current epoch (a pure append, keep = 1) and the label flips `doe` → `ted`. The append's
/// transition rows (the Tender's `changed`, a new lot's `added`) are followed by exactly one
/// correction set: rule T, then every lot of the head version — LOT-3 included, which this very
/// call minted, so its id is looked up by key rather than read from disk.
#[tokio::test]
async fn a_label_moved_by_an_ordinary_fold_writes_rule_t_and_the_head_lots_seqless() {
    let (db, conn) = open("ordinary").await;
    let doe = version(1, T0, &["LOT-1", "LOT-2"]);
    db.apply_tenders(&[projection("doe", vec![doe.clone()])], NOW, false).await.unwrap();
    assert_eq!(label(&conn).await, "doe");
    let cursor = last_cursor(&conn).await;

    let ted = version(2, T0 + DAY, &["LOT-1", "LOT-2", "LOT-3"]);
    let a = db.apply_tenders(&[projection("ted", vec![doe, ted])], NOW, false).await.unwrap();
    assert_eq!(label(&conn).await, "ted", "the TED twin relabels the Tender");
    let rows = feed(&conn, cursor, &BTreeMap::new()).await;
    assert_eq!(
        rows,
        [
            "tender@2:changed",
            "lot:LOT-3@2:added",
            "tender@-:changed",
            "lot:LOT-1@-:changed",
            "lot:LOT-2@-:changed",
            "lot:LOT-3@-:changed",
        ],
        "the v2 transitions, then rule T and every head lot, seq-less"
    );
    assert_eq!((a.versions_written, a.label_corrections), (1, 1), "{a:?}");
    assert_eq!(a.changes, rows.len() as u64, "the correction rows count toward Applied.changes: {a:?}");
}

/// A second DÖE version on a DÖE Tender leaves the label alone: its transition row only.
#[tokio::test]
async fn a_fold_that_leaves_the_label_alone_writes_no_correction() {
    let (db, conn) = open("unmoved").await;
    let v1 = version(1, T0, &["LOT-1"]);
    db.apply_tenders(&[projection("doe", vec![v1.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    let a = db.apply_tenders(&[projection("doe", vec![v1, version(2, T0 + DAY, &["LOT-1"])])], NOW, false).await.unwrap();
    assert_eq!(feed(&conn, cursor, &BTreeMap::new()).await, ["tender@2:changed"]);
    assert_eq!(a.label_corrections, 0, "{a:?}");
}

/// A Tender minted TED-first stays `ted` when its DÖE notice is prepended (published earlier,
/// so the chain is replayed from version 1, keep = 0): the label never moved, so the replay's
/// transition rows are the whole announcement.
#[tokio::test]
async fn a_ted_first_tender_gaining_its_doe_notice_writes_no_correction() {
    let (db, conn) = open("ted-first").await;
    let ted = version(2, T0 + DAY, &["LOT-1"]);
    db.apply_tenders(&[projection("ted", vec![ted.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    let a = db.apply_tenders(&[projection("ted", vec![version(1, T0, &["LOT-1"]), ted])], NOW, false).await.unwrap();
    assert_eq!(label(&conn).await, "ted");
    let rows = feed(&conn, cursor, &BTreeMap::new()).await;
    assert!(rows.iter().all(|row| !row.contains("@-")), "no seq-less row: {rows:?}");
    assert_eq!(rows.first().map(String::as_str), Some("tender@1:added"), "the replay announces its chain: {rows:?}");
    assert_eq!((a.versions_written, a.label_corrections), (2, 0), "{a:?}");
}

/// A DÖE and a TED notice folded together mint the Tender `ted`: nothing was stored, so nothing
/// moved.
#[tokio::test]
async fn a_co_minted_doe_and_ted_pair_writes_no_correction() {
    let (db, conn) = open("co-minted").await;
    let a = db
        .apply_tenders(&[projection("ted", vec![version(1, T0, &["LOT-1"]), version(2, T0 + DAY, &["LOT-1"])])], NOW, false)
        .await
        .unwrap();
    assert_eq!(label(&conn).await, "ted");
    assert_eq!(
        feed(&conn, 0, &BTreeMap::new()).await,
        ["tender@1:added", "lot:LOT-1@1:added", "tender@2:changed"],
        "the transitions alone"
    );
    assert_eq!((a.tenders_created, a.label_corrections), (1, 0), "{a:?}");
}

/// A stale Tender (an older projection epoch) whose refold also takes its TED twin. Under `On`
/// the fold acts on the compare, and D3's own branch writes the one correction set (the moved
/// label is a head difference there); B must not write a second. Under `Off` and `Shadow` the
/// chain is replayed and B writes the set after the replay: not guarded by `!stale`, and the
/// two modes write identical rows.
#[tokio::test]
async fn a_stale_refold_that_moves_the_label_writes_one_correction_set() {
    async fn stale_flip(mode: RefoldCompare) -> (Vec<String>, Applied) {
        let (db, conn) = open(&format!("stale-{mode:?}")).await;
        // The compare refuses to run without the by-version indexes.
        db.build_tender_indexes().await.unwrap();
        let doe = version(1, T0, &["LOT-1", "LOT-2"]);
        db.apply_tenders(&[projection("doe", vec![doe.clone()])], NOW, false).await.unwrap();
        db.set_projection_epoch_for_test(-1).await.unwrap();
        db.set_refold_compare(mode);
        let cursor = last_cursor(&conn).await;
        let ted = version(2, T0 + DAY, &["LOT-1", "LOT-2"]);
        let a = db.apply_tenders(&[projection("ted", vec![doe, ted])], NOW, false).await.unwrap();
        assert_eq!(label(&conn).await, "ted", "{mode:?}");
        (feed(&conn, cursor, &BTreeMap::new()).await, a)
    }

    let (on, a_on) = stale_flip(RefoldCompare::On).await;
    assert_eq!(
        on,
        ["tender@2:changed", "tender@-:changed", "lot:LOT-1@-:changed", "lot:LOT-2@-:changed"],
        "On: the appended version's transition, then D3's one set"
    );
    assert_eq!((a_on.tenders_corrected, a_on.label_corrections), (1, 0), "D3 wrote it, not B: {a_on:?}");

    let (off, a_off) = stale_flip(RefoldCompare::Off).await;
    assert_eq!(
        off,
        [
            "tender@1:added",
            "lot:LOT-1@1:added",
            "lot:LOT-2@1:added",
            "tender@2:changed",
            "tender@-:changed",
            "lot:LOT-1@-:changed",
            "lot:LOT-2@-:changed",
        ],
        "Off: the replay, then B's set"
    );
    assert_eq!(a_off.label_corrections, 1, "{a_off:?}");
    let (shadow, a_shadow) = stale_flip(RefoldCompare::Shadow).await;
    assert_eq!(shadow, off, "Shadow writes what Off writes");
    assert_eq!(a_shadow.label_corrections, 1, "{a_shadow:?}");
    for (mode, rows) in [("On", &on), ("Off", &off), ("Shadow", &shadow)] {
        assert_eq!(corrections(rows), 1, "{mode}: exactly one correction set: {rows:?}");
    }
}

/// The adversarial check's hole in the write-loop reading: a `ted` Tender whose TED notice was
/// its LAST version is regrouped out, so the chain shrinks back to its DÖE prefix (keep ==
/// versions < stored) and the label flips `ted` → `doe`. The write loop never runs and the head
/// version's lot rows are on disk, not pending: the correction still names every head lot,
/// after the sweep's `removed` for the lot only the TED version carried.
#[tokio::test]
async fn a_label_moved_by_a_chain_shrinking_to_its_prefix_announces_the_head_lots() {
    let (db, conn) = open("shrink").await;
    let doe = version(1, T0, &["LOT-1", "LOT-2"]);
    let ted = version(2, T0 + DAY, &["LOT-1", "LOT-2", "LOT-3"]);
    db.apply_tenders(&[projection("ted", vec![doe.clone(), ted])], NOW, false).await.unwrap();
    assert_eq!(label(&conn).await, "ted");
    let known = lot_keys(&conn).await;
    let cursor = last_cursor(&conn).await;

    let a = db.apply_tenders(&[projection("doe", vec![doe])], NOW, false).await.unwrap();
    assert_eq!(label(&conn).await, "doe", "the TED notice left, and the label with it");
    assert_eq!(scalar(&conn, "SELECT current_seq FROM tenders").await, Some(1));
    let rows = feed(&conn, cursor, &known).await;
    assert_eq!(
        rows,
        ["lot:LOT-3@-:removed", "tender@-:changed", "lot:LOT-1@-:changed", "lot:LOT-2@-:changed"],
        "the sweep, then rule T and every lot of the kept head"
    );
    assert_eq!((a.versions_written, a.versions_removed, a.label_corrections), (0, 1, 1), "{a:?}");
    assert_eq!(a.changes, rows.len() as u64, "{a:?}");
}
