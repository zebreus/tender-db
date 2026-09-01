//! Issue 334: for the widest name keys, does the stored org name match what the
//! notices actually said?
//!
//! Issue 332's census listed the widest key in the corpus as
//! `avenue web systèmes` — 62,084 org rows holding 62,080 DISTINCT identifiers.
//! That is a French e-procurement vendor's name, so the name field looks to have
//! taken the platform while the identifier took the buyer. But a count cannot say
//! whether the name is wrong or the identifier is, nor whether the notice
//! published it that way. `organization_mentions.name` can.
//!
//! What these tests pin: that `published` and `replaced` are decided from the
//! mentions rather than guessed, that a carrier with no mentions abstains, and
//! that the probe examines the WIDEST keys rather than whichever it met first —
//! because meeting the wrong ones would answer a different question entirely.

use store::turso::{self, Value};

/// Stand-in for `project::match_norm` — `ingest` depends on `store`, so the real
/// one cannot be imported here. Same shape: every non-alphanumeric is a gap.
fn norm(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
        (),
    )
    .await
    .unwrap();
    (db, conn)
}

/// An org with `stored` as its canonical name, carrying `key`, and one mention per
/// entry in `said` — which is what the notices called it.
async fn org(
    conn: &turso::Connection,
    id: i64,
    key: &str,
    stored: &str,
    said: &[&str],
) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, 'FR', 'national', 'FR' || ?, ?, 0, 0)",
        (Value::Integer(id), Value::Integer(id), Value::Text(stored.into())),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
        (Value::Integer(id), Value::Text(key.into())),
    )
    .await
    .unwrap();
    for (i, m) in said.iter().enumerate() {
        let notice = id * 1000 + i as i64;
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', 'pub-' || ?, 'h' || ?, 'eforms', 1, 'm', 0, 'parsed', 1)",
            (Value::Integer(notice), Value::Integer(notice), Value::Integer(notice)),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'ORG-' || ?, ?, ?, 'FR', NULL)",
            (
                Value::Integer(notice),
                Value::Integer(notice),
                Value::Integer(id),
                Value::Text((*m).into()),
            ),
        )
        .await
        .unwrap();
    }
}

fn never() -> bool {
    false
}

fn nowhere(_done: u64, _detail: &str) {}

const CAP: usize = 3;
const WIDE: usize = 10_000;

async fn run(db: &store::Db, keys: usize, per_key: usize) -> store::NameAttributionReport {
    db.name_attribution_probe(norm, CAP, keys, per_key, WIDE, &never, &nowhere).await.unwrap()
}

#[tokio::test]
async fn a_name_the_notices_agree_with_is_published() {
    let (db, conn) = open("na-published").await;
    for n in 1..=5i64 {
        org(&conn, n, "tribunal administratif", "Tribunal Administratif", &["Tribunal Administratif"])
            .await;
    }
    let r = run(&db, 10, 100).await;
    assert_eq!(r.keys_examined, 1);
    assert_eq!(r.agrees, 5);
    assert_eq!(r.differs, 0);
    assert_eq!(r.published_keys, 1);
    assert_eq!(r.rows[0].verdict, "published");
    assert!(r.rows[0].examples.is_empty(), "nothing to show when nothing differs");
}

#[tokio::test]
async fn a_name_no_notice_ever_used_is_replaced() {
    let (db, conn) = open("na-replaced").await;
    // THE ISSUE-334 SHAPE. Every row stores the platform's name; every notice
    // named the actual buyer. If prod looks like this, something downstream put
    // the vendor there and it is ours to fix.
    for n in 1..=5i64 {
        org(
            &conn,
            n,
            "avenue web systemes",
            "Avenue Web Systèmes",
            &[&format!("Mairie de Commune {n}")],
        )
        .await;
    }
    let r = run(&db, 10, 100).await;
    assert_eq!(r.differs, 5);
    assert_eq!(r.agrees, 0);
    assert_eq!(r.replaced_keys, 1);
    assert_eq!(r.rows[0].verdict, "replaced");
    // The examples are the point: counts say how big, only values say what
    // happened, and a reader must be able to see both sides of the swap.
    assert_eq!(r.rows[0].examples.len(), 3, "capped at three, and present");
    assert_eq!(r.rows[0].examples[0].0, "Avenue Web Systèmes");
    assert!(r.rows[0].examples[0].1.starts_with("Mairie de Commune"));
}

#[tokio::test]
async fn one_disagreeing_carrier_makes_the_key_mixed_not_published() {
    let (db, conn) = open("na-mixed").await;
    // `replaced` and `published` both claim something about EVERY carrier, so a
    // single dissenter has to demote the key rather than be rounded away.
    for n in 1..=4i64 {
        org(&conn, n, "groupe scolaire", "Groupe Scolaire", &["Groupe Scolaire"]).await;
    }
    org(&conn, 5, "groupe scolaire", "Groupe Scolaire", &["Ecole Jules Ferry"]).await;
    let r = run(&db, 10, 100).await;
    assert_eq!(r.agrees, 4);
    assert_eq!(r.differs, 1);
    assert_eq!(r.mixed_keys, 1);
    assert_eq!(r.published_keys, 0);
    assert_eq!(r.rows[0].verdict, "mixed");
}

#[tokio::test]
async fn a_carrier_with_no_mentions_abstains() {
    let (db, conn) = open("na-silent").await;
    for n in 1..=4i64 {
        org(&conn, n, "sans suite", "Sans Suite", &[]).await;
    }
    let r = run(&db, 10, 100).await;
    assert_eq!(r.silent, 4);
    assert_eq!(r.agrees, 0);
    assert_eq!(r.differs, 0);
    assert_eq!(r.no_evidence_keys, 1);
    assert_eq!(r.rows[0].verdict, "no-evidence");
}

#[tokio::test]
async fn the_probe_examines_the_widest_keys_and_not_the_first_it_meets() {
    let (db, conn) = open("na-widest").await;
    // Asking the wrong keys answers a different question. `aaa` sorts first and
    // is narrow; `zzz` sorts last and is wide. With room for one key, the probe
    // must take `zzz`.
    for n in 1..=4i64 {
        org(&conn, n, "aaa narrow", "Aaa Narrow", &["Aaa Narrow"]).await;
    }
    for n in 100..=112i64 {
        org(&conn, n, "zzz wide", "Zzz Wide", &["Something Else"]).await;
    }
    let r = run(&db, 1, 100).await;
    assert_eq!(r.keys_examined, 1);
    assert_eq!(r.rows[0].key, "zzz wide");
    assert_eq!(r.rows[0].carriers, 13);
    assert_eq!(r.rows[0].verdict, "replaced");
}

#[tokio::test]
async fn the_per_key_sample_bounds_the_carriers_read_and_says_so() {
    let (db, conn) = open("na-sample").await;
    for n in 1..=12i64 {
        org(&conn, n, "grande ville", "Grande Ville", &["Grande Ville"]).await;
    }
    let r = run(&db, 10, 5).await;
    // `carriers` is the true width; `sampled` is what was actually asked. Both
    // are reported so a reader is never left inferring one from the other.
    assert_eq!(r.rows[0].carriers, 12);
    assert_eq!(r.rows[0].sampled, 5);
    assert_eq!(r.sampled, 5);
}

#[tokio::test]
async fn a_stop_request_returns_stopped_and_no_half_report() {
    let (db, conn) = open("na-stop").await;
    for n in 1..=5i64 {
        org(&conn, n, "alpha", "Alpha", &["Alpha"]).await;
    }
    fn always() -> bool {
        true
    }
    let r = db
        .name_attribution_probe(norm, CAP, 10, 100, WIDE, &always, &nowhere)
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!(r.keys_examined, 0);
    assert!(r.rows.is_empty());
}
