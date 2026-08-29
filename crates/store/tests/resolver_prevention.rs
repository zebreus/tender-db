//! Issue 300 Stage 2, the PREVENTION half: the mention resolver's canonical
//! pre-probe. An equivalent representation of a standing registration reuses
//! its Organization instead of minting a twin; consortium-named mentions and
//! poisoned (multi-owner) keys fall through to the pre-Stage-2 behavior.
//! The injected rules are test-local minis (the real crosswalk pins its own
//! behavior in `ingest::crosswalk`); this test pins the resolver machinery.

use store::{Identifier, Mention};

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    let digits: String = value.chars().filter(char::is_ascii_alphanumeric).collect();
    let body = if kind == "vat" { digits.get(2..)?.to_owned() } else { digits };
    match (country?, body.len()) {
        ("FI", 8) => Some(("FI:ytunnus", body, true)),
        ("FR", 9) => Some(("FR:siren", body, true)),
        ("FR", 14) => Some(("FR:siren", body[..9].to_owned(), true)),
        // The pad analog: 7-digit CZ keys E2 — never unifies at mint.
        ("CZ", 8) => Some(("CZ:ico", body, true)),
        ("CZ", 7) => Some(("CZ:ico", format!("0{body}"), false)),
        _ => None,
    }
}

fn consortium(name: &str) -> bool {
    name.to_lowercase().contains("groupement")
}

fn mention(notice: i64, section: &str, name: &str, country: &str, kind: &str, value: &str) -> Mention {
    Mention {
        notice_id: notice,
        section_id: section.into(),
        name: name.into(),
        country: Some(country.into()),
        raw_identifier: Some(value.into()),
        scheme: None,
        identifier: Some(Identifier {
            country: Some(country.into()),
            kind: kind.into(),
            value: value.into(),
        }),
        variants: Vec::new(),
    }
}

async fn count(db: &store::Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(store::turso::Value::Integer(n)) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

#[tokio::test]
async fn equivalent_representations_reuse_and_hazards_fall_through() {
    let path = "test-resolver-prevention.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    // Parents for the mention rows' FKs (notices + notice_sections), seeded
    // on a raw FK-off connection like the sibling tests.
    {
        let raw = store::turso::Builder::new_local(path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
        for n in 1..=11i64 {
            conn.execute(
                "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                      member_path, ingested_at, parse_state, projected)
                 VALUES (?, 'ted', ?, ?, 'eforms:test', 1, 'p', 0, 'parsed', 0)",
                (
                    store::turso::Value::Integer(n),
                    store::turso::Value::Text(format!("pub-{n}")),
                    store::turso::Value::Text(format!("h{n}")),
                ),
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
                 VALUES (?, 'S-1', 'Organization', NULL)",
                (store::turso::Value::Integer(n),),
            )
            .await
            .unwrap();
        }
    }

    let mut resolver = db.mention_resolver(Some(key), Some(consortium)).await.unwrap();
    let ids = db
        .resolve_mentions(
            &mut resolver,
            &[
                // The standing registration, then its equivalent VAT form.
                mention(1, "S-1", "Alpha Oy", "FI", "national", "01003158"),
                mention(2, "S-1", "Alpha", "FI", "vat", "FI01003158"),
                // A lead company by SIREN, then a groupement publishing the
                // lead's SIRET: the veto splits it and poisons the key…
                mention(3, "S-1", "Lead SA", "FR", "national", "111222333"),
                mention(4, "S-1", "groupement lead / other", "FR", "national", "11122233300012"),
                // …so a later establishment mention falls through and mints.
                mention(5, "S-1", "Lead Est", "FR", "national", "11122233300020"),
                // The pad analog: E2 keys never unify at mint.
                mention(6, "S-1", "Ministerstvo", "CZ", "national", "00006947"),
                mention(7, "S-1", "Ministerstvo pad", "CZ", "national", "0006947"),
            ],
            0,
        )
        .await
        .unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();

    assert_eq!(ids[0], ids[1], "the VAT form reuses the standing Y-tunnus org");
    assert_ne!(ids[2], ids[3], "the groupement never canon-binds to its lead");
    assert_ne!(ids[3], ids[4], "…and the poisoned key stops binding anyone");
    assert_ne!(ids[2], ids[4], "the establishment falls through to a fresh mint");
    assert_ne!(ids[5], ids[6], "an E2 pad key never unifies at mint");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 6);

    // The PRELOAD half: a fresh resolver over the standing table must rebuild
    // the same maps — the unique FI key binds, the FR key (three standing
    // owners) is poisoned on preload.
    let mut resolver = db.mention_resolver(Some(key), Some(consortium)).await.unwrap();
    let ids2 = db
        .resolve_mentions(
            &mut resolver,
            &[
                mention(8, "S-1", "Alpha again", "FI", "vat", "FI01003158"),
                mention(9, "S-1", "Lead Est 2", "FR", "national", "11122233300038"),
            ],
            0,
        )
        .await
        .unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_eq!(ids2[0], ids[0], "preload rebinds the equivalent form across runs");
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM organizations").await,
        7,
        "the poisoned FR family minted fresh; nothing else did"
    );

    // Prevention disabled (None): the same equivalent form mints a twin —
    // the pre-Stage-2 behavior, byte-identical.
    let mut resolver = db.mention_resolver(None, None).await.unwrap();
    let ids3 = db
        .resolve_mentions(
            &mut resolver,
            &[mention(10, "S-1", "Beta Oy", "FI", "national", "20445111"),
              mention(11, "S-1", "Beta", "FI", "vat", "FI20445111")],
            0,
        )
        .await
        .unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_ne!(ids3[0], ids3[1], "without the injected crosswalk nothing unifies");
}
