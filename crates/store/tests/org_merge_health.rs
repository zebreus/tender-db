//! Issue 300 Stage 0: the org-merge-health census — distinct normalized
//! mention names per identifier-bearing org. The walk must count through the
//! injected N2 normalizer (case/punctuation variants collapse), skip
//! identifier-less orgs entirely, skip empty names, and iterate correctly at
//! batch=1 (the watermark shape).

use store::turso::Value;

fn n2(name: &str) -> String {
    // The test's stand-in for ingest's match_norm (store cannot depend on
    // ingest): lowercase + non-alphanumeric→space + collapse, same contract.
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

async fn seed(path: &str) -> store::Db {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute("BEGIN", ()).await.unwrap();

    // 10 = identifier-bearing, three mention spellings of ONE name plus one
    // genuinely different name -> 2 distinct N2 keys.
    // 11 = identifier-bearing, one name + an empty-name mention -> 1.
    // 12 = NO identifier -> invisible to the census however many mentions.
    // 13 = identifier-bearing, zero mentions -> counted with 0.
    for (id, ident, name) in [
        (10, Some("49371185"), "Gymnázium"),
        (11, Some("DE123456789"), "Land BW"),
        (12, None, "Provisional Body"),
        (13, Some("180014045"), "CNFPT"),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, country, name, name_norm, provisional, identifier_kind, identifier, created_at)
             VALUES (?, 'CZ', ?, ?, 0, CASE WHEN ? IS NULL THEN NULL ELSE 'national' END, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
                match ident {
                    Some(s) => Value::Text(s.into()),
                    None => Value::Null,
                },
                match ident {
                    Some(s) => Value::Text(s.into()),
                    None => Value::Null,
                },
            ),
        )
        .await
        .unwrap();
    }
    for (nid, sid, org, name) in [
        (100, "S-1", 10, "Gymnázium Praha"),
        (101, "S-1", 10, "GYMNÁZIUM  PRAHA"),
        (102, "S-1", 10, "Gymnázium, Praha."),
        (103, "S-1", 10, "Střední škola"),
        (104, "S-1", 11, "Land BW"),
        (105, "S-1", 11, ""),
        (106, "S-1", 12, "Provisional Body"),
        (107, "S-2", 12, "Provisional Body II"),
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name)
             VALUES (?, ?, ?, ?)",
            (
                Value::Integer(nid),
                Value::Text(sid.into()),
                Value::Integer(org),
                Value::Text(name.into()),
            ),
        )
        .await
        .unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    db
}

#[tokio::test]
async fn the_census_counts_distinct_n2_names_for_identifier_bearing_orgs_only() {
    let db = seed("test-omh.db").await;

    // batch=1 forces the watermark to iterate one org at a time.
    let mut all: Vec<(i64, u64)> = Vec::new();
    let mut after = 0i64;
    let mut rounds = 0;
    loop {
        let (rows, next) = db.org_merge_health_batch(n2, 1, after).await.expect("batch");
        if rows.is_empty() {
            break;
        }
        assert_eq!(rows.len(), 1, "batch=1 yields one org per round");
        for r in rows {
            // The identity triple rides along for the gate census.
            assert!(!r.identifier.is_empty(), "walked orgs carry their identifier");
            if r.org_id == 10 {
                assert_eq!(r.country.as_deref(), Some("CZ"));
                assert_eq!(r.kind.as_deref(), Some("national"));
            }
            all.push((r.org_id, r.distinct_names));
        }
        assert!(next > after, "watermark must advance");
        after = next;
        rounds += 1;
        assert!(rounds < 10, "walk must terminate");
    }

    assert_eq!(
        all,
        vec![(10, 2), (11, 1), (13, 0)],
        "three spellings collapse to one key plus the distinct second name; \
         the empty-name mention is skipped; the identifier-less org 12 is \
         invisible; the mention-less org 13 reports zero"
    );

    let meta = db.org_health_meta(&[10, 13]).await.expect("meta");
    assert_eq!(meta.len(), 2);
    let cnfpt = meta.iter().find(|m| m.0 == 13).expect("org 13");
    assert_eq!(cnfpt.3.as_deref(), Some("180014045"));
    assert_eq!(cnfpt.4, "CNFPT");
}
