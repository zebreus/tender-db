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

    let mut resolver = db.mention_resolver(Some(key), Some(consortium), None, None, None, None, 0).await.unwrap();
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
    let mut resolver = db.mention_resolver(Some(key), Some(consortium), None, None, None, None, 0).await.unwrap();
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
    let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
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

// ------------- issue 310: the Stage-3 (country-less anchor) prevention -------------

fn anchors(v: &str) -> Vec<(&'static str, String)> {
    // A miniature checksum probe: 8-digit values starting with 0 anchor
    // uniquely FI; other 8-digit values pass TWO schemes (ambiguous by
    // construction — the SI/CZ co-anchor shape).
    let digits: String = v.chars().filter(char::is_ascii_digit).collect();
    if v.bytes().any(|b| b.is_ascii_alphabetic()) || digits.len() != 8 {
        return Vec::new();
    }
    if digits.starts_with('0') {
        vec![("FI:ytunnus", digits)]
    } else {
        vec![("FI:ytunnus", digits.clone()), ("CZ:ico", digits)]
    }
}

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect()
}

fn legal_form(name: &str) -> Option<&'static str> {
    let l = name.to_lowercase();
    if l.ends_with(" oy") {
        Some("oy")
    } else if l.ends_with(" ab") {
        Some("ab")
    } else {
        None
    }
}

fn mention_nc(notice: i64, name: &str, value: &str) -> Mention {
    Mention {
        notice_id: notice,
        section_id: "S-1".into(),
        name: name.into(),
        country: None,
        raw_identifier: Some(value.into()),
        scheme: None,
        identifier: Some(Identifier { country: None, kind: "national".into(), value: value.into() }),
        variants: Vec::new(),
    }
}

fn with_variants(mut m: Mention, variants: &[(&str, &str)]) -> Mention {
    m.variants = variants.iter().map(|(l, n)| ((*l).into(), (*n).into())).collect();
    m
}

#[tokio::test]
async fn country_less_anchored_corroborated_mentions_bind_and_everything_else_mints() {
    let path = "test-r3-prevention.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    {
        let raw = store::turso::Builder::new_local(path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
        for n in 1..=12i64 {
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

    let mut resolver = db
        .mention_resolver(Some(key), Some(consortium), Some(anchors), Some(norm), Some(legal_form), None, 0)
        .await
        .unwrap();
    let ids = db
        .resolve_mentions(
            &mut resolver,
            &[
                // 0. The standing FI registration.
                mention(1, "S-1", "Gamma Oy", "FI", "national", "01003158"),
                // 1. A COUNTRY-LESS formatted form of the same number with the
                //    same name: unique anchor + corroboration ⇒ BINDS.
                mention_nc(2, "Gamma Oy", "0100315-8"),
                // 2. Same digits, a different name: uncorroborated ⇒ mints the
                //    NULL twin (the R3 merge job's material, not prevention's).
                mention_nc(3, "Different Name", "01003158"),
                // 3. A multi-anchor value with a matching name: ambiguous ⇒
                //    mints (the checksum cannot say which register).
                mention_nc(4, "Gamma Oy", "20445111"),
                // 4. BYTE-IDENTICAL to the bound form but a different name:
                //    anchor binds are never cached (panel catch — a cached
                //    triple would let this ride E0 past corroboration).
                mention_nc(5, "Another Name", "0100315-8"),
                // 5. The mention's own VARIANT is groupement-labelled while
                //    its head name is clean: the veto covers variants too.
                with_variants(
                    mention_nc(6, "Gamma Oy", "010.03158"),
                    &[("fr", "groupement Gamma / X")],
                ),
                // 6. A standing org whose SATELLITE carries the other legal
                //    family (head "Delta AB", fi satellite "Delta Oy")…
                with_variants(
                    mention(7, "S-1", "Delta AB", "FI", "national", "04444444"),
                    &[("fi", "Delta Oy")],
                ),
                // 7. …a country-less "Delta Oy" corroborates via that
                //    satellite but the HEAD families conflict ⇒ mints (the
                //    cross-country twin shape, the merge arm's rule).
                mention_nc(8, "Delta Oy", "0444-4444"),
                // 8. A consortium-named OWNER with a clean satellite…
                with_variants(
                    mention(9, "S-1", "groupement Epsilon", "FI", "national", "05555555"),
                    &[("fi", "Epsilon Oy")],
                ),
                // 9. …never captures country-less mentions, even via the
                //    clean satellite: the owner's names are evidence too.
                mention_nc(10, "Epsilon Oy", "0555-5555"),
                // 10. A groupement publishing the standing number WITH
                //     country: the Stage-2 veto mints beside it and POISONS…
                mention(11, "S-1", "groupement Gamma / X", "FI", "national", "0100-3158"),
                // 11. …so a later anchored corroborated form no longer binds
                //     either: poisoned keys are the merge job's.
                mention_nc(12, "Gamma Oy", "01.00.31.58"),
            ],
            0,
        )
        .await
        .unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();

    assert_eq!(ids[1], ids[0], "anchored + corroborated country-less form binds the standing org");
    assert_ne!(ids[2], ids[0], "uncorroborated mints");
    assert_ne!(ids[3], ids[0], "multi-anchor mints");
    assert_ne!(ids[4], ids[0], "a byte-identical repeat with a different name re-earns the bar");
    assert_ne!(ids[5], ids[0], "a groupement-labelled VARIANT vetoes the anchor bind");
    assert_ne!(ids[7], ids[6], "satellite corroboration across a family conflict mints");
    assert_ne!(ids[9], ids[8], "a consortium-named owner never captures via its clean satellite");
    assert_ne!(ids[11], ids[0], "a poisoned key stops anchor-binding too");
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM organizations").await,
        11,
        "three standing orgs + eight deliberate mints; the one bind minted nothing"
    );
}

/// Issue 318: the resolver's anchor bind applied the R3 bar WITHOUT the
/// genericness wall the batch merge arm enforces, so the same evidence the
/// batch arm denies still bound at ingest — on every notice, without an audit
/// row.
///
/// The wall is injected, like every other piece of linguistics here, and the
/// three tests below pin the three answers it can give: refuse a soft-scheme
/// anchor on a generic name, exempt a hard one, and stay LENIENT when the key
/// store says nothing.
/// Issue 318: the resolver's anchor bind applied the R3 bar WITHOUT the
/// genericness wall the batch merge arm enforces, so the same evidence the
/// batch arm denies still bound at ingest — on every notice, and without the
/// audit row the batch arm leaves.
///
/// `indexed` controls whether `org_match_keys_kk` exists when the resolver is
/// opened. That is not a detail: the index is created at the END of a key build
/// and dropped at its start, so its absence is a designed, durable state, and
/// the wall must disable itself there rather than full-scan the satellite once
/// per bind.
async fn bind_with_wall_indexed(
    tag: &str,
    carriers: usize,
    value: &str,
    hard_scheme: Option<fn(&str) -> bool>,
    cap: usize,
    indexed: bool,
) -> (i64, u64) {
    let path = format!("test-318-wall-{tag}.db");
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, parse_state, projected)
         VALUES (900, 'ted', 'pub-900', 'h900', 'eforms:test', 1, 'p', 0, 'parsed', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
         VALUES (900, 'S-1', 'Organization', NULL)",
        (),
    )
    .await
    .unwrap();
    // The standing owner: same name the mention will carry, so corroboration
    // succeeds and the WALL is the only thing left that can refuse.
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (1, 'FI', 'national', ?, 'Yhteinen Nimi', 'yhteinen nimi', 0, 0)",
        (store::turso::Value::Text(value.into()),),
    )
    .await
    .unwrap();
    // `carriers` orgs share the N2 key, which is what makes it generic.
    for o in 1..=carriers as i64 {
        // Issue 354: the wall counts LIVE carriers — a standing row behind every key row.
        conn.execute(
            "INSERT OR IGNORE INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, NULL, NULL, NULL, 'yhteinennimi', 'yhteinennimi', 0, 0)",
            (store::turso::Value::Integer(o),),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'yhteinennimi')",
            (store::turso::Value::Integer(o),),
        )
        .await
        .unwrap();
    }

    if indexed {
        conn.execute(
            "CREATE INDEX IF NOT EXISTS org_match_keys_kk \
                 ON org_match_keys(key_kind, key, org_id)",
            (),
        )
        .await
        .unwrap();
    }
    let mut resolver = db
        .mention_resolver(
            Some(key),
            Some(consortium),
            Some(anchors),
            Some(norm),
            Some(legal_form),
            hard_scheme,
            cap,
        )
        .await
        .unwrap();
    db.resolve_mentions(&mut resolver, &[mention_nc(900, "Yhteinen Nimi", value)], 1)
        .await
        .unwrap();
    let w = store::Db::wall_counts(&resolver);
    assert_eq!(w.errored, 0, "no probe should error in a fixture");
    // `enabled` is what makes a zero legible: a disabled wall and an unfired
    // anchor path both report `asked 0`, and only this tells them apart.
    assert_eq!(w.enabled, hard_scheme.is_some() && indexed);
    let denied = w.denied;
    db.finish_mention_resolver(resolver).await.unwrap();
    let bound = count(&db, "SELECT organization_id FROM organization_mentions WHERE notice_id = 900").await;
    (bound, denied)
}

/// A SOFT-scheme anchor corroborating on a name 25 orgs share is exactly what
/// the batch arm refuses. Now the resolver refuses it too, and mints instead.
#[tokio::test]
async fn a_generic_name_under_a_soft_scheme_no_longer_binds() {
    let (bound, denied) = bind_with_wall_indexed("soft", 25, "01234567", Some(|s| s == "CZ:ico"), 20, true).await;
    assert_ne!(bound, 1, "the mention minted its own row rather than binding the standing owner");
    assert_eq!(denied, 1, "and the wall says so, rather than refusing silently");
}

/// A HARD-checksummed anchor is the design's single exemption: it stands on
/// its own arithmetic, not on the name, so a generic name does not sink it.
#[tokio::test]
async fn a_generic_name_under_a_hard_scheme_still_binds() {
    let (bound, denied) =
        bind_with_wall_indexed("hard", 25, "01234567", Some(|s| s == "FI:ytunnus"), 20, true).await;
    assert_eq!(bound, 1, "the exemption holds — this is design §4.1, not an oversight");
    assert_eq!(denied, 0);
}

/// LENIENT on unknown, and this is the load-bearing one. `org_match_keys` is
/// built by a separate batch job and may be EMPTY or mid-build while the fold
/// runs; this path has no "refuse" to return. An unseen key counts 0 carriers,
/// which is under any cap, so it is not generic and today's behaviour stands.
/// A wall that failed closed here would silently stop preventing the twins
/// Stage 3 exists to prevent, every time the key store was rebuilt.
#[tokio::test]
async fn an_empty_key_store_binds_exactly_as_before() {
    let (bound, denied) = bind_with_wall_indexed("lenient", 0, "01234567", Some(|s| s == "CZ:ico"), 20, true).await;
    assert_eq!(bound, 1, "no keys known ⇒ nothing is generic ⇒ pre-318 behaviour");
    assert_eq!(denied, 0);
}

/// And with no `hard_scheme` injected at all the wall does not exist — the
/// byte-identical pre-318 path every store-level caller keeps.
#[tokio::test]
async fn no_injected_wall_is_the_pre_318_path() {
    let (bound, denied) = bind_with_wall_indexed("off", 25, "01234567", None, 0, true).await;
    assert_eq!(bound, 1);
    assert_eq!(denied, 0);
}

/// The index is created at the END of a key build and dropped at its start, so
/// through every window of a ~2-hour build — and DURABLY after an interrupted
/// one, since nothing re-enqueues a cancelled build — `org_match_keys` has no
/// index at all. The probe would then full-scan a ~21M-row satellite once per
/// bind, on the ingest hot path, and the LENIENT verdict is the expensive case:
/// concluding "0 carriers" means reading everything.
///
/// Every other consumer of this table refuses in that state. This path has no
/// refuse, so it disables the wall for the run — which is the same lenient
/// answer, at no cost. Without this the first version's "one indexed seek"
/// cost claim was false in exactly the state the design calls load-bearing.
#[tokio::test]
async fn an_unindexed_key_store_disables_the_wall_for_the_run() {
    let (bound, denied) =
        bind_with_wall_indexed("noindex", 25, "01234567", Some(|s| s == "CZ:ico"), 20, false).await;
    assert_eq!(bound, 1, "the wall is off, so the bind proceeds at the pre-318 bar");
    assert_eq!(denied, 0, "and nothing is reported as refused, because nothing was asked");
}

/// A wall denial must not leave a CACHE entry behind (panel round 1,
/// reproduced). The mint that follows a denial claims the raw
/// `(country, kind, value)` triple, so the NEXT mention carrying that
/// identifier rides E0 onto the freshly minted provisional — even when its own
/// name is perfectly specific and would have anchored to the standing owner.
///
/// The captured mention pays for a name it does not have, and which row it
/// lands on depends on the ORDER the batch happens to see the two mentions in.
/// That order-dependence is what makes it a defect rather than a policy.
#[tokio::test]
async fn a_wall_denial_does_not_capture_the_next_clean_mention() {
    let path = "test-318-nocache.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for n in [900i64, 901] {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', 'pub-' || ?, 'h' || ?, 'eforms:test', 1, 'p', 0, 'parsed', 0)",
            (
                store::turso::Value::Integer(n),
                store::turso::Value::Integer(n),
                store::turso::Value::Integer(n),
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
    // The standing owner carries BOTH names: a generic head and a specific
    // satellite. So the clean mention corroborates on the satellite and would
    // anchor-bind; the generic one is refused by the wall.
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (1, 'FI', 'national', '01234567', 'Yhteinen Nimi', 'yhteinen nimi', 0, 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm)
         VALUES (1, 'FIN', 'Uniikki Nimi', 'uniikki nimi')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS org_match_keys_kk ON org_match_keys(key_kind, key, org_id)",
        (),
    )
    .await
    .unwrap();
    // 25 carriers of the GENERIC key; the specific one has none.
    for o in 1..=25i64 {
        // Issue 354: the wall counts LIVE carriers — a standing row behind every key row.
        conn.execute(
            "INSERT OR IGNORE INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, NULL, NULL, NULL, 'yhteinennimi', 'yhteinennimi', 0, 0)",
            (store::turso::Value::Integer(o),),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'yhteinennimi')",
            (store::turso::Value::Integer(o),),
        )
        .await
        .unwrap();
    }

    let mut resolver = db
        .mention_resolver(
            Some(key),
            Some(consortium),
            Some(anchors),
            Some(norm),
            Some(legal_form),
            Some(|s| s == "CZ:ico"), // FI:ytunnus is SOFT here
            20,
        )
        .await
        .unwrap();
    let ids = db
        .resolve_mentions(
            &mut resolver,
            &[
                // Denied by the wall — mints, and must NOT cache.
                mention_nc(900, "Yhteinen Nimi", "01234567"),
                // Its own name is specific: this one has earned the anchor
                // bind to the standing owner and must get it.
                mention_nc(901, "Uniikki Nimi", "01234567"),
            ],
            1,
        )
        .await
        .unwrap();
    let w = store::Db::wall_counts(&resolver);
    let (denied, errored) = (w.denied, w.errored);
    assert!(w.enabled, "the wall must be live for this test to mean anything");
    assert_eq!(w.anchor_reached, 2, "both mentions reached the anchor gate");
    db.finish_mention_resolver(resolver).await.unwrap();

    assert_eq!(denied, 1, "the generic-named mention is refused");
    assert_eq!(errored, 0);
    assert_ne!(ids[0], 1, "…and mints rather than binding the standing owner");
    assert_eq!(
        ids[1], 1,
        "but the CLEAN mention still reaches the standing owner — it was not \
         captured by the denied mention's fresh row"
    );
}

fn mention_name_only(notice: i64, name: &str) -> Mention {
    Mention {
        notice_id: notice,
        section_id: "S-1".into(),
        name: name.into(),
        country: None,
        raw_identifier: None,
        scheme: None,
        identifier: None,
        variants: Vec::new(),
    }
}

/// Issue 351: a country-less, identifier-less mention reuses the standing
/// `(name, NULL)` provisional row when its N2 key is under the wall, and
/// mints a fresh row — the 234 behaviour — when the key is over it. Without a
/// normaliser the resolver cannot ask the wall, so it mints as before.
#[tokio::test]
async fn country_less_mentions_reuse_under_the_wall_and_mint_over_it() {
    let path = "test-351-prevention.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    {
        let raw = store::turso::Builder::new_local(path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
        conn.execute(
            "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
             VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
            (),
        )
        .await
        .unwrap();
        for n in 1..=10i64 {
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
        // "gemeindetaufkirchen" is carried by three org rows: over a cap of 2,
        // and none of them identified — over the wall with no other tier.
        for id in [9001, 9002, 9003] {
            // Issue 354: the wall counts LIVE carriers — a standing row behind every key row.
            conn.execute(
                "INSERT OR IGNORE INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, NULL, NULL, NULL, 'gemeindetaufkirchen', 'gemeindetaufkirchen', 0, 0)",
                (store::turso::Value::Integer(id),),
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'gemeindetaufkirchen')",
                (store::turso::Value::Integer(id),),
            )
            .await
            .unwrap();
        }
        // "stadtbig": over the wall by five carriers, two of them identified
        // DE rows — an echo of one entity (unit 4's tier 2).
        for id in [9101, 9102, 9103] {
            // Issue 354: the wall counts LIVE carriers — a standing row behind every key row.
            conn.execute(
                "INSERT OR IGNORE INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, NULL, NULL, NULL, 'stadtbig', 'stadtbig', 0, 0)",
                (store::turso::Value::Integer(id),),
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'stadtbig')",
                (store::turso::Value::Integer(id),),
            )
            .await
            .unwrap();
        }
        for (id, vat) in [(9104, "DE9104"), (9105, "DE9105")] {
            conn.execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, 'DE', 'vat', ?, 'Stadt Big', 'stadt big', 0, 0)",
                (store::turso::Value::Integer(id), store::turso::Value::Text(vat.into())),
            )
            .await
            .unwrap();
            // Issue 354: the wall counts LIVE carriers — a standing row behind every key row.
            conn.execute(
                "INSERT OR IGNORE INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, NULL, NULL, NULL, 'stadtbig', 'stadtbig', 0, 0)",
                (store::turso::Value::Integer(id),),
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'stadtbig')",
                (store::turso::Value::Integer(id),),
            )
            .await
            .unwrap();
        }
    }
    // A recorded `single` verdict admits a name the wall would refuse (tier 1).
    for id in [9201, 9202, 9203] {
        let raw = store::turso::Builder::new_local(path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        // Issue 354: the wall counts LIVE carriers — a standing row behind every key row.
        conn.execute(
            "INSERT OR IGNORE INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, NULL, NULL, NULL, 'tribunalverdict', 'tribunalverdict', 0, 0)",
            (store::turso::Value::Integer(id),),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'tribunalverdict')",
            (store::turso::Value::Integer(id),),
        )
        .await
        .unwrap();
    }
    db.record_name_verdicts(
        "t",
        &[store::NameVerdict { name_norm: "tribunal verdict".into(), verdict: "single".into(), rationale: "one court".into() }],
        0,
    )
    .await
    .unwrap();
    let mut resolver = db.mention_resolver(None, None, None, Some(norm), None, None, 2).await.unwrap();
    let ids = db
        .resolve_mentions(
            &mut resolver,
            &[
                mention_name_only(1, "Stadt Burghausen"),
                mention_name_only(2, "Stadt Burghausen"),
                mention_name_only(3, "Gemeinde Taufkirchen"),
                mention_name_only(4, "Gemeinde Taufkirchen"),
                mention_name_only(5, "Stadt Big"),
                mention_name_only(6, "Stadt Big"),
                mention_name_only(7, "Tribunal Verdict"),
                mention_name_only(8, "Tribunal Verdict"),
            ],
            0,
        )
        .await
        .unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_eq!(ids[0], ids[1], "under the wall: one standing (name, NULL) row for both");
    assert_ne!(ids[2], ids[3], "over the wall, nothing identified: the 234 behaviour, one row per mention");
    assert_eq!(ids[4], ids[5], "echo of one identified entity: reused");
    assert_eq!(ids[6], ids[7], "a recorded single verdict: reused");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt burghausen'").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'gemeinde taufkirchen'").await, 2);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt burghausen' AND country IS NULL AND provisional = 1").await,
        1,
        "the reused row stays a NULL-country provisional"
    );
    // A later batch, a fresh resolver: the standing row is found in the table,
    // not only in the batch cache.
    let mut resolver = db.mention_resolver(None, None, None, Some(norm), None, None, 2).await.unwrap();
    let later = db.resolve_mentions(&mut resolver, &[mention_name_only(9, "Stadt Burghausen")], 0).await.unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_eq!(later[0], ids[0]);
    // Without a normaliser the wall cannot be asked: mint, as before 351.
    let mut resolver = db.mention_resolver(None, None, None, None, None, None, 2).await.unwrap();
    let blind = db.resolve_mentions(&mut resolver, &[mention_name_only(10, "Stadt Burghausen")], 0).await.unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_ne!(blind[0], ids[0]);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt burghausen'").await, 2);
}
