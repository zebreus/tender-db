//! Issue 100: a re-parse must survive the projection's own output.
//!
//! Two prod runs of the DE-1.x re-parse died on `immediate foreign key constraint
//! failed` (jobs 721 and 733, 2026-08-17/18), and nothing in the suite covered
//! `reparse_notice` at all — which is exactly why the second failure was a surprise
//! rather than a red test. This file is that coverage: re-parse a notice that has
//! already been projected, i.e. one carrying the `organization_mentions` rows whose
//! FK points into `notice_sections`.

use store::turso::{self, Value};
use store::{Fetch, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-reparsefk-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    (db, conn)
}

/// One notice with an Organization section, as the parser would emit it.
fn parsed(name: &str) -> Parsed {
    Parsed {
        sections: vec![
            Section { id: "PROC".into(), kind: "Notice".into(), parent: None },
            Section { id: "ORG-1".into(), kind: "Organization".into(), parent: Some("PROC".into()) },
        ],
        values: vec![ValueRow {
            section_id: "ORG-1".into(),
            field_id: "BT-500-Organization-Company".into(),
            ordinal: 0,
            value: NoticeValue::Text { lang: None, value: name.into() },
        }],
    }
}

fn notice_ref() -> Notice {
    Notice {
        source: "doe".into(),
        publication_id: "pub-1".into(),
        content_hash: "hash-1".into(),
        profile: "eforms:eforms-de-1.0".into(),
        fetch_id: 1,
        member_path: "m.xml".into(),
        ingested_at: 0,
        declared_version: None,
        published_at: None,
        dispatched_at: None,
    }
}

#[tokio::test]
async fn a_projected_notice_can_be_reparsed() {
    let (db, conn) = open("projected").await;
    // `notices.fetch_id` references `fetches`, so the package has to exist first.
    db.record_fetch(&Fetch {
        source: "doe".into(),
        kind: "daily".into(),
        period: "2026-07-18".into(),
        url: "https://example.invalid/p.zip".into(),
        sha256: "deadbeef".into(),
        bytes: 1,
        fetched_at: 0,
        path: "doe/p.zip".into(),
    })
    .await
    .expect("fetch row");
    let n = notice_ref();
    db.record_notice(&n, &Parse::Parsed(parsed("ACME GmbH"))).await.expect("first parse");

    let id: i64 = {
        let mut rows = conn.query("SELECT id FROM notices", ()).await.unwrap();
        let row = rows.next().await.unwrap().expect("the notice row");
        row.get_value(0).unwrap().as_integer().copied().unwrap()
    };

    // What the PROJECTION's phase 1 writes: an organization and the mention that
    // ties it to the notice's Organization SECTION. This row is the FK that makes a
    // naive re-parse fail, and prod always has it — a notice that has been folded
    // once has been mentioned once.
    conn.execute(
        "INSERT INTO organizations (country, identifier_kind, identifier, name, name_norm,
             provisional, created_at) VALUES (NULL, NULL, NULL, 'ACME GmbH', 'acme gmbh', 1, 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name)
         VALUES (?, 'ORG-1', 1, 'ACME GmbH')",
        (Value::Integer(id),),
    )
    .await
    .unwrap();

    // And what the projection's PHASE 2 writes: the canonical party row, anchored
    // to that mention by (mention_notice_id, mention_section_id). THIS is the row
    // that made prod fail — `tender_version_parties` and `tender_version_bid_parties`
    // both carry an FK onto `organization_mentions`, so the mention cannot be deleted
    // while a party points at it. Without these rows the test is not prod-like: a
    // notice that has been folded once has been party-ed once.
    conn.execute(
        "INSERT INTO tenders (source, procedure_key, kind, created_at)
         VALUES ('doe', 'proc-1', 'procedure', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at,
             publication_id) VALUES (1, 1, ?, 0, 'pub-1')",
        (Value::Integer(id),),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id,
             mention_notice_id, mention_section_id) VALUES (1, 1, 'buyer', 1, ?, 'ORG-1')",
        (Value::Integer(id),),
    )
    .await
    .unwrap();

    // A SECOND notice with its own mention and party row, which must survive. The
    // clear deletes by notice, and a `DELETE FROM tender_version_parties` that lost
    // its WHERE would empty the corpus while still passing every assertion above.
    let other = Notice { publication_id: "pub-2".into(), content_hash: "hash-2".into(), ..notice_ref() };
    db.record_notice(&other, &Parse::Parsed(parsed("OTHER GmbH"))).await.expect("second parse");
    let other_id: i64 = {
        let mut rows = conn
            .query("SELECT id FROM notices WHERE publication_id = 'pub-2'", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().expect("the second notice");
        row.get_value(0).unwrap().as_integer().copied().unwrap()
    };
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name)
         VALUES (?, 'ORG-1', 1, 'OTHER GmbH')",
        (Value::Integer(other_id),),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at,
             publication_id) VALUES (1, 2, ?, 0, 'pub-2')",
        (Value::Integer(other_id),),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id,
             mention_notice_id, mention_section_id) VALUES (1, 2, 'buyer', 1, ?, 'ORG-1')",
        (Value::Integer(other_id),),
    )
    .await
    .unwrap();

    // The re-parse under test: same notice, a parser that now reads a better name.
    let reparsed = db.reparse_notice(&n, &parsed("ACME GESELLSCHAFT MBH")).await;
    let reparsed = reparsed.expect("a projected notice must be re-parsable, FK and all");
    assert!(reparsed, "the notice exists, so the re-parse replaced its parsed layer");

    // The new parse replaced the old one rather than doubling it (the reason
    // reparse_notice exists instead of a plain insert_parsed).
    let names: Vec<String> = {
        let mut out = Vec::new();
        let mut rows = conn
            .query("SELECT value FROM notice_texts WHERE notice_id = ?", (Value::Integer(id),))
            .await
            .unwrap();
        while let Some(row) = rows.next().await.unwrap() {
            out.push(row.get_value(0).unwrap().as_text().cloned().unwrap_or_default());
        }
        out
    };
    assert_eq!(names, vec!["ACME GESELLSCHAFT MBH".to_string()], "replaced, not doubled");

    // And the notice is queued for the fold that re-derives its mentions.
    let projected: i64 = {
        let mut rows = conn
            .query("SELECT projected FROM notices WHERE id = ?", (Value::Integer(id),))
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        row.get_value(0).unwrap().as_integer().copied().unwrap()
    };
    assert_eq!(projected, 0, "a re-parsed notice must be re-folded");

    // The mention-anchored canonical rows for THIS notice are gone (the re-fold
    // rebuilds them from the new parse), and the other notice's are untouched.
    let parties = |nid: i64| {
        let conn = &conn;
        async move {
            let mut rows = conn
                .query(
                    "SELECT COUNT(*) FROM tender_version_parties WHERE mention_notice_id = ?",
                    (Value::Integer(nid),),
                )
                .await
                .unwrap();
            let row = rows.next().await.unwrap().unwrap();
            row.get_value(0).unwrap().as_integer().copied().unwrap()
        }
    };
    assert_eq!(parties(id).await, 0, "the re-parsed notice's party rows are cleared for the re-fold");
    assert_eq!(parties(other_id).await, 1, "another notice's party rows are NOT collateral");
    let mentions: i64 = {
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM organization_mentions WHERE notice_id = ?",
                (Value::Integer(other_id),),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        row.get_value(0).unwrap().as_integer().copied().unwrap()
    };
    assert_eq!(mentions, 1, "and neither are its mentions");
}
