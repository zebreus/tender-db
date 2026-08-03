//! Issue 117 Class B: the `/v1/tenders` existence short-circuit must change SPEED
//! and never RESULTS.
//!
//! The guard answers "can any Tender satisfy this filter at all?" with one index
//! seek, so a value matching nothing returns an empty page instead of walking 4.26M
//! Tenders (>380s on prod, unauthenticated, on a documented filter).
//!
//! The hazard is entirely one-sided. A guard that is too GENEROUS costs a walk we
//! could have avoided — slow, correct. A guard that is too NARROW returns an empty
//! page for a filter that does match — fast, WRONG, and silent. So every case here
//! that could go either way is written to catch the narrow direction.
//!
//! The lowercase case below is not hypothetical: the first version of this guard had
//! exactly that bug. `LIKE` folds ASCII case and `>=`/`<` do not, so with `DE300`
//! stored, `LIKE 'de%'` matches while `code >= 'de' AND code < 'df'` does not — and
//! `?country=de` would have returned an empty page while the real query returns rows.

use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

/// One Tender at seq 2, with a superseded seq 1 — so a value present ONLY in the old
/// version is available as a case the guard must not short-circuit on.
async fn seed(conn: &turso::Connection) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (1, 'ted', 'pk-1', 'procedure', 2, 1700000000, 1700000000)",
        (),
    ).await.unwrap();
    for seq in 1..=2 {
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (1, ?, 1700000000, ?, ?)",
            (Value::Integer(seq), Value::Text(format!("pub-{seq}")), Value::Integer(seq)),
        ).await.unwrap();
    }
    for (seq, scheme, code) in [
        (2i64, "nuts", "DE300"),
        (2, "cpv", "45210000"),
        // Present ONLY in the superseded version: the current-version query must not
        // return the Tender for it, and the guard must not be what decides that.
        (1, "nuts", "FR101"),
    ] {
        conn.execute(
            "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
             VALUES (1, ?, NULL, 'place', ?, ?)",
            (Value::Integer(seq), Value::Text(scheme.into()), Value::Text(code.into())),
        ).await.unwrap();
    }
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (7, 'DE', 'vat', 'X', 'Buyer Ltd', 0, 1700000000)",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties
             (tender_id, seq, lot_id, role, organization_id, mention_notice_id,
              mention_section_id)
         VALUES (1, 2, NULL, 'Procedure-Buyer', 7, 1, 'S1')",
        (),
    ).await.unwrap();
}

async fn ids(conn: &turso::Connection, f: &Filter) -> Vec<i64> {
    read::tenders(conn, f, Scope::Page { after: 0, limit: 100 })
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

/// The invariant the guard rests on, asserted against TURSO rather than remembered
/// from a scratch run against another engine.
///
/// `prefix_ranges` claims the union of a prefix's ASCII-case variants covers exactly
/// what `LIKE prefix || '%'` matches. That holds only if the engine folds AT MOST
/// ASCII case in `LIKE` — if it folded Unicode too, the union would be narrower than
/// the predicate and the guard would drop rows. sqlite3 and turso are not
/// interchangeable for questions like this (issue 112's wrong-engine discipline), and
/// the deployed engine is the one whose answer counts.
#[tokio::test]
async fn turso_folds_ascii_case_in_like_and_no_more() {
    let path = format!("/tmp/tender-db-fold-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    for (id, code) in [(1i64, "DE300"), (2, "\u{130}STANBUL")] {
        conn.execute(
            "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
             VALUES (?, 1, NULL, 'place', 'nuts', ?)",
            (Value::Integer(id), Value::Text(code.to_owned())),
        )
        .await
        .unwrap();
    }
    let matches = |pat: &str| {
        let pat = pat.to_owned();
        let conn = &conn;
        async move {
            let mut rows = conn
                .query(
                    "SELECT tender_id FROM tender_version_classifications
                      WHERE scheme = 'nuts' AND code LIKE ? ORDER BY tender_id",
                    (Value::Text(pat),),
                )
                .await
                .unwrap();
            let mut out = Vec::new();
            while let Some(r) = rows.next().await.unwrap() {
                out.push(r.get_value(0).unwrap().as_integer().copied().unwrap());
            }
            out
        }
    };

    // ASCII case IS folded — both directions. This is why one range over the prefix
    // as given is not enough, and why the variants exist at all.
    assert_eq!(matches("DE%").await, vec![1]);
    assert_eq!(matches("de%").await, vec![1], "turso folds ASCII case in LIKE");
    assert_eq!(matches("dE%").await, vec![1]);

    // Non-ASCII is NOT folded: the dotted capital I does not match its lowercase
    // form. If this ever fails, turso has grown Unicode folding and the ASCII-variant
    // union is no longer a superset of LIKE — `prefix_ranges` would then have to
    // decline any prefix that could case-fold outside ASCII, not merely non-ASCII
    // ones.
    assert!(
        matches("i%").await.is_empty(),
        "turso must not fold non-ASCII case in LIKE, or the ASCII-variant union in \
         prefix_ranges is narrower than the predicate it stands in for"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn the_guard_changes_speed_not_results() {
    let path = format!("/tmp/tender-db-scguard-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    seed(&conn).await;

    let country = |c: &str| Filter { country: Some(c.to_owned()), ..Filter::default() };
    let cpv = |c: &str| Filter { cpv: Some(c.to_owned()), ..Filter::default() };

    // Present, exactly as stored.
    assert_eq!(ids(&conn, &country("DE")).await, vec![1], "DE must match");
    assert_eq!(ids(&conn, &country("DE300")).await, vec![1], "the full code must match");
    assert_eq!(ids(&conn, &cpv("45")).await, vec![1], "cpv prefix must match");

    // THE REGRESSION CASE. `LIKE 'de%'` matches the stored `DE300`, so the guard must
    // not veto it. The first version of this guard did.
    assert_eq!(
        ids(&conn, &country("de")).await,
        vec![1],
        "a lowercase country prefix must match a stored uppercase code, because the \
         predicate it stands in for is `LIKE`, which folds ASCII case — a guard \
         narrower than its predicate returns wrong rows, not slow ones"
    );
    assert_eq!(ids(&conn, &country("dE300")).await, vec![1], "mixed case likewise");

    // LIKE METACHARACTERS. The bound predicate is `LIKE prefix || '%'`, so `%` and `_`
    // are wildcards in the prefix and a range comparison is not a pattern match. These
    // are the same family as the case-fold bug: a guard narrower than its predicate.
    // `?country=%` is the sharpest — `LIKE '%%'` matches EVERY code, so a guard that
    // vetoed it would return an empty page for the filter that matches everything.
    // Nothing upstream validates these: `Params` passes country/cpv through verbatim.
    assert_eq!(
        ids(&conn, &country("%")).await,
        vec![1],
        "`%` is a LIKE wildcard matching everything — the guard must decline, not veto"
    );
    assert_eq!(
        ids(&conn, &country("_E")).await,
        vec![1],
        "`_` matches any single character, so `_E%` matches the stored DE300"
    );
    assert_eq!(ids(&conn, &cpv("%")).await, vec![1], "same for cpv");
    assert_eq!(
        ids(&conn, &country("D%0")).await,
        vec![1],
        "a metacharacter anywhere in the prefix, not just at the start"
    );

    // Absent: the whole point. Empty, and reached without the walk.
    assert!(ids(&conn, &country("ZZ")).await.is_empty(), "absent country -> empty");
    assert!(ids(&conn, &cpv("99")).await.is_empty(), "absent cpv -> empty");

    // Present only in a SUPERSEDED version: the guard ignores `seq`, so it falls
    // through and the full query returns empty.
    //
    // Note what this does NOT prove. Adding the seq correlation to the guard also
    // passes here — verified by mutation — because such a guard would be equivalent
    // in scope to the `EXISTS` it stands in for, merely costlier (it needs a join).
    // So this case documents the conservative design and pins the ANSWER; it is not a
    // falsifier for a seq-aware variant, and there is nothing for it to falsify.
    // Recorded rather than implied, because a test comment claiming coverage it lacks
    // is how a suite stops meaning what it says.
    assert!(
        ids(&conn, &country("FR")).await.is_empty(),
        "a value only in a superseded version yields no current Tender"
    );

    // Buyer: present and absent.
    assert_eq!(
        ids(&conn, &Filter { buyer: Some(7), ..Filter::default() }).await,
        vec![1],
        "the buyer organization matches"
    );
    assert!(
        ids(&conn, &Filter { buyer: Some(4242), ..Filter::default() }).await.is_empty(),
        "an organization in no party row -> empty"
    );
    // Winner: the org exists as a PARTY but never as a winner, so the winner probe
    // must veto on its own table rather than on organizations at large.
    assert!(
        ids(&conn, &Filter { winner: Some(7), ..Filter::default() }).await.is_empty(),
        "an organization that never won -> empty"
    );

    // Combined filters: one absent leg is enough to empty the page, and a present one
    // must not rescue it.
    assert!(
        ids(&conn, &Filter {
            country: Some("DE".into()),
            cpv: Some("99".into()),
            ..Filter::default()
        })
        .await
        .is_empty(),
        "a present country must not rescue an absent cpv"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
