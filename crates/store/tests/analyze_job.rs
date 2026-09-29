//! Issue 429: the weekly `analyze` job gathers statistics for exactly
//! `store::ANALYZE_TABLES`, leaves `organizations` without any (removing a stray
//! row a manual `ANALYZE` left), and the plans the decision rests on hold after
//! it runs — asserted on each statement's own text (issue 114's rule).

async fn exec(conn: &turso::Connection, sql: &str) {
    conn.execute(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

async fn plan(conn: &turso::Connection, sql: &str) -> String {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        out.push(r.get_value(3).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default());
    }
    out.join("\n")
}

async fn stat_tables(conn: &turso::Connection) -> Vec<String> {
    let mut rows = conn.query("SELECT DISTINCT tbl FROM sqlite_stat1 ORDER BY tbl", ()).await.unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        out.push(r.get_value(0).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default());
    }
    out
}

/// The resolver's provisional reuse lookup, verbatim from `canonical.rs`.
const RESOLVER: &str = "SELECT id FROM organizations \
  WHERE name_norm = 'stadt' AND country = 'DE' AND identifier IS NULL \
  ORDER BY id LIMIT 1";

/// Issue 421's plain-JOIN shape: a range of Tenders joined to a version table
/// with a predicate on the version table's own column.
const PLAIN_JOIN: &str = "SELECT t.id FROM tenders t JOIN tender_version_classifications c \
  ON c.tender_id = t.id AND c.seq = t.current_seq AND c.scheme = 'cpv' \
  WHERE t.id > 5000 AND t.id <= 6000";

#[tokio::test]
async fn the_job_analyzes_its_tables_keeps_organizations_bare_and_holds_the_plans() {
    let path = format!("/tmp/tender-db-429-analyze-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");

    // A skewed name distribution: one name carried by thousands of rows (428's
    // regression case), the rest near-unique.
    exec(&conn, "INSERT INTO organizations (name, name_norm, country, provisional, created_at) \
                 SELECT CASE WHEN value % 2 = 0 THEN 'stadt' ELSE 'org ' || value END, \
                        CASE WHEN value % 2 = 0 THEN 'stadt' ELSE 'org ' || value END, 'DE', 1, 0 \
                   FROM generate_series(1, 3000)").await;
    // 10,000 Tenders, three classifications each (two CPV, one NUTS): `scheme = 'cpv'`
    // matches two thirds of the table, which only statistics can tell the planner.
    exec(&conn, "INSERT INTO tenders (id, source, kind, created_at, current_seq) \
                 SELECT value, 'ted', 'procedure', 0, 1 FROM generate_series(1, 10000)").await;
    for (scheme, code) in [("cpv", "45000000"), ("cpv", "71000000"), ("nuts", "DE300")] {
        exec(&conn, &format!(
            "INSERT INTO tender_version_classifications (tender_id, seq, field, scheme, code) \
             SELECT value, 1, 'main', '{scheme}', '{code}' FROM generate_series(1, 10000)"
        )).await;
    }

    // The deferred indexes after the seed, as on prod: a bulk build, not 126k
    // one-row index inserts (which made this test take two minutes).
    db.build_organization_indexes().await.expect("org indexes");
    db.build_tender_indexes().await.expect("tender indexes");

    // A stray manual ANALYZE of the excluded table, the thing the job must undo.
    exec(&conn, "ANALYZE organizations").await;
    let stray = {
        let mut rows = conn.query("SELECT COUNT(*) FROM sqlite_stat1 WHERE tbl = 'organizations'", ()).await.unwrap();
        let row = rows.next().await.unwrap().expect("a count");
        row.get::<i64>(0).unwrap()
    };
    assert!(stray > 0, "the stray stats exist (one row per index)");
    let before_join = plan(&conn, PLAIN_JOIN).await;

    // The job, as the supervisor arm runs it.
    for table in store::ANALYZE_TABLES {
        db.analyze_table(table).await.unwrap_or_else(|e| panic!("{table}: {e}"));
    }
    let removed = db.finish_analyze().await.expect("finish");
    assert_eq!(removed, stray, "every stray organizations row is removed");

    // A fresh connection plans with what is on disk now.
    let conn = raw.connect().expect("reconnect");
    let tables = stat_tables(&conn).await;
    assert!(!tables.contains(&"organizations".to_owned()), "no organizations statistics: {tables:?}");
    for t in &tables {
        assert!(store::ANALYZE_TABLES.contains(&t.as_str()), "{t} was analyzed but is not in ANALYZE_TABLES");
    }
    assert!(tables.contains(&"tenders".to_owned()) && tables.contains(&"tender_version_classifications".to_owned()), "{tables:?}");

    // (b) the resolver still seeks its index on the skewed name.
    let resolver = plan(&conn, RESOLVER).await;
    assert!(resolver.contains("organizations_name_country"), "resolver plan:\n{resolver}");

    // (c) the plain JOIN drives from the Tenders range once the version table has
    // statistics. Without them it drives from `(scheme, code)` — 421's trap.
    let after_join = plan(&conn, PLAIN_JOIN).await;
    let lines: Vec<&str> = after_join.lines().collect();
    assert!(
        lines.first().is_some_and(|l| l.contains("t USING INTEGER PRIMARY KEY"))
            && lines.get(1).is_some_and(|l| l.contains("tender_id=? AND seq=?")),
        "tenders must drive and the version table be sought by (tender_id, seq); before ANALYZE \
         (it drove from the (scheme, code) index):\n{before_join}\nafter:\n{after_join}"
    );

    // Refused outright: a table outside the list is never analyzed.
    let err = db.analyze_table("organizations").await.expect_err("organizations is refused");
    assert!(err.contains("not in ANALYZE_TABLES"), "{err}");
    let err = db.analyze_table("notice_texts").await.expect_err("the raw layer is refused");
    assert!(err.contains("not in ANALYZE_TABLES"), "{err}");

    drop(conn);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
