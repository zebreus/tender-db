//! ADR-0014 unit 1: the currency_rates reference table — seed, upsert, and the
//! nearest-previous-day lookup with the daily window vs the irrevocable
//! forever-valid rule. All dormant relative to the serving path.

use store::rates::ResolvedRate;

async fn open(name: &str) -> (store::Db, String) {
    let path = format!("/tmp/tender-db-rates-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    (store::Db::open(&path).await.expect("open"), path)
}

#[tokio::test]
async fn lookup_honours_the_window_and_the_irrevocable_exemption() {
    let (db, path) = open("lookup").await;
    db.seed_irrevocable_euro_rates().await.expect("seed");
    db.upsert_currency_rates(&[
        // A Friday fixing; the following Mon/Tue have no row (weekend + holiday).
        ("PLN".into(), "2015-06-05".into(), 4.17, "ecb".into()),
        ("PLN".into(), "2015-06-30".into(), 4.19, "ecb".into()),
        // An ECU-era row.
        ("GBP".into(), "1995-03-01".into(), 0.82, "ecu".into()),
    ])
    .await
    .expect("upsert");

    // Exact day.
    let r = db.rate_to_eur("PLN", "2015-06-05").await.unwrap().expect("exact");
    assert_eq!((r.rate_to_eur, r.source.as_str()), (4.17, "ecb"));
    // Weekend rolls back to the previous business day, within the window.
    let r = db.rate_to_eur("PLN", "2015-06-08").await.unwrap().expect("rolled");
    assert_eq!(r.rate_date, "2015-06-05");
    // Beyond the 7-day window: honest absence, never a stale daily rate.
    assert_eq!(db.rate_to_eur("PLN", "2015-06-20").await.unwrap(), None);
    // The irrevocable DEM rate serves ANY later date — the currency is frozen.
    let r = db.rate_to_eur("DEM", "2001-07-15").await.unwrap().expect("frozen");
    assert_eq!(
        r,
        ResolvedRate { rate_to_eur: 1.95583, rate_date: "1999-01-01".into(), source: "irrevocable".into() }
    );
    // But NOT an earlier one: pre-adoption DEM needs the daily ECU series.
    assert_eq!(db.rate_to_eur("DEM", "1998-06-01").await.unwrap(), None);
    // EUR is identity; an unknown code is honest absence.
    assert_eq!(db.rate_to_eur("EUR", "2020-01-01").await.unwrap().unwrap().rate_to_eur, 1.0);
    assert_eq!(db.rate_to_eur("OP_DATPRO", "2020-01-01").await.unwrap(), None);

    // REPLACE semantics: a corrected day wins.
    db.upsert_currency_rates(&[("PLN".into(), "2015-06-05".into(), 4.18, "ecb".into())])
        .await
        .expect("correct");
    assert_eq!(db.rate_to_eur("PLN", "2015-06-05").await.unwrap().unwrap().rate_to_eur, 4.18);

    // The seed is idempotent.
    let n = db.seed_irrevocable_euro_rates().await.expect("reseed");
    assert_eq!(n as usize, store::rates::IRREVOCABLE_EURO_RATES.len());

    // The ECU era (ADR-0014 D2a): once the eurostat-ecu daily series is
    // loaded, a pre-adoption DEM date resolves via the DAILY row inside its
    // 7-day window — while dates before the series' local coverage stay
    // honestly absent, and post-adoption dates keep resolving irrevocable.
    db.upsert_currency_rates(&[(
        "DEM".into(),
        "1997-06-02".into(),
        1.96438,
        "eurostat-ecu".into(),
    )])
    .await
    .expect("ecu row");
    let ecu = db.rate_to_eur("DEM", "1997-06-04").await.unwrap().expect("daily ECU resolves");
    assert_eq!(
        ecu,
        store::rates::ResolvedRate {
            rate_to_eur: 1.96438,
            rate_date: "1997-06-02".into(),
            source: "eurostat-ecu".into()
        }
    );
    assert_eq!(
        db.rate_to_eur("DEM", "1996-01-15").await.unwrap(),
        None,
        "outside the daily window and before adoption — absence, not the irrevocable rate"
    );
    assert_eq!(
        db.rate_to_eur("DEM", "1999-03-01").await.unwrap().unwrap().source,
        "irrevocable",
        "the adoption-era resolution is unchanged by the ECU load"
    );

    // The legacy-code aliases (issue 172 validation pass, 2026-08-27):
    // 1993-1996 notices publish the OJ's own codes (UKL/LIT/DKR/…), which must
    // resolve against the ISO-keyed series; ECU is EUR identity by law; a
    // published typo (`GPB`, observed once) stays honestly unconvertible.
    db.upsert_currency_rates(&[(
        "GBP".into(),
        "1993-09-20".into(),
        0.77436,
        "eurostat-ecu".into(),
    )])
    .await
    .expect("gbp row");
    let ukl = db.rate_to_eur("UKL", "1993-09-21").await.unwrap().expect("UKL rides GBP");
    assert_eq!((ukl.rate_to_eur, ukl.source.as_str()), (0.77436, "eurostat-ecu"));
    assert_eq!(db.rate_to_eur("ECU", "1995-05-05").await.unwrap().unwrap().rate_to_eur, 1.0);
    assert_eq!(db.rate_to_eur("GPB", "1998-06-01").await.unwrap(), None);

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn reconcile_deletes_the_dates_a_fresh_file_disowns_and_only_its_own_source() {
    let (db, path) = open("reconcile").await;
    db.upsert_currency_rates(&[
        // Genuine rows the fresh file still carries.
        ("USD".into(), "2010-02-12".into(), 1.3572, "ecb".into()),
        // The issue-306 garbage Sunday row — absent from the real series, and
        // REPLACE alone can never remove it.
        ("USD".into(), "2010-02-14".into(), 2.0, "ecb".into()),
        // A poisoned row in a year the fresh file has no dates for at all.
        ("USD".into(), "2011-03-03".into(), 3.0, "ecb".into()),
        // Another source's row on a date the ecb file doesn't know: untouched.
        ("GBP".into(), "1995-03-01".into(), 0.82, "eurostat-ecu".into()),
    ])
    .await
    .expect("seed");

    let fresh = vec![
        ("USD".to_owned(), "2010-02-12".to_owned(), 1.3572, "ecb".to_owned()),
        ("USD".to_owned(), "2026-08-26".to_owned(), 1.1645, "ecb".to_owned()),
    ];
    let removed = db.reconcile_currency_dates("ecb", &fresh).await.expect("reconcile");
    assert_eq!(removed, 2, "the garbage Sunday row and the orphaned-year row");

    // The Sunday now rolls back to Friday's genuine fixing instead of hitting
    // the garbage rate.
    let r = db.rate_to_eur("USD", "2010-02-14").await.unwrap().expect("rolled");
    assert_eq!((r.rate_to_eur, r.rate_date.as_str()), (1.3572, "2010-02-12"));
    assert_eq!(db.rate_to_eur("USD", "2011-03-05").await.unwrap(), None, "orphaned year swept");
    let ecu = db.rate_to_eur("GBP", "1995-03-01").await.unwrap();
    assert_eq!(ecu.expect("other source intact").rate_to_eur, 0.82);

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn rederive_walks_all_four_loci_by_rowid_and_only_writes_changes() {
    // The issue-306 repair: wrong stored eur_cents corrected, missing ones
    // filled, underivable garbage NULLed — via rowid-addressed UPDATEs, which
    // this test also pins as working on turso's STRICT tables.
    let (db, path) = open("rederive").await;
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (1, 'ted', 'p1', 'contract', 1, 1266278400, 0)",
        (),
    )
    .await
    .unwrap();
    // Published 2010-02-16 — inside the incident's garbage week.
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (1, 1, 1266278400, 'OJ 1', 1)",
        (),
    )
    .await
    .unwrap();
    for sql in [
        // Garbage-rate derivation to CORRECT (stored 7777, true 5000).
        "INSERT INTO tender_version_amounts (tender_id, seq, field, cents, currency, eur_cents)
         VALUES (1, 1, 'estimated', 10000, 'USD', 7777)",
        // Honest NULL to FILL now that the rate exists.
        "INSERT INTO tender_version_amounts (tender_id, seq, field, cents, currency, eur_cents)
         VALUES (1, 1, 'total', 30000, 'USD', NULL)",
        // Unconvertible currency, already NULL: no write at all.
        "INSERT INTO tender_version_amounts (tender_id, seq, field, cents, currency, eur_cents)
         VALUES (1, 1, 'other', 5000, 'XYZ', NULL)",
        "INSERT INTO tender_version_lot_results (tender_id, seq, lot_result_id, decision, awarded_cents, awarded_currency, awarded_eur_cents)
         VALUES (1, 1, 1, 'awarded', 20000, 'USD', NULL)",
        "INSERT INTO tender_version_bids (tender_id, seq, bid_id, cents, currency, eur_cents)
         VALUES (1, 1, 1, 40000, 'USD', 1)",
        // No cents at all: stored garbage must be NULLed, not recomputed.
        "INSERT INTO tender_version_bids (tender_id, seq, bid_id, cents, currency, eur_cents)
         VALUES (1, 1, 2, NULL, 'USD', 999)",
        "INSERT INTO tender_version_contracts (tender_id, seq, contract_id, cents, currency, eur_cents)
         VALUES (1, 1, 1, 60000, 'USD', 12345)",
    ] {
        conn.execute(sql, ()).await.unwrap();
    }

    // Friday 2010-02-12's REAL fixing, within the 7-day window of the 16th.
    db.upsert_currency_rates(&[("USD".into(), "2010-02-12".into(), 2.0, "ecb".into())])
        .await
        .expect("rate");
    db.reload_rates_lookup().await.expect("reload");
    let rates = db.rates_lookup();

    let mut total_scanned = 0i64;
    let mut total_updated = 0i64;
    {
        let mut watermark = 0i64;
        loop {
            // batch=1 (one tender per window) forces the watermark loop to
            // iterate; the fixture's single tender means one full window.
            let (tenders, scanned, updated, next) =
                db.rederive_eur_window(&rates, 1, watermark).await.expect("window");
            if tenders == 0 {
                break;
            }
            total_scanned += scanned;
            total_updated += updated;
            watermark = next;
        }
    }
    assert_eq!(total_scanned, 7, "every money row visited");
    assert_eq!(total_updated, 6, "the already-correct XYZ row is not rewritten");

    let expect = [
        ("tender_version_amounts", "field = 'estimated'", Some(5000i64)),
        ("tender_version_amounts", "field = 'total'", Some(15000)),
        ("tender_version_amounts", "field = 'other'", None),
        ("tender_version_lot_results", "lot_result_id = 1", Some(10000)),
        ("tender_version_bids", "bid_id = 1", Some(20000)),
        ("tender_version_bids", "bid_id = 2", None),
        ("tender_version_contracts", "contract_id = 1", Some(30000)),
    ];
    for (table, cond, want) in expect {
        let col = if table == "tender_version_lot_results" { "awarded_eur_cents" } else { "eur_cents" };
        let mut rows =
            conn.query(&format!("SELECT {col} FROM {table} WHERE {cond}"), ()).await.unwrap();
        let row = rows.next().await.unwrap().expect("row");
        let got = match row.get_value(0).unwrap() {
            store::turso::Value::Integer(i) => Some(i),
            store::turso::Value::Null => None,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(got, want, "{table} WHERE {cond}");
    }

    // The persisted resume point (issue 306): survives round-trips, clears to 0.
    assert_eq!(db.rederive_watermark().await.unwrap(), 0, "no walk in flight");
    db.set_rederive_watermark(42).await.unwrap();
    assert_eq!(db.rederive_watermark().await.unwrap(), 42);
    db.set_rederive_watermark(0).await.unwrap();
    assert_eq!(db.rederive_watermark().await.unwrap(), 0);

    // Idempotence: the repaired layer re-derives to itself.
    let mut second_pass = 0i64;
    {
        let mut watermark = 0i64;
        loop {
            let (tenders, _, updated, next) =
                db.rederive_eur_window(&rates, 100, watermark).await.expect("window");
            if tenders == 0 {
                break;
            }
            second_pass += updated;
            watermark = next;
        }
    }
    assert_eq!(second_pass, 0, "a second walk writes nothing");

    drop(conn);
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
