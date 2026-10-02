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
            published_at: Some(store::Stamp::utc(0)),
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

/// A keyed notice that always publishes lots LOT-1..LOT-3 and a sole LotsGroup
/// GLO-1, with a `GroupComposition` making GLO-1 contain `members` (issue 237's
/// sole-group inference resolves the group, so no BT-330 is needed). Title and
/// lot facts are FIXED across calls, so the ONLY thing that can differ between
/// two versions is the group composition — which is exactly issue 283's scenario.
/// `pub_at` varies the notice's dispatch instant (a version field, not a fact),
/// only to order the versions.
fn grouped(key: &str, pub_at: i64, members: &[&str]) -> Parsed {
    let mut parsed = Parsed {
        sections: vec![
            sec("PROC", "Procedure", None),
            sec("LOT-1", "Lot", Some("PROC")),
            sec("LOT-2", "Lot", Some("PROC")),
            sec("LOT-3", "Lot", Some("PROC")),
            sec("GLO-1", "LotsGroup", Some("PROC")),
            sec("COMP", "GroupComposition", None),
        ],
        values: vec![
            id_val("PROC", "BT-04-notice", key),
            date_val("PROC", "BT-05(a)-notice", pub_at),
            text_val("PROC", "BT-21-Procedure", "Grouped procedure"),
            // Fixed lot facts (NOT pub_at-derived): the lots must be byte-identical
            // across versions so only the composition moves.
            date_val("LOT-1", "BT-131(d)-Lot", 700_000_001),
            date_val("LOT-2", "BT-131(d)-Lot", 700_000_002),
            date_val("LOT-3", "BT-131(d)-Lot", 700_000_003),
        ],
    };
    for (i, m) in members.iter().enumerate() {
        // BT-1375-Procedure on the GroupComposition section — one ref per member.
        parsed.values.push(ValueRow {
            section_id: "COMP".into(),
            field_id: "BT-1375-Procedure".into(),
            ordinal: i as i64,
            value: NoticeValue::Id { scheme: None, value: (*m).into(), is_ref: true },
        });
    }
    // Deliberately NO buyer: a `Fact::Party` carries the notice_id it came from, so
    // a republished party differs every version and would mask the composition as
    // the sole delta. With no party, the tender-level facts are just the (constant)
    // title — leaving group composition as the only thing that can move.
    parsed
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

/// A delta containing a LEGACY notice with no component to join (its keys match
/// nothing durable) stays SCOPED through the closure walk (issue 58 v2, step 3 —
/// this was the v1 full-fallback trigger) and still matches a full projection.
#[tokio::test]
async fn incremental_legacy_delta_stays_scoped_and_matches_full() {
    let (full, ff, pf) = scratch("legfull").await;
    let (incr, fi, pi) = scratch("legincr").await;
    establish(&full, ff).await;
    establish(&incr, fi).await;

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
    project::project_incremental(&incr).await.expect("incremental (closure) absorb");

    assert_eq!(
        snapshot(&full).await,
        snapshot(&incr).await,
        "a scoped legacy delta must match a full non-rebuild projection"
    );
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "the run drains the change-set");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// The `changes` feed as an ORDER-FREE set — for comparing two projection paths
/// that emit the same events at different cursor positions (retire-before-apply
/// vs retire-after-apply).
async fn changes_set(db: &Db) -> String {
    match db
        .scalar(
            "SELECT group_concat(r, x'0a') FROM (
                 SELECT entity_kind||'|'||op||'|'||coalesce(version_seq,-1)||'|'||entity_id AS r
                   FROM changes ORDER BY entity_kind, op, version_seq, entity_id)",
        )
        .await
        .expect("changes set")
    {
        Some(store::turso::Value::Text(s)) => s,
        _ => String::new(),
    }
}

/// Reparse an existing notice in place with new parsed content (issue 100's
/// primitive): clears its parsed rows, inserts the new ones, marks it
/// unprojected so the next projection re-folds it — the way a real reparse
/// changes a notice's BT-04 key.
async fn reparse(db: &Db, fetch_id: i64, pub_id: &str, parsed: Parsed) {
    // A real reparse re-interprets the SAME raw bytes, so content_hash is stable —
    // and `notice_state` looks the notice up by (source, publication_id,
    // content_hash), so this must match what `record_p` wrote (`h-{pub_id}`).
    let applied = db
        .reparse_notice(
            &Notice {
                source: SOURCE.into(),
                publication_id: pub_id.into(),
                content_hash: format!("h-{pub_id}"),
                profile: "eforms:eforms-sdk-1.13".into(),
                declared_version: None,
                fetch_id,
                member_path: format!("{pub_id}.xml"),
                ingested_at: 0,
                published_at: Some(store::Stamp::utc(0)),
                dispatched_at: None,
            },
            &parsed,
        )
        .await
        .expect("reparse notice");
    assert_eq!(
        applied,
        store::Reparsed::Replaced,
        "reparse must find {pub_id} by its full identity and re-parse it"
    );
}

/// Issue 278: a reparse that regroups a keyed Tender to a new BT-04 — or upgrades
/// an island to a key — leaves the old Tender's key un-produced. The scoped
/// incremental path retires it via its touched set; the FULL non-rebuild path used
/// to retire only `ojs:%`, so the old keyed/island Tender survived as a ghost
/// (~45k measured on prod, 2026-08-26). Both paths must now converge on the same
/// layer, and the full path must leave NO notice mapped to two Tenders.
#[tokio::test]
async fn a_regroup_reparse_retires_keyed_and_island_ghosts_on_the_full_path() {
    let (full, ff, pf) = scratch("regroupfull").await;
    let (incr, fi, pi) = scratch("regroupincr").await;
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        record(db, fetch, "A-cn", keyed("bt04-0001", 1, "Alpha CN")).await; // 1-notice keyed
        record(db, fetch, "B-cn", keyed("bt04-0002", 1, "Beta CN")).await; // untouched keyed
        record(db, fetch, "ISL", island("Island one")).await; // island
        project::project(db, false).await.expect("establish");
    }
    // Regroup: A moves to a new key, the island gains a key — both orphan their old
    // Tender (bt04-0001's Tender, and the island Tender).
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        reparse(db, fetch, "A-cn", keyed("bt04-9999", 1, "Alpha CN")).await;
        reparse(db, fetch, "ISL", keyed("bt04-8888", 5, "Island one")).await;
    }
    project::project(&full, false).await.expect("full reproject");
    project::project_incremental(&incr).await.expect("incremental reproject");

    assert_eq!(
        snapshot_content(&full).await,
        snapshot_content(&incr).await,
        "full non-rebuild must retire the regrouped keyed/island ghosts, matching incremental",
    );
    assert_eq!(
        count(&full, "SELECT COUNT(*) FROM tenders WHERE procedure_key = 'bt04-0001'").await,
        0,
        "the regrouped-away keyed Tender is retired",
    );
    assert_eq!(
        count(&full, "SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL AND procedure_key IS NULL").await,
        0,
        "the upgraded island Tender is retired",
    );
    assert_eq!(
        count(
            &full,
            "SELECT COUNT(*) FROM (SELECT caused_by_notice_id FROM tender_versions
                 GROUP BY caused_by_notice_id HAVING COUNT(DISTINCT tender_id) > 1)",
        )
        .await,
        0,
        "no notice maps to two Tenders — the issue-278 ghost signature is absent",
    );
    assert!(
        count(&full, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND op = 'removed'").await >= 2,
        "removed change events were emitted for the retired ghosts",
    );

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Issue 278: given a ghost Tender (a notice under two Tenders — the pre-fix state
/// the full path used to leave), `ghost_census` finds the shared notice, and marking
/// it unprojected + an incremental project retires the ghost via
/// `retire_regrouped_tenders` while keeping the real Tender.
///
/// This is the whole repair path, and it is the one that actually cleared prod's
/// ~45k: no bulk sweep ever ran: the incremental fold that drained the reparse
/// backlog on 2026-08-26 retired them exactly as this test does. The census that
/// replaced the sweep only counts.
#[tokio::test]
async fn the_ghost_sweep_finds_and_retires_a_duplicated_tender() {
    let (db, fetch, path) = scratch("ghostsweep").await;
    record(&db, fetch, "A-cn", keyed("bt04-0001", 1, "Alpha CN")).await;
    record(&db, fetch, "B-cn", keyed("bt04-0002", 1, "Beta CN")).await;
    project::project(&db, false).await.expect("establish");

    // Inject the ghost by hand: a second Tender under a DIFFERENT key holding
    // A-cn's notice, exactly the pre-fix duplication (a notice under two Tenders).
    let nid = count(&db, "SELECT id FROM notices WHERE publication_id = 'A-cn'").await;
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, created_at)
         VALUES (9999, 'ted', 'bt04-GHOST', 'procedure', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
         VALUES (9999, 1, ?, 0, 'ghost')",
        (store::turso::Value::Integer(nid),),
    )
    .await
    .unwrap();

    let census = db.ghost_census(1_000_000, 100, &|| false, &|_, _| {}).await.unwrap();
    assert_eq!(census.ghost_notices, 1, "the shared notice is the only dup");
    assert_eq!(census.ghost_tender_refs, 2, "it is claimed by two Tenders");
    assert_eq!(census.sample.first().map(|g| g.notice_id), Some(nid), "and the census names it");

    // The sweep: mark the dup unprojected, then an incremental project retires the
    // ghost of the pair (the un-produced key) and keeps the real Tender.
    let requeued = db.unmark_projected_by_ids(&[nid]).await.unwrap();
    assert_eq!(requeued, 1, "the dup notice was re-queued");
    project::project_incremental(&db).await.expect("sweep project");

    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM tenders WHERE procedure_key = 'bt04-GHOST'").await,
        0,
        "the ghost Tender is retired",
    );
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM tenders WHERE procedure_key = 'bt04-0001'").await,
        1,
        "the real Tender is kept",
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM (SELECT caused_by_notice_id FROM tender_versions
                 GROUP BY caused_by_notice_id HAVING COUNT(DISTINCT tender_id) > 1)",
        )
        .await,
        0,
        "no notice maps to two Tenders after the sweep",
    );
    assert!(
        count(&db, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND op = 'removed'").await >= 1,
        "the ghost's retirement announced itself",
    );
    // An idempotent second pass finds nothing to do.
    let after = db.ghost_census(1_000_000, 100, &|| false, &|_, _| {}).await.unwrap();
    assert_eq!(after.ghost_notices, 0, "no dups remain — a re-run is a no-op");
    assert!(after.sample.is_empty(), "and nothing is sampled");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// A legacy notice with `pub_id` as its own OJS number, plus `refs` edges.
fn legacy_notice(title: &str, refs: &[&str]) -> Parsed {
    let mut parsed = Parsed {
        sections: vec![sec("PROC", "Notice", None)],
        values: vec![
            text_val("PROC", "TED-TITLE", title),
            date_val("PROC", "TED-DS_DATE_DISPATCH", 42),
        ],
    };
    for (i, r) in refs.iter().enumerate() {
        parsed.values.push(ValueRow {
            section_id: "PROC".into(),
            field_id: "TED-REF_OJS".into(),
            ordinal: i as i64,
            value: NoticeValue::Id { scheme: Some("ojs".into()), value: (*r).into(), is_ref: true },
        });
    }
    parsed
}

/// Issue 58 v2 step 3, the correctness crux: a delta notice BRIDGING two existing
/// single-notice legacy Tenders must merge them into ONE Tender — which only
/// happens if the closure walk pulls BOTH existing notices into the plan through
/// their durable key rows. A scoped run without the closure would group the
/// bridge alone and diverge from the full projection.
#[tokio::test]
async fn a_legacy_bridge_delta_merges_existing_components_like_full() {
    let (full, ff, pf) = scratch("bridgefull").await;
    let (incr, fi, pi) = scratch("bridgeincr").await;
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        establish(db, fetch).await;
        record_p(db, fetch, "100-2008", "ted-export-r209", legacy_notice("Legacy A", &[])).await;
        record_p(db, fetch, "200-2008", "ted-export-r209", legacy_notice("Legacy B", &[])).await;
        project::project(db, false).await.expect("establish legacy pair");
    }

    // The bridge: references BOTH components' keys.
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        record_p(
            db,
            fetch,
            "300-2008",
            "ted-export-r209",
            legacy_notice("Legacy bridge", &["100-2008", "200-2008"]),
        )
        .await;
    }
    project::project(&full, false).await.expect("full absorb");
    project::project_incremental(&incr).await.expect("incremental (closure) absorb");

    // Content byte-identity. The `changes` FEED is compared as a set below: the
    // full path retires the absorbed legacy Tender AFTER apply, the incremental
    // retires regrouped Tenders BEFORE apply (deliberately — the old Tender must
    // be gone before its notices reappear elsewhere), so the same events carry
    // different cursor positions. Both orders are coherent records of the merge.
    assert_eq!(
        snapshot_content(&full).await,
        snapshot_content(&incr).await,
        "the bridge merge content must be identical to a full non-rebuild projection"
    );
    assert_eq!(
        changes_set(&full).await,
        changes_set(&incr).await,
        "the bridge merge must emit the same change events (as a set)"
    );
    let tenders = count(
        &incr,
        "SELECT COUNT(DISTINCT tender_id) FROM tender_versions tv
          JOIN notices n ON n.id = tv.caused_by_notice_id
         WHERE n.publication_id IN ('100-2008','200-2008','300-2008')",
    )
    .await;
    assert_eq!(tenders, 1, "all three legacy notices fold into ONE merged Tender");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Issue 58 v2 step 3, late back-reference: an EXISTING notice references an OJS
/// number that only now arrives as the delta notice's OWN number. The closure
/// must find the existing notice through the edge row written when IT was
/// planned — the delta's self key is the seed.
#[tokio::test]
async fn a_late_back_referenced_legacy_delta_joins_its_referrer() {
    let (full, ff, pf) = scratch("backreffull").await;
    let (incr, fi, pi) = scratch("backrefincr").await;
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        establish(db, fetch).await;
        record_p(db, fetch, "100-2008", "ted-export-r209", legacy_notice("Legacy plain", &[])).await;
        // References 300-2008, which does not exist yet.
        record_p(db, fetch, "200-2008", "ted-export-r209", legacy_notice("Legacy referrer", &["300-2008"]))
            .await;
        project::project(db, false).await.expect("establish referrer");
    }

    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        record_p(db, fetch, "300-2008", "ted-export-r209", legacy_notice("Legacy target", &[])).await;
    }
    project::project(&full, false).await.expect("full absorb");
    project::project_incremental(&incr).await.expect("incremental (closure) absorb");

    assert_eq!(
        snapshot(&full).await,
        snapshot(&incr).await,
        "the late back-reference must match a full non-rebuild projection"
    );
    let tenders = count(
        &incr,
        "SELECT COUNT(DISTINCT tender_id) FROM tender_versions tv
          JOIN notices n ON n.id = tv.caused_by_notice_id
         WHERE n.publication_id IN ('200-2008','300-2008')",
    )
    .await;
    assert_eq!(tenders, 1, "referrer and target fold into one Tender");
    let plain = count(
        &incr,
        "SELECT COUNT(DISTINCT tender_id) FROM tender_versions tv
          JOIN notices n ON n.id = tv.caused_by_notice_id
         WHERE n.publication_id = '100-2008'",
    )
    .await;
    assert_eq!(plain, 1, "the unrelated legacy notice stays its own Tender");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Issue 58 v2 step 3, the watermark gate: with the witness never established
/// (watermark 0) a legacy delta must take the FULL path. Proven by the watermark
/// itself: the full path ESTABLISHES it (advance refuses a 0 base), so watermark
/// == max parsed id afterwards ⟺ the fallback ran.
#[tokio::test]
async fn a_stale_witness_forces_the_full_fallback() {
    let (db, fetch, path) = scratch("gatewm").await;
    establish(&db, fetch).await;
    record_p(&db, fetch, "100-2008", "ted-export-r209", legacy_notice("Legacy seed", &[])).await;
    project::project(&db, false).await.expect("establish legacy");

    let raw = store::turso::Builder::new_local(&path).build().await.expect("raw db");
    raw.connect()
        .expect("raw conn")
        .execute("UPDATE legacy_adjacency SET watermark = 0 WHERE id = 0", ())
        .await
        .expect("wipe the witness");

    record_p(&db, fetch, "200-2008", "ted-export-r209", legacy_notice("Legacy late", &["100-2008"]))
        .await;
    project::project_incremental(&db).await.expect("incremental (gated → full)");

    let max_id = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(
        db.legacy_adjacency_watermark().await.expect("watermark"),
        max_id,
        "only the full path establishes — the gate must have routed there"
    );
    let tenders = count(
        &db,
        "SELECT COUNT(DISTINCT tender_id) FROM tender_versions tv
          JOIN notices n ON n.id = tv.caused_by_notice_id
         WHERE n.publication_id IN ('100-2008','200-2008')",
    )
    .await;
    assert_eq!(tenders, 1, "the fallback still groups the chain correctly");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 58 v2 step 3, the coverage-gap verify: a watermark that CLAIMS coverage
/// while a projected parsed notice sits above it (a rollback binary folded
/// without writing key rows) must be distrusted — full fallback, and the full
/// pass repairs the witness.
#[tokio::test]
async fn a_coverage_gap_forces_the_full_fallback() {
    let (db, fetch, path) = scratch("gategap").await;
    establish(&db, fetch).await;
    record_p(&db, fetch, "100-2008", "ted-export-r209", legacy_notice("Legacy seed", &[])).await;
    project::project(&db, false).await.expect("establish legacy");

    // Simulate the rollback hole: pull the watermark BELOW the newest projected
    // notice, as if a pre-feature binary had folded it without writing rows.
    let max_id = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    let raw = store::turso::Builder::new_local(&path).build().await.expect("raw db");
    raw.connect()
        .expect("raw conn")
        .execute(
            "UPDATE legacy_adjacency SET watermark = ? WHERE id = 0",
            (store::turso::Value::Integer(max_id - 1),),
        )
        .await
        .expect("pull the watermark under a projected notice");

    record_p(&db, fetch, "200-2008", "ted-export-r209", legacy_notice("Legacy late", &["100-2008"]))
        .await;
    project::project_incremental(&db).await.expect("incremental (gap → full)");

    let new_max = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(
        db.legacy_adjacency_watermark().await.expect("watermark"),
        new_max,
        "the full pass repairs the witness to true coverage"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 58 v2 step 3, the cap: an over-cap component reports the fallback reason
/// instead of a wrong scope; the production cap admits the same small component.
#[tokio::test]
async fn an_over_cap_closure_reports_fallback() {
    let (db, fetch, path) = scratch("gatecap").await;
    establish(&db, fetch).await;
    record_p(&db, fetch, "100-2008", "ted-export-r209", legacy_notice("Legacy A", &[])).await;
    record_p(&db, fetch, "200-2008", "ted-export-r209", legacy_notice("Legacy B", &["100-2008"]))
        .await;
    project::project(&db, false).await.expect("establish legacy chain");

    let seeds = vec![2_008_000_000_100];
    let over = project::legacy_closure_capped(&db, &seeds, 1).await.expect("walk (tiny cap)");
    let reason = over.expect_err("a 2-notice component must exceed a cap of 1");
    assert!(reason.contains("exceeds cap"), "the reason names the cap: {reason}");

    let ok = project::legacy_closure_capped(&db, &seeds, 500_000).await.expect("walk (real cap)");
    let (notices, tenders) = ok.expect("the production cap admits the component");
    assert_eq!(notices.len(), 2, "both chain notices in scope");
    assert_eq!(tenders.len(), 1, "their one Tender in scope");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
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
    // The issue-108 split: the all-skip fold SAYS so in its own report — every
    // consideration went to the verified-unchanged exit, none to the write path.
    // Before these counters, this fold and a fold that wrote everything were
    // indistinguishable from their reports, which is how issue 85's 2,185
    // factless shells could read as "fully projected".
    assert_eq!(
        (skipped.applied.tenders_written, skipped.applied.tenders_unchanged),
        (0, skipped.tenders),
        "every touched Tender verified current, none written"
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
    // And the inverse split under identical inputs: only the stored epoch
    // differs, so every consideration now goes to the write path.
    assert_eq!(
        (forced.applied.tenders_written, forced.applied.tenders_unchanged),
        (forced.tenders, 0),
        "every touched Tender rewritten, none skipped"
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
    project::project_incremental(&db).await.expect("incremental (scoped legacy closure)");
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
    let done =
        project::backfill_legacy_adjacency(&db, |t| ticks = t.swept).await.expect("backfill");
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

/// Issue 228: the sweep must keep advancing through a stretch of corpus that
/// holds NO legacy notices, and must reach the target rather than stopping at
/// the first window that finds nothing.
///
/// The bug this pins: the walk used to terminate on an empty chunk and let the
/// row LIMIT decide how far a query scanned. With a legacy pre-filter that
/// combination cannot stop early in a legacy-free range — one query scans to the
/// end of the table to prove no match remains (~3.3M rows and ~40 silent minutes
/// on prod) — and, worse, an eForms notice ordered BEFORE a legacy one would
/// have ended the sweep early had the filter ever returned an empty chunk mid-
/// corpus. Windowing the id range fixes both: every window advances the cursor,
/// finding nothing is normal, and only the target ends the walk.
///
/// A window of 1 makes each notice id its own window over a 5-notice corpus, so
/// the legacy-free stretch is real rather than simulated.
#[tokio::test]
async fn the_backfill_sweeps_past_a_stretch_with_no_legacy_notices() {
    let (db, fetch, path) = scratch("adjbackfill-tail").await;
    // A legacy notice FIRST, then three eForms notices, then a second legacy one:
    // the sweep must cross the middle stretch and still find the last row.
    record_p(&db, fetch, "300-2008", "ted-export-r209", island("Legacy CN")).await;
    record(&db, fetch, "E-1", keyed("bt04-tail-1", 1, "Modern one")).await;
    record(&db, fetch, "E-2", keyed("bt04-tail-2", 1, "Modern two")).await;
    record(&db, fetch, "E-3", keyed("bt04-tail-3", 1, "Modern three")).await;
    record_p(&db, fetch, "400-2008", "ted-export-r209", island("Legacy CAN")).await;
    project::project(&db, false).await.expect("full projection");

    let baseline = key_rows(&db).await;
    assert_eq!(baseline.len(), 2, "one self key per legacy notice");

    let raw = store::turso::Builder::new_local(&path).build().await.expect("raw db");
    let conn = raw.connect().expect("raw conn");
    conn.execute("DELETE FROM legacy_ojs_keys", ()).await.expect("wipe rows");
    conn.execute("UPDATE legacy_adjacency SET watermark = 0 WHERE id = 0", ())
        .await
        .expect("wipe watermark");
    drop(conn);

    let mut ticks = Vec::new();
    let done = project::backfill_legacy_adjacency_windowed(&db, 1, |n| ticks.push(n))
        .await
        .expect("backfill");

    // Both legacy notices found — the one after the gap is the assertion that
    // matters, since the old loop would have stopped at the gap.
    assert_eq!(key_rows(&db).await, baseline, "the sweep crossed the gap and re-derived both");
    assert_eq!((done.swept, done.keys), (2, 2), "two legacy notices; the three eForms rows skipped");

    // Progress fired once per window, including the windows that found nothing —
    // that visible movement is the whole point of issue 228. With a window of 1
    // there is one tick per id in the corpus, and the counter is non-decreasing.
    let max_id = count(&db, "SELECT MAX(id) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(ticks.len() as i64, max_id, "one progress tick per window, gaps included");
    assert!(
        ticks.windows(2).all(|w| w[0].swept <= w[1].swept),
        "swept never goes backwards: {ticks:?}"
    );
    assert_eq!(ticks.last().map(|t| t.swept), Some(2), "the last tick reports both legacy notices");

    // Issue 65 — the assertion that actually matches the operator complaint. The
    // count alone was never the signal: across the three eForms notices it does
    // not move, which is exactly what looked like a hang for 40 minutes on prod.
    // The CURSOR is what distinguishes working from wedged, so pin that it climbs
    // strictly while `swept` sits still, and that every tick carries the target it
    // is climbing towards (a position with no destination is not a progress bar).
    assert!(
        ticks.windows(2).all(|w| w[0].cursor < w[1].cursor),
        "the cursor advances on EVERY tick, gaps included: {ticks:?}"
    );
    assert!(ticks.iter().all(|t| t.target == max_id), "every tick names the sweep's end: {ticks:?}");
    assert_eq!(ticks.last().map(|t| t.cursor), Some(max_id), "the last tick reaches the target");
    let gap: Vec<&project::SweepTick> =
        ticks.iter().filter(|t| t.cursor > 1 && t.cursor < max_id).collect();
    assert!(
        gap.iter().all(|t| t.swept == 1),
        "through the legacy-free stretch the count is frozen at 1 — only the cursor moves: {gap:?}"
    );
    assert!(gap.len() >= 3, "the three eForms notices each produced a silent-but-moving tick");

    // And the walk still ends by reaching the target, establishing coverage.
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


/// Issue 262: the incremental plan build must be VISIBLE and STOPPABLE. Fold
/// job 303 (2.7M-notice delta) ran with `phase: None` for its whole plan build
/// and took ~17 minutes to honour a cancel — the plan-build loops carried no
/// progress events and no stop checks. Three claims, one scenario each:
/// the observed run surfaces `Planning` (per pass, over that pass's own total)
/// and `Grouped`; an immediate stop returns `stopped` having planned nothing;
/// and a stop that lands during pass 2 does NOT advance the legacy-adjacency
/// watermark, whose attestation is only true of a COMPLETED plan build.
#[tokio::test]
async fn the_incremental_plan_build_reports_progress_and_stops_within_a_chunk() {
    use ingest::project::Progress;
    use std::sync::Mutex;

    let (db, fid, path) = scratch("observed").await;
    establish(&db, fid).await;
    record(&db, fid, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;
    record(&db, fid, "ISL2", island("Island two")).await;

    let events: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let report = project::project_incremental_observed_stoppable(
        &db,
        |p| {
            events.lock().unwrap().push(match p {
                Progress::Planning { .. } => "planning",
                Progress::Identity { .. } => "identity",
                Progress::Grouped { .. } => "grouped",
                Progress::Applying { .. } => "applying",
                Progress::PrePass { .. } => "pre-pass",
            });
        },
        &|| false,
    )
    .await
    .expect("observed incremental");
    assert!(!report.stopped);
    let seen = events.into_inner().unwrap();
    // Issue 305: pass 1 reports under its OWN name now — the two passes are
    // distinguishable in the record instead of one "planning" counter that
    // resets midway.
    assert!(
        seen.contains(&"identity") && seen.contains(&"planning"),
        "both passes must report, each under its own name: {seen:?}"
    );
    assert!(seen.contains(&"grouped"), "the grouping boundary must report: {seen:?}");

    // An immediate stop is honoured in pass 1: nothing planned, nothing folded,
    // and the report says stopped rather than pretending completion.
    record(&db, fid, "ISL3", island("Island three")).await;
    let stopped =
        project::project_incremental_observed_stoppable(&db, |_| {}, &|| true).await.expect("stop");
    assert!(stopped.stopped);
    assert_eq!((stopped.notices, stopped.tenders), (0, 0));

    // A stop DURING pass 2 must not advance the adjacency watermark. The delta
    // is ISL3 plus a late attach to bt04-0001, so pass 1 scans 2 changed
    // notices while pass 2 scans those PLUS the touched Tender's existing
    // notices — pass 2 is the pass whose Planning total exceeds 2, and that is
    // how the trip detects it. chunk_size = 1 gives pass 2 several chunks, so
    // the flag set during its first chunk's event is honoured at the second
    // chunk's stop check — a genuine mid-pass-2 stop, after plan rows were
    // written and before the build completed.
    record(&db, fid, "A-late", keyed("bt04-0001", 4, "Alpha late attach")).await;
    let before = db.legacy_adjacency_watermark().await.expect("watermark");
    let flag = std::sync::atomic::AtomicBool::new(false);
    let stop_flag = &flag;
    db.set_foreign_keys(false).await.expect("fk off");
    let report = project::project_incremental_chunked_observed(
        &db,
        1,
        None,
        |p| {
            if let Progress::Planning { total, .. } = p {
                if total > 2 {
                    stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
        },
        &|| flag.load(std::sync::atomic::Ordering::SeqCst),
    )
    .await
    .expect("pass-2 stop");
    db.set_foreign_keys(true).await.expect("fk on");
    assert!(report.stopped, "the flag tripped mid-build must stop the run");
    assert_eq!(
        db.legacy_adjacency_watermark().await.expect("watermark"),
        before,
        "a stopped plan build must NOT advance the adjacency attestation"
    );

    // And the delta is still whole: an unstopped run now converges normally.
    let done = project::project_incremental(&db).await.expect("resume");
    assert!(!done.stopped);
    assert!(done.notices > 0, "the stopped delta re-enters whole");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}


/// Issue 262's last corner: when the incremental routes to the FULL fallback
/// (untrusted or over-cap legacy closure — r208's real delta pulled a 2.9M
/// closure, so this is the common path for the biggest folds), the caller's
/// progress sink must ride along. Before this, the fallback swapped in a
/// stderr-only sink and the phase record went dark for the entire ~3h walk.
#[tokio::test]
async fn the_full_fallback_still_surfaces_the_callers_progress() {
    use ingest::project::Progress;
    use std::sync::Mutex;

    // A legacy notice on a db whose adjacency watermark was NEVER established:
    // the closure cannot be trusted, so the incremental MUST take the fallback.
    let (db, fid, path) = scratch("fallbackobs").await;
    let legacy = Parsed {
        sections: vec![sec("PROC", "Notice", None)],
        values: vec![
            text_val("PROC", "TED-TITLE", "Legacy works"),
            date_val("PROC", "TED-DS_DATE_DISPATCH", 42),
        ],
    };
    record_p(&db, fid, "100001-2019", "ted-export-r209", legacy).await;

    let events: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let report = project::project_incremental_observed_stoppable(
        &db,
        |p| {
            events.lock().unwrap().push(match p {
                Progress::Planning { .. } => "planning",
                Progress::Identity { .. } => "identity",
                Progress::Grouped { .. } => "grouped",
                Progress::Applying { .. } => "applying",
                Progress::PrePass { .. } => "pre-pass",
            });
        },
        &|| false,
    )
    .await
    .expect("fallback run");
    assert!(!report.stopped);
    assert!(report.tenders > 0, "the fallback folded the corpus");
    let seen = events.into_inner().unwrap();
    assert!(
        seen.contains(&"planning") && seen.contains(&"applying"),
        "the fallback must surface the caller's sink, not only stderr: {seen:?}"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 283: a version whose ONLY delta is lots-group composition must still emit
/// a `tender changed` event. `fold` carries `group_members` forward with per-group
/// supersession and writes the new membership rows, but `append_version_changes`
/// used to diff only `facts` and `rounds` — so a composition-only version wrote
/// membership rows yet appended zero `changes` rows, and a `/v1/changes` or SSE
/// subscriber never learned which lots a group (and thus a group-scoped bid) now
/// covers.
#[tokio::test]
async fn a_group_composition_change_emits_a_tender_changed_event() {
    let (db, fetch_id, path) = scratch("group-change-event").await;

    // One Tender, three versions of it. Everything is byte-identical between
    // versions EXCEPT the composition of GLO-1: v1 = {LOT-1, LOT-2}, v2 moves it to
    // {LOT-1, LOT-3}, v3 republishes v2's composition unchanged.
    record(&db, fetch_id, "G1", grouped("bt04-grp", 1, &["LOT-1", "LOT-2"])).await;
    record(&db, fetch_id, "G2", grouped("bt04-grp", 2, &["LOT-1", "LOT-3"])).await;
    record(&db, fetch_id, "G3", grouped("bt04-grp", 3, &["LOT-1", "LOT-3"])).await;
    project::project(&db, false).await.expect("project");

    assert_eq!(count(&db, "SELECT COUNT(*) FROM tenders").await, 1, "one grouped Tender");
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM tender_versions").await,
        3,
        "three versions"
    );

    // Sanity: the delta the change feed SHOULD reflect is real — both compositions
    // materialised, so GLO-1 has covered LOT-2 (v1) and LOT-3 (v2/v3) across seqs.
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(DISTINCT m.lot_key) FROM tender_version_lot_group_members x
               JOIN lots g ON g.id = x.group_lot_id
               JOIN lots m ON m.id = x.member_lot_id
              WHERE g.lot_key = 'GLO-1' AND m.lot_key IN ('LOT-2','LOT-3')"
        )
        .await,
        2,
        "GLO-1's composition genuinely moved from LOT-2 to LOT-3 across versions"
    );

    // The Tender is `added` once (v1) and `changed` exactly once — the composition
    // move at v2. Before the fix this count was 0 (the bug). v3 republishes the same
    // composition, so it must NOT add a second `changed` (no false positive).
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='added'").await,
        1,
        "the Tender is added once"
    );
    // The Tender is `added` once (v1) and `changed` exactly once — the composition
    // move at v2. Before the fix this `changed` count was 0 (the bug). v3 republishes
    // v2's composition unchanged, so it must add no second `changed` (no false
    // positive on a byte-identical re-fold).
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='added'").await,
        1,
        "the Tender is added once"
    );
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='changed'").await,
        1,
        "a composition-only change is visible on the feed (issue 283), and an \
         identical re-publish adds no spurious change"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 279: a partial chain rewrite (`keep > 0`) drops the tail's versions but
/// the entity-table rows the departed notice introduced are not swept and no
/// `removed` event is emitted — the `keep == 0` full-rewrite sweep (issue 103)
/// does not cover the kept-prefix case. Reproduced with a lot: notice B (a later
/// notice under the same key) introduces LOT-2; B is then reparsed to a different
/// key and regroups away, leaving T's chain `[A]` (keep=1). LOT-2 in `lots` is now
/// referenced by no surviving `tender_version_lots` row — an orphan the sweep must
/// remove and announce.
#[tokio::test]
async fn a_partial_rewrite_sweeps_the_dropped_tails_orphaned_lot() {
    let (db, fetch_id, path) = scratch("partial-tail").await;

    // A: CN under bt04-tail with LOT-1. B: a later notice under the SAME key adding
    // LOT-2 (so LOT-2 exists only because of B).
    record(&db, fetch_id, "T-cn", keyed("bt04-tail", 1, "Tail CN")).await;
    let mut b = keyed("bt04-tail", 2, "Tail follow-up");
    b.sections.push(sec("LOT-2", "Lot", Some("PROC")));
    b.values.push(date_val("LOT-2", "BT-131(d)-Lot", 700_000_222));
    record(&db, fetch_id, "T-b", b).await;
    project::project(&db, false).await.expect("initial projection");

    let tid = "(SELECT id FROM tenders WHERE procedure_key='bt04-tail')";
    assert_eq!(
        count(&db, &format!("SELECT COUNT(*) FROM tender_versions WHERE tender_id={tid}")).await,
        2,
        "two versions before the regroup",
    );
    assert_eq!(
        count(&db, &format!("SELECT COUNT(*) FROM lots WHERE tender_id={tid} AND lot_key='LOT-2'")).await,
        1,
        "B introduced LOT-2",
    );

    // Reparse B to a DIFFERENT key: it regroups away, so T's chain shrinks to [A].
    reparse(&db, fetch_id, "T-b", keyed("bt04-moved", 2, "Tail follow-up")).await;
    project::project_incremental(&db).await.expect("incremental reproject");

    // T is down to its single CN version — the tail dropped (keep=1).
    assert_eq!(
        count(&db, &format!("SELECT COUNT(*) FROM tender_versions WHERE tender_id={tid}")).await,
        1,
        "the tail version is gone",
    );

    // THE BUG (issue 279): LOT-2 must not survive as an orphan — no surviving
    // tender_version_lots row references it, and a `removed` event must be emitted.
    assert_eq!(
        count(
            &db,
            &format!(
                "SELECT COUNT(*) FROM lots l WHERE l.tender_id={tid} AND l.lot_key='LOT-2' \
                   AND NOT EXISTS (SELECT 1 FROM tender_version_lots tvl \
                                    WHERE tvl.tender_id=l.tender_id AND tvl.lot_id=l.id)"
            ),
        )
        .await,
        0,
        "the dropped tail's orphaned LOT-2 was swept (issue 279)",
    );
    assert!(
        count(&db, "SELECT COUNT(*) FROM changes WHERE entity_kind='lot' AND op='removed'").await >= 1,
        "a removed event announced the swept lot",
    );

    let _ = std::fs::remove_file(&path);
}

/// Issue 292: legacy notices carry two-letter lang tags (`EN`, `DE` — the raw
/// r208/r209 `LG` attribute), but every "English wins" pick in the read layer
/// compares the literal `'ENG'` — so before fold-time normalization the English
/// preference never fired for the pre-eForms corpus and the title fell to scan
/// order. `DE` sorts before `EN` in the facts BTreeSet, so this fixture's
/// first-seen title is the German one: without normalization `current_title`
/// reads "Dacharbeiten"; with it, the tags land as `DEU`/`ENG` and the English
/// pick fires.
#[tokio::test]
async fn legacy_two_letter_lang_tags_normalize_so_the_english_pick_fires() {
    let (db, fetch_id, path) = scratch("lang-norm").await;

    let mut parsed = Parsed {
        sections: vec![sec("PROC", "Procedure", None)],
        values: vec![
            id_val("PROC", "BT-04-notice", "bt04-lang"),
            date_val("PROC", "BT-05(a)-notice", 1),
        ],
    };
    for (i, (lang, title)) in [("DE", "Dacharbeiten"), ("EN", "Roof works")].iter().enumerate() {
        parsed.values.push(ValueRow {
            section_id: "PROC".into(),
            field_id: "BT-21-Procedure".into(),
            ordinal: i as i64,
            value: NoticeValue::Text { lang: Some((*lang).into()), value: (*title).into() },
        });
    }
    record(&db, fetch_id, "LANG", parsed).await;
    project::project(&db, false).await.expect("project");

    // The stored tags are the canonical three-letter vocabulary…
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM tender_version_texts WHERE field='title' AND lang IN ('DEU','ENG')"
        )
        .await,
        2,
        "both language variants stored under normalized tags"
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM tender_version_texts WHERE lang IN ('DE','EN')"
        )
        .await,
        0,
        "no raw two-letter tag survives the fold"
    );
    // …so the English preference actually fires (the bug: German, by scan order).
    let title = match db
        .scalar("SELECT current_title FROM tenders WHERE procedure_key='bt04-lang'")
        .await
        .expect("title")
    {
        Some(store::turso::Value::Text(s)) => s,
        other => panic!("no title: {other:?}"),
    };
    assert_eq!(title, "Roof works", "the English variant wins the current_title pick (issue 292)");

    let _ = std::fs::remove_file(&path);
}

/// ADR-0014: the fold derives `eur_cents` beside every published amount from the
/// run-start rates snapshot — at the version's publication date, NULL when no
/// rate resolves (honest absence, D4). The published cents/currency are
/// untouched either way (D1).
#[tokio::test]
async fn the_fold_derives_eur_cents_beside_published_amounts() {
    let (db, fetch_id, path) = scratch("eur-cents").await;
    // The fixture versions' publication instant is epoch ~0 → 1970-01-01; seed a
    // rate there (2 USD per EUR) so the derivation has something to resolve.
    db.upsert_currency_rates(&[("USD".into(), "1970-01-01".into(), 2.0, "ecb".into())])
        .await
        .expect("seed rate");

    let mut parsed = Parsed {
        sections: vec![sec("PROC", "Procedure", None)],
        values: vec![
            id_val("PROC", "BT-04-notice", "bt04-eur"),
            date_val("PROC", "BT-05(a)-notice", 1),
        ],
    };
    for (i, (cents, currency)) in [(1000, "USD"), (777, "XXX")].iter().enumerate() {
        parsed.values.push(ValueRow {
            section_id: "PROC".into(),
            field_id: "BT-27-Procedure".into(),
            ordinal: i as i64,
            value: NoticeValue::Amount { cents: *cents, currency: (*currency).into() },
        });
    }
    record(&db, fetch_id, "EUR1", parsed).await;
    project::project(&db, false).await.expect("project");

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM tender_version_amounts
              WHERE currency='USD' AND cents=1000 AND eur_cents=500"
        )
        .await,
        1,
        "1000 USD cents at 2 USD/EUR derive 500 eur_cents; published value untouched"
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM tender_version_amounts
              WHERE currency='XXX' AND cents=777 AND eur_cents IS NULL"
        )
        .await,
        1,
        "an unresolvable currency stays honestly NULL (D4)"
    );
    // The head column (D5): MAX over the head version's derived amounts — the
    // convertible 500 wins, the unconvertible 777 contributes nothing.
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM tenders WHERE current_value_eur_cents = 500").await,
        1,
        "the fold stamps the head's MAX derived-EUR value beside the other head pointers"
    );

    let _ = std::fs::remove_file(&path);
}

// ---------------------------------- issue 481: the Tender-link ledger, incrementally

/// The notice id a TED twin publishes as `BT-701-notice` and DÖE as `<id>-<version>`.
const LOGICAL: &str = "5c1f2e7a-9b3d-4e8f-a1c2-7d6e5f4a3b2c";
const LOGICAL_ISLANDS: &str = "0e9d8c7b-6a5f-4e3d-b2c1-a0f9e8d7c6b5";
const LOGICAL_UNHELD: &str = "7a8b9c0d-1e2f-4a3b-9c4d-5e6f7a8b9c0d";
const LOGICAL_KEYED: &str = "8b9c0d1e-2f3a-4b4c-8d5e-6f7a8b9c0d1e";
/// Procedure keys (BT-04).
const KEY: &str = "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d";
const KEY_CN: &str = "9f8e7d6c-5b4a-4392-8170-6f5e4d3c2b1a";
const KEY_SHARED: &str = "be2f3a4b-5c6d-4e7f-9a8b-9c0d1e2f3a4b";

/// A synthetic eForms notice of `source`, dispatched and published on `day`, with these
/// id fields on its procedure root — the Tender-link tests' notice (tests/project.rs
/// builds the same one).
async fn record_linked(db: &Db, fetch_id: i64, source: &str, pub_id: &str, day: i64, ids: &[(&str, &str)]) {
    record_linked_buyers(db, fetch_id, source, pub_id, day, ids, &[]).await;
}

/// A buyer of a Tender-link test notice (issue 481 unit 2b): its BT-500 name, its BT-514
/// country (alpha-3, as eForms writes it) and its BT-501 identifier (`""` for none).
type Buyer<'a> = (&'a str, &'a str, &'a str);

/// The buyers of issue 481 unit 2b's placeholder sample (job 1882): the real owner of the
/// cited number, and a municipality that copied it into its OPP-090.
const SCB: Buyer = ("Statistiska centralbyrån", "SWE", "2021000837");
const ALVKARLEBY: Buyer = ("Älvkarleby kommun", "SWE", "2120000258");

/// The Tender-link tests' notice as `record_linked` builds it, naming `buyers` as its
/// buyers: one Organization section each, referenced from the procedure root by
/// `OPT-300-Procedure-Buyer`.
fn linked_parse(day: i64, ids: &[(&str, &str)], buyers: &[Buyer]) -> Parsed {
    let mut values = vec![date_val("PROCEDURE", "BT-05(a)-notice", day * 86_400)];
    for (field, value) in ids {
        let ordinal = values.iter().filter(|v| v.field_id == *field).count() as i64;
        values.push(ValueRow {
            section_id: "PROCEDURE".into(),
            field_id: (*field).into(),
            ordinal,
            value: NoticeValue::Id { scheme: None, value: (*value).into(), is_ref: false },
        });
    }
    let mut sections = vec![sec("PROCEDURE", "Notice", None)];
    for (i, (name, country, id)) in buyers.iter().enumerate() {
        let org = format!("ORG-{}", i + 1);
        sections.push(sec(&org, "Organization", None));
        let mut field = |field_id: &str, value: NoticeValue| {
            values.push(ValueRow { section_id: org.clone(), field_id: field_id.into(), ordinal: 0, value });
        };
        field("BT-500-Organization-Company", NoticeValue::Text { value: (*name).into(), lang: None });
        if !country.is_empty() {
            field("BT-514-Organization-Company", NoticeValue::Code { list: None, code: (*country).into() });
        }
        if !id.is_empty() {
            field("BT-501-Organization-Company", NoticeValue::Id { scheme: None, value: (*id).into(), is_ref: false });
        }
        values.push(ValueRow {
            section_id: "PROCEDURE".into(),
            field_id: "OPT-300-Procedure-Buyer".into(),
            ordinal: i as i64,
            value: NoticeValue::Id { scheme: None, value: org, is_ref: true },
        });
    }
    Parsed { sections, values }
}

/// The Tender-link tests' notice as `record_linked` records it.
fn linked_notice(fetch_id: i64, source: &str, pub_id: &str, day: i64) -> Notice {
    Notice {
        source: source.into(),
        publication_id: pub_id.into(),
        content_hash: pub_id.into(),
        profile: if source == "ted" { "eforms:eforms-sdk-1.13" } else { "eforms:eforms-de-2.0" }.into(),
        declared_version: None,
        fetch_id,
        member_path: pub_id.into(),
        ingested_at: 0,
        published_at: Some(store::Stamp::utc(day * 86_400)),
        dispatched_at: Some(store::Stamp::utc(day * 86_400)),
    }
}

async fn record_linked_buyers(
    db: &Db,
    fetch_id: i64,
    source: &str,
    pub_id: &str,
    day: i64,
    ids: &[(&str, &str)],
    buyers: &[Buyer<'_>],
) {
    db.record_notice(&linked_notice(fetch_id, source, pub_id, day), &Parse::Parsed(linked_parse(day, ids, buyers)))
        .await
        .expect("record linked notice");
}

/// The linked corpus, notice by notice: `(source, publication id, day, id fields)`.
/// - a TED contract notice and a DÖE notice citing it by OPP-090 (cross-source);
/// - a keyed TED notice whose BT-701 names a DÖE notice published in two versions;
/// - a TED island whose BT-701 names a DÖE island (island ↔ island, the prod shape).
fn linked_corpus() -> Vec<(&'static str, String, i64, Vec<(&'static str, String)>)> {
    vec![
        ("ted", "00200002-2024".into(), 19_990, vec![("BT-04-notice", KEY_CN.into())]),
        ("doe", format!("{LOGICAL_UNHELD}-01"), 20_000, vec![("OPP-090-Procedure", "200002-2024".into())]),
        ("ted", "00400001-2024".into(), 20_003, vec![("BT-04-notice", KEY.into()), ("BT-701-notice", LOGICAL.into())]),
        ("doe", format!("{LOGICAL}-01"), 20_000, vec![]),
        ("doe", format!("{LOGICAL}-02"), 20_005, vec![]),
        ("ted", "00400003-2024".into(), 20_004, vec![("BT-701-notice", LOGICAL_ISLANDS.into())]),
        ("doe", format!("{LOGICAL_ISLANDS}-01"), 20_001, vec![]),
    ]
}

async fn record_corpus_member(db: &Db, fetch_id: i64, member: &(&str, String, i64, Vec<(&str, String)>)) {
    let ids: Vec<(&str, &str)> = member.3.iter().map(|(f, v)| (*f, v.as_str())).collect();
    record_linked(db, fetch_id, member.0, &member.1, member.2, &ids).await;
}

/// The Tender holding the notice published as `pub_id`.
async fn tender_of(db: &Db, pub_id: &str) -> i64 {
    count(db, &format!("SELECT tender_id FROM tender_versions WHERE publication_id = '{pub_id}'")).await
}

/// Notices folded into two Tenders at once — the issue-278 ghost signature.
async fn ghosts(db: &Db) -> i64 {
    count(
        db,
        "SELECT COUNT(*) FROM (SELECT caused_by_notice_id FROM tender_versions
           GROUP BY caused_by_notice_id HAVING COUNT(*) > 1)",
    )
    .await
}

/// Absorb the same delta both ways and require the same canonical layer — content
/// byte-identical, surrogate ids included, and the same change events (as a set: the
/// two paths retire before and after applying).
async fn absorb_and_compare(full: &Db, incr: &Db, label: &str) -> project::Report {
    project::project(full, false).await.expect("full absorb");
    let report = project::project_incremental(incr).await.expect("incremental absorb");
    assert_eq!(snapshot_content(full).await, snapshot_content(incr).await, "{label}: canonical content differs");
    assert_eq!(changes_set(full).await, changes_set(incr).await, "{label}: change events differ");
    assert_eq!(ghosts(incr).await, 0, "{label}: a notice in two Tenders");
    assert!(incr.unprojected_parsed_notice_ids().await.unwrap().is_empty(), "{label}: change-set not drained");
    report
}

/// Issue 481: the ledger takes effect through the DAILY fold, in either arrival order.
/// Each delta is absorbed by a full non-rebuild projection on one DB and incrementally
/// on the other, and the two canonical layers must match after every step:
///
/// - TED first: the TED notices land, then their DÖE twins and the DÖE citer — the twin
///   finds the TED notice's unresolved `logical-notice` row by name, the citer resolves
///   its TED predecessor forward — then a later DÖE version of the same notice;
/// - DÖE first: the DÖE notices land as islands, then the TED notices — whose own
///   `logical-notice` links resolve forward, and whose CN the DÖE citer's unresolved
///   `opp-090` row names.
///
/// Issue 481 unit 2b's buyer guard rides along: TED `00123456-2024` belongs to SCB, a
/// municipality's notice copies that number into its OPP-090 (refused, `buyer_disjoint`)
/// and a later SCB notice naming SCB by name only cites it too (joined). In TED-first
/// order the copier finds its target forward and the SCB citer's fold reaches the copier
/// back through the ledger; DÖE-first, the copier lands before its target and keeps an
/// unresolved row the target's arrival finds by name. The refusal is a verdict on two
/// notices, so it never waits and is the same on both paths. So is the admission of the
/// unit 2b review's shape: an AP-HP award citing its contract notice under another SIRET
/// of the same SIREN and another spelling of its name joins on both paths — and of unit
/// 2c's: a school-purchasing agency's notice `namens` a school citing the school's own
/// (job 1893's ws18), which overlaps through the principal's name.
///
/// Before the closure an incremental plan held only the touched notices, so every one
/// of these joins waited for a full re-projection.
#[tokio::test]
async fn the_ledger_joins_incrementally_exactly_as_a_full_fold_in_either_order() {
    const KEY_SCB: &str = "4d5e6f7a-8b9c-4d0e-9f1a-2b3c4d5e6f7a";
    const KEY_COPIER: &str = "5e6f7a8b-9c0d-4e1f-8a2b-3c4d5e6f7a8b";
    const KEY_SCB_LATER: &str = "6f7a8b9c-0d1e-4f2a-9b3c-4d5e6f7a8b9c";
    const KEY_APHP_CN: &str = "7a8b9c0d-1e2f-4a3b-8c4d-5e6f7a8b9c0d";
    const KEY_APHP_CAN: &str = "8b9c0d1e-2f3a-4b4c-9d5e-6f7a8b9c0d1e";
    const KEY_PRISMA: &str = "9c0d1e2f-3a4b-4c5d-8e6f-7a8b9c0d1e2f";
    const KEY_AGENCY: &str = "0d1e2f3a-4b5c-4d6e-9f7a-8b9c0d1e2f3a";
    let mut corpus: Vec<((&str, String, i64, Vec<(&str, String)>), Vec<Buyer>)> =
        linked_corpus().into_iter().map(|member| (member, Vec::new())).collect();
    corpus.extend([
        (("ted", "00123456-2024".into(), 19_980, vec![("BT-04-notice", KEY_SCB.into())]), vec![SCB]),
        (
            ("ted", "00500001-2024".into(), 20_002, vec![("BT-04-notice", KEY_COPIER.into()), ("OPP-090-Procedure", "123456-2024".into())]),
            vec![ALVKARLEBY],
        ),
        (
            ("ted", "00500002-2024".into(), 20_006, vec![("BT-04-notice", KEY_SCB_LATER.into()), ("OPP-090-Procedure", "123456-2024".into())]),
            vec![("STATISTISKA CENTRALBYRÅN", "SWE", "")],
        ),
        (("ted", "00700001-2024".into(), 19_985, vec![("BT-04-notice", KEY_APHP_CN.into())]), vec![("AP-HP — AGEPS (achats)", "FRA", "26750045200672")]),
        (
            ("ted", "00700002-2024".into(), 20_007, vec![("BT-04-notice", KEY_APHP_CAN.into()), ("OPP-090-Procedure", "700001-2024".into())]),
            vec![("ASSISTANCE PUBLIQUE HOPITAUX DE PARIS", "FRA", "26750045201928")],
        ),
        (("ted", "00581933-2024".into(), 19_988, vec![("BT-04-notice", KEY_PRISMA.into())]), vec![("Stichting Prisma", "NLD", "937097089")]),
        (
            ("ted", "00519549-2025".into(), 20_008, vec![("BT-04-notice", KEY_AGENCY.into()), ("OPP-090-Procedure", "581933-2024".into())]),
            vec![("Onderwijs Inkoop Groep B.V. namens Stichting Prisma", "NLD", "933220822")],
        ),
    ]);
    let record = async |db: &Db, fetch: i64, ((source, pub_id, day, ids), buyers): &((&str, String, i64, Vec<(&str, String)>), Vec<Buyer>)| {
        let ids: Vec<(&str, &str)> = ids.iter().map(|(f, v)| (*f, v.as_str())).collect();
        record_linked_buyers(db, fetch, source, pub_id, *day, &ids, buyers).await;
    };
    let ted_first: [&[usize]; 3] = [&[0, 2, 5, 7, 10, 12], &[1, 3, 6, 8, 11, 13], &[4, 9]];
    let doe_first: [&[usize]; 2] = [&[1, 3, 4, 6, 8, 11, 13], &[0, 2, 5, 7, 9, 10, 12]];
    for (order, deltas) in [("ted-first", &ted_first[..]), ("doe-first", &doe_first[..])] {
        let (full, ff, pf) = scratch(&format!("links-{order}-full")).await;
        let (incr, fi, pi) = scratch(&format!("links-{order}-incr")).await;
        establish(&full, ff).await;
        establish(&incr, fi).await;
        let mut last = None;
        for (step, delta) in deltas.iter().enumerate() {
            for &i in *delta {
                record(&full, ff, &corpus[i]).await;
                record(&incr, fi, &corpus[i]).await;
            }
            let report = absorb_and_compare(&full, &incr, &format!("{order} step {step}")).await;
            assert_eq!(report.links.deferred, 0, "{order} step {step}: the closure reached every link");
            last = Some(report);
        }
        let last = last.expect("a step");
        assert_eq!(last.links.buyer_disjoint, 1, "{order}: the copier's reference, refused: {:?}", last.links);

        let cn = tender_of(&incr, "00200002-2024").await;
        assert_eq!(tender_of(&incr, &format!("{LOGICAL_UNHELD}-01")).await, cn, "{order}: the DÖE citer joins its TED CN");
        let keyed = tender_of(&incr, "00400001-2024").await;
        for version in ["01", "02"] {
            assert_eq!(tender_of(&incr, &format!("{LOGICAL}-{version}")).await, keyed, "{order}: DÖE -{version} joins");
        }
        assert_eq!(
            tender_of(&incr, &format!("{LOGICAL_ISLANDS}-01")).await,
            tender_of(&incr, "00400003-2024").await,
            "{order}: the DÖE island joins its TED island twin"
        );
        let scb = tender_of(&incr, "00123456-2024").await;
        assert_ne!(tender_of(&incr, "00500001-2024").await, scb, "{order}: the copier stays out of SCB's Tender");
        assert_eq!(tender_of(&incr, "00500002-2024").await, scb, "{order}: SCB's own later notice joins it");
        assert_eq!(
            tender_of(&incr, "00700002-2024").await,
            tender_of(&incr, "00700001-2024").await,
            "{order}: AP-HP's award joins its contract notice across two SIRETs of one SIREN"
        );
        assert_eq!(
            tender_of(&incr, "00519549-2025").await,
            tender_of(&incr, "00581933-2024").await,
            "{order}: the agency's notice for Stichting Prisma joins the school's own"
        );
        // The established corpus's three Tenders, the three linked ones, SCB's, the
        // copier's, AP-HP's and Stichting Prisma's.
        assert_eq!(count(&incr, "SELECT COUNT(*) FROM tenders").await, 3 + 7, "{order}");

        for p in [pf, pi] {
            for s in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{p}{s}"));
            }
        }
    }
}

/// Issue 481: a procedure key a previous-notice link folded into ANOTHER key's Tender
/// still finds that Tender on the daily. The award's key is absorbed — the merged Tender
/// is named after the contract notice's key — so a later notice under the award's key
/// that cites nothing used to find no Tender by `procedure_key` and fold into a Tender
/// of its own, where a full fold puts it in the merged one (and the next full
/// re-projection retired the split again). Once TED citing TED, once DÖE citing TED,
/// the cross-source joins the daily now makes.
#[tokio::test]
async fn a_notice_under_an_absorbed_key_finds_the_merged_tender_on_the_daily() {
    const KEY_DOE: &str = "3c4d5e6f-7a8b-4c9d-8e1f-2a3b4c5d6e7f";
    let corpus: [(&str, String, i64, Vec<(&str, String)>); 5] = [
        ("ted", "00200001-2024".into(), 20_000, vec![("BT-04-notice", KEY_CN.into())]),
        ("ted", "00200002-2024".into(), 20_010, vec![("BT-04-notice", KEY.into()), ("OPP-090-Procedure", "200001-2024".into())]),
        ("doe", format!("{LOGICAL_UNHELD}-01"), 20_012, vec![("BT-04-notice", KEY_DOE.into()), ("OPP-090-Procedure", "200001-2024".into())]),
        ("ted", "00200003-2024".into(), 20_020, vec![("BT-04-notice", KEY.into())]),
        ("doe", format!("{LOGICAL_UNHELD}-02"), 20_022, vec![("BT-04-notice", KEY_DOE.into())]),
    ];
    let (full, ff, pf) = scratch("absorbed-full").await;
    let (incr, fi, pi) = scratch("absorbed-incr").await;
    establish(&full, ff).await;
    establish(&incr, fi).await;
    for (step, delta) in [&[0usize][..], &[1, 2], &[3, 4]].into_iter().enumerate() {
        for &i in delta {
            record_corpus_member(&full, ff, &corpus[i]).await;
            record_corpus_member(&incr, fi, &corpus[i]).await;
        }
        absorb_and_compare(&full, &incr, &format!("step {step}")).await;
    }
    let merged = tender_of(&incr, "00200001-2024").await;
    for member in &corpus[1..] {
        assert_eq!(tender_of(&incr, &member.1).await, merged, "{} is in the merged Tender", member.1);
    }
    assert_eq!(
        count(&incr, &format!("SELECT COUNT(*) FROM tenders WHERE id = {merged} AND procedure_key = '{KEY_CN}'")).await,
        1,
        "named by the contract notice's key"
    );

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// The Tender-link tests' notice as `record_linked` builds it, recorded under `parse` —
/// pending or quarantined — instead of parsed.
async fn record_unparsed(db: &Db, fetch_id: i64, source: &str, pub_id: &str, day: i64, parse: Parse) {
    db.record_notice(
        &Notice {
            source: source.into(),
            publication_id: pub_id.into(),
            content_hash: pub_id.into(),
            profile: if source == "ted" { "eforms:eforms-sdk-1.13" } else { "eforms:eforms-de-2.0" }.into(),
            declared_version: None,
            fetch_id,
            member_path: pub_id.into(),
            ingested_at: 0,
            published_at: Some(store::Stamp::utc(day * 86_400)),
            dispatched_at: Some(store::Stamp::utc(day * 86_400)),
        },
        &parse,
    )
    .await
    .expect("record unparsed notice");
}

/// Issue 481: a link to a notice that is not parsed — pending, or quarantined — resolves
/// to nothing, as a full plan (which holds parsed notices only) sees it. The incremental
/// closure used to pull the unparsed notice into its plan, where it folded as an empty
/// island published at 0 and the citer joined it: a Tender no full fold makes. The row
/// stays unresolved until the target parses — and its parse re-queues it, where the
/// closure finds the citer by name and the join lands on the daily.
#[tokio::test]
async fn a_link_to_an_unparsed_notice_joins_only_once_it_parses() {
    for (label, parse) in [
        ("pending", Parse::Pending),
        ("quarantined", Parse::Quarantined { reason: "test".into(), detail: None }),
    ] {
        let (full, ff, pf) = scratch(&format!("unparsed-{label}-full")).await;
        let (incr, fi, pi) = scratch(&format!("unparsed-{label}-incr")).await;
        for (db, fetch) in [(&full, ff), (&incr, fi)] {
            establish(db, fetch).await;
            record_unparsed(db, fetch, "ted", "00500001-2024", 19_990, parse.clone()).await;
            record_linked(db, fetch, "doe", &format!("{LOGICAL_UNHELD}-01"), 20_000, &[("OPP-090-Procedure", "500001-2024")])
                .await;
        }
        absorb_and_compare(&full, &incr, label).await;
        assert_eq!(
            count(&incr, "SELECT COUNT(*) FROM tender_links WHERE b_ref = '00500001-2024' AND b_notice_id IS NULL").await,
            1,
            "{label}: the reference stays unresolved"
        );

        let (notice, parsed) = (
            Notice {
                source: "ted".into(),
                publication_id: "00500001-2024".into(),
                content_hash: "00500001-2024".into(),
                profile: "eforms:eforms-sdk-1.13".into(),
                declared_version: None,
                fetch_id: fi,
                member_path: "00500001-2024".into(),
                ingested_at: 0,
                published_at: Some(store::Stamp::utc(19_990 * 86_400)),
                dispatched_at: Some(store::Stamp::utc(19_990 * 86_400)),
            },
            Parsed {
                sections: vec![sec("PROCEDURE", "Notice", None)],
                values: vec![date_val("PROCEDURE", "BT-05(a)-notice", 19_990 * 86_400)],
            },
        );
        for (db, fetch) in [(&full, ff), (&incr, fi)] {
            let notice = Notice { fetch_id: fetch, ..notice.clone() };
            assert_eq!(db.reparse_notice(&notice, &parsed).await.expect("parse"), store::Reparsed::Replaced);
        }
        absorb_and_compare(&full, &incr, &format!("{label}, parsed")).await;
        assert_eq!(
            tender_of(&incr, "00500001-2024").await,
            tender_of(&incr, &format!("{LOGICAL_UNHELD}-01")).await,
            "{label}: the citer joins the notice once it parses"
        );

        for p in [pf, pi] {
            for s in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{p}{s}"));
            }
        }
    }
}

/// Issue 481: until the ledger is attested complete, the daily holds every weld-guarded
/// join back. Prod's notices were planned before the ledger existed, so a carrier the
/// one-to-one guard must count can have no row for the closure to reach it by — and the
/// guard, seeing one carrier, would admit what a full fold refuses. Here two TED notices
/// carry one BT-701 (a full fold refuses the DÖE twin to both); the ledger is emptied and
/// unattested the way prod is; the twin arrives with one carrier re-queued. The daily
/// holds the join (`deferred`). The wet backfill writes the other carrier's row, attests
/// the ledger, and re-queues every resolved pair still apart — the held one included,
/// whose row the daily's producer had already written — and the next daily refuses, as a
/// full fold does.
#[tokio::test]
async fn the_daily_holds_guarded_joins_until_the_ledger_is_attested() {
    let (db, fetch, path) = scratch("links-unattested").await;
    establish(&db, fetch).await;
    record_linked(&db, fetch, "ted", "00940001-2024", 20_003, &[("BT-701-notice", LOGICAL)]).await;
    record_linked(&db, fetch, "ted", "00940002-2024", 20_003, &[("BT-04-notice", KEY), ("BT-701-notice", LOGICAL)]).await;
    project::project(&db, false).await.expect("project");
    db.execute_for_test("DELETE FROM tender_links").await.expect("planned before the ledger");
    db.execute_for_test("UPDATE projection_state SET tender_links_complete = 0").await.expect("unattested");

    let twin = format!("{LOGICAL}-01");
    record_linked(&db, fetch, "doe", &twin, 20_000, &[]).await;
    db.execute_for_test("UPDATE notices SET projected = 0 WHERE publication_id = '00940001-2024'").await.expect("re-queue");
    let report = project::project_incremental(&db).await.expect("the daily");
    assert_eq!((report.links.logical_notice, report.links.deferred), (0, 1), "{:?}", report.links);
    assert_ne!(tender_of(&db, "00940001-2024").await, tender_of(&db, &twin).await, "held, not joined");

    let never = || false;
    project::backfill_tender_links_windowed(&db, false, 1_000, 1_000, &never, |_| {}).await.expect("wet");
    assert!(db.tender_links_complete().await.unwrap());
    let report = project::project_incremental(&db).await.expect("the next daily");
    assert_eq!((report.links.not_one_to_one, report.links.deferred), (2, 0), "{:?}", report.links);
    for ted in ["00940001-2024", "00940002-2024"] {
        assert_ne!(tender_of(&db, ted).await, tender_of(&db, &twin).await, "{ted} and the twin stay apart");
    }
    let tenders = snapshot_content(&db).await;
    project::project(&db, false).await.expect("a full fold");
    assert_eq!(snapshot_content(&db).await, tenders, "as a full fold leaves them");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 481: a weld-guarded join beside a link the plan holds only one end of waits for
/// the fold that holds both. A TED notice's BT-701 names two DÖE versions, one keyed under
/// another procedure — a keyed weld a full fold refuses. With the far version outside the
/// plan, the guard saw one keyed component and admitted the island version; the next
/// daily refused and split it again, retiring a Tender for nothing. Now the join waits
/// (`deferred`), the far version is re-queued, and the next daily refuses with no Tender
/// moved.
#[tokio::test]
async fn a_guarded_join_beside_a_one_ended_link_waits_for_both_ends() {
    let (db, fetch, path) = scratch("links-one-ended").await;
    establish(&db, fetch).await;
    record_linked(&db, fetch, "ted", "00930001-2024", 20_003, &[("BT-04-notice", KEY), ("BT-701-notice", LOGICAL)]).await;
    record_linked(&db, fetch, "doe", &format!("{LOGICAL}-01"), 20_000, &[]).await;
    record_linked(&db, fetch, "doe", &format!("{LOGICAL}-02"), 20_001, &[("BT-04-notice", KEY_SHARED)]).await;
    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.links.keyed_weld, 2, "{:?}", report.links);
    let removed = "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND op = 'removed'";
    let removed_before = count(&db, removed).await;
    // The closure misses the far version: its row is gone (an incomplete ledger, the
    // attestation left standing so this guard is what is exercised).
    db.execute_for_test("DELETE FROM tender_links").await.expect("rows the closure cannot see");
    record_linked(&db, fetch, "ted", "00930002-2024", 20_010, &[("BT-04-notice", KEY)]).await;
    db.execute_for_test(&format!("UPDATE notices SET projected = 0 WHERE publication_id = '{LOGICAL}-01'"))
        .await
        .expect("re-queue");
    let report = project::project_incremental(&db).await.expect("the daily");
    assert_eq!((report.links.logical_notice, report.links.deferred), (0, 2), "{:?}", report.links);
    assert_ne!(tender_of(&db, "00930001-2024").await, tender_of(&db, &format!("{LOGICAL}-01")).await, "held");
    assert_eq!(db.unprojected_parsed_notice_ids().await.unwrap().len(), 1, "the far version is re-queued");

    let report = project::project_incremental(&db).await.expect("the next daily");
    assert_eq!((report.links.keyed_weld, report.links.deferred), (2, 0), "{:?}", report.links);
    assert_ne!(tender_of(&db, "00930001-2024").await, tender_of(&db, &format!("{LOGICAL}-01")).await);
    assert_eq!(count(&db, removed).await, removed_before, "no Tender retired along the way");
    let tenders = snapshot_content(&db).await;
    project::project(&db, false).await.expect("a full fold");
    assert_eq!(snapshot_content(&db).await, tenders, "as a full fold leaves them");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 481: a ledger write or delete outside a fold re-queues both notices, so a
/// measured match — unit 3's — or its undo reaches the next DAILY fold, and does
/// exactly what a full fold does with it: the match retires the DÖE island into the TED
/// Tender (one `removed` event), the undo splits it out again as a fresh island (no
/// removal, no ghost). Writing a match between notices that already share a Tender
/// re-queues nothing.
#[tokio::test]
async fn a_matched_row_merges_on_the_next_daily_and_its_undo_splits() {
    let (full, ff, pf) = scratch("matched-full").await;
    let (incr, fi, pi) = scratch("matched-incr").await;
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        establish(db, fetch).await;
        record_linked(db, fetch, "doe", "123456-1", 20_000, &[]).await;
        record_linked(db, fetch, "ted", "00700001-2024", 20_002, &[("BT-04-notice", KEY)]).await;
    }
    absorb_and_compare(&full, &incr, "the two notices alone").await;
    assert_ne!(tender_of(&incr, "123456-1").await, tender_of(&incr, "00700001-2024").await);

    let id = async |db: &Db, pub_id: &str| count(db, &format!("SELECT id FROM notices WHERE publication_id = '{pub_id}'")).await;
    let removed = async |db: &Db| count(db, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND op = 'removed'").await;
    let removed_before = removed(&incr).await;
    for db in [&full, &incr] {
        let link = store::MatchedLink {
            a_notice_id: id(db, "00700001-2024").await,
            b_notice_id: id(db, "123456-1").await,
            rule: "r1".into(),
            evidence: Some("{}".into()),
            job_id: Some(7),
        };
        assert_eq!(db.write_matched_links(&[link.clone()]).await.expect("write"), 1);
        assert_eq!(db.write_matched_links(&[link]).await.expect("rewrite"), 0, "idempotent");
    }
    let mut queued = vec![id(&incr, "123456-1").await, id(&incr, "00700001-2024").await];
    queued.sort_unstable();
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap(), queued, "both notices re-queued");
    let report = absorb_and_compare(&full, &incr, "the match").await;
    assert_eq!(report.links.matched, 1, "{:?}", report.links);
    assert_eq!(tender_of(&incr, "123456-1").await, tender_of(&incr, "00700001-2024").await, "merged");
    assert_eq!(removed(&incr).await, removed_before + 1, "the island's Tender retired, announced once");

    // A match between notices that already share a Tender joins nothing new.
    let other = store::MatchedLink {
        a_notice_id: id(&incr, "123456-1").await,
        b_notice_id: id(&incr, "00700001-2024").await,
        rule: "r1".into(),
        evidence: None,
        job_id: None,
    };
    assert_eq!(incr.write_matched_links(&[other]).await.expect("write"), 1);
    assert!(incr.unprojected_parsed_notice_ids().await.unwrap().is_empty(), "nothing to re-fold");
    for db in [&full, &incr] {
        let rows: Vec<i64> = match db.scalar("SELECT group_concat(id) FROM tender_links WHERE kind = 'matched'").await.unwrap() {
            Some(store::turso::Value::Text(s)) => s.split(',').map(|v| v.parse().unwrap()).collect(),
            _ => Vec::new(),
        };
        assert!(!rows.is_empty());
        assert_eq!(db.delete_matched_links(&rows).await.expect("undo"), rows.len() as u64);
    }
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap(), queued, "the undo re-queues both");
    absorb_and_compare(&full, &incr, "the undo").await;
    assert_ne!(tender_of(&incr, "123456-1").await, tender_of(&incr, "00700001-2024").await, "split again");
    assert_eq!(removed(&incr).await, removed_before + 1, "a split retires nothing");
    assert_eq!(
        count(&incr, "SELECT COUNT(*) FROM tenders t JOIN notices n ON n.id = t.island_notice_id
                       WHERE n.publication_id = '123456-1' AND t.source = 'doe'")
        .await,
        1,
        "the DÖE notice is its own island again"
    );

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Issue 481 unit 2b: a buyer-disjoint weld made BEFORE the buyer guard — by ADR-0011's
/// full re-projection of 2026-08-20, or by a daily between unit 2's deploy and the
/// guard's — is counted by the backfill's census and split by the next daily.
///
/// The pre-guard state is built the way it arose: the copier and SCB's notice are folded
/// while their buyers are unknown (the guard never refuses unknown), then re-parsed with
/// their buyers and marked projected, as the pre-guard binary leaves them — one Tender,
/// buyer-disjoint, nothing queued. A pair under ONE BT-04 citing each other with disjoint
/// buyers stands beside it: one group by its key whatever the link says, so neither
/// counted nor split.
///
/// - dry: `would_split` 1 with the pair sampled by both publication ids, two notices
///   that would be re-queued, nothing written or queued;
/// - wet: both re-queued;
/// - the daily splits them exactly as a full fold does (content and change events): SCB's
///   notice keeps the Tender (named by its key), the copier's key — absorbed by the weld
///   — gets a Tender of its own and loses its `tender_key_merges` row, no Tender is
///   retired, no notice sits in two;
/// - a re-run census finds nothing left to split.
///
/// Unit 2c: a legitimate weld stands beside them — Kommunaler Immobilien Service
/// Potsdam's award citing its contract notice under the name without the city (job
/// 1893's ws10, which the 2b guard refused and counted as a split). Its buyers are known
/// from the start: it is joined, neither counted nor re-queued, and the full fold the
/// daily is compared with keeps it whole.
#[tokio::test]
async fn a_pre_guard_buyer_disjoint_weld_is_counted_and_split_by_the_next_daily() {
    const KEY_SCB: &str = "f3a4b5c6-d7e8-4f9a-8b1c-2d3e4f5a6b7c";
    const KEY_COPIER: &str = "a4b5c6d7-e8f9-4a0b-9c2d-3e4f5a6b7c8d";
    const KEY_ONE: &str = "b5c6d7e8-f9a0-4b1c-8d3e-4f5a6b7c8d9e";
    const KEY_KIS_CN: &str = "c6d7e8f9-a0b1-4c2d-9e4f-5a6b7c8d9e0f";
    const KEY_KIS_CAN: &str = "d7e8f9a0-b1c2-4d3e-8f5a-6b7c8d9e0f1a";
    let kis: Buyer = ("Kommunaler Immobilien Service (KIS) - Eigenbetrieb der Landeshauptstadt Potsdam", "DEU", "keine Angabe");
    let kis_potsdam: Buyer =
        ("Kommunaler Immobilien Service Potsdam (KIS) Eigenbetrieb der Landeshauptstadt Potsdam", "DEU", "DE138408386");
    let (full, ff, pf) = scratch("split-full").await;
    let (incr, fi, pi) = scratch("split-incr").await;
    let target_ids = [("BT-04-notice", KEY_SCB)];
    let copier_ids = [("BT-04-notice", KEY_COPIER), ("OPP-090-Procedure", "123456-2024")];
    let one_ids = [("BT-04-notice", KEY_ONE), ("OPP-090-Procedure", "600001-2024")];
    let kis_cn_ids = [("BT-04-notice", KEY_KIS_CN)];
    let kis_can_ids = [("BT-04-notice", KEY_KIS_CAN), ("OPP-090-Procedure", "252055-2024")];
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        establish(db, fetch).await;
        record_linked(db, fetch, "ted", "00123456-2024", 19_980, &target_ids).await;
        record_linked(db, fetch, "ted", "00500001-2024", 20_002, &copier_ids).await;
        record_linked_buyers(db, fetch, "ted", "00252055-2024", 19_982, &kis_cn_ids, &[kis]).await;
        record_linked_buyers(db, fetch, "ted", "00513804-2024", 20_003, &kis_can_ids, &[kis_potsdam]).await;
        record_linked_buyers(db, fetch, "ted", "00600001-2024", 19_990, &[("BT-04-notice", KEY_ONE)], &[SCB]).await;
        record_linked_buyers(db, fetch, "ted", "00600002-2024", 20_004, &one_ids, &[ALVKARLEBY]).await;
    }
    absorb_and_compare(&full, &incr, "folded while the buyers are unknown").await;
    let welded = tender_of(&incr, "00123456-2024").await;
    assert_eq!(tender_of(&incr, "00500001-2024").await, welded, "the pre-guard weld");
    let absorbed = format!("SELECT COUNT(*) FROM tender_key_merges WHERE from_key = '{KEY_COPIER}'");
    assert_eq!(count(&incr, &absorbed).await, 1, "the copier's key absorbed into SCB's Tender");
    let kis_tender = tender_of(&incr, "00252055-2024").await;
    assert_eq!(tender_of(&incr, "00513804-2024").await, kis_tender, "KIS's award joined its contract notice");
    for (db, fetch) in [(&full, ff), (&incr, fi)] {
        for (pub_id, day, ids, buyer) in
            [("00123456-2024", 19_980, &target_ids[..], SCB), ("00500001-2024", 20_002, &copier_ids[..], ALVKARLEBY)]
        {
            let (notice, parsed) = (linked_notice(fetch, "ted", pub_id, day), linked_parse(day, ids, &[buyer]));
            assert_eq!(db.reparse_notice(&notice, &parsed).await.expect("reparse"), store::Reparsed::Replaced);
        }
        db.execute_for_test("UPDATE notices SET projected = 1").await.expect("as the pre-guard binary left them");
    }
    let removed = "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND op = 'removed'";
    let removed_before = count(&incr, removed).await;
    let tenders_before = count(&incr, "SELECT COUNT(*) FROM tenders").await;
    let sample = |s: &store::TenderLinkSample| {
        format!("{} {}:{} {}:{}", s.rule, s.a_source, s.a_publication_id, s.b_source, s.b_publication_id)
    };
    let opp = |r: &store::TenderLinkBackfill| r.rules.iter().find(|(n, _)| n == "opp-090").unwrap().1.clone();

    let never = || false;
    let dry = project::backfill_tender_links_windowed(&incr, true, 1_000, 1_000, &never, |_| {}).await.expect("dry");
    assert_eq!((opp(&dry).would_split, opp(&dry).buyer_disjoint, opp(&dry).would_merge), (1, 0, 0), "{dry:?}");
    assert_eq!((dry.would_split, dry.requeued), (1, 2), "{dry:?}");
    assert_eq!(
        dry.split_samples.iter().map(sample).collect::<Vec<_>>(),
        vec!["opp-090 ted:00500001-2024 ted:00123456-2024".to_owned()]
    );
    assert!(incr.unprojected_parsed_notice_ids().await.unwrap().is_empty(), "a dry run queues nothing");

    let wet = project::backfill_tender_links_windowed(&incr, false, 1_000, 1_000, &never, |_| {}).await.expect("wet");
    assert_eq!((wet.would_split, wet.requeued), (1, 2), "{wet:?}");
    let id = async |pub_id: &str| count(&incr, &format!("SELECT id FROM notices WHERE publication_id = '{pub_id}'")).await;
    let mut pair = vec![id("00123456-2024").await, id("00500001-2024").await];
    pair.sort_unstable();
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap(), pair, "both notices of the pair re-queued");

    let report = absorb_and_compare(&full, &incr, "the split").await;
    assert_eq!(report.links.buyer_disjoint, 1, "{:?}", report.links);
    assert_eq!(tender_of(&incr, "00123456-2024").await, welded, "SCB's notice keeps the Tender");
    assert_ne!(tender_of(&incr, "00500001-2024").await, welded, "the copier is split out");
    assert_eq!(
        count(&incr, &format!("SELECT COUNT(*) FROM tenders WHERE procedure_key = '{KEY_COPIER}'")).await,
        1,
        "under its own key"
    );
    assert_eq!(count(&incr, &absorbed).await, 0, "and the key is no longer absorbed");
    assert_eq!(tender_of(&incr, "00600002-2024").await, tender_of(&incr, "00600001-2024").await, "one BT-04, one Tender");
    assert_eq!(tender_of(&incr, "00513804-2024").await, kis_tender, "KIS's one buyer keeps its Tender");
    assert_eq!(tender_of(&incr, "00252055-2024").await, kis_tender);
    assert_eq!(count(&incr, "SELECT COUNT(*) FROM tenders").await, tenders_before + 1);
    assert_eq!(count(&incr, removed).await, removed_before, "a split retires nothing");

    let again = project::backfill_tender_links_windowed(&incr, true, 1_000, 1_000, &never, |_| {}).await.expect("re-run");
    assert_eq!((again.would_split, again.requeued), (0, 0), "{again:?}");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// Issue 481, the cap: an over-cap link closure reports the full-path fallback instead
/// of a wrong scope; the production cap admits the same component.
#[tokio::test]
async fn an_over_cap_link_closure_reports_fallback() {
    let (db, fetch, path) = scratch("linkcap").await;
    establish(&db, fetch).await;
    record_linked(&db, fetch, "ted", "00400001-2024", 20_003, &[("BT-04-notice", KEY), ("BT-701-notice", LOGICAL)]).await;
    record_linked(&db, fetch, "doe", &format!("{LOGICAL}-01"), 20_000, &[]).await;
    project::project(&db, false).await.expect("project");
    let ted = count(&db, "SELECT id FROM notices WHERE publication_id = '00400001-2024'").await;
    let doe = count(&db, &format!("SELECT id FROM notices WHERE publication_id = '{LOGICAL}-01'")).await;

    let over = project::link_closure_capped(&db, &[ted], &[], 0).await.expect("walk (cap 0)");
    let reason = over.expect_err("the twin is one notice past a cap of 0");
    assert!(reason.contains("exceeds cap"), "the reason names the cap: {reason}");

    let ok = project::link_closure_capped(&db, &[ted], &[], 500_000).await.expect("walk (real cap)");
    let (notices, tenders) = ok.expect("the production cap admits it");
    assert_eq!(notices, vec![doe], "the twin, reached through the ledger");
    assert_eq!(tenders, vec![tender_of(&db, "00400001-2024").await], "and the Tender it folds into");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// The ledger, one row per line in id order — what a zero-drift re-plan must leave alone.
async fn ledger_rows(db: &Db) -> String {
    match db
        .scalar(
            "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||a_notice_id||'|'||coalesce(b_notice_id, -1)||'|'||
                    kind||'|'||rule||'|'||b_source||'|'||b_ref AS r FROM tender_links ORDER BY id)",
        )
        .await
        .expect("ledger rows")
    {
        Some(store::turso::Value::Text(s)) => s,
        _ => String::new(),
    }
}

/// Issue 481: the ledger backfill gives notices planned before the ledger existed their
/// declared rows, and the next DAILY fold makes the joins — no full re-projection.
///
/// The corpus is built the way prod is: projected while the ledger kept nothing, so a
/// TED notice whose DÖE twin arrived later, a DÖE notice citing a TED CN that arrived
/// later, and a TED island with its DÖE island twin all stand apart; a TED/DÖE pair
/// already sharing a BT-04 is one Tender; one TED notice names a DÖE id nobody holds;
/// and (issue 481 unit 2b) one TED notice copies SCB's publication number into its
/// OPP-090 — a would-merge pair the buyer guard refuses, counted `buyer_disjoint` and
/// sampled apart from the joins, never re-queued — and another, published BEFORE SCB's
/// notice, copies it too: job 1882's sampled shape, which the fold refuses by direction
/// first, so it is counted `not_earlier`, neither sampled among the joins nor re-queued
/// (the unit 2b review: it had read as a join). Walked through windows of two notices
/// and three ids:
///
/// - dry: per-rule counts, the would-merge pairs by both publication ids, the re-queue
///   counted — and nothing written or queued;
/// - stopped: the walk ends between windows and says how far it got;
/// - wet: the rows, with ONLY the would-merge pairs' notices re-queued (the keyed pair
///   already shares a Tender), and a re-run finds everything present;
/// - the daily fold joins all three; a full re-plan afterwards rewrites no ledger row
///   and moves no Tender, which is the backfill's zero drift from the producer.
#[tokio::test]
async fn the_ledger_backfill_counts_dry_writes_wet_and_the_daily_joins() {
    const LOGICAL_NEVER: &str = "c0d1e2f3-a4b5-4c6d-9e7f-8a9b0c1d2e3f";
    let (db, fetch, path) = scratch("link-backfill").await;
    establish(&db, fetch).await;
    record_linked(&db, fetch, "ted", "00400001-2024", 20_003, &[("BT-04-notice", KEY), ("BT-701-notice", LOGICAL)]).await;
    record_linked(&db, fetch, "ted", "00400003-2024", 20_004, &[("BT-701-notice", LOGICAL_ISLANDS)]).await;
    record_linked(&db, fetch, "ted", "00400005-2024", 20_004, &[("BT-04-notice", "c1d2e3f4-a5b6-4c7d-8e9f-0a1b2c3d4e5f"), ("BT-701-notice", LOGICAL_NEVER)])
        .await;
    let citer = format!("{LOGICAL_UNHELD}-01");
    record_linked(&db, fetch, "doe", &citer, 20_000, &[("OPP-090-Procedure", "200002-2024")]).await;
    record_linked(&db, fetch, "ted", "00400006-2024", 20_006, &[("BT-04-notice", KEY_SHARED), ("BT-701-notice", LOGICAL_KEYED)])
        .await;
    record_linked(&db, fetch, "doe", &format!("{LOGICAL_KEYED}-01"), 20_006, &[("BT-04-notice", KEY_SHARED)]).await;
    // Issue 481 unit 2b: a municipality copying SCB's publication number into its OPP-090
    // — a would-merge pair (two Tenders today) the buyer guard refuses.
    record_linked_buyers(&db, fetch, "ted", "00123456-2024", 19_980, &[("BT-04-notice", "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a")], &[SCB])
        .await;
    let copier = [("BT-04-notice", "e2f3a4b5-c6d7-4e8f-9a0b-1c2d3e4f5a6b"), ("OPP-090-Procedure", "123456-2024")];
    record_linked_buyers(&db, fetch, "ted", "00400009-2024", 20_007, &copier, &[ALVKARLEBY]).await;
    let early_copier = [("BT-04-notice", "f3a4b5c6-d7e8-4f9a-8b1c-2d3e4f5a6b70"), ("OPP-090-Procedure", "123456-2024")];
    record_linked_buyers(&db, fetch, "ted", "00400010-2024", 19_970, &early_copier, &[ALVKARLEBY]).await;
    project::project(&db, false).await.expect("project");
    db.execute_for_test("DELETE FROM tender_links").await.expect("the binary that planned them kept no ledger");
    db.execute_for_test("UPDATE projection_state SET tender_links_complete = 0").await.expect("nor attested one");
    record_linked(&db, fetch, "doe", &format!("{LOGICAL}-01"), 20_000, &[]).await;
    record_linked(&db, fetch, "doe", &format!("{LOGICAL_ISLANDS}-01"), 20_001, &[]).await;
    record_linked(&db, fetch, "ted", "00200002-2024", 19_990, &[("BT-04-notice", KEY_CN)]).await;
    project::project_incremental(&db).await.expect("the twins arrive with no ledger to find them by");
    let pairs = [
        ("00400001-2024".to_owned(), format!("{LOGICAL}-01")),
        ("00400003-2024".to_owned(), format!("{LOGICAL_ISLANDS}-01")),
        (citer.clone(), "00200002-2024".to_owned()),
    ];
    for (a, b) in &pairs {
        assert_ne!(tender_of(&db, a).await, tender_of(&db, b).await, "{a} and {b} start apart");
    }
    assert_eq!(ledger_rows(&db).await, "", "the ledger starts empty");
    let tenders_before = count(&db, "SELECT COUNT(*) FROM tenders").await;

    let never = || false;
    let dry = project::backfill_tender_links_windowed(&db, true, 3, 2, &never, |_| {}).await.expect("dry");
    let rule = |r: &store::TenderLinkBackfill, name: &str| r.rules.iter().find(|(n, _)| n == name).unwrap().1.clone();
    let logical = store::TenderLinkRuleCounts {
        declared: 4,
        present: 0,
        resolved: 3,
        unresolved: 1,
        would_merge: 2,
        cross_source: 2,
        not_earlier: 0,
        buyer_disjoint: 0,
        would_split: 0,
        stale: 0,
    };
    // The copier's row is a would-merge row the buyer guard refuses: counted, sampled
    // apart, and NOT re-queued — the next fold refuses it, as a full fold does. The early
    // copier's row is refused by direction before any buyer is read: counted only.
    let opp = store::TenderLinkRuleCounts {
        declared: 3,
        present: 0,
        resolved: 3,
        unresolved: 0,
        would_merge: 3,
        cross_source: 1,
        not_earlier: 1,
        buyer_disjoint: 1,
        would_split: 0,
        stale: 0,
    };
    assert_eq!(rule(&dry, "logical-notice"), logical, "{dry:?}");
    assert_eq!(rule(&dry, "opp-090"), opp, "{dry:?}");
    assert_eq!((dry.notices, dry.declaring, dry.would_merge, dry.requeued), (16, 7, 5, 6), "{dry:?}");
    assert_eq!((dry.not_earlier, dry.buyer_disjoint, dry.would_split), (1, 1, 0), "{dry:?}");
    let sample = |s: &store::TenderLinkSample| {
        format!("{} {}:{} {}:{}", s.rule, s.a_source, s.a_publication_id, s.b_source, s.b_publication_id)
    };
    assert_eq!(
        dry.disjoint_samples.iter().map(sample).collect::<Vec<_>>(),
        vec!["opp-090 ted:00400009-2024 ted:00123456-2024".to_owned()]
    );
    assert!(dry.split_samples.is_empty(), "{dry:?}");
    // The joins only: the copier's refused pair is in `disjoint_samples`, and the early
    // copier's is in no list — neither is here.
    let mut samples: Vec<String> = dry.samples.iter().map(sample).collect();
    samples.sort();
    assert_eq!(
        samples,
        vec![
            format!("logical-notice ted:00400001-2024 doe:{LOGICAL}-01"),
            format!("logical-notice ted:00400003-2024 doe:{LOGICAL_ISLANDS}-01"),
            format!("opp-090 doe:{citer} ted:00200002-2024"),
        ]
    );
    assert_eq!(ledger_rows(&db).await, "", "a dry run writes nothing");
    assert!(db.unprojected_parsed_notice_ids().await.unwrap().is_empty(), "and queues nothing");
    assert!(!db.tender_links_complete().await.unwrap(), "nor attests anything");

    let calls = std::sync::atomic::AtomicUsize::new(0);
    let second = || calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 1;
    let stopped = project::backfill_tender_links_windowed(&db, true, 3, 2, &second, |_| {}).await.expect("stopped");
    assert!(stopped.stopped && stopped.cursor < stopped.target, "{stopped:?}");
    let wet_calls = std::sync::atomic::AtomicUsize::new(0);
    let second_wet = || wet_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 1;
    let stopped = project::backfill_tender_links_windowed(&db, false, 3, 2, &second_wet, |_| {}).await.expect("stopped wet");
    assert!(stopped.stopped && !db.tender_links_complete().await.unwrap(), "a stopped walk attests nothing");
    db.execute_for_test("DELETE FROM tender_links").await.expect("undo the stopped window");
    db.execute_for_test("UPDATE notices SET projected = 1").await.expect("and its re-queue");

    let wet = project::backfill_tender_links_windowed(&db, false, 3, 2, &never, |_| {}).await.expect("wet");
    assert_eq!((rule(&wet, "logical-notice"), rule(&wet, "opp-090")), (logical, opp), "{wet:?}");
    assert_eq!(wet.requeued, 6, "{wet:?}");
    assert!(db.tender_links_complete().await.unwrap(), "a finished wet walk attests the ledger complete");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM tender_links WHERE kind = 'declared'").await, 7);
    let id = async |pub_id: &str| count(&db, &format!("SELECT id FROM notices WHERE publication_id = '{pub_id}'")).await;
    let mut queued = Vec::new();
    for (a, b) in &pairs {
        queued.extend([id(a).await, id(b).await]);
    }
    queued.sort_unstable();
    assert_eq!(db.unprojected_parsed_notice_ids().await.unwrap(), queued, "only the would-merge pairs' notices");
    let again = project::backfill_tender_links_windowed(&db, false, 3, 2, &never, |_| {}).await.expect("re-run");
    let present = rule(&again, "logical-notice");
    assert_eq!((present.declared, present.present, present.resolved, again.requeued), (4, 4, 0, 0), "{again:?}");

    let report = project::project_incremental(&db).await.expect("the daily");
    assert_eq!((report.links.logical_notice, report.links.previous_notice, report.links.deferred), (2, 1, 0), "{:?}", report.links);
    for (a, b) in &pairs {
        assert_eq!(tender_of(&db, a).await, tender_of(&db, b).await, "{a} and {b} joined by the daily fold");
    }
    assert_eq!(count(&db, "SELECT COUNT(*) FROM tenders").await, tenders_before - 3);
    for copier in ["00400009-2024", "00400010-2024"] {
        assert_ne!(tender_of(&db, copier).await, tender_of(&db, "00123456-2024").await, "{copier} stays out");
    }
    assert_eq!(ghosts(&db).await, 0);

    let rows = ledger_rows(&db).await;
    let tenders = snapshot_content(&db).await;
    project::project(&db, false).await.expect("a full re-plan");
    assert_eq!(ledger_rows(&db).await, rows, "the producer finds the backfill's rows exactly as it would write them");
    assert_eq!(snapshot_content(&db).await, tenders, "and the full fold moves nothing");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

// ------------------------------ issue 482: the procedure-key census (buyer-disjoint clusters)

/// Issue 482 unit 1: the procedure-key census clusters each UUID-keyed Tender's notices
/// by the link guard's buyer overlap. Four Tenders, each folded under one BT-04 as the
/// fold does today:
///
/// - **Tender 1110706's shape**: a German DÖE/TED pair and a Bulgarian award 15 months
///   later, plus a TED notice naming no buyer. Two clusters, across jurisdictions and
///   Sources, span over a year; the buyerless notice is counted, never a third cluster.
/// - **A joint procurement**: the CN names two buyers, an award names one of them and a
///   third, a later award names only the third. One cluster (transitive overlap).
/// - **A central purchasing body's framework** naming its call-off buyers on the CN, and
///   one call-off award per buyer naming that buyer alone. One cluster.
/// - **No-buyer notices never split**: one buyer, a buyerless notice between its two
///   notices, is one cluster; one buyer beside a buyerless notice is undecidable.
/// - **Interleaved clusters** (the hub/gate shape): one buyer's notices on either side of
///   another same-country buyer's. Gap 0, `interleaved`, one jurisdiction.
/// - **A buyer naming no country** beside a German one: `unknown-jurisdiction`, not one.
/// - **A link-joined split**: a DÖE notice citing a TED CN by OPP-090 (same buyer) joins
///   that CN's Tender, which also holds another buyer under the shared BT-04. The linked
///   cluster says `other_keys` 1, and the Tender is not `split_same_key`.
/// - **A whitespace-padded UUID key** is walked (the SQL pre-filter trims, as `is_uuid`).
///
/// The walk gives the same report in windows of 1 and of 1,000 Tender ids.
#[tokio::test]
async fn the_procedure_key_census_splits_only_buyer_disjoint_clusters() {
    use ingest::project::key_census;
    const KEY_WELD: &str = "f3943baf-54ae-441a-aee9-3e802998024a";
    const KEY_JOINT: &str = "2b3c4d5e-6f7a-4b8c-9d0e-1f2a3b4c5d6e";
    const KEY_CPB: &str = "3c4d5e6f-7a8b-4c9d-8e1f-2a3b4c5d6e7f";
    const KEY_ONE: &str = "4d5e6f7a-8b9c-4d0e-9f2a-3b4c5d6e7f8a";
    const KEY_LONE: &str = "5e6f7a8b-9c0d-4e1f-8a3b-4c5d6e7f8a9b";
    const KEY_HUB: &str = "6f7a8b9c-0d1e-4f2a-9b4c-5d6e7f8a9b0c";
    const KEY_NOCOUNTRY: &str = "7a8b9c0d-1e2f-4a3b-8c5d-6e7f8a9b0c1d";
    const KEY_LINK: &str = "8b9c0d1e-2f3a-4b4c-9d6e-7f8a9b0c1d2e";
    const KEY_PADDED: &str = " 9c0d1e2f-3a4b-4c5d-8e7f-8a9b0c1d2e3f";
    let wesel: Buyer = ("Stadt Wesel", "DEU", "");
    let belogradchik: Buyer = ("Община Белоградчик", "BGR", "000320517");
    let (jointa, jointb, jointc): (Buyer, Buyer, Buyer) = (
        ("Gemeente Utrecht", "NLD", ""),
        ("Gemeente Amersfoort", "NLD", ""),
        ("Provincie Utrecht", "NLD", ""),
    );
    let dataport: Buyer = ("Dataport AöR", "DEU", "");
    let (hamburg, kiel): (Buyer, Buyer) = (("Freie und Hansestadt Hamburg", "DEU", ""), ("Landeshauptstadt Kiel", "DEU", ""));
    let solo: Buyer = ("Landkreis Görlitz", "DEU", "");
    let (lutjenburg, segeberg): (Buyer, Buyer) = (("Amt Lütjenburg", "DEU", ""), ("Stadt Bad Segeberg", "DEU", ""));
    let (potsdam, countryless): (Buyer, Buyer) = (("Landeshauptstadt Potsdam", "DEU", ""), ("Zweckverband Nordwest", "", ""));
    let (bonn, koeln): (Buyer, Buyer) = (("Bundesstadt Bonn", "DEU", ""), ("Stadt Köln", "DEU", ""));
    let (db, fetch, path) = scratch("key-census").await;
    let doe_twin = "0a746ecc-1b2c-4d3e-8f4a-5b6c7d8e9f0a-01";
    record_linked_buyers(&db, fetch, "doe", doe_twin, 19_758, &[("BT-04-notice", KEY_WELD)], &[wesel]).await;
    record_linked_buyers(&db, fetch, "ted", "00081139-2024", 19_760, &[("BT-04-notice", KEY_WELD)], &[wesel]).await;
    record_linked_buyers(&db, fetch, "ted", "00090001-2024", 19_800, &[("BT-04-notice", KEY_WELD)], &[]).await;
    record_linked_buyers(&db, fetch, "ted", "00274114-2025", 20_207, &[("BT-04-notice", KEY_WELD)], &[belogradchik])
        .await;
    record_linked_buyers(&db, fetch, "ted", "00300001-2024", 19_900, &[("BT-04-notice", KEY_JOINT)], &[jointa, jointb])
        .await;
    record_linked_buyers(&db, fetch, "ted", "00300002-2024", 19_950, &[("BT-04-notice", KEY_JOINT)], &[jointb, jointc])
        .await;
    record_linked_buyers(&db, fetch, "ted", "00300003-2024", 19_990, &[("BT-04-notice", KEY_JOINT)], &[jointc]).await;
    record_linked_buyers(&db, fetch, "ted", "00310001-2024", 19_900, &[("BT-04-notice", KEY_CPB)], &[dataport, hamburg, kiel])
        .await;
    record_linked_buyers(&db, fetch, "ted", "00310002-2024", 20_000, &[("BT-04-notice", KEY_CPB)], &[hamburg]).await;
    record_linked_buyers(&db, fetch, "ted", "00310003-2024", 20_010, &[("BT-04-notice", KEY_CPB)], &[kiel]).await;
    record_linked_buyers(&db, fetch, "ted", "00320001-2024", 19_900, &[("BT-04-notice", KEY_ONE)], &[solo]).await;
    record_linked_buyers(&db, fetch, "ted", "00320002-2024", 19_910, &[("BT-04-notice", KEY_ONE)], &[]).await;
    record_linked_buyers(&db, fetch, "ted", "00320003-2024", 19_920, &[("BT-04-notice", KEY_ONE)], &[solo]).await;
    record_linked_buyers(&db, fetch, "ted", "00330001-2024", 19_900, &[("BT-04-notice", KEY_LONE)], &[solo]).await;
    record_linked_buyers(&db, fetch, "ted", "00330002-2024", 19_910, &[("BT-04-notice", KEY_LONE)], &[]).await;
    record_linked_buyers(&db, fetch, "ted", "00340001-2024", 19_900, &[("BT-04-notice", KEY_HUB)], &[lutjenburg]).await;
    record_linked_buyers(&db, fetch, "ted", "00340002-2024", 20_000, &[("BT-04-notice", KEY_HUB)], &[segeberg]).await;
    record_linked_buyers(&db, fetch, "ted", "00340003-2024", 20_100, &[("BT-04-notice", KEY_HUB)], &[lutjenburg]).await;
    record_linked_buyers(&db, fetch, "ted", "00350001-2024", 19_900, &[("BT-04-notice", KEY_NOCOUNTRY)], &[potsdam])
        .await;
    let countryless_doe = "2c3d4e5f-6a7b-4c8d-9e0f-1a2b3c4d5e6f-01";
    record_linked_buyers(&db, fetch, "doe", countryless_doe, 19_950, &[("BT-04-notice", KEY_NOCOUNTRY)], &[countryless])
        .await;
    record_linked_buyers(&db, fetch, "ted", "00360001-2024", 19_900, &[("BT-04-notice", KEY_LINK)], &[bonn]).await;
    record_linked_buyers(&db, fetch, "ted", "00360002-2024", 19_910, &[("BT-04-notice", KEY_LINK)], &[koeln]).await;
    let citer = "1b2c3d4e-5f6a-4b7c-8d9e-0f1a2b3c4d5e-01";
    record_linked_buyers(&db, fetch, "doe", citer, 19_920, &[("OPP-090-Procedure", "360001-2024")], &[bonn]).await;
    record_linked_buyers(&db, fetch, "ted", "00370001-2024", 19_900, &[("BT-04-notice", KEY_PADDED)], &[solo]).await;
    record_linked_buyers(&db, fetch, "ted", "00370002-2024", 19_910, &[("BT-04-notice", KEY_PADDED)], &[solo]).await;
    project::project(&db, false).await.expect("fold");
    assert_eq!(tender_of(&db, citer).await, tender_of(&db, "00360001-2024").await, "the OPP-090 link joins the CN");
    assert_eq!(tender_of(&db, "00360002-2024").await, tender_of(&db, "00360001-2024").await, "and Köln shares its BT-04");
    assert_eq!(tender_of(&db, "00370002-2024").await, tender_of(&db, "00370001-2024").await, "the padded key folds");
    let weld = tender_of(&db, "00081139-2024").await;
    assert_eq!(tender_of(&db, "00274114-2025").await, weld, "today's fold welds the Bulgarian award in");
    assert_eq!(tender_of(&db, doe_twin).await, weld);

    let never = || false;
    let r = key_census::procedure_key_census_windowed(&db, 1_000, &never, |_| {}).await.expect("census");
    assert!(!r.stopped);
    assert_eq!(r.tenders, 9, "nine UUID-keyed Tenders of two or more notices, the padded key's among them");
    assert_eq!(r.notices, 25);
    assert_eq!(r.notices_without_buyers, 3);
    assert_eq!(r.undecidable, 1, "one buyer beside a buyerless notice: nothing to compare");
    assert_eq!(r.split, 4, "the weld, the hub, the countryless pair and the linked Tender: {:?}", r.hubs);
    assert_eq!((r.split_with_buyerless, r.split_notices_without_buyers), (1, 1));
    assert_eq!(r.split_same_key, 3, "the linked Tender holds a notice under another key");
    assert_eq!((r.interleaved, r.sequential, r.singleton_minorities), (2, 2, 4), "the hub and the linked Tender interleave");
    assert_eq!(r.max_clusters, 2);
    let count = |b: &str| r.buckets[b].tenders;
    assert_eq!(
        key_census::KEY_CENSUS_BUCKETS.map(count),
        [2, 1, 1, 3, 1, 3, 0, 1, 2, 1, 1],
        "jurisdictions one/several/unknown, Sources one/several, gap ≤90d/≤1y/>1y, span ≤90d/≤1y/>1y"
    );
    let cross: Vec<(&str, u64)> = r.cross.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    assert_eq!(
        cross,
        vec![
            ("one-jurisdiction/one-source/gap-le-90d", 2),
            ("several-jurisdictions/one-source/gap-gt-1y", 1),
            ("unknown-jurisdiction/several-sources/gap-le-90d", 1),
        ]
    );
    assert_eq!(r.cluster_counts.get("2"), Some(&4));
    let sample = &r.buckets["several-jurisdictions"].samples[0];
    assert_eq!(sample.tender_id, weld);
    assert_eq!(sample.procedure_key, KEY_WELD);
    assert_eq!((sample.notices, sample.span_days, sample.gap_days), (4, 449, 447));
    assert_eq!((sample.clusters_total, sample.clusters.len()), (2, 2));
    let (wesel_c, bg) = (&sample.clusters[0], &sample.clusters[1]);
    assert_eq!(wesel_c.publications, vec![format!("doe:{doe_twin}"), "ted:00081139-2024".to_owned()]);
    assert_eq!(wesel_c.buyer, "Stadt Wesel");
    assert_eq!(wesel_c.jurisdictions, vec!["DE".to_owned()]);
    assert_eq!(wesel_c.sources, vec!["doe".to_owned(), "ted".to_owned()]);
    assert_eq!((wesel_c.first_published.as_str(), wesel_c.last_published.as_str(), wesel_c.gap_days), ("2024-02-05", "2024-02-07", 0));
    assert_eq!(bg.publications, vec!["ted:00274114-2025".to_owned()]);
    assert_eq!(bg.buyer, "Община Белоградчик");
    assert_eq!(bg.jurisdictions, vec!["BG".to_owned()]);
    assert_eq!(bg.sources, vec!["ted".to_owned()], "a TED notice colliding with a DÖE/TED pair: one Source");
    assert_eq!((bg.first_published.as_str(), bg.last_published.as_str(), bg.gap_days), ("2025-04-29", "2025-04-29", 447));
    assert_eq!((wesel_c.other_keys, bg.other_keys), (0, 0));
    assert_eq!((sample.without_buyers_total, sample.without_buyers.clone()), (1, vec!["ted:00090001-2024".to_owned()]));
    assert!(r.hubs.contains(sample));
    let by_key = |key: &str| r.hubs.iter().find(|s| s.procedure_key == key).unwrap_or_else(|| panic!("{key}: {:?}", r.hubs));
    let hub = by_key(KEY_HUB);
    assert_eq!((hub.gap_days, hub.span_days), (0, 200), "Segeberg sits inside Lütjenburg's range");
    assert_eq!(hub.clusters[0].buyer, "Amt Lütjenburg");
    assert!(r.buckets["one-jurisdiction"].samples.contains(hub));
    let nocountry = by_key(KEY_NOCOUNTRY);
    assert_eq!(nocountry.clusters[1].jurisdictions, Vec::<String>::new());
    assert!(r.buckets["unknown-jurisdiction"].samples.contains(nocountry), "a countryless side is unknown, not one");
    assert!(r.buckets["several-sources"].samples.contains(nocountry), "a DÖE-only cluster beside a TED-only one");
    assert_eq!(nocountry.gap_days, 50);
    let linked = by_key(KEY_LINK);
    assert_eq!(linked.clusters[0].buyer, "Bundesstadt Bonn");
    assert_eq!(linked.clusters[0].publications, vec!["ted:00360001-2024".to_owned(), format!("doe:{citer}")]);
    assert_eq!((linked.clusters[0].other_keys, linked.clusters[1].other_keys), (1, 0), "the citer is joined by its link");
    assert!(r.buckets["one-source"].samples.contains(linked), "a TED+DÖE cluster beside a TED-only one shares TED");

    let narrow = key_census::procedure_key_census_windowed(&db, 1, &never, |_| {}).await.expect("narrow census");
    assert_eq!(
        serde_json::to_string(&narrow).unwrap(),
        serde_json::to_string(&r).unwrap(),
        "the window size changes nothing"
    );
    let stopped = std::sync::atomic::AtomicUsize::new(0);
    let second = || stopped.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 1;
    let partial = key_census::procedure_key_census_windowed(&db, 1, &second, |_| {}).await.expect("stopped census");
    assert!(partial.stopped && partial.cursor == 1, "stopped before its second window");
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
