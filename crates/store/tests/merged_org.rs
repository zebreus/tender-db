//! Issue 455: `read::resolve_org` follows `org_merge_log` from an organization
//! id a merge removed to its live survivor, so the API can redirect a stale id
//! instead of answering a bare 404 or a certified-empty filtered page.

use store::read::{MERGE_HOPS, OrgResolution, resolve_org};

async fn exec(conn: &turso::Connection, sql: &str) {
    conn.execute(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

async fn org(conn: &turso::Connection, id: i64) {
    exec(conn, &format!(
        "INSERT INTO organizations (id, name, name_norm, country, provisional, created_at) \
         VALUES ({id}, 'org {id}', 'org {id}', 'GB', 0, 0)"
    ))
    .await;
}

async fn merged(conn: &turso::Connection, keep: i64, loser: i64, at: i64) {
    exec(conn, &format!(
        "INSERT INTO org_merge_log (keep, loser, rule, evidence, at) VALUES ({keep}, {loser}, 'r2', '{{}}', {at})"
    ))
    .await;
}

#[tokio::test]
async fn a_merged_away_id_resolves_through_the_ledger_to_its_live_survivor() {
    let path = format!("/tmp/tender-db-455-resolve-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let _db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");

    // A two-hop chain: 1 merged into 2, and 2 later merged into the live 3.
    org(&conn, 3).await;
    merged(&conn, 2, 1, 100).await;
    merged(&conn, 3, 2, 200).await;
    // A loser merged twice (into 10, restored by a later mint, then into 20): the
    // NEWEST merge is where it lives now.
    org(&conn, 10).await;
    org(&conn, 20).await;
    merged(&conn, 10, 5, 100).await;
    merged(&conn, 20, 5, 300).await;
    // A cycle with no live row on it.
    merged(&conn, 7, 6, 100).await;
    merged(&conn, 6, 7, 200).await;
    // A chain one hop longer than the bound: 100 → 101 → … → the live 100 + MERGE_HOPS + 1.
    let hops = MERGE_HOPS as i64;
    org(&conn, 100 + hops + 1).await;
    for loser in 100..=100 + hops {
        merged(&conn, loser + 1, loser, 100).await;
    }

    assert_eq!(resolve_org(&conn, 3).await.unwrap(), OrgResolution::Live);
    assert_eq!(resolve_org(&conn, 1).await.unwrap(), OrgResolution::MergedInto(3), "two hops");
    assert_eq!(resolve_org(&conn, 2).await.unwrap(), OrgResolution::MergedInto(3), "one hop");
    assert_eq!(resolve_org(&conn, 5).await.unwrap(), OrgResolution::MergedInto(20), "newest merge wins");
    assert_eq!(resolve_org(&conn, 6).await.unwrap(), OrgResolution::Unknown, "a cycle ends the walk");
    assert_eq!(resolve_org(&conn, 999).await.unwrap(), OrgResolution::Unknown, "never an org");
    assert_eq!(
        resolve_org(&conn, 101).await.unwrap(),
        OrgResolution::MergedInto(100 + hops + 1),
        "exactly MERGE_HOPS hops resolve"
    );
    assert_eq!(resolve_org(&conn, 100).await.unwrap(), OrgResolution::Unknown, "one hop past the bound");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

// ---------------------------------------------------------------- issue 460

/// One test-local canonical key for the R2 and rekey arms: the GB company
/// number with any `GB`/`GBCOH` prefix, and Sellafield's PPON keyed onto its
/// company number (the pair 448's altid merge folded on prod).
fn gb_key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    if kind != "national" || country != Some("GB") {
        return None;
    }
    if value == "GBPPONPWYP8439MZWY" {
        return Some(("GB:coh", "01002607".into(), true));
    }
    let body = value.strip_prefix("GBCOH").or_else(|| value.strip_prefix("GB")).unwrap_or(value);
    (body.len() == 8 && body.bytes().all(|b| b.is_ascii_digit())).then(|| ("GB:coh", body.to_owned(), true))
}

fn r2_args(dry_run: bool) -> store::R2MergeArgs<'static> {
    store::R2MergeArgs {
        key: gb_key,
        condemns: |_, _, _| false,
        consortium: |_| false,
        legal_form: |_| None,
        rule: "r2",
        n3: |n| n.to_lowercase(),
        stoplist_cap: 20,
        dry_run,
        max_groups: None,
        expect_groups: None,
        job_id: Some(460),
        stop: &|| false,
    }
}

fn rekey_args(dry_run: bool, expect: Option<Vec<String>>) -> store::RekeyArgs<'static> {
    store::RekeyArgs {
        key: gb_key,
        consortium: |_| false,
        legal_family: |n| n.to_lowercase().ends_with(" ltd").then_some("ltd"),
        name_key: |n| n.to_lowercase(),
        names_agree: |a, b| a == b,
        dry_run,
        max_rekeys: None,
        expect,
        job_id: Some(460),
        stop: &|| false,
    }
}

async fn gb_org(conn: &turso::Connection, id: i64, identifier: &str, name: &str, provisional: i64) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, 'GB', 'national', ?, ?, ?, ?, 0)",
        (
            turso::Value::Integer(id),
            turso::Value::Text(identifier.into()),
            turso::Value::Text(name.into()),
            turso::Value::Text(name.to_lowercase()),
            turso::Value::Integer(provisional),
        ),
    )
    .await
    .unwrap();
}

/// What `?identifier=` answers in BOTH org builders — the id-ordered page and the
/// name-ordered search — asserted equal, so the two never disagree about a merge.
async fn lookup(conn: &turso::Connection, identifier: &str, kind: Option<&str>, country: Option<&str>) -> Vec<i64> {
    use store::read::{self, Filter, Scope};
    let filter = Filter {
        identifier: Some(identifier.into()),
        kind: kind.map(str::to_owned),
        country: country.map(str::to_owned),
        ..Filter::default()
    };
    let by_id: Vec<i64> = read::organizations(conn, &filter, Scope::Page { after: 0, limit: 100 })
        .await
        .unwrap()
        .iter()
        .map(|r| r.id)
        .collect();
    let by_name: Vec<i64> = read::organizations_by_name(conn, &filter, "sellafield", None, 100)
        .await
        .unwrap()
        .iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(by_id, by_name, "the two org builders disagree on ?identifier={identifier}");
    by_id
}

/// Issue 460: every merge deletes the loser's row, and with it the only place
/// the identity index held the loser's identifier. `repoint_org_references` now
/// keeps it in `organization_merged_identifiers` for the survivor, carries it
/// along a chain of merges, and leaves a `rekey` loser's number out (it is
/// another company's, 452/453); both builders read it.
#[tokio::test]
async fn a_merged_away_identifier_finds_its_survivor() {
    use store::read::{self, Filter, Scope};
    let path = format!("/tmp/tender-db-460-merged-identifier-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let conn = turso::Builder::new_local(&path).build().await.expect("raw").connect().expect("connect");

    // Sellafield's PPON org (30) and its company-number org (20), and Crown
    // Commercial Service (50), whose PPON nothing ever merged.
    gb_org(&conn, 20, "01002607", "Sellafield Ltd", 0).await;
    gb_org(&conn, 30, "GBPPONPWYP8439MZWY", "Sellafield Ltd", 1).await;
    gb_org(&conn, 50, "GBPPONPBZB4962TVLR", "Sellafield Crown Commercial Service", 0).await;
    // A wrong-number org: a reviewer found 02202746 is not Sellafield's.
    gb_org(&conn, 40, "02202746", "Sellafield Ltd", 0).await;

    let wet = db.match_org_identifiers_r2(r2_args(false)).await.expect("r2");
    assert_eq!(wet.removed, 1, "the PPON org folds into the company-number org");
    assert_eq!(lookup(&conn, "GBPPONPWYP8439MZWY", None, None).await, vec![20], "the PPON finds its survivor");
    assert_eq!(lookup(&conn, "01002607", None, None).await, vec![20], "the survivor's own identifier still matches");

    // A second merge: the company-number org folds into a third (10, another
    // spelling of the same number). The PPON travels with it, and so does 20's
    // own number.
    gb_org(&conn, 10, "GB01002607", "Sellafield Ltd", 0).await;
    let wet = db.match_org_identifiers_r2(r2_args(false)).await.expect("r2 again");
    assert_eq!(wet.removed, 1);
    assert_eq!(lookup(&conn, "GBPPONPWYP8439MZWY", None, None).await, vec![10], "A→B→C carries A's identifier to C");
    assert_eq!(lookup(&conn, "01002607", None, None).await, vec![10], "B's own identifier lands on C");
    assert_eq!(lookup(&conn, "GB01002607", None, None).await, vec![10]);

    // The rekey: 40's number is another company's, so a lookup by it must not
    // answer Sellafield once 40 folds into it.
    let verdict = store::IdentifierVerdict {
        org_id: 40,
        identifier: "02202746".into(),
        verdict: "wrong".into(),
        correct_identifier: Some("01002607".into()),
        rationale: "fixture".into(),
        confidence: "high".into(),
    };
    db.record_identifier_verdicts("452", &[verdict], 0).await.expect("verdict");
    let dry = db.match_org_rekey(rekey_args(true, None)).await.expect("rekey dry");
    assert_eq!(dry.plan_merge, 1, "{:#?}", dry.denied);
    let wet = db.match_org_rekey(rekey_args(false, Some(dry.keys))).await.expect("rekey wet");
    assert_eq!(wet.merged, 1);
    assert!(lookup(&conn, "02202746", None, None).await.is_empty(), "a rekey loser's number is not written");
    assert_eq!(lookup(&conn, "GBPPONPWYP8439MZWY", None, None).await, vec![10], "the rekey left the PPON standing");

    // A live org's own identifier, and a value nothing ever held.
    assert_eq!(lookup(&conn, "GBPPONPBZB4962TVLR", None, None).await, vec![50]);
    assert!(lookup(&conn, "GBPPONZZZZ0000ZZZZ", None, None).await.is_empty());
    // `kind` constrains the MATCHED identifier's kind; `country` the org's.
    assert_eq!(lookup(&conn, "GBPPONPWYP8439MZWY", Some("national"), None).await, vec![10]);
    assert!(lookup(&conn, "GBPPONPWYP8439MZWY", Some("vat"), None).await.is_empty());
    assert_eq!(lookup(&conn, "GBPPONPWYP8439MZWY", None, Some("GB")).await, vec![10]);
    assert!(lookup(&conn, "GBPPONPWYP8439MZWY", None, Some("FR")).await.is_empty());

    // The cursor and the single-row probe (the SSE diff's shape) agree.
    let ppon = Filter { identifier: Some("GBPPONPWYP8439MZWY".into()), ..Filter::default() };
    assert!(read::organizations(&conn, &ppon, Scope::Page { after: 10, limit: 100 }).await.unwrap().is_empty());
    assert_eq!(read::organizations(&conn, &ppon, Scope::At { id: 10, seq: 0 }).await.unwrap().len(), 1);
    assert!(read::organizations(&conn, &ppon, Scope::At { id: 50, seq: 0 }).await.unwrap().is_empty());

    // The survivor names what it carries, and the list rows agree with it.
    let survivor = read::organizations(&conn, &Filter::default(), Scope::At { id: 10, seq: 0 }).await.unwrap();
    assert_eq!(
        survivor[0].merged_identifiers,
        vec![merged_identifier("01002607"), merged_identifier("GBPPONPWYP8439MZWY")],
        "both folded identifiers, the rekey's wrong number not among them"
    );
    let page = read::organizations(&conn, &ppon, Scope::Page { after: 0, limit: 100 }).await.unwrap();
    assert_eq!(page[0].merged_identifiers, survivor[0].merged_identifiers);
    let live = read::organizations(&conn, &Filter::default(), Scope::At { id: 50, seq: 0 }).await.unwrap();
    assert!(live[0].merged_identifiers.is_empty(), "nothing was ever merged into 50");

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

fn merged_identifier(identifier: &str) -> store::read::MergedIdentifier {
    store::read::MergedIdentifier {
        identifier: identifier.into(),
        identifier_kind: Some("national".into()),
        country: Some("GB".into()),
    }
}

async fn ledger(conn: &turso::Connection, keep: i64, loser: i64, rule: &str, evidence: &str, at: i64) {
    conn.execute(
        "INSERT INTO org_merge_log (keep, loser, rule, evidence, at) VALUES (?, ?, ?, ?, ?)",
        (
            turso::Value::Integer(keep),
            turso::Value::Integer(loser),
            turso::Value::Text(rule.into()),
            turso::Value::Text(evidence.into()),
            turso::Value::Integer(at),
        ),
    )
    .await
    .unwrap();
}

/// A miniature of `ingest::project::normalise_identifier`'s kind: a PPON or a
/// bare company number is `national`, `GB` + nine digits a `vat`, and anything
/// else unclassified (the survivor's kind is taken and counted).
fn kind_of(_country: Option<&str>, literal: &str) -> Option<String> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if literal.starts_with("GBPPON") || digits(literal) {
        Some("national".into())
    } else if literal.strip_prefix("GB").is_some_and(|b| b.len() == 9 && digits(b)) {
        Some("vat".into())
    } else {
        None
    }
}

/// Issue 460 unit 3: every merge before the table existed left its loser's
/// literal only in the ledger's evidence. The one-shot backfill reads it per
/// rule (`e2-altid` through its partial index, `r2`/`e0`/`r3` through a rowid
/// walk), follows each keep to today's survivor, and writes what is missing —
/// dry first, idempotent after.
#[tokio::test]
async fn the_altid_ledger_backfills_merged_identifiers() {
    use store::read::{self, Filter, Scope};
    let path = format!("/tmp/tender-db-460-backfill-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let conn = turso::Builder::new_local(&path).build().await.expect("raw").connect().expect("connect");

    // Today's live rows. 20 (the PPON's first survivor) was itself folded into 10.
    gb_org(&conn, 10, "GB01002607", "Sellafield Ltd", 0).await;
    gb_org(&conn, 60, "12345678", "Acme Ltd", 0).await;
    gb_org(&conn, 64, "64646464", "Acme Restored Ltd", 0).await;
    gb_org(&conn, 70, "SC123456", "Beta Ltd", 0).await;
    // A row the merge itself wrote, as every merge since issue 460 does.
    conn.execute(
        "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
         VALUES ('GBPPONPRES0001AAAA', 'national', 'GB', 10, 33, 'e2-altid')",
        (),
    )
    .await
    .unwrap();

    // The ledger, in merge order.
    ledger(&conn, 20, 30, "e2-altid", r#"{"scheme":"GB:altid","loser_id":"GBPPONPWYP8439MZWY"}"#, 100).await;
    ledger(&conn, 10, 20, "r2", r#"{"scheme":"GB:coh","key":"01002607","keep_id":"GB01002607","loser_id":"01002607"}"#, 200).await;
    ledger(&conn, 60, 61, "e0", r#"{"scheme":"GB:coh","loser_id":"12345678"}"#, 210).await;
    ledger(&conn, 70, 71, "r3", r#"{"scheme":"GB:coh","loser_id":"GBSC123456"}"#, 220).await;
    ledger(&conn, 60, 62, "p0", r#"{"name_norm":"acme ltd","keep_id":"60","loser_id":"62"}"#, 230).await;
    ledger(&conn, 60, 63, "rekey", r#"{"scheme":"GB:rekey","wrong":"99999999","right":"12345678"}"#, 240).await;
    ledger(&conn, 10, 31, "e2-altid", r#"{"scheme":"GB:altid"}"#, 250).await;
    ledger(&conn, 999, 32, "e2-altid", r#"{"scheme":"GB:altid","loser_id":"GBPPONGONE0000AAAA"}"#, 260).await;
    ledger(&conn, 60, 64, "r2", r#"{"scheme":"GB:coh","loser_id":"64646464"}"#, 270).await;
    ledger(&conn, 10, 33, "e2-altid", r#"{"scheme":"GB:altid","loser_id":"GBPPONPRES0001AAAA"}"#, 280).await;
    ledger(&conn, 60, 65, "r2", r#"{"scheme":"GB:coh","loser_id":"GB123456789"}"#, 290).await;
    // Two rows a from-archive rebuild would leave behind, whose ids now name
    // something else: a keep minted AFTER the merge that names it, and a keep
    // whose own merge happened BEFORE the merge into it.
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (80, 'GB', 'national', '80808080', 'Someone Else Ltd', 'someone else ltd', 0, 1000)",
        (),
    )
    .await
    .unwrap();
    ledger(&conn, 80, 81, "r2", r#"{"scheme":"GB:coh","loser_id":"GB808080808"}"#, 300).await;
    ledger(&conn, 10, 83, "p0", r#"{"loser_id":"83"}"#, 50).await;
    ledger(&conn, 83, 84, "r2", r#"{"scheme":"GB:coh","loser_id":"84848484"}"#, 310).await;

    // Two ledger rows a window: the walk crosses seven windows over these fourteen.
    let args =
        |dry_run: bool| store::MergedIdentifierBackfillArgs { kind_of, key: gb_key, dry_run, window: 2, stop: &|| false };
    let counts = |r: &store::MergedIdentifierBackfill, rule: &str| {
        r.rules.iter().find(|(name, _)| name == rule).map(|(_, c)| c.clone()).expect("rule counted")
    };
    let expect = |rows, written, present, same, no_literal, unresolved, out_of_time, loser_live, kind_from_survivor| {
        store::MergedIdentifierRuleCounts {
            rows,
            written,
            present,
            same_as_survivor: same,
            no_literal,
            unresolved,
            out_of_time,
            wrong_number: 0,
            loser_live,
            kind_from_survivor,
        }
    };
    let merged_rows = async || -> i64 {
        let mut rows = conn.query("SELECT COUNT(*) FROM organization_merged_identifiers", ()).await.unwrap();
        let row = rows.next().await.unwrap().unwrap();
        row.get_value(0).unwrap().as_integer().copied().unwrap()
    };

    // Dry: every class counted per rule, nothing written.
    let dry = db.backfill_merged_identifiers(args(true)).await.expect("dry");
    assert_eq!(dry.ledger_rows, 14, "the walk reads the whole ledger");
    assert_eq!(counts(&dry, "e2-altid"), expect(4, 1, 1, 0, 1, 1, 0, 0, 0));
    assert_eq!(counts(&dry, "r2"), expect(5, 2, 0, 0, 0, 0, 2, 1, 0), "both rebuild-stale rows refused");
    assert_eq!(counts(&dry, "e0"), expect(1, 0, 0, 1, 0, 0, 0, 0, 0), "e0 merges identical triples: nothing lost");
    assert_eq!(counts(&dry, "r3"), expect(1, 1, 0, 0, 0, 0, 0, 0, 1));
    assert_eq!(merged_rows().await, 1, "dry wrote nothing");

    // Wet: the same counts, written to TODAY's survivor.
    let wet = db.backfill_merged_identifiers(args(false)).await.expect("wet");
    for rule in store::MERGED_IDENTIFIER_BACKFILL_RULES {
        assert_eq!(counts(&wet, rule), counts(&dry, rule), "{rule}: wet does what dry counted");
    }
    assert_eq!(merged_rows().await, 5);
    assert_eq!((dry.organizations_changed, wet.organizations_changed), (3, 3), "10, 60 and 70 serve more");
    let mut rows = conn
        .query("SELECT entity_id FROM changes WHERE entity_kind = 'organization' AND op = 'changed' ORDER BY entity_id", ())
        .await
        .unwrap();
    let mut changed = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        changed.push(row.get_value(0).unwrap().as_integer().copied().unwrap());
    }
    drop(rows);
    assert_eq!(changed, vec![10, 60, 70], "each survivor that gained one is an `organization changed`");
    assert_eq!(lookup(&conn, "GBPPONPWYP8439MZWY", None, None).await, vec![10], "through 20 to 10");
    assert_eq!(lookup(&conn, "01002607", None, None).await, vec![10]);
    let by = async |identifier: &str, kind: Option<&str>| -> Vec<i64> {
        let f = Filter { identifier: Some(identifier.into()), kind: kind.map(str::to_owned), ..Filter::default() };
        read::organizations(&conn, &f, Scope::Page { after: 0, limit: 10 }).await.unwrap().iter().map(|r| r.id).collect()
    };
    assert_eq!(by("GBSC123456", Some("national")).await, vec![70], "r3: the survivor's kind, counted");
    assert_eq!(by("GB123456789", Some("vat")).await, vec![60], "r2: the literal's own kind, not the survivor's");
    assert!(by("GB123456789", Some("national")).await.is_empty());
    assert!(by("99999999", None).await.is_empty(), "a rekey's wrong number is never backfilled");
    assert!(by("62", None).await.is_empty(), "a p0 loser_id is an org id, never read");
    assert!(by("GBPPONGONE0000AAAA", None).await.is_empty());
    assert!(by("GB808080808", None).await.is_empty(), "never attached to a row minted after its merge");
    assert!(by("84848484", None).await.is_empty(), "never through a hop older than the merge it continues");

    // Idempotent: a second run finds every row standing.
    let again = db.backfill_merged_identifiers(args(false)).await.expect("again");
    assert_eq!(counts(&again, "e2-altid"), expect(4, 0, 2, 0, 1, 1, 0, 0, 0));
    assert_eq!(counts(&again, "r2"), expect(5, 0, 2, 0, 0, 0, 2, 1, 0));
    assert_eq!(merged_rows().await, 5);
    assert_eq!(again.organizations_changed, 0, "a re-run that writes nothing publishes nothing");

    // A cancel before the first window writes nothing and says so.
    let stopped = db
        .backfill_merged_identifiers(store::MergedIdentifierBackfillArgs {
            kind_of,
            key: gb_key,
            dry_run: false,
            window: store::MERGED_BACKFILL_WINDOW,
            stop: &|| true,
        })
        .await
        .expect("stopped");
    assert!(stopped.stopped);

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

// ------------------------------------------- issue 460 review: wrong numbers

/// A `wrong` identifier verdict (issue 452) on `(identifier, national, GB)`,
/// written as the table holds it — the backfill tests name orgs a merge
/// removed, which `record_identifier_verdicts` cannot read. `applied` is the
/// literal a re-key stamped (453), or `None` for a verdict not yet acted on.
async fn wrong_verdict(conn: &turso::Connection, identifier: &str, org: i64, right: &str, applied: Option<&str>) {
    conn.execute(
        "INSERT INTO org_identifier_verdicts (identifier, identifier_kind, country, org_id, cohort, verdict,
             correct_identifier, rationale, confidence, reviewed_at, applied_at, applied_literal, job_id)
         VALUES (?, 'national', 'GB', ?, '452', 'wrong', ?, 'fixture', 'high', 0, ?, ?, NULL)",
        (
            turso::Value::Text(identifier.into()),
            turso::Value::Integer(org),
            turso::Value::Text(right.into()),
            if applied.is_some() { turso::Value::Integer(1) } else { turso::Value::Null },
            applied.map_or(turso::Value::Null, |a| turso::Value::Text(a.into())),
        ),
    )
    .await
    .unwrap();
}

/// What `organization_merged_identifiers` holds, `(identifier, org_id)` in
/// identifier order.
async fn merged_table(conn: &turso::Connection) -> Vec<(String, i64)> {
    let mut rows = conn
        .query("SELECT identifier, org_id FROM organization_merged_identifiers ORDER BY identifier, loser", ())
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        out.push((
            row.get_value(0).unwrap().as_text().cloned().unwrap(),
            row.get_value(1).unwrap().as_integer().copied().unwrap(),
        ));
    }
    out
}

fn fresh_path(name: &str) -> String {
    let path = format!("/tmp/tender-db-460-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    path
}

fn remove_db(path: &str) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Review R1 (D1): a rekey loser's OWN number was left out, but the identifiers
/// it carried moved to the right company — and an earlier r2 had folded the
/// bare spelling of the very wrong number into it. The rekey drops every
/// spelling of the wrong number from the entity, and carries the rest (a PPON
/// an altid merge folded into the loser) as any merge does.
#[tokio::test]
async fn a_rekey_merge_never_carries_a_spelling_of_the_wrong_number() {
    let path = fresh_path("rekey-merge-spelling");
    let db = store::Db::open(&path).await.expect("open");
    let conn = turso::Builder::new_local(&path).build().await.expect("raw").connect().expect("connect");
    gb_org(&conn, 20, "01002607", "Sellafield Ltd", 0).await;
    // Two spellings of another company's number, both on Sellafield's mentions.
    gb_org(&conn, 39, "GB02202746", "Sellafield Ltd", 0).await;
    gb_org(&conn, 40, "02202746", "Sellafield Ltd", 0).await;
    let wet = db.match_org_identifiers_r2(r2_args(false)).await.expect("r2");
    assert_eq!(wet.removed, 1, "r2 folds the bare spelling into 39");
    assert_eq!(lookup(&conn, "02202746", None, None).await, vec![39]);
    // A PPON an altid merge folded into 39 — Sellafield's, whatever its number.
    conn.execute(
        "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
         VALUES ('GBPPONPAAA0001AAAA', 'national', 'GB', 39, 41, 'e2-altid')",
        (),
    )
    .await
    .unwrap();

    let verdict = store::IdentifierVerdict {
        org_id: 39,
        identifier: "GB02202746".into(),
        verdict: "wrong".into(),
        correct_identifier: Some("01002607".into()),
        rationale: "fixture".into(),
        confidence: "high".into(),
    };
    db.record_identifier_verdicts("452", &[verdict], 0).await.expect("verdict");
    let dry = db.match_org_rekey(rekey_args(true, None)).await.expect("rekey dry");
    assert_eq!(dry.plan_merge, 1, "{:#?}", dry.denied);
    let wet = db.match_org_rekey(rekey_args(false, Some(dry.keys))).await.expect("rekey wet");
    assert_eq!(wet.merged, 1);

    assert!(lookup(&conn, "02202746", None, None).await.is_empty(), "the bare wrong number answers nobody");
    assert!(lookup(&conn, "GB02202746", None, None).await.is_empty(), "nor the spelling the loser carried");
    assert_eq!(lookup(&conn, "GBPPONPAAA0001AAAA", None, None).await, vec![20], "the PPON travels to the entity");
    assert_eq!(merged_table(&conn).await, vec![("GBPPONPAAA0001AAAA".into(), 20)]);
    drop(db);
    remove_db(&path);
}

/// Review R5 (D3): the move arm re-keys the org in place, so it is its own
/// survivor — and the bare wrong number an earlier r2 folded into it stayed a
/// merged identifier of it, answering beside the right number. The move drops
/// it.
#[tokio::test]
async fn a_rekey_move_drops_the_wrong_numbers_folded_spellings() {
    let path = fresh_path("rekey-move-spelling");
    let db = store::Db::open(&path).await.expect("open");
    let conn = turso::Builder::new_local(&path).build().await.expect("raw").connect().expect("connect");
    gb_org(&conn, 39, "GB02202746", "Sellafield Ltd", 0).await;
    gb_org(&conn, 40, "02202746", "Sellafield Ltd", 0).await;
    db.match_org_identifiers_r2(r2_args(false)).await.expect("r2");
    let verdict = store::IdentifierVerdict {
        org_id: 39,
        identifier: "GB02202746".into(),
        verdict: "wrong".into(),
        correct_identifier: Some("01002607".into()),
        rationale: "fixture".into(),
        confidence: "high".into(),
    };
    db.record_identifier_verdicts("452", &[verdict], 0).await.expect("verdict");
    let dry = db.match_org_rekey(rekey_args(true, None)).await.expect("rekey dry");
    assert_eq!(dry.plan_move, 1, "{:#?}", dry.denied);
    let wet = db.match_org_rekey(rekey_args(false, Some(dry.keys))).await.expect("rekey wet");
    assert_eq!(wet.moved, 1);
    assert_eq!(lookup(&conn, "01002607", None, None).await, vec![39], "moved onto the right number");
    assert!(lookup(&conn, "02202746", None, None).await.is_empty(), "the folded wrong number left with the move");
    assert!(merged_table(&conn).await.is_empty());
    drop(db);
    remove_db(&path);
}

/// The backfill's args over [`kind_of`], every row in one window.
fn backfill(dry_run: bool) -> store::MergedIdentifierBackfillArgs<'static> {
    store::MergedIdentifierBackfillArgs { kind_of, key: gb_key, dry_run, window: 100, stop: &|| false }
}

/// Review R2 (D2, and D3's backfill half): the backfill followed a keep THROUGH
/// a `rekey` hop, so an r2 loser's bare spelling of the wrong number — and an
/// e0 loser's identical one — landed on the right company; and a MOVED org is
/// its own live survivor, so its r2 loser's wrong spelling landed on it. A
/// literal keyed like a number a reviewer found wrong is refused unless the
/// survivor carries that number itself: the real owner of a number keeps its
/// folded spellings.
#[tokio::test]
async fn the_backfill_never_writes_a_number_a_reviewer_found_wrong() {
    let path = fresh_path("backfill-wrong-number");
    let db = store::Db::open(&path).await.expect("open");
    let conn = turso::Builder::new_local(&path).build().await.expect("raw").connect().expect("connect");
    // Sellafield (20), which a rekey merged 39 (GB02202746, wrong) into.
    gb_org(&conn, 20, "01002607", "Sellafield Ltd", 0).await;
    wrong_verdict(&conn, "GB02202746", 39, "01002607", Some("01002607")).await;
    ledger(&conn, 39, 40, "r2", r#"{"scheme":"GB:coh","keep_id":"GB02202746","loser_id":"02202746"}"#, 100).await;
    ledger(&conn, 39, 41, "e0", r#"{"scheme":"GB:coh","loser_id":"GB02202746"}"#, 110).await;
    ledger(&conn, 20, 39, "rekey", r#"{"scheme":"GB:rekey","wrong":"GB02202746","right":"01002607"}"#, 200).await;
    // A moved org (50): GB03333333 was not its number; it carries 01111111 since.
    gb_org(&conn, 50, "01111111", "Moved Ltd", 0).await;
    wrong_verdict(&conn, "GB03333333", 50, "01111111", Some("01111111")).await;
    ledger(&conn, 50, 51, "r2", r#"{"scheme":"GB:coh","loser_id":"03333333"}"#, 100).await;
    // 03333333's real owner (70) keeps the spelling r2 folded into it.
    gb_org(&conn, 70, "03333333", "Owner Ltd", 0).await;
    ledger(&conn, 70, 71, "r2", r#"{"scheme":"GB:coh","loser_id":"GB03333333"}"#, 100).await;

    let rule = |r: &store::MergedIdentifierBackfill, name: &str| {
        r.rules.iter().find(|(n, _)| n == name).map(|(_, c)| c.clone()).expect("rule counted")
    };
    let dry = db.backfill_merged_identifiers(backfill(true)).await.expect("dry");
    let r = db.backfill_merged_identifiers(backfill(false)).await.expect("wet");
    for name in store::MERGED_IDENTIFIER_BACKFILL_RULES {
        assert_eq!(rule(&r, name), rule(&dry, name), "{name}: wet does what dry counted");
    }
    let r2 = rule(&r, "r2");
    assert_eq!((r2.rows, r2.written, r2.wrong_number), (3, 1, 2), "{r2:?}");
    let e0 = rule(&r, "e0");
    assert_eq!((e0.rows, e0.written, e0.same_as_survivor, e0.wrong_number), (1, 0, 0, 1), "{e0:?}");
    assert!(lookup(&conn, "02202746", None, None).await.is_empty(), "through the rekey hop: refused");
    assert!(lookup(&conn, "GB02202746", None, None).await.is_empty(), "the e0 row: refused");
    let by = async |identifier: &str| -> Vec<i64> {
        let f = store::read::Filter { identifier: Some(identifier.into()), ..store::read::Filter::default() };
        store::read::organizations(&conn, &f, store::read::Scope::Page { after: 0, limit: 10 })
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect()
    };
    assert_eq!(by("03333333").await, vec![70], "the owner answers its own number, the moved org does not");
    assert_eq!(by("GB03333333").await, vec![70], "the owner keeps its folded spelling");
    assert_eq!(merged_table(&conn).await, vec![("GB03333333".into(), 70)]);
    drop(db);
    remove_db(&path);
}

/// Review R4 (D4): a ledger row from before a from-archive rebuild names org
/// ids the rebuild re-minted. When the id its keep names was minted again and
/// merged in the new era, both per-hop time checks pass (the deleted hop's own
/// mint time is gone), and the old era's PPON landed on an unrelated company.
/// No live organization is older than the era it was minted in, so a ledger
/// row older than the oldest live organization is refused as `out_of_time`.
#[tokio::test]
async fn the_backfill_refuses_a_ledger_row_older_than_the_org_era() {
    let path = fresh_path("backfill-era");
    let db = store::Db::open(&path).await.expect("open");
    let conn = turso::Builder::new_local(&path).build().await.expect("raw").connect().expect("connect");
    // The new era began at 500: org 10 minted at 550, the new org 20 (another
    // company) minted at 600 and merged into 10 at 700.
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (10, 'GB', 'national', '55556666', 'Unrelated Ltd', 'unrelated ltd', 0, 550)",
        (),
    )
    .await
    .unwrap();
    // The old era: Sellafield's PPON org 30 merged into the then-org 20 at 100.
    ledger(&conn, 20, 30, "e2-altid", r#"{"scheme":"GB:altid","loser_id":"GBPPONPWYP8439MZWY"}"#, 100).await;
    ledger(&conn, 10, 20, "r2", r#"{"scheme":"GB:coh","loser_id":"GB55556666"}"#, 700).await;

    let r = db.backfill_merged_identifiers(backfill(false)).await.expect("wet");
    assert_eq!(r.era_floor, Some(550), "the oldest live organization's mint time");
    let altid = r.rules.iter().find(|(n, _)| n == "e2-altid").map(|(_, c)| c.clone()).unwrap();
    assert_eq!((altid.rows, altid.written, altid.out_of_time), (1, 0, 1), "{altid:?}");
    let by = async |identifier: &str| -> Vec<i64> {
        let f = store::read::Filter { identifier: Some(identifier.into()), ..store::read::Filter::default() };
        store::read::organizations(&conn, &f, store::read::Scope::Page { after: 0, limit: 10 })
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect()
    };
    assert!(by("GBPPONPWYP8439MZWY").await.is_empty(), "the old era's PPON never lands on the new era's org 10");
    assert_eq!(by("GB55556666").await, vec![10], "the new era's own merge is written");
    drop(db);
    remove_db(&path);
}
