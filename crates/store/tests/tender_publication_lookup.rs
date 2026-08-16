//! Issue 217-A: `/v1/tenders?publication_id=` — the official notice number
//! resolving to its Tender. The load-bearing properties: the seed is the
//! PREDICATE (a superseded version's number still resolves its tender; a
//! multi-version tender is not duplicated), it composes with the participation
//! EXISTS filters it displaced from the seed slot, and it rides the ordered
//! shape too, because both list shapes share `tender_from`.

use store::read::{self, Filter, HeadOrder, Scope};
use store::turso::{self, Value};

async fn open(name: &str) -> turso::Connection {
    let path = format!("/tmp/tender-db-publook-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn
}

async fn version(conn: &turso::Connection, tender: i64, seq: i64, publication: &str) {
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (?, ?, ?, ?, ?)",
        (
            Value::Integer(tender),
            Value::Integer(seq),
            Value::Integer(100 * tender + seq),
            Value::Text(publication.into()),
            Value::Integer(10 * tender + seq),
        ),
    )
    .await
    .unwrap();
}

async fn seed(conn: &turso::Connection) {
    for (id, current_seq) in [(1i64, 2i64), (2, 1), (3, 1)] {
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, 'procedure', ?, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(format!("pk-{id}")),
                Value::Integer(current_seq),
                Value::Integer(100 * id + current_seq),
            ),
        )
        .await
        .unwrap();
    }
    // Tender 1: two versions — the number that caused seq 1 is superseded.
    version(conn, 1, 1, "OLD-1").await;
    version(conn, 1, 2, "NEW-1").await;
    // Tenders 2 and 3 share a number (a corrigendum republished, say).
    version(conn, 2, 1, "SHARED").await;
    version(conn, 3, 1, "SHARED").await;
    // Org 7 won something on tender 2's current version only.
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
         VALUES (2, 1, 1, 7)",
        (),
    )
    .await
    .unwrap();
}

async fn ids(conn: &turso::Connection, filter: &Filter) -> Vec<i64> {
    read::tenders(conn, filter, Scope::Page { after: 0, limit: 100 })
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect()
}

fn by(publication: &str) -> Filter {
    Filter { publication_id: Some(publication.into()), ..Filter::default() }
}

#[tokio::test]
async fn the_official_number_resolves_its_tender() {
    let conn = open("resolve").await;
    seed(&conn).await;

    // The current version's number, and a SUPERSEDED version's number, both
    // resolve the tender — the number names the notice, the notice names the
    // tender, and history does not un-name it.
    assert_eq!(ids(&conn, &by("NEW-1")).await, vec![1]);
    assert_eq!(ids(&conn, &by("OLD-1")).await, vec![1]);
    // A two-version tender comes back once (DISTINCT in the seed)…
    assert_eq!(ids(&conn, &Filter { publication_id: Some("NEW-1".into()), source: Some("ted".into()), ..Filter::default() }).await, vec![1]);
    // …a shared number returns every tender it caused, an unknown one nothing.
    assert_eq!(ids(&conn, &by("SHARED")).await, vec![2, 3]);
    assert!(ids(&conn, &by("NOPE")).await.is_empty());

    // The seed displaced the participation seed, but its EXISTS still narrows:
    // of the two SHARED tenders only tender 2 has org 7 as a winner.
    let winner = Filter { winner: Some(7), ..by("SHARED") };
    assert_eq!(ids(&conn, &winner).await, vec![2]);
    // And a wrong-source companion excludes exactly like before the seed.
    let wrong = Filter { source: Some("doe".into()), ..by("SHARED") };
    assert!(ids(&conn, &wrong).await.is_empty());
}

#[tokio::test]
async fn the_ordered_shape_applies_the_number_too() {
    let conn = open("ordered").await;
    seed(&conn).await;

    // Both list shapes share `tender_from`, so sort=published_at composes with
    // the lookup; honesty (issue 118) says honoured means honoured on EVERY path.
    let rows = read::tenders_ordered(&conn, &by("SHARED"), HeadOrder::PublishedAt, true, None, 100)
        .await
        .unwrap();
    let got: Vec<i64> = rows.into_iter().map(|r| r.id).collect();
    assert_eq!(got, vec![3, 2], "newest first within the number's tenders");
}
