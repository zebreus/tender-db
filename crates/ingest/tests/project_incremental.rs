//! Output-identity proof for the incremental projection (issue 58).
//!
//! The daily projection re-derives only the Tenders TOUCHED by notices parsed
//! since the last run. Correctness hinges on it producing EXACTLY what a full
//! non-rebuild projection over the whole corpus produces — untouched Tenders
//! left byte-identical, touched ones re-derived in full. Each test builds two
//! DBs with an identical established layer, applies the SAME delta, absorbs it
//! one way with a full non-rebuild projection and the other way incrementally,
//! and asserts the canonical layer is byte-identical (surrogate ids included —
//! the reconcile reuses ids by natural key, so they match).

use ingest::project;
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projincr-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = Db::open(&path).await.expect("open scratch db");
    db.record_fetch(&store::Fetch {
        source: SOURCE.into(),
        kind: "daily".into(),
        period: "2026-00136".into(),
        url: "https://example.invalid/pkg".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: "ted/daily/2026-00136.tar.gz".into(),
    })
    .await
    .expect("record fetch");
    let fetch_id = db.current_packages(SOURCE, "daily", None).await.expect("packages")[0].fetch_id;
    (db, fetch_id, path)
}

fn sec(id: &str, kind: &str, parent: Option<&str>) -> Section {
    Section { id: id.into(), kind: kind.into(), parent: parent.map(str::to_owned) }
}
fn id_val(section: &str, field: &str, value: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: None, value: value.into(), is_ref: false },
    }
}
fn text_val(section: &str, field: &str, value: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Text { lang: Some("ENG".into()), value: value.into() },
    }
}
fn date_val(section: &str, field: &str, utc: i64) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Date { utc_seconds: utc, offset_minutes: 0, has_time: false },
    }
}

/// An eForms Organization (name on the section, VAT id on its legal-entity child)
/// referenced as a role from the procedure.
fn org(parsed: &mut Parsed, section: &str, name: &str, vat: &str, role: &str, ordinal: i64) {
    parsed.sections.push(sec(section, "Organization", Some("PROC")));
    parsed.values.push(text_val(section, "BT-500-Organization-Company", name));
    let legal = format!("{section}-legal");
    parsed.sections.push(sec(&legal, "CompanyLegalEntity", Some(section)));
    parsed.values.push(ValueRow {
        section_id: legal,
        field_id: "BT-501-Organization-Company".into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: Some("VAT".into()), value: vat.into(), is_ref: false },
    });
    parsed.values.push(ValueRow {
        section_id: "PROC".into(),
        field_id: role.into(),
        ordinal,
        value: NoticeValue::Id { scheme: None, value: section.into(), is_ref: true },
    });
}

async fn record_p(db: &Db, fetch_id: i64, pub_id: &str, profile: &str, parsed: Parsed) {
    db.record_notice(
        &Notice {
            source: SOURCE.into(),
            publication_id: pub_id.into(),
            content_hash: format!("h-{pub_id}"),
            profile: profile.into(),
            declared_version: None,
            fetch_id,
            member_path: format!("{pub_id}.xml"),
            ingested_at: 0,
            published_at: Some(0),
            dispatched_at: None,
        },
        &Parse::Parsed(parsed),
    )
    .await
    .expect("record notice");
}

async fn record(db: &Db, fetch_id: i64, pub_id: &str, parsed: Parsed) {
    record_p(db, fetch_id, pub_id, "eforms:eforms-sdk-1.13", parsed).await;
}

/// A keyed notice under BT-04 `key`, published at `pub_at`, with a buyer org.
fn keyed(key: &str, pub_at: i64, title: &str) -> Parsed {
    let mut parsed = Parsed {
        sections: vec![sec("PROC", "Procedure", None), sec("LOT-1", "Lot", Some("PROC"))],
        values: vec![
            id_val("PROC", "BT-04-notice", key),
            date_val("PROC", "BT-05(a)-notice", pub_at),
            text_val("PROC", "BT-21-Procedure", title),
            date_val("LOT-1", "BT-131(d)-Lot", 700_000_000 + pub_at),
        ],
    };
    org(&mut parsed, "ORG-A", "Buyer One", "NL000000001B01", "OPT-300-Procedure-Buyer", 0);
    parsed
}

/// An island notice — no procedure key.
fn island(title: &str) -> Parsed {
    Parsed {
        sections: vec![sec("PROC", "Procedure", None)],
        values: vec![
            date_val("PROC", "BT-05(a)-notice", 5),
            text_val("PROC", "BT-21-Procedure", title),
        ],
    }
}

/// Build the SAME established corpus on `db`: a 2-notice keyed Tender, an
/// unrelated 1-notice keyed Tender, and an island — then project it fully.
async fn establish(db: &Db, fetch_id: i64) {
    record(db, fetch_id, "A-cn", keyed("bt04-0001", 1, "Alpha CN")).await;
    record(db, fetch_id, "A-corr", keyed("bt04-0001", 2, "Alpha corrigendum")).await;
    record(db, fetch_id, "B-cn", keyed("bt04-0002", 1, "Beta CN")).await;
    record(db, fetch_id, "ISL", island("Island one")).await;
    project::project(db, false).await.expect("establish projection");
}

/// A deterministic digest of every canonical table with business content, keyed
/// and ordered so it is stable across runs. Timestamps excluded; surrogate ids
/// included (the reconcile reuses them by natural key). Mirrors
/// `project_equivalence::snapshot`.
async fn snapshot(db: &Db) -> String {
    let digests = [
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source AS r FROM tenders ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||coalesce(publication_id,'')||'|'||published_at AS r FROM tender_versions ORDER BY tender_id, seq)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value||'|'||coalesce(lot_id,-1) AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds||'|'||coalesce(lot_id,-1) AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||role||'|'||organization_id||'|'||coalesce(lot_id,-1) AS r FROM tender_version_parties ORDER BY tender_id, seq, role, organization_id, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||lot_key AS r FROM lots ORDER BY tender_id, lot_key)",
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(country,'')||'|'||coalesce(identifier_kind,'')||'|'||coalesce(identifier,'')||'|'||name||'|'||provisional AS r FROM organizations ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT notice_id||'|'||section_id||'|'||organization_id AS r FROM organization_mentions ORDER BY notice_id, section_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT entity_kind||'|'||op||'|'||coalesce(version_seq,-1)||'|'||entity_id AS r FROM changes ORDER BY cursor)",
    ];
    let mut out = String::new();
    for (i, sql) in digests.iter().enumerate() {
        let part = match db.scalar(sql).await.expect("digest query") {
            Some(store::turso::Value::Text(s)) => s,
            _ => String::new(),
        };
        out.push_str(&format!("--- digest {i} ---\n{part}\n"));
    }
    out
}

/// `snapshot` minus the change feed — the CONTENT surface. A forced rewrite
/// re-emits version change events by design (issue 99), so the feed is expected to
/// grow; what must not move is the canonical content itself.
async fn snapshot_content(db: &Db) -> String {
    let full = snapshot(db).await;
    match full.find("--- digest 8 ---") {
        Some(at) => full[..at].to_owned(),
        None => full,
    }
}

async fn count(db: &Db, sql: &str) -> i64 {
    match db.scalar(sql).await.expect("count") {
        Some(store::turso::Value::Integer(i)) => i,
        _ => 0,
    }
}

/// A late award notice attaching to an existing keyed Tender is absorbed
/// identically by a full non-rebuild projection and an incremental one.
#[tokio::test]
async fn incremental_late_attach_to_keyed_tender_matches_full() {
    let (full, ff, pf) = scratch("full").await;
    let (incr, fi, pi) = scratch("incr").await;
    establish(&full, ff).await;
    establish(&incr, fi).await;

    // The established layers must already match.
    assert_eq!(snapshot(&full).await, snapshot(&incr).await, "established layers differ");
    // Everything is projected: the incremental change-set is empty.
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "nothing left to project");

    // Delta: a later notice under the SAME key bt04-0001 (a late attach).
    record(&full, ff, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;
    record(&incr, fi, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;

    // Exactly one notice is now unprojected on the incremental DB.
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap(), vec![5], "one changed notice");

    // Absorb: full re-scan vs incremental.
    project::project(&full, false).await.expect("full absorb");
    let report = project::project_incremental(&incr).await.expect("incremental absorb");
    assert_eq!(report.notices, 1, "incremental touched exactly the delta");

    // Byte-identical canonical layer.
    assert_eq!(
        snapshot(&full).await,
        snapshot(&incr).await,
        "incremental late-attach must match a full non-rebuild projection"
    );
    // The Tender count did not grow (attach, not a new Tender), and the change-set
    // is drained.
    assert_eq!(count(&incr, "SELECT COUNT(*) FROM tenders").await, 3, "still three Tenders");
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "change-set drained");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Issue 81: folding the delta in tiny chunks is byte-identical to a single pass,
/// and a Tender whose changed notices span chunks is re-folded whole in the later
/// chunk (bounded RAM without splitting Tenders).
#[tokio::test]
async fn incremental_chunked_is_byte_identical_to_single_pass() {
    let (one, f1, p1) = scratch("chunk-one").await;
    let (many, fm, pm) = scratch("chunk-many").await;
    establish(&one, f1).await;
    establish(&many, fm).await;
    assert_eq!(snapshot(&one).await, snapshot(&many).await, "established layers differ");

    // A multi-Tender delta: a NEW keyed Tender (bt04-0003) with TWO notices — under
    // chunk=1 they land in different chunks and must still fold into ONE Tender — a
    // late attach to the existing Alpha, a new keyed Tender, and a new island.
    for (db, fid) in [(&one, f1), (&many, fm)] {
        record(db, fid, "C-cn", keyed("bt04-0003", 1, "Gamma CN")).await;
        record(db, fid, "C-corr", keyed("bt04-0003", 2, "Gamma corrigendum")).await;
        record(db, fid, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;
        record(db, fid, "D-cn", keyed("bt04-0004", 1, "Delta CN")).await;
        record(db, fid, "ISL2", island("Island two")).await;
    }
    assert_eq!(one.unprojected_parsed_notice_ids().await.unwrap().len(), 5, "five changed notices");

    // One pass vs one-notice-at-a-time chunks.
    project::project_incremental_chunked(&one, 10_000).await.expect("single-pass incremental");
    project::project_incremental_chunked(&many, 1).await.expect("chunked incremental");

    assert_eq!(
        snapshot(&one).await,
        snapshot(&many).await,
        "chunked incremental must be byte-identical to a single pass"
    );
    assert_eq!(many.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "change-set drained");
    // Gamma's two notices folded into ONE Tender with two versions, across chunks.
    assert_eq!(
        count(
            &many,
            "SELECT COUNT(*) FROM tender_versions WHERE tender_id=(SELECT id FROM tenders WHERE procedure_key='bt04-0003')"
        )
        .await,
        2,
        "the cross-chunk Tender's two notices did not split"
    );

    for p in [p1, pm] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// A mixed delta — a late attach to an existing Tender, a brand-new keyed Tender,
/// and a new island — is absorbed identically by full and incremental, and the
/// Tenders the delta does NOT touch are left byte-for-byte unchanged.
#[tokio::test]
async fn incremental_mixed_delta_matches_full_and_leaves_untouched_alone() {
    let (full, ff, pf) = scratch("mixfull").await;
    let (incr, fi, pi) = scratch("mixincr").await;
    establish(&full, ff).await;
    establish(&incr, fi).await;

    // Fingerprint the untouched Beta Tender (bt04-0002) before the delta.
    let beta_before = db_fingerprint(&incr, "bt04-0002").await;

    for (db, fid) in [(&full, ff), (&incr, fi)] {
        record(db, fid, "A-award", keyed("bt04-0001", 3, "Alpha award")).await; // attach
        record(db, fid, "C-cn", keyed("bt04-0003", 1, "Gamma CN")).await; // new Tender
        record(db, fid, "ISL2", island("Island two")).await; // new island
    }
    // Three notices changed on the incremental DB (establish used ids 1-4).
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap(), vec![5, 6, 7]);

    project::project(&full, false).await.expect("full absorb");
    project::project_incremental(&incr).await.expect("incremental absorb");

    assert_eq!(
        snapshot(&full).await,
        snapshot(&incr).await,
        "mixed incremental delta must match a full non-rebuild projection"
    );
    assert_eq!(count(&incr, "SELECT COUNT(*) FROM tenders").await, 5, "two new Tenders added");
    // Beta was never in the touched set — its rows are unchanged.
    assert_eq!(db_fingerprint(&incr, "bt04-0002").await, beta_before, "untouched Tender changed");
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "change-set drained");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Fold-source invariance for the INCREMENTAL path (issue 91). Phase 2 now picks
/// its parsed-layer read by plan size — `ParsedFold` for a small daily delta,
/// the bucketed sequential sweep for a re-fold — so both must absorb the SAME delta
/// into a byte-identical canonical layer, and both must equal what a full
/// non-rebuild projection produces.
///
/// This is the load-bearing proof for routing a re-fold at `bucketed_fold`. The
/// bucketed pre-pass sweeps the WHOLE parsed layer and routes each notice by the
/// plan's `group_key`, skipping notices absent from the plan (`write_shard`) — so
/// the assertion that actually matters here is that it honours a SCOPED plan:
/// the untouched Tenders must come through unchanged, not be re-folded or dropped.
#[tokio::test]
async fn incremental_bucketed_fold_matches_parsed_fold_and_full() {
    use ingest::project::Phase2;

    let (full, ff, pf) = scratch("fsfull").await;
    let (parsed_fold, fp, pp) = scratch("fsparsed").await;
    let (buckets, fb, pb) = scratch("fsbuckets").await;
    establish(&full, ff).await;
    establish(&parsed_fold, fp).await;
    establish(&buckets, fb).await;
    assert_eq!(snapshot(&parsed_fold).await, snapshot(&buckets).await, "established layers differ");

    // Fingerprint the Tender the delta does NOT touch — the bucketed sweep reads
    // its notices too (it sweeps everything) and must still leave it alone.
    let beta_before = db_fingerprint(&buckets, "bt04-0002").await;

    // A delta spanning every grouping regime: a late attach to an existing keyed
    // Tender, a brand-new keyed Tender with TWO notices (a chain), and a new island.
    for (db, fid) in [(&full, ff), (&parsed_fold, fp), (&buckets, fb)] {
        record(db, fid, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;
        record(db, fid, "C-cn", keyed("bt04-0003", 1, "Gamma CN")).await;
        record(db, fid, "C-corr", keyed("bt04-0003", 2, "Gamma corrigendum")).await;
        record(db, fid, "ISL2", island("Island two")).await;
    }

    project::project(&full, false).await.expect("full absorb");
    project::project_incremental_chunked_phase2(&parsed_fold, 10_000, Some(Phase2::ParsedFold))
        .await
        .expect("incremental absorb via ParsedFold");
    project::project_incremental_chunked_phase2(
        &buckets,
        10_000,
        Some(Phase2::Buckets { shards: None }),
    )
    .await
    .expect("incremental absorb via Buckets");

    let (full_snap, parsed_snap, bucket_snap) =
        (snapshot(&full).await, snapshot(&parsed_fold).await, snapshot(&buckets).await);
    assert_eq!(
        parsed_snap, bucket_snap,
        "the bucketed incremental fold must be byte-identical to ParsedFold over the same scoped plan"
    );
    assert_eq!(
        full_snap, bucket_snap,
        "the bucketed incremental fold must match a full non-rebuild projection"
    );
    // The scoped plan was honoured: the untouched Tender was not re-folded, and the
    // chain that arrived as two notices folded into one Tender with two versions.
    assert_eq!(db_fingerprint(&buckets, "bt04-0002").await, beta_before, "untouched Tender changed");
    assert_eq!(
        count(
            &buckets,
            "SELECT COUNT(*) FROM tender_versions WHERE tender_id=(SELECT id FROM tenders WHERE procedure_key='bt04-0003')"
        )
        .await,
        2,
        "the new chain's two notices folded into one Tender"
    );
    assert_eq!(
        buckets.unprojected_parsed_notice_ids().await.unwrap().len(),
        0,
        "the bucketed fold marks every folded notice projected"
    );

    for p in [pf, pp, pb] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// A delta containing a LEGACY notice falls back to a full non-rebuild projection
/// (issue 58 v1) — the transitive OJS graph is not persisted incrementally — and
/// still produces the correct canonical layer.
#[tokio::test]
async fn incremental_legacy_delta_falls_back_to_full() {
    let (full, ff, pf) = scratch("legfull").await;
    let (incr, fi, pi) = scratch("legincr").await;
    establish(&full, ff).await;
    establish(&incr, fi).await;

    // A legacy (ted-export-r209) notice in the delta — must trigger the fallback.
    let legacy = Parsed {
        sections: vec![sec("PROC", "Notice", None)],
        values: vec![
            text_val("PROC", "TED-TITLE", "Legacy works"),
            date_val("PROC", "TED-DS_DATE_DISPATCH", 42),
        ],
    };
    record_p(&full, ff, "100000-2019", "ted-export-r209", legacy.clone()).await;
    record_p(&incr, fi, "100000-2019", "ted-export-r209", legacy).await;

    project::project(&full, false).await.expect("full absorb");
    // Incremental sees a legacy notice → falls back internally to a full run.
    project::project_incremental(&incr).await.expect("incremental (fallback) absorb");

    assert_eq!(
        snapshot(&full).await,
        snapshot(&incr).await,
        "legacy fallback must match a full non-rebuild projection"
    );
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "fallback drains the change-set");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// A per-Tender content fingerprint keyed by natural key — its versions and their
/// texts — so an "untouched" assertion does not depend on surrogate ids.
async fn db_fingerprint(db: &Db, procedure_key: &str) -> String {
    let sql = format!(
        "SELECT group_concat(r, x'0a') FROM (
             SELECT tv.seq||'|'||tv.caused_by_notice_id||'|'||coalesce(x.value,'') AS r
               FROM tenders t JOIN tender_versions tv ON tv.tender_id = t.id
               LEFT JOIN tender_version_texts x ON x.tender_id = tv.tender_id AND x.seq = tv.seq
              WHERE t.procedure_key = '{procedure_key}'
              ORDER BY tv.seq, x.field)"
    );
    match db.scalar(&sql).await.expect("fingerprint") {
        Some(store::turso::Value::Text(s)) => s,
        _ => String::new(),
    }
}

/// Issue 93: retirement is now committed in BOUNDED chunks rather than one
/// unbounded transaction, and the split must be invisible.
///
/// The `changes` feed is ordered by its autoincrement cursor and is part of the
/// projection's byte-identity surface, so chunking retirement is only safe if the
/// `removed` events come out in exactly the same sequence however the work is
/// divided. That is the claim this pins: retire the same Tenders one-at-a-time and
/// all-at-once, and the whole canonical layer — feed included — must match.
///
/// Driven through the real API with an EMPTY plan, which is the orphan condition
/// (`retire_regrouped_tenders` retires every touched Tender whose key the new plan
/// did not reproduce), so every established Tender is retired.
#[tokio::test]
async fn retirement_is_identical_however_it_is_chunked() {
    async fn retire_with(name: &str, chunk: usize) -> (String, u64) {
        let (db, fetch, path) = scratch(name).await;
        establish(&db, fetch).await;
        // Extra Tenders so a chunk of 1 genuinely crosses several boundaries.
        for p in 0..5u64 {
            record(&db, fetch, &format!("X{p}-cn"), keyed(&format!("bt04-x{p:03}"), 1, "Extra")).await;
            record(&db, fetch, &format!("Y{p}-isl"), island(&format!("Extra island {p}"))).await;
        }
        project::project(&db, false).await.expect("establish");

        // A generous id span: ids with no `tenders` row are skipped, so this is
        // simply "every Tender", without needing a private reader.
        let live = count(&db, "SELECT COUNT(*) FROM tenders").await;
        assert!(live >= 12, "need enough Tenders to cross chunk boundaries, got {live}");
        let ids: Vec<i64> = (1..=live + 10).collect();

        // An empty plan means no touched Tender's key is reproduced → all orphaned.
        db.reset_plan().await.expect("reset plan");
        let retired = db
            .retire_regrouped_tenders_chunked(&ids, 1_000_000, chunk)
            .await
            .expect("retire");
        let snap = snapshot(&db).await;
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        (snap, retired)
    }

    let (one_shot, n_one) = retire_with("retire-oneshot", 10_000).await;
    let (chunked, n_chunk) = retire_with("retire-chunked", 1).await;
    let (paired, n_pair) = retire_with("retire-paired", 3).await;

    assert!(n_one >= 12, "the retirement must actually have happened, got {n_one}");
    assert_eq!((n_one, n_one), (n_chunk, n_pair), "the same Tenders must be retired");
    assert_eq!(
        one_shot, chunked,
        "retiring one Tender per chunk must be byte-identical to one transaction — \
         the cursor-ordered changes feed included"
    );
    assert_eq!(one_shot, paired, "an uneven chunk size must be identical too");
    assert!(one_shot.contains("removed"), "the feed must actually carry the removed events");
}

/// Issue 99 gate 1: a forced rewrite is a CONTENT no-op.
///
/// The fold skips a Tender whose chain of causing notices is unchanged, on the
/// assumption that the chain is a state key. It is only while the projection LOGIC
/// is fixed — a mapping change makes the same chain yield different content, and the
/// unchanged chain then discards it (issue 85's 2,185 factless shells; issue 98's
/// zero parties). The epoch forces the rewrite.
///
/// This pins the half that makes the mechanism SAFE: forced against UNCHANGED logic,
/// the rewrite must reproduce byte-identical content. If it did not, an epoch bump
/// would perturb every Tender it touched and the whole scheme would be unusable.
///
/// Staleness is simulated by writing an impossible stored epoch rather than by
/// making `PROJECTION_EPOCH` injectable — the production constant stays a constant,
/// and the test drives the exact condition the code branches on.
#[tokio::test]
async fn an_epoch_forced_rewrite_reproduces_identical_content() {
    let (db, fetch, path) = scratch("epoch-noop").await;
    establish(&db, fetch).await;
    for p in 0..4u64 {
        record(&db, fetch, &format!("E{p}-cn"), keyed(&format!("bt04-e{p:03}"), 1, "Epoch")).await;
        record(&db, fetch, &format!("E{p}-corr"), keyed(&format!("bt04-e{p:03}"), 2, "Epoch corr")).await;
    }
    project::project(&db, false).await.expect("establish");
    let before = snapshot_content(&db).await;

    // Nothing has changed, so an ordinary incremental fold is a no-op: it has no
    // unprojected notices at all. Re-mark everything and prove the chain-unchanged
    // early-return really does skip — this is the defect, asserted.
    db.unmark_projected_for_profiles(&["eforms:eforms-sdk-1.13"]).await.expect("re-mark");
    let skipped = project::project_incremental(&db).await.expect("re-fold, epoch current");
    assert_eq!(
        skipped.applied.versions_written, 0,
        "with a current epoch and unchanged chains the fold must skip every Tender — \
         this is the issue-99 defect, and gate 2 depends on it being real"
    );

    // Now age every Tender's stored epoch and re-fold: the chains are still
    // unchanged, so ONLY the epoch can force the rewrite.
    db.set_projection_epoch_for_test(-1).await.expect("age the stored epoch");
    db.unmark_projected_for_profiles(&["eforms:eforms-sdk-1.13"]).await.expect("re-mark");
    let forced = project::project_incremental(&db).await.expect("re-fold, epoch stale");
    assert!(
        forced.applied.versions_written > 0,
        "a stale epoch must force the rewrite the chain check skipped"
    );
    assert_eq!(
        before,
        snapshot_content(&db).await,
        "a forced rewrite under UNCHANGED logic must reproduce byte-identical content"
    );
    assert_eq!(
        count(
            &db,
            &format!(
                "SELECT COUNT(*) FROM tenders WHERE projection_epoch <> {}",
                store::canonical::PROJECTION_EPOCH
            ),
        )
        .await,
        0,
        "every rewritten Tender must be stamped with the current epoch"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 179: a profile-scoped refold must not owe the whole corpus a rewrite.
/// The refold job's two halves — requeue + `stamp_stale_for_profiles` — make
/// exactly the cohort's tenders rewrite (their chains are unchanged, so ONLY
/// the scoped stamp can force it), while a tender of another profile keeps its
/// chain-unchanged early-return and is never written. The global
/// PROJECTION_EPOCH is untouched throughout.
#[tokio::test]
async fn a_scoped_stale_stamp_rewrites_only_the_profiles_tenders() {
    let (db, fetch, path) = scratch("scoped-stamp").await;
    // Two single-tender cohorts under different profiles.
    record_p(&db, fetch, "R2-cn", "ted-export-r208", keyed("bt04-r208", 1, "Legacy era")).await;
    record_p(&db, fetch, "S13-cn", "eforms:eforms-sdk-1.13", keyed("bt04-s13", 1, "Modern era"))
        .await;
    project::project(&db, false).await.expect("establish");
    let before = snapshot_content(&db).await;

    // The refold job's two steps, scoped to the r208 cohort.
    let requeued =
        db.unmark_projected_for_profiles(&["ted-export-r208"]).await.expect("requeue");
    assert_eq!(requeued, 1);
    let stamped = db.stamp_stale_for_profiles(&["ted-export-r208"]).await.expect("stamp");
    assert_eq!(stamped, 1, "exactly the cohort's tender is stamped");
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM tenders WHERE projection_epoch = 0").await,
        1,
        "the stamp ages the cohort's tender and nobody else"
    );

    // The re-fold rewrites the stamped tender — its chain is unchanged, so the
    // stamp is doing the forcing — and leaves the other cohort untouched.
    let refolded = project::project(&db, false).await.expect("re-fold");
    assert_eq!(
        refolded.applied.versions_written, 1,
        "exactly the stamped tender's chain rewrites — the other cohort's \
         chain-unchanged early-return must hold"
    );
    assert_eq!(
        before,
        snapshot_content(&db).await,
        "a scoped rewrite under UNCHANGED logic reproduces byte-identical content"
    );
    assert_eq!(
        count(
            &db,
            &format!(
                "SELECT COUNT(*) FROM tenders WHERE projection_epoch <> {}",
                store::canonical::PROJECTION_EPOCH
            ),
        )
        .await,
        0,
        "the rewrite restamps the cohort with the current epoch"
    );

    // An unrelated-profile stamp is a no-op: nothing matches, nothing ages.
    assert_eq!(db.stamp_stale_for_profiles(&["no-such-profile"]).await.expect("noop"), 0);

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 58 v2, step 1: the durable OJS adjacency is written wherever the plan
/// is built, and the watermark attests coverage with the right lifecycle — a
/// full plan build ESTABLISHES it, an incremental ADVANCES it, and an
/// incremental on a never-established base leaves it 0 (the closure must not
/// trust rows no full pass vouched for). No behavior change to the fold itself.
#[tokio::test]
async fn the_legacy_adjacency_rows_and_watermark_follow_the_plan_builds() {
    let (db, fetch, path) = scratch("adjacency").await;
    // A two-notice legacy chain: 200-2008 references 100-2008; both carry their
    // own OJS number via publication_id. An eForms notice rides along to prove
    // non-legacy rows are never written.
    let mut cn = island("Legacy CN");
    let mut can = island("Legacy CAN");
    can.values.push(ValueRow {
        section_id: "PROC".into(),
        field_id: "TED-REF_OJS".into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: Some("ojs".into()), value: "100-2008".into(), is_ref: true },
    });
    record_p(&db, fetch, "100-2008", "ted-export-r209", cn.clone()).await;
    record_p(&db, fetch, "200-2008", "ted-export-r209", can).await;
    record(&db, fetch, "E-cn", keyed("bt04-adj", 1, "Modern")).await;
    project::project(&db, false).await.expect("full projection");

    // The full plan build wrote self + edge rows for the legacy pair only, and
    // established the watermark at the newest planned notice.
    let rows = count(&db, "SELECT COUNT(*) FROM legacy_ojs_keys").await;
    assert_eq!(rows, 3, "two self keys + one edge key; the eForms notice writes none");
    let shared = count(
        &db,
        "SELECT COUNT(DISTINCT notice_id) FROM legacy_ojs_keys WHERE ojs_key = 2008000000100",
    )
    .await;
    assert_eq!(shared, 2, "the referenced key names both the owner and the referrer");
    let max_id = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(
        db.legacy_adjacency_watermark().await.expect("watermark"),
        max_id,
        "a full plan build establishes coverage up to the newest planned notice"
    );

    // An incremental delta ADVANCES the established watermark…
    record_p(&db, fetch, "300-2008", "ted-export-r209", island("Legacy late")).await;
    project::project_incremental(&db).await.expect("incremental (legacy → v1 fallback is fine)");
    let new_max = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(db.legacy_adjacency_watermark().await.expect("watermark"), new_max);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM legacy_ojs_keys").await,
        4,
        "the late notice's self key joined the durable rows"
    );

    // …but on a never-established base it refuses to move.
    let raw = store::turso::Builder::new_local(&path).build().await.expect("raw db");
    raw.connect()
        .expect("raw conn")
        .execute("UPDATE legacy_adjacency SET watermark = 0 WHERE id = 0", ())
        .await
        .expect("reset the base");
    db.advance_legacy_adjacency(9_999_999).await.expect("advance");
    assert_eq!(
        db.legacy_adjacency_watermark().await.expect("watermark"),
        0,
        "advance must not establish: only a full pass vouches for the base"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 58 v2, step 2: the backfill job re-derives EXACTLY the rows the
/// choke-point writer records — proven by set equality on the same corpus —
/// and establishes the watermark. This is the drift test between the two
/// writers: both must read keys through `Ident::read`, so a corpus the plan
/// build has annotated, wiped back to the pre-feature state (no rows,
/// watermark 0), must come back byte-identical from the sweep.
#[tokio::test]
async fn the_backfill_rederives_the_choke_points_rows_exactly() {
    let (db, fetch, path) = scratch("adjbackfill").await;
    let mut can = island("Legacy CAN");
    can.values.push(ValueRow {
        section_id: "PROC".into(),
        field_id: "TED-REF_OJS".into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: Some("ojs".into()), value: "100-2008".into(), is_ref: true },
    });
    record_p(&db, fetch, "100-2008", "ted-export-r209", island("Legacy CN")).await;
    record_p(&db, fetch, "200-2008", "ted-export-r209", can).await;
    record(&db, fetch, "E-cn", keyed("bt04-adjb", 1, "Modern")).await;
    project::project(&db, false).await.expect("full projection");

    let baseline = key_rows(&db).await;
    assert_eq!(baseline.len(), 3, "choke point wrote two self keys + one edge key");

    // Wipe to the pre-feature state a standing prod corpus is in.
    let raw = store::turso::Builder::new_local(&path).build().await.expect("raw db");
    let conn = raw.connect().expect("raw conn");
    conn.execute("DELETE FROM legacy_ojs_keys", ()).await.expect("wipe rows");
    conn.execute("UPDATE legacy_adjacency SET watermark = 0 WHERE id = 0", ())
        .await
        .expect("wipe watermark");
    drop(conn);

    let mut ticks = 0u64;
    let done = project::backfill_legacy_adjacency(&db, |n| ticks = n).await.expect("backfill");
    assert_eq!(key_rows(&db).await, baseline, "the sweep and the choke point must never drift");
    assert_eq!((done.swept, done.keys), (2, 3), "two legacy notices, three key rows; eForms skipped");
    assert_eq!(ticks, 2, "progress surfaced per chunk");
    let max_id = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(done.watermark, max_id);
    assert_eq!(db.legacy_adjacency_watermark().await.expect("watermark"), max_id);

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// The full `(ojs_key, notice_id)` set, encoded one pair per scalar probe so the
/// comparison needs no direct row access (`Db::scalar` is the test surface).
async fn key_rows(db: &Db) -> Vec<(i64, i64)> {
    let n = count(db, "SELECT COUNT(*) FROM legacy_ojs_keys").await;
    let mut out = Vec::new();
    for i in 0..n {
        let k = count(
            db,
            &format!(
                "SELECT ojs_key FROM legacy_ojs_keys ORDER BY ojs_key, notice_id LIMIT 1 OFFSET {i}"
            ),
        )
        .await;
        let id = count(
            db,
            &format!(
                "SELECT notice_id FROM legacy_ojs_keys ORDER BY ojs_key, notice_id LIMIT 1 OFFSET {i}"
            ),
        )
        .await;
        out.push((k, id));
    }
    out
}

/// Issue 133: the 2026-07-30 shape. A killed rebuild's `reset_tender_layer` is
/// DDL — durable the instant it runs — so the layer sits empty with every
/// later signal green. The next incremental fold must REFUSE to compound that
/// (folding a daily delta onto a wiped corpus and calling it success), while a
/// rebuild — the repair path — must pass. The witness is `layer_presence`,
/// which survives the wipe in its own table; a box that never observed has no
/// witness and the guard stays silent (fresh installs fold from empty
/// legitimately).
#[tokio::test]
async fn an_incremental_fold_refuses_a_wiped_layer_a_rebuild_repairs_it() {
    let (db, fetch, path) = scratch("wipe-guard").await;

    // A fresh box folds from empty without complaint: no witness, no guard.
    establish(&db, fetch).await;
    assert!(count(&db, "SELECT COUNT(*) FROM tenders").await >= 3, "established");

    // The supervisor's observer records the populated state...
    db.observe_layer_presence(1_000_000).await.expect("observe populated");

    // ...then the killed rebuild's committed wipe — FKs off around the DROP,
    // exactly as the real rebuild runs it (issue 19).
    db.set_foreign_keys(false).await.expect("fk off");
    db.reset_tender_layer().await.expect("the 07-30 wipe");
    db.set_foreign_keys(true).await.expect("fk on");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM tenders").await, 0, "layer wiped");

    let refused = project::project_incremental(&db).await;
    let msg = match refused {
        Err(e) => e.to_string(),
        Ok(report) => panic!("an incremental fold onto a wiped layer must refuse, got {report:?}"),
    };
    assert!(
        msg.contains("rebuild") && msg.contains("133"),
        "the refusal names the repair path and its issue: {msg}"
    );

    // The same guard passes the repair path, and the repair makes the layer
    // whole again — after which incremental folds are welcome back.
    project::project(&db, true).await.expect("a rebuild is the repair path");
    assert!(count(&db, "SELECT COUNT(*) FROM tenders").await >= 3, "repaired");
    project::project_incremental(&db).await.expect("incremental folds resume after repair");

    // The end assertion (wired at the tail of both entry points): a run that
    // began populated and ends empty must refuse to report success. Driven
    // directly — no real fold empties a layer on purpose — against the state a
    // mid-run wipe would leave.
    db.set_foreign_keys(false).await.expect("fk off");
    db.reset_tender_layer().await.expect("wipe again");
    db.set_foreign_keys(true).await.expect("fk on");
    let post = project::wipe_guard_post(&db, true).await;
    let msg = match post {
        Err(e) => e.to_string(),
        Ok(()) => panic!("a run that emptied a populated layer must not report success"),
    };
    assert!(msg.contains("failed job") && msg.contains("133"), "names the signal and issue: {msg}");
    // Relative, not absolute: a run that began empty and ends empty is a
    // legitimate first fold, not a wipe.
    project::wipe_guard_post(&db, false).await.expect("began-empty is not a wipe");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
