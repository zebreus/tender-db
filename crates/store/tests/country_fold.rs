//! Issue 319: folding non-canonical country codes on organization rows. The
//! fold itself is ingest's (ISO tables); this pins the store machinery —
//! plan, change events, collision counting, and the promise that a label
//! rewrite never touches identity.

use store::turso::Value;

/// A miniature `project::canonical_country`: the shapes that actually sit in
/// the corpus — an alpha-3, a country NAME, the two non-ISO specials — plus
/// junk that must survive untouched.
fn fold(raw: &str) -> String {
    let up = raw.trim().to_ascii_uppercase();
    match up.as_str() {
        "GRL" => "GL".into(),
        "SEN" => "SN".into(),
        "DEU" => "DE".into(),
        "LUXEMBOURG" => "LU".into(),
        "UK" => "GB".into(),
        _ => up,
    }
}

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

async fn one(conn: &store::turso::Connection, sql: &str) -> String {
    let mut rows = conn.query(sql, ()).await.unwrap();
    match rows.next().await.unwrap() {
        Some(row) => match row.get_value(0).unwrap() {
            Value::Text(s) => s,
            Value::Null => "NULL".into(),
            v => format!("{v:?}"),
        },
        None => "NONE".into(),
    }
}

#[tokio::test]
async fn the_fold_rewrites_the_label_and_never_the_identity() {
    let path = "test-country-fold.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // 1/2: the split pair — one Greenlandic utility under GL and GRL, the
    // shape that found this bug. Same identifier, so folding GRL onto GL
    // makes them a COLLISION: two rows, one identity.
    // 3: an alpha-3 with no twin — folds cleanly.
    // 4: a country NAME.
    // 5: junk that nothing maps ('1A0' is real, 3 rows on prod).
    // 6: already canonical — must not be touched, and must not be counted.
    // 7: NULL country — the R3 pool's shape, invisible to this job.
    for (id, country, ident, name) in [
        (1i64, Some("GL"), Some("18440202"), "Nukissiorfiit"),
        (2, Some("GRL"), Some("18440202"), "Nukissiorfiit"),
        (3, Some("SEN"), Some("SN12345"), "Dakar Port Authority"),
        (4, Some("LUXEMBOURG"), Some("LU9999"), "Ville de Luxembourg"),
        (5, Some("1A0"), Some("X1"), "Mystery Ltd"),
        (6, Some("DE"), Some("DE811111111"), "Berlin GmbH"),
        (7, None, Some("999"), "Country-less"),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, 'national', ?, ?, ?, 0, 0)",
            (
                Value::Integer(id),
                match country { Some(c) => Value::Text(c.into()), None => Value::Null },
                match ident { Some(v) => Value::Text(v.into()), None => Value::Null },
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }

    let never = || false;
    // DRY: the whole plan, nothing written.
    let dry = db.fold_org_countries(fold, true, 100, &never).await.unwrap();
    assert_eq!(dry.values, 6, "six distinct non-NULL values; the NULL row is not one");
    assert_eq!(
        dry.plan,
        vec![
            ("1A0".to_owned(), "1A0".to_owned(), 1),
            ("GRL".to_owned(), "GL".to_owned(), 1),
            ("LUXEMBOURG".to_owned(), "LU".to_owned(), 1),
            ("SEN".to_owned(), "SN".to_owned(), 1),
        ]
        .into_iter()
        .filter(|(f, t, _)| f != t)
        .collect::<Vec<_>>(),
        "only values the fold actually changes are planned"
    );
    assert_eq!(dry.rows, 3);
    assert_eq!(
        dry.collisions, 1,
        "org 2 folds onto org 1's identity — one pair, handed to R2, not merged here"
    );
    assert_eq!(dry.unmapped, vec![("1A0".to_owned(), 1)], "the residue is NAMED, not just counted");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE country = 'GRL'").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 0, "a dry run is silent");

    // WET.
    let wet = db.fold_org_countries(fold, false, 200, &never).await.unwrap();
    assert_eq!((wet.rows, wet.collisions), (3, 1), "the wet run reports the same plan it ran");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 2").await, "GL");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 3").await, "SN");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 4").await, "LU");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 5").await, "1A0", "junk stands");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 6").await, "DE");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 7").await, "NULL");

    // Identity is untouched: same rows, same identifiers, nothing merged or
    // deleted — including the colliding pair, which is R2's to decide.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 7);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = '18440202'").await,
        2,
        "the collision is left standing for the merge arm"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log").await, 0);

    // The change feed carries one 'changed' per moved row — country is a
    // published field, so a consumer filtering on it must see the correction.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND op = 'changed'").await,
        3
    );
    for id in [2i64, 3, 4] {
        assert_eq!(
            count(&conn, &format!("SELECT COUNT(*) FROM changes WHERE entity_id = {id}")).await,
            1
        );
    }
    for id in [1i64, 5, 6, 7] {
        assert_eq!(
            count(&conn, &format!("SELECT COUNT(*) FROM changes WHERE entity_id = {id}")).await,
            0,
            "org {id} did not move, so nothing was published about it"
        );
    }

    // Idempotent: a second run finds nothing to do and publishes nothing.
    let again = db.fold_org_countries(fold, false, 300, &never).await.unwrap();
    assert_eq!((again.rows, again.plan.len()), (0, 0));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 3, "no second wave of events");

    // Cancel is honest: no partial plan.
    let always = || true;
    let stopped = db.fold_org_countries(fold, true, 400, &always).await.unwrap();
    assert!(stopped.stopped);
    assert_eq!((stopped.values, stopped.rows), (0, 0));
}
