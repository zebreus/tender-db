//! Issue 307: the organization_names satellite backfill for the standing
//! corpus — labelled name-field texts on a recorded mention's exact section
//! REPLACE into the satellite; unlabelled and non-name texts stay out;
//! a re-run leaves the row count unchanged.

use store::turso::Value;

async fn walk(db: &store::Db) -> store::OrgNameBackfill {
    let mut totals = store::OrgNameBackfill::default();
    let mut watermark = 0i64;
    loop {
        let (batch, next) = db
            .backfill_org_name_variants_batch(
                &["BT-500-Organization-Company", "TED-OFFICIALNAME"],
                normalize,
                50,
                watermark,
            )
            .await
            .expect("window");
        if batch.notices == 0 {
            break;
        }
        totals.mentions += batch.mentions;
        totals.written += batch.written;
        watermark = next;
    }
    totals
}

fn normalize(lang: Option<&str>) -> Option<String> {
    // A two-case stand-in for ingest's normalize_lang (dependency direction
    // keeps the real one out of store's tests; the walk takes a fn pointer).
    match lang?.to_ascii_uppercase().as_str() {
        "DE" | "DEU" => Some("DEU".into()),
        "FR" | "FRA" => Some("FRA".into()),
        other => Some(other.to_owned()),
    }
}

#[tokio::test]
async fn labelled_variants_backfill_once_and_idempotently() {
    let path = format!("/tmp/tender-db-orgbackfill-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    for (id, name) in [(10, "Stadt Brüssel"), (11, "Unrelated")] {
        conn.execute(
            "INSERT INTO organizations (id, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, 1, 0)",
            (
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }
    // Notice 100: an eForms org section with DE+FR BT-500 variants, one
    // unlabelled repeat, and a labelled NON-name field that must stay out.
    for (nid, sid, field, ord, lang, value) in [
        (100i64, "ORG-0001", "BT-500-Organization-Company", 0i64, Some("DE"), "Stadt Brüssel"),
        (100, "ORG-0001", "BT-500-Organization-Company", 1, Some("FR"), "Ville de Bruxelles"),
        (100, "ORG-0001", "BT-500-Organization-Company", 2, None, "City of Brussels"),
        (100, "ORG-0001", "BT-513-Organization-Company", 0, Some("DE"), "Brüssel"),
        // A mention-less section's labelled name goes nowhere.
        (100, "ORG-0002", "BT-500-Organization-Company", 0, Some("DE"), "Ohne Mention"),
    ] {
        conn.execute(
            "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value)
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(nid),
                Value::Text(sid.into()),
                Value::Text(field.into()),
                Value::Integer(ord),
                match lang {
                    Some(l) => Value::Text(l.into()),
                    None => Value::Null,
                },
                Value::Text(value.into()),
            ),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id)
         VALUES (100, 'ORG-0001', 10)",
        (),
    )
    .await
    .unwrap();

    // The walk windows over notices — seed the notice row's id space.
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
             member_path, ingested_at, parse_state, projected)
         VALUES (100, 'ted', 'p1', 'h1', 'eforms:eforms-sdk-1.13', 1, 'm', 0, 'parsed', 1)",
        (),
    )
    .await
    .unwrap();

    let totals = walk(&db).await;
    assert_eq!(totals.mentions, 1, "one recorded mention visited");
    assert_eq!(totals.written, 2, "DEU + FRA written; unlabelled and BT-513 stay out");

    let count = |sql: &'static str| {
        let conn = &conn;
        async move {
            let mut rows = conn.query(sql, ()).await.unwrap();
            match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
                Value::Integer(i) => i,
                other => panic!("unexpected {other:?}"),
            }
        }
    };
    assert_eq!(count("SELECT COUNT(*) FROM organization_names").await, 2);
    assert_eq!(
        count("SELECT COUNT(*) FROM organization_names WHERE org_id = 10 AND lang = 'FRA' AND name = 'Ville de Bruxelles' AND name_norm = 'ville de bruxelles'").await,
        1
    );

    // Idempotent: the second walk rewrites the same rows, count unchanged.
    let second = walk(&db).await;
    assert_eq!(second.written, 2, "REPLACE rewrites the same two rows");
    assert_eq!(count("SELECT COUNT(*) FROM organization_names").await, 2);

    drop(conn);
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
