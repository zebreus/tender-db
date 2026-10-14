//! Issue 524 unit 1: the `org-head-drift` census.
//!
//! `organizations.name` is written at mint and never again, so after merges and mention drops the served
//! head can name what the org's mentions no longer say: 31558903 (the University of Sheffield) is served as
//! `Sheffield Hallam University` over one stray mention, and 22713056 (a German review chamber) as the
//! pointer `s.o.` over 28 `Vergabekammer bei der Bezirksregierung` mentions. What these tests pin is how the
//! census sorts an organization: by where the head's KEY stands among its VOTING mentions' keys (aligned,
//! minority, absent), with the cosmetic, not-a-name, single-stray, below-half and form-only flags beside
//! it, the three splits, the nameless and non-name mentions kept out of every key, the merge-survivor
//! basis, the stray mentions by scheme, the canaries, the sample cap and the stop flag.

use std::sync::atomic::{AtomicUsize, Ordering};

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    // The census refuses without the mention and merged-identifier indexes, as prod has them.
    db.build_organization_indexes().await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

/// An org served as `head`, `provisional` as given, with one mention per entry of `mentions`: its name
/// (`None` is a NULL name) and its scheme (`None` is NULL).
async fn org_full(
    conn: &turso::Connection,
    id: i64,
    identifier: Option<&str>,
    provisional: bool,
    head: &str,
    mentions: &[(Option<&str>, Option<&str>)],
) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, 'GB', ?, ?, ?, ?, 0)",
        (
            Value::Integer(id),
            identifier.map_or(Value::Null, |_| Value::Text("national".into())),
            identifier.map_or(Value::Null, |i| Value::Text(i.into())),
            Value::Text(head.into()),
            Value::Integer(i64::from(provisional)),
        ),
    )
    .await
    .unwrap();
    for (i, (name, scheme)) in mentions.iter().enumerate() {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
             VALUES (?, 'ORG-0001', ?, ?, 'GB', NULL, ?)",
            (
                Value::Integer(id * 1000 + i as i64),
                Value::Integer(id),
                name.map_or(Value::Null, |n| Value::Text(n.into())),
                scheme.map_or(Value::Null, |s| Value::Text(s.into())),
            ),
        )
        .await
        .unwrap();
    }
}

/// An org served as `head`, with one scheme-less mention per entry of `mentions` (`None` is a NULL
/// name). No identifier makes it provisional, as the resolver mints it.
async fn org(conn: &turso::Connection, id: i64, identifier: Option<&str>, head: &str, mentions: &[Option<&str>]) {
    let mentions: Vec<(Option<&str>, Option<&str>)> = mentions.iter().map(|name| (*name, None)).collect();
    org_full(conn, id, identifier, identifier.is_none(), head, &mentions).await;
}

/// `n` mentions all spelled `name`.
fn times(name: &str, n: usize) -> Vec<Option<&str>> {
    vec![Some(name); n]
}

/// A merge folded a loser's identifier into `org_id`: the census's merge-survivor mark.
async fn merged(conn: &turso::Connection, org_id: i64, identifier: &str) {
    conn.execute(
        "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
         VALUES (?, 'national', 'GB', ?, 999999, 'e2-altid')",
        (Value::Text(identifier.into()), Value::Integer(org_id)),
    )
    .await
    .unwrap();
}

/// An alias row — the org named as its own loser (`altid-alias`, `renormalise`, `nhs-body`): no merge.
async fn alias(conn: &turso::Connection, org_id: i64, identifier: &str, rule: &str) {
    conn.execute(
        "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
         VALUES (?, 'national', 'GB', ?, ?, ?)",
        (Value::Text(identifier.into()), Value::Integer(org_id), Value::Integer(org_id), Value::Text(rule.into())),
    )
    .await
    .unwrap();
}

/// Stand-in for the app's `crosswalk::altid_name_key`: lower-cased alphanumeric words, which folds the
/// casing and punctuation the census calls cosmetic; a leading `the` is dropped and `ltd` / `limited`
/// read as the GB form `§ltd`, as the real key does. `org_name_norm` keeps both, so the two keys differ.
fn alt(name: &str) -> String {
    let mut words: Vec<String> =
        name.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_lowercase).collect();
    if words.first().is_some_and(|w| w == "the") {
        words.remove(0);
    }
    words
        .into_iter()
        .map(|w| if w == "ltd" || w == "limited" { "§ltd".to_owned() } else { w })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `crosswalk::altid_keys_agree`, mirrored: equal, or equal but for a GB form only one side carries.
fn agree(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let strip = |key: &str| {
        let mut tokens: Vec<&str> = key.split(' ').collect();
        let mut formed = false;
        while tokens.last().is_some_and(|t| matches!(*t, "§ltd" | "§plc" | "§llp" | "§lp" | "§cic")) {
            tokens.pop();
            formed = true;
        }
        (tokens.join(" "), formed)
    };
    let ((a, a_formed), (b, b_formed)) = (strip(a), strip(b));
    a_formed != b_formed && !a.is_empty() && a == b
}

/// Stand-in for `partyname::not_a_name` as its class: the pointer `s.o.`, a bare dash or zero, and an
/// award phrase beside a name.
fn class(name: &str) -> Option<&'static str> {
    match name.trim() {
        "s.o." => Some("placeholder"),
        "-" | "0" => Some("blank"),
        n if n.starts_with("Awarded to ") => Some("award_summary"),
        _ => None,
    }
}

fn never() -> bool {
    false
}

fn nowhere(_done: u64, _detail: &str) {}

async fn run(db: &store::Db, min_mentions: u64, samples: usize, page: i64, canaries: &[i64]) -> store::OrgHeadDriftReport {
    db.org_head_drift(store::OrgHeadDriftArgs {
        alt_key: alt,
        keys_agree: agree,
        not_a_name: class,
        min_mentions,
        samples,
        page,
        canaries,
        stop: &never,
        progress: &nowhere,
    })
    .await
    .unwrap()
}

/// The exhibits and their neighbours, one of each shape.
async fn corpus(conn: &turso::Connection) {
    // 1 — aligned, the head spelled exactly as its mentions.
    org(conn, 1, Some("GB00000001"), "Stadt Mainz", &times("Stadt Mainz", 2)).await;
    // 2 — aligned on the key, but no mention spells the head byte for byte: casing only. The two
    // spellings tie, and the mixed-case one is the display spelling.
    let mut acme = times("ACME LTD", 2);
    acme.extend(times("Acme Ltd", 2));
    org(conn, 2, Some("GB00000002"), "acme ltd", &acme).await;
    // 3 — the Sheffield shape (31558903): one stray mention elected the head, a merge survivor.
    let mut sheffield = times("UNIVERSITY OF SHEFFIELD", 3);
    sheffield.push(Some("University of Sheffield"));
    sheffield.push(Some("Sheffield Hallam University"));
    org(conn, 3, Some("GBUKPRN10007157"), "Sheffield Hallam University", &sheffield).await;
    merged(conn, 3, "GBUKPRNUKPRN10007157").await;
    // 4 — the `s.o.` shape (22713056): the placeholder head no mention holds.
    org(conn, 4, Some("0251411691"), "s.o.", &times("Vergabekammer bei der Bezirksregierung", 3)).await;
    // 5 — one mention: below the census.
    org(conn, 5, Some("GB00000005"), "Einzel GmbH", &times("Einzel GmbH", 1)).await;
    // 6 — no mention at all: below the census too.
    org(conn, 6, Some("GB00000006"), "Niemand AG", &[]).await;
    // 7 — a provisional org whose head key ties another: the head keeps the top.
    org(conn, 7, None, "Alpha GmbH", &[Some("Alpha GmbH"), Some("Beta GmbH")]).await;
}

#[tokio::test]
async fn head_drift_sorts_each_org_into_one_bucket_by_its_head_key() {
    let (db, conn) = open("ohd-buckets").await;
    corpus(&conn).await;
    let r = run(&db, 2, 30, 2_000, &[]).await;
    assert!(!r.stopped);
    assert_eq!(r.stray_top_min, store::STRAY_TOP_MIN);
    assert_eq!(r.orgs_walked, 7);
    assert_eq!(r.mentions_walked, 17);
    assert_eq!(r.below_min, store::HeadDriftCount { orgs: 2, mentions: 1 }, "orgs 5 and 6");
    assert_eq!((r.population.orgs, r.population.mentions), (5, 16));
    assert_eq!(r.aligned.orgs, 3, "1, 2 and the tie 7");
    assert_eq!((r.minority.orgs, r.minority.mentions), (1, 5));
    assert_eq!((r.absent.orgs, r.absent.mentions), (1, 3));
    assert_eq!(r.aligned.orgs + r.minority.orgs + r.absent.orgs, r.population.orgs, "the buckets partition");
    assert_eq!(r.cosmetic.orgs, 1);
    assert_eq!(r.not_a_name.orgs, 1);
    assert_eq!(r.single_stray.orgs, 1);
    // Every minority and absent head is below half; no aligned one here (7 holds one of two: half).
    assert_eq!(r.below_half.orgs, 2);
    assert_eq!(r.form_only.orgs, 0);
    assert!(r.non_name_mentions.is_empty() && r.award_summary_mentions == 0);
    assert_eq!(r.head_class.len(), 1);
    assert_eq!(r.head_class["placeholder"], store::HeadDriftCount { orgs: 1, mentions: 3 });
    // On org_name_norm the casing folds too, so the two keys agree here.
    assert_eq!(r.minority_norm.orgs, 1);
    assert_eq!(r.absent_norm.orgs, 1);
    assert_eq!(r.not_measured, vec!["roles"]);

    // The splits: 7 is the only identifier-NULL (and provisional) org, 3 the only merge survivor.
    assert_eq!(r.population.identifier_null, store::HeadDriftCount { orgs: 1, mentions: 2 });
    assert_eq!(r.population.identifier, store::HeadDriftCount { orgs: 4, mentions: 14 });
    assert_eq!(r.population.provisional.orgs, 1);
    assert_eq!(r.population.not_provisional.orgs, 4);
    assert_eq!(r.population.merged, store::HeadDriftCount { orgs: 1, mentions: 5 });
    assert_eq!(r.population.not_merged.orgs, 4);
    assert_eq!(r.minority.merged.orgs, 1);
    assert_eq!(r.minority.not_merged.orgs, 0);
    assert_eq!(r.absent.merged.orgs, 0);
    assert_eq!(r.aligned.identifier_null.orgs, 1);

    // Issue 456: one stray mention (Hallam) beside a top key of four; nothing else qualifies.
    assert_eq!(r.stray_mentions.all, store::HeadDriftCount { orgs: 1, mentions: 1 });
    assert_eq!(r.stray_mentions.identifier.mentions, 1);
    assert_eq!(r.stray_mentions.identifier_null.mentions, 0);
    assert_eq!(r.stray_mentions.by_scheme.len(), 1);
    assert_eq!(r.stray_mentions.by_scheme[store::STRAY_SCHEME_NONE], store::HeadDriftCount { orgs: 1, mentions: 1 });

    let sheffield = &r.samples.minority[0];
    assert_eq!(sheffield.org_id, 3);
    assert_eq!(sheffield.head, "Sheffield Hallam University");
    assert_eq!((sheffield.n, sheffield.voting, sheffield.head_key_count, sheffield.top_key_count), (5, 5, 1, 4));
    assert_eq!(sheffield.top_key, "university of sheffield");
    assert_eq!(sheffield.top_spelling, "UNIVERSITY OF SHEFFIELD", "three of four: the count wins over casing");
    assert_eq!(sheffield.distinct_keys, 2);
    assert!(sheffield.merged && sheffield.single_stray && sheffield.in_census && !sheffield.not_a_name);
    assert!(sheffield.below_half && !sheffield.form_only && sheffield.head_class.is_none());
    assert_eq!((sheffield.bucket, sheffield.norm_bucket), ("minority", "minority"));
    assert_eq!(r.samples.single_stray, vec![sheffield.clone()]);

    let so = &r.samples.absent[0];
    assert_eq!((so.org_id, so.head.as_str(), so.head_key_count, so.top_key_count), (4, "s.o.", 0, 3));
    assert_eq!(so.top_spelling, "Vergabekammer bei der Bezirksregierung");
    assert!(so.not_a_name && !so.single_stray);
    assert_eq!(so.head_class, Some("placeholder"));
    assert_eq!(r.samples.not_a_name, vec![so.clone()]);
    // Both are below half, and both are listed under their own bucket; the below-half list keeps its
    // places for the ALIGNED plurality heads, of which this corpus has none (org 7 is a 1:1 tie).
    assert_eq!(r.below_half.orgs, 2);
    assert!(r.samples.below_half.is_empty());

    let acme = &r.samples.cosmetic[0];
    assert_eq!(acme.org_id, 2);
    assert_eq!(acme.bucket, "aligned");
    assert_eq!(acme.top_spelling, "Acme Ltd", "a 2:2 tie prefers the mixed case over ALL-CAPS");
    assert_eq!(r.samples.minority.len() + r.samples.absent.len(), 2, "aligned orgs are not listed");

    // A higher bar drops the two-mention orgs (1 and 7) from the census.
    let r3 = run(&db, 3, 30, 2_000, &[]).await;
    assert_eq!(r3.below_min.orgs, 4);
    assert_eq!(r3.population.orgs, 3);
    assert_eq!(r3.aligned.orgs, 1);
}

#[tokio::test]
async fn head_drift_counts_nameless_mentions_but_keys_none() {
    let (db, conn) = open("ohd-nameless").await;
    // Three mention rows, one named: the census counts the org on its rows, the key on its name.
    org(&conn, 1, Some("DE1"), "Stadt Köln", &[None, Some("   "), Some("Stadt Köln")]).await;
    // Rows enough, but no name at all: no key to judge the head against, so in no bucket.
    org(&conn, 2, Some("DE2"), "Irgendwer", &[None, None]).await;
    let r = run(&db, 2, 30, 2_000, &[2]).await;
    assert_eq!(r.nameless, 4);
    assert_eq!(r.population.orgs, 1);
    assert_eq!(r.population.mentions, 3, "an org's mention count keeps its nameless rows");
    assert_eq!(r.aligned.orgs, 1);
    assert_eq!(r.cosmetic.orgs, 0);
    assert_eq!(r.unnamed, store::HeadDriftCount { orgs: 1, mentions: 2 });
    assert_eq!(r.absent.orgs, 0, "no name is no evidence against the head");
    let canary = &r.canaries[0];
    assert_eq!((canary.org_id, canary.bucket, canary.in_census, canary.n), (2, "unnamed", false, 2));
}

/// F3: a non-name mention (any class but `award_summary`) holds no key — `0` three times does not
/// out-vote the one real spelling — and an org of non-names alone is `unnamed`. A head classed
/// `award_summary` is counted under `head_class` but not flagged, and a mention of that class votes.
#[tokio::test]
async fn head_drift_keeps_non_name_mentions_out_of_every_key() {
    let (db, conn) = open("ohd-non-names").await;
    let mut firma = times("0", 3);
    firma.push(Some("Firma ApS"));
    org(&conn, 1, Some("DK1"), "Firma ApS", &firma).await;
    // Non-names alone: no vote, so in no bucket.
    org(&conn, 2, Some("DK2"), "Ghost AG", &times("s.o.", 2)).await;
    // An award phrase as the head: absent, counted by class, not flagged.
    org(&conn, 3, Some("DK3"), "Awarded to Bar Ltd", &times("Bar Ltd", 2)).await;
    // A placeholder head whose own spelling sits among the mentions: that mention does not vote for it.
    let mut kammer = times("s.o.", 1);
    kammer.extend(times("Kammer", 2));
    org(&conn, 4, Some("DK4"), "s.o.", &kammer).await;
    // Award-phrase mentions vote: the head is aligned on them.
    org(&conn, 5, Some("DK5"), "Awarded to Baz", &times("Awarded to Baz", 2)).await;
    let r = run(&db, 2, 30, 2_000, &[1, 2, 4]).await;

    assert_eq!(r.non_name_mentions.len(), 2);
    assert_eq!(r.non_name_mentions["blank"], 3);
    assert_eq!(r.non_name_mentions["placeholder"], 3);
    assert_eq!(r.award_summary_mentions, 2);
    assert_eq!(r.unnamed, store::HeadDriftCount { orgs: 1, mentions: 2 });
    assert_eq!(r.population.orgs, 4);
    assert_eq!(r.aligned.orgs, 2, "1 and 5");
    assert_eq!(r.absent.orgs, 2, "3 and 4");
    assert_eq!(r.single_stray.orgs, 0, "the zeros elect nothing");
    assert_eq!(r.stray_mentions.all.mentions, 0);
    assert_eq!(r.not_a_name.orgs, 1, "4 only: an award_summary head is not flagged");
    assert_eq!(r.head_class.len(), 2);
    assert_eq!(r.head_class["award_summary"], store::HeadDriftCount { orgs: 2, mentions: 4 });
    assert_eq!(r.head_class["placeholder"], store::HeadDriftCount { orgs: 1, mentions: 3 });

    let firma = &r.canaries[0];
    assert_eq!(firma.org_id, 1);
    assert_eq!((firma.bucket, firma.n, firma.voting, firma.head_key_count), ("aligned", 4, 1, 1));
    assert!(!firma.single_stray && !firma.below_half && !firma.cosmetic);
    let ghost = &r.canaries[1];
    assert_eq!((ghost.org_id, ghost.bucket, ghost.in_census, ghost.voting), (2, "unnamed", false, 0));
    let so = &r.canaries[2];
    assert_eq!((so.org_id, so.bucket, so.voting, so.head_key_count), (4, "absent", 2, 0));
    assert!(so.not_a_name);
}

/// F2: issue 524 (b), the head key under half the voting mentions, is a flag beside the bucket: a
/// 2:2:1 split leaves the head aligned (tied for the top) and below half.
#[tokio::test]
async fn head_drift_flags_a_plurality_head_below_half() {
    let (db, conn) = open("ohd-below-half").await;
    let mut split = times("Alpha GmbH", 2);
    split.extend(times("Beta GmbH", 2));
    split.push(Some("Gamma GmbH"));
    org(&conn, 1, Some("DE1"), "Alpha GmbH", &split).await;
    // Two of three: a majority, not below half.
    let mut majority = times("Delta AG", 2);
    majority.push(Some("Echo AG"));
    org(&conn, 2, Some("DE2"), "Delta AG", &majority).await;
    let r = run(&db, 2, 30, 2_000, &[1, 2]).await;
    assert_eq!(r.aligned.orgs, 2);
    assert_eq!(r.below_half.orgs, 1);
    assert_eq!(r.below_half.mentions, 5);
    assert_eq!(r.below_half.identifier.orgs, 1);
    let plurality = &r.canaries[0];
    assert_eq!((plurality.bucket, plurality.voting, plurality.head_key_count), ("aligned", 5, 2));
    assert_eq!(plurality.top_key, "alpha gmbh");
    assert!(plurality.below_half && !plurality.single_stray);
    assert!(!r.canaries[1].below_half);
    assert_eq!(r.samples.below_half, vec![plurality.clone()]);
}

/// F4: issue 456's agreement. A head that differs from the top key only by a GB legal form one side
/// carries is `form_only` drift, never a single stray, and such a mention is no stray either; the
/// Sheffield shape at the bar (one against two) still is.
#[tokio::test]
async fn head_drift_reads_a_legal_form_apart_as_form_only_not_a_stray() {
    let (db, conn) = open("ohd-form-only").await;
    org(&conn, 1, Some("GB1"), "Carnall Farrar", &times("Carnall Farrar Ltd", 3)).await;
    let mut limited = times("Carnall Farrar Limited", 3);
    limited.push(Some("Carnall Farrar"));
    org(&conn, 2, Some("GB2"), "Carnall Farrar", &limited).await;
    let r = run(&db, 2, 30, 2_000, &[1, 2]).await;
    assert_eq!((r.absent.orgs, r.minority.orgs), (1, 1));
    assert_eq!(r.form_only.orgs, 2);
    assert_eq!(r.form_only.mentions, 7);
    assert_eq!(r.single_stray.orgs, 0, "one mention a form apart from the top is not foreign");
    assert_eq!(r.stray_mentions.all, store::HeadDriftCount::default());
    let absent = &r.canaries[0];
    assert_eq!((absent.bucket, absent.top_key.as_str()), ("absent", "carnall farrar §ltd"));
    assert!(absent.form_only && !absent.single_stray);
    let minority = &r.canaries[1];
    assert_eq!((minority.bucket, minority.head_key_count, minority.top_key_count), ("minority", 1, 3));
    assert!(minority.form_only && !minority.single_stray);
    assert_eq!(r.samples.form_only.iter().map(|s| s.org_id).collect::<Vec<_>>(), vec![1, 2]);

    // A foreign head one mention holds, under a top of two (STRAY_TOP_MIN): a single stray and a
    // stray mention, not form-only.
    let mut hallam = times("University of Sheffield", 2);
    hallam.push(Some("Hallam University"));
    org(&conn, 3, Some("GB3"), "Hallam University", &hallam).await;
    let r = run(&db, 2, 30, 2_000, &[3]).await;
    assert_eq!(store::STRAY_TOP_MIN, 2);
    assert_eq!(r.single_stray.orgs, 1);
    assert_eq!(r.form_only.orgs, 2, "still 1 and 2 only");
    assert_eq!(r.stray_mentions.all, store::HeadDriftCount { orgs: 1, mentions: 1 });
    assert!(r.canaries[0].single_stray && !r.canaries[0].form_only);
}

/// F5: the stray mentions by their scheme (NULL is `none`), the first STRAY_SCHEMES_MAX distinct
/// schemes by name in walk order and the rest under `other`, an org counted there once.
#[tokio::test]
async fn head_drift_splits_stray_mentions_by_scheme_under_a_cap() {
    let (db, conn) = open("ohd-schemes").await;
    let mut main: Vec<(Option<&str>, Option<&str>)> = vec![(Some("Main Body"), Some("GB-COH")); 3];
    main.push((Some("Other Body"), Some("GB-PPON")));
    main.push((Some("Third Body"), None));
    org_full(&conn, 1, Some("GB1"), false, "Main Body", &main).await;
    let strays: Vec<(String, String)> = (0..22).map(|i| (format!("Stray {i:02}"), format!("s{i:02}"))).collect();
    let mut big: Vec<(Option<&str>, Option<&str>)> = vec![(Some("Big Body"), None); 2];
    big.extend(strays.iter().map(|(name, scheme)| (Some(name.as_str()), Some(scheme.as_str()))));
    org_full(&conn, 2, Some("GB2"), false, "Big Body", &big).await;
    let r = run(&db, 2, 30, 2_000, &[]).await;

    assert_eq!(r.stray_mentions.all, store::HeadDriftCount { orgs: 2, mentions: 24 });
    let by = &r.stray_mentions.by_scheme;
    assert_eq!(by["GB-PPON"], store::HeadDriftCount { orgs: 1, mentions: 1 });
    assert_eq!(by[store::STRAY_SCHEME_NONE], store::HeadDriftCount { orgs: 1, mentions: 1 });
    assert!(!by.contains_key("GB-COH"), "the top key's mentions are no strays");
    // GB-PPON and none, then s00..s17 by name: twenty; s18..s21 under `other`, org 2 once.
    assert_eq!(by.len(), store::STRAY_SCHEMES_MAX + 1);
    assert_eq!(by["s17"], store::HeadDriftCount { orgs: 1, mentions: 1 });
    assert!(!by.contains_key("s18"));
    assert_eq!(by[store::STRAY_SCHEME_OTHER], store::HeadDriftCount { orgs: 1, mentions: 4 });
    assert_eq!(by.values().map(|c| c.mentions).sum::<u64>(), r.stray_mentions.all.mentions);
}

/// F1: a merge survivor is an org holding a merged identifier some OTHER org lost. The rows that name
/// the org as its own loser (alias binds, renormalise pre-images, nhs-body codes) record no merge.
#[tokio::test]
async fn head_drift_reads_a_merge_survivor_only_from_a_loser_row() {
    let (db, conn) = open("ohd-merged").await;
    org(&conn, 1, Some("GB1"), "Kept Ltd", &times("Kept Ltd", 2)).await;
    merged(&conn, 1, "GB0LOSER").await;
    org(&conn, 2, Some("GB2"), "Aliased Ltd", &times("Aliased Ltd", 2)).await;
    alias(&conn, 2, "GBNHSRXX", "nhs-body").await;
    alias(&conn, 2, "GB2OLD", "renormalise").await;
    // Both: a real loser row beside an alias row.
    org(&conn, 3, Some("GB3"), "Both Ltd", &times("Both Ltd", 2)).await;
    alias(&conn, 3, "GBALIAS3", "altid-alias").await;
    merged(&conn, 3, "GB3LOSER").await;
    let r = run(&db, 2, 30, 2_000, &[1, 2, 3]).await;
    assert_eq!(r.population.merged, store::HeadDriftCount { orgs: 2, mentions: 4 });
    assert_eq!(r.population.not_merged, store::HeadDriftCount { orgs: 1, mentions: 2 });
    let flags: Vec<(i64, bool)> = r.canaries.iter().map(|c| (c.org_id, c.merged)).collect();
    assert_eq!(flags, vec![(1, true), (2, false), (3, true)]);
}

/// F6 (a): the secondary key judges on its own. `org_name_norm` keeps a leading `the` the alt key
/// drops, so a head aligned on the alt key is a minority, or absent, on the norm key.
#[tokio::test]
async fn head_drift_judges_the_norm_key_apart_from_the_alt_key() {
    let (db, conn) = open("ohd-norm").await;
    let mut foo = times("Foo Trust", 2);
    foo.push(Some("The Foo Trust"));
    org(&conn, 1, Some("GB1"), "The Foo Trust", &foo).await;
    org(&conn, 2, Some("GB2"), "The Bar Trust", &times("Bar Trust", 2)).await;
    let r = run(&db, 2, 30, 2_000, &[1, 2]).await;
    assert_eq!(r.aligned.orgs, 2);
    assert_eq!((r.minority.orgs, r.absent.orgs), (0, 0));
    assert_eq!((r.minority_norm.orgs, r.absent_norm.orgs), (1, 1));
    let foo = &r.canaries[0];
    assert_eq!((foo.bucket, foo.norm_bucket), ("aligned", "minority"));
    assert!(!foo.cosmetic, "a mention spells the head");
    let bar = &r.canaries[1];
    assert_eq!((bar.bucket, bar.norm_bucket), ("aligned", "absent"));
    assert!(bar.cosmetic);
}

/// F6 (b): on a tie the head keeps the top whatever its place in the order — `Beta` tied with
/// `Alpha` is aligned, and the top key is the head's.
#[tokio::test]
async fn head_drift_keeps_a_tied_head_on_top_when_it_sorts_last() {
    let (db, conn) = open("ohd-tie").await;
    org(&conn, 1, None, "Beta GmbH", &[Some("Alpha GmbH"), Some("Beta GmbH")]).await;
    let r = run(&db, 2, 30, 2_000, &[1]).await;
    assert_eq!(r.aligned.orgs, 1);
    let beta = &r.canaries[0];
    assert_eq!((beta.bucket, beta.top_key.as_str(), beta.top_spelling.as_str()), ("aligned", "beta gmbh", "Beta GmbH"));
    assert_eq!((beta.head_key_count, beta.top_key_count), (1, 1));
}

/// F6 (c): the provisional split reads `provisional`, not the identifier — an identified provisional
/// org and an unidentified settled one land on opposite sides of each.
#[tokio::test]
async fn head_drift_splits_provisional_apart_from_the_identifier() {
    let (db, conn) = open("ohd-provisional").await;
    let two: Vec<(Option<&str>, Option<&str>)> = vec![(Some("Eins GmbH"), None); 2];
    org_full(&conn, 1, Some("DE1"), true, "Eins GmbH", &two).await;
    let three: Vec<(Option<&str>, Option<&str>)> = vec![(Some("Zwei GmbH"), None); 3];
    org_full(&conn, 2, None, false, "Zwei GmbH", &three).await;
    let r = run(&db, 2, 30, 2_000, &[]).await;
    let p = &r.population;
    assert_eq!(p.provisional, store::HeadDriftCount { orgs: 1, mentions: 2 });
    assert_eq!(p.not_provisional, store::HeadDriftCount { orgs: 1, mentions: 3 });
    assert_eq!(p.identifier, store::HeadDriftCount { orgs: 1, mentions: 2 });
    assert_eq!(p.identifier_null, store::HeadDriftCount { orgs: 1, mentions: 3 });
    assert_eq!(r.aligned.provisional, p.provisional);
    assert_eq!(r.aligned.identifier_null, p.identifier_null);
}

#[tokio::test]
async fn head_drift_reports_canaries_whatever_their_bucket_and_paging_changes_nothing() {
    let (db, conn) = open("ohd-canaries").await;
    corpus(&conn).await;
    // 5 is below the census, 3 a minority, 99 not there at all.
    let one_page = run(&db, 2, 30, 2_000, &[5, 3, 99]).await;
    let ids: Vec<i64> = one_page.canaries.iter().map(|c| c.org_id).collect();
    assert_eq!(ids, vec![3, 5], "in walk order; a missing id yields no row");
    let below = &one_page.canaries[1];
    assert_eq!((below.bucket, below.in_census, below.n), ("aligned", false, 1));
    assert_eq!(one_page.canaries[0].bucket, "minority");
    assert!(one_page.canaries[0].in_census);
    // A canary below the census is counted there and in no bucket.
    assert_eq!(one_page.below_min.orgs, 2);
    assert_eq!(one_page.population.orgs, 5);

    // Pages of two: the same report, field for field.
    let paged = run(&db, 2, 30, 2, &[5, 3, 99]).await;
    assert_eq!(paged, one_page);
}

#[tokio::test]
async fn head_drift_caps_the_samples_in_id_order_and_never_the_tallies() {
    let (db, conn) = open("ohd-cap").await;
    for id in 1..=5i64 {
        org(&conn, id, Some(format!("GB{id}").as_str()), "-", &times(&format!("Firma {id}"), 2)).await;
    }
    let r = run(&db, 2, 2, 2, &[]).await;
    assert_eq!(r.absent.orgs, 5);
    assert_eq!(r.not_a_name.orgs, 5);
    assert_eq!(r.samples_per_bucket, 2);
    assert_eq!(r.samples.absent.iter().map(|s| s.org_id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(r.samples.not_a_name.iter().map(|s| s.org_id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(r.below_half.orgs, 5, "the tally counts every below-half head");
    assert!(r.samples.below_half.is_empty(), "absent heads are listed under absent, not below half");
}

/// D3: the below-half list exists for the aligned plurality heads, so minority and absent orgs that
/// come first by id cannot crowd them out of a capped list.
#[tokio::test]
async fn head_drift_lists_below_half_samples_for_aligned_plurality_heads_only() {
    let (db, conn) = open("ohd-below-half-samples").await;
    for id in 1..=3i64 {
        org(&conn, id, Some(format!("GB{id}").as_str()), "Absent Head", &times(&format!("Firma {id}"), 2)).await;
    }
    let mut split = times("Alpha GmbH", 2);
    split.extend(times("Beta GmbH", 2));
    split.push(Some("Gamma GmbH"));
    org(&conn, 4, Some("GB4"), "Alpha GmbH", &split).await;
    let r = run(&db, 2, 1, 2_000, &[]).await;
    assert_eq!((r.absent.orgs, r.below_half.orgs), (3, 4));
    assert_eq!(r.samples.below_half.iter().map(|s| (s.org_id, s.bucket)).collect::<Vec<_>>(), vec![(4, "aligned")]);
}

/// D4: issue 456's stray "shares no key with the org's other mentions" — a mention whose key agrees
/// with ANY other voting key (a GB form apart) is no stray, whatever key leads and whatever the head;
/// and a head a form apart from a key tied for the top is form-only even when that key sorts last.
#[tokio::test]
async fn head_drift_judges_a_stray_against_every_other_key_not_the_top_alone() {
    let (db, conn) = open("ohd-stray-agree").await;
    // `Hallam` agrees with `Hallam Ltd`×2, not with the top `University of Sheffield`×3: no stray.
    let mut sheffield = times("University of Sheffield", 3);
    sheffield.extend(times("Hallam Ltd", 2));
    sheffield.push(Some("Hallam"));
    org(&conn, 1, Some("GB1"), "Hallam", &sheffield).await;
    // A tie between `Alpha Ltd`×2 and `Beta GmbH`×2, and `Alpha`×1 a form apart from the first: no stray
    // whether the head is Beta (Beta leads the tie) or Alpha Ltd (Alpha leads it).
    let tied = |n: usize| {
        let mut v = times("Alpha Ltd", n);
        v.extend(times("Beta GmbH", 2));
        v.push(Some("Alpha"));
        v
    };
    org(&conn, 2, Some("GB2"), "Beta GmbH", &tied(2)).await;
    org(&conn, 3, Some("GB3"), "Alpha Ltd", &tied(2)).await;
    // `Zeta Farrar` absent, a form apart from `Zeta Farrar Ltd`×2, tied with `Alpha GmbH`×2 (which sorts
    // first and so is the top key): still form-only.
    let mut zeta = times("Alpha GmbH", 2);
    zeta.extend(times("Zeta Farrar Ltd", 2));
    org(&conn, 4, Some("GB4"), "Zeta Farrar", &zeta).await;
    let r = run(&db, 2, 30, 2_000, &[1, 2, 3, 4]).await;
    assert_eq!(r.stray_mentions.all, store::HeadDriftCount::default(), "no mention here shares no key");
    let hallam = &r.canaries[0];
    assert_eq!((hallam.bucket, hallam.head_key_count, hallam.top_key_count), ("minority", 1, 3));
    assert!(!hallam.single_stray, "the head agrees with `hallam §ltd`, so it is not foreign");
    assert!(!hallam.form_only, "…but no key AT THE TOP agrees with it");
    assert!(!r.canaries[1].single_stray && !r.canaries[2].single_stray);
    let zeta = &r.canaries[3];
    assert_eq!((zeta.bucket, zeta.top_key.as_str()), ("absent", "alpha gmbh"));
    assert!(zeta.form_only, "a form apart from a key tied for the top");
    assert_eq!(r.form_only.orgs, 1);
}

#[tokio::test]
async fn head_drift_reads_the_stop_flag_before_every_page_and_a_stopped_walk_reports_nothing() {
    let (db, conn) = open("ohd-stop").await;
    corpus(&conn).await;
    let checks = AtomicUsize::new(0);
    let pages = AtomicUsize::new(0);
    // Free for the first page, stopped before the second.
    let stop = || checks.fetch_add(1, Ordering::SeqCst) >= 1;
    let progress = |_: u64, _: &str| {
        pages.fetch_add(1, Ordering::SeqCst);
    };
    let r = db
        .org_head_drift(store::OrgHeadDriftArgs {
            alt_key: alt,
            keys_agree: agree,
            not_a_name: class,
            min_mentions: 2,
            samples: 30,
            page: 3,
            canaries: &[3],
            stop: &stop,
            progress: &progress,
        })
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!(pages.load(Ordering::SeqCst), 1, "one page walked before the stop was read");
    assert_eq!(checks.load(Ordering::SeqCst), 2);
    assert_eq!(
        r,
        store::OrgHeadDriftReport { stopped: true, ..Default::default() },
        "a partial census reads as a clean corpus: nothing is reported"
    );
}

#[tokio::test]
async fn head_drift_refuses_without_the_mention_index() {
    let (db, conn) = open("ohd-refuse").await;
    corpus(&conn).await;
    conn.execute("DROP INDEX organization_mentions_org", ()).await.unwrap();
    let err = db
        .org_head_drift(store::OrgHeadDriftArgs {
            alt_key: alt,
            keys_agree: agree,
            not_a_name: class,
            min_mentions: 2,
            samples: 30,
            page: 2_000,
            canaries: &[],
            stop: &never,
            progress: &nowhere,
        })
        .await
        .expect_err("every chunk would scan the mention table");
    assert!(err.to_string().contains("organization_mentions_org") && err.to_string().contains("refused"), "{err}");
}
