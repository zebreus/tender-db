//! Issue 332: are over-cap name keys widely-shared names, or single identities
//! fragmented?
//!
//! The wall refuses a key carried by more than `STOPLIST_CAP` orgs, on the
//! premise that such a name is one MANY DIFFERENT BODIES CHOSE. Issue 331's run
//! printed the counts and the top of the list did not look like that —
//! `siemens §ag` at 1,238 carriers, `stadt roth` at 30 — but a carrier count
//! cannot tell a shared name from one fragmented identity.
//!
//! These tests pin that the cut can tell the two apart, that a split key run
//! across a window boundary is not miscounted (which would hide the widest keys
//! entirely), and that a key whose carriers hold no identifiers abstains instead
//! of being filed under either answer.

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

/// An org carrying `key` under kind `n2`. `ident` of `None` leaves the triple
/// incomplete, which is what most of the provisional layer looks like.
async fn carrier(conn: &turso::Connection, id: i64, key: &str, ident: Option<&str>) {
    match ident {
        Some(v) => conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
                 VALUES (?, 'DE', 'vat', ?, 'n', 0, 0)",
                (Value::Integer(id), Value::Text(v.into())),
            )
            .await
            .unwrap(),
        None => conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
                 VALUES (?, 'DE', NULL, NULL, 'n', 1, 0)",
                (Value::Integer(id),),
            )
            .await
            .unwrap(),
    };
    conn.execute(
        "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
        (Value::Integer(id), Value::Text(key.into())),
    )
    .await
    .unwrap();
}

fn never() -> bool {
    false
}

fn nowhere(_done: u64, _detail: &str) {}

const CAP: usize = 5;

/// A window far larger than any fixture here, so the paging never bites unless a
/// test asks it to.
const WIDE: usize = 10_000;

async fn run(db: &store::Db) -> store::GenericStatisticReport {
    db.genericness_statistic_census(CAP, 50, WIDE, &never, &nowhere).await.unwrap()
}

#[tokio::test]
async fn a_key_under_the_cap_is_not_examined() {
    let (db, conn) = open("gs-under").await;
    for n in 1..=5i64 {
        carrier(&conn, n, "kleine firma", Some(&format!("DE{n:09}"))).await;
    }
    let r = run(&db).await;
    assert_eq!(r.keys_walked, 1);
    assert_eq!(r.keys_over_cap, 0, "five carriers is not over a cap of five");
    assert!(r.rows.is_empty());
}

#[tokio::test]
async fn many_carriers_on_one_identity_is_a_fragmented_identity() {
    let (db, conn) = open("gs-single").await;
    // THE SHAPE THE ISSUE IS ABOUT: eight carriers, all one identity. The wall
    // calls this generic; nobody else uses the name at all.
    for n in 1..=8i64 {
        carrier(&conn, n, "siemens ag", Some("DE129274202")).await;
    }
    let r = run(&db).await;
    assert_eq!(r.keys_over_cap, 1);
    assert_eq!(r.over_cap_carriers, 8);
    assert_eq!(r.single_identity, 1);
    assert_eq!(r.mostly_distinct, 0);
    let row = &r.rows[0];
    assert_eq!(row.carriers, 8);
    assert_eq!(row.with_identifier, 8);
    assert_eq!(row.distinct_identities, 1);
    assert_eq!(row.verdict, "single-identity");
}

#[tokio::test]
async fn many_carriers_on_many_identities_is_a_genuinely_shared_name() {
    let (db, conn) = open("gs-shared").await;
    // The wall working exactly as designed: eight different bodies that each
    // chose the same generic word.
    for n in 1..=8i64 {
        carrier(&conn, n, "stadtverwaltung", Some(&format!("DE{n:09}"))).await;
    }
    let r = run(&db).await;
    assert_eq!(r.mostly_distinct, 1);
    assert_eq!(r.single_identity, 0);
    assert_eq!(r.rows[0].distinct_identities, 8);
    assert_eq!(r.rows[0].verdict, "mostly-distinct");
}

#[tokio::test]
async fn half_and_half_reads_as_mostly_fragmented() {
    let (db, conn) = open("gs-half").await;
    // Eight carriers, four identities, two rows apiece: the boundary case, and
    // it must land on the fragmented side of `distinct * 2 <= with_identifier`
    // rather than silently on whichever side the arithmetic happens to fall.
    for n in 0..8i64 {
        carrier(&conn, n + 1, "mittelstand bau", Some(&format!("DE{:09}", n / 2))).await;
    }
    let r = run(&db).await;
    assert_eq!(r.rows[0].with_identifier, 8);
    assert_eq!(r.rows[0].distinct_identities, 4);
    assert_eq!(r.mostly_fragmented, 1);
    assert_eq!(r.identity_ratio, vec![50]);
}

#[tokio::test]
async fn a_key_whose_carriers_hold_no_identifier_abstains() {
    let (db, conn) = open("gs-noid").await;
    // Most of the org layer is this: provisional rows with no triple. The cut
    // cannot decide, and saying so is the point — if this bucket dominates on
    // prod, the whole route is unanswerable and the census says that rather
    // than reporting a confident split from the decidable minority.
    for n in 1..=8i64 {
        carrier(&conn, n, "irgendein amt", None).await;
    }
    let r = run(&db).await;
    assert_eq!(r.keys_over_cap, 1);
    assert_eq!(r.no_identifiers, 1);
    assert_eq!(r.single_identity, 0);
    assert_eq!(r.mostly_distinct, 0);
    assert!(r.identity_ratio.is_empty(), "an undecidable key contributes no ratio");
    assert_eq!(r.rows[0].verdict, "no-identifiers");
}

#[tokio::test]
async fn the_widest_keys_survive_the_listing_cap() {
    let (db, conn) = open("gs-widest").await;
    // The cap must keep the keys that prompted the question — the huge ones —
    // not an alphabetical slice. `zzz` is last by key and widest by carriers.
    for n in 1..=6i64 {
        carrier(&conn, n, "aaa narrow", Some(&format!("DE1{n:08}"))).await;
    }
    for n in 100..=120i64 {
        carrier(&conn, n, "zzz wide", Some(&format!("DE2{n:08}"))).await;
    }
    let r = db.genericness_statistic_census(CAP, 1, WIDE, &never, &nowhere).await.unwrap();
    assert_eq!(r.keys_over_cap, 2, "both are tallied");
    assert!(r.truncated);
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0].key, "zzz wide", "widest first, not alphabetical");
}

#[tokio::test]
async fn a_stop_request_returns_stopped_and_no_half_report() {
    let (db, conn) = open("gs-stop").await;
    for n in 1..=8i64 {
        carrier(&conn, n, "alpha", Some("DE111111111")).await;
    }
    fn always() -> bool {
        true
    }
    let r = db.genericness_statistic_census(CAP, 50, WIDE, &always, &nowhere).await.unwrap();
    assert!(r.stopped);
    assert_eq!(r.keys_over_cap, 0);
    assert!(r.rows.is_empty());
}

#[tokio::test]
async fn a_key_run_split_across_a_window_boundary_is_still_counted_whole() {
    let (db, conn) = open("gs-split").await;
    // THE BUG THAT WOULD HIDE EXACTLY THE KEYS THIS CENSUS EXISTS TO SEE. A key
    // run spanning two pages reads as two shorter runs, so the widest keys —
    // guaranteed to span pages — would each fall under the cap and vanish. The
    // walk drops the last (possibly split) run of a full page and re-reads it
    // from that key; this drives a window of 3 across a run of 8.
    for n in 1..=8i64 {
        carrier(&conn, n, "one long run", Some("DE129274202")).await;
    }
    let r = db.genericness_statistic_census(CAP, 50, 3, &never, &nowhere).await.unwrap();
    assert_eq!(r.keys_over_cap, 1, "one key, not several short ones");
    assert_eq!(r.over_cap_carriers, 8, "all eight carriers, counted once each");
    assert_eq!(r.single_identity, 1);
    assert_eq!(r.rows[0].carriers, 8);
}

#[tokio::test]
async fn two_keys_split_across_windows_are_not_merged_into_one() {
    let (db, conn) = open("gs-split2").await;
    // The opposite error, and the reason the re-read is keyed on the KEY rather
    // than on a row offset: two adjacent runs must stay two.
    for n in 1..=7i64 {
        carrier(&conn, n, "aaa run", Some(&format!("DE1{n:08}"))).await;
    }
    for n in 20..=27i64 {
        carrier(&conn, n, "bbb run", Some("DE999999999")).await;
    }
    let r = db.genericness_statistic_census(CAP, 50, 3, &never, &nowhere).await.unwrap();
    assert_eq!(r.keys_over_cap, 2);
    assert_eq!(r.mostly_distinct, 1, "aaa run: seven bodies, seven identities");
    assert_eq!(r.single_identity, 1, "bbb run: eight rows, one identity");
    assert_eq!(r.over_cap_carriers, 15);
}

#[tokio::test]
async fn one_identified_carrier_among_many_is_not_evidence_of_fragmentation() {
    let (db, conn) = open("gs-thin").await;
    // THE BUG THE FIRST PROD RUN SHIPPED WITH. Eight carriers, exactly ONE
    // holding an identifier. `distinct == 1` is arithmetically true and says
    // nothing at all — yet it filed 8,565 keys as fragmented identities,
    // `enel spa` among them: 3,004 carriers, one identifier between them.
    //
    // The tell was that the ratio distribution read 100% at every percentile
    // while thousands of keys supposedly sat at one-identity-over-many. Reading
    // the values, not the counts, is what caught it.
    carrier(&conn, 1, "enel spa", Some("IT00811720580")).await;
    for n in 2..=8i64 {
        carrier(&conn, n, "enel spa", None).await;
    }
    let r = run(&db).await;
    assert_eq!(r.keys_over_cap, 1);
    assert_eq!(r.too_little_evidence, 1);
    assert_eq!(r.single_identity, 0, "one identified carrier is not a fragmented identity");
    assert!(r.identity_ratio.is_empty(), "and it contributes no ratio");
    assert_eq!(r.rows[0].verdict, "too-little-evidence");
}

#[tokio::test]
async fn two_identified_carriers_on_one_identity_still_reads_as_fragmented() {
    let (db, conn) = open("gs-two").await;
    // The floor of the real signal: two is enough to say the same identity
    // appears twice, which is what `single-identity` claims. The fix must not
    // have raised the bar past the shape the issue is about.
    carrier(&conn, 1, "siemens ag", Some("DE129274202")).await;
    carrier(&conn, 2, "siemens ag", Some("DE129274202")).await;
    for n in 3..=8i64 {
        carrier(&conn, n, "siemens ag", None).await;
    }
    let r = run(&db).await;
    assert_eq!(r.single_identity, 1);
    assert_eq!(r.too_little_evidence, 0);
    assert_eq!(r.identity_ratio, vec![50], "one identity over two identified carriers");
}
