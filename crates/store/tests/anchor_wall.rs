//! Issue 318: the two implementations of "the R3 bar" disagree, and this is
//! the measurement that sizes the disagreement before anyone decides how hard
//! to close it.
//!
//! The batch merge arm refuses to corroborate on a name shared by more orgs
//! than the stoplist cap unless the anchor's scheme hard-checksums. The
//! resolver's ingest-time anchor bind applies the same bar minus that wall —
//! and the resolver runs on every notice, without leaving an audit row.

use store::turso::Value;

/// Stand-ins for `idgate`: linguistics stay out of store, so the census takes
/// both as injected fns and a test can pin the boundary exactly.
fn anchors(value: &str) -> Vec<(&'static str, String)> {
    match value {
        "552100554" => vec![("FR:siren", "552100554".into())],
        "5561234567" => vec![("SE:orgnr", "5561234567".into())],
        // Two real schemes: poisoned, and the resolver would not bind either.
        "12345678" => vec![("DK:cvr", "12345678".into()), ("SI:davcna", "12345678".into())],
        // A pipe-scheme is not a real one — it must not be counted, and it
        // must not poison the real one beside it.
        "999" => vec![("FR:siren", "999".into()), ("XX:a|b", "999".into())],
        _ => Vec::new(),
    }
}
fn hard(scheme: &str) -> bool {
    matches!(scheme, "SE:orgnr")
}

struct Fx {
    db: store::Db,
    conn: store::turso::Connection,
}

async fn fixture(name: &str) -> Fx {
    let path = format!("test-anchor-wall-{name}.db");
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    Fx { db, conn }
}

impl Fx {
    async fn org(&self, id: i64, name: &str, identifier: Option<&str>) {
        self.conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, NULL, 'national', ?, ?, ?, 0, 0)",
                (
                    Value::Integer(id),
                    match identifier {
                        Some(v) => Value::Text(v.into()),
                        None => Value::Null,
                    },
                    Value::Text(name.into()),
                    Value::Text(name.to_lowercase()),
                ),
            )
            .await
            .unwrap();
    }
    async fn key(&self, org: i64, key: &str) {
        self.conn
            .execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
                (Value::Integer(org), Value::Text(key.into())),
            )
            .await
            .unwrap();
    }
}

/// A page far larger than any fixture here, so paging never bites unless a test
/// asks it to (issue 333).
const WIDE: usize = 10_000;

fn never() -> bool {
    false
}

/// The whole classification in one fixture, at cap 2: a generic key holding a
/// soft-anchored org (the gap), a hard-anchored one (the design's exemption,
/// where both paths already agree), a poisoned one, an unidentified one, and
/// a NON-generic key whose soft-anchored org must not be counted at all.
#[tokio::test]
async fn only_soft_anchored_orgs_on_an_over_cap_key_count_as_the_gap() {
    let fx = fixture("classify").await;
    // Generic: 4 orgs > cap 2.
    fx.org(1, "Tribunal Administratif", Some("552100554")).await; // FR:siren, SOFT
    fx.org(2, "Tribunal Administratif", Some("5561234567")).await; // SE:orgnr, HARD
    fx.org(3, "Tribunal Administratif", Some("12345678")).await; // poisoned: 2 schemes
    fx.org(4, "Tribunal Administratif", None).await; // no identifier
    for o in 1..=4 {
        fx.key(o, "tribunal administratif").await;
    }
    // Not generic: 2 orgs == cap, so the wall never applies here.
    fx.org(5, "Acme SARL", Some("552100554")).await;
    fx.org(6, "Acme SARL", Some("552100554")).await;
    for o in 5..=6 {
        fx.key(o, "acme sarl").await;
    }

    let r = fx.db.anchor_wall_census(anchors, hard, 2, 200, WIDE, &never).await.unwrap();
    assert_eq!(r.keys_walked, 2);
    assert_eq!(r.generic_keys, 1, "only the 4-carrier key is over the cap");
    assert_eq!(r.generic_orgs, 4, "carrier slots on the one generic key");
    assert_eq!(r.probed, 4, "the non-generic key's orgs are never probed");
    assert_eq!(r.anchored, 2, "org 3 is poisoned by a second scheme, org 4 has no identifier");
    assert_eq!(r.anchored_hard, 1);
    assert_eq!(r.anchored_soft, 1, "org 1 alone: ingest would bind it, batch would refuse");
    assert_eq!(r.by_scheme, vec![("FR:siren".to_owned(), 1)]);
    assert_eq!(r.rows.len(), 1);
    assert_eq!((r.rows[0].org, r.rows[0].carriers), (1, 4));
    assert_eq!(r.rows[0].key, "tribunal administratif");
}

/// The census must measure the rule the RESOLVER runs, not a reasonable-
/// sounding variant of it. The resolver's test is
/// `real.len() == 1 && anchors.len() == real.len()` (canonical.rs:6041): a
/// pipe-scheme is not a real anchor AND its mere presence disqualifies the
/// bind. So a value carrying one alongside a real scheme is NOT reachable,
/// and counting it would inflate the gap with rows ingest never binds.
///
/// This test exists because the first draft asserted the opposite — that the
/// pipe-scheme would be filtered away and the real one would stand — and the
/// code disagreed. The code is right and the assumption was wrong.
#[tokio::test]
async fn a_pipe_scheme_beside_a_real_one_disqualifies_the_bind() {
    let fx = fixture("pipe").await;
    for o in 1..=3 {
        fx.org(o, "European Commission", Some("999")).await;
        fx.key(o, "european commission").await;
    }
    let r = fx.db.anchor_wall_census(anchors, hard, 2, 200, WIDE, &never).await.unwrap();
    assert_eq!(r.generic_keys, 1, "the key is generic either way");
    assert_eq!(r.probed, 3);
    assert_eq!(
        (r.anchored, r.anchored_soft),
        (0, 0),
        "the resolver would not bind these, so the census must not count them"
    );
    assert!(r.rows.is_empty());
}

/// Very large groups are sampled. The carrier COUNT stays whole — it is what
/// makes the name generic — while the per-org probes are capped, so one
/// vendor's boilerplate cannot dominate the measurement it appears in.
#[tokio::test]
async fn a_huge_group_is_sampled_but_its_carrier_count_is_not() {
    let fx = fixture("sampled").await;
    for o in 1..=50 {
        fx.org(o, "Vendor Boilerplate", Some("552100554")).await;
        fx.key(o, "vendor boilerplate").await;
    }
    let r = fx.db.anchor_wall_census(anchors, hard, 2, 10, WIDE, &never).await.unwrap();
    assert_eq!(r.generic_orgs, 50, "every carrier counts toward genericness");
    assert_eq!(r.probed, 10, "but only the sample is probed");
    assert_eq!(r.anchored_soft, 10);
    assert_eq!(r.rows[0].carriers, 50, "and the row reports the WHOLE group, not the sample");
}

/// A cancelled census stores nothing. An undercount of a disagreement would
/// read as agreement, which is the one wrong answer this can give.
#[tokio::test]
async fn a_cancelled_census_reports_nothing_rather_than_less() {
    let fx = fixture("stop").await;
    for o in 1..=3 {
        fx.org(o, "Tribunal Administratif", Some("552100554")).await;
        fx.key(o, "tribunal administratif").await;
    }
    let always = || true;
    let r = fx.db.anchor_wall_census(anchors, hard, 2, 200, WIDE, &always).await.unwrap();
    assert!(r.stopped);
    assert_eq!((r.generic_keys, r.anchored_soft, r.keys_walked), (0, 0, 0));
    assert!(r.rows.is_empty());
}

/// An org standing on SEVERAL generic keys is one row to fix, not several.
/// The quoted number must be distinct orgs; the slot count is reported beside
/// it so the difference is visible rather than silently folded away.
#[tokio::test]
async fn an_org_on_two_generic_keys_counts_once() {
    let fx = fixture("distinct").await;
    // Two generic keys (3 carriers each at cap 2), sharing org 1.
    for o in 1..=3 {
        fx.org(o, "Shared Name", Some("552100554")).await;
        fx.key(o, "key one").await;
    }
    for o in 4..=5 {
        fx.org(o, "Other Name", Some("552100554")).await;
    }
    for o in [1i64, 4, 5] {
        fx.key(o, "key two").await;
    }

    let r = fx.db.anchor_wall_census(anchors, hard, 2, 200, WIDE, &never).await.unwrap();
    assert_eq!(r.generic_keys, 2);
    assert_eq!(r.generic_orgs, 6, "6 carrier SLOTS across the two keys");
    assert_eq!(r.probed, 6);
    assert_eq!(r.anchored_soft, 5, "5 DISTINCT orgs — org 1 is on both keys");
    assert_eq!(r.soft_slots, 6, "…reachable by 6 (org, generic-name) pairs");
    assert_eq!(r.anchored, 5);
    assert_eq!(r.by_scheme, vec![("FR:siren".to_owned(), 5)]);
    assert_eq!(r.rows.len(), 5, "and org 1 is listed once, not twice");
}

/// Issue 333: a key run longer than one page must still be counted whole.
///
/// The previous walk read a page of ROWS, trimmed the trailing possibly-split
/// run and resumed from its key — but the trim was guarded on `groups.len() > 1`,
/// so a page filled by ONE key kept the truncated run and resumed past it,
/// losing every carrier beyond the page. That hides the WIDEST keys, which are
/// the whole point of a genericness census.
///
/// Latent rather than live when found: the widest key measured on prod is 1,238
/// carriers against a page of 20,000. Pinned here so it stays that way.
#[tokio::test]
async fn a_key_run_longer_than_a_page_is_counted_whole() {
    let fx = fixture("issue-333").await;
    // Eight carriers of one key, driven with a page of three.
    for n in 1..=8i64 {
        fx.org(n, "Gemeinsamer Name", Some("552100554")).await;
        fx.key(n, "gemeinsamer name").await;
    }
    let r = fx.db.anchor_wall_census(anchors, hard, 2, 200, 3, &never).await.unwrap();
    assert_eq!(r.generic_keys, 1, "one key, not several short ones");
    assert_eq!(r.generic_orgs, 8, "all eight carriers, not just the first page");
}
