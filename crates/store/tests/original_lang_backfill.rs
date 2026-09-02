//! ADR-0013 D3's third leg on the STANDING corpus: `backfill_original_lang`
//! stamps `tender_versions.original_lang` from the notice-level language code
//! each era already stores, through an injected normaliser, in bounded windows.
//!
//! What is pinned: every era's field id is read; the value goes through the
//! injected map (the fold's own `normalize_lang` in production — `store` cannot
//! depend on `ingest`, so it arrives as a `fn`); a notice with none of the three
//! codes leaves its version NULL rather than guessing; a window with nothing to
//! stamp still advances the walk; a second run is a no-op.

use store::turso::{self, Value};

async fn fixture(tag: &str) -> (String, store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-origbf-{tag}-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    (path, db, conn)
}

async fn exec(conn: &turso::Connection, sql: String) {
    conn.execute(&sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// A notice, its (optional) PROCEDURE-level language code, and one tender with
/// one version caused by it, `original_lang` NULL — the pre-column corpus shape.
async fn seed(conn: &turso::Connection, id: i64, code: Option<(&str, &str)>) {
    exec(
        conn,
        format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at, parse_state)
             VALUES ({id}, 'ted', 'pub-{id}', 'h{id}', 'p', 1, 'm', 0, 'parsed')"
        ),
    )
    .await;
    if let Some((field, value)) = code {
        exec(
            conn,
            format!(
                "INSERT INTO notice_codes (notice_id, section_id, field_id, ordinal, list_name, code)
                 VALUES ({id}, 'PROCEDURE', '{field}', 0, NULL, '{value}')"
            ),
        )
        .await;
    }
    exec(
        conn,
        format!(
            "INSERT INTO tenders (id, source, kind, current_seq, created_at)
             VALUES ({id}, 'ted', 'procedure', 1, 0)"
        ),
    )
    .await;
    exec(
        conn,
        format!(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES ({id}, 1, 100, 'pub-{id}', {id})"
        ),
    )
    .await;
}

async fn stamped(conn: &turso::Connection) -> Vec<(i64, Option<String>)> {
    let mut rows = conn
        .query("SELECT tender_id, original_lang FROM tender_versions ORDER BY tender_id", ())
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let Value::Integer(id) = row.get_value(0).unwrap() else { panic!("id") };
        let lang = match row.get_value(1).unwrap() {
            Value::Text(t) => Some(t),
            _ => None,
        };
        out.push((id, lang));
    }
    out
}

/// A stand-in for the fold's map: two-letter → a marked three-letter, so the
/// test can tell "went through the normaliser" from "copied raw".
fn norm(code: &str) -> Option<String> {
    match code {
        "EN" => Some("ENG".into()),
        "FR" => Some("FRA".into()),
        "DEU" => Some("DEU".into()),
        _ => None,
    }
}

#[tokio::test]
async fn every_era_field_is_read_and_normalised_and_absence_stays_null() {
    let (path, db, conn) = fixture("eras").await;
    seed(&conn, 1, Some(("TED-LG_ORIG", "FR"))).await; // r208/r209
    seed(&conn, 2, Some(("BT-702(a)-notice", "DEU"))).await; // eForms
    seed(&conn, 3, Some(("TXT-OL", "EN"))).await; // text era
    seed(&conn, 4, None).await; // a 1990s notice: no OL line
    seed(&conn, 5, Some(("TED-TD_DOCUMENT_TYPE", "3"))).await; // a code, but not a language

    // Window of 2 tenders per batch: three windows to cover five, the walk
    // must not stop at a window that stamps nothing (tender 4 and 5's).
    let mut after = 0;
    let mut windows = 0;
    loop {
        let (rows, next) = db.backfill_original_lang(2, after, norm).await.expect("batch");
        if rows == 0 {
            break;
        }
        windows += 1;
        after = next;
    }
    assert_eq!(windows, 3, "five tenders in windows of two");
    assert_eq!(
        stamped(&conn).await,
        vec![
            (1, Some("FRA".into())),
            (2, Some("DEU".into())),
            (3, Some("ENG".into())),
            (4, None),
            (5, None),
        ],
        "each era's code lands through the normaliser; no code, or a non-language code, stays NULL"
    );

    // A second run finds nothing left to stamp and changes nothing.
    let before = stamped(&conn).await;
    let mut after = 0;
    loop {
        let (rows, next) = db.backfill_original_lang(2, after, norm).await.expect("batch");
        if rows == 0 {
            break;
        }
        after = next;
    }
    assert_eq!(stamped(&conn).await, before, "idempotent");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// A row the fold already stamped is never overwritten by the backfill — the
/// walk touches `original_lang IS NULL` only.
#[tokio::test]
async fn an_already_stamped_version_is_left_alone() {
    let (path, db, conn) = fixture("keep").await;
    seed(&conn, 1, Some(("TED-LG_ORIG", "FR"))).await;
    exec(&conn, "UPDATE tender_versions SET original_lang = 'POL' WHERE tender_id = 1".into()).await;
    let (rows, _) = db.backfill_original_lang(10, 0, norm).await.expect("batch");
    assert_eq!(rows, 1, "the window held the tender");
    assert_eq!(stamped(&conn).await, vec![(1, Some("POL".into()))], "the fold's value stands");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
