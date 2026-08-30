//! Issue 319: folding non-canonical country codes on organization rows. The
//! fold itself is ingest's (ISO tables); this pins the store machinery —
//! plan, change events, collision counting, and the promise that a label
//! rewrite never touches identity.

use store::turso::Value;

/// A miniature `project::canonical_country`: the shapes that actually sit in
/// the corpus — an alpha-3, a country NAME, the two non-ISO specials — plus
/// junk that must survive untouched.
fn fold(raw: &str) -> String {
    let up = raw.trim().to_uppercase();
    match up.as_str() {
        "GRL" => "GL".into(),
        "GRD" => "GD".into(),
        "SEN" => "SN".into(),
        "DEU" => "DE".into(),
        "LUXEMBOURG" => "LU".into(),
        "LËTZEBUERG" => "LU".into(),
        "EL" => "GR".into(),
        "UK" => "GB".into(),
        _ => up,
    }
}

/// `countries::is_alpha2`'s shape: the real codes this fixture uses.
fn known(code: &str) -> bool {
    matches!(code.trim(), "GL" | "SN" | "LU" | "DE" | "GB" | "GR" | "GD" | "XI")
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
    for (id, country, kind, ident, name) in [
        (1i64, Some("GL"), "national", Some("18440202"), "Nukissiorfiit"),
        (2, Some("GRL"), "national", Some("18440202"), "Nukissiorfiit"),
        (3, Some("SEN"), "national", Some("SN12345"), "Dakar Port Authority"),
        (4, Some("LUXEMBOURG"), "national", Some("LU9999"), "Ville de Luxembourg"),
        (5, Some("1A0"), "national", Some("X1"), "Mystery Ltd"),
        (6, Some("DE"), "national", Some("DE811111111"), "Berlin GmbH"),
        (7, None, "national", Some("999"), "Country-less"),
        // SEN carries THREE rows, so `rows` and `plan.len()` cannot be
        // confused (panel catch: every value had exactly one row, which made
        // both numbers 3 for different reasons).
        (8, Some("SEN"), "national", Some("SN22222"), "Dakar Water"),
        (9, Some("SEN"), "national", Some("SN33333"), "Dakar Rail"),
        // A SECOND spelling of Luxembourg, colliding with row 4's identity
        // only AFTER both fold onto LU — a collision between two rows this
        // run moves, which a per-source probe against the target cannot see.
        (10, Some("LËTZEBUERG"), "national", Some("LU9999"), "Ville de Luxembourg"),
        // The VAT scope labels: folding these would fragment the resolver's
        // binding key, so the job must SKIP them and say so.
        (11, Some("EL"), "vat", Some("EL094019245"), "Hellenic Post"),
        (12, Some("UK"), "vat", Some("GB123456789"), "Royal Mail"),
        // Two characters and no country at all: the residue test is "is this
        // a real code", not "is it two characters", so this must be LISTED.
        (13, Some("ZZ"), "national", Some("Z9"), "Nowhere Ltd"),
        // A NATIONAL-kind row spelled 'EL': not a VAT scope prefix, so this
        // one DOES fold to GR. Excluding by kind rather than by value is
        // exactly what buys this.
        (14, Some("EL"), "national", Some("123456"), "Greek Municipality"),
        // A row with NO identifier_kind at all — a provisional minted from a
        // name alone. In SQL `identifier_kind = 'vat'` is NULL here, not
        // false, so a bare comparison puts this row in a THIRD group and
        // plans 'GRL' twice. Prod's dry run showed 253 plan entries for 152
        // values before this was pinned.
        (15, Some("GRL"), "", None, "Nameless Greenland Body"),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 0, 0)",
            (
                Value::Integer(id),
                match country { Some(c) => Value::Text(c.into()), None => Value::Null },
                if kind.is_empty() { Value::Null } else { Value::Text(kind.into()) },
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
    let dry = db.fold_org_countries(fold, known, true, None, 100, &never).await.unwrap();
    assert_eq!(
        dry.values, 10,
        "ten DISTINCT non-NULL values — 'EL' appears under two kinds and is still one value"
    );
    assert_eq!(
        dry.plan,
        vec![
            ("SEN".to_owned(), "SN".to_owned(), 3),
            ("GRL".to_owned(), "GL".to_owned(), 2),
            ("EL".to_owned(), "GR".to_owned(), 1),
            ("LUXEMBOURG".to_owned(), "LU".to_owned(), 1),
            ("LËTZEBUERG".to_owned(), "LU".to_owned(), 1),
        ],
        "planned values only, biggest first — and 1A0 is absent because it folds to itself"
    );
    assert_eq!(dry.rows, 8, "EIGHT rows over FIVE values — the two numbers are not the same");
    assert_eq!(
        dry.plan.iter().filter(|(f, _, _)| f == "GRL").count(),
        1,
        "one entry per VALUE: a NULL identifier_kind must not split the group"
    );
    assert_ne!(dry.rows as usize, dry.plan.len());
    assert_eq!(
        dry.collisions, 2,
        "org 2 lands on org 1's standing identity, AND rows 4 and 10 collide with each \
         other only because this same run moves both onto LU — the case a per-source \
         probe against the target cannot see"
    );
    assert_eq!(
        dry.unmapped,
        vec![("1A0".to_owned(), 1), ("ZZ".to_owned(), 1)],
        "the residue is NAMED, not just counted — and 'ZZ' proves the test is code \
         validity, not string length"
    );
    assert_eq!(
        dry.vat_scope_skipped,
        vec![("EL".to_owned(), 1), ("UK".to_owned(), 1)],
        "the VAT-kind rows are skipped ON PURPOSE and reported, not silently folded — \
         while the NATIONAL-kind 'EL' row beside them is planned"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE country = 'GRL'").await,
        2,
        "both GRL rows still stand — the identifier-bearing one and the kind-less one"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 0, "a dry run is silent");

    // WET. The parity gate: a run whose plan disagrees with the reviewed
    // one aborts before touching a row.
    let err = db.fold_org_countries(fold, known, false, Some(400), 200, &never).await;
    assert!(err.is_err(), "plan divergence beyond tolerance must abort");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 2").await, "GRL");

    let wet = db.fold_org_countries(fold, known, false, Some(8), 200, &never).await.unwrap();
    assert_eq!((wet.rows, wet.collisions), (8, 2), "the wet run reports the plan it ran");
    for (id, want) in [
        (2i64, "GL"),
        (3, "SN"),
        (4, "LU"),
        (5, "1A0"),   // junk stands
        (6, "DE"),    // already canonical
        (7, "NULL"),  // country-less
        (8, "SN"),
        (9, "SN"),
        (10, "LU"),
        (11, "EL"),   // VAT scope label: NOT folded, or the resolver loses it
        (12, "UK"),
    ] {
        assert_eq!(
            one(&conn, &format!("SELECT country FROM organizations WHERE id = {id}")).await,
            want,
            "org {id}"
        );
    }

    // Identity is untouched: same rows, same identifiers, nothing merged or
    // deleted — including the colliding pairs, which are R2's to decide.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 15);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = '18440202'").await,
        2,
        "the pre-existing collision is left standing for the merge arm"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'LU9999'").await,
        2,
        "and so is the one this run created by folding two spellings onto LU"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log").await, 0);

    // The change feed carries one 'changed' per MOVED row — country is a
    // published field, so a consumer filtering on it must see the correction.
    // SEN alone moves three rows, so this cannot be read as one per VALUE.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND op = 'changed'").await,
        8
    );
    for id in [2i64, 3, 4, 8, 9, 10, 14, 15] {
        assert_eq!(
            count(&conn, &format!("SELECT COUNT(*) FROM changes WHERE entity_id = {id}")).await,
            1,
            "org {id} moved, exactly once"
        );
    }
    for id in [1i64, 5, 6, 7, 11, 12, 13] {
        assert_eq!(
            count(&conn, &format!("SELECT COUNT(*) FROM changes WHERE entity_id = {id}")).await,
            0,
            "org {id} did not move, so nothing was published about it"
        );
    }

    // Idempotent: a second run finds nothing to do and publishes nothing.
    let again = db.fold_org_countries(fold, known, false, Some(0), 300, &never).await.unwrap();
    assert_eq!((again.rows, again.plan.len()), (0, 0));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 8, "no second wave of events");

    // Cancel is honest: no partial plan.
    let always = || true;
    let stopped = db.fold_org_countries(fold, known, true, None, 400, &always).await.unwrap();
    assert!(stopped.stopped);
    assert_eq!((stopped.values, stopped.rows), (0, 0));
}

/// The WET path honours cancellation too (panel catch: the kind is in
/// STOPPABLE_KINDS, and the first version checked the flag only while
/// planning — a stoppable kind that ignores the flag is issue 252's
/// dishonest-cancel bug).
#[tokio::test]
async fn a_cancelled_wet_fold_stops_and_says_so() {
    let path = "test-country-fold-cancel.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for (id, country) in [(1i64, "GRL"), (2, "SEN"), (3, "DEU")] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, 'national', 'X', 'n', 'n', 0, 0)",
            (Value::Integer(id), Value::Text(country.into())),
        )
        .await
        .unwrap();
    }
    // Stops on the FIRST wet value, before any update: the flag is read at
    // the top of the loop, so nothing is written and the report says stopped.
    let always = || true;
    let r = db.fold_org_countries(fold, known, false, Some(3), 100, &always).await.unwrap();
    assert!(r.stopped, "a cancelled wet run must report it");
    assert_eq!(one(&conn, "SELECT country FROM organizations WHERE id = 1").await, "GRL");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 0);
}
