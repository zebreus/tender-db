//! ADR-0013 D3's third leg: the read-time title pick honours the version's
//! ORIGINAL language between English and "any labelled".
//!
//! Chain: requested → ENG → original → any labelled → unlabelled. The cases that
//! distinguish the new leg from the old chain are the ones with NO English
//! variant: before this column existed such a tender served whichever labelled
//! row came first in scan order; now it serves the language the notice was
//! published in, unless the reader asked for another that exists.

use store::read::{self, Filter, Scope};
use store::turso;

async fn fixture(tag: &str) -> (String, turso::Connection) {
    let path = format!("/tmp/tender-db-origlang-{tag}-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    let _db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    (path, conn)
}

async fn exec(conn: &turso::Connection, sql: String) {
    conn.execute(&sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// One tender, one version, with the given original language and a title in
/// each listed language — deliberately inserted in the order given, so scan
/// order is known and a pick that ignores the leg is detectable.
async fn tender(conn: &turso::Connection, id: i64, original: Option<&str>, titles: &[(&str, &str)]) {
    exec(
        conn,
        format!(
            "INSERT INTO tenders (id, source, kind, current_seq, current_published_at, created_at)
             VALUES ({id}, 'ted', 'procedure', 1, 100, 0)"
        ),
    )
    .await;
    let original = original.map_or("NULL".to_owned(), |l| format!("'{l}'"));
    exec(
        conn,
        format!(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id, original_lang)
             VALUES ({id}, 1, 100, 'pub-{id}', {id}, {original})"
        ),
    )
    .await;
    for (lang, value) in titles {
        exec(
            conn,
            format!(
                "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
                 VALUES ({id}, 1, NULL, 'title', '{lang}', '{value}')"
            ),
        )
        .await;
    }
}

async fn lot_title_of(conn: &turso::Connection, tender_id: i64, lang: Option<&str>) -> Option<String> {
    let filter = Filter { lang: lang.map(str::to_owned), ..Filter::default() };
    read::lots(conn, &filter, Scope::Page { after: 0, limit: 25 })
        .await
        .expect("lots")
        .into_iter()
        .find(|l| l.tender_id == tender_id)
        .and_then(|l| l.title)
}

async fn title_of(conn: &turso::Connection, id: i64, lang: Option<&str>) -> Option<String> {
    let filter = Filter { lang: lang.map(str::to_owned), ..Filter::default() };
    read::tenders(conn, &filter, Scope::Page { after: id - 1, limit: 1 })
        .await
        .expect("read")
        .into_iter()
        .find(|t| t.id == id)
        .and_then(|t| t.title)
}

#[tokio::test]
async fn the_original_language_outranks_any_other_labelled_variant() {
    let (path, conn) = fixture("rank").await;
    // No English anywhere: the leg under test is the only thing that can decide.
    // FRA is inserted FIRST so that "first labelled in scan order" — the old
    // behaviour — would serve the French title.
    tender(&conn, 1, Some("DEU"), &[("FRA", "Titre"), ("DEU", "Titel")]).await;

    assert_eq!(title_of(&conn, 1, None).await.as_deref(), Some("Titel"), "no request: the original wins");
    assert_eq!(
        title_of(&conn, 1, Some("FRA")).await.as_deref(),
        Some("Titre"),
        "a requested language that exists still outranks the original"
    );
    assert_eq!(
        title_of(&conn, 1, Some("ENG")).await.as_deref(),
        Some("Titel"),
        "a requested language that is absent falls through ENG to the original"
    );
    assert_eq!(
        title_of(&conn, 1, Some("POL")).await.as_deref(),
        Some("Titel"),
        "any absent request lands on the original, not on scan order"
    );
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

#[tokio::test]
async fn english_still_outranks_the_original() {
    let (path, conn) = fixture("eng").await;
    tender(&conn, 2, Some("DEU"), &[("DEU", "Titel"), ("ENG", "Title")]).await;
    assert_eq!(title_of(&conn, 2, None).await.as_deref(), Some("Title"), "ENG is the second leg, original the third");
    assert_eq!(title_of(&conn, 2, Some("DEU")).await.as_deref(), Some("Titel"), "unless the reader asks for the original");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// The lots list picks lot titles in memory (`summarise`), not in SQL — a
/// second implementation of the same ladder, so it gets the same case: no
/// English, French first in scan order, German the original.
#[tokio::test]
async fn the_lot_title_pick_honours_the_original_too() {
    let (path, conn) = fixture("lot").await;
    tender(&conn, 4, Some("DEU"), &[]).await;
    exec(&conn, "INSERT INTO lots (id, tender_id, lot_key) VALUES (40, 4, 'LOT-0001')".into()).await;
    exec(&conn, "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (4, 1, 40, 'Lot')".into())
        .await;
    for (lang, value) in [("FRA", "Lot titre"), ("DEU", "Los Titel")] {
        exec(
            &conn,
            format!(
                "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
                 VALUES (4, 1, 40, 'title', '{lang}', '{value}')"
            ),
        )
        .await;
    }
    assert_eq!(
        lot_title_of(&conn, 4, None).await.as_deref(),
        Some("Los Titel"),
        "the lot serves its notice's original language"
    );
    assert_eq!(
        lot_title_of(&conn, 4, Some("FRA")).await.as_deref(),
        Some("Lot titre"),
        "unless the reader asks for one that exists"
    );
    assert_eq!(
        lot_title_of(&conn, 4, Some("ENG")).await.as_deref(),
        Some("Los Titel"),
        "an absent request falls through to the original"
    );
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// A version whose era never said its language (the 1990s text notices) must
/// rank exactly as before the column existed: no leg, deterministic tail.
#[tokio::test]
async fn a_null_original_leaves_the_old_chain_untouched() {
    let (path, conn) = fixture("null").await;
    tender(&conn, 3, None, &[("FRA", "Titre"), ("DEU", "Titel")]).await;
    assert_eq!(
        title_of(&conn, 3, None).await.as_deref(),
        Some("Titre"),
        "with no original recorded, the first labelled variant in scan order serves — the pre-column behaviour"
    );
    assert_eq!(title_of(&conn, 3, Some("DEU")).await.as_deref(), Some("Titel"));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
