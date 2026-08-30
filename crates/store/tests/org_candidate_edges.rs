//! Issue 300 Stage 4 Unit 4: the candidate-edge scan — E3 emission rules
//! (canonical×canonical and provisional×canonical only), the e3-name /
//! e3-xlang split from reach provenance, the >cap stoplist with its top
//! sample, the same-key-fills-page walk-termination guard, refresh
//! semantics preserving first_seen/state, T4 parity aborts, honest cancel,
//! and the no-entity-writes contract. Key fns are test-local stand-ins;
//! the real N2/N3 content is pinned by ingest's own tests.

use store::turso::Value;

fn n2(s: &str) -> String {
    s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn n3(s: &str) -> String {
    // The mini canonicalizer: the token "gmbh" becomes the family marker.
    n2(s)
        .split(' ')
        .map(|t| if t == "gmbh" { "§gmbh" } else { t })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

async fn open(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn org(conn: &store::turso::Connection, id: i64, name: &str, ident: Option<&str>) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, 'DE', ?, ?, ?, ?, ?, 0)",
        (
            Value::Integer(id),
            match ident {
                Some(_) => Value::Text("national".into()),
                None => Value::Null,
            },
            match ident {
                Some(i) => Value::Text(i.into()),
                None => Value::Null,
            },
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
            Value::Integer(if ident.is_some() { 0 } else { 1 }),
        ),
    )
    .await
    .unwrap();
}

async fn key_row(conn: &store::turso::Connection, org: i64, kind: &str, key: &str) {
    conn.execute(
        "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, ?, ?)",
        (Value::Integer(org), Value::Text(kind.into()), Value::Text(key.into())),
    )
    .await
    .unwrap();
}

fn args<'a>(
    dry_run: bool,
    stop: &'a (dyn Fn() -> bool + Sync),
    progress: &'a (dyn Fn(u64, &str) + Sync),
) -> store::OrgEdgeScanArgs<'a> {
    store::OrgEdgeScanArgs {
        n2,
        n3,
        stoplist_cap: 20,
        key_window: store::SCAN_KEY_WINDOW,
        max_edges: None,
        expect_edges: None,
        exemplar_org: None,
        dry_run,
        job_id: Some(7),
        stop,
        progress,
    }
}

#[tokio::test]
async fn the_scan_emits_labels_stoplists_refreshes_and_writes_no_entities() {
    let (db, conn) = open("test-org-candidate-edges.db").await;
    // Canonical pair sharing an N2 key (and its N3 twin — the double-
    // discovery dedupe fixture).
    org(&conn, 1, "Alfa GmbH", Some("A1")).await;
    org(&conn, 2, "Alfa GmbH", Some("A2")).await;
    for id in [1i64, 2] {
        key_row(&conn, id, "n2", "alfa gmbh").await;
        key_row(&conn, id, "n3", "alfa §gmbh").await;
    }
    // Provisional + canonical → emits; provisional-only → never.
    org(&conn, 3, "Beta Stadtwerke", None).await;
    org(&conn, 4, "Beta Stadtwerke", Some("B1")).await;
    key_row(&conn, 3, "n2", "beta stadtwerke").await;
    key_row(&conn, 4, "n2", "beta stadtwerke").await;
    org(&conn, 5, "Gamma Verein", None).await;
    org(&conn, 6, "Gamma Verein", None).await;
    key_row(&conn, 5, "n2", "gamma verein").await;
    key_row(&conn, 6, "n2", "gamma verein").await;
    // The contamination shape (org 23294544's): org 10 carries a foreign
    // satellite name equal to org 11's head name — head-vs-foreign-
    // satellite reach labels e3-xlang.
    org(&conn, 10, "PostAuto AG", Some("P1")).await;
    org(&conn, 11, "CarPostal SA", Some("P2")).await;
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm) \
         VALUES (10, 'FRA', 'CarPostal SA', 'carpostal sa')",
        (),
    )
    .await
    .unwrap();
    key_row(&conn, 10, "n2", "postauto ag").await;
    key_row(&conn, 10, "n2", "carpostal sa").await;
    key_row(&conn, 11, "n2", "carpostal sa").await;
    // Stale keys: both orgs renamed since the build — no name reaches the
    // shared key any more, so there is no evidence and no edge.
    org(&conn, 401, "New Name One", Some("S1")).await;
    org(&conn, 402, "New Name Two", Some("S2")).await;
    key_row(&conn, 401, "n2", "old name").await;
    key_row(&conn, 402, "n2", "old name").await;
    // Dangling member: 302 merged away since the build (no org row) — the
    // group's live half is one org, so nothing pairs.
    org(&conn, 301, "Ghost Co", Some("G1")).await;
    key_row(&conn, 301, "n2", "ghost co").await;
    key_row(&conn, 302, "n2", "ghost co").await;
    // A 21-org generic key (cap 20): counted, sampled, never emitted.
    for id in 100..121 {
        key_row(&conn, id, "n2", "gymnazium").await;
    }
    let changes_before = count(&conn, "SELECT COUNT(*) FROM changes").await;
    let orgs_before = count(&conn, "SELECT COUNT(*) FROM organizations").await;
    let names_before = count(&conn, "SELECT COUNT(*) FROM organization_names").await;
    let stop = || false;
    let progress = |_: u64, _: &str| {};

    // DRY census: the whole story, nothing written — and the exemplar
    // probe answers from the WOULD-EMIT set (the pre-wet review must not
    // be vacuous on an empty edge table).
    let mut a = args(true, &stop, &progress);
    a.exemplar_org = Some(10);
    let r = db.scan_org_match_keys(a, 1000).await.unwrap();
    assert_eq!(r.keys_walked, 9, "8 n2 keys + 1 n3 key");
    assert_eq!(r.groups_ge2, 8);
    assert_eq!(r.would_emit, 3, "alfa, beta, carpostal — n3 re-finds alfa, dedupes");
    assert_eq!((r.e3_name, r.e3_xlang), (2, 1));
    assert_eq!(r.groups_emitting, 3);
    assert_eq!(r.provisional_only_groups, 1, "gamma verein never pairs");
    assert_eq!(r.pairs_considered, 5, "alfa n2 + beta + carpostal + stale + alfa n3");
    assert_eq!(r.stale_pairs, 1);
    assert_eq!(r.dangling_members, 1);
    assert_eq!((r.stoplist_skipped_n2, r.stoplist_skipped_n3), (1, 0));
    assert_eq!(r.stoplist_top, vec![("gymnazium".to_owned(), 21)]);
    assert_eq!(r.max_group, 21);
    assert!(r.bounds_ok && !r.capped && !r.stopped);
    assert_eq!(r.edges_written, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 0);
    assert_eq!((r.exemplar_edges, r.exemplar_peers), (1, vec![11]));
    assert_eq!(r.exemplar_rules, vec!["e3-xlang".to_owned()]);

    // T4 parity: a census that moved beyond max(2%, 500) from the recorded
    // plan aborts before any write.
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(5000);
    assert!(db.scan_org_match_keys(a, 1001).await.is_err());
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 0);

    // Capped wet: the census stays whole (parity passes against 3), the
    // WRITE stops at the cap — a stable walk-order prefix.
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(3);
    a.max_edges = Some(1);
    let r = db.scan_org_match_keys(a, 1002).await.unwrap();
    assert!(r.capped);
    assert_eq!((r.would_emit, r.edges_written, r.edges_new), (3, 1, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 1);

    // Full wet: refreshes the capped prefix, writes the rest.
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(3);
    let r = db.scan_org_match_keys(a, 1003).await.unwrap();
    assert!(!r.capped);
    assert_eq!((r.edges_written, r.edges_new, r.edges_refreshed), (3, 2, 1));
    assert_eq!(r.total_edges_after, 3);
    {
        let mut rows = conn
            .query(
                "SELECT org_a, org_b, rule, tier, score, evidence, state, first_seen, last_seen \
                   FROM org_candidate_edges WHERE org_a = 10",
                (),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().expect("the xlang edge");
        assert_eq!(row.get_value(0).unwrap(), Value::Integer(10), "org_a < org_b");
        assert_eq!(row.get_value(1).unwrap(), Value::Integer(11));
        assert_eq!(row.get_value(2).unwrap(), Value::Text("e3-xlang".into()));
        assert_eq!(row.get_value(3).unwrap(), Value::Text("E3".into()));
        assert_eq!(row.get_value(4).unwrap(), Value::Real(0.9));
        let Value::Text(ev) = row.get_value(5).unwrap() else { panic!("evidence") };
        for frag in [
            "\"kind\":\"n2\"",
            "\"key\":\"carpostal sa\"",
            "\"src\":\"lang:FRA\"",
            "\"src\":\"head\"",
            "\"name\":\"CarPostal SA\"",
        ] {
            assert!(ev.contains(frag), "evidence lacks {frag}: {ev}");
        }
        assert_eq!(row.get_value(6).unwrap(), Value::Text("open".into()));
        assert_eq!(row.get_value(8).unwrap(), Value::Integer(1003));
    }

    // Refresh semantics: a later scan bumps last_seen and evidence but
    // preserves first_seen AND a review decision someone recorded.
    conn.execute(
        "UPDATE org_candidate_edges SET state = 'approved' WHERE org_a = 1 AND org_b = 2",
        (),
    )
    .await
    .unwrap();
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(3);
    let r = db.scan_org_match_keys(a, 2000).await.unwrap();
    assert_eq!((r.edges_written, r.edges_new, r.edges_refreshed), (3, 0, 3));
    {
        let mut rows = conn
            .query(
                "SELECT state, first_seen, last_seen FROM org_candidate_edges \
                  WHERE org_a = 1 AND org_b = 2",
                (),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get_value(0).unwrap(), Value::Text("approved".into()));
        assert_eq!(row.get_value(1).unwrap(), Value::Integer(1002), "first_seen survives");
        assert_eq!(row.get_value(2).unwrap(), Value::Integer(2000), "last_seen bumps");
    }

    // Honest cancel: a stopped census plans nothing and writes nothing
    // more; the caller records no report either way.
    let cancel = || true;
    let r = db.scan_org_match_keys(args(false, &cancel, &progress), 3000).await.unwrap();
    assert!(r.stopped);
    assert_eq!((r.would_emit, r.edges_written), (0, 0));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 3);

    // The no-entity-writes contract (decision 6): no change rows at all,
    // no cursor movement, entity tables untouched, no merges.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes").await,
        changes_before,
        "no change rows at all"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, orgs_before);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_names").await, names_before);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log").await, 0);
}

#[tokio::test]
async fn a_key_filling_the_whole_page_is_stoplisted_and_stepped_past() {
    let (db, conn) = open("test-org-edge-fills-page.db").await;
    // Key 'aaa' holds MORE rows than the whole page: no complete trailing
    // group to trim, so without the guard the watermark could never
    // advance — the infinite-loop-on-the-serving-box defect. 'bbb' after
    // it proves the walk continues.
    for id in 1..=5 {
        key_row(&conn, id, "n2", "aaa").await;
    }
    org(&conn, 201, "Bbb", Some("B1")).await;
    org(&conn, 202, "Bbb", Some("B2")).await;
    key_row(&conn, 201, "n2", "bbb").await;
    key_row(&conn, 202, "n2", "bbb").await;
    let stop = || false;
    let progress = |_: u64, _: &str| {};
    let mut a = args(true, &stop, &progress);
    a.key_window = 4;
    a.stoplist_cap = 3;
    let r = db.scan_org_match_keys(a, 1000).await.unwrap();
    assert_eq!(r.stoplist_skipped_n2, 1, "the page-filling key is a generic name");
    assert_eq!(r.stoplist_top, vec![("aaa".to_owned(), 5)], "counted ONCE, in full");
    assert_eq!(r.max_group, 5);
    assert_eq!(r.would_emit, 1, "the walk stepped past it and found bbb");
    assert_eq!(r.keys_walked, 2);
}

#[tokio::test]
async fn chase_merged_org_follows_the_loser_chain_to_the_standing_id() {
    let (db, conn) = open("test-org-edge-chase.db").await;
    for (keep, loser) in [(11i64, 99i64), (10, 11)] {
        conn.execute(
            "INSERT INTO org_merge_log (keep, loser, rule, evidence, job_id, at) \
             VALUES (?, ?, 'r2', '{}', NULL, 5)",
            (Value::Integer(keep), Value::Integer(loser)),
        )
        .await
        .unwrap();
    }
    assert_eq!(db.chase_merged_org(99).await.unwrap(), 10, "two hops");
    assert_eq!(db.chase_merged_org(11).await.unwrap(), 10, "one hop");
    assert_eq!(db.chase_merged_org(10).await.unwrap(), 10, "standing id resolves to itself");
    assert_eq!(db.chase_merged_org(7777).await.unwrap(), 7777, "never merged");
}
