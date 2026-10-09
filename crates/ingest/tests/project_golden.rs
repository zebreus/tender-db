//! Cross-commit golden for the Phase-2 fold apply path (task #3, prepared-statement
//! writes). The other projection tests — `project_fold_source`, `project_resume`,
//! `project_equivalence` — compare two runs that SHARE `apply_tenders`, so a change
//! that alters `apply_tenders`' output UNIFORMLY (a reorder, a subtle bind change)
//! moves both arms together and those tests stay green while the derived layer
//! silently diverges from what it produced before. This test is the guard against
//! that: it pins the canonical layer of a fixed rich corpus to a byte-for-byte
//! golden captured BEFORE the prepared-statement conversion.
//!
//! The golden file `fixtures/golden/project_apply.snapshot` was captured on the
//! commit immediately before the fold apply path switched from `conn.execute(fresh
//! SQL)` to reused prepared statements. It MUST NOT be regenerated to make a change
//! pass — a diff here means the derived layer moved, which is exactly the
//! ADR-0001 byte-identical violation this test exists to catch. Regenerate it only
//! for a DELIBERATE, reviewed change to the derived layer's content.
//!
//! The corpus is the real Maltese CN → corrigendum → corrigendum → CAN chain (one
//! keyed Tender, four versions, with lots, lot results, contracts, parties, amounts,
//! dates, classifications and the resulting change log) plus two island fixtures —
//! every AUTOINCREMENT surrogate id (tenders, lots, bids, contracts, lot_results)
//! and the `changes.entity_id` values that depend on their INSERT order are in the
//! digest.

use ingest::project::Phase2;
use ingest::{eforms, profile, project};
use store::{Db, Notice, Parse};

const SOURCE: &str = "ted";

/// `name` keeps each test on its own file: the tests in this binary run in parallel threads of
/// ONE process, so a pid-only path would put two folds into one database.
async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projgolden-{name}-{}.db", std::process::id());
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

/// Ingest a fixture through the real dispatch + parse chain, exactly as `process`
/// would from an archived package. `source` is the notice's Source identity —
/// "ted" or "doe" — which is what the ADR-0003 cross-source merge keys on.
async fn ingest(db: &Db, fetch_id: i64, source: &str, relative: &str) {
    let bytes = std::fs::read(format!("tests/fixtures/{relative}")).expect("fixture");
    let profile::Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped a fixture");
    };
    let [profile::Record::Notice(n)] = &records[..] else {
        panic!("{relative}: expected one notice record");
    };
    let parse = eforms::parse_payload(&n.profile, &bytes);
    let (published_at, dispatched_at) = match &parse {
        Parse::Parsed(parsed) => project::notice_stamps(parsed),
        _ => panic!("{relative}: not parsed"),
    };
    db.record_notice(
        &Notice {
            source: source.into(),
            publication_id: n.publication_id.clone(),
            content_hash: n.content_hash.clone(),
            profile: n.profile.clone(),
            declared_version: n.declared_version.clone(),
            fetch_id,
            member_path: n.member_path.clone(),
            ingested_at: 0,
            published_at,
            dispatched_at,
        },
        &parse,
    )
    .await
    .expect("record notice");
}

/// Every content table the fold apply path writes, plus the change log — keyed and
/// ordered so it is stable across runs. Surrogate ids ARE compared (a fresh rebuild
/// restarts them at 1 in fold order), so a reordered INSERT under the prepared-
/// statement conversion would shift an id and break the digest.
async fn snapshot(db: &Db) -> String {
    let digests = [
        ("tenders", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source||'|'||coalesce(current_seq,-1) AS r FROM tenders ORDER BY id)"),
        ("tender_versions", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||coalesce(publication_id,'')||'|'||coalesce(notice_subtype,'')||'|'||published_at||'|'||coalesce(dispatched_at,-1) AS r FROM tender_versions ORDER BY tender_id, seq)"),
        // Issue 490: the stored elected value rides the lot row (-1 / '' = NULL).
        ("tender_version_lots", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_id||'|'||kind||'|'||coalesce(value_cents,-1)||'|'||coalesce(value_currency,'')||'|'||coalesce(value_eur_cents,-1) AS r FROM tender_version_lots ORDER BY tender_id, seq, lot_id)"),
        ("tender_version_texts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value||'|'||coalesce(lot_id,-1) AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value, lot_id)"),
        ("tender_version_dates", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds||'|'||offset_minutes||'|'||has_time||'|'||coalesce(lot_id,-1) AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds, lot_id)"),
        ("tender_version_classifications", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||scheme||'|'||code||'|'||coalesce(lot_id,-1) AS r FROM tender_version_classifications ORDER BY tender_id, seq, field, scheme, code, lot_id)"),
        ("tender_version_amounts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||cents||'|'||currency||'|'||coalesce(lot_id,-1) AS r FROM tender_version_amounts ORDER BY tender_id, seq, field, cents, lot_id)"),
        ("tender_version_parties", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||role||'|'||organization_id||'|'||mention_notice_id||'|'||mention_section_id||'|'||coalesce(lot_id,-1) AS r FROM tender_version_parties ORDER BY tender_id, seq, role, organization_id, mention_section_id, lot_id)"),
        ("lots", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||lot_key AS r FROM lots ORDER BY id)"),
        ("bids", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||notice_id||'|'||bid_key AS r FROM bids ORDER BY id)"),
        ("contracts", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||notice_id||'|'||contract_key AS r FROM contracts ORDER BY id)"),
        ("lot_results", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||notice_id||'|'||result_key AS r FROM lot_results ORDER BY id)"),
        ("tender_version_lot_results", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||coalesce(lot_id,-1)||'|'||coalesce(decision,'')||'|'||coalesce(reason,'')||'|'||coalesce(awarded_cents,-1)||'|'||coalesce(awarded_currency,'') AS r FROM tender_version_lot_results ORDER BY tender_id, seq, lot_result_id)"),
        ("tender_version_result_winners", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||organization_id AS r FROM tender_version_result_winners ORDER BY tender_id, seq, lot_result_id, organization_id)"),
        ("tender_version_result_stats", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||kind||'|'||count AS r FROM tender_version_result_stats ORDER BY tender_id, seq, lot_result_id, kind)"),
        ("tender_version_bids", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||bid_id||'|'||coalesce(lot_id,-1)||'|'||coalesce(cents,-1)||'|'||coalesce(currency,'') AS r FROM tender_version_bids ORDER BY tender_id, seq, bid_id)"),
        ("tender_version_bid_parties", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||bid_id||'|'||role||'|'||organization_id||'|'||mention_notice_id||'|'||mention_section_id AS r FROM tender_version_bid_parties ORDER BY tender_id, seq, bid_id, role, organization_id, mention_section_id)"),
        ("tender_version_contracts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||contract_id||'|'||coalesce(buyer_contract_id,'')||'|'||coalesce(concluded_utc,-1)||'|'||coalesce(concluded_offset,-1)||'|'||coalesce(concluded_has_time,-1)||'|'||coalesce(cents,-1)||'|'||coalesce(currency,'') AS r FROM tender_version_contracts ORDER BY tender_id, seq, contract_id)"),
        ("changes", "SELECT group_concat(r, x'0a') FROM (SELECT entity_kind||'|'||op||'|'||coalesce(version_seq,-1)||'|'||entity_id AS r FROM changes ORDER BY cursor)"),
    ];
    // The epoch is part of the golden so that regenerating it after a projection-
    // logic change puts PROJECTION_EPOCH in front of the person doing it (issue
    // 104): an unchanged epoch in a diff that changes fold output is exactly the
    // forgotten bump issue 99's discipline exists to prevent.
    let mut out = format!("--- projection epoch ---\n{}\n", store::canonical::PROJECTION_EPOCH);
    for (name, sql) in digests {
        let part = match db.scalar(sql).await.expect("digest query") {
            Some(turso::Value::Text(s)) => s,
            _ => String::new(),
        };
        out.push_str(&format!("--- {name} ---\n{part}\n"));
    }
    out
}

/// The canonical layer of a fixed rich corpus is byte-for-byte what the fold apply
/// path produced before the prepared-statement conversion. See the module header:
/// the golden file is a cross-commit anchor and must not be regenerated to pass.
///
/// Run on an explicit large-stack thread: turso's debug-build query execution
/// (the wide `group_concat` digests below in particular) is stack-hungry enough to
/// overflow libtest's default worker stack, so the test carries its own runtime
/// rather than depend on a `RUST_MIN_STACK` in the environment.
#[test]
fn fold_apply_output_matches_the_committed_golden() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(run())
        })
        .expect("spawn")
        .join()
        .expect("join");
}

async fn run() {
    let (db, fetch_id, path) = scratch("apply").await;
    for (source, fixture) in [
        ("ted", "eforms-chain/1-cn-16-831374-2025.xml"),
        ("ted", "eforms-chain/2-change-16-6281-2026.xml"),
        ("ted", "eforms-chain/3-change-16-18902-2026.xml"),
        ("ted", "eforms-chain/4-can-29-380868-2026.xml"),
        ("ted", "eforms/brin-x01-00497689-2026.xml"),
        ("ted", "eforms/pin-4-00496860-2026.xml"),
        // Issue 104: the DE paths were invisible to this golden — `normalise_de1`
        // is profile-gated, so a DE-only mapping change could not turn it red, and
        // the epoch-bump prompt this file exists to give never fired for exactly
        // the change class (issue 98, then issue 100) that kept happening. The
        // cross-source pair is the cohort's dominant real shape (216,450 of
        // 218,635 DE notices merged onto TED twins) and gives the fold ORDER a
        // pinned two-version chain; the two DE-1.x notices put the empirical
        // inventory's whole mapping surface into the digest.
        ("doe", "doe-ted-pair/doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml"),
        ("ted", "doe-ted-pair/ted-cn-00373130-2026.xml"),
        ("doe", "doe/eforms-de-1.1-cn-7d69b0f7.xml"),
        ("doe", "doe/eforms-de-1.2-can-799811c4.xml"),
    ] {
        ingest(&db, fetch_id, source, fixture).await;
    }

    project::project_with_progress_phase2(&db, true, 7, Phase2::Buckets { shards: None }, |_| {})
        .await
        .expect("projection");

    // The pair must fold as ONE Tender with TWO versions in published_at order —
    // asserted structurally, not only via the byte digest, so a failure here says
    // "the cross-source merge broke" rather than "some bytes differ" (issue 104).
    let pair_scalar = |sql: &'static str| async {
        match db.scalar(sql).await.expect("pair query") {
            Some(turso::Value::Integer(n)) => n,
            other => panic!("expected an integer, got {other:?}"),
        }
    };
    assert_eq!(
        pair_scalar(
            "SELECT COUNT(*) FROM tender_versions v JOIN tenders t ON t.id = v.tender_id \
              WHERE t.procedure_key = '1af86e3c-411f-4c2e-aacc-ecac61717472'"
        )
        .await,
        2,
        "the DÖE notice and its TED twin fold into one Tender with two versions"
    );
    assert_eq!(
        pair_scalar(
            "SELECT COUNT(*) FROM tender_versions a JOIN tender_versions b \
                 ON b.tender_id = a.tender_id AND b.seq = a.seq + 1 \
               JOIN tenders t ON t.id = a.tender_id \
              WHERE t.procedure_key = '1af86e3c-411f-4c2e-aacc-ecac61717472' \
                AND b.published_at < a.published_at"
        )
        .await,
        0,
        "the pair's versions are ordered by published_at"
    );

    let got = snapshot(&db).await;
    // Opt-in regeneration for a DELIBERATE, reviewed derived-layer change only —
    // never to make a failing run pass (see the module header).
    if std::env::var_os("GOLDEN_CAPTURE").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/project_apply.snapshot"),
            &got,
        )
        .expect("write golden");
    }
    let golden = include_str!("fixtures/golden/project_apply.snapshot");
    assert_eq!(
        got, golden,
        "the fold apply path's canonical layer diverged from the committed golden \
         (fixtures/golden/project_apply.snapshot) — an ADR-0001 byte-identical \
         violation. Do NOT regenerate the golden to make this pass."
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 495 unit 2's cross-commit anchor for the paths `fold_apply_output_matches_the_committed_golden`
/// never runs. That test folds with `rebuild = true`, so `stored_chain`, `delete_version` and the
/// orphan sweep are skipped; it orders leaf rows by content rather than rowid; and it leaves out
/// whole tables and columns. Unit 2 rewrites exactly those paths (prepared DELETEs, a widened
/// `stored_chain`, a leaf-indexed `Pending`, an identity cache), so this golden pins what they
/// produce TODAY, captured on the commit before unit 2 touched them:
///
/// 1. a non-rebuild fold of the corpus WITHOUT the chain's third notice;
/// 2. the third notice arrives late: a mid-chain repair with keep = 2 that deletes and rewrites
///    the tail (`versions_removed >= 1`) — snapshot A;
/// 3. every Tender aged to epoch 0 and every notice re-queued: a keep = 0 rewrite of everything,
///    the all-profile refold's path — snapshot B.
///
/// The digest is every column of every leaf table, rowid included, in (tender_id, seq, rowid)
/// order, with the column lists read from the schema rather than written here, so a column the
/// fold writes cannot escape it. It also covers the entity tables, the head columns, the currency
/// presence and the change log. Read at RUN TIME and captured only with `GOLDEN_CAPTURE_495=1`, so
/// a broad `GOLDEN_CAPTURE` can never rewrite it. Like its sibling, it must not be regenerated to
/// make a change pass.
#[test]
fn fold_refold_output_matches_the_committed_golden() {
    on_a_big_stack(|| run_refold(store::RefoldCompare::Off));
}

/// Issue 495 unit 3: the shadow compare only reads. The same refold with it on writes the
/// committed golden byte for byte, and, with no logic change between the two folds, every
/// stale Tender compares identical: nothing to rewrite, nothing to announce.
#[test]
fn a_shadow_refold_writes_the_golden_and_verifies_every_tender() {
    on_a_big_stack(|| run_refold(store::RefoldCompare::Shadow));
}

/// Issue 495 unit 3: the shadow compare counts exactly the split the flip would act on.
/// One stored row is edited at a time, every Tender is aged and re-folded in shadow, and
/// the one corrected Tender, its one rewritten table and its planned correction rows
/// (rule T, plus rule L's lots) are pinned. The rewrite then restores the edited row.
#[test]
fn the_shadow_compare_counts_what_the_flip_would_rewrite_and_announce() {
    on_a_big_stack(run_shadow_split);
}

/// Issue 495 unit 3: a notice that arrives while its Tender is stale appends a version past
/// the stored chain. That version is written and announced by its own transition rows, as
/// today: the shadow compare verifies the prefix, counts the appended version's tables as
/// written, and plans no correction.
#[test]
fn a_version_appended_to_a_stale_chain_is_not_a_correction() {
    on_a_big_stack(run_shadow_append);
}

/// Issue 495 unit 3 review: off and shadow write the same database for every chain shape a
/// stale Tender can take: equal, appended past the stored chain, moved mid-chain (only the
/// prefix of unchanged causing notices is compared), and cut back (the tail deleted up front,
/// the prefix interleaved). Compared by the full digest: every leaf row with its rowid, the
/// entity tables, the head columns and the change log in cursor order.
#[test]
fn off_and_shadow_write_the_same_for_every_stale_chain_shape() {
    on_a_big_stack(run_shapes);
}

/// Run `f`'s future on an explicit 64 MiB stack: turso's debug-build query execution (the wide
/// `group_concat` digests in particular) overflows libtest's default worker stack.
fn on_a_big_stack<F: std::future::Future<Output = ()>>(f: impl FnOnce() -> F + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(f())
        })
        .expect("spawn")
        .join()
        .expect("join");
}

async fn text_of(db: &Db, sql: &str) -> String {
    match db.scalar(sql).await.expect("digest query") {
        Some(turso::Value::Text(s)) => s,
        Some(turso::Value::Null) | None => String::new(),
        other => panic!("{sql}: expected text, got {other:?}"),
    }
}

/// One table's rows, every column `quote()`d (NULL stays distinguishable from '' and 0), rowid
/// first, in `order`. `skip` names columns that hold the wall clock.
///
/// The column list comes from the `PRAGMA table_info` STATEMENT, drained on a connection of its
/// own. Not the `pragma_table_info()` table-valued function through `Db::scalar`: turso 0.7.2
/// leaves the read snapshot of a `pragma_*` function open when its statement is dropped before
/// it finishes (`scalar` reads one row), and the pooled reader goes back to the pool still
/// holding it, `is_autocommit()` notwithstanding. Every later `scalar` then read that frozen
/// snapshot, and the first capture of this golden recorded phase A twice. See
/// docs/research/turso-scale.md.
async fn table_digest(db: &Db, table: &str, order: &str, skip: &[&str]) -> String {
    let pool = db.readers(1).expect("a reader of its own");
    let conn = pool.get().await.expect("reader connection");
    let mut info = conn.query(&format!("PRAGMA table_info(\"{table}\")"), ()).await.expect("table_info");
    let mut names = Vec::new();
    while let Some(row) = info.next().await.expect("table_info row") {
        match row.get_value(1) {
            Ok(turso::Value::Text(name)) => names.push(name),
            other => panic!("{table}: table_info name is {other:?}"),
        }
    }
    drop(info);
    drop(conn);
    assert!(!names.is_empty(), "{table}: table_info named no columns");
    let cols = names.join(",");
    let mut expr = String::from("quote(rowid)");
    for col in names.iter().filter(|c| !skip.contains(&c.as_str())) {
        expr.push_str(&format!("||'|'||quote(\"{col}\")"));
    }
    let rows = text_of(db, &format!("SELECT group_concat(r, x'0a') FROM (SELECT {expr} AS r FROM \"{table}\" ORDER BY {order})")).await;
    let count = text_of(db, &format!("SELECT CAST(COUNT(*) AS TEXT) FROM \"{table}\"")).await;
    format!("--- {table} ({count} rows; {cols}) ---\n{rows}\n")
}

async fn full_digest(db: &Db) -> String {
    let leaves = text_of(
        db,
        "SELECT group_concat(name, ',') FROM (SELECT name FROM sqlite_master WHERE type = 'table' \
           AND (name = 'tender_versions' OR name GLOB 'tender_version_*') ORDER BY name)",
    )
    .await;
    assert_eq!(leaves.split(',').count(), 14, "the fold's version-keyed tables: {leaves}");
    let mut out = format!("--- projection epoch ---\n{}\n", store::canonical::PROJECTION_EPOCH);
    for table in leaves.split(',') {
        out.push_str(&table_digest(db, table, "tender_id, seq, rowid", &[]).await);
    }
    out.push_str(&table_digest(db, "tenders", "id", &["created_at"]).await);
    for table in ["lots", "lot_results", "bids", "contracts"] {
        out.push_str(&table_digest(db, table, "id", &[]).await);
    }
    out.push_str(&table_digest(db, "tender_currency_presence", "rowid", &[]).await);
    out.push_str(&table_digest(db, "changes", "cursor", &["changed_at"]).await);
    out
}

async fn run_refold(compare: store::RefoldCompare) {
    let (db, fetch_id, path) = scratch(&format!("refold-{compare:?}")).await;
    db.set_refold_compare(compare);
    const LATE: &str = "eforms-chain/3-change-16-18902-2026.xml";
    let corpus = [
        ("ted", "eforms-chain/1-cn-16-831374-2025.xml"),
        ("ted", "eforms-chain/2-change-16-6281-2026.xml"),
        ("ted", "eforms-chain/4-can-29-380868-2026.xml"),
        ("ted", "eforms/brin-x01-00497689-2026.xml"),
        ("ted", "eforms/pin-4-00496860-2026.xml"),
        ("doe", "doe-ted-pair/doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml"),
        ("ted", "doe-ted-pair/ted-cn-00373130-2026.xml"),
        ("doe", "doe/eforms-de-1.1-cn-7d69b0f7.xml"),
        ("doe", "doe/eforms-de-1.2-can-799811c4.xml"),
    ];
    for (source, fixture) in corpus {
        ingest(&db, fetch_id, source, fixture).await;
    }
    project::project(&db, false).await.expect("first fold");

    ingest(&db, fetch_id, "ted", LATE).await;
    let repair = project::project_incremental(&db).await.expect("late-notice repair");
    assert!(
        repair.applied.versions_removed >= 1,
        "the late notice must repair mid-chain (delete_version and the sweep run): {:?}",
        repair.applied
    );
    assert_eq!(repair.applied.compare_rows, 0, "a current-epoch repair is never compared: {:?}", repair.applied);
    let mut got = String::from("=== A: the late notice's mid-chain repair ===\n");
    got.push_str(&full_digest(&db).await);

    // The compare refuses to run without `tender_version_bid_parties_version` (a rebuild
    // defers it). Indexes are not in the digest, so both arms build them.
    db.build_tender_indexes().await.expect("the by-version indexes");
    db.set_projection_epoch_for_test(0).await.expect("age every Tender");
    let ids = text_of(&db, "SELECT group_concat(id, ',') FROM (SELECT id FROM notices ORDER BY id)").await;
    let ids: Vec<i64> = ids.split(',').map(|id| id.parse().expect("notice id")).collect();
    db.unmark_projected_by_ids(&ids).await.expect("re-queue every notice");
    let refold = project::project_incremental(&db).await.expect("epoch-stale refold");
    assert!(
        refold.applied.versions_written > 0 && refold.applied.versions_removed == refold.applied.versions_written,
        "every stale Tender must be rewritten from keep = 0: {:?}",
        refold.applied
    );
    got.push_str("=== B: the keep = 0 refold of everything ===\n");
    got.push_str(&full_digest(&db).await);
    let a = refold.applied;
    match compare {
        store::RefoldCompare::Off => {
            assert_eq!((a.tenders_verified, a.tenders_corrected, a.compare_rows), (0, 0, 0), "{a:?}");
        }
        store::RefoldCompare::Shadow => {
            assert!(a.tenders_written > 0, "{a:?}");
            assert_eq!(a.tenders_verified, a.tenders_written, "every stale Tender compares identical: {a:?}");
            assert_eq!(
                (a.tenders_corrected, a.tables_rewritten, a.rows_rewritten, a.correction_rows_planned),
                (0, 0, 0, 0),
                "nothing differs, so nothing would be rewritten or announced: {a:?}"
            );
            assert!(a.tables_skipped > 0, "{a:?}");
            assert_eq!(a.compare_rows, a.rows_skipped, "every stored row read back matched a fresh one: {a:?}");
            assert!(a.compare_line().is_some());
        }
    }

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/project_refold.snapshot");
    if std::env::var_os("GOLDEN_CAPTURE_495").is_some() {
        std::fs::write(file, &got).expect("write golden");
    }
    let golden = std::fs::read_to_string(file).expect(
        "fixtures/golden/project_refold.snapshot is missing — it is captured once, with \
         GOLDEN_CAPTURE_495=1, on a commit that changes no production code",
    );
    assert_eq!(
        got, golden,
        "the non-rebuild fold paths (stored_chain, delete_version, the sweep, keep = 0) diverged \
         from the committed golden (fixtures/golden/project_refold.snapshot). Do NOT regenerate \
         it to make this pass."
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

async fn int_of(db: &Db, sql: &str) -> i64 {
    match db.scalar(sql).await.expect("query") {
        Some(turso::Value::Integer(n)) => n,
        other => panic!("{sql}: expected an integer, got {other:?}"),
    }
}

async fn run_shadow_split() {
    let (db, fetch_id, path) = scratch("shadow-split").await;
    db.set_refold_compare(store::RefoldCompare::Shadow);
    for (source, fixture) in [
        ("ted", "eforms-chain/1-cn-16-831374-2025.xml"),
        ("ted", "eforms-chain/2-change-16-6281-2026.xml"),
        ("ted", "eforms-chain/3-change-16-18902-2026.xml"),
        ("ted", "eforms-chain/4-can-29-380868-2026.xml"),
        ("ted", "eforms/pin-4-00496860-2026.xml"),
    ] {
        ingest(&db, fetch_id, source, fixture).await;
    }
    project::project(&db, false).await.expect("first fold");
    db.build_tender_indexes().await.expect("the by-version indexes the compare needs");
    let chain = int_of(
        &db,
        "SELECT tender_id FROM tender_versions GROUP BY tender_id ORDER BY COUNT(*) DESC, tender_id LIMIT 1",
    )
    .await;
    let head = int_of(&db, &format!("SELECT MAX(seq) FROM tender_versions WHERE tender_id = {chain}")).await;
    assert!(head >= 3, "the chain fixture folds to a multi-version Tender");
    let head_lots = int_of(
        &db,
        &format!("SELECT COUNT(DISTINCT lot_id) FROM tender_version_lots WHERE tender_id = {chain} AND seq = {head}"),
    )
    .await;
    assert!(head_lots > 0, "the head version declares lots");
    // A lot-scoped text below the head: the lowest seq that has one.
    let lot_seq = int_of(
        &db,
        &format!(
            "SELECT MIN(seq) FROM tender_version_texts WHERE tender_id = {chain} AND seq < {head} AND lot_id IS NOT NULL"
        ),
    )
    .await;

    // (what, seq, which text row, correction rows planned)
    let cases = [
        ("a Tender-level text below the head", 1, "lot_id IS NULL", 1),
        ("a lot's text below the head", lot_seq, "lot_id IS NOT NULL", 2),
        ("a Tender-level text at the head", head, "lot_id IS NULL", 1 + head_lots),
    ];
    for (what, seq, which, planned) in cases {
        let rows = int_of(
            &db,
            &format!("SELECT COUNT(*) FROM tender_version_texts WHERE tender_id = {chain} AND seq = {seq}"),
        )
        .await;
        let edited = db
            .execute_for_test(&format!(
                "UPDATE tender_version_texts SET value = value || ' (edited)' WHERE rowid = \
                   (SELECT MIN(rowid) FROM tender_version_texts WHERE tender_id = {chain} AND seq = {seq} AND {which})"
            ))
            .await
            .expect("edit one stored row");
        assert_eq!(edited, 1, "{what}: one row to edit");
        db.set_projection_epoch_for_test(0).await.expect("age every Tender");
        let ids = text_of(&db, "SELECT group_concat(id, ',') FROM (SELECT id FROM notices ORDER BY id)").await;
        let ids: Vec<i64> = ids.split(',').map(|id| id.parse().expect("notice id")).collect();
        db.unmark_projected_by_ids(&ids).await.expect("re-queue every notice");
        let a = project::project_incremental(&db).await.expect("shadow refold").applied;
        assert_eq!(a.tenders_corrected, 1, "{what}: one corrected Tender: {a:?}");
        assert_eq!(a.tenders_verified, a.tenders_written - 1, "{what}: every other Tender verified: {a:?}");
        assert_eq!((a.tables_rewritten, a.rows_rewritten), (1, rows as u64), "{what}: that version's texts, whole: {a:?}");
        assert_eq!(a.correction_rows_planned, planned as u64, "{what}: rule T plus rule L: {a:?}");
        assert_eq!(
            int_of(&db, "SELECT COUNT(*) FROM tender_version_texts WHERE value LIKE '% (edited)'").await,
            0,
            "{what}: the shadow fold still rewrites everything, so the edit is gone"
        );
    }

    // Changes outside the leaf tables, each announced as a head difference (rule T plus the
    // head version's lots) with no table rewritten: `tender_identity` moving a stored `kind`
    // back in place, and a head column that moved while every version stayed the same.
    for (what, column) in [("a moved kind", "kind"), ("a moved head title", "current_title")] {
        let before = text_of(&db, &format!("SELECT {column} FROM tenders WHERE id = {chain}")).await;
        db.execute_for_test(&format!("UPDATE tenders SET {column} = 'edited' WHERE id = {chain}"))
            .await
            .expect("edit the stored Tender row");
        db.set_projection_epoch_for_test(0).await.expect("age every Tender");
        let ids = text_of(&db, "SELECT group_concat(id, ',') FROM (SELECT id FROM notices ORDER BY id)").await;
        let ids: Vec<i64> = ids.split(',').map(|id| id.parse().expect("notice id")).collect();
        db.unmark_projected_by_ids(&ids).await.expect("re-queue every notice");
        let a = project::project_incremental(&db).await.expect("shadow refold").applied;
        assert_eq!(
            (a.tenders_corrected, a.tables_rewritten, a.correction_rows_planned),
            (1, 0, 1 + head_lots as u64),
            "{what}: a head difference with no table rewritten: {a:?}"
        );
        assert_eq!(text_of(&db, &format!("SELECT {column} FROM tenders WHERE id = {chain}")).await, before, "{what}");
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

async fn run_shadow_append() {
    let (db, fetch_id, path) = scratch("shadow-append").await;
    db.set_refold_compare(store::RefoldCompare::Shadow);
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
    ] {
        ingest(&db, fetch_id, "ted", fixture).await;
    }
    project::project(&db, false).await.expect("first fold");
    db.build_tender_indexes().await.expect("the by-version indexes the compare needs");
    db.set_projection_epoch_for_test(0).await.expect("age every Tender");
    ingest(&db, fetch_id, "ted", "eforms-chain/4-can-29-380868-2026.xml").await;
    let ids = text_of(&db, "SELECT group_concat(id, ',') FROM (SELECT id FROM notices ORDER BY id)").await;
    let ids: Vec<i64> = ids.split(',').map(|id| id.parse().expect("notice id")).collect();
    db.unmark_projected_by_ids(&ids).await.expect("re-queue every notice");
    let a = project::project_incremental(&db).await.expect("shadow refold with an appended notice").applied;
    assert_eq!(a.versions_written, 4, "the three stored versions rewritten and the fourth appended: {a:?}");
    assert_eq!((a.tenders_verified, a.tenders_corrected, a.correction_rows_planned), (1, 0, 0), "{a:?}");
    assert!(a.tables_rewritten > 0 && a.rows_rewritten > 0, "the appended version's tables are written: {a:?}");
    assert_eq!(a.compare_rows, a.rows_skipped, "the stored prefix compared identical: {a:?}");
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// One stale-chain shape under one compare mode: the full digest after the refold, and its tally.
async fn stale_shape(compare: store::RefoldCompare, shape: &str) -> (String, store::Applied) {
    const CHAIN: [&str; 4] = [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ];
    let (db, fetch_id, path) = scratch(&format!("shape-{shape}-{compare:?}")).await;
    db.set_refold_compare(compare);
    let first: &[usize] = match shape {
        "append" => &[0, 1, 2],
        "move" => &[0, 1, 3],
        _ => &[0, 1, 2, 3],
    };
    for &i in first {
        ingest(&db, fetch_id, "ted", CHAIN[i]).await;
    }
    ingest(&db, fetch_id, "ted", "eforms/pin-4-00496860-2026.xml").await;
    project::project(&db, false).await.expect("first fold");
    db.build_tender_indexes().await.expect("the by-version indexes the compare needs");
    db.set_projection_epoch_for_test(0).await.expect("age every Tender");
    match shape {
        "append" => ingest(&db, fetch_id, "ted", CHAIN[3]).await,
        "move" => ingest(&db, fetch_id, "ted", CHAIN[2]).await,
        "cut" => {
            // The stored chain one version longer than the one the fold derives: a phantom
            // version past the chain's end, with a leaf row and the head pointing at it, as
            // a notice dropped from the plan would leave behind.
            let chain = int_of(
                &db,
                "SELECT tender_id FROM tender_versions GROUP BY tender_id ORDER BY COUNT(*) DESC, tender_id LIMIT 1",
            )
            .await;
            let notice = int_of(&db, "SELECT MAX(id) FROM notices").await;
            for sql in [
                format!(
                    "INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, publication_id) \
                     VALUES ({chain}, 5, {notice}, 0, 'phantom')"
                ),
                format!(
                    "INSERT INTO tender_version_texts(tender_id, seq, lot_id, field, lang, value) \
                     VALUES ({chain}, 5, NULL, 'title', 'en', 'phantom')"
                ),
                format!("UPDATE tenders SET current_seq = 5 WHERE id = {chain}"),
            ] {
                assert_eq!(db.execute_for_test(&sql).await.expect("plant the phantom"), 1, "{sql}");
            }
        }
        _ => {}
    }
    let ids = text_of(&db, "SELECT group_concat(id, ',') FROM (SELECT id FROM notices ORDER BY id)").await;
    let ids: Vec<i64> = ids.split(',').map(|id| id.parse().expect("notice id")).collect();
    db.unmark_projected_by_ids(&ids).await.expect("re-queue every notice");
    let applied = project::project_incremental(&db).await.expect("stale refold").applied;
    let digest = full_digest(&db).await;
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    (digest, applied)
}

async fn run_shapes() {
    for shape in ["equal", "append", "move", "cut"] {
        let (off, a_off) = stale_shape(store::RefoldCompare::Off, shape).await;
        let (shadow, a) = stale_shape(store::RefoldCompare::Shadow, shape).await;
        assert_eq!(off, shadow, "{shape}: shadow wrote something off did not");
        assert_eq!(
            (a_off.versions_written, a_off.versions_removed, a_off.changes, a_off.entities_swept),
            (a.versions_written, a.versions_removed, a.changes, a.entities_swept),
            "{shape}: the same work, counted the same"
        );
        assert!(a.compare_rows > 0 || shape == "move", "{shape}: the compare ran: {a:?}");
        match shape {
            // Nothing moved inside the compared prefix: versions past it are transitions.
            "equal" | "append" | "move" => {
                assert_eq!((a.tenders_corrected, a.correction_rows_planned), (0, 0), "{shape}: {a:?}");
            }
            // The head went back a version with nothing written past the prefix.
            _ => {
                assert_eq!(a.tenders_corrected, 1, "{shape}: {a:?}");
                assert!(a.correction_rows_planned >= 1, "{shape}: {a:?}");
            }
        }
    }
}
