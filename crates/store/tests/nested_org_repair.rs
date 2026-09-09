//! Issue 259 landing repair: the stale pre-fix mention layer — an award's
//! winner bound to a nameless provisional minted from the outer wrapper of a
//! nested Organization pair while the name sits on the inner section's org —
//! is repaired in place: references repoint to the named inner org, the empty
//! row is deleted, the change feed hears about all of it.

use store::turso::Value;

async fn seed(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute("BEGIN", ()).await.unwrap();

    // Organizations: 10 = the nameless outer wrapper (the repair target),
    // 11 = the named inner party, 13 = nameless with TWO mentions (guard),
    // 14 = nameless whose only child is also nameless (guard), 15 = that child,
    // 20/21 = the outer and middle of a THREE-level sdk-0.1 nest, 22 = its
    // named innermost party.
    for (id, name, provisional) in [
        (10, "", 1),
        (11, "Opal Publicidade, S. A.", 1),
        (13, "", 1),
        (14, "", 1),
        (15, "", 1),
        (20, "", 1),
        (21, "", 1),
        (22, "Drei Ebenen GmbH", 1),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
                Value::Integer(provisional),
            ),
        )
        .await
        .unwrap();
    }

    // The exemplar topology (notice 100): RES-1 > ORG-2 > ORG-3.
    for (nid, sid, kind, parent) in [
        (100, "RES-1", "LotResult", None),
        (100, "ORG-2", "Organization", Some("RES-1")),
        (100, "ORG-3", "Organization", Some("ORG-2")),
        // Guard case: OG-1 > OG-2, both orgs nameless.
        (103, "OG-1", "Organization", None),
        (103, "OG-2", "Organization", Some("OG-1")),
        // A THREE-level sdk-0.1 nest with a non-party section in the middle of
        // the chain: P1 > MID > X > INN. Both P1's and MID's nameless orgs
        // must land on INN's named org — depth and the non-party intermediate
        // must not matter (the `nested_org_aliases` semantics).
        (105, "P1", "WinningParty", None),
        (105, "MID", "WinningParty", Some("P1")),
        (105, "X", "Address", Some("MID")),
        (105, "INN", "WinningParty", Some("X")),
    ] {
        conn.execute(
            "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
             VALUES (?, ?, ?, ?)",
            (
                Value::Integer(nid),
                Value::Text(sid.into()),
                Value::Text(kind.into()),
                match parent {
                    Some(p) => Value::Text(p.into()),
                    None => Value::Null,
                },
            ),
        )
        .await
        .unwrap();
    }

    for (nid, sid, org) in [
        (100, "ORG-2", 10),
        (100, "ORG-3", 11),
        // Org 13 carries two mentions — not the single-wrapper shape.
        (101, "A", 13),
        (102, "B", 13),
        // Org 14's only child mention names a nameless org.
        (103, "OG-1", 14),
        (103, "OG-2", 15),
        // The three-level nest's mentions.
        (105, "P1", 20),
        (105, "MID", 21),
        (105, "INN", 22),
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id)
             VALUES (?, ?, ?)",
            (Value::Integer(nid), Value::Text(sid.into()), Value::Integer(org)),
        )
        .await
        .unwrap();
    }

    // Satellite rows (ADR-0013 D4): the loser carries a language the keep
    // lacks (moves over) and one the keep has (keep's wins).
    for (org, lang, name) in [
        (10, "NLD", "Stad Brussel"),
        (10, "POR", "loser variant that must lose"),
        (11, "POR", "Opal Publicidade, S. A."),
    ] {
        conn.execute(
            "INSERT INTO organization_names (org_id, lang, name, name_norm)
             VALUES (?, ?, ?, ?)",
            (
                Value::Integer(org),
                Value::Text(lang.into()),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }

    // The pre-fix doubled award: both ends of the nest stand on one lot_result.
    for org in [10, 11] {
        conn.execute(
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
             VALUES (500, 1, 900, ?)",
            (Value::Integer(org),),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (500, 1, 'winner', 10, 100, 'ORG-2')",
        (),
    )
    .await
    .unwrap();
    conn.execute("COMMIT", ()).await.unwrap();
    (db, conn)
}

#[tokio::test]
async fn the_nested_org_repair_repoints_dedups_and_deletes_with_guards() {
    let path = format!("/tmp/tender-db-nestedorg-{}.db", std::process::id());
    let (db, conn) = seed(&path).await;

    let walk = |dry_run: bool, batch_size: i64| {
        let db = &db;
        async move {
            let mut totals = store::NestedOrgRepair::default();
            let mut watermark = 0i64;
            loop {
                let (batch, next) = db
                    .repair_nested_org_mentions_batch(batch_size, watermark, dry_run)
                    .await
                    .expect("batch");
                if batch.scanned == 0 {
                    break;
                }
                totals.scanned += batch.scanned;
                totals.repaired += batch.repaired;
                totals.skipped += batch.skipped;
                totals.winner_dups += batch.winner_dups;
                totals.tender_changes += batch.tender_changes;
                watermark = next;
            }
            totals
        }
    };

    // The dry run previews the same numbers and writes NOTHING.
    let preview = walk(true, 2).await;
    assert_eq!(
        (preview.scanned, preview.repaired, preview.skipped, preview.winner_dups),
        (6, 3, 3, 1),
        "the preview predicts the repair"
    );
    {
        let mut rows = conn
            .query("SELECT COUNT(*) FROM organizations WHERE id IN (10, 20, 21)", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get_value(0).unwrap(), Value::Integer(3), "the dry run deleted nothing");
    }

    // batch=2 forces the watermark loop to iterate.
    let totals = walk(false, 2).await;
    assert_eq!(totals.scanned, 6, "the six nameless provisionals are visited");
    assert_eq!(
        totals.repaired, 3,
        "the exemplar pair and both levels of the three-level nest repair"
    );
    assert_eq!(totals.skipped, 3, "multi-mention (13), nameless child (14), childless (15)");
    assert_eq!(totals.winner_dups, 1, "the doubled award collapses");
    assert_eq!(totals.tender_changes, 1, "tender 500 is told");

    // The winner is the named party, exactly once.
    let mut rows = conn
        .query(
            "SELECT organization_id FROM tender_version_result_winners
              WHERE tender_id = 500 AND seq = 1 AND lot_result_id = 900",
            (),
        )
        .await
        .unwrap();
    let mut winners = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        winners.push(match row.get_value(0).unwrap() {
            Value::Integer(i) => i,
            other => panic!("unexpected {other:?}"),
        });
    }
    drop(rows);
    assert_eq!(winners, vec![11], "one winner, the named org");

    // The empty row is gone; the guarded rows survive; references repointed.
    let count = |sql: &'static str| {
        let conn = &conn;
        async move {
            let mut rows = conn.query(sql, ()).await.unwrap();
            match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
                Value::Integer(i) => i,
                other => panic!("unexpected {other:?}"),
            }
        }
    };
    assert_eq!(count("SELECT COUNT(*) FROM organizations WHERE id IN (10, 20, 21)").await, 0);
    assert_eq!(count("SELECT COUNT(*) FROM organizations WHERE id IN (13, 14, 15)").await, 3);
    assert_eq!(
        count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 11").await,
        2,
        "the outer mention now stands on the named org beside the inner one"
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 22").await,
        3,
        "both nest levels' mentions landed on the innermost named org"
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM tender_version_parties WHERE organization_id = 11").await,
        1,
        "the party row repointed"
    );

    // The satellite moved with the repoint: the loser's unique language rides
    // over, a language collision keeps the keep's row, no loser rows remain.
    assert_eq!(count("SELECT COUNT(*) FROM organization_names WHERE org_id = 10").await, 0);
    assert_eq!(
        count("SELECT COUNT(*) FROM organization_names WHERE org_id = 11 AND lang = 'NLD' AND name = 'Stad Brussel'").await,
        1
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM organization_names WHERE org_id = 11 AND lang = 'POR' AND name = 'Opal Publicidade, S. A.'").await,
        1,
        "the keep's existing variant wins the collision"
    );

    // The change feed heard: loser removed, keep changed, tender changed.
    assert_eq!(
        count("SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 10 AND op = 'removed'").await,
        1
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 11 AND op = 'changed'").await,
        1
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id = 500 AND op = 'changed'").await,
        1
    );

    // Idempotent: a second full walk repairs nothing further.
    let second = walk(false, 100).await;
    assert_eq!(second.repaired, 0, "a second walk is a no-op");

    drop(conn);
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 365 unit 5: the nameless class is counted, including the subset that
/// carries no country either.
///
/// This population is invisible to the `org-merge-health` walk by construction —
/// that walk visits identifier-BEARING rows, and `provisional` is exactly
/// `identifier IS NULL` — so the biggest group in the table had no number in the
/// standing weekly report. The class is left to grow by design (issue 234: a
/// nameless mention is a distinct unknown party), which is precisely why it needs
/// to be observed rather than assumed.
#[tokio::test]
async fn the_nameless_provisional_rows_are_counted_with_and_without_a_country() {
    let path = "/tmp/tender-nameless-count.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();

    // Four nameless rows — two with a country, two without — and two that must
    // NOT be counted: a named provisional, and an identifier-bearing row whose
    // name happens to be empty.
    //
    // All the nameless ones use the EMPTY STRING because `organizations.name` is
    // NOT NULL; the first draft of this test tried a NULL name and the schema
    // refused it, which is how the counter's dead `name IS NULL` branch came out.
    for (id, name, country, identifier, provisional) in [
        (100, "", None, None, 1),
        (101, "", Some("DE"), None, 1),
        (102, "", Some("FR"), None, 1),
        (103, "", None, None, 1),
        (104, "Ein Name GmbH", Some("DE"), None, 1),
        (105, "", Some("DE"), Some("DE123456780"), 0),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, name, name_norm, country, identifier, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
                country.map_or(Value::Null, |c| Value::Text(c.into())),
                identifier.map_or(Value::Null, |i| Value::Text(i.into())),
                Value::Integer(provisional),
            ),
        )
        .await
        .unwrap();
    }

    let (nameless, without_country) = db.count_nameless_provisional_orgs().await.unwrap();
    assert_eq!(nameless, 4, "every nameless provisional counts; a named row does not");
    assert_eq!(without_country, 2, "and the no-information subset is separable");

    // The identifier-bearing row is excluded even though its name is empty —
    // it is not provisional, so the resolver can still reach it by key.
    assert_eq!(
        db.scalar("SELECT COUNT(*) FROM organizations WHERE name = '' AND provisional = 0")
            .await
            .unwrap(),
        Some(Value::Integer(1)),
    );
}

/// Issue 365 unit 6: the windowed cohort selector that finds standing rows the
/// unit-4 scheme gate cannot reach.
///
/// The gate refuses a denied scheme at MINT time only, so anything folded before
/// it keeps its key, and no existing path gets there: `repair-placeholder-orgs`
/// asks a value-shaped predicate that never sees a scheme, and `refold-notices`
/// caps explicit ids at 1,000 while `refold`/`refold-fields` are profile- and
/// field-scoped.
#[tokio::test]
async fn the_denied_scheme_cohort_is_selected_by_window_and_case_insensitively() {
    let path = "/tmp/tender-denied-scheme-cohort.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    for (notice_id, section, scheme) in [
        (10, "ORG-1", Some("OTROS")),
        (10, "ORG-2", Some("NIF")),   // same notice, a real scheme too
        (20, "ORG-1", Some("otros")), // lower case, as some publishers send it
        (30, "ORG-1", Some("NIF")),   // never selected
        (40, "ORG-1", None),          // no scheme at all
        (900, "ORG-1", Some("OTROS")), // outside the first window
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, scheme)
             VALUES (?, ?, 1, 'x', ?)",
            (
                Value::Integer(notice_id),
                Value::Text(section.into()),
                scheme.map_or(Value::Null, |s| Value::Text(s.into())),
            ),
        )
        .await
        .unwrap();
    }

    // First window: notices 10 and 20, each ONCE despite notice 10 having two
    // mentions, and `otros` matched case-insensitively because the scheme is
    // stored exactly as the publisher wrote it.
    let first = db.notices_with_denied_scheme(&["OTROS"], 0, 100).await.unwrap();
    assert_eq!(first, vec![10, 20]);

    // The next window picks up what the first deliberately did not reach — the
    // property that makes a stride walk safe to resume.
    let second = db.notices_with_denied_scheme(&["OTROS"], 100, 1000).await.unwrap();
    assert_eq!(second, vec![900]);

    // An empty denial list selects nothing rather than everything, which is the
    // failure mode that would re-fold the entire corpus by accident.
    assert!(db.notices_with_denied_scheme(&[], 0, 1000).await.unwrap().is_empty());

    // And a scheme that is not denied is never swept in.
    assert!(
        db.notices_with_denied_scheme(&["OTROS"], 0, 1000)
            .await
            .unwrap()
            .iter()
            .all(|id| *id != 30 && *id != 40)
    );
}
