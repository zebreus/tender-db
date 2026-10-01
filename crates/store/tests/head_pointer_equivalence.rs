//! Issue 457 R2: `tenders.current_seq` IS `MAX(tender_versions.seq)` for every
//! Tender, through every writer of either column — the equivalence the list
//! pages now rest on, since they read the pointer instead of recomputing the
//! correlated `MAX(seq)` turso 0.8.1 may unnest into a GROUP BY over every
//! version row.
//!
//! The writers, read off the code (2026-10-01): the fold's `apply_tender_tx`
//! (deletes the stale tail by seq, writes seq `keep+1..=N`, sets the head to
//! `N`, and `assert_heads_match` refuses the batch before COMMIT otherwise),
//! the retirements (`retire_chunk_tx` deletes a Tender's versions WITH the
//! Tender), `reset_tender_layer`/`clear_canonical` (drop both tables), and the
//! one-time backfill at open (`lib.rs`, pinned by its own test). Nothing else
//! writes `tender_versions.seq` or `tenders.current_seq`; the twin repair
//! deletes notices and leaves their versions to the next fold.
//!
//! So this drives the real `apply_tenders` through every chain transition it
//! has — a rebuild, an appended (superseding) version, a late arrival
//! mid-chain, a withdrawn tail, a withdrawn middle, an epoch-forced full
//! rewrite, and a merge (one Tender absorbing another's notices while the
//! absorbed one is retired) — and after each asserts the invariant for every
//! Tender, then that the shipped pages agree with a `MAX(seq)` oracle on the
//! same data. The satellites differ per version, so a page reading the wrong
//! version returns different rows, not the same ones. A final control corrupts
//! one pointer by hand and shows both checks notice.

use std::collections::{BTreeMap, BTreeSet};

use store::canonical::{Fact, LotState, TenderProjection, TenderVersion};
use store::read::{self, Filter, HeadOrder, Scope, Status};
use store::turso::{Connection, Value};
use store::Db;

const NOW: i64 = 1_790_000_000;

/// One version: published at `day` (seconds), placed in `nuts`, with a buyer,
/// a submission deadline and two lots whose kind says which version wrote them.
fn version(notice: i64, day: i64, nuts: &str, buyer: i64, deadline: i64, subtype: Option<&str>) -> TenderVersion {
    let mut facts = BTreeSet::new();
    facts.insert(Fact::Classification { field: "place".into(), scheme: "nuts".into(), code: nuts.into() });
    facts.insert(Fact::Classification { field: "main".into(), scheme: "cpv".into(), code: format!("45{notice:06}") });
    facts.insert(Fact::Party {
        role: "Buyer".into(),
        organization_id: buyer,
        notice_id: notice,
        section_id: "ORG-1".into(),
    });
    facts.insert(Fact::Date {
        field: "submission_deadline".into(),
        utc_seconds: deadline,
        offset_minutes: 0,
        has_time: true,
    });
    facts.insert(Fact::Text { field: "title".into(), lang: Some("ENG".into()), value: format!("notice {notice}") });
    // The kind moves with the version, so `?kind=` discriminates the seq too.
    let kind = if notice % 2 == 0 { "Lot" } else { "Part" };
    let lots = ["LOT-1", "LOT-2"]
        .iter()
        .map(|key| LotState { key: (*key).into(), kind: kind.into(), facts: BTreeSet::new() })
        .collect();
    TenderVersion {
        caused_by_notice_id: notice,
        published_at: day,
        dispatched_at: None,
        notice_subtype: subtype.map(str::to_owned),
        original_lang: None,
        publication_id: format!("{notice:08}-2026"),
        facts,
        lots,
        rounds: Vec::new(),
        group_members: Vec::new(),
    }
}

fn keyed(key: &str, versions: Vec<TenderVersion>) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(key.into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions,
    }
}

fn island(notice: i64, v: TenderVersion) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: None,
        island_notice_id: Some(notice),
        kind: "procedure".into(),
        versions: vec![v],
    }
}

/// The notices each Tender folds, by key — the test's model of the chains.
#[derive(Clone)]
struct Corpus(BTreeMap<&'static str, Vec<TenderVersion>>);

impl Corpus {
    fn projection(&self, key: &str) -> TenderProjection {
        keyed(key, self.0[key].clone())
    }
}

async fn id_of(db: &Db, key: &str) -> i64 {
    match db.scalar(&format!("SELECT id FROM tenders WHERE procedure_key = '{key}'")).await.unwrap() {
        Some(Value::Integer(id)) => id,
        other => panic!("no tender {key}: {other:?}"),
    }
}

/// Every Tender's `(current_seq, MAX(seq))`, as the reads see them.
async fn heads(conn: &Connection) -> BTreeMap<i64, (Option<i64>, Option<i64>)> {
    let mut rows = conn
        .query(
            "SELECT t.id, t.current_seq,
                    (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)
               FROM tenders t ORDER BY t.id",
            (),
        )
        .await
        .unwrap();
    let mut out = BTreeMap::new();
    while let Some(row) = rows.next().await.unwrap() {
        let opt = |i| match row.get_value(i).unwrap() {
            Value::Integer(n) => Some(n),
            _ => None,
        };
        out.insert(opt(0).unwrap(), (opt(1), opt(2)));
    }
    out
}

/// The invariant itself, for every Tender: the pointer IS the last version, and
/// no version row belongs to a Tender that is gone.
async fn assert_invariant(db: &Db, conn: &Connection, step: &str) -> BTreeMap<i64, i64> {
    let heads = heads(conn).await;
    assert!(!heads.is_empty(), "{step}: no tenders at all — the check would be vacuous");
    let wrong: Vec<_> = heads.iter().filter(|(_, (cur, max))| cur != max).collect();
    assert!(wrong.is_empty(), "{step}: current_seq is not MAX(seq) for (id, (current_seq, max)): {wrong:?}");
    let orphans = db
        .scalar("SELECT COUNT(*) FROM tender_versions v WHERE NOT EXISTS (SELECT 1 FROM tenders t WHERE t.id = v.tender_id)")
        .await
        .unwrap();
    assert_eq!(orphans, Some(Value::Integer(0)), "{step}: version rows outlive their Tender");
    heads.into_iter().filter_map(|(id, (_, max))| max.map(|m| (id, m))).collect()
}

/// The tender ids whose HEAD — recomputed as `MAX(seq)`, never read from the
/// pointer — satisfies `head_sql`, a predicate on `m.id` and its head seq `m.h`.
async fn oracle(conn: &Connection, head_sql: &str) -> BTreeSet<i64> {
    let sql = format!(
        "SELECT m.id FROM (SELECT t.id AS id,
                                  (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id) AS h
                             FROM tenders t) m
          WHERE m.h IS NOT NULL AND ({head_sql}) ORDER BY m.id"
    );
    let mut rows = conn.query(&sql, ()).await.unwrap();
    let mut out = BTreeSet::new();
    while let Some(row) = rows.next().await.unwrap() {
        if let Value::Integer(id) = row.get_value(0).unwrap() {
            out.insert(id);
        }
    }
    out
}

/// The shipped pages, read against the `MAX(seq)` oracle on the same data. Every
/// row's `seq` must be its Tender's last version, and every filtered set must be
/// the set whose LAST version matches.
async fn assert_pages_agree(conn: &Connection, step: &str, max: &BTreeMap<i64, i64>) -> Vec<String> {
    let mut faults = Vec::new();
    let base = Filter { now: NOW, ..Filter::default() };
    let cases: Vec<(&str, Filter, String)> = vec![
        ("none", base.clone(), "1 = 1".into()),
        (
            "country=DE",
            Filter { country: Some("DE".into()), ..base.clone() },
            "EXISTS (SELECT 1 FROM tender_version_classifications c WHERE c.tender_id = m.id
                      AND c.seq = m.h AND c.scheme = 'nuts' AND c.code LIKE 'DE%')"
                .into(),
        ),
        (
            "buyer=7",
            Filter { buyer: Some(7), ..base.clone() },
            "EXISTS (SELECT 1 FROM tender_version_parties p WHERE p.tender_id = m.id
                      AND p.seq = m.h AND p.organization_id = 7 AND p.role LIKE '%Buyer%')"
                .into(),
        ),
        (
            "status=open",
            Filter { status: Some(Status::Open), ..base.clone() },
            format!(
                "EXISTS (SELECT 1 FROM tender_version_dates d WHERE d.tender_id = m.id AND d.seq = m.h
                          AND d.field = 'submission_deadline' AND d.utc_seconds > {NOW})"
            ),
        ),
    ];
    for (name, f, head_sql) in &cases {
        let want = oracle(conn, head_sql).await;
        // A filter that selects every Tender or none cannot tell a head read from
        // a stale one (lots_filter_fixture's lesson), so each must split the set.
        if *name != "none" && (want.is_empty() || want.len() == max.len()) {
            faults.push(format!("{step} {name}: the fixture does not discriminate ({} of {})", want.len(), max.len()));
        }
        let check = |shape: &str, rows: Vec<(i64, i64)>, faults: &mut Vec<String>| {
            for (id, seq) in &rows {
                if max.get(id) != Some(seq) {
                    faults.push(format!("{step} {shape} {name}: tender {id} served seq {seq}, last is {:?}", max.get(id)));
                }
            }
            let got: BTreeSet<i64> = rows.iter().map(|(id, _)| *id).collect();
            if got != want {
                faults.push(format!("{step} {shape} {name}: served {got:?}, the MAX(seq) oracle says {want:?}"));
            }
        };
        let page = read::tenders_page(conn, f, 0, 1000, 1_000_000).await.unwrap();
        check("tenders_page", page.rows.iter().map(|r| (r.id, r.seq)).collect(), &mut faults);
        for order in [HeadOrder::PublishedAt, HeadOrder::Deadline] {
            let rows = read::tenders_ordered(conn, f, order, true, None, 1000).await.unwrap();
            // The ordered pages drop a Tender whose head lacks the ordering value
            // (every head here has both), so the set is the oracle's.
            check(&format!("tenders_ordered {order:?}"), rows.iter().map(|r| (r.id, r.seq)).collect(), &mut faults);
        }
        // Lots: the shipped stream against its MAX(seq) oracle shape, row for row.
        let scope = Scope::Page { after: 0, limit: 1000 };
        let shipped = read::lots(conn, f, scope).await.unwrap();
        let previous = read::lots_previous_shape(conn, f, scope).await.unwrap();
        if shipped != previous {
            faults.push(format!("{step} lots {name}: shipped {shipped:?}\n  oracle {previous:?}"));
        }
        for lot in &shipped {
            if max.get(&lot.tender_id) != Some(&lot.seq) {
                faults.push(format!("{step} lots {name}: lot {} served seq {}", lot.id, lot.seq));
            }
        }
        let banded = read::lots_page(conn, f, 0, 1000, 1_000_000).await.unwrap();
        if banded.rows != shipped {
            faults.push(format!("{step} lots_page {name}: the banded page differs from the stream"));
        }
    }
    // `kind` is a property of the version's lot row: only the last version's kind
    // may match.
    for kind in ["Lot", "Part"] {
        let f = Filter { kind: Some(kind.into()), ..base.clone() };
        let scope = Scope::Page { after: 0, limit: 1000 };
        let shipped = read::lots(conn, &f, scope).await.unwrap();
        let previous = read::lots_previous_shape(conn, &f, scope).await.unwrap();
        if shipped != previous {
            faults.push(format!("{step} lots kind={kind}: shipped {shipped:?}\n  oracle {previous:?}"));
        }
    }
    faults
}

async fn check(db: &Db, conn: &Connection, step: &str) {
    let max = assert_invariant(db, conn, step).await;
    let faults = assert_pages_agree(conn, step, &max).await;
    assert!(faults.is_empty(), "{step}: the pages disagree with MAX(seq):\n{}", faults.join("\n"));
}

#[tokio::test]
async fn the_head_pointer_is_the_last_version_through_every_fold_transition() {
    let path = format!("/tmp/tender-db-457-r2-heads-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = Db::open(&path).await.unwrap();
    // Reads go through a plain connection on the same file, as the other read
    // tests here do; the pages take a `Connection`.
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    // The fold runs with foreign keys off (issue 19); the parties here name
    // mentions this test never writes.
    db.set_foreign_keys(false).await.unwrap();

    let open = NOW + 86_400 * 30;
    let closed = NOW - 86_400 * 30;
    let day = |d: i64| 1_700_000_000 + d * 86_400;
    // A: three versions whose place, buyer and deadline all move — the head is
    // DE / buyer 7 / open, every superseded version something else.
    // B: two versions; D and E: two each, to be merged; C: one; plus an island.
    let mut corpus = Corpus(BTreeMap::from([
        (
            "A",
            vec![
                version(100, day(1), "FR101", 3, closed, None),
                version(101, day(2), "FR102", 3, closed, Some("corrigendum")),
                version(102, day(3), "DE300", 7, open, Some("can-modif")),
            ],
        ),
        ("B", vec![version(200, day(1), "DE111", 7, open, None), version(202, day(5), "AT130", 3, closed, None)]),
        ("C", vec![version(300, day(2), "DE212", 7, open, None)]),
        ("D", vec![version(400, day(1), "IT100", 3, closed, None), version(402, day(4), "DE600", 7, open, None)]),
        ("E", vec![version(401, day(2), "PL911", 3, closed, None), version(403, day(6), "DE700", 7, open, None)]),
    ]));
    let all = |c: &Corpus| -> Vec<TenderProjection> {
        let mut out: Vec<TenderProjection> = c.0.keys().map(|k| c.projection(k)).collect();
        out.push(island(900, version(900, day(1), "DE999", 7, open, None)));
        out
    };

    // 1. A rebuild: the layer emptied, every Tender minted fresh.
    db.reset_tender_layer().await.unwrap();
    db.apply_tenders(&all(&corpus), NOW, true).await.expect("the rebuild fold");
    check(&db, &conn, "rebuild").await;
    let a = id_of(&db, "A").await;
    assert_eq!(heads(&conn).await[&a], (Some(3), Some(3)), "A's head is its third version");

    // 2. A superseding version appended to A — the incremental fold's common case.
    corpus.0.get_mut("A").unwrap().push(version(104, day(7), "FR103", 3, closed, Some("cancellation")));
    db.apply_tenders(&[corpus.projection("A")], NOW, false).await.expect("append");
    check(&db, &conn, "append").await;
    assert_eq!(heads(&conn).await[&a], (Some(4), Some(4)), "the head moved to the new version");

    // 3. A late arrival mid-chain: B's new notice sorts between its two, so the
    //    tail from seq 2 is rewritten (keep = 1).
    corpus.0.get_mut("B").unwrap().insert(1, version(201, day(3), "BE100", 3, closed, None));
    db.apply_tenders(&[corpus.projection("B")], NOW, false).await.expect("late arrival");
    check(&db, &conn, "late arrival").await;

    // 4. A withdrawn tail: A's head notice leaves the chain (regrouped away,
    //    re-parsed, or a twin repair) — the head falls back to seq 3.
    corpus.0.get_mut("A").unwrap().pop();
    db.apply_tenders(&[corpus.projection("A")], NOW, false).await.expect("withdrawn tail");
    check(&db, &conn, "withdrawn tail").await;
    assert_eq!(heads(&conn).await[&a], (Some(3), Some(3)), "the head fell back with the withdrawal");

    // 5. A withdrawn middle: A's corrigendum leaves, so seq 2..3 are rewritten as
    //    seq 2 and the old seq 3 is deleted.
    corpus.0.get_mut("A").unwrap().remove(1);
    db.apply_tenders(&[corpus.projection("A")], NOW, false).await.expect("withdrawn middle");
    check(&db, &conn, "withdrawn middle").await;
    assert_eq!(heads(&conn).await[&a], (Some(2), Some(2)));

    // 6. An epoch-forced refold: C stamped stale and re-folded unchanged — the
    //    keep = 0 rewrite of the whole chain.
    let c = id_of(&db, "C").await;
    assert_eq!(db.stamp_stale_for_tenders(&[c]).await.unwrap(), 1);
    let applied = db.apply_tenders(&[corpus.projection("C")], NOW, false).await.expect("forced refold");
    assert_eq!(applied.tenders_written, 1, "the stale stamp forced a rewrite");
    check(&db, &conn, "epoch refold").await;

    // 7. A merge: E's notices regroup into D (the legacy-bridge / BT-04 shape).
    //    The incremental path retires the absorbed Tender first, then folds the
    //    survivor's interleaved chain — rewritten from seq 2.
    let e = id_of(&db, "E").await;
    db.reset_plan().await.unwrap(); // an empty plan: E's key is produced by nothing
    assert_eq!(db.retire_regrouped_tenders(&[e], NOW).await.unwrap(), 1, "E is retired");
    let absorbed = corpus.0.remove("E").unwrap();
    let d = corpus.0.get_mut("D").unwrap();
    d.extend(absorbed);
    d.sort_by_key(|v| v.published_at);
    db.apply_tenders(&[corpus.projection("D")], NOW, false).await.expect("merge");
    db.clear_plan().await.unwrap();
    check(&db, &conn, "merge").await;
    let d = id_of(&db, "D").await;
    assert_eq!(heads(&conn).await[&d], (Some(4), Some(4)), "the survivor holds all four notices");

    // 8. An unchanged re-fold of everything writes nothing and moves nothing.
    let before = heads(&conn).await;
    let applied = db.apply_tenders(&all(&corpus), NOW, false).await.expect("idempotent refold");
    assert_eq!(applied.tenders_written, 0, "an unchanged corpus re-folds to no writes");
    assert_eq!(heads(&conn).await, before);
    check(&db, &conn, "idempotent refold").await;

    // The control: both checks can say NO. Point B's head at a superseded
    // version by hand — the state the fold refuses to commit — and the invariant
    // names it while the pages, which now read the pointer, part from the oracle.
    let b = id_of(&db, "B").await;
    conn.execute("UPDATE tenders SET current_seq = 1 WHERE id = ?", (Value::Integer(b),)).await.unwrap();
    let corrupted = heads(&conn).await;
    assert_eq!(corrupted[&b], (Some(1), Some(3)), "the control corrupted B's pointer");
    let max: BTreeMap<i64, i64> =
        corrupted.iter().filter_map(|(id, (_, max))| max.map(|m| (*id, m))).collect();
    let faults = assert_pages_agree(&conn, "control", &max).await;
    assert!(
        faults.iter().any(|f| f.contains(&format!("tender {b} served seq 1"))),
        "a pointer that is not the last version must surface in the page check:\n{}",
        faults.join("\n")
    );
    conn.execute("UPDATE tenders SET current_seq = 3 WHERE id = ?", (Value::Integer(b),)).await.unwrap();
    check(&db, &conn, "control restored").await;

    drop(conn);
    drop(raw);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
