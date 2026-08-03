//! Task 16: a fixture that can actually DISCRIMINATE on every `lots` filter.
//!
//! The combination-equivalence test #16 needs before shipping is only as good as the
//! data under it. run-driver's 40.6M bed had `tenders.source` at cardinality 1 — every
//! row `'ted'` — so `?source=` matched everything or nothing and an equivalence check
//! across it would have **passed while testing nothing**. That is the failure this file
//! exists to prevent on my side before I write the equivalence assertions.
//!
//! So this test asserts the *fixture*, not the query: for every filter the read layer
//! supports, the fixture must return a **proper non-empty subset** — neither all lots
//! nor none. A filter that selects everything cannot detect a candidate shape that
//! drops the predicate; one that selects nothing cannot detect a shape that returns the
//! wrong rows. Both pass an equivalence check vacuously.
//!
//! The satellites deliberately DIFFER between the current and superseded versions, so a
//! shape that resolves the wrong `seq` produces different rows rather than the same
//! ones — the discriminator the single-version fixture lacked.

use store::read::{Filter, Scope, Status};
use store::turso::Value;

const TENDERS: i64 = 60;
const LOTS_PER: i64 = 4;
const SEQS: i64 = 3; // seq 3 is current; 1 and 2 are superseded

async fn seed(path: &str) -> store::turso::Connection {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(path).await.unwrap();
    let db = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute("BEGIN", ()).await.unwrap();

    for org in 1..=4i64 {
        conn.execute(
            "INSERT INTO organizations (id, name, country, provisional, created_at)
             VALUES (?, ?, 'DE', 0, 1700000000)",
            (Value::Integer(org), Value::Text(format!("org-{org}"))),
        ).await.unwrap();
    }

    for t in 1..=TENDERS {
        // Two sources at a realistic skew, so `?source=` is a proper subset both ways.
        let source = if t % 8 == 0 { "doe" } else { "ted" };
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, ?, ?, 'procedure', ?, 1700000000, 1700000000)",
            (Value::Integer(t), Value::Text(source.into()),
             Value::Text(format!("pk-{t}")), Value::Integer(SEQS)),
        ).await.unwrap();
        for seq in 1..=SEQS {
            conn.execute(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
                 VALUES (?, ?, 1700000000, ?, ?)",
                (Value::Integer(t), Value::Integer(seq),
                 Value::Text(format!("pub-{t}-{seq}")), Value::Integer((t - 1) * SEQS + seq)),
            ).await.unwrap();

            // Satellites DIFFER by version. Superseded rows carry values that would
            // match a filter the current version does not — so a shape resolving the
            // wrong seq returns different rows rather than the same ones.
            let current = seq == SEQS;
            let (nuts, cpv) = if current {
                (if t % 3 == 0 { "DE300" } else { "FR101" }, if t % 5 == 0 { "45200" } else { "72000" })
            } else {
                ("ZZ999", "99999")
            };
            for (field, scheme, code) in
                [("place", "nuts", nuts), ("main", "cpv", cpv)]
            {
                conn.execute(
                    "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
                     VALUES (?, ?, NULL, ?, ?, ?)",
                    (Value::Integer(t), Value::Integer(seq), Value::Text(field.into()),
                     Value::Text(scheme.into()), Value::Text(code.into())),
                ).await.unwrap();
            }
            let buyer = if current { if t % 4 == 0 { 1 } else { 2 } } else { 3 };
            conn.execute(
                "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id)
                 VALUES (?, ?, NULL, 'Buyer', ?, 1, 's')",
                (Value::Integer(t), Value::Integer(seq), Value::Integer(buyer)),
            ).await.unwrap();
            // Value bounds: current versions straddle a threshold, superseded do not.
            let cents = if current { if t % 6 == 0 { 5_000_00 } else { 100_00 } } else { 9_999_999_00 };
            conn.execute(
                "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
                 VALUES (?, ?, NULL, 'estimated', ?, 'EUR')",
                (Value::Integer(t), Value::Integer(seq), Value::Integer(cents)),
            ).await.unwrap();
            // Deadlines: some in the past, some in the future, so Open and Closed both
            // select proper subsets rather than everything or nothing.
            let deadline = if current {
                if t % 2 == 0 { 1_900_000_000i64 } else { 1_600_000_000 }
            } else {
                1_500_000_000
            };
            conn.execute(
                "INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
                 VALUES (?, ?, NULL, 'submission_deadline', ?, 0, 1)",
                (Value::Integer(t), Value::Integer(seq), Value::Integer(deadline)),
            ).await.unwrap();

            for k in 1..=LOTS_PER {
                let lot = (t - 1) * LOTS_PER + k;
                if seq == 1 {
                    conn.execute("INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
                        (Value::Integer(lot), Value::Integer(t), Value::Text(format!("L-{lot}")))).await.unwrap();
                }
                // `kind` differs by version: superseded says Part, current says Lot for
                // most lots and LotsGroup for a few. So `?kind=Part` must return NOTHING
                // (every Part is superseded) — the seq discriminator.
                let kind = if !current { "Part" } else if lot % 7 == 0 { "LotsGroup" } else { "Lot" };
                conn.execute(
                    "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, ?, ?, ?)",
                    (Value::Integer(t), Value::Integer(seq), Value::Integer(lot), Value::Text(kind.into())),
                ).await.unwrap();
            }
        }
    }
    conn.execute("COMMIT", ()).await.unwrap();
    conn
}

fn base() -> Filter {
    Filter { now: 1_700_000_000, ..Filter::default() }
}

/// Every filter must select a PROPER NON-EMPTY SUBSET. Anything else cannot
/// discriminate, and an equivalence test built on it would pass without testing.
#[tokio::test]
async fn every_lots_filter_selects_a_proper_subset() {
    let path = format!("/tmp/tender-db-filterfix-{}.db", std::process::id());
    let conn = seed(&path).await;
    let all = TENDERS * LOTS_PER;

    let page = Scope::Page { after: 0, limit: all + 10 };
    let total = store::read::lots(&conn, &base(), page).await.unwrap().len() as i64;
    assert_eq!(total, all, "unfiltered must return every lot");

    let cases: Vec<(&str, Filter)> = vec![
        ("kind=Lot", Filter { kind: Some("Lot".into()), ..base() }),
        ("kind=LotsGroup", Filter { kind: Some("LotsGroup".into()), ..base() }),
        ("source=ted", Filter { source: Some("ted".into()), ..base() }),
        ("source=doe", Filter { source: Some("doe".into()), ..base() }),
        ("country=DE", Filter { country: Some("DE".into()), ..base() }),
        ("cpv=452", Filter { cpv: Some("452".into()), ..base() }),
        ("buyer=1", Filter { buyer: Some(1), ..base() }),
        ("status=Open", Filter { status: Some(Status::Open), ..base() }),
        ("status=Closed", Filter { status: Some(Status::Closed), ..base() }),
        ("min_value", Filter { min_value: Some(1_000_00), ..base() }),
        ("max_value", Filter { max_value: Some(1_000_00), ..base() }),
        ("kind=Lot+source=doe", Filter { kind: Some("Lot".into()), source: Some("doe".into()), ..base() }),
        ("kind=Lot+country=DE", Filter { kind: Some("Lot".into()), country: Some("DE".into()), ..base() }),
    ];

    println!("\n{:<24}{:>8}  {}", "filter", "rows", "verdict");
    let mut bad = Vec::new();
    for (name, f) in &cases {
        let n = store::read::lots(&conn, f, page).await.unwrap().len() as i64;
        let ok = n > 0 && n < total;
        println!("{name:<24}{n:>8}  {}", if ok { "proper subset" } else { "CANNOT DISCRIMINATE" });
        if !ok {
            bad.push(format!("{name} -> {n} of {total}"));
        }
    }

    // The seq discriminator: `Part` exists only on superseded versions, so a correct
    // read returns none. This one is SUPPOSED to be empty and is asserted separately,
    // because it tests the opposite property from the subset check above.
    let superseded = Filter { kind: Some("Part".into()), ..base() };
    let n = store::read::lots(&conn, &superseded, page).await.unwrap().len();
    println!("{:<24}{n:>8}  {}", "kind=Part (superseded)", "must be 0 — seq discriminator");
    assert_eq!(n, 0, "Part exists only on superseded versions; a correct read returns none");

    assert!(bad.is_empty(), "filters that cannot discriminate: {bad:?}");
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}


/// **The equivalence gate.** `lots_s2c` must return exactly what `lots` returns, for
/// every filter and combination, on data where each filter provably discriminates.
///
/// Both sides are real read paths — `store::read::lots` and `store::read::lots_s2c` —
/// not hand-written SQL. A test comparing today's builder against an imitation of the
/// builder I intend to write would pass while `read.rs` shipped something else, which
/// is the artifact-versus-proxy gap that cost the issue-16 shape detour.
#[tokio::test]
async fn the_s2c_candidate_answers_identically_across_the_filter_surface() {
    let path = format!("/tmp/tender-db-s2ceq-{}.db", std::process::id());
    let conn = seed(&path).await;
    let all = TENDERS * LOTS_PER;

    let mut cases: Vec<(String, Filter)> = vec![("unfiltered".into(), base())];
    for (name, f) in [
        ("kind=Lot", Filter { kind: Some("Lot".into()), ..base() }),
        ("kind=LotsGroup", Filter { kind: Some("LotsGroup".into()), ..base() }),
        ("kind=Part(superseded)", Filter { kind: Some("Part".into()), ..base() }),
        ("kind=zzz", Filter { kind: Some("zzz".into()), ..base() }),
        ("source=ted", Filter { source: Some("ted".into()), ..base() }),
        ("source=doe", Filter { source: Some("doe".into()), ..base() }),
        ("country=DE", Filter { country: Some("DE".into()), ..base() }),
        ("cpv=452", Filter { cpv: Some("452".into()), ..base() }),
        ("buyer=1", Filter { buyer: Some(1), ..base() }),
        ("status=Open", Filter { status: Some(Status::Open), ..base() }),
        ("status=Closed(NOT EXISTS)", Filter { status: Some(Status::Closed), ..base() }),
        ("min_value", Filter { min_value: Some(1_000_00), ..base() }),
        ("max_value", Filter { max_value: Some(1_000_00), ..base() }),
        ("kind+source", Filter { kind: Some("Lot".into()), source: Some("doe".into()), ..base() }),
        ("kind+country", Filter { kind: Some("Lot".into()), country: Some("DE".into()), ..base() }),
        ("kind+source+country", Filter { kind: Some("Lot".into()), source: Some("ted".into()),
                                          country: Some("DE".into()), ..base() }),
        ("kind+status+min_value", Filter { kind: Some("Lot".into()), status: Some(Status::Open),
                                            min_value: Some(1_000_00), ..base() }),
        ("tender=3 (containment)", Filter { tender: Some(3), ..base() }),
    ] {
        cases.push((name.into(), f));
    }

    // Two cursor positions: the second lands mid-stream, where a shape that mishandles
    // the cursor returns a correct-looking but shifted page.
    println!("\n{:<28}{:>8}{:>8}  {}", "filter", "today", "s2c", "verdict");
    for (name, f) in &cases {
        for after in [0, all / 2] {
            let scope = Scope::Page { after, limit: all + 10 };
            let today = store::read::lots(&conn, f, scope).await.unwrap();
            let cand = store::read::lots_s2c(&conn, f, scope).await.unwrap();
            let key = |r: &store::read::LotRow| {
                (r.id, r.tender_id, r.lot_key.clone(), r.kind.clone(), r.seq,
                 r.title.clone(), r.value_cents, r.currency.clone())
            };
            let a: Vec<_> = today.iter().map(key).collect();
            let b: Vec<_> = cand.iter().map(key).collect();
            if after == 0 {
                println!("{name:<28}{:>8}{:>8}  {}", a.len(), b.len(),
                         if a == b { "identical" } else { "DIFFERS" });
            }
            assert_eq!(b, a, "s2c differs from today: filter={name} after={after}");
        }
    }
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}


/// Dump the GENERATED S2c statements for the version-predicate arms, so plan
/// confirmation at 40.6M plans the builder rather than a paraphrase of it.
#[tokio::test]
#[ignore = "utility: prints generated S2c SQL for off-box plan confirmation"]
async fn dump_s2c_statements_for_scale_plan_check() {
    let scope = Scope::Page { after: 0, limit: 50 };
    for (name, f) in [
        ("country", Filter { country: Some("DE".into()), ..base() }),
        ("cpv", Filter { cpv: Some("452".into()), ..base() }),
        ("buyer", Filter { buyer: Some(1), ..base() }),
        ("winner", Filter { winner: Some(1), ..base() }),
        ("status=Open", Filter { status: Some(Status::Open), ..base() }),
        ("status=Closed(NOT EXISTS)", Filter { status: Some(Status::Closed), ..base() }),
        ("min_value", Filter { min_value: Some(100000), ..base() }),
        ("max_value", Filter { max_value: Some(100000), ..base() }),
    ] {
        let (sql, params) = store::read::lots_statement_s2c(&f, scope);
        println!("\n===== {name} =====\n{sql};\n-- params: {params:?}");
    }
}
