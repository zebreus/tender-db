//! Issue 432: the provisional reuse key. `organizations.name_norm` was bare
//! lower-casing, so a publisher's trailing space or ` .` minted a second
//! identifier-less row for one buyer (FTS: `'procurement for housing '` beside
//! `'procurement for housing'`). What is pinned: the key's exact rules and its
//! limits (spelling noise only — never a legal form), the resolver binding
//! the FTS spellings to ONE row, and the repair — re-keying the stock,
//! folding the same-country twins through the echo fold's loop under ledger
//! rule `p1`, re-keying the verdict table, parity, and a second run finding
//! nothing.

use store::turso::{self, Value};
use store::{Mention, org_name_norm};

#[test]
fn the_key_folds_whitespace_and_trailing_punctuation_and_nothing_else() {
    // The FTS twins behind issue 386's three welds.
    assert_eq!(org_name_norm("Procurement for Housing "), "procurement for housing");
    assert_eq!(org_name_norm("Procurement for Housing"), "procurement for housing");
    assert_eq!(org_name_norm("ScotRail Trains Limited ."), "scotrail trains limited");
    assert_eq!(org_name_norm("ScotRail Trains Limited"), "scotrail trains limited");
    // Decided on the issue: a trailing period is spelling.
    assert_eq!(org_name_norm("ACME Ltd."), org_name_norm("ACME Ltd"));
    // …and the legal form itself is untouched: that is N2's job, behind the wall.
    assert_ne!(org_name_norm("ACME Ltd"), org_name_norm("ACME"));
    // Every trailing `.`/`,`/`;`, with the whitespace before each, repeatedly.
    for noisy in ["x .", "x.", "x ,", "x;", "x..", "x .,; .", " x\t. "] {
        assert_eq!(org_name_norm(noisy), "x", "{noisy:?}");
    }
    // Whitespace: trimmed, and every internal run — tab, newline, NBSP — is one space.
    assert_eq!(org_name_norm("  Stadt   Muster\tstadt\n"), "stadt muster stadt");
    assert_eq!(org_name_norm("Stadt\u{a0}Musterstadt\u{a0}"), "stadt musterstadt");
    // Punctuation that is not trailing is part of the name.
    assert_eq!(org_name_norm("S.A. Foo"), "s.a. foo");
    assert_eq!(org_name_norm("Foo, Bar & Co. KG"), "foo, bar & co. kg");
    assert_eq!(org_name_norm(".Foo"), ".foo");
    assert_eq!(org_name_norm("Foo!"), "foo!", "only `.`, `,` and `;` strip");
    // An all-punctuation name keeps its own key rather than becoming another
    // one — and never the empty key, which would move it into the nameless class.
    assert_eq!(org_name_norm("."), ".");
    assert_eq!(org_name_norm(" . "), ".");
    assert_eq!(org_name_norm("..."), "...");
    assert_eq!(org_name_norm(". ,"), ". ,");
    // Blank stays empty.
    assert_eq!(org_name_norm(""), "");
    assert_eq!(org_name_norm(" \t "), "");
    // Unicode lower-casing is exactly `to_lowercase` — no accent folding, no
    // transliteration, ß stays ß, Greek keeps its tonos and final sigma.
    for clean in ["Straßenbauamt München", "ΔΉΜΟΣ ΑΘΗΝΑΊΩΝ", "Δήμος Αβδήρων", "Łódź Urząd Miasta", "İzmir"] {
        assert_eq!(org_name_norm(clean), clean.to_lowercase(), "{clean:?}");
    }
    assert_eq!(org_name_norm("ΔΉΜΟΣ ΑΘΗΝΑΊΩΝ."), "δήμος αθηναίων");
    assert_ne!(org_name_norm("Gymnázium"), org_name_norm("Gymnazium"), "accents are identity here");
    // Idempotent, so a stored key re-derives to itself.
    for name in ["ScotRail Trains Limited .", " . ", "ΔΉΜΟΣ ΑΘΗΝΑΊΩΝ.", "İzmir ;", "x .,; ."] {
        let once = org_name_norm(name);
        assert_eq!(org_name_norm(&once), once, "{name:?}");
    }
}

fn mention(notice: i64, name: &str, country: &str) -> Mention {
    Mention {
        notice_id: notice,
        section_id: "ORG-1".into(),
        name: name.into(),
        country: Some(country.into()),
        raw_identifier: None,
        scheme: None,
        identifier: None,
        variants: Vec::new(),
    }
}

async fn fresh(name: &str) -> (store::Db, String) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    (store::Db::open(&path).await.unwrap(), path)
}

async fn count(db: &store::Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(Value::Integer(n)) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

async fn norm_of(db: &store::Db, id: i64) -> String {
    match db.scalar(&format!("SELECT name_norm FROM organizations WHERE id = {id}")).await.unwrap() {
        Some(Value::Text(s)) => s,
        other => panic!("org {id}: {other:?}"),
    }
}

async fn resolve(db: &store::Db, mentions: &[Mention]) -> Vec<i64> {
    let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
    let ids = db.resolve_mentions(&mut resolver, mentions, 0).await.unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    ids
}

#[tokio::test]
async fn the_fts_spellings_of_one_identifierless_buyer_bind_one_row() {
    let (db, _path) = fresh("name-norm-mint").await;
    db.set_foreign_keys(false).await.unwrap();
    let ids = resolve(
        &db,
        &[
            mention(1, "Procurement for Housing", "GB"),
            mention(2, "Procurement for Housing ", "GB"),
            mention(3, "ScotRail Trains Limited", "GB"),
            mention(4, "ScotRail Trains Limited .", "GB"),
            mention(5, "ACME Ltd", "GB"),
            mention(6, "ACME Ltd.", "GB"),
            mention(7, "ACME", "GB"),
        ],
    )
    .await;
    assert_eq!(ids[0], ids[1], "a trailing space is one buyer (tender 8579928)");
    assert_eq!(ids[2], ids[3], "so is a trailing ` .` (tender 7957971)");
    assert_eq!(ids[4], ids[5], "and a trailing period (decided on the issue)");
    assert_ne!(ids[4], ids[6], "but `ACME Ltd` and `ACME` stay two: legal forms are N2's, behind the wall");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 4);
    // The row and the key agree: the stored column is the key, not the name.
    assert_eq!(norm_of(&db, ids[0]).await, "procurement for housing");
    assert_eq!(norm_of(&db, ids[2]).await, "scotrail trains limited");

    // A fresh resolver (empty caches) reaches the same rows through the
    // `(name_norm, country)` probe, in either direction of the noise.
    let again = resolve(
        &db,
        &[mention(8, "PROCUREMENT  FOR HOUSING ;", "GB"), mention(9, "ScotRail Trains Limited", "GB")],
    )
    .await;
    assert_eq!(again, vec![ids[0], ids[2]]);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 4);
}

async fn raw(path: &str) -> turso::Connection {
    let raw = turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn
}

/// An org row keyed the way the resolver keyed it BEFORE this issue:
/// `name.to_lowercase()`, nothing more.
async fn legacy_org(conn: &turso::Connection, id: i64, cc: Option<&str>, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, NULL, NULL, ?, ?, 1, 0)",
        (
            Value::Integer(id),
            cc.map(|c| Value::Text(c.into())).unwrap_or(Value::Null),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
}

async fn mention_row(conn: &turso::Connection, notice: i64, org: i64, name: &str) {
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (?, 'ORG-1', ?, ?, 'GB', NULL)",
        (Value::Integer(notice), Value::Integer(org), Value::Text(name.into())),
    )
    .await
    .unwrap();
}

fn args<'a>(
    dry_run: bool,
    expect: Option<(u64, u64)>,
    stop: &'a (dyn Fn() -> bool + Sync),
    progress: &'a (dyn Fn(u64, &str) + Sync),
) -> store::ProvisionalNameNormArgs<'a> {
    store::ProvisionalNameNormArgs {
        dry_run,
        max_groups: None,
        expect_groups: expect.map(|e| e.0),
        expect_rows: expect.map(|e| e.1),
        job_id: Some(432),
        stop,
        progress,
    }
}

#[tokio::test]
async fn the_repair_rekeys_the_stock_and_folds_the_same_country_twins() {
    let (db, path) = fresh("name-norm-repair").await;
    let conn = raw(&path).await;
    // The two FTS pairs. Procurement: the TRIMMED row is older (1880052 on
    // prod), so the keep is already on the corrected key. ScotRail: the NOISY
    // row is older, so the keep itself must be re-keyed inside its fold.
    legacy_org(&conn, 10, Some("GB"), "Procurement for Housing").await;
    legacy_org(&conn, 11, Some("GB"), "Procurement for Housing ").await;
    legacy_org(&conn, 20, Some("GB"), "ScotRail Trains Limited .").await;
    legacy_org(&conn, 21, Some("GB"), "ScotRail Trains Limited").await;
    // Two noisy spellings and no clean one: a group of re-keyed rows alone.
    legacy_org(&conn, 80, Some("GB"), "Foo Ltd.").await;
    legacy_org(&conn, 81, Some("GB"), "FOO LTD .").await;
    // The control: different keys under the new rule too — never folded.
    legacy_org(&conn, 30, Some("GB"), "ACME Ltd").await;
    legacy_org(&conn, 31, Some("GB"), "ACME").await;
    // Another country: re-keyed, but a different body from the GB rows.
    legacy_org(&conn, 40, Some("FR"), "Procurement for Housing.").await;
    // Country-less: re-keyed, never folded here — identical country-less keys
    // are the echo fold's, behind the wall.
    legacy_org(&conn, 50, None, "Stadt  Echo").await;
    legacy_org(&conn, 51, None, "Stadt Echo").await;
    // A blank-but-for-whitespace name: its key becomes empty (nameless class).
    legacy_org(&conn, 70, Some("GB"), "   ").await;
    // A re-keyed singleton.
    legacy_org(&conn, 90, Some("GB"), "Solo Ltd.").await;
    // Identifier-bearing: not the reuse key's class, untouched.
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (60, 'GB', 'national', 'X1', 'Procurement for Housing ', 'procurement for housing ', 0, 0)",
        (),
    )
    .await
    .unwrap();

    mention_row(&conn, 1, 11, "Procurement for Housing ").await;
    mention_row(&conn, 2, 10, "Procurement for Housing").await;
    mention_row(&conn, 3, 21, "ScotRail Trains Limited").await;
    mention_row(&conn, 4, 20, "ScotRail Trains Limited .").await;
    mention_row(&conn, 5, 81, "FOO LTD .").await;
    // 8579929's shape: two versions, one per spelling, of one tender.
    for (seq, org, notice) in [(1, 10, 2), (2, 11, 1)] {
        conn.execute(
            "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id)
             VALUES (8579929, ?, NULL, 'buyer', ?, ?, 'ORG-1')",
            (Value::Integer(seq), Value::Integer(org), Value::Integer(notice)),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (500, 1, 9, 'tenderer', 81, 5, 'ORG-1')",
        (),
    )
    .await
    .unwrap();
    // The same award on both ScotRail rows: the loser's winner row is a dup.
    for org in [20, 21] {
        conn.execute(
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
             VALUES (7957971, 1, 77, ?)",
            (Value::Integer(org),),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (11, 'ENG', 'Procurement for Housing ', 'procurement for housing ')",
        (),
    )
    .await
    .unwrap();
    // Verdict keys predating the rule: one moves, one lands on a standing
    // verdict and is left where it is.
    db.record_name_verdicts(
        "t",
        &[
            store::NameVerdict { name_norm: "stadt echo.".into(), verdict: "single".into(), rationale: "one city".into() },
            store::NameVerdict { name_norm: "kreis zwei".into(), verdict: "generic".into(), rationale: "a class".into() },
            store::NameVerdict { name_norm: "kreis zwei.".into(), verdict: "single".into(), rationale: "disagrees".into() },
        ],
        0,
    )
    .await
    .unwrap();

    let never = || false;
    let quiet = |_: u64, _: &str| {};
    let dry = db.repair_provisional_name_norm(args(true, None, &never, &quiet)).await.unwrap();
    assert_eq!(dry.rows_walked, 13, "every identifier-less row; 60 carries an identifier");
    assert_eq!(dry.renormalised, 8, "11, 20, 40, 50, 70, 80, 81, 90");
    assert_eq!((dry.renormalised_country_less, dry.emptied), (1, 1));
    assert_eq!(dry.groups, 3, "procurement/GB, scotrail/GB, foo ltd/GB");
    assert_eq!((dry.group_rows, dry.standing_twins, dry.fold_rows), (6, 2, 3));
    assert_eq!(dry.verdicts_rekeyed, 1);
    assert_eq!(dry.verdict_conflicts, vec![("kreis zwei.".to_owned(), "kreis zwei".to_owned())]);
    assert!(dry.listing.contains(&(
        "scotrail trains limited".to_owned(),
        "GB".to_owned(),
        "ScotRail Trains Limited .".to_owned(),
        2,
        20
    )));
    assert!(dry.sample.contains(&(11, "Procurement for Housing ".to_owned(), Some("procurement for housing ".to_owned()), "procurement for housing".to_owned())));
    assert_eq!(dry.residual_rows, 8);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 14, "a dry run writes nothing");
    assert_eq!(norm_of(&db, 11).await, "procurement for housing ");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_name_verdicts WHERE name_norm = 'stadt echo.'").await, 1);

    // Parity: a plan that is not the recorded one aborts before any write.
    let off = db.repair_provisional_name_norm(args(false, Some((200, 8)), &never, &quiet)).await;
    assert!(off.is_err(), "3 groups vs a recorded 200 is outside max(2%, 50)");
    let off = db.repair_provisional_name_norm(args(false, Some((3, 900)), &never, &quiet)).await;
    assert!(off.is_err(), "8 rows vs a recorded 900 is outside max(2%, 50)");
    // And a stop before the first window writes nothing either.
    let halt = || true;
    let stopped = db.repair_provisional_name_norm(args(false, Some((3, 8)), &halt, &quiet)).await.unwrap();
    assert!(stopped.stopped);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 14);
    assert_eq!(norm_of(&db, 11).await, "procurement for housing ");

    let wet = db.repair_provisional_name_norm(args(false, Some((3, 8)), &never, &quiet)).await.unwrap();
    assert!(!wet.stopped);
    assert_eq!((wet.merged_groups, wet.removed), (3, 3));
    assert_eq!(wet.mentions, 3, "off 11, 21 and 81");
    assert_eq!((wet.parties, wet.bid_parties, wet.winners, wet.winner_dups), (1, 1, 0, 1));
    assert_eq!(wet.rewritten, 4, "40, 50, 70, 90 — the fold keeps were re-keyed in their folds");
    assert_eq!((wet.verdicts_rekeyed, wet.verdicts_moved), (1, 1));
    assert_eq!(wet.residual_rows, 0);
    assert!(db.foreign_keys_enabled().await.unwrap(), "the wet phase's foreign-keys OFF is bracketed");

    // The keeps are the lowest ids, still provisional, on the corrected key.
    for (keep, gone, key) in [
        (10, 11, "procurement for housing"),
        (20, 21, "scotrail trains limited"),
        (80, 81, "foo ltd"),
    ] {
        assert_eq!(count(&db, &format!("SELECT COUNT(*) FROM organizations WHERE id = {gone}")).await, 0);
        assert_eq!(count(&db, &format!("SELECT COUNT(*) FROM organizations WHERE id = {keep} AND provisional = 1")).await, 1);
        assert_eq!(norm_of(&db, keep).await, key);
        assert_eq!(
            count(&db, &format!("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'p1' AND keep = {keep} AND loser = {gone} AND job_id = 432")).await,
            1
        );
        assert_eq!(
            count(&db, &format!("SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = {gone} AND op = 'removed'")).await,
            1
        );
    }
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_merge_log").await, 3);
    // Every reference moved with its row.
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 10").await, 2);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 20").await, 2);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 80").await, 1);
    assert_eq!(
        count(&db, "SELECT COUNT(DISTINCT organization_id) FROM tender_version_parties WHERE tender_id = 8579929").await,
        1,
        "8579929's two versions now name ONE buyer — issue 386's gauge reads it as agreeing"
    );
    assert_eq!(count(&db, "SELECT COUNT(*) FROM tender_version_bid_parties WHERE organization_id = 80").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM tender_version_result_winners WHERE tender_id = 7957971").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id = 20").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_names WHERE org_id = 10 AND lang = 'ENG'").await, 1, "the loser's variants ride along");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_names WHERE org_id = 11").await, 0);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id IN (8579929, 7957971, 500) AND op = 'changed'").await,
        3
    );
    // Re-keyed without folding.
    assert_eq!(norm_of(&db, 40).await, "procurement for housing", "FR: its own row");
    assert_eq!(norm_of(&db, 50).await, "stadt echo");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt echo' AND country IS NULL").await, 2, "country-less: left to the echo fold");
    assert_eq!(norm_of(&db, 70).await, "");
    assert_eq!(norm_of(&db, 90).await, "solo ltd");
    // Untouched.
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations WHERE id IN (30, 31)").await, 2, "ACME Ltd and ACME stay two");
    assert_eq!(norm_of(&db, 30).await, "acme ltd");
    assert_eq!(norm_of(&db, 60).await, "procurement for housing ", "identifier-bearing: out of scope");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 11);
    // The verdicts: moved where free, left standing on a conflict.
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_name_verdicts WHERE name_norm = 'stadt echo' AND verdict = 'single'").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_name_verdicts WHERE name_norm = 'stadt echo.'").await, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_name_verdicts WHERE name_norm IN ('kreis zwei', 'kreis zwei.')").await, 2);

    // The stock now agrees with the resolver: a fresh noisy mention binds the keep.
    db.set_foreign_keys(false).await.unwrap();
    let bound = resolve(&db, &[mention(6, "Procurement for Housing  .", "GB"), mention(7, "ScotRail Trains Limited", "GB")]).await;
    assert_eq!(bound, vec![10, 20]);

    // Idempotent: nothing left to re-key or fold; the conflict still reports.
    let again = db.repair_provisional_name_norm(args(true, None, &never, &quiet)).await.unwrap();
    assert_eq!((again.renormalised, again.groups, again.verdicts_rekeyed), (0, 0, 0));
    assert_eq!(again.verdict_conflicts.len(), 1);
}
