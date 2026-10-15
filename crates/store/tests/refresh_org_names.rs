//! Issue 524 unit 2a: the head ELECTION (`store::elect_head`) and the dry `refresh-org-names` plan.
//!
//! `organizations.name` is the first mention's first-seen spelling, written at mint and never again, so
//! 31558903 (the University of Sheffield) is served as `Sheffield Hallam University` over one stray
//! mention and 22713056 as the pointer `s.o.`. The unit 2 decision re-elects the head of
//! identifier-bearing organizations from their mentions, sticky: no single mention, near tie,
//! abbreviation majority or legal-form difference moves it, and a Latin head over Cyrillic mentions is
//! TED's ENG rendering (ADR-0013 D4). These tests pin each step of the rule on the decision's sample
//! shapes (178, 84/131, 10/301, 334 by their census sample ids), the exclusions, the plan's counts,
//! listings, uniform samples and canaries, paging, the scope, the stop flag, the index refusal, and that
//! the plan writes nothing.

use std::sync::atomic::{AtomicUsize, Ordering};

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    // The planner refuses without the mention and merged-identifier indexes, as prod has them.
    db.build_organization_indexes().await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

/// An org of `country` served as `head` (its `name_norm` as the resolver derives it), identified as a
/// `national` `identifier` or provisional without one, with `n` mentions of each spelling.
async fn org(
    conn: &turso::Connection,
    id: i64,
    country: &str,
    identifier: Option<&str>,
    head: &str,
    mentions: &[(&str, usize)],
) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
        vec![
            Value::Integer(id),
            Value::Text(country.into()),
            identifier.map_or(Value::Null, |_| Value::Text("national".into())),
            identifier.map_or(Value::Null, |i| Value::Text(i.into())),
            Value::Text(head.into()),
            Value::Text(store::org_name_norm(head)),
            Value::Integer(i64::from(identifier.is_none())),
        ],
    )
    .await
    .unwrap();
    let mut i = 0i64;
    for (name, n) in mentions {
        for _ in 0..*n {
            conn.execute(
                "INSERT INTO organization_mentions
                   (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
                 VALUES (?, 'ORG-0001', ?, ?, ?, NULL, NULL)",
                vec![
                    Value::Integer(id * 1000 + i),
                    Value::Integer(id),
                    Value::Text((*name).into()),
                    Value::Text(country.into()),
                ],
            )
            .await
            .unwrap();
            i += 1;
        }
    }
}

/// An `org_identifier_verdicts` row on `(identifier, 'national', country)`.
async fn verdict(
    conn: &turso::Connection,
    identifier: &str,
    country: &str,
    org_id: i64,
    verdict: &str,
    applied_at: Option<i64>,
) {
    conn.execute(
        "INSERT INTO org_identifier_verdicts (identifier, identifier_kind, country, org_id, cohort, verdict, rationale,
                                              confidence, reviewed_at, applied_at)
         VALUES (?, 'national', ?, ?, 'test', ?, 'fixture', 'high', 0, ?)",
        vec![
            Value::Text(identifier.into()),
            Value::Text(country.into()),
            Value::Integer(org_id),
            Value::Text(verdict.into()),
            applied_at.map_or(Value::Null, Value::Integer),
        ],
    )
    .await
    .unwrap();
}

/// An `organization_merged_identifiers` row naming `loser` as the org that lost `identifier` to
/// `org_id`: a merge survivor when `loser <> org_id`, an alias row when equal.
async fn merged(conn: &turso::Connection, org_id: i64, identifier: &str, loser: i64) {
    conn.execute(
        "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
         VALUES (?, 'national', 'GB', ?, ?, 'e2-altid')",
        vec![Value::Text(identifier.into()), Value::Integer(org_id), Value::Integer(loser)],
    )
    .await
    .unwrap();
}

async fn eng(conn: &turso::Connection, org_id: i64, name: &str) {
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (?, 'ENG', ?, ?)",
        vec![Value::Integer(org_id), Value::Text(name.into()), Value::Text(store::org_name_norm(name))],
    )
    .await
    .unwrap();
}

/// Stand-in for `crosswalk::head_vote_key`: lower-cased alphanumeric words, a run of single letters one
/// initialism (`S.R.L.` is `srl`), a leading-or-anywhere `the` and the legal forms dropped; a name of
/// forms alone keeps its words.
fn vote(name: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut initials = String::new();
    for word in name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let word = word.to_lowercase();
        if word.chars().count() == 1 {
            initials.push_str(&word);
            continue;
        }
        if !initials.is_empty() {
            words.push(std::mem::take(&mut initials));
        }
        words.push(word);
    }
    if !initials.is_empty() {
        words.push(initials);
    }
    let kept: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|w| !matches!(*w, "the" | "ltd" | "limited" | "plc" | "srl" | "sarl" | "gmbh" | "ag"))
        .collect();
    if kept.is_empty() { words.join(" ") } else { kept.join(" ") }
}

/// Stand-in for `partyname::not_a_name` as its class.
fn class(name: &str) -> Option<&'static str> {
    match name.trim() {
        "s.o." => Some("placeholder"),
        "-" | "0" => Some("blank"),
        n if n.starts_with("Awarded to ") => Some("award_summary"),
        _ => None,
    }
}

fn consortium(name: &str) -> bool {
    name.to_lowercase().split(|c: char| !c.is_alphanumeric()).any(|t| matches!(t, "groupement" | "consortium"))
}

/// Stand-in for `crosswalk::head_spelling_unfit`.
fn unfit(candidate: &str, head: &str) -> Option<&'static str> {
    if candidate.chars().any(|c| c.is_control() || c == '\u{FFFD}') {
        return Some("control");
    }
    if candidate.chars().count() > 200 {
        return Some("long");
    }
    let cased = candidate.chars().any(|c| c.is_lowercase() || c.is_uppercase());
    if cased && !candidate.chars().any(char::is_uppercase) {
        return Some("lowercase");
    }
    if class(candidate).is_some() {
        return Some("non_name");
    }
    if consortium(candidate) && !consortium(head) {
        return Some("consortium");
    }
    None
}

fn last_word(name: &str) -> Option<String> {
    name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).last().map(str::to_owned)
}

/// Stand-in for `crosswalk::legal_form_family`: the continental forms.
fn continental(name: &str) -> Option<&'static str> {
    match last_word(name).as_deref() {
        Some("gmbh") => Some("gmbh"),
        Some("ag") => Some("ag"),
        Some("sarl") => Some("sarl"),
        _ => None,
    }
}

/// Stand-in for `crosswalk::gb_legal_family`.
fn gb(name: &str) -> Option<&'static str> {
    match last_word(name).as_deref() {
        Some("ltd" | "limited") => Some("ltd"),
        Some("plc") => Some("plc"),
        _ => None,
    }
}

/// Stand-in for `crosswalk::head_family`: the continental table, else the GB one.
fn family(name: &str) -> Option<&'static str> {
    continental(name).or_else(|| gb(name))
}

/// Stand-in for `crosswalk::head_change_hold`: a family on both sides of ONE table that differs, or a
/// consortium mark dropped. A pair read from the two tables (`gmbh` -> `ltd`) is not held.
fn hold(old: &str, new: &str, _kind: Option<&str>, _identifier: Option<&str>) -> Option<&'static str> {
    if let (Some(a), Some(b)) = (continental(old), continental(new))
        && a != b
    {
        return Some("family");
    }
    if let (Some(a), Some(b)) = (gb(old), gb(new))
        && a != b
    {
        return Some("gb_family");
    }
    if consortium(old) && !consortium(new) {
        return Some("consortium_removed");
    }
    None
}

/// Stand-in for `crosswalk::head_r2_hold`: an upper-case token of 2-5 letters that is the initials of a
/// run of the head's words, or the head's vote tokens cut to a proper subset that drops only words, one
/// of three letters or more.
fn r2_hold(old: &str, new: &str) -> Option<&'static str> {
    let initials: String = old
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|w| w.chars().next())
        .flat_map(char::to_lowercase)
        .collect();
    let abbreviates = new.split(|c: char| !c.is_alphanumeric()).any(|t| {
        (2..=5).contains(&t.chars().count())
            && t.chars().all(|c| c.is_alphabetic() && c.is_uppercase())
            && initials.contains(&t.to_lowercase())
    });
    if abbreviates {
        return Some("initialism");
    }
    let tokens = |name: &str| -> std::collections::BTreeSet<String> { vote(name).split(' ').map(str::to_owned).collect() };
    let (was, is) = (tokens(old), tokens(new));
    if is.len() >= was.len() || !is.is_subset(&was) {
        return None;
    }
    let dropped: Vec<&String> = was.difference(&is).collect();
    (dropped.iter().all(|t| t.chars().all(char::is_alphabetic)) && dropped.iter().any(|t| t.chars().count() >= 3))
        .then_some("truncation")
}

const FNS: store::HeadElectFns = store::HeadElectFns { vote_key: vote, not_a_name: class, unfit, hold, r2_hold };

fn never() -> bool {
    false
}

fn nowhere(_done: u64, _detail: &str) {}

fn args<'a>(page: i64, samples: usize, org_ids: Option<&'a [i64]>, canaries: &'a [i64]) -> store::OrgNameRefreshArgs<'a> {
    store::OrgNameRefreshArgs {
        fns: FNS,
        family,
        min_mentions: 2,
        samples,
        page,
        listing_cap: store::ORG_NAME_REFRESH_LISTING_CAP,
        org_ids,
        canaries,
        stop: &never,
        progress: &nowhere,
    }
}

async fn plan(db: &store::Db, page: i64, org_ids: Option<&[i64]>, canaries: &[i64]) -> store::OrgNameRefreshPlan {
    db.plan_org_name_refresh(args(page, 30, org_ids, canaries)).await.unwrap()
}

/// The decision's shapes, one org each.
async fn corpus(conn: &turso::Connection) {
    let sheffield = [("University of Sheffield", 4), ("Sheffield Hallam University", 1)];
    // 1 — 31558903: 16 of 17 mentions say Sheffield, the head is the one stray. R1, and the served
    // spelling is the fit mixed-case one, never the count-first ALL-CAPS. A merge survivor, and a
    // `right` verdict on its triple is no bind defect.
    org(conn, 1, "GB", Some("GBUKPRN10007157"), "Sheffield Hallam University", &[
        ("UNIVERSITY OF SHEFFIELD", 14),
        ("University of Sheffield", 1),
        ("The University of Sheffield", 1),
        ("Sheffield Hallam University", 1),
    ])
    .await;
    merged(conn, 1, "GBUKPRNUKPRN10007157", 999_999).await;
    verdict(conn, "GBUKPRN10007157", "GB", 1, "right", None).await;
    // 2 — 22713056: the pointer `s.o.` over 28 mentions of the chamber. R0n. Its alias row is no merge.
    org(conn, 2, "DE", Some("0251411691"), "s.o.", &[("Vergabekammer bei der Bezirksregierung", 28)]).await;
    merged(conn, 2, "DE0251411691", 2).await;
    // 3 — 6341366: an empty head. R0e (`not_a_name("")` is None).
    org(conn, 3, "SE", Some("2120000829"), "", &[("Kalmar kommun", 4)]).await;
    // 4 — 7912592: TED's Latin rendering over Cyrillic mentions; its ENG satellite IS the head. D4.
    let belovo = [("ОБЩИНА БЕЛОВО", 29), ("Obshtina Belovo", 1)];
    org(conn, 4, "BG", Some("000215853"), "Obshtina Belovo", &belovo).await;
    eng(conn, 4, "Obshtina Belovo").await;
    // 5 — sample 178: the abbreviation holds 66% but not twice the full name (63 < 64). Bar.
    let full = "Communauté d'agglomération Plaine Vallée";
    org(conn, 5, "FR", Some("FR178"), full, &[("CA Plaine Vallée", 63), (full, 32)]).await;
    // 6 — sample 131's share, 52.5% (the conflated identifier's coin flip, Grasse -> Marseille). The top
    // holds 10x the head (2x passes) and the head holds 2 (R2 is off), so the 55% bar alone keeps it.
    let grasse = "Tribunal administratif de Grasse";
    let marseille = [("Tribunal administratif de Marseille", 21), ("Tribunal administratif de Nice", 17), (grasse, 2)];
    org(conn, 6, "FR", Some("FR131"), grasse, &marseille).await;
    // 7 — sample 84's share, 50.3%: again 2x passes and R2 is off; 55% keeps it.
    org(conn, 7, "FR", Some("FR84"), "Hopital Nord", &[("Hopital Sud", 81), ("Hopital Est", 78), ("Hopital Nord", 2)])
        .await;
    // 8 — samples 10/301: the forms are one vote key, so a head no mention spells is sticky. S1.
    org(conn, 8, "RO", Some("RO10"), "WECO TMC S.R.L.", &[("WECO TMC", 4), ("WECO TMC SRL", 2)]).await;
    // 9 — sample 334: the winning cluster spells only lower case. Held, no fit spelling.
    org(conn, 9, "FR", Some("FR334"), "Atelier Nord", &[("les alchimistes", 3), ("Atelier Nord", 1)]).await;
    // 10 — identifier NULL: name_norm is the 234 bind key. E1, whatever the mentions say.
    org(conn, 10, "GB", None, "Sheffield Hallam University", &sheffield).await;
    // 11 — a `wrong` verdict on the triple: the bind is the defect. E2.
    org(conn, 11, "GB", Some("GB00000011"), "Sheffield Hallam University", &sheffield).await;
    verdict(conn, "GB00000011", "GB", 11, "wrong", None).await;
    // 12 — a `related` verdict not yet applied: E2 too (the ORG_COLUMNS predicate).
    org(conn, 12, "GB", Some("GB00000012"), "Sheffield Hallam University", &sheffield).await;
    verdict(conn, "GB00000012", "GB", 12, "related", None).await;
    // 13 — a `related` verdict already applied is no defect: R1.
    org(conn, 13, "GB", Some("GB00000013"), "Sheffield Hallam University", &sheffield).await;
    verdict(conn, "GB00000013", "GB", 13, "related", Some(1)).await;
    // 14 — T = 2: no cluster under three ever replaces a real name. Bar.
    org(conn, 14, "DE", Some("DE14"), "Alpha GmbH", &[("Beta AG", 2), ("Alpha GmbH", 1)]).await;
    // 15 — R2: a stray head (one vote) under a unique top of three at exactly 25%; the change gains a
    // legal form (none -> x is allowed and counted).
    org(conn, 15, "DE", Some("DE15"), "Stray Body", &[
        ("Main Body Ltd", 3),
        ("Nord Body", 2),
        ("Sued Body", 2),
        ("Ost Body", 2),
        ("West Body", 2),
        ("Stray Body", 1),
    ])
    .await;
    // 16 — the same stray under a TIED top: neither rule fires. Bar.
    org(conn, 16, "DE", Some("DE16"), "Stray Body", &[("Main Body", 3), ("Second Body", 3), ("Stray Body", 1)]).await;
    // 17 — plc -> Ltd: R1 elects it, the bind guard holds it.
    org(conn, 17, "GB", Some("GB00000017"), "Foo Holdings plc", &[("Bar Services Ltd", 5), ("Foo Holdings plc", 1)]).await;
    // 18 — one mention: below the population.
    org(conn, 18, "DE", Some("DE18"), "Einzel GmbH", &[("Einzel GmbH", 1)]).await;
    // 19 — non-names alone: no voter. E3.
    org(conn, 19, "DE", Some("DE19"), "Ghost GmbH", &[("-", 2), ("s.o.", 1)]).await;
    // 20 — D4 whose ENG satellite agrees with the head on the vote key but not byte for byte.
    org(conn, 20, "BG", Some("000000020"), "OBSHTINA SOFIA", &[("СТОЛИЧНА ОБЩИНА", 5)]).await;
    eng(conn, 20, "Obshtina Sofia").await;
    // 21 — D4 over Greek mentions, no ENG satellite.
    org(conn, 21, "GR", Some("GR21"), "Dimos Athinaion", &[("ΔΗΜΟΣ ΑΘΗΝΑΙΩΝ", 4)]).await;
    // 22 — D4 whose ENG satellite names the body some other way: `differs`.
    org(conn, 22, "BG", Some("000000022"), "Obshtina Varna", &[("ОБЩИНА ВАРНА", 3)]).await;
    eng(conn, 22, "Municipality of Varna").await;
    // 23 — R1 from a continental form to a GB one: two families, but read from different tables, so
    // the hold lets it through and the plan counts it `x->y`.
    org(conn, 23, "GB", Some("GB00000023"), "Foo Trading GmbH", &[("Bar Services Ltd", 5), ("Foo Trading GmbH", 1)])
        .await;
    // 24 — 15075247's shape: R2 would serve the initialism of the head's first words. Held.
    let caux = "Communauté de communes Terroir de Caux";
    let others = [("Nord", 2), ("Sued", 2), ("Ost", 2), ("West", 2)];
    let mut cdc = vec![("CDC Terroir de Caux", 3), (caux, 1)];
    cdc.extend(others);
    org(conn, 24, "FR", Some("FR24"), caux, &cdc).await;
    // 25 — R2 would serve the head cut to its first words. Held.
    let greffe = "Tribunal administratif de Nice Greffe";
    let mut nice = vec![("Tribunal administratif de Nice", 3), (greffe, 1)];
    nice.extend(others);
    org(conn, 25, "FR", Some("FR25"), greffe, &nice).await;
}

/// Every organization's `(id, name, name_norm)`, by id.
async fn heads(conn: &turso::Connection) -> Vec<(Value, Value, Value)> {
    let mut rows = conn.query("SELECT id, name, name_norm FROM organizations ORDER BY id", ()).await.unwrap();
    let mut all = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        all.push((row.get_value(0).unwrap(), row.get_value(1).unwrap(), row.get_value(2).unwrap()));
    }
    all
}

async fn change_rows(conn: &turso::Connection) -> Value {
    let mut rows = conn.query("SELECT COUNT(*) FROM changes", ()).await.unwrap();
    rows.next().await.unwrap().unwrap().get_value(0).unwrap()
}

fn count(orgs: u64, mentions: u64) -> store::HeadDriftCount {
    store::HeadDriftCount { orgs, mentions }
}

fn ids(entries: &[store::HeadRefreshEntry]) -> Vec<i64> {
    entries.iter().map(|e| e.org_id).collect()
}

#[tokio::test]
async fn the_plan_elects_each_shape_as_the_decision_reads_it() {
    let (db, conn) = open("ron-shapes").await;
    corpus(&conn).await;
    // One feed row already there, so "no change row written" is a count that must not move.
    conn.execute(
        "INSERT INTO changes (entity_kind, entity_id, version_seq, op, changed_at)
         VALUES ('organization', 1, NULL, 'added', 0)",
        (),
    )
    .await
    .unwrap();
    let before = heads(&conn).await;
    assert_eq!(before.len(), 25);
    let r = plan(&db, 2_000, None, &[]).await;
    assert!(!r.stopped);
    assert_eq!(r.epoch, store::HEAD_ELECTION_EPOCH);
    assert_eq!((r.orgs_walked, r.mentions_walked), (25, 479));
    assert_eq!(r.below_min, count(1, 1), "org 18");
    assert_eq!(r.population, count(24, 478));
    assert_eq!(r.empty_heads, count(1, 4), "org 3");
    assert_eq!(r.changes.orgs + r.holds.orgs + r.keeps.orgs, r.population.orgs, "every elected org has one verdict");

    assert_eq!(r.by_rule.len(), 4);
    assert_eq!(r.by_rule["R0e"], count(1, 4));
    assert_eq!(r.by_rule["R0n"], count(1, 28));
    assert_eq!(r.by_rule["R1"], count(3, 28), "1, 13 and 23");
    assert_eq!(r.by_rule["R2"], count(1, 12));
    assert_eq!(r.by_hold.len(), 4);
    assert_eq!(r.by_hold["no_fit"], count(1, 4));
    assert_eq!(r.by_hold["gb_family"], count(1, 6));
    assert_eq!(r.by_hold["initialism"], count(1, 12), "24, R2 only");
    assert_eq!(r.by_hold["truncation"], count(1, 12), "25, R2 only");
    assert_eq!(r.by_keep.len(), 6);
    assert_eq!(r.by_keep["D4"], count(4, 42), "4, 20, 21 and 22");
    assert_eq!(r.by_keep["bar"], count(5, 306), "5, 6, 7, 14 and 16");
    assert_eq!(r.by_keep["S1"], count(1, 6));
    assert_eq!(r.by_keep["E1"], count(1, 5));
    assert_eq!(r.by_keep["E2"], count(2, 10), "11 wrong, 12 related unapplied");
    assert_eq!(r.by_keep["E3"], count(1, 3));
    assert_eq!(r.planned_mentions, 72);
    assert_eq!(r.planned_merged, count(1, 17), "1 only: 2's row names itself");
    assert_eq!(r.family_moves.len(), 3);
    assert_eq!(r.family_moves["none->none"], 4);
    assert_eq!(r.family_moves["none->x"], 1, "15 gains Ltd");
    assert_eq!(r.family_moves["x->y"], 1, "23: GmbH -> Ltd, two tables, so not held");
    assert_eq!(r.d4_eng.len(), 4);
    assert_eq!(
        (r.d4_eng["equal"], r.d4_eng["agrees"], r.d4_eng["differs"], r.d4_eng["absent"]),
        (1, 1, 1, 1)
    );
    assert_eq!(r.verdict_probes, 12, "the candidates only: 1, 2, 3, 9, 11, 12, 13, 15, 17, 23, 24, 25");

    assert_eq!(ids(&r.planned), vec![1, 2, 3, 13, 15, 23]);
    assert!(!r.truncated && !r.held_truncated);
    let sheffield = &r.planned[0];
    assert_eq!((sheffield.verdict, sheffield.reason, sheffield.rule), ("change", "R1", Some("R1")));
    assert_eq!(sheffield.old_name, "Sheffield Hallam University");
    assert_eq!(sheffield.old_name_norm.as_deref(), Some("sheffield hallam university"));
    assert_eq!(sheffield.new_name.as_deref(), Some("The University of Sheffield"), "fit, mixed case, then byte order");
    assert_eq!(sheffield.new_name_norm.as_deref(), Some("the university of sheffield"), "name_norm follows");
    assert_eq!(
        (sheffield.voting, sheffield.top, sheffield.head, sheffield.second, sheffield.clusters, sheffield.n),
        (17, 16, 1, 1, 2, 17)
    );
    assert!(sheffield.merged && sheffield.in_population);
    assert_eq!((sheffield.family_old, sheffield.family_new), (None, None));
    assert_eq!(
        sheffield.spellings,
        vec![
            ("UNIVERSITY OF SHEFFIELD".to_owned(), 14),
            ("Sheffield Hallam University".to_owned(), 1),
            ("The University of Sheffield".to_owned(), 1),
            ("University of Sheffield".to_owned(), 1),
        ]
    );
    let so = &r.planned[1];
    assert_eq!((so.reason, so.new_name.as_deref()), ("R0n", Some("Vergabekammer bei der Bezirksregierung")));
    assert!(!so.merged);
    let empty = &r.planned[2];
    assert_eq!((empty.reason, empty.old_name.as_str(), empty.new_name.as_deref()), ("R0e", "", Some("Kalmar kommun")));
    assert_eq!(r.planned[3].new_name.as_deref(), Some("University of Sheffield"));
    let stray = &r.planned[4];
    assert_eq!((stray.reason, stray.new_name.as_deref()), ("R2", Some("Main Body Ltd")));
    assert_eq!((stray.voting, stray.top, stray.head, stray.second), (12, 3, 1, 2));
    assert_eq!((stray.family_old, stray.family_new), (None, Some("ltd")));

    assert_eq!(ids(&r.held), vec![9, 17, 24, 25]);
    let lower = &r.held[0];
    assert_eq!((lower.verdict, lower.reason, lower.rule, lower.new_name.as_deref()), ("hold", "no_fit", Some("R1"), None));
    let plc = &r.held[1];
    assert_eq!((plc.reason, plc.rule, plc.new_name.as_deref()), ("gb_family", Some("R1"), Some("Bar Services Ltd")));
    assert_eq!((plc.family_old, plc.family_new), (Some("plc"), Some("ltd")));
    let cdc = &r.held[2];
    assert_eq!((cdc.reason, cdc.rule, cdc.new_name.as_deref()), ("initialism", Some("R2"), Some("CDC Terroir de Caux")));
    let cut = &r.held[3];
    assert_eq!((cut.reason, cut.rule, cut.new_name.as_deref()), ("truncation", Some("R2"), Some("Tribunal administratif de Nice")));

    // One sample list per verdict and reason, every org of these small classes listed.
    let labels: Vec<&str> = r.samples.keys().map(String::as_str).collect();
    assert_eq!(
        labels,
        vec![
            "change:R0e", "change:R0n", "change:R1", "change:R2", "hold:gb_family", "hold:initialism", "hold:no_fit",
            "hold:truncation", "keep:D4", "keep:E1", "keep:E2", "keep:E3", "keep:S1", "keep:bar"
        ]
    );
    let mut bar = ids(&r.samples["keep:bar"]);
    assert_eq!(bar.len(), 5);
    bar.sort_unstable();
    assert_eq!(bar, vec![5, 6, 7, 14, 16]);
    let d4 = &r.samples["keep:D4"];
    let readings: Vec<(i64, Option<&str>)> = {
        let mut v: Vec<(i64, Option<&str>)> = d4.iter().map(|e| (e.org_id, e.eng)).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(
        readings,
        vec![(4, Some("equal")), (20, Some("agrees")), (21, Some("absent")), (22, Some("differs"))]
    );
    let gmbh = &r.planned[5];
    assert_eq!((gmbh.family_old, gmbh.family_new), (Some("gmbh"), Some("ltd")));
    assert_eq!(r.samples["keep:E1"][0].org_id, 10);
    assert!(r.samples["change:R1"].iter().any(|e| e.org_id == 1 && e.merged), "a sample reads its merge mark");

    // Dry: no organization's (name, name_norm) moved, and no change row was written.
    assert_eq!(heads(&conn).await, before);
    let sheffield = (
        Value::Integer(1),
        Value::Text("Sheffield Hallam University".into()),
        Value::Text("sheffield hallam university".into()),
    );
    assert_eq!(before[0], sheffield);
    assert_eq!(change_rows(&conn).await, Value::Integer(1));
}

#[tokio::test]
async fn the_canaries_are_reported_whatever_their_verdict_and_paging_changes_nothing() {
    let (db, conn) = open("ron-canaries").await;
    corpus(&conn).await;
    // 18 is below the population, 1 a change, 99 not there at all.
    let one_page = plan(&db, 2_000, None, &[18, 1, 99]).await;
    assert_eq!(ids(&one_page.canaries), vec![1, 18], "in walk order; a missing id yields no row");
    let below = &one_page.canaries[1];
    assert_eq!((below.verdict, below.reason, below.in_population, below.n), ("keep", "S1", false, 1));
    assert_eq!(one_page.canaries[0].new_name.as_deref(), Some("The University of Sheffield"));
    assert!(one_page.canaries[0].merged);
    assert_eq!(one_page.population.orgs, 24, "a canary below the population is counted there only");
    // Pages of three: the same plan, field for field — the samples are hash-chosen, not page-chosen.
    let paged = plan(&db, 3, None, &[18, 1, 99]).await;
    assert_eq!(paged, one_page);
}

/// The samples are the `samples` orgs with the smallest `(org_id × 2654435761) mod 2³²` — uniform over
/// the class — not the first by id; and the listing cap cuts `planned`, never the counts.
#[tokio::test]
async fn the_samples_are_hash_chosen_and_the_listing_is_capped() {
    let (db, conn) = open("ron-uniform").await;
    for id in 1..=5i64 {
        org(&conn, id, "DE", Some(format!("DE{id}").as_str()), "-", &[(format!("Firma {id}").as_str(), 2)]).await;
    }
    let r = db
        .plan_org_name_refresh(store::OrgNameRefreshArgs { listing_cap: 3, ..args(2_000, 2, None, &[]) })
        .await
        .unwrap();
    assert_eq!(r.by_rule["R0n"], count(5, 10), "a blank head, R0n");
    let hash = |id: i64| ((id as u64).wrapping_mul(2_654_435_761) & 0xFFFF_FFFF, id);
    let mut expected: Vec<i64> = (1..=5).collect();
    expected.sort_by_key(|&id| hash(id));
    expected.truncate(2);
    assert_eq!(expected, vec![5, 2]);
    assert_eq!(ids(&r.samples["change:R0n"]), expected, "smallest hash first, not 1 and 2");
    assert_eq!(r.samples_per_reason, 2);
    assert_eq!(ids(&r.planned), vec![1, 2, 3]);
    assert!(r.truncated);
    assert_eq!((r.listing_cap, r.planned_mentions), (3, 10), "the counts are whole");
}

/// A scoped plan reads only the named organizations (deduplicated; a missing id reads nothing).
#[tokio::test]
async fn a_scoped_plan_reads_only_its_org_ids() {
    let (db, conn) = open("ron-scope").await;
    corpus(&conn).await;
    let r = plan(&db, 2_000, Some(&[17, 1, 1, 999]), &[]).await;
    assert_eq!(r.scope_ids, Some(3));
    assert_eq!(r.orgs_walked, 2);
    assert_eq!(r.population.orgs, 2);
    assert_eq!(ids(&r.planned), vec![1]);
    assert_eq!(ids(&r.held), vec![17]);
    assert_eq!(r.verdict_probes, 2);
    let whole = plan(&db, 2_000, None, &[]).await;
    assert_eq!(whole.scope_ids, None);
    assert_eq!(r.planned, whole.planned[..1].to_vec(), "the same entry as the whole walk's");
}

#[tokio::test]
async fn the_plan_reads_the_stop_flag_before_every_page_and_a_stopped_plan_reports_nothing() {
    let (db, conn) = open("ron-stop").await;
    corpus(&conn).await;
    let checks = AtomicUsize::new(0);
    let pages = AtomicUsize::new(0);
    // Free for the first page, stopped before the second.
    let stop = || checks.fetch_add(1, Ordering::SeqCst) >= 1;
    let progress = |_: u64, _: &str| {
        pages.fetch_add(1, Ordering::SeqCst);
    };
    let r = db
        .plan_org_name_refresh(store::OrgNameRefreshArgs { stop: &stop, progress: &progress, ..args(5, 30, None, &[1]) })
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!(pages.load(Ordering::SeqCst), 1, "one page walked before the stop was read");
    assert_eq!(checks.load(Ordering::SeqCst), 2);
    assert_eq!(r, store::OrgNameRefreshPlan { stopped: true, ..Default::default() }, "a partial plan is no plan");
}

#[tokio::test]
async fn the_plan_refuses_without_the_mention_index() {
    let (db, conn) = open("ron-refuse").await;
    corpus(&conn).await;
    conn.execute("DROP INDEX organization_mentions_org", ()).await.unwrap();
    let err = db.plan_org_name_refresh(args(2_000, 30, None, &[])).await.expect_err("every chunk would scan");
    assert!(err.to_string().contains("organization_mentions_org") && err.to_string().contains("refused"), "{err}");
}

/// The pure rule, on the shapes the plan reads above, through the public `elect_head`.
#[test]
fn elect_head_is_the_rule_the_plan_runs() {
    let identified = |head| store::HeadElectOrg {
        head,
        identifier_kind: Some("national"),
        identifier: Some("GB1"),
        bind_defect: false,
    };
    let sheffield = [
        ("UNIVERSITY OF SHEFFIELD", 14),
        ("University of Sheffield", 1),
        ("The University of Sheffield", 1),
        ("Sheffield Hallam University", 1),
    ];
    assert_eq!(
        store::elect_head(&sheffield, &identified("Sheffield Hallam University"), &FNS),
        store::HeadVerdict::Change { rule: store::HeadRule::R1, new: "The University of Sheffield".into() }
    );
    // The new head is sticky: the old one would need 55% AND twice the new head's count to flip back.
    assert_eq!(
        store::elect_head(&sheffield, &identified("The University of Sheffield"), &FNS),
        store::HeadVerdict::Keep(store::HeadKeep::S1)
    );
    let tie = [("Main Body", 3), ("Second Body", 3), ("Stray Body", 1)];
    assert_eq!(store::elect_head(&tie, &identified("Stray Body"), &FNS), store::HeadVerdict::Keep(store::HeadKeep::Bar));
    let unnamed = store::HeadElectOrg { identifier: None, identifier_kind: None, ..identified("s.o.") };
    assert_eq!(store::elect_head(&sheffield, &unnamed, &FNS), store::HeadVerdict::Keep(store::HeadKeep::E1));
    assert_eq!(store::HEAD_ELECT_TOP_MIN, 3);
}
