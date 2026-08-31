//! Issue 300 Stage 4 Unit 4: the candidate-edge scan — E3 emission rules
//! (canonical×canonical and provisional×canonical only), the e3-name /
//! e3-xlang split from reach provenance, the >cap stoplist with its top
//! sample, the same-key-fills-page walk-termination guard, refresh
//! semantics preserving first_seen/state, T4 parity aborts, honest cancel,
//! and the no-entity-writes contract. Key fns are test-local stand-ins;
//! the real N2/N3 content is pinned by ingest's own tests.

use store::turso::Value;

/// A miniature checksum probe for the packet's anchor evidence: 9-digit values
/// are Norwegian org numbers, 8-digit ones Czech ICO. Enough to tell "the value
/// only works as a NO number while the row claims DK" from "the value works
/// where the row says it is".
fn packet_anchors(value: &str) -> Vec<(&'static str, String)> {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) {
        return Vec::new();
    }
    match digits.len() {
        9 => vec![("NO:orgnr", digits)],
        8 => vec![("CZ:ico", digits)],
        _ => Vec::new(),
    }
}

/// What the fake probe ASKS about, pass or fail — the store's second injected
/// function (issue 314). Faithful to the real one's decisive structure: the
/// arms are DISJOINT BY LENGTH, so a country whose scheme lives at another
/// length is never tested and its silence carries no information. `DK:cvr`
/// sits in the 8-digit arm exactly as it does in production, which is why a
/// 9-digit value on a DK row is unprobed rather than contradicted.
fn packet_vocabulary(value: &str) -> Vec<&'static str> {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) {
        return Vec::new();
    }
    match digits.len() {
        9 => vec!["NO:orgnr", "PT:nif"],
        8 => vec!["CZ:ico", "DK:cvr"],
        _ => Vec::new(),
    }
}

/// Issue 314 step 2: the census classifies cross-border components by whether
/// their member names normalize alike, so it takes a norm. Case- and
/// punctuation-insensitive, like the real one.
fn census_norm(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.extend(c.to_lowercase());
        } else {
            gap = true;
        }
    }
    out
}

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

    // Unit 5's acceptance surfaces: the exemplar probe reports the TABLE
    // states of touching edges (a review decision showing up here is a
    // finding), and the tripwire-6 baseline round-trips durably.
    let mut a = args(true, &stop, &progress);
    a.exemplar_org = Some(1);
    let r = db.scan_org_match_keys(a, 2500).await.unwrap();
    assert_eq!(r.exemplar_states, vec!["approved".to_owned()], "the seeded review surfaces");
    assert_eq!(db.org_edge_baseline().await.unwrap(), 0);
    db.set_org_edge_baseline(7).await.unwrap();
    assert_eq!(db.org_edge_baseline().await.unwrap(), 7);

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

/// Issue 314: the edge census. Components (not edges) are review cases, so
/// the census counts them over LIVE endpoints only, splits them by whether
/// a merge verdict could ever act on them, and notices when a component
/// spans countries.
#[tokio::test]
async fn the_edge_census_counts_components_not_edges() {
    let (db, conn) = open("test-org-edge-census.db").await;
    // Component 1: a canonical CHAIN of three (1-2, 2-3) — one case, not two.
    org(&conn, 1, "Alfa", Some("A1")).await;
    org(&conn, 2, "Alfa", Some("A2")).await;
    org(&conn, 3, "Alfa", Some("A3")).await;
    // Component 2: mixed canonical + provisional, and it spans countries.
    org(&conn, 4, "Beta", Some("B1")).await;
    org(&conn, 5, "Beta", None).await;
    conn.execute("UPDATE organizations SET country = 'FR' WHERE id = 5", ()).await.unwrap();
    // Component 3: provisional-only — nothing a merge verdict could act on.
    org(&conn, 6, "Gamma", None).await;
    org(&conn, 7, "Gamma", None).await;
    // An edge to an org that has since been merged away: NOT a component.
    org(&conn, 8, "Delta", Some("D1")).await;
    for (a, b, rule) in [
        (1i64, 2i64, "e3-name"),
        (2, 3, "e3-name"),
        (4, 5, "e3-xlang"),
        (6, 7, "e3-name"),
        (8, 9999, "e3-name"),
    ] {
        conn.execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
             VALUES (?, ?, ?, 'E3', 1.0, '{}', 1, 1, NULL)",
            (Value::Integer(a), Value::Integer(b), Value::Text(rule.into())),
        )
        .await
        .unwrap();
    }
    let never = || false;
    let r = db.census_org_candidate_edges(1, census_norm, &never).await.unwrap();
    assert_eq!((r.edges, r.e3_name, r.e3_xlang), (5, 4, 1));
    assert_eq!(r.orgs_touched, 9, "8 live rows + the merged-away 9999 endpoint");
    assert_eq!(r.dangling_orgs, 1, "…and as dangling, since its row is gone");
    assert_eq!(r.components, 3, "the 8-9999 edge is no component; the 1-2-3 chain is ONE");
    assert_eq!(r.max_component, 3);
    assert_eq!(
        (r.canonical_only_components, r.mixed_components, r.provisional_only_components),
        (1, 1, 1)
    );
    assert_eq!(r.multi_country_components, 1, "beta spans DE and FR");
    assert_eq!(r.country_pairs, vec![("DE-FR".to_owned(), 1)]);
    assert_eq!(r.null_country_components, 0, "no country-less members in this fixture");
    let two = r.size_buckets.iter().find(|(k, _)| *k == "2").unwrap().1;
    let three_five = r.size_buckets.iter().find(|(k, _)| *k == "3-5").unwrap().1;
    assert_eq!((two, three_five), (2, 1));
    assert_eq!(r.sample.len(), 3, "sample_every=1 shows every component");
    // The cohort is the INTERSECTION, not the smaller half: this fixture has
    // one canonical-only component AND one cross-border component, and they
    // are DIFFERENT components, so the cohort is empty.
    assert_eq!(
        r.canonical_cross_border_components, 0,
        "the canonical-only chain is single-country; the cross-border pair is mixed"
    );
    assert!(r.canonical_cross_border_pairs.is_empty());
    assert!(r.canonical_cross_border_sample.is_empty());

    // A country-less member must NOT read as cross-border — the defect the
    // first prod census shipped with, where every "??-XX" pair was just a
    // country-less row beside a known one.
    org(&conn, 10, "Epsilon", Some("E1")).await;
    org(&conn, 11, "Epsilon", Some("E2")).await;
    conn.execute("UPDATE organizations SET country = NULL WHERE id = 11", ()).await.unwrap();
    conn.execute(
        "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
         VALUES (10, 11, 'e3-name', 'E3', 1.0, '{}', 1, 1, NULL)",
        (),
    )
    .await
    .unwrap();
    let r2 = db.census_org_candidate_edges(1, census_norm, &never).await.unwrap();
    assert_eq!(r2.components, 4);
    assert_eq!(r2.null_country_components, 1, "the country-less pair is counted here…");
    assert_eq!(r2.multi_country_components, 1, "…and NOT as a second cross-border component");
    assert_eq!(r2.country_pairs, vec![("DE-FR".to_owned(), 1)], "still only the real pair");

    // Now one component that IS in the cohort: both members keyed, two known
    // countries. It is the slice the first review campaign draws from.
    org(&conn, 12, "Zeta", Some("Z1")).await;
    org(&conn, 13, "Zeta", Some("Z2")).await;
    conn.execute("UPDATE organizations SET country = 'AT' WHERE id = 13", ()).await.unwrap();
    conn.execute(
        "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
         VALUES (12, 13, 'e3-name', 'E3', 1.0, '{}', 1, 1, NULL)",
        (),
    )
    .await
    .unwrap();
    let r3 = db.census_org_candidate_edges(1, census_norm, &never).await.unwrap();
    assert_eq!(
        r3.canonical_only_components, 3,
        "the DE chain, the country-less epsilon pair (keyed, one KNOWN country), \
         and the new AT-DE pair"
    );
    assert_eq!(r3.multi_country_components, 2, "beta (DE-FR) and zeta (AT-DE)");
    assert_eq!(
        r3.canonical_cross_border_components, 1,
        "only zeta is in both halves — epsilon is keyed but its '??' member is \
         not a second country, and beta is cross-border but half provisional"
    );
    assert_eq!(r3.canonical_cross_border_pairs, vec![("AT-DE".to_owned(), 1)]);
    assert_eq!(
        r3.canonical_cross_border_sample,
        vec![(12, 2, vec!["AT:Zeta".to_owned(), "DE:Zeta".to_owned()])],
        "the cohort sample carries the members a reviewer needs to see"
    );

    // Read-only: the census writes nothing at all. (Seven edges now — the
    // five seeded above, the country-less pair, and the cohort pair.)
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 7);

    // Cancel is honest: no partial numbers.
    let always = || true;
    let stopped = db.census_org_candidate_edges(1, census_norm, &always).await.unwrap();
    assert!(stopped.stopped);
    assert_eq!((stopped.edges, stopped.components), (0, 0));
}

/// Issue 314 step 2: the "canonical cross-border" cohort is not one cohort.
///
/// Reading its live sample showed four classes with four different right
/// answers, and a single merge/distinct rubric written for the first would be
/// actively wrong on the second. The census splits them now, so a pilot gets a
/// cohort one rubric can serve.
#[tokio::test]
async fn the_cross_border_cohort_splits_into_its_three_shapes() {
    let (db, conn) = open("test-org-edge-xb-split.db").await;

    // SAME-NAME: one entity under three country codes. The merge-candidate
    // class — the live specimen is `Mercell Holding ASA` as DK, LT and NO.
    for (id, cc) in [(1i64, "DK"), (2, "LT"), (3, "NO")] {
        org(&conn, id, "Mercell Holding ASA", Some(&format!("M{id}"))).await;
        conn.execute(
            "UPDATE organizations SET country = ? WHERE id = ?",
            (Value::Text(cc.into()), Value::Integer(id)),
        )
        .await
        .unwrap();
    }

    // DIFF-NAME: corporate siblings in different jurisdictions. Separate legal
    // entities that must NOT merge — the live specimen is `Steelco Belimed
    // GmbH` (AT) beside `Belimed GmbH` (DE). This is the class that costs
    // something to get wrong, which is why it must not share a rubric with the
    // one above.
    org(&conn, 4, "Steelco Belimed GmbH", Some("B-AT")).await;
    org(&conn, 5, "Belimed GmbH", Some("B-DE")).await;
    conn.execute("UPDATE organizations SET country = 'AT' WHERE id = 4", ()).await.unwrap();
    conn.execute("UPDATE organizations SET country = 'DE' WHERE id = 5", ()).await.unwrap();

    // WITH-INTRA: a same-country duplicate riding inside a "cross-border"
    // component — the live specimen is `Merck Life Science` three times under
    // CZ plus one SK sibling. The intra-country question is different, and
    // easier, and should be settled first.
    for (id, cc) in [(6i64, "CZ"), (7, "CZ"), (8, "SK")] {
        org(&conn, id, "Merck Life Science spol. s r.o.", Some(&format!("K{id}"))).await;
        conn.execute(
            "UPDATE organizations SET country = ? WHERE id = ?",
            (Value::Text(cc.into()), Value::Integer(id)),
        )
        .await
        .unwrap();
    }

    for (a, b) in [(1i64, 2i64), (2, 3), (4, 5), (6, 7), (7, 8)] {
        conn.execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
             VALUES (?, ?, 'e3-name', 'E3', 1.0, '{}', 1, 1, NULL)",
            (Value::Integer(a), Value::Integer(b)),
        )
        .await
        .unwrap();
    }

    let never = || false;
    let r = db.census_org_candidate_edges(1, census_norm, &never).await.unwrap();
    assert_eq!(r.canonical_cross_border_components, 3, "all three are the cohort");
    assert_eq!(r.xb_same_name, 1, "Mercell: countries distinct, one name");
    assert_eq!(r.xb_diff_name, 1, "Belimed: countries distinct, names differ — do NOT merge");
    assert_eq!(r.xb_with_intra, 1, "Merck: two members share CZ");
    assert_eq!(
        r.xb_same_name + r.xb_diff_name + r.xb_with_intra,
        r.canonical_cross_border_components,
        "the split is a partition — every cohort component lands in exactly one class"
    );

    // The sample carries the class, so a reviewer can read one of each without
    // re-deriving the classification by hand.
    let classes: std::collections::BTreeSet<&str> =
        r.xb_class_sample.iter().map(|(c, ..)| c.as_str()).collect();
    assert_eq!(
        classes.into_iter().collect::<Vec<_>>(),
        vec!["diff-name", "same-name", "with-intra"]
    );

    // Case differences must not split a same-name component: the real corpus
    // pairs `SARSTEDT spol. s r.o.` with `Sarstedt spol. s r.o.`, and counting
    // that as diff-name would put a plain duplicate in the dangerous class.
    let (db2, conn2) = open("test-org-edge-xb-case.db").await;
    org(&conn2, 1, "SARSTEDT spol. s r.o.", Some("S1")).await;
    org(&conn2, 2, "Sarstedt spol. s r.o.", Some("S2")).await;
    conn2.execute("UPDATE organizations SET country = 'CZ' WHERE id = 1", ()).await.unwrap();
    conn2.execute("UPDATE organizations SET country = 'SK' WHERE id = 2", ()).await.unwrap();
    conn2
        .execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
             VALUES (1, 2, 'e3-name', 'E3', 1.0, '{}', 1, 1, NULL)",
            (),
        )
        .await
        .unwrap();
    let r2 = db2.census_org_candidate_edges(1, census_norm, &never).await.unwrap();
    assert_eq!(r2.xb_same_name, 1, "case alone is not a different name");
    assert_eq!(r2.xb_diff_name, 0);
}

/// Issues 311 + 314: the same-name packet is the first thing to CONSUME
/// `org_candidate_edges`. It must select exactly the census's same-name class
/// — the two definitions cannot be allowed to drift, or the packet reviews a
/// cohort whose size nobody measured.
#[tokio::test]
async fn the_packet_carries_only_the_same_name_class_with_its_evidence() {
    let (db, conn) = open("test-xb-packet.db").await;

    // IN: same name, distinct countries, both canonical.
    for (id, cc) in [(1i64, "DK"), (2, "NO")] {
        org(&conn, id, "Mercell Holding ASA", Some(&format!("M{id}"))).await;
        conn.execute(
            "UPDATE organizations SET country = ? WHERE id = ?",
            (Value::Text(cc.into()), Value::Integer(id)),
        )
        .await
        .unwrap();
    }
    // OUT: names differ (the sibling-risk class).
    org(&conn, 3, "Steelco Belimed GmbH", Some("B-AT")).await;
    org(&conn, 4, "Belimed GmbH", Some("B-DE")).await;
    conn.execute("UPDATE organizations SET country = 'AT' WHERE id = 3", ()).await.unwrap();
    conn.execute("UPDATE organizations SET country = 'DE' WHERE id = 4", ()).await.unwrap();
    // OUT: two members share a country (with-intra).
    for (id, cc) in [(5i64, "CZ"), (6, "CZ"), (7, "SK")] {
        org(&conn, id, "Merck Life Science spol. s r.o.", Some(&format!("K{id}"))).await;
        conn.execute(
            "UPDATE organizations SET country = ? WHERE id = ?",
            (Value::Text(cc.into()), Value::Integer(id)),
        )
        .await
        .unwrap();
    }
    // OUT: one side provisional — no identifier for a merge verdict to act on.
    org(&conn, 8, "Provisional Pair", Some("P1")).await;
    org(&conn, 9, "Provisional Pair", None).await;
    conn.execute("UPDATE organizations SET country = 'FI' WHERE id = 8", ()).await.unwrap();
    conn.execute("UPDATE organizations SET country = 'SE' WHERE id = 9", ()).await.unwrap();

    for (a, b) in [(1i64, 2i64), (3, 4), (5, 6), (6, 7), (8, 9)] {
        conn.execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
             VALUES (?, ?, 'e3-name', 'E3', 1.0, '{}', 1, 1, NULL)",
            (Value::Integer(a), Value::Integer(b)),
        )
        .await
        .unwrap();
    }
    // Evidence for the one case that qualifies: a variant on one side and
    // an uneven mention spread, which is the reviewer's first read.
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm)
         VALUES (1, 'NOR', 'Mercell Holding AS', 'mercell holding as')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
        (),
    )
    .await
    .unwrap();
    for n in 1..=4i64 {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', 'pub-' || ?, 'h' || ?, 'eforms', 1, 'm', 0, 'parsed', 1)",
            (Value::Integer(n), Value::Integer(n), Value::Integer(n)),
        )
        .await
        .unwrap();
    }
    // Org 1 heavy (3 mentions), org 2 light (1) — a stray duplicate's shape.
    for (n, o) in [(1i64, 1i64), (2, 1), (3, 1), (4, 2)] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'ORG-' || ?, ?, 'Mercell Holding ASA', 'DK', NULL)",
            (Value::Integer(n), Value::Integer(n), Value::Integer(o)),
        )
        .await
        .unwrap();
    }

    let never = || false;
    let p = db.xb_same_name_packet(census_norm, packet_anchors, packet_vocabulary, 600, 3, &never).await.unwrap();
    assert_eq!(p.cohort, 1, "only the Mercell component qualifies");
    assert_eq!(p.cases.len(), 1);
    assert!(!p.truncated);

    let c = &p.cases[0];
    assert_eq!(c.size, 2);
    assert_eq!(c.key, "mercell holding asa");
    assert_eq!(c.countries, vec!["DK".to_owned(), "NO".to_owned()]);
    assert_eq!(c.members.len(), 2);

    let m1 = &c.members[0];
    assert_eq!(m1.org, 1);
    assert_eq!(m1.country.as_deref(), Some("DK"));
    assert_eq!(m1.identifier.as_deref(), Some("M1"));
    assert_eq!(m1.mentions, 3, "the heavy side");
    assert_eq!(m1.variants, vec![("NOR".to_owned(), "Mercell Holding AS".to_owned())]);
    assert_eq!(m1.notices.len(), 3, "capped at the notices_cap");

    let m2 = &c.members[1];
    assert_eq!((m2.org, m2.mentions), (2, 1), "the light side — a stray duplicate's shape");
    assert!(m2.variants.is_empty());

    // The anchor evidence, which is what decides whether the repair is a merge
    // or a COUNTRY correction. Both rows carry "M1"/"M2" — letter-bearing, so
    // the probe declines them and `country_agrees` stays false without
    // implying contamination. The distinction the packet has to preserve is
    // "anchors nowhere" versus "anchors somewhere ELSE".
    assert!(m1.anchors.is_empty(), "a letter-bearing value anchors nowhere");
    assert!(!m1.country_agrees);
    assert!(
        !m1.country_probed,
        "and the probe declined the value wholesale, so no country was asked \
         about — `country_agrees == false` here says nothing about DK"
    );

    // The cap is honoured and reported, not silently applied.
    let capped = db.xb_same_name_packet(census_norm, packet_anchors, packet_vocabulary, 0, 3, &never).await.unwrap();
    assert_eq!(capped.cohort, 1, "the cohort size is the WHOLE class, not the page");
    assert!(capped.truncated);
    assert!(capped.cases.is_empty());

    // A cancel carries nothing: a partial packet reviewed as a whole one would
    // under-run the campaign silently.
    let always = || true;
    let stopped = db.xb_same_name_packet(census_norm, packet_anchors, packet_vocabulary, 600, 3, &always).await.unwrap();
    assert!(stopped.stopped);
    assert_eq!(stopped.cohort, 0);
    assert!(stopped.cases.is_empty());
}

/// The packet's anchor evidence has to separate THREE states, not two, and the
/// campaign's first 100 verdicts are why. `country_agrees == false` was read as
/// "the row's country is contaminated" in every case, but it also fires when the
/// probe never asked about that country at all — and 6 of 100 reviews went into
/// dispute on exactly that reading (issue 314). So the packet reports what was
/// ASKED beside what PASSED:
///
///   * agrees            — the arithmetic works where the row says it is.
///   * probed, disagrees — tested under the row's own register and refused,
///                         accepted under another's. THIS is a contaminated
///                         country code, and the repair is a country
///                         correction through an already-tested path rather
///                         than a new merge arm.
///   * not probed        — no scheme of the row's country has this value's
///                         shape. Silence with no content.
#[tokio::test]
async fn the_packet_separates_a_contradicted_country_from_an_unprobed_one() {
    let (db, conn) = open("test-xb-anchor.db").await;
    // A 9-digit value: Norwegian by arithmetic. NO agrees. PT is in the
    // 9-digit arm and refused it, so PT is contradicted. DK's scheme is
    // 8-digit, so DK was never asked — its silence proves nothing.
    for (id, cc) in [(1i64, "NO"), (2, "PT"), (3, "DK")] {
        org(&conn, id, "Mercell Holding ASA", Some("980921565")).await;
        conn.execute(
            "UPDATE organizations SET country = ? WHERE id = ?",
            (Value::Text(cc.into()), Value::Integer(id)),
        )
        .await
        .unwrap();
    }
    for (a, b) in [(1i64, 2i64), (2, 3)] {
        conn.execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
             VALUES (?, ?, 'e3-name', 'E3', 1.0, '{}', 1, 1, NULL)",
            (Value::Integer(a), Value::Integer(b)),
        )
        .await
        .unwrap();
    }

    let never = || false;
    let p =
        db.xb_same_name_packet(census_norm, packet_anchors, packet_vocabulary, 600, 3, &never)
            .await
            .unwrap();
    assert_eq!(p.cases.len(), 1);
    let ms = &p.cases[0].members;
    let at = |cc: &str| ms.iter().find(|m| m.country.as_deref() == Some(cc)).unwrap();

    let no = at("NO");
    assert_eq!(no.anchors, vec!["NO:orgnr".to_owned()]);
    assert!(no.country_probed);
    assert!(no.country_agrees, "the value's arithmetic works where this row says it is");

    let pt = at("PT");
    assert_eq!(pt.anchors, vec!["NO:orgnr".to_owned()], "same value, same arithmetic");
    assert!(pt.country_probed, "PT:nif is in the 9-digit arm — it WAS asked");
    assert!(
        !pt.country_agrees,
        "…and it refused, so PT is the contaminated side and the repair is a \
         country correction rather than a merge"
    );

    let dk = at("DK");
    assert_eq!(dk.anchors, vec!["NO:orgnr".to_owned()], "same value again");
    assert!(
        !dk.country_probed,
        "DK's scheme is 8-digit, so a 9-digit value was never tested as DK"
    );
    assert!(
        !dk.country_agrees,
        "so this false is the UNINFORMATIVE one — identical to PT's on the old \
         shape, and the pair is the whole reason `country_probed` exists"
    );
}
