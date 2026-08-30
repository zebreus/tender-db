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
    // The mini canonicalizer: TWO form variants collapse into one family
    // marker, like the real FAMILY_SEQUENCES table — so two names can
    // share an n3 key while their n2 keys differ (the novel-pair case the
    // n3 walk exists for).
    n2(s)
        .split(' ')
        .map(|t| if t == "gmbh" || t == "gesmbh" { "§gmbh" } else { t })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Full-content snapshot of an entity table — COUNT parity alone cannot
/// catch an in-place UPDATE, which is exactly the regression class the
/// no-entity-writes contract guards against.
async fn snapshot(conn: &store::turso::Connection, sql: &str) -> Vec<String> {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let mut line = String::new();
        for i in 0..8 {
            match row.get_value(i) {
                Ok(v) => line.push_str(&format!("{v:?}|")),
                Err(_) => break,
            }
        }
        out.push(line);
    }
    out
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
    // The NOVEL n3 pair: two form VARIANTS of one family — n2 keys differ,
    // n3 keys agree, so ONLY the n3 walk can find the pair.
    org(&conn, 20, "Delta GmbH", Some("D1")).await;
    org(&conn, 21, "Delta GesmbH", Some("D2")).await;
    key_row(&conn, 20, "n2", "delta gmbh").await;
    key_row(&conn, 21, "n2", "delta gesmbh").await;
    key_row(&conn, 20, "n3", "delta §gmbh").await;
    key_row(&conn, 21, "n3", "delta §gmbh").await;
    // Satellite-satellite, SAME lang → e3-name with lang provenance.
    org(&conn, 30, "Thirty Head", Some("T1")).await;
    org(&conn, 31, "ThirtyOne Head", Some("T2")).await;
    for id in [30i64, 31] {
        conn.execute(
            "INSERT INTO organization_names (org_id, lang, name, name_norm) \
             VALUES (?, 'ENG', 'Shared Alias', 'shared alias')",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
        key_row(&conn, id, "n2", "shared alias").await;
    }
    // Satellite-satellite, DIFFERENT langs → e3-xlang.
    org(&conn, 40, "Forty Head", Some("F1")).await;
    org(&conn, 41, "FortyOne Head", Some("F2")).await;
    for (id, lang) in [(40i64, "DEU"), (41, "FRA")] {
        conn.execute(
            "INSERT INTO organization_names (org_id, lang, name, name_norm) \
             VALUES (?, ?, 'Autre Nom', 'autre nom')",
            (Value::Integer(id), Value::Text(lang.into())),
        )
        .await
        .unwrap();
        key_row(&conn, id, "n2", "autre nom").await;
    }
    // The peer-aware pick (panel catch): org 50 reaches under DEU **and**
    // ENG, org 51 under ENG only — a same-language witness pair exists, so
    // the edge MUST be e3-name on lang:ENG, never e3-xlang on an arbitrary
    // first-reaching row.
    org(&conn, 50, "Grosse Bank Aktiengesellschaft", Some("G5")).await;
    org(&conn, 51, "Fifty One Head", Some("G6")).await;
    for lang in ["DEU", "ENG"] {
        conn.execute(
            "INSERT INTO organization_names (org_id, lang, name, name_norm) \
             VALUES (50, ?, 'Grosse Bank AG', 'grosse bank ag')",
            (Value::Text(lang.into()),),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm) \
         VALUES (51, 'ENG', 'Grosse Bank AG', 'grosse bank ag')",
        (),
    )
    .await
    .unwrap();
    key_row(&conn, 50, "n2", "grosse bank ag").await;
    key_row(&conn, 51, "n2", "grosse bank ag").await;

    let changes_before = count(&conn, "SELECT COUNT(*) FROM changes").await;
    let cursor_before = db.latest_cursor().await.unwrap();
    const ORG_SNAP: &str = "SELECT id, country, identifier_kind, identifier, name, name_norm, \
                            provisional, created_at FROM organizations ORDER BY id";
    const NAME_SNAP: &str =
        "SELECT org_id, lang, name, name_norm FROM organization_names ORDER BY org_id, lang";
    let orgs_snap = snapshot(&conn, ORG_SNAP).await;
    let names_snap = snapshot(&conn, NAME_SNAP).await;
    let stop = || false;
    let progress = |_: u64, _: &str| {};

    // DRY census: the whole story, nothing written — and the exemplar
    // probe answers from the WOULD-EMIT set (the pre-wet review must not
    // be vacuous on an empty edge table).
    let mut a = args(true, &stop, &progress);
    a.exemplar_org = Some(10);
    let r = db.scan_org_match_keys(a, 1000).await.unwrap();
    assert_eq!(r.keys_walked, 15, "13 n2 keys + 2 n3 keys");
    assert_eq!(r.groups_ge2, 12);
    assert_eq!(
        r.would_emit, 7,
        "alfa, beta, carpostal, autre, shared, grosse via n2 + the NOVEL delta pair via n3"
    );
    assert_eq!((r.e3_name, r.e3_xlang), (5, 2));
    assert_eq!(r.groups_emitting, 7);
    assert_eq!(r.provisional_only_groups, 1, "gamma verein never pairs");
    assert_eq!(r.pairs_considered, 9, "7 n2 pairs (stale incl.) + alfa n3 re-find + delta n3");
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
    assert_eq!(r.sample.len(), 7, "a census this small samples every edge");
    assert!(
        r.sample.iter().all(|(a, b, _, ev)| a < b && ev.starts_with('{')),
        "sample rows carry ordered ids and evidence JSON"
    );

    // T4 parity: a census that moved beyond max(2%, 500) from the recorded
    // plan aborts before any write.
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(5000);
    assert!(db.scan_org_match_keys(a, 1001).await.is_err());
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 0);

    // Capped wet: the census stays whole (parity passes against 7), the
    // WRITE stops at the cap — a stable walk-order prefix.
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(7);
    a.max_edges = Some(1);
    let r = db.scan_org_match_keys(a, 1002).await.unwrap();
    assert!(r.capped);
    assert_eq!((r.would_emit, r.edges_written, r.edges_new), (7, 1, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 1);

    // Full wet: refreshes the capped prefix, writes the rest.
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(7);
    let r = db.scan_org_match_keys(a, 1003).await.unwrap();
    assert!(!r.capped);
    assert_eq!((r.edges_written, r.edges_new, r.edges_refreshed), (7, 6, 1));
    assert_eq!(r.total_edges_after, 7);
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
    // The other labeled edges, each hand-verifiable from evidence alone.
    let edge = |a: i64, b: i64| {
        let conn = &conn;
        async move {
        let mut rows = conn
            .query(
                "SELECT rule, evidence FROM org_candidate_edges WHERE org_a = ? AND org_b = ?",
                (Value::Integer(a), Value::Integer(b)),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap_or_else(|| panic!("edge {a}-{b}"));
        let Value::Text(rule) = row.get_value(0).unwrap() else { panic!("rule") };
        let Value::Text(ev) = row.get_value(1).unwrap() else { panic!("evidence") };
        (rule, ev)
        }
    };
    let (rule, ev) = edge(20, 21).await;
    assert_eq!(rule, "e3-name", "form variants share the family key head-to-head");
    assert!(ev.contains("\"kind\":\"n3\""), "the delta pair is the n3 walk's novel find: {ev}");
    let (rule, ev) = edge(30, 31).await;
    assert_eq!(rule, "e3-name", "same-language satellite equality is e3-name");
    assert!(ev.matches("lang:ENG").count() == 2, "both witnesses name the shared lang: {ev}");
    let (rule, ev) = edge(40, 41).await;
    assert_eq!(rule, "e3-xlang");
    assert!(ev.contains("lang:DEU") && ev.contains("lang:FRA"), "{ev}");
    let (rule, ev) = edge(50, 51).await;
    assert_eq!(
        rule, "e3-name",
        "a same-language witness pair exists — the peer-aware pick must find it"
    );
    assert!(
        ev.matches("lang:ENG").count() == 2 && !ev.contains("lang:DEU"),
        "witnesses are the common lang, never an arbitrary first row: {ev}"
    );

    // Refresh semantics: a later scan bumps last_seen and evidence but
    // preserves first_seen AND a review decision someone recorded.
    conn.execute(
        "UPDATE org_candidate_edges SET state = 'approved' WHERE org_a = 1 AND org_b = 2",
        (),
    )
    .await
    .unwrap();
    let mut a = args(false, &stop, &progress);
    a.expect_edges = Some(7);
    let r = db.scan_org_match_keys(a, 2000).await.unwrap();
    assert_eq!((r.edges_written, r.edges_new, r.edges_refreshed), (7, 0, 7));
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
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 7);

    // The no-entity-writes contract (decision 6): no change rows at all,
    // no cursor movement, entity tables BYTE-identical (COUNT parity alone
    // would miss an in-place UPDATE), no merges.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes").await,
        changes_before,
        "no change rows at all"
    );
    assert_eq!(db.latest_cursor().await.unwrap(), cursor_before, "the cursor never moved");
    assert_eq!(snapshot(&conn, ORG_SNAP).await, orgs_snap, "organizations untouched");
    assert_eq!(snapshot(&conn, NAME_SNAP).await, names_snap, "organization_names untouched");
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
