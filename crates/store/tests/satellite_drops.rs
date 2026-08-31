//! Issue 321's repair: drop the name variants a re-homing left on the ORIGIN
//! that already stand on the row it re-homed to, and be able to put every one
//! of them back.
//!
//! The variant is real data a resolver wrote from a real publication. Issue
//! 312 is the standing record of what an apply pass costs when it outruns the
//! evidence for it, so the pre-image and the restore are not optional extras
//! here — they are the reason the drop is allowed to exist.

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

struct Fx {
    db: store::Db,
    conn: store::turso::Connection,
}

async fn fixture(name: &str) -> Fx {
    let path = format!("test-satellite-drops-{name}.db");
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
    async fn org(&self, id: i64, name: &str) {
        self.conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, 'DE', NULL, NULL, ?, ?, 0, 0)",
                (Value::Integer(id), Value::Text(name.into()), Value::Text(name.to_lowercase())),
            )
            .await
            .unwrap();
    }
    async fn variant(&self, id: i64, lang: &str, name: &str) {
        self.conn
            .execute(
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
    async fn mention(&self, notice: i64, on: i64, name: &str) {
        self.conn
            .execute(
                "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
                 VALUES (?, 'ORG-1', ?, ?, 'DE', NULL)",
                (Value::Integer(notice), Value::Integer(on), Value::Text(name.into())),
            )
            .await
            .unwrap();
    }
    async fn applied(&self, org_id: i64, notice: i64, target: i64) {
        self.conn
            .execute(
                "INSERT INTO org_mention_rehoming
                   (case_org_id, notice_id, section_id, cohort, action, target_org_id, target_name,
                    rationale, confidence, reviewed_at, applied_at, applied_action, job_id)
                 VALUES (?, ?, 'ORG-1', 'c', 'rehome', ?, 'n', 'r', 'high', 1, 2, 'rehomed from ' || ?, 1)",
                (
                    Value::Integer(org_id),
                    Value::Integer(notice),
                    Value::Integer(target),
                    Value::Integer(org_id),
                ),
            )
            .await
            .unwrap();
    }
    async fn variants_of(&self, id: i64) -> Vec<(String, String)> {
        let mut rows = self
            .conn
            .query(
                "SELECT lang, name FROM organization_names WHERE org_id = ? ORDER BY lang",
                (Value::Integer(id),),
            )
            .await
            .unwrap();
        let mut got = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            let (Value::Text(l), Value::Text(n)) =
                (row.get_value(0).unwrap(), row.get_value(1).unwrap())
            else {
                panic!("text")
            };
            got.push((l, n));
        }
        got
    }
    /// The standard shape: a vehicle keeping its member's name, a member that
    /// has it, an unsupported variant nobody else carries, and a supported one.
    async fn standard(&self) {
        self.org(1, "Bietergemeinschaft Dobler / Oberall").await;
        self.org(2, "Dobler GmbH").await;
        self.variant(1, "DEU", "Dobler GmbH").await; // drops: target 2 has it
        self.variant(1, "FRA", "Groupement Dobler SARL").await; // stays: nowhere else
        self.variant(1, "ENG", "Consortium Dobler / Oberall").await; // stays: supported
        self.mention(900, 1, "Consortium Dobler / Oberall").await;
        self.mention(901, 2, "Dobler GmbH").await;
        self.applied(1, 901, 2).await;
    }
}

fn never() -> bool {
    false
}

#[tokio::test]
async fn the_wet_pass_drops_only_the_variant_the_destination_already_carries() {
    let fx = fixture("wet").await;
    fx.standard().await;
    let stop = never;

    let dry = fx.db.drop_orphan_satellites(norm, true, None, None, 10, &stop).await.unwrap();
    assert_eq!(dry.candidates, 1, "FRA is an orphan but no destination carries it");
    assert_eq!(dry.dropped, 0, "a dry run writes nothing");
    assert_eq!(fx.variants_of(1).await.len(), 3);

    let plan: Vec<(i64, String, String)> =
        dry.rows.iter().map(|o| (o.org, o.lang.clone(), o.key.clone())).collect();
    let wet =
        fx.db.drop_orphan_satellites(norm, false, Some(&plan), Some(7), 20, &stop).await.unwrap();
    assert!(!wet.drifted);
    assert_eq!((wet.candidates, wet.dropped, wet.skipped_recheck), (1, 1, 0));
    assert_eq!(
        fx.variants_of(1).await,
        vec![
            ("ENG".to_owned(), "Consortium Dobler / Oberall".to_owned()),
            ("FRA".to_owned(), "Groupement Dobler SARL".to_owned()),
        ],
        "only the DEU variant went"
    );
    // The destination is untouched — the name is not gone from the corpus.
    assert_eq!(
        fx.conn
            .query("SELECT COUNT(*) FROM organizations WHERE id = 2 AND name = 'Dobler GmbH'", ())
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .map(|r| r.get_value(0).unwrap()),
        Some(Value::Integer(1))
    );
    // And the pre-image is byte-exact, with the reason beside it.
    let mut rows = fx
        .conn
        .query("SELECT org_id, lang, name, key, target_org, job_id FROM org_name_drops", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("one pre-image");
    assert_eq!(row.get_value(0).unwrap(), Value::Integer(1));
    assert_eq!(row.get_value(1).unwrap(), Value::Text("DEU".into()));
    assert_eq!(row.get_value(2).unwrap(), Value::Text("Dobler GmbH".into()));
    assert_eq!(row.get_value(3).unwrap(), Value::Text("dobler gmbh".into()));
    assert_eq!(row.get_value(4).unwrap(), Value::Integer(2));
    assert_eq!(row.get_value(5).unwrap(), Value::Integer(7));
    assert!(rows.next().await.unwrap().is_none());
}

#[tokio::test]
async fn a_drop_is_undone_exactly_and_only_once() {
    let fx = fixture("restore").await;
    fx.standard().await;
    let stop = never;
    let dry = fx.db.drop_orphan_satellites(norm, true, None, None, 10, &stop).await.unwrap();
    let plan: Vec<(i64, String, String)> =
        dry.rows.iter().map(|o| (o.org, o.lang.clone(), o.key.clone())).collect();
    fx.db.drop_orphan_satellites(norm, false, Some(&plan), Some(7), 20, &stop).await.unwrap();

    let dry = fx.db.restore_dropped_satellites(true, None, 30).await.unwrap();
    assert_eq!((dry.outstanding, dry.restored, dry.occupied), (1, 0, 0));
    assert_eq!(dry.rows.len(), 1, "a dry restore names the row without writing it");
    assert_eq!(fx.variants_of(1).await.len(), 2);

    let wet = fx.db.restore_dropped_satellites(false, None, 30).await.unwrap();
    assert_eq!((wet.outstanding, wet.restored, wet.occupied), (1, 1, 0));
    assert_eq!(
        fx.variants_of(1).await,
        vec![
            ("DEU".to_owned(), "Dobler GmbH".to_owned()),
            ("ENG".to_owned(), "Consortium Dobler / Oberall".to_owned()),
            ("FRA".to_owned(), "Groupement Dobler SARL".to_owned()),
        ],
        "byte-exact, back where it was"
    );
    // Stamped, so a second pass has nothing to do rather than doing it twice.
    let again = fx.db.restore_dropped_satellites(false, None, 40).await.unwrap();
    assert_eq!((again.outstanding, again.restored), (0, 0));
}

#[tokio::test]
async fn a_restore_never_clobbers_what_was_written_since() {
    let fx = fixture("occupied").await;
    fx.standard().await;
    let stop = never;
    let dry = fx.db.drop_orphan_satellites(norm, true, None, None, 10, &stop).await.unwrap();
    let plan: Vec<(i64, String, String)> =
        dry.rows.iter().map(|o| (o.org, o.lang.clone(), o.key.clone())).collect();
    fx.db.drop_orphan_satellites(norm, false, Some(&plan), Some(7), 20, &stop).await.unwrap();
    // Something wrote that slot after the drop — a resolver seeing a new
    // publication, say. Newer truth wins.
    fx.variant(1, "DEU", "Bietergemeinschaft Dobler / Oberall").await;

    let r = fx.db.restore_dropped_satellites(false, None, 30).await.unwrap();
    assert_eq!((r.outstanding, r.restored, r.occupied), (1, 0, 1));
    assert_eq!(
        fx.variants_of(1).await[0].1,
        "Bietergemeinschaft Dobler / Oberall",
        "the newer row stands"
    );
}

#[tokio::test]
async fn a_plan_that_drifted_stops_the_wet_pass() {
    let fx = fixture("drift").await;
    fx.standard().await;
    let stop = never;
    // A plan naming a tuple that is not in the fresh set, and missing the one
    // that is — the shape a count could never show.
    let stale: Vec<(i64, String, String)> =
        vec![(1, "ITA".to_owned(), "qualcosa".to_owned())];
    let r = fx.db.drop_orphan_satellites(norm, false, Some(&stale), Some(7), 20, &stop).await.unwrap();
    assert!(r.drifted);
    assert_eq!(r.plan_added, vec!["1/DEU/dobler gmbh".to_owned()]);
    assert_eq!(r.plan_removed, vec!["1/ITA/qualcosa".to_owned()]);
    assert_eq!(r.dropped, 0, "nothing is written when the plan and the world disagree");
    assert_eq!(fx.variants_of(1).await.len(), 3);
}

/// A destination renamed after the plan was taken must not cost the corpus
/// its only copy of a name. Two mechanisms could stop that, and it matters
/// which one does: the tuple parity is computed from the FRESH candidate set,
/// and a candidate whose destination no longer carries the key is not a
/// candidate at all — so the plan's tuple goes missing and the pass refuses
/// before it opens a transaction.
///
/// The in-transaction re-check behind it covers what parity cannot: a writer
/// landing between the read and the write. That window needs a concurrent
/// writer to reach, so no test here forces it; what these tests do show is
/// that the re-check runs and agrees on every path that writes
/// (`skipped_recheck == 0` above), so it is live code rather than a comment.
#[tokio::test]
async fn a_destination_renamed_after_the_plan_stops_the_pass() {
    let fx = fixture("recheck").await;
    fx.standard().await;
    let stop = never;
    let dry = fx.db.drop_orphan_satellites(norm, true, None, None, 10, &stop).await.unwrap();
    let plan: Vec<(i64, String, String)> =
        dry.rows.iter().map(|o| (o.org, o.lang.clone(), o.key.clone())).collect();
    assert_eq!(plan.len(), 1);

    fx.conn
        .execute("UPDATE organizations SET name = 'Dobler Holding AG' WHERE id = 2", ())
        .await
        .unwrap();

    let wet =
        fx.db.drop_orphan_satellites(norm, false, Some(&plan), Some(7), 20, &stop).await.unwrap();
    assert!(wet.drifted, "the candidate lost its destination, so the plan no longer matches");
    assert_eq!(wet.plan_removed, vec!["1/DEU/dobler gmbh".to_owned()]);
    assert!(wet.plan_added.is_empty());
    assert_eq!(wet.dropped, 0);
    assert_eq!(fx.variants_of(1).await.len(), 3, "the corpus keeps its only copy");
    assert_eq!(
        fx.conn
            .query("SELECT COUNT(*) FROM org_name_drops", ())
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .map(|r| r.get_value(0).unwrap()),
        Some(Value::Integer(0)),
        "and no pre-image is written for a row it did not touch"
    );
}

/// A slot dropped, restored and dropped again has two outstanding pre-images.
/// Scan order must not pick which one comes back — the newest is the truth,
/// and the older one is left outstanding rather than overwriting it.
#[tokio::test]
async fn the_newest_pre_image_wins_a_slot_with_two_outstanding_drops() {
    let fx = fixture("newest").await;
    fx.standard().await;
    let stop = never;
    // Two drop rows for (1, DEU) with different pre-images, oldest first —
    // insert order deliberately opposite to the answer.
    for (at, name) in [(10i64, "Dobler GmbH (old spelling)"), (20, "Dobler GmbH")] {
        fx.conn
            .execute(
                "INSERT INTO org_name_drops (org_id, lang, name, name_norm, key, target_org, dropped_at, job_id)
                 VALUES (1, 'DEU', ?, ?, 'dobler gmbh', 2, ?, 7)",
                (
                    Value::Text(name.into()),
                    Value::Text(name.to_lowercase()),
                    Value::Integer(at),
                ),
            )
            .await
            .unwrap();
    }
    fx.conn
        .execute("DELETE FROM organization_names WHERE org_id = 1 AND lang = 'DEU'", ())
        .await
        .unwrap();
    let _ = stop;

    let r = fx.db.restore_dropped_satellites(false, None, 30).await.unwrap();
    assert_eq!((r.outstanding, r.restored, r.superseded), (2, 1, 1));
    assert_eq!(
        fx.variants_of(1).await[0],
        ("DEU".to_owned(), "Dobler GmbH".to_owned()),
        "the NEWEST pre-image is the one that comes back"
    );
}

/// A pre-image whose org was merged away since the drop must not take the
/// rest of the pass with it. organization_names has a foreign key onto
/// organizations and the pragma is on, so an unguarded INSERT would abort the
/// loop — and a re-run would hit the same row again, forever.
#[tokio::test]
async fn a_pre_image_whose_org_is_gone_is_skipped_not_fatal() {
    let fx = fixture("orphaned").await;
    fx.standard().await;
    fx.conn
        .execute("PRAGMA foreign_keys = ON", ())
        .await
        .unwrap();
    let stop = never;
    let dry = fx.db.drop_orphan_satellites(norm, true, None, None, 10, &stop).await.unwrap();
    let plan: Vec<(i64, String, String)> =
        dry.rows.iter().map(|o| (o.org, o.lang.clone(), o.key.clone())).collect();
    fx.db.drop_orphan_satellites(norm, false, Some(&plan), Some(7), 20, &stop).await.unwrap();
    // A second pre-image naming an org that does not exist — the shape a
    // merge leaves behind.
    fx.conn
        .execute(
            "INSERT INTO org_name_drops (org_id, lang, name, name_norm, key, target_org, dropped_at, job_id)
             VALUES (4242, 'DEU', 'Ghost GmbH', 'ghost gmbh', 'ghost gmbh', 2, 25, 7)",
            (),
        )
        .await
        .unwrap();

    let r = fx.db.restore_dropped_satellites(false, None, 30).await.unwrap();
    assert_eq!((r.outstanding, r.restored, r.orphaned), (2, 1, 1));
    assert_eq!(
        fx.variants_of(1).await.len(),
        3,
        "the restorable row came back despite the unrestorable one beside it"
    );
}

/// The undo is bounded to one pass. Without that it is no undo at all once
/// there are two: unwinding a bad run would drag back every good one with it.
#[tokio::test]
async fn the_undo_can_be_bounded_to_one_drop_pass() {
    let fx = fixture("bounded").await;
    fx.standard().await;
    fx.conn
        .execute("DELETE FROM organization_names WHERE org_id = 1", ())
        .await
        .unwrap();
    for (job, lang, name) in [(7i64, "DEU", "Dobler GmbH"), (8, "FRA", "Dobler SARL")] {
        fx.conn
            .execute(
                "INSERT INTO org_name_drops (org_id, lang, name, name_norm, key, target_org, dropped_at, job_id)
                 VALUES (1, ?, ?, ?, 'k', 2, 10, ?)",
                (
                    Value::Text(lang.into()),
                    Value::Text(name.into()),
                    Value::Text(name.to_lowercase()),
                    Value::Integer(job),
                ),
            )
            .await
            .unwrap();
    }

    let r = fx.db.restore_dropped_satellites(false, Some(8), 30).await.unwrap();
    assert_eq!((r.outstanding, r.restored), (1, 1), "job 8's pass only");
    assert_eq!(fx.variants_of(1).await, vec![("FRA".to_owned(), "Dobler SARL".to_owned())]);
    // Job 7's pre-image is untouched and still available.
    let r = fx.db.restore_dropped_satellites(true, Some(7), 40).await.unwrap();
    assert_eq!(r.outstanding, 1);
}
