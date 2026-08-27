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
