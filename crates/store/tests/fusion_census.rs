//! Issue 317 Unit A: the fusion census. The campaign's flagship finding — a
//! consortium vehicle row also holding its lead member's SOLO mentions —
//! lived only in free-text handling notes. This pins the structural version:
//! a mention whose published name is not the row's name is evidence about
//! somebody else, counted and NAMED so a reviewer can act on it.

use store::CaseReview;
use store::turso::Value;

/// The shared N2 key's shape: lowercase, alphanumeric only.
fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect()
}

async fn org(conn: &store::turso::Connection, id: i64, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, 'DE', 'national', NULL, ?, ?, 0, 0)",
        (Value::Integer(id), Value::Text(name.into()), Value::Text(name.to_lowercase())),
    )
    .await
    .unwrap();
}

async fn mention(conn: &store::turso::Connection, notice: i64, org: i64, name: Option<&str>) {
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (?, 'S-1', ?, ?, NULL, NULL)",
        (
            Value::Integer(notice),
            Value::Integer(org),
            match name {
                Some(n) => Value::Text(n.into()),
                None => Value::Null,
            },
        ),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn the_census_finds_rows_holding_somebody_elses_mentions() {
    let path = "test-fusion-census.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // 40: THE FUSION SHAPE — a vehicle whose mentions mostly name the member.
    org(&conn, 40, "Bietergemeinschaft Dobler / Huber").await;
    // 41: a clean vehicle — every mention names it, modulo spacing and case.
    org(&conn, 41, "Bietergemeinschaft Sauber GmbH").await;
    // 42: reviewed but never applied — outside this census.
    org(&conn, 42, "Bietergemeinschaft Unapplied").await;
    // 43: applied, but no mentions at all.
    org(&conn, 43, "Bietergemeinschaft Empty").await;
    for (notice, o, name) in [
        (900i64, 40i64, Some("Dobler GmbH")),
        (901, 40, Some("Dobler  GMBH")),           // same name, different spacing/case
        (902, 40, Some("Huber Bau AG")),
        (903, 40, Some("Bietergemeinschaft Dobler / Huber")), // the vehicle itself
        (904, 40, None),                            // no name: no evidence either way
        (905, 40, Some("   ")),                     // whitespace: likewise
        (906, 40, Some("!!!")),                     // normalizes to nothing
        (907, 41, Some("Bietergemeinschaft Sauber GmbH")),
        (908, 41, Some("BIETERGEMEINSCHAFT  SAUBER  GMBH")),
        (909, 42, Some("Somebody Else Entirely")),
    ] {
        mention(&conn, notice, o, name).await;
    }
    db.record_case_reviews(
        "biege",
        &[
            CaseReview {
                case_org_id: 40,
                verdict: "consortium-vehicle-wrong-identifier".into(),
                diagnosis: "d".into(),
                handling: "h".into(),
                rationale: "r".into(),
                confidence: "high".into(),
            },
            CaseReview {
                case_org_id: 41,
                verdict: "consortium-vehicle-wrong-identifier".into(),
                diagnosis: "d".into(),
                handling: "h".into(),
                rationale: "r".into(),
                confidence: "high".into(),
            },
            CaseReview {
                case_org_id: 42,
                verdict: "consortium-vehicle-wrong-identifier".into(),
                diagnosis: "d".into(),
                handling: "h".into(),
                rationale: "r".into(),
                confidence: "high".into(),
            },
            CaseReview {
                case_org_id: 43,
                verdict: "consortium-vehicle-wrong-identifier".into(),
                diagnosis: "d".into(),
                handling: "h".into(),
                rationale: "r".into(),
                confidence: "high".into(),
            },
        ],
        1,
    )
    .await
    .unwrap();
    // Stamp 40, 41 and 43 as APPLIED; 42 stays pending.
    for id in [40i64, 41, 43] {
        conn.execute(
            "UPDATE org_case_reviews SET applied_at = 5 WHERE case_org_id = ?",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
    }

    let never = || false;
    let r = db.fusion_candidates(norm, 120, &never).await.unwrap();
    assert_eq!(r.cases, 3, "only the APPLIED verdicts — 42 is pending");
    assert_eq!(r.with_mentions, 2, "43 has no mentions at all");
    assert_eq!(r.fused, 1, "only 40 holds mentions naming somebody else");
    assert_eq!(
        r.off_name_mentions, 3,
        "two Dobler spellings plus one Huber; the vehicle's own mention, the NULL, \
         the whitespace and the punctuation-only name are not evidence"
    );
    assert!(!r.truncated);
    assert_eq!(r.candidates.len(), 1);
    let c = &r.candidates[0];
    assert_eq!(
        (c.org, c.mentions, c.off_name),
        (40, 4, 3),
        "four JUDGEABLE mentions, three of them naming somebody else — the NULL, the \
         whitespace and the punctuation-only name are on neither side of the ratio"
    );
    assert_eq!(
        c.groups,
        vec![("Dobler GmbH".to_owned(), 2), ("Huber Bau AG".to_owned(), 1)],
        "the other names, biggest first — spacing and case do not split a group"
    );

    // The clean vehicle is silent: matching on the SHARED KEY, not the raw
    // string, is what keeps 41 out of the list.
    assert!(r.candidates.iter().all(|c| c.org != 41));

    // Read-only.
    let mut rows = conn.query("SELECT COUNT(*) FROM changes", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get_value(0).unwrap(), Value::Integer(0));

    // The cap clips the LIST but not the counts — a truncated list must never
    // read as the whole finding.
    let small = db.fusion_candidates(norm, 0, &never).await.unwrap();
    assert!(small.truncated);
    assert_eq!((small.fused, small.off_name_mentions), (1, 3), "counts are still whole");
    assert!(small.candidates.is_empty());

    // Cancel is honest.
    let always = || true;
    let stopped = db.fusion_candidates(norm, 120, &always).await.unwrap();
    assert!(stopped.stopped);
    assert_eq!((stopped.cases, stopped.fused), (0, 0));
}
