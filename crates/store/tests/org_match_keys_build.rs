//! Issue 300 Stage 4 Unit 3: the windowed org_match_keys build — exactly-once
//! windows (watermark in the same transaction), per-org dedupe across head and
//! satellite names, N3-only-when-different elision, the self-healing finish,
//! and the no-entity-writes contract. Key fns are test-local stand-ins; the
//! real N2/N3 content is pinned by ingest's own tests.

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

#[tokio::test]
async fn the_key_build_walks_exactly_once_and_finishes_self_healingly() {
    let path = "test-org-match-keys-build.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // Orgs: 1 has a satellite that yields the SAME n2 (dedupe) and a head
    // with a form token (n3 differs); 2 is formless (n3 == n2, elided);
    // 3 has an empty-normalizing name; 4 has a satellite with a NEW key.
    for (id, name) in
        [(1i64, "Alfa GmbH"), (2, "Ville de Calais"), (3, "  "), (4, "Gamma Oy")]
    {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, ?, ?, 1, 0)",
            (Value::Integer(id), Value::Text(name.into()), Value::Text(name.to_lowercase())),
        )
        .await
        .unwrap();
    }
    for (org, lang, name) in [(1i64, "de", "alfa gmbh"), (4, "sv", "Gamma Aktiebolag")] {
        conn.execute(
            "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (?, ?, ?, ?)",
            (
                Value::Integer(org),
                Value::Text(lang.into()),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }
    let changes_before = count(&conn, "SELECT COUNT(*) FROM changes").await;

    // Dry run: full walk, nothing written.
    let mut after = 0i64;
    let mut dry_rows = 0u64;
    loop {
        let (w, next) = db.build_org_match_keys_batch(n2, n3, 2, after, true).await.unwrap();
        if w.orgs == 0 {
            break;
        }
        dry_rows += w.n2_rows + w.n3_rows;
        assert_eq!(w.rows_written, 0, "dry windows write nothing");
        after = next;
    }
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_match_keys").await, 0);
    // 1: n2 'alfa gmbh' (head + satellite dedupe to one) + n3 'alfa §gmbh';
    // 2: n2 only; 3: skipped; 4: n2 'gamma oy' + satellite n2
    // 'gamma aktiebolag'. Total: 4 n2 + 1 n3 = 5.
    assert_eq!(dry_rows, 5, "4 n2 + 1 n3 keys");

    // Wet build with batch=2: window 1 covers orgs 1-2, window 2 covers 3-4;
    // the watermark advances IN the window transaction.
    db.reset_org_match_keys("epoch-1").await.unwrap();
    let (w1, wm1) = db.build_org_match_keys_batch(n2, n3, 2, 0, false).await.unwrap();
    assert_eq!((w1.orgs, wm1), (2, 2));
    assert_eq!(
        count(&conn, "SELECT org_match_keys_watermark FROM projection_state WHERE id = 0").await,
        2,
        "the watermark committed with the window"
    );
    // A crash-resume re-runs FROM the watermark: no duplicate rows.
    let (w2, wm2) = db.build_org_match_keys_batch(n2, n3, 2, wm1, false).await.unwrap();
    assert_eq!((w2.orgs, wm2), (2, 4));
    let (w3, _) = db.build_org_match_keys_batch(n2, n3, 2, wm2, false).await.unwrap();
    assert_eq!(w3.orgs, 0, "the walk terminates");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_match_keys").await, 5);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_match_keys WHERE key_kind = 'n3'").await,
        1,
        "n3 stored only where it differs from n2"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_match_keys WHERE org_id = 1 AND key_kind = 'n2'").await,
        1,
        "head + same-key satellite dedupe to one row"
    );

    // Finish: index lands, watermark resets; running finish TWICE (the
    // crashed-finish window) is a no-op thanks to IF NOT EXISTS.
    let rows = db.finish_org_match_keys().await.unwrap();
    assert_eq!(rows, 5);
    let rows2 = db.finish_org_match_keys().await.unwrap();
    assert_eq!(rows2, 5);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='org_match_keys_kk'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT org_match_keys_watermark FROM projection_state WHERE id = 0").await,
        0
    );
    let (wm, epoch) = db.org_match_keys_state().await.unwrap();
    assert_eq!((wm, epoch.as_str()), (0, "epoch-1"));

    // No entity writes anywhere in this path (decision 6).
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes").await,
        changes_before,
        "the key build emits no change events"
    );

    // A rebuild under a NEW epoch starts clean: no stale rows survive.
    db.reset_org_match_keys("epoch-2").await.unwrap();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_match_keys").await, 0);
    let (_, epoch) = db.org_match_keys_state().await.unwrap();
    assert_eq!(epoch, "epoch-2");
}
