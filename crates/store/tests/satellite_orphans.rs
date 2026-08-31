//! Issue 321: the measurement that decides whether the leftover name variants
//! need machinery or one line in the review schema.
//!
//! `apply_rehoming` corrects the mentions and lets the fold re-derive what
//! follows. `organization_names` is not part of that derivation, so the
//! consortium vehicle keeps its member's name — and satellites feed the
//! Stage-4 key build, so the vehicle keeps a key matching the member and the
//! pair goes on generating the E3 edges the verdict just resolved.

use store::turso::Value;

fn norm(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.extend(c.to_lowercase());
        } else {
            gap = true;
        }
    }
    out
}

async fn org(conn: &store::turso::Connection, id: i64, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, 'DE', NULL, NULL, ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
}

async fn variant(conn: &store::turso::Connection, id: i64, lang: &str, name: &str) {
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (?, ?, ?, ?)",
        (
            Value::Integer(id),
            Value::Text(lang.into()),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
}

async fn mention(conn: &store::turso::Connection, notice: i64, on: i64, name: &str) {
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (?, 'ORG-1', ?, ?, 'DE', NULL)",
        (Value::Integer(notice), Value::Integer(on), Value::Text(name.into())),
    )
    .await
    .unwrap();
}

async fn applied(conn: &store::turso::Connection, org_id: i64, notice: i64, target: i64, from: i64) {
    conn.execute(
        "INSERT INTO org_mention_rehoming
           (case_org_id, notice_id, section_id, cohort, action, target_org_id, target_name,
            rationale, confidence, reviewed_at, applied_at, applied_action, job_id)
         VALUES (?, ?, 'ORG-1', 'c', 'rehome', ?, 'n', 'r', 'high', 1, 2, 'rehomed from ' || ?, 1)",
        (
            Value::Integer(org_id),
            Value::Integer(notice),
            Value::Integer(target),
            Value::Integer(from),
        ),
    )
    .await
    .unwrap();
}

/// The whole shape in one fixture: an orphan that stands on the destination
/// (the case issue 321 is about), an orphan that does not, a variant a
/// remaining mention still supports, and a variant that is just the origin's
/// own head name in another language.
#[tokio::test]
async fn a_variant_no_remaining_mention_supports_is_reported_with_its_destination() {
    let path = "test-satellite-orphans.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    org(&conn, 1, "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "Dobler GmbH").await;
    // DE: the member's name, left behind by the move — and org 2 IS Dobler.
    variant(&conn, 1, "DE", "Dobler GmbH").await;
    // FR: a spelling nothing on this row publishes any more, and no
    // destination carries either.
    variant(&conn, 1, "FR", "Groupement Dobler et Oberall SARL").await;
    // EN: still published by a mention that stayed.
    variant(&conn, 1, "EN", "Consortium Dobler / Oberall").await;
    // NL: the origin's own head name, so never an orphan.
    variant(&conn, 1, "NL", "Bietergemeinschaft Dobler / Oberall").await;
    mention(&conn, 900, 1, "Consortium Dobler / Oberall").await;
    // The mention that moved now sits on org 2.
    mention(&conn, 901, 2, "Dobler GmbH").await;
    applied(&conn, 1, 901, 2, 1).await;

    let never = || false;
    let r = db.satellite_orphans(norm, 200, &never).await.unwrap();
    assert_eq!(r.origins, 1);
    assert_eq!(r.standing, 1);
    assert_eq!(r.variants, 4);
    assert_eq!(r.orphans, 2, "the DE and FR variants; EN and NL are supported");
    assert_eq!(r.orphans_at_target, 1, "only DE stands on the row the mention moved to");
    assert_eq!(r.origins_with_orphans, 1);
    assert!(!r.truncated);

    let de = r.rows.iter().find(|o| o.lang == "DE").expect("DE is listed");
    assert_eq!(de.org, 1);
    assert_eq!(de.key, "dobler gmbh");
    assert_eq!(de.target, Some(2), "and it names the destination that already has it");
    assert_eq!(de.target_name.as_deref(), Some("Dobler GmbH"));

    let fr = r.rows.iter().find(|o| o.lang == "FR").expect("FR is listed");
    assert_eq!(fr.target, None, "unsupported here, and nowhere else either");
    assert!(r.rows.iter().all(|o| o.lang != "EN" && o.lang != "NL"));
}

/// A destination that carries the key on a SATELLITE rather than its head is
/// still the destination: the Stage-4 build reads head plus satellites, so
/// that is the same duplicate key by the same path.
#[tokio::test]
async fn a_destination_carrying_the_key_as_its_own_variant_counts() {
    let path = "test-satellite-orphans-alias.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    org(&conn, 1, "Bietergemeinschaft Rijk / Nord").await;
    org(&conn, 2, "Rijkswaterstaat").await;
    variant(&conn, 1, "DE", "Reichswasserstaat").await;
    variant(&conn, 2, "DE", "Reichswasserstaat").await;
    mention(&conn, 901, 2, "Reichswasserstaat").await;
    applied(&conn, 1, 901, 2, 1).await;

    let never = || false;
    let r = db.satellite_orphans(norm, 200, &never).await.unwrap();
    assert_eq!(r.orphans, 1);
    assert_eq!(r.orphans_at_target, 1, "the head does not match, but the satellite does");
    assert_eq!(r.rows[0].target, Some(2));
}

/// A cancel stores nothing: an undercount of leftover keys would read as a
/// clean campaign, which is the one wrong answer this measurement can give.
#[tokio::test]
async fn a_cancelled_measurement_reports_nothing_rather_than_less() {
    let path = "test-satellite-orphans-stop.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    org(&conn, 1, "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "Dobler GmbH").await;
    variant(&conn, 1, "DE", "Dobler GmbH").await;
    applied(&conn, 1, 901, 2, 1).await;

    let always = || true;
    let r = db.satellite_orphans(norm, 200, &always).await.unwrap();
    assert!(r.stopped);
    assert_eq!(r.orphans, 0);
    assert_eq!(r.origins, 0, "even the totals are dropped");
    assert!(r.rows.is_empty());
}
