//! Issue 331: does duplication actually push a name key over the genericness
//! wall?
//!
//! `STOPLIST_CAP` counts carriers as org ROWS, so a fragmented organization is
//! several carriers of its own name — and the duplicates the merge arms exist to
//! fold can push a distinctive name over the wall that then blocks the merge.
//! Self-reinforcing, and a false "generic" is a SILENT refusal in all three
//! places the wall is armed (R3's corroboration, the resolver's anchor bind, the
//! E3 scan).
//!
//! Issue 312's precedent is the reason this is a census and not a fix: a
//! mechanism is not a frequency. What these tests pin is that the census can
//! tell the three cases apart — a key the collapse rescues, a key that is
//! genuinely generic and stays so, and a key that was never near the wall — and
//! that savings are counted per group rather than across unrelated ones.

use store::turso::{self, Value};

// STAND-INS for the injected normalisers. `ingest` depends on `store`, so the
// real `match_norm` and `n3_key` cannot be imported here; they are unit-tested
// in their own crate, and what belongs here is the surrounding judgement.
fn n2(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Abstracts the legal-form family, as `n3_key` does — so it differs from `n2`
/// exactly when the name carries one, which is what decides whether the build
/// would have written an `n3` row at all.
fn n3(name: &str) -> String {
    n2(name)
        .split(' ')
        .map(|t| if t == "gmbh" { "§gmbh".to_owned() } else { t.to_owned() })
        .collect::<Vec<String>>()
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
    (db, conn)
}

/// An org row, and the `org_match_keys` rows the build would have written for
/// it — `n2` always, `n3` only when it differs, which is what the real build
/// does and what the census has to assume.
async fn org(conn: &turso::Connection, id: i64, cc: &str, ident: &str, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, 'vat', ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(cc.into()),
            Value::Text(ident.into()),
            Value::Text(name.into()),
        ),
    )
    .await
    .unwrap();
    let k2 = n2(name);
    let k3 = n3(name);
    conn.execute(
        "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
        (Value::Integer(id), Value::Text(k2.clone().into())),
    )
    .await
    .unwrap();
    if k3 != k2 {
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n3', ?)",
            (Value::Integer(id), Value::Text(k3.into())),
        )
        .await
        .unwrap();
    }
}

fn never() -> bool {
    false
}

fn nowhere(_done: u64, _detail: &str) {}

const CAP: usize = 5;

async fn run(db: &store::Db) -> store::GenericWallReport {
    db.generic_wall_inflation_census(n2, n3, CAP, 50, &never, &nowhere).await.unwrap()
}

#[tokio::test]
async fn without_duplicates_nothing_is_inflated() {
    let (db, conn) = open("gw-none").await;
    // Six DIFFERENT bodies sharing a name, each with its own identifier. Over
    // the cap of five, and legitimately so — this is what the wall is for.
    for n in 1..=6i64 {
        org(&conn, n, "DE", &format!("DE{n:09}"), "Stadtverwaltung").await;
    }
    let r = run(&db).await;
    assert_eq!(r.duplicate_groups, 0);
    assert_eq!(r.keys_with_savings, 0, "no group carries a key twice, so nothing collapses");
    assert_eq!(r.keys_falsely_generic, 0);
}

#[tokio::test]
async fn a_key_the_collapse_rescues_is_falsely_generic() {
    let (db, conn) = open("gw-rescued").await;
    // THE CLASS. Four genuinely distinct bodies named "Hüther", plus ONE more
    // that is fragmented across three rows under a single identifier. Raw
    // carriers = 6, over the cap of five; collapsed = 4, under it. The wall is
    // refusing this name because of the very fragmentation a merge would fix.
    for n in 1..=4i64 {
        org(&conn, n, "DE", &format!("DE{n:09}"), "Hüther").await;
    }
    for n in 10..=12i64 {
        org(&conn, n, "DE", "DE999999999", "Hüther").await;
    }
    let r = run(&db).await;
    assert_eq!(r.duplicate_groups, 1);
    assert_eq!(r.duplicate_rows, 3);
    assert_eq!(r.keys_falsely_generic, 1);
    assert_eq!(r.keys_over_cap, 1);
    let row = r.rows.iter().find(|k| k.verdict == "falsely-generic").expect("the rescued key");
    assert_eq!(row.carriers, 7, "seven org rows carry the n2 key today");
    assert_eq!(row.savings, 2, "three rows under one identity collapse to one");
    assert_eq!(row.collapsed, 5, "at the cap, so no longer over it");
}

#[tokio::test]
async fn a_genuinely_generic_key_stays_generic_and_reports_its_distance() {
    let (db, conn) = open("gw-stays").await;
    // Ten distinct bodies plus one fragmented pair. Collapsing saves one and
    // leaves ten — still well over the cap. `nearest_miss` is what tells a
    // reader HOW FAR the class is from mattering, rather than only that it is.
    for n in 1..=10i64 {
        org(&conn, n, "DE", &format!("DE{n:09}"), "Gemeinde").await;
    }
    for n in 20..=21i64 {
        org(&conn, n, "DE", "DE888888888", "Gemeinde").await;
    }
    let r = run(&db).await;
    assert_eq!(r.keys_falsely_generic, 0);
    assert_eq!(r.keys_over_cap, 1);
    assert_eq!(r.nearest_miss, 11, "eleven collapsed carriers against a cap of five");
    assert_eq!(r.rows[0].verdict, "still-generic");
}

#[tokio::test]
async fn savings_are_counted_inside_a_group_and_never_across_two() {
    let (db, conn) = open("gw-pergroup").await;
    // Two SEPARATE fragmented organizations that happen to share a name. Each
    // collapses within itself — 1 saving apiece — and the two must not be
    // collapsed into one another, which would be a false merge dressed up as
    // arithmetic and would understate the wall.
    org(&conn, 1, "DE", "DE111111111", "Müller Bau GmbH").await;
    org(&conn, 2, "DE", "DE111111111", "Müller Bau GmbH").await;
    org(&conn, 3, "DE", "DE222222222", "Müller Bau GmbH").await;
    org(&conn, 4, "DE", "DE222222222", "Müller Bau GmbH").await;
    let r = run(&db).await;
    assert_eq!(r.duplicate_groups, 2);
    assert_eq!(r.duplicate_rows, 4);
    // Both the n2 and the n3 key are carried twice per group, so each gains 2.
    for row in &r.rows {
        assert_eq!(row.savings, 2, "one per group, two groups — not three");
        assert_eq!(row.carriers, 4);
        assert_eq!(row.verdict, "never-generic", "four carriers is under the cap of five");
    }
    assert_eq!(r.keys_with_savings, 2, "the n2 key and the n3 key");
    assert_eq!(r.keys_falsely_generic, 0);
}

#[tokio::test]
async fn a_name_with_no_legal_form_is_found_under_n2_alone() {
    let (db, conn) = open("gw-n2only").await;
    // The issue-329 trap, re-pinned here: the build writes an `n3` row only when
    // it differs from `n2`, so a name carrying no legal-form token exists under
    // `n2` alone. A census that looked only under `n3` would see zero carriers
    // and silently report nothing inflated.
    for n in 1..=6i64 {
        org(&conn, n, "DE", &format!("DE{n:09}"), "Kreisverwaltung Ahrweiler").await;
    }
    for n in 10..=12i64 {
        org(&conn, n, "DE", "DE777777777", "Kreisverwaltung Ahrweiler").await;
    }
    assert_eq!(n3("Kreisverwaltung Ahrweiler"), n2("Kreisverwaltung Ahrweiler"));
    let r = run(&db).await;
    assert_eq!(r.keys_with_savings, 1, "one key, stored under n2");
    assert_eq!(r.rows[0].carriers, 9, "found despite there being no n3 row at all");
    assert_eq!(r.rows[0].savings, 2);
}

#[tokio::test]
async fn a_stop_request_returns_stopped_and_no_half_report() {
    let (db, conn) = open("gw-stop").await;
    org(&conn, 1, "DE", "DE111111111", "Alpha GmbH").await;
    org(&conn, 2, "DE", "DE111111111", "Alpha GmbH").await;
    fn always() -> bool {
        true
    }
    let r = db
        .generic_wall_inflation_census(n2, n3, CAP, 50, &always, &nowhere)
        .await
        .unwrap();
    assert!(r.stopped);
    // Issue 252's honest cancel.
    assert_eq!(r.duplicate_groups, 0);
    assert!(r.rows.is_empty());
}
