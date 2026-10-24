//! Issue 524 unit 2a: the head ELECTION (`store::elect_head`) and the dry `refresh-org-names` plan.
//!
//! `organizations.name` is the first mention's first-seen spelling, written at mint and never again, so
//! 31558903 (the University of Sheffield) is served as `Sheffield Hallam University` over one stray
//! mention and 22713056 as the pointer `s.o.`. The unit 2 decision re-elects the head of
//! identifier-bearing organizations from their mentions, sticky: no single mention, near tie,
//! abbreviation majority or legal-form difference moves it, and a Latin head over Cyrillic mentions is
//! TED's ENG rendering (ADR-0013 D4). These tests pin each step of the rule on the decision's sample
//! shapes (178, 84/131, 10/301 by their census sample ids, CDC's initialism), the exclusions, the plan's counts,
//! listings, uniform samples and canaries, paging, the scope, the stop flag, the index refusal, and that
//! the plan writes nothing. Unit 2b's wet path and revert close the file: one rename, ledger row and
//! change row per planned head, `name_norm` following, a replay that writes nothing, the drift skip and
//! abort, the identifier-NULL exclusion, the stop and resume, and the revert's compare-and-set.

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
/// consortium mark dropped. A pair read from the two tables (`gmbh` -> `ltd`) is not held. It holds at
/// any H (`head_votes`): rule v13's releases (the family guards at H ≤ 1, a consortium name cut to a
/// member's own words) are crosswalk's and are pinned there; these tests pin the planner around them.
fn hold(old: &str, new: &str, _kind: Option<&str>, _identifier: Option<&str>, _head_votes: u64) -> Option<&'static str> {
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

/// Stand-in for `crosswalk::head_r2_hold` (unit 2a.2): in a new name that is not ALL-CAPS, an
/// upper-case token of 3-5 letters, no legal form, that is the initials of a run of the head's words.
fn r2_hold(old: &str, new: &str) -> Option<&'static str> {
    if new.chars().any(char::is_uppercase) && !new.chars().any(char::is_lowercase) {
        return None;
    }
    let initials: String = old
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|w| w.chars().next())
        .flat_map(char::to_lowercase)
        .collect();
    let abbreviates = new.split(|c: char| !c.is_alphanumeric()).any(|t| {
        (3..=5).contains(&t.chars().count())
            && t.chars().all(|c| c.is_alphabetic() && c.is_uppercase())
            && !matches!(t, "SRL" | "SARL" | "GMBH" | "LTD" | "PLC")
            && initials.contains(&t.to_lowercase())
    });
    abbreviates.then_some("initialism")
}

/// Stand-in for `crosswalk::head_twin_key`: lower-cased, every non-alphanumeric character dropped.
fn twin(name: &str) -> String {
    name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Stand-in for `crosswalk::head_serve_hold` (rule v9): a new name led by `Organ of` the head is not
/// led by is an organ served for its entity.
fn serve_hold(old: &str, new: &str, _r2: bool) -> Option<&'static str> {
    (new.starts_with("Organ of ") && !old.starts_with("Organ of ")).then_some(store::HEAD_HOLD_ORGAN)
}

/// Stand-in for `crosswalk::head_tail_family` (rule v12): the family of the line's last word.
const FNS: store::HeadElectFns =
    store::HeadElectFns { vote_key: vote, not_a_name: class, unfit, hold, r2_hold, twin_key: twin, serve_hold, family };

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
    // 9 — the winning cluster spells only lower case: held, no fit spelling. (The decision read this
    // shape for census sample 334; that org's real mentions also carry `Les Alchimistes SAS`, so it
    // changes by R1, as the supervisor's canary test pins.)
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
    // 17 — plc -> Ltd: R1 elects it, the bind guard holds it (the stand-in's; the real guard releases
    // this H = 1 head since rule v13, crosswalk's `the_v13_listing_rows_release_and_hold_as_decided`).
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
    // the hold lets it through and the plan counts it `x->y`. The H-blind stand-in still holds 17's
    // same-table swap at H = 1, so here only the two tables pass 23; the real guard (rule v13) would
    // release it at H = 1 on either reading, and plans a same-table stray swap as `x->y` too.
    org(conn, 23, "GB", Some("GB00000023"), "Foo Trading GmbH", &[("Bar Services Ltd", 5), ("Foo Trading GmbH", 1)])
        .await;
    // 24 — 15075247's shape: R2 would serve the initialism of the head's first words. Held.
    let caux = "Communauté de communes Terroir de Caux";
    let others = [("Nord", 2), ("Sued", 2), ("Ost", 2), ("West", 2)];
    let mut cdc = vec![("CDC Terroir de Caux", 3), (caux, 1)];
    cdc.extend(others);
    org(conn, 24, "FR", Some("FR24"), caux, &cdc).await;
    // 25 — R2 serves the head cut to its first words. Unit 2a.1 held it as a `truncation`; unit 2a.2
    // dropped that arm (it held right strip-downs), so it changes.
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
    assert_eq!(r.by_rule["R2"], count(2, 24), "15 and 25");
    assert_eq!(r.by_hold.len(), 3);
    assert_eq!(r.by_hold["no_fit"], count(1, 4));
    assert_eq!(r.by_hold["gb_family"], count(1, 6));
    assert_eq!(r.by_hold["initialism"], count(1, 12), "24, R2 only");
    assert_eq!(r.by_keep.len(), 6);
    assert_eq!(r.by_keep["D4"], count(4, 42), "4, 20, 21 and 22");
    assert_eq!(r.by_keep["bar"], count(5, 306), "5, 6, 7, 14 and 16");
    assert_eq!(r.by_keep["S1"], count(1, 6));
    assert_eq!(r.by_keep["E1"], count(1, 5));
    assert_eq!(r.by_keep["E2"], count(2, 10), "11 wrong, 12 related unapplied");
    assert_eq!(r.by_keep["E3"], count(1, 3));
    assert_eq!(r.planned_mentions, 84);
    assert_eq!(r.planned_merged, count(1, 17), "1 only: 2's row names itself");
    assert_eq!(r.family_moves.len(), 3);
    assert_eq!(r.family_moves["none->none"], 5);
    assert_eq!(r.family_moves["none->x"], 1, "15 gains Ltd");
    assert_eq!(r.family_moves["x->y"], 1, "23: GmbH -> Ltd, two tables, so not held (by the H-blind stand-in)");
    assert_eq!(r.d4_eng.len(), 4);
    assert_eq!(
        (r.d4_eng["equal"], r.d4_eng["agrees"], r.d4_eng["differs"], r.d4_eng["absent"]),
        (1, 1, 1, 1)
    );
    assert_eq!(r.verdict_probes, 12, "the candidates only: 1, 2, 3, 9, 11, 12, 13, 15, 17, 23, 24, 25");

    assert_eq!(ids(&r.planned), vec![1, 2, 3, 13, 15, 23, 25]);
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

    assert_eq!(ids(&r.held), vec![9, 17, 24]);
    let lower = &r.held[0];
    assert_eq!((lower.verdict, lower.reason, lower.rule, lower.new_name.as_deref()), ("hold", "no_fit", Some("R1"), None));
    let plc = &r.held[1];
    assert_eq!((plc.reason, plc.rule, plc.new_name.as_deref()), ("gb_family", Some("R1"), Some("Bar Services Ltd")));
    assert_eq!((plc.family_old, plc.family_new), (Some("plc"), Some("ltd")));
    let cdc = &r.held[2];
    assert_eq!((cdc.reason, cdc.rule, cdc.new_name.as_deref()), ("initialism", Some("R2"), Some("CDC Terroir de Caux")));
    let cut = &r.planned[6];
    assert_eq!((cut.reason, cut.new_name.as_deref()), ("R2", Some("Tribunal administratif de Nice")));

    // One sample list per verdict and reason, every org of these small classes listed.
    let labels: Vec<&str> = r.samples.keys().map(String::as_str).collect();
    assert_eq!(
        labels,
        vec![
            "change:R0e", "change:R0n", "change:R1", "change:R2", "hold:gb_family", "hold:initialism", "hold:no_fit",
            "keep:D4", "keep:E1", "keep:E2", "keep:E3", "keep:S1", "keep:bar"
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
    // Rule v10: but for its `as_of`, each plan's own start (the two may straddle a second).
    let paged = plan(&db, 3, None, &[18, 1, 99]).await;
    assert!(paged.as_of >= one_page.as_of && paged.as_of - one_page.as_of < 60, "{} then {}", one_page.as_of, paged.as_of);
    let paged = store::OrgNameRefreshPlan { as_of: one_page.as_of, as_of_date: one_page.as_of_date.clone(), ..paged };
    assert_eq!(paged, one_page);
}

/// The samples are the `samples` orgs with the smallest `((org_id XOR salt) × 2654435761) mod 2³²` —
/// uniform over the class — not the first by id; and the listing cap cuts `planned`, never the counts.
/// Rule v9 salts the hash with the epoch (`ORG_NAME_REFRESH_SAMPLE_SALT`). Over v8's five orgs the
/// salted draw is `[2, 1]` — the first two ids, inside the capped listing `[1, 2, 3]` — so a planner
/// that sampled the oldest ids, or the listing, and ordered them by hash would pass. Eight orgs put
/// the draw at `[8, 7]`, outside both, which keeps the test's point under the salt. Rule v10's epoch
/// drew `[8, 3]` from the eight, so the listing is capped at 2 (`[1, 2]`), outside it again; rule v11's
/// draws `[3, 8]`, still outside it. Rule v12's salt gives id 1 a small hash (it draws `[1, 4]` from ids
/// 1–8, inside the listing), so its eight were ids 2–9: the listing `[2, 3]` and the draw `[9, 4]`. Rule
/// v13's salt drew `[6, 3]` from ids 2–9 (3 inside that listing) and from ids 1–8 alike, so its eight
/// were ids 1–8 again: the listing `[1, 2]` and the draw `[6, 3]`. Rule v14's salt draws `[1, 4]` from ids
/// 1–8 (1 inside that listing), so the eight are ids 2–9 again: the listing is `[2, 3]` and the draw
/// `[4, 7]`, outside it and not in id order.
#[tokio::test]
async fn the_samples_are_hash_chosen_and_the_listing_is_capped() {
    let (db, conn) = open("ron-uniform").await;
    for id in 2..=9i64 {
        org(&conn, id, "DE", Some(format!("DE{id}").as_str()), "-", &[(format!("Firma {id}").as_str(), 2)]).await;
    }
    let r = db
        .plan_org_name_refresh(store::OrgNameRefreshArgs { listing_cap: 2, ..args(2_000, 2, None, &[]) })
        .await
        .unwrap();
    assert_eq!(r.by_rule["R0n"], count(8, 16), "a blank head, R0n");
    let hash = |id: i64| (((id as u64) ^ store::ORG_NAME_REFRESH_SAMPLE_SALT).wrapping_mul(2_654_435_761) & 0xFFFF_FFFF, id);
    let mut expected: Vec<i64> = (2..=9).collect();
    expected.sort_by_key(|&id| hash(id));
    expected.truncate(2);
    assert_eq!(expected, vec![4, 7]);
    assert_eq!(ids(&r.samples["change:R0n"]), expected, "smallest hash first, not 2 and 3 in id order");
    assert_eq!(r.samples_per_reason, 2);
    assert_eq!(ids(&r.planned), vec![2, 3]);
    assert!(
        ids(&r.samples["change:R0n"]).iter().all(|id| !ids(&r.planned).contains(id)),
        "drawn from the whole class, not from the capped listing"
    );
    assert!(r.truncated);
    assert_eq!((r.listing_cap, r.planned_mentions), (2, 16), "the counts are whole");
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
        dated: None,
        // Rule v10: read only with dates.
        as_of: 0,
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

// ---- Issue 524 unit 2b: the wet path (`Db::apply_org_name_refresh`) and its revert.

/// A stored plan's `planned` entries as the wet path takes them (the app reads them off the report),
/// with the plan's `as_of` (rule v10: the wet pre-pass re-elects with the STORED plan's). It reads as
/// the target list.
#[derive(Clone, Default)]
struct Planned {
    targets: Vec<store::OrgNameRefreshTarget>,
    as_of: i64,
}

impl std::ops::Deref for Planned {
    type Target = Vec<store::OrgNameRefreshTarget>;
    fn deref(&self) -> &Self::Target {
        &self.targets
    }
}

impl std::ops::DerefMut for Planned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.targets
    }
}

fn targets(plan: &store::OrgNameRefreshPlan) -> Planned {
    let targets = plan
        .planned
        .iter()
        .map(|e| store::OrgNameRefreshTarget {
            org_id: e.org_id,
            rule: e.rule.expect("a change has a rule").to_owned(),
            old_name: e.old_name.clone(),
            old_name_norm: e.old_name_norm.clone(),
            new_name: e.new_name.clone().expect("a change has a new name"),
        })
        .collect();
    Planned { targets, as_of: plan.as_of }
}

/// The wet run's args for `planned`, re-electing with its plan's `as_of` (rule v10).
fn wet<'a>(planned: Planned, job: i64) -> store::OrgNameRefreshApplyArgs<'a> {
    store::OrgNameRefreshApplyArgs {
        fns: FNS,
        as_of: planned.as_of,
        targets: planned.targets,
        max_groups: None,
        job_id: Some(job),
        resume_after: None,
        window: store::ORG_NAME_REFRESH_WINDOW,
        exclude_released: &[],
        exclude_stale_top: false,
        stop: &never,
        progress: &nowhere,
    }
}

fn revert<'a>(job: i64, by: i64) -> store::OrgNameRefreshRevertArgs<'a> {
    store::OrgNameRefreshRevertArgs {
        job,
        revert_job: Some(by),
        window: store::ORG_NAME_REFRESH_WINDOW,
        stop: &never,
        progress: &nowhere,
    }
}

/// `(org_id, job_id, rule, old_name, old_name_norm, new_name, new_name_norm, reverted_job)` per ledger
/// row, by org id and instant.
async fn ledger(conn: &turso::Connection) -> Vec<Vec<Value>> {
    let mut rows = conn
        .query(
            "SELECT org_id, job_id, rule, old_name, old_name_norm, new_name, new_name_norm, reverted_job
               FROM org_name_refresh_log ORDER BY org_id, at",
            (),
        )
        .await
        .unwrap();
    let mut all = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        all.push((0..8).map(|i| row.get_value(i).unwrap()).collect());
    }
    all
}

/// The seq-less `organization changed` rows, by entity id.
async fn org_changes(conn: &turso::Connection) -> Vec<i64> {
    let mut rows = conn
        .query(
            "SELECT entity_id FROM changes WHERE entity_kind = 'organization' AND op = 'changed' \
             AND version_seq IS NULL ORDER BY entity_id",
            (),
        )
        .await
        .unwrap();
    let mut all = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        all.push(*row.get_value(0).unwrap().as_integer().unwrap());
    }
    all
}

fn text(s: &str) -> Value {
    Value::Text(s.to_owned())
}

/// An organization's newest ledger instant.
async fn latest_at(conn: &turso::Connection, org_id: i64) -> i64 {
    let mut rows =
        conn.query("SELECT MAX(at) FROM org_name_refresh_log WHERE org_id = ?", (Value::Integer(org_id),)).await.unwrap();
    *rows.next().await.unwrap().unwrap().get_value(0).unwrap().as_integer().unwrap()
}

/// Every queued job's `(id, progress)`: the durable resume cursor recovery hands back.
async fn progress_of(db: &store::Db) -> Vec<(i64, Option<String>)> {
    db.pending_jobs().await.unwrap().into_iter().map(|j| (j.id, j.progress)).collect()
}

#[tokio::test]
async fn the_wet_run_renames_each_planned_head_once_and_the_revert_restores_it() {
    let (db, conn) = open("ron-wet").await;
    corpus(&conn).await;
    // The 2b review: the pre-image is the STORED bytes, not a recomputed key. An identifier-bearing row
    // minted before issue 432 keeps a plain lower-cased key (432 re-keyed identifier-less rows only), and
    // `name_norm` is nullable.
    conn.execute("UPDATE organizations SET name_norm = 'sheffield hallam university .' WHERE id = 1", ()).await.unwrap();
    conn.execute("UPDATE organizations SET name_norm = NULL WHERE id = 2", ()).await.unwrap();
    let before = heads(&conn).await;
    let p = plan(&db, 2_000, None, &[]).await;
    let planned = targets(&p);
    assert_eq!(planned.iter().map(|t| t.org_id).collect::<Vec<_>>(), vec![1, 2, 3, 13, 15, 23, 25]);

    let r = db.apply_org_name_refresh(wet(planned.clone(), 7)).await.unwrap();
    assert_eq!((r.targets, r.ready, r.already, r.drifted, r.gone), (7, 7, 0, 0, 0));
    assert_eq!((r.applied, r.raced, r.windows, r.cursor, r.aborted, r.stopped), (7, 0, 1, Some(25), false, false));
    assert_eq!(r.drift_limit, 5, "max(2% of 7, 5)");
    assert_eq!(r.applied_samples.iter().map(|s| s.org_id).collect::<Vec<_>>(), vec![1, 2, 3, 13, 15, 23, 25]);
    assert_eq!(r.applied_samples[1].old_name, "s.o.");

    // Every planned head moved, `name_norm` with it; every other row is byte for byte as it was —
    // the identifier-NULL 10 above all, whose mentions say what 1's say.
    let after = heads(&conn).await;
    for (was, now) in before.iter().zip(&after) {
        let id = *was.0.as_integer().unwrap();
        match planned.iter().find(|t| t.org_id == id) {
            Some(t) => assert_eq!(
                now,
                &(was.0.clone(), text(&t.new_name), text(&store::org_name_norm(&t.new_name))),
                "{id}: the new head, its name_norm following"
            ),
            None => assert_eq!(now, was, "{id} is untouched"),
        }
    }
    assert_eq!(after[0].1, text("The University of Sheffield"));
    assert_eq!(after[9], before[9], "10: identifier NULL, its name_norm the bind key");
    assert_eq!(org_changes(&conn).await, vec![1, 2, 3, 13, 15, 23, 25], "one change row per moved org");
    let rows = ledger(&conn).await;
    assert_eq!(rows.len(), 7);
    assert_eq!(
        rows[0],
        vec![
            Value::Integer(1),
            Value::Integer(7),
            text("R1"),
            text("Sheffield Hallam University"),
            text("sheffield hallam university ."),
            text("The University of Sheffield"),
            text("the university of sheffield"),
            Value::Null,
        ],
        "the byte-exact pre-image beside what was written: the legacy key, not org_name_norm(old)"
    );
    assert_ne!(rows[0][4], text(&store::org_name_norm("Sheffield Hallam University")), "the fixture keeps its legacy key");
    assert_eq!(rows[1][4], Value::Null, "2's NULL name_norm is its pre-image");
    assert_eq!(rows[2][3], text(""), "3's empty head is its pre-image");
    let mut evidence = conn.query("SELECT evidence FROM org_name_refresh_log WHERE org_id = 1", ()).await.unwrap();
    let evidence = evidence.next().await.unwrap().unwrap().get_value(0).unwrap();
    assert_eq!(
        evidence,
        text(&format!(
            "{{\"epoch\":\"{}\",\"as_of\":\"{}\",\"rule\":\"R1\",\"voting\":17,\"top\":16,\"head\":1,\"second\":1,\
             \"clusters\":2,\"head_procs\":0,\"head_in_window\":0,\"head_newest\":null,\"top_newest\":null}}",
            store::HEAD_ELECTION_EPOCH,
            store::rates::civil_date(p.as_of),
        )),
        "rule v10: the plan's as_of, as the pre-pass used it"
    );

    // A replayed run (the same plan, a restart without its cursor) renames nothing and announces
    // nothing: the pre-pass reads every target as already renamed.
    let replay = db.apply_org_name_refresh(wet(planned.clone(), 8)).await.unwrap();
    assert_eq!((replay.ready, replay.already, replay.drifted, replay.applied, replay.windows), (0, 7, 0, 0, 0));
    assert_eq!(heads(&conn).await, after);
    assert_eq!(org_changes(&conn).await.len(), 7);
    assert_eq!(ledger(&conn).await.len(), 7);

    // The revert restores every pre-image, announces each, and stamps the ledger.
    let back = db.revert_org_name_refresh(revert(7, 9)).await.unwrap();
    assert_eq!((back.ledger_rows, back.reverted, back.renamed_again, back.gone, back.windows), (7, 7, 0, 0, 1));
    assert_eq!(heads(&conn).await, before, "the old (name, name_norm) pairs, byte for byte: the legacy key, the NULL");
    assert_eq!(org_changes(&conn).await.len(), 14);
    assert!(ledger(&conn).await.iter().all(|row| row[7] == Value::Integer(9)), "stamped reverted_job");
    let again = db.revert_org_name_refresh(revert(7, 10)).await.unwrap();
    assert_eq!((again.ledger_rows, again.reverted), (0, 0), "nothing left to restore");
    assert_eq!(org_changes(&conn).await.len(), 14);

    // Re-applied, the ledger keeps one row per rename: `at` steps past the latest instant rather than
    // colliding on (org_id, at). Pushed 1,000 s ahead first, so the step is read, not the clock.
    conn.execute("UPDATE org_name_refresh_log SET at = at + 1000", ()).await.unwrap();
    let shifted = latest_at(&conn, 1).await;
    let re = db.apply_org_name_refresh(wet(planned, 11)).await.unwrap();
    assert_eq!(re.applied, 7);
    assert_eq!(heads(&conn).await, after);
    assert_eq!(ledger(&conn).await.len(), 14);
    assert_eq!(latest_at(&conn, 1).await, shifted + 1, "max(now, latest + 1)");
}

#[tokio::test]
async fn the_wet_run_skips_a_drifted_target_and_aborts_past_its_drift_limit() {
    // At the limit: 5 drifted or gone of 7 (max(2%, 5) = 5) still runs, writing only the ready ones —
    // and a target whose row lost its identifier since the plan, or never had one, is drifted.
    let (db, conn) = open("ron-drift").await;
    corpus(&conn).await;
    let mut planned = targets(&plan(&db, 2_000, None, &[]).await);
    conn.execute("UPDATE organizations SET name = 'Somebody Else' WHERE id = 1", ()).await.unwrap();
    // 2's key alone moved: the name is the plan's, the pre-image is not.
    conn.execute("UPDATE organizations SET name_norm = 'moved' WHERE id = 2", ()).await.unwrap();
    conn.execute("UPDATE organizations SET identifier = NULL WHERE id = 13", ()).await.unwrap();
    conn.execute("DELETE FROM organizations WHERE id = 15", ()).await.unwrap();
    // 10 as if a plan had listed it: its row is identifier NULL (E1), and stays as it is.
    planned.push(store::OrgNameRefreshTarget {
        org_id: 10,
        rule: "R1".into(),
        old_name: "Sheffield Hallam University".into(),
        old_name_norm: Some("sheffield hallam university".into()),
        new_name: "University of Sheffield".into(),
    });
    let before = heads(&conn).await;
    let r = db.apply_org_name_refresh(wet(planned.clone(), 7)).await.unwrap();
    assert_eq!((r.targets, r.ready, r.drifted, r.gone, r.drift_limit, r.aborted), (8, 3, 4, 1, 5, false));
    assert_eq!(r.applied, 3, "3, 23 and 25");
    assert_eq!(org_changes(&conn).await, vec![3, 23, 25]);
    let why: Vec<(i64, &str)> = r.drift_samples.iter().map(|d| (d.org_id, d.why.as_str())).collect();
    assert_eq!(why.len(), 5);
    assert!(why[0].1.contains("Somebody Else") && why[0].0 == 1, "{why:?}");
    assert_eq!(why[1], (2, "the name_norm is Some(\"moved\") now, not the plan's Some(\"s.o\")"));
    assert_eq!(why[2], (10, "the identifier is NULL now (E1)"));
    assert_eq!(why[3], (13, "the identifier is NULL now (E1)"));
    assert_eq!(why[4], (15, "the organization is gone"));
    let after = heads(&conn).await;
    assert_eq!(after.iter().find(|h| h.0 == Value::Integer(10)), before.iter().find(|h| h.0 == Value::Integer(10)));
    assert_eq!(after.iter().find(|h| h.0 == Value::Integer(13)), before.iter().find(|h| h.0 == Value::Integer(13)));

    // Past it: one more drifted target aborts the run before any write.
    let (db, conn) = open("ron-abort").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    conn.execute("UPDATE organizations SET name = 'Somebody Else' WHERE id IN (1, 2, 3, 13, 15)", ()).await.unwrap();
    conn.execute("DELETE FROM organizations WHERE id = 23", ()).await.unwrap();
    let before = heads(&conn).await;
    let r = db.apply_org_name_refresh(wet(planned, 7)).await.unwrap();
    assert_eq!((r.ready, r.drifted, r.gone, r.drift_limit, r.aborted), (1, 5, 1, 5, true));
    assert_eq!((r.applied, r.windows), (0, 0));
    assert_eq!(heads(&conn).await, before, "nothing written: 25, still ready, keeps its head");
    assert!(org_changes(&conn).await.is_empty());
    assert!(ledger(&conn).await.is_empty());
}

#[tokio::test]
async fn the_wet_run_reads_the_stop_flag_before_every_window_and_resumes_after_its_cursor() {
    let (db, conn) = open("ron-wet-stop").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    // The job's durable row, and a bystander whose cursor no window may touch.
    db.enqueue_job(7, "refresh-org-names", "refresh-org-names wet max_groups=3", "{}").await.unwrap();
    db.enqueue_job(8, "process", "bystander", "{}").await.unwrap();
    let checks = AtomicUsize::new(0);
    let windows = AtomicUsize::new(0);
    // Free for the first window, stopped before the second.
    let stop = || checks.fetch_add(1, Ordering::SeqCst) >= 1;
    let progress = |_: u64, detail: &str| {
        if detail.starts_with("renaming") {
            windows.fetch_add(1, Ordering::SeqCst);
        }
    };
    let capped = |targets, resume_after| store::OrgNameRefreshApplyArgs {
        max_groups: Some(3),
        resume_after,
        window: 2,
        ..wet(targets, 7)
    };
    let r = db
        .apply_org_name_refresh(store::OrgNameRefreshApplyArgs { stop: &stop, progress: &progress, ..capped(planned.clone(), None) })
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!((r.ready, r.capped, r.applied, r.windows, r.cursor), (7, 4, 2, 1, Some(2)));
    assert_eq!((checks.load(Ordering::SeqCst), windows.load(Ordering::SeqCst)), (2, 1));
    assert_eq!(org_changes(&conn).await, vec![1, 2], "the committed window stands");
    assert_eq!(progress_of(&db).await, vec![(7, Some("2".to_owned())), (8, None)], "the window's last id, durably");

    // The restart resumes after the DURABLE cursor (what recovery hands back as `resume_after`), and
    // `max_groups` counts the renames already made.
    let cursor = progress_of(&db).await[0].1.as_deref().map(|c| c.parse::<i64>().unwrap());
    let r = db.apply_org_name_refresh(capped(planned.clone(), cursor)).await.unwrap();
    assert_eq!((r.resumed_after, r.resumed_past, r.prior_applied), (Some(2), 2, 2));
    assert_eq!((r.ready, r.capped, r.applied, r.cursor), (5, 4, 1, Some(3)));
    assert_eq!(org_changes(&conn).await, vec![1, 2, 3]);
    assert_eq!(progress_of(&db).await, vec![(7, Some("3".to_owned())), (8, None)]);
    // Uncapped, the rest.
    let r = db.apply_org_name_refresh(wet(planned, 7)).await.unwrap();
    assert_eq!((r.already, r.applied, r.prior_applied), (3, 4, 3));
    assert_eq!(org_changes(&conn).await, vec![1, 2, 3, 13, 15, 23, 25]);
    assert!(ledger(&conn).await.iter().all(|row| row[1] == Value::Integer(7)));

    // Refused while the mention index is absent, as the planner is.
    conn.execute("DROP INDEX organization_mentions_org", ()).await.unwrap();
    let err = db.apply_org_name_refresh(wet(Planned::default(), 8)).await.expect_err("the pre-pass would scan");
    assert!(err.to_string().contains("organization_mentions_org") && err.to_string().contains("refused"), "{err}");
}

/// The v7 follow-up check: the pre-pass reads the stop flag between its 512-target chunks, and a
/// cancel there reports how far it read — no pre-pass counts, the whole run's limit — writes nothing
/// and leaves a recovered job's durable cursor where its last window put it.
#[tokio::test]
async fn a_cancel_during_the_pre_pass_reports_how_far_it_read_and_writes_nothing() {
    let (db, conn) = open("ron-wet-prepass-stop").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    db.enqueue_job(7, "refresh-org-names", "refresh-org-names wet", "{}").await.unwrap();
    // An earlier window of job 7 renamed org 1 and moved the cursor to it.
    let capped = store::OrgNameRefreshApplyArgs { max_groups: Some(1), ..wet(planned.clone(), 7) };
    let r = db.apply_org_name_refresh(capped).await.unwrap();
    assert_eq!((r.applied, r.cursor), (1, Some(1)));
    assert_eq!(progress_of(&db).await, vec![(7, Some("1".to_owned()))]);
    let (changes, log) = (org_changes(&conn).await, ledger(&conn).await);
    assert_eq!(changes, vec![1]);

    // The plan's 7 plus 600 targets past them: 606 after the cursor, so a second chunk exists.
    let mut many = planned;
    many.extend((0..600).map(|i| store::OrgNameRefreshTarget {
        org_id: 1_000_000 + i,
        rule: "R1".to_owned(),
        old_name: format!("Org {i}"),
        old_name_norm: Some(format!("org {i}")),
        new_name: format!("Organisation {i}"),
    }));
    let checks = AtomicUsize::new(0);
    let windows = AtomicUsize::new(0);
    // Stopped at the first read, which the pre-pass makes before its second chunk.
    let stop = || {
        checks.fetch_add(1, Ordering::SeqCst);
        true
    };
    let progress = |_: u64, detail: &str| {
        if detail.starts_with("renaming") {
            windows.fetch_add(1, Ordering::SeqCst);
        }
    };
    let r = db
        .apply_org_name_refresh(store::OrgNameRefreshApplyArgs {
            resume_after: Some(1),
            stop: &stop,
            progress: &progress,
            ..wet(many, 7)
        })
        .await
        .unwrap();
    assert!(r.stopped && !r.aborted);
    assert_eq!((r.targets, r.resumed_past, r.resumed_after, r.cursor, r.prior_applied), (607, 1, Some(1), Some(1), 1));
    assert_eq!(r.prepass_stopped_after, Some(512), "the first chunk, of 606");
    assert_eq!(r.drift_limit, store::org_name_refresh_drift_limit(606), "the whole run's limit, never 0");
    assert_eq!((r.ready, r.already, r.drifted, r.gone, r.capped), (0, 0, 0, 0, 0), "no pre-pass counts");
    assert_eq!((r.applied, r.raced, r.windows, r.left), (0, 0, 0, 0));
    assert!(r.drift_samples.is_empty() && r.applied_samples.is_empty() && r.window_error.is_none());
    assert_eq!((checks.load(Ordering::SeqCst), windows.load(Ordering::SeqCst)), (1, 0));
    assert_eq!(org_changes(&conn).await, changes, "nothing written");
    assert_eq!(ledger(&conn).await, log);
    assert_eq!(progress_of(&db).await, vec![(7, Some("1".to_owned()))], "the durable cursor is unchanged");
}

#[tokio::test]
async fn the_revert_leaves_a_head_renamed_since_and_counts_a_gone_one() {
    let (db, conn) = open("ron-revert").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    let r = db.apply_org_name_refresh(wet(planned, 7)).await.unwrap();
    assert_eq!(r.applied, 7);
    // After the run: 2 renamed again (a later job), 23's key alone moved, 3 merged away, and a case
    // review stripped 13's identifier, making its name_norm the issue-234 bind key.
    conn.execute("UPDATE organizations SET name = 'Vergabekammer Detmold', name_norm = 'vergabekammer detmold' WHERE id = 2", ())
        .await
        .unwrap();
    conn.execute("UPDATE organizations SET name_norm = 'moved' WHERE id = 23", ()).await.unwrap();
    conn.execute("DELETE FROM organizations WHERE id = 3", ()).await.unwrap();
    conn.execute("UPDATE organizations SET identifier = NULL, identifier_kind = NULL WHERE id = 13", ()).await.unwrap();
    // 25 renamed again AND stripped: renamed again (the re-review) — restoring its identifier would not
    // let a re-run take it up, so it is no `identifier_stripped`.
    conn.execute(
        "UPDATE organizations SET name = 'Tribunal administratif de Nice (greffe)', identifier = NULL WHERE id = 25",
        (),
    )
    .await
    .unwrap();
    let stripped_after_apply = heads(&conn).await.into_iter().find(|h| h.0 == Value::Integer(13)).unwrap();
    let back = db.revert_org_name_refresh(revert(7, 9)).await.unwrap();
    assert_eq!(
        (back.ledger_rows, back.reverted, back.renamed_again, back.identifier_stripped, back.gone),
        (7, 2, 3, 1, 1)
    );
    let now = heads(&conn).await;
    let at = |id: i64| now.iter().find(|h| h.0 == Value::Integer(id)).unwrap().clone();
    assert_eq!(at(2).1, text("Vergabekammer Detmold"), "left as found");
    assert_eq!(at(23).2, text("moved"), "a key moved alone is renamed again: left as found");
    assert_eq!(at(13), stripped_after_apply, "identifier stripped: byte-identical to after the run");
    assert_eq!(at(13).1, text("University of Sheffield"));
    assert_eq!(at(1).1, text("Sheffield Hallam University"));
    assert_eq!(at(25).1, text("Tribunal administratif de Nice (greffe)"), "left as found");
    assert_eq!(org_changes(&conn).await.len(), 7 + 2, "a change row per restored head only");
    let unreverted: Vec<Value> =
        ledger(&conn).await.into_iter().filter(|row| row[7] == Value::Null).map(|row| row[0].clone()).collect();
    assert_eq!(unreverted, [2, 3, 13, 23, 25].map(Value::Integer).to_vec(), "none of the misses is stamped");
    // A job with no ledger rows restores nothing.
    let none = db.revert_org_name_refresh(revert(42, 9)).await.unwrap();
    assert_eq!((none.ledger_rows, none.reverted, none.windows), (0, 0, 0));
}

#[test]
fn the_drift_limit_is_two_percent_of_the_targets_read_with_a_floor_of_five() {
    for (read, limit) in [(0, 5), (7, 5), (299, 5), (300, 6), (349, 6), (350, 7), (1_049, 20), (29_000, 580)] {
        assert_eq!(store::org_name_refresh_drift_limit(read), limit, "{read} targets read");
    }
}

#[tokio::test]
async fn a_recovered_capped_run_counts_its_earlier_renames_without_a_cursor() {
    // The 2b review: a capped job that died after its window committed but (before the cursor moved into
    // the window's transaction) before its cursor landed came back with no `resume_after`, counted 0
    // prior renames and applied `max_groups` again.
    let (db, conn) = open("ron-recovered").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    let capped = || store::OrgNameRefreshApplyArgs { max_groups: Some(1), ..wet(planned.clone(), 9) };
    let r = db.apply_org_name_refresh(capped()).await.unwrap();
    assert_eq!((r.prior_applied, r.ready, r.capped, r.applied), (0, 7, 6, 1));
    let r = db.apply_org_name_refresh(capped()).await.unwrap();
    assert_eq!((r.resumed_after, r.prior_applied, r.already, r.ready, r.capped, r.applied), (None, 1, 1, 6, 6, 0));
    assert_eq!(ledger(&conn).await.len(), 1, "still the one rename the operator capped the run at");
    assert_eq!(org_changes(&conn).await, vec![1]);
}

#[tokio::test]
async fn a_row_changed_after_the_pre_pass_fails_its_compare_and_set_and_writes_nothing() {
    // Each row breaks ONE predicate of the compare-and-set, inside the window's transaction, after the
    // pre-pass read all seven as ready: the trigger fires on 1's ledger row, the window's first.
    let (db, conn) = open("ron-raced").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    conn.execute(
        "CREATE TRIGGER t524r AFTER INSERT ON org_name_refresh_log WHEN NEW.org_id = 1 BEGIN
           UPDATE organizations SET name = 'Somebody Else' WHERE id = 2;
           UPDATE organizations SET identifier = NULL WHERE id = 13;
           UPDATE organizations SET name_norm = 'drifted' WHERE id = 23;
         END",
        (),
    )
    .await
    .unwrap();
    let before = heads(&conn).await;
    let r = db.apply_org_name_refresh(wet(planned, 7)).await.unwrap();
    assert_eq!((r.ready, r.drifted, r.applied, r.raced, r.windows), (7, 0, 4, 3, 1));
    assert_eq!(org_changes(&conn).await, vec![1, 3, 15, 25], "no change row for a raced org");
    let ids: Vec<Value> = ledger(&conn).await.into_iter().map(|row| row[0].clone()).collect();
    assert_eq!(ids, [1, 3, 15, 25].map(Value::Integer).to_vec(), "no ledger row for a raced org");
    let now = heads(&conn).await;
    let at = |rows: &[(Value, Value, Value)], id: i64| rows.iter().find(|h| h.0 == Value::Integer(id)).unwrap().clone();
    assert_eq!(at(&now, 2), (Value::Integer(2), text("Somebody Else"), at(&before, 2).2), "name: what the trigger left");
    assert_eq!(at(&now, 13), at(&before, 13), "identifier stripped: the head and key stay");
    assert_eq!(at(&now, 23), (Value::Integer(23), at(&before, 23).1, text("drifted")), "name_norm: what the trigger left");
}

#[tokio::test]
async fn a_window_that_fails_after_a_commit_reports_the_committed_windows() {
    // The 513 U4 fold_error shape: window 2 (3 and 13) fails after window 1 (1 and 2) committed. The run
    // comes back Ok with window 1's counts, cursor and samples and the error; window 2 rolled back whole.
    let (db, conn) = open("ron-window-error").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    db.enqueue_job(7, "refresh-org-names", "refresh-org-names wet", "{}").await.unwrap();
    let refuse = |id: i64, table: &str, op: &str| {
        format!(
            "CREATE TRIGGER t524_{id} BEFORE {op} ON {table} WHEN NEW.org_id = {id}
             BEGIN SELECT RAISE(ABORT, 'fixture: the ledger refuses {id}'); END"
        )
    };
    conn.execute(&refuse(13, "org_name_refresh_log", "INSERT"), ()).await.unwrap();
    let windowed = |targets, job| store::OrgNameRefreshApplyArgs { window: 2, ..wet(targets, job) };
    let r = db.apply_org_name_refresh(windowed(planned.clone(), 7)).await.unwrap();
    assert_eq!((r.windows, r.applied, r.cursor, r.stopped), (1, 2, Some(2), false));
    assert_eq!((r.ready, r.left, r.capped), (7, 5, 0), "3 and 13 rolled back, 15, 23 and 25 never reached");
    let error = r.window_error.as_deref().expect("the failure is reported");
    assert!(error.contains("window 2 (org ids 3..=13)") && error.contains("fixture: the ledger refuses 13"), "{error}");
    assert_eq!(r.applied_samples.iter().map(|s| s.org_id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(org_changes(&conn).await, vec![1, 2], "window 2 rolled back: 3 was not renamed");
    assert_eq!(db.pending_jobs().await.unwrap()[0].progress.as_deref(), Some("2"), "the cursor committed with window 1");

    // The revert has the same shape: window 2 of its rows (13 and 15, once the rest is applied) fails.
    conn.execute("DROP TRIGGER t524_13", ()).await.unwrap();
    assert_eq!(db.apply_org_name_refresh(wet(planned.clone(), 7)).await.unwrap().applied, 5);
    conn.execute(&refuse(13, "org_name_refresh_log", "UPDATE"), ()).await.unwrap();
    let back = db
        .revert_org_name_refresh(store::OrgNameRefreshRevertArgs { window: 2, ..revert(7, 9) })
        .await
        .unwrap();
    assert_eq!((back.windows, back.reverted), (1, 2));
    assert!(back.window_error.as_deref().is_some_and(|e| e.contains("fixture: the ledger refuses 13")), "{back:?}");

    // A failure in the FIRST window is the error itself: nothing was written.
    let (db, conn) = open("ron-window-error-first").await;
    corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    conn.execute(&refuse(1, "org_name_refresh_log", "INSERT"), ()).await.unwrap();
    let before = heads(&conn).await;
    let err = db.apply_org_name_refresh(windowed(planned.clone(), 7)).await.expect_err("nothing committed");
    assert!(err.to_string().contains("fixture: the ledger refuses 1"), "{err}");
    assert_eq!(heads(&conn).await, before);
    assert!(org_changes(&conn).await.is_empty() && ledger(&conn).await.is_empty());
    assert_eq!(db.org_name_refresh_job_renames(7).await.unwrap(), 0);

    // But a RECOVERED job's first window of this run failing is no bare error (the re-review): job 8
    // renamed 1 before, so the report comes back with `prior_applied` and the error.
    conn.execute("DROP TRIGGER t524_1", ()).await.unwrap();
    let one = store::OrgNameRefreshApplyArgs { max_groups: Some(1), ..windowed(planned.clone(), 8) };
    assert_eq!(db.apply_org_name_refresh(one).await.unwrap().applied, 1);
    assert_eq!(db.org_name_refresh_job_renames(8).await.unwrap(), 1);
    conn.execute(&refuse(2, "org_name_refresh_log", "INSERT"), ()).await.unwrap();
    let r = db.apply_org_name_refresh(windowed(planned, 8)).await.expect("the earlier attempt's rename stands");
    assert_eq!((r.prior_applied, r.windows, r.applied, r.left), (1, 0, 0, 6));
    assert!(r.window_error.as_deref().is_some_and(|e| e.contains("window 1 (org ids 2..=3)")), "{r:?}");
}

// ---- Issue 524 rule v7: the dated planner and wet pre-pass (the recency decision's tests 13 and 14).

const DAY: i64 = 86_400;
/// 2016-07-18: the dated fixtures' first day. Rule v10 moved it from 2023-12-09 (day 19,700): the
/// planner's `as_of` is the clock, and a head the fixtures print within 3 years of it holds
/// `head_current`, so the v7–v9 shapes sit more than 3 years back, where they keep their meaning. The
/// v10 fixtures date themselves from the clock.
const BASE: i64 = 17_000;

/// An identifier-bearing org served as `head`, with one mention per `(spelling, versions)`, each its
/// own notice (`id * 1000 + i`), and that notice's `tender_versions` rows `(tender, published_at)`.
async fn dated_org(conn: &turso::Connection, id: i64, head: &str, mentions: &[(&str, &[(i64, i64)])]) {
    org(conn, id, "DE", Some(&format!("DE{id}")), head, &[]).await;
    for (i, (name, versions)) in mentions.iter().enumerate() {
        let notice = id * 1000 + i as i64;
        conn.execute(
            "INSERT INTO organization_mentions
               (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
             VALUES (?, 'ORG-0001', ?, ?, 'DE', NULL, NULL)",
            vec![Value::Integer(notice), Value::Integer(id), Value::Text((*name).into())],
        )
        .await
        .unwrap();
        for &(tender, at) in *versions {
            conn.execute(
                "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
                 VALUES (?, ?, ?, ?, 'x')",
                vec![Value::Integer(tender), Value::Integer(notice), Value::Integer(notice), Value::Integer(at)],
            )
            .await
            .unwrap();
        }
    }
}

/// `n` mentions of `name`, each in its own tender from `tender` on, a day apart from `day` on.
fn spread(name: &'static str, tender: i64, day: i64, n: i64) -> Vec<(&'static str, Vec<(i64, i64)>)> {
    (0..n).map(|i| (name, vec![(tender + i, (day + i) * DAY)])).collect()
}

/// The dated corpus: 40 a rename (held `recent`), 41 a forward change, 42 the date read's edges, 43 an
/// H = 1 stray that is its org's newest item (31558903's shape: R1 with no window, held `residue` since
/// rule v8), 44 an empty head (R0e), 45 an H = 1 stray printed newest whose head adds a pier number
/// (the junk exemption releases its residue hold: it ships). 43, 44 and 45 are no `hold:undated`
/// before their dates, so only a planner that dates EVERY candidate gives their entries a reading
/// (the decision rejected dating only the undated).
async fn dated_corpus(conn: &turso::Connection) {
    let mut forty = spread("Old Name Agency", 4_000, BASE, 8);
    forty.extend(spread("New Name Agency", 4_100, BASE + 100, 3));
    let mut forty_one = spread("New Name Office", 4_200, BASE, 20);
    forty_one.extend(spread("Old Name Office", 4_250, BASE + 5, 2));
    let mut forty_two = spread("Bright Services", 4_300, BASE, 7);
    // A future-dated elected mention: no date, counted undated.
    forty_two.push(("Bright Services", vec![(4_310, crate_now() + 30 * DAY)]));
    // A notice in TWO tenders (an issue-278 ghost) keeps the smaller; a second head notice in that
    // tender is then the same procedure: k = 1 (`head_procs` pins it). Kept as the larger tender, k
    // would be 2 — under rule v8's newer elected item below no longer a pair of newest items, so only
    // the count tells the two apart.
    forty_two.push(("Dim Services", vec![(4_390, (BASE + 100) * DAY), (4_380, (BASE + 100) * DAY)]));
    forty_two.push(("Dim Services", vec![(4_380, (BASE + 101) * DAY)]));
    // A head notice dated 0: no date, counted undated.
    forty_two.push(("Dim Services", vec![(4_395, 0)]));
    // The newest item is an elected one, so rule v8's residue holdout leaves 42 a change.
    forty_two.push(("Bright Services", vec![(4_320, (BASE + 200) * DAY)]));
    let mut forty_three = spread("Main Office Group", 4_400, BASE, 5);
    forty_three.extend(spread("Stray Office", 4_450, BASE + 300, 1));
    let forty_four = spread("Kalmar Office", 4_500, BASE, 3);
    let mut forty_five = spread("Harbour Works", 4_600, BASE, 5);
    forty_five.extend(spread("Harbour Works Pier 17", 4_650, BASE + 300, 1));
    for (id, head, rows) in [
        (40, "New Name Agency", forty),
        (41, "Old Name Office", forty_one),
        (42, "Dim Services", forty_two),
        (43, "Stray Office", forty_three),
        (44, "", forty_four),
        (45, "Harbour Works Pier 17", forty_five),
    ] {
        let mentions: Vec<(&str, &[(i64, i64)])> = rows.iter().map(|(name, v)| (*name, v.as_slice())).collect();
        dated_org(conn, id, head, &mentions).await;
    }
}

fn crate_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

#[tokio::test]
async fn the_planner_dates_its_candidates_and_holds_a_rename() {
    let (db, conn) = open("ron-dated").await;
    dated_corpus(&conn).await;
    let r = plan(&db, 2_000, None, &[]).await;
    assert_eq!(r.epoch, "524u2-v14");
    assert_eq!(r.by_hold.get("undated"), None, "a finished plan holds nothing undated");
    assert_eq!(r.by_hold.get("head_current"), None, "every head printed 3+ years before the plan");
    assert_eq!(r.by_hold["recent"], count(1, 11), "40, the rename");
    assert_eq!(r.by_hold["residue"], count(1, 6), "43, the newest stray (rule v8)");
    assert_eq!(ids(&r.held), vec![40, 43]);
    let held = &r.held[0];
    assert_eq!((held.reason, held.rule, held.new_name.as_deref()), ("recent", Some("R1"), Some("Old Name Agency")));
    assert_eq!((held.head_procs, held.head_in_window, held.newest_is_head), (Some(3), Some(3), Some(true)));
    // 43: H = 1, never `hold:undated`, dated all the same: its single newest head mention is the
    // residue, held with the spelling R1 would serve.
    let stray = &r.held[1];
    assert_eq!((stray.reason, stray.rule, stray.new_name.as_deref()), ("residue", Some("R1"), Some("Main Office Group")));
    assert_eq!((stray.head, stray.head_procs, stray.newest_is_head, stray.undated_mentions), (1, Some(1), Some(true), Some(0)));
    assert_eq!(stray.recent_released, None);
    assert!(r.samples.contains_key("hold:residue"));
    assert_eq!(ids(&r.planned), vec![41, 42, 44, 45]);
    let forward = &r.planned[0];
    assert_eq!((forward.head_procs, forward.head_in_window, forward.newest_is_head), (Some(2), Some(0), Some(false)));
    assert_eq!(forward.head_newest, Some(store::rates::civil_date((BASE + 6) * DAY)));
    assert_eq!(forward.top_newest, Some(store::rates::civil_date((BASE + 19) * DAY)));
    assert_eq!((forward.undated_mentions, forward.recent_released), (Some(0), None));
    // 42: the ghost's smaller tender makes its two head notices one procedure; the future-dated and
    // the zero-dated mentions are undated.
    let edges = &r.planned[1];
    assert_eq!((edges.head, edges.head_procs, edges.undated_mentions), (3, Some(1), Some(2)));
    assert_eq!(edges.newest_is_head, Some(false));
    // 44: R0e never reads the window, and is dated all the same.
    let empty = &r.planned[2];
    assert_eq!((empty.reason, empty.head_procs, empty.newest_is_head), ("R0e", Some(0), Some(false)));
    // 45: the residue holdout's reading, released by the junk exemption (`17`, a number the elected
    // key lacks): it ships, and is the residue the plan still counts and samples.
    let pier = &r.planned[3];
    assert_eq!((pier.reason, pier.head, pier.newest_is_head, pier.recent_released), ("R1", 1, Some(true), Some("digits")));
    assert_eq!((r.recent_released.len(), r.recent_released["digits"], r.recent_released_ids.clone()), (1, 1, vec![45]));
    assert_eq!((r.residue.len(), r.residue["R1"]), (1, count(1, 6)));
    assert_eq!(ids(&r.samples["residue:head_newest"]), vec![45]);
    assert_eq!((r.dated_orgs, r.undated_mentions), (6, 2));
    assert_eq!(r.dated_notices, 11 + 22 + 12 + 6 + 3 + 6, "every voting mention's notice of the six candidates");
    assert_eq!(r.dated_version_rows, 11 + 22 + 13 + 6 + 3 + 6, "42's ghost notice reads two rows");
    assert!(r.samples.contains_key("hold:recent"));

    // Refused without the date read's index, as the wet path is.
    conn.execute("DROP INDEX tender_versions_notice", ()).await.unwrap();
    let err = db.plan_org_name_refresh(args(2_000, 30, None, &[])).await.expect_err("the date read would scan");
    assert!(err.to_string().contains("tender_versions_notice is absent (a rebuild's fold?)"), "{err}");
    let err = db.apply_org_name_refresh(wet(targets(&r), 7)).await.expect_err("so would the pre-pass's");
    assert!(err.to_string().contains("tender_versions_notice is absent (a rebuild's fold?)"), "{err}");
}

#[tokio::test]
async fn a_target_whose_head_gained_newer_procedures_reads_drifted() {
    let (db, conn) = open("ron-dated-wet").await;
    dated_corpus(&conn).await;
    let planned = targets(&plan(&db, 2_000, None, &[]).await);
    assert_eq!(planned.iter().map(|t| t.org_id).collect::<Vec<_>>(), vec![41, 42, 44, 45]);
    // A head mention in a new procedure published after the plan.
    let newer = |org: i64, name: &'static str, notice: i64, tender: i64, day: i64| {
        let conn = &conn;
        async move {
            conn.execute(
                "INSERT INTO organization_mentions
                   (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
                 VALUES (?, 'ORG-0001', ?, ?, 'DE', NULL, NULL)",
                vec![Value::Integer(notice), Value::Integer(org), Value::Text(name.into())],
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
                 VALUES (?, 1, ?, ?, 'x')",
                vec![Value::Integer(tender), Value::Integer(notice), Value::Integer(day * DAY)],
            )
            .await
            .unwrap();
        }
    };
    // 41's head names four new procedures, its newest: the window now fires (`hold:recent`).
    for i in 0..4 {
        newer(41, "Old Name Office", 41_900 + i, 4_900 + i, BASE + 200 + i).await;
    }
    // Rule v8: 42's head names one new procedure, the organization's newest item. k = 2 with one head
    // item among the newest two (h_W = 1) is under the window's bar, so it reads `hold:residue`.
    newer(42, "Dim Services", 42_900, 4_399, BASE + 300).await;
    let r = db.apply_org_name_refresh(wet(planned, 7)).await.unwrap();
    assert_eq!((r.ready, r.drifted, r.applied), (2, 2, 2));
    let why: Vec<(i64, &str)> = r.drift_samples.iter().map(|d| (d.org_id, d.why.as_str())).collect();
    assert!(why[0].0 == 41 && why[0].1.contains("re-elected hold:recent"), "{why:?}");
    assert!(why[1].0 == 42 && why[1].1.contains(r#"re-elected hold:residue "Bright Services""#), "{why:?}");
    assert_eq!(org_changes(&conn).await, vec![44, 45]);
    // The ledger's evidence carries the window's reading.
    let mut rows = conn.query("SELECT evidence FROM org_name_refresh_log WHERE org_id = 45", ()).await.unwrap();
    let evidence = rows.next().await.unwrap().unwrap().get_value(0).unwrap();
    let evidence = evidence.as_text().unwrap();
    assert!(evidence.contains("\"head_procs\":1,\"head_in_window\":1"), "{evidence}");
    assert!(evidence.contains(&format!("\"head_newest\":\"{}\"", store::rates::civil_date((BASE + 300) * DAY))), "{evidence}");
}

/// Rule v9 in the planner: a serve hold lands in `by_hold`, `held` and the samples with the spelling
/// it would serve; a `digits` release v9 withholds (`glue`, `junk_only`) leaves the org under its v8
/// hold, its entry reading `release_withheld` and the plan counting it in `release_withheld` /
/// `release_withheld_ids`; a release the two tests let through still ships, counted as before.
#[tokio::test]
async fn the_planner_counts_the_serve_holds_and_the_withheld_releases() {
    let (db, conn) = open("ron-v9").await;
    // 50: an H = 1 stray printed newest whose head adds a pier number AND a dock and a road the elected
    // name lacks: more than one lost word, so v9 withholds v8's release (`junk_only`).
    let mut fifty = spread("Harbour Works", 5_000, BASE, 5);
    fifty.extend(spread("Harbour Works Pier 17 Dock Road", 5_050, BASE + 300, 1));
    // 51: the head's number is glued to its next word in the elected key (`3 Dent` → `3dent`): `glue`.
    let mut fifty_one = spread("Studio 3dent", 5_100, BASE, 5);
    fifty_one.extend(spread("Studio 3 Dent", 5_150, BASE + 300, 1));
    // 52: one lost word (`pier`): still released, it ships.
    let mut fifty_two = spread("Harbour Office", 5_200, BASE, 5);
    fifty_two.extend(spread("Harbour Office Pier 17", 5_250, BASE + 300, 1));
    // 53: an organ for its entity, the elected cluster newest (no date hold): the serve hold.
    let mut fifty_three = spread("Acme Works", 5_300, BASE, 1);
    fifty_three.extend(spread("Organ of Acme Works", 5_350, BASE + 10, 5));
    for (id, head, rows) in [
        (50, "Harbour Works Pier 17 Dock Road", fifty),
        (51, "Studio 3 Dent", fifty_one),
        (52, "Harbour Office Pier 17", fifty_two),
        (53, "Acme Works", fifty_three),
    ] {
        let mentions: Vec<(&str, &[(i64, i64)])> = rows.iter().map(|(name, v)| (*name, v.as_slice())).collect();
        dated_org(&conn, id, head, &mentions).await;
    }
    let r = plan(&db, 2_000, None, &[]).await;
    assert_eq!(r.epoch, "524u2-v14");
    assert_eq!(r.by_hold["residue"], count(2, 12), "50 and 51 stay held under their v8 reason");
    assert_eq!(r.by_hold[store::HEAD_HOLD_ORGAN], count(1, 6), "53");
    assert_eq!(ids(&r.held), vec![50, 51, 53]);
    let (junk, glue, organ) = (&r.held[0], &r.held[1], &r.held[2]);
    assert_eq!((junk.reason, junk.release_withheld, junk.recent_released), ("residue", Some("junk_only"), None));
    assert_eq!(junk.new_name.as_deref(), Some("Harbour Works"));
    assert_eq!((glue.reason, glue.release_withheld, glue.new_name.as_deref()), ("residue", Some("glue"), Some("Studio 3dent")));
    assert_eq!((organ.reason, organ.rule, organ.new_name.as_deref()), ("organ", Some("R1"), Some("Organ of Acme Works")));
    assert_eq!((organ.newest_is_head, organ.release_withheld), (Some(false), None));
    assert_eq!(r.release_withheld.len(), 2);
    assert_eq!((r.release_withheld["glue"], r.release_withheld["junk_only"]), (1, 1));
    assert_eq!(r.release_withheld_ids, vec![50, 51]);
    assert_eq!(ids(&r.planned), vec![52]);
    assert_eq!((r.planned[0].recent_released, r.planned[0].release_withheld), (Some("digits"), None));
    assert_eq!((r.recent_released.len(), r.recent_released["digits"], r.recent_released_ids.clone()), (1, 1, vec![52]));
    assert!(r.samples.contains_key("hold:organ") && r.samples.contains_key("hold:residue"));
}

// ---- Issue 524 rule v10: the current-head hold, the plan's `as_of`, and the wet pre-pass's.

/// Rule v10 in the planner: the plan stores its start as `as_of` (and `as_of_date`), and an R1 change
/// on a head printed within 3 years of it is `hold:head_current` — in `by_hold`, `held` and the
/// samples, with the spelling it would serve. The same shape on a head last printed more than 3 years
/// back changes, and a current head the junk exemption releases ships, counted `recent_released`.
#[tokio::test]
async fn the_planner_holds_a_current_head_and_stores_its_as_of() {
    let (db, conn) = open("ron-v10").await;
    let today = crate_now() / DAY;
    // 60: the head printed 400 and 399 days ago, the elected cluster newer: no window, no residue.
    let mut sixty = spread("Acme Works Office", 6_000, today - 400, 2);
    sixty.extend(spread("Acme Group Office", 6_050, today - 300, 5));
    // 61: the same shape 2,000 days back: the head is outdated, and R1 changes it.
    let mut sixty_one = spread("Beta Works Office", 6_100, today - 2_000, 2);
    sixty_one.extend(spread("Beta Group Office", 6_150, today - 1_900, 5));
    // 62: a current H = 1 head that adds a pier number to the elected name (one lost word): junk.
    let mut sixty_two = spread("Harbour Office", 6_200, today - 300, 5);
    sixty_two.extend(spread("Harbour Office Pier 17", 6_250, today - 400, 1));
    for (id, head, rows) in [
        (60, "Acme Works Office", sixty),
        (61, "Beta Works Office", sixty_one),
        (62, "Harbour Office Pier 17", sixty_two),
    ] {
        let mentions: Vec<(&str, &[(i64, i64)])> = rows.iter().map(|(name, v)| (*name, v.as_slice())).collect();
        dated_org(&conn, id, head, &mentions).await;
    }
    let before = crate_now();
    let r = plan(&db, 2_000, None, &[]).await;
    let after = crate_now();
    assert_eq!(r.epoch, "524u2-v14");
    assert!(before <= r.as_of && r.as_of <= after, "as_of is the planner's start: {before} <= {} <= {after}", r.as_of);
    assert_eq!(r.as_of_date, store::rates::civil_date(r.as_of));
    assert_eq!(r.by_hold[store::HEAD_HOLD_CURRENT], count(1, 7), "60");
    assert_eq!(r.by_hold.len(), 1);
    assert_eq!(ids(&r.held), vec![60]);
    let held = &r.held[0];
    assert_eq!((held.verdict, held.reason, held.rule, held.new_name.as_deref()), ("hold", "head_current", Some("R1"), Some("Acme Group Office")));
    assert_eq!(held.head_newest, Some(store::rates::civil_date((today - 399) * DAY)));
    assert_eq!((held.newest_is_head, held.recent_released, held.release_withheld), (Some(false), None, None));
    assert_eq!(ids(&r.samples["hold:head_current"]), vec![60], "sampled per reason like every hold");
    assert_eq!(ids(&r.planned), vec![61, 62]);
    assert_eq!((r.planned[0].reason, r.planned[0].recent_released), ("R1", None));
    assert_eq!((r.planned[1].reason, r.planned[1].recent_released), ("R1", Some("digits")));
    assert_eq!(r.recent_released_ids, vec![62]);
    // The wet run takes the plan's `as_of`: both planned changes are ready.
    let w = db.apply_org_name_refresh(wet(targets(&r), 7)).await.unwrap();
    assert_eq!((w.ready, w.drifted, w.applied), (2, 0, 2));
}

/// Rule v10 at wet time: the pre-pass re-elects with the STORED plan's `as_of`, so a target whose head
/// was printed after the plan (and whose elected cluster printed after that, so no `recent` or
/// `residue` reads it) is `hold:head_current` now and drifts. The same pre-pass handed an `as_of`
/// more than 3 years past that print reads it ready — the pre-pass reads `as_of`, not the clock —
/// and the ledger's `evidence` names the `as_of` it re-elected with.
#[tokio::test]
async fn a_head_printed_after_the_plan_reads_head_current_at_wet_time() {
    let (db, conn) = open("ron-v10-wet").await;
    let mut seventy = spread("Old Works Office", 7_000, BASE, 1);
    seventy.extend(spread("New Works Office", 7_050, BASE + 10, 8));
    let mentions: Vec<(&str, &[(i64, i64)])> = seventy.iter().map(|(name, v)| (*name, v.as_slice())).collect();
    dated_org(&conn, 70, "Old Works Office", &mentions).await;
    let p = plan(&db, 2_000, None, &[]).await;
    let planned = targets(&p);
    assert_eq!(planned.iter().map(|t| (t.org_id, t.new_name.as_str())).collect::<Vec<_>>(), vec![(70, "New Works Office")]);
    // After the plan: the head printed once more, the elected cluster once after it.
    for (notice, tender, name, at) in [(70_900, 7_900, "Old Works Office", p.as_of + 10), (70_901, 7_901, "New Works Office", p.as_of + 20)] {
        conn.execute(
            "INSERT INTO organization_mentions
               (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
             VALUES (?, 'ORG-0001', 70, ?, 'DE', NULL, NULL)",
            vec![Value::Integer(notice), Value::Text(name.into())],
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (?, 1, ?, ?, 'x')",
            vec![Value::Integer(tender), Value::Integer(notice), Value::Integer(at)],
        )
        .await
        .unwrap();
    }
    let r = db.apply_org_name_refresh(wet(planned.clone(), 7)).await.unwrap();
    assert_eq!((r.ready, r.drifted, r.applied), (0, 1, 0));
    let why = &r.drift_samples[0];
    assert_eq!(why.org_id, 70);
    assert!(
        why.why.contains(r#"re-elected hold:head_current "New Works Office", not the plan's change:R1 "New Works Office""#),
        "{why:?}"
    );
    assert!(org_changes(&conn).await.is_empty());
    // An `as_of` 2,000 days on: that print is no longer current, and the target is ready.
    let later = p.as_of + 2_000 * DAY;
    let r = db.apply_org_name_refresh(store::OrgNameRefreshApplyArgs { as_of: later, ..wet(planned, 8) }).await.unwrap();
    assert_eq!((r.ready, r.drifted, r.applied), (1, 0, 1));
    let mut rows = conn.query("SELECT evidence FROM org_name_refresh_log WHERE org_id = 70", ()).await.unwrap();
    let evidence = rows.next().await.unwrap().unwrap().get_value(0).unwrap();
    let evidence = evidence.as_text().unwrap();
    assert!(evidence.contains(&format!("\"as_of\":\"{}\"", store::rates::civil_date(later))), "{evidence}");
}

// ---- Issue 524 rule v11: the stale-top hold in the planner, `rival_newest`, and `exclude_stale_top`
// at wet time.

/// Rule v11 in the planner: an R1 change on a stale top (the elected cluster last printed 801 days
/// before the plan) with a rival cluster is `hold:top_stale` with the spelling it would serve; the same
/// shape with no rival, or with a fresh top, is planned. Every dated entry carries `rival_newest`, the
/// rival's newest print (a diagnostic). At wet time `exclude_stale_top` drifts the stale no-rival
/// target (the caller drops it from the plan first; the pre-pass holds the filter if it did not), and
/// a run without it renames that same target.
#[tokio::test]
async fn the_planner_holds_a_stale_top_with_a_rival_and_the_wet_run_can_exclude_a_stale_top() {
    let (db, conn) = open("ron-v11").await;
    let today = crate_now() / DAY;
    // 80: a stale top and a rival — held `top_stale`.
    let mut eighty = spread("Head Office Eighty", 8_000, today - 4_000, 1);
    eighty.extend(spread("Rival Office Eighty", 8_010, today - 3_000, 2));
    eighty.extend(spread("Elected Office Eighty", 8_050, today - 805, 5));
    // 81: a stale top and no rival (two clusters, H = 1) — planned: the stale no-rival cell.
    let mut eighty_one = spread("Head Office Eighty One", 8_100, today - 4_000, 1);
    eighty_one.extend(spread("Elected Office Eighty One", 8_150, today - 805, 5));
    // 82: a fresh top and a rival — planned.
    let mut eighty_two = spread("Head Office Eighty Two", 8_200, today - 4_000, 1);
    eighty_two.extend(spread("Rival Office Eighty Two", 8_210, today - 3_000, 2));
    eighty_two.extend(spread("Elected Office Eighty Two", 8_250, today - 100, 5));
    for (id, head, rows) in [
        (80, "Head Office Eighty", eighty),
        (81, "Head Office Eighty One", eighty_one),
        (82, "Head Office Eighty Two", eighty_two),
    ] {
        let mentions: Vec<(&str, &[(i64, i64)])> = rows.iter().map(|(name, v)| (*name, v.as_slice())).collect();
        dated_org(&conn, id, head, &mentions).await;
    }
    let r = plan(&db, 2_000, None, &[]).await;
    assert_eq!(r.epoch, "524u2-v14");
    assert_eq!(r.by_hold[store::HEAD_HOLD_TOP_STALE], count(1, 8), "80");
    assert_eq!(r.by_hold.len(), 1);
    assert_eq!(ids(&r.held), vec![80]);
    let held = &r.held[0];
    assert_eq!(
        (held.verdict, held.reason, held.rule, held.new_name.as_deref()),
        ("hold", "top_stale", Some("R1"), Some("Elected Office Eighty"))
    );
    assert_eq!((held.clusters, held.head), (3, 1));
    assert_eq!(held.top_newest, Some(store::rates::civil_date((today - 801) * DAY)));
    assert_eq!(held.rival_newest, Some(store::rates::civil_date((today - 2_999) * DAY)), "the rival's newest print");
    assert_eq!((held.recent_released, held.release_withheld), (None, None), "no release is minted");
    assert_eq!(ids(&r.samples["hold:top_stale"]), vec![80], "sampled per reason like every hold");
    assert_eq!(ids(&r.planned), vec![81, 82]);
    assert_eq!((r.planned[0].reason, r.planned[0].rival_newest.as_deref()), ("R1", None), "81: no rival");
    assert_eq!(r.planned[1].rival_newest, Some(store::rates::civil_date((today - 2_999) * DAY)), "82");
    assert!(store::head_top_stale_date(r.planned[0].top_newest.as_deref(), r.as_of), "81's top is stale");
    assert!(!store::head_top_stale_date(r.planned[1].top_newest.as_deref(), r.as_of), "82's is fresh");
    // Wet with `exclude_stale_top`: 81 drifts, 82 is renamed.
    let filtered = store::OrgNameRefreshApplyArgs { exclude_stale_top: true, ..wet(targets(&r), 7) };
    let w = db.apply_org_name_refresh(filtered).await.unwrap();
    assert_eq!((w.ready, w.drifted, w.applied), (1, 1, 1));
    let why = &w.drift_samples[0];
    assert_eq!(why.org_id, 81);
    assert!(why.why.contains("exclude_stale_top") && why.why.contains(&store::rates::civil_date((today - 801) * DAY)), "{why:?}");
    assert_eq!(org_changes(&conn).await, vec![82]);
    // Without it, the same target is renamed (82 reads already renamed).
    let w = db.apply_org_name_refresh(wet(targets(&r), 8)).await.unwrap();
    assert_eq!((w.ready, w.already, w.drifted, w.applied), (1, 1, 0, 1));
    assert_eq!(org_changes(&conn).await, vec![81, 82]);
}

// ---- Issue 524 rule v12 (issue 552): the lines guard and the wrap collapse, planned and applied.

/// Rule v12 end to end: a control-only head the lines guard stops and whose break wraps is planned
/// `R0w` (its own words, the break collapsed; `release_withheld: lines` on the entry, counted in
/// `by_rule.R0w` and not in `release_withheld`); one whose break follows a legal form stays under its
/// hold, counted `release_withheld.lines`; one the guard passes is released `unfit` as before. The wet
/// run applies the R0w change under its own rule, and the re-plan HOLDS the written head on the same
/// votes (the durability test, through the store): it is fit, and its vote key is the raw head's.
#[tokio::test]
async fn the_planner_plans_a_wrap_collapse_and_the_written_head_holds() {
    let (db, conn) = open("ron-v12").await;
    let today = crate_now() / DAY;
    // Each head printed 400 and 399 days ago (within 3 years: head_current), the elected cluster newer.
    let shape = |head: &'static str, elected: &'static str, tender: i64| {
        let mut rows = spread(head, tender, today - 400, 2);
        rows.extend(spread(elected, tender + 50, today - 300, 5));
        rows
    };
    let wraps = "Rhine Port Chamber\nat the Ministry of Transport";
    let branch = "Delta Works GmbH\nBranch Office North";
    let street = "Echo Services\nHigh Street 5";
    for (id, head, rows) in [
        (90, wraps, shape(wraps, "Port Chamber Ministry of Transport", 9_000)),
        (91, branch, shape(branch, "Delta Works Office North", 9_100)),
        (92, street, shape(street, "Echo Services", 9_200)),
    ] {
        let mentions: Vec<(&str, &[(i64, i64)])> = rows.iter().map(|(name, v)| (*name, v.as_slice())).collect();
        dated_org(&conn, id, head, &mentions).await;
    }
    let r = plan(&db, 2_000, None, &[]).await;
    assert_eq!(r.epoch, "524u2-v14");
    assert_eq!(ids(&r.planned), vec![90, 92]);
    let (collapse, released) = (&r.planned[0], &r.planned[1]);
    assert_eq!(
        (collapse.verdict, collapse.reason, collapse.rule, collapse.new_name.as_deref()),
        ("change", "R0w", Some("R0w"), Some("Rhine Port Chamber at the Ministry of Transport"))
    );
    assert_eq!((collapse.recent_released, collapse.release_withheld), (None, Some(store::HEAD_WITHHELD_LINES)));
    assert_eq!(collapse.new_name_norm.as_deref(), Some(store::org_name_norm(wraps).as_str()), "name_norm does not move");
    assert_eq!((released.reason, released.new_name.as_deref(), released.recent_released), ("R1", Some("Echo Services"), Some("unfit")));
    assert_eq!(r.by_rule["R0w"], count(1, 7));
    assert_eq!(r.by_rule["R1"], count(1, 7));
    assert_eq!(ids(&r.held), vec![91]);
    let held = &r.held[0];
    assert_eq!((held.reason, held.new_name.as_deref(), held.release_withheld), ("head_current", Some("Delta Works Office North"), Some("lines")));
    assert_eq!((r.release_withheld.len(), r.release_withheld["lines"], r.release_withheld_ids.clone()), (1, 1, vec![91]));
    assert_eq!((r.recent_released.len(), r.recent_released["unfit"], r.recent_released_ids.clone()), (1, 1, vec![92]));
    assert_eq!(ids(&r.samples["change:R0w"]), vec![90], "sampled per rule like every change");
    // Wet: both planned changes apply, the collapse under its own rule.
    let w = db.apply_org_name_refresh(wet(targets(&r), 7)).await.unwrap();
    assert_eq!((w.ready, w.drifted, w.applied), (2, 0, 2));
    let rows = ledger(&conn).await;
    assert_eq!((rows[0][0].clone(), rows[0][2].clone(), rows[0][5].clone()), (Value::Integer(90), text("R0w"), text("Rhine Port Chamber at the Ministry of Transport")));
    assert_eq!(org_changes(&conn).await, vec![90, 92]);
    // The re-plan, on the same votes: the written head holds (head_current), never a change back or on.
    let again = plan(&db, 2_000, None, &[]).await;
    assert!(again.planned.is_empty(), "{:?}", ids(&again.planned));
    assert_eq!(ids(&again.held), vec![90, 91]);
    let written = &again.held[0];
    assert_eq!(
        (written.old_name.as_str(), written.reason, written.rule, written.new_name.as_deref()),
        ("Rhine Port Chamber at the Ministry of Transport", "head_current", Some("R1"), Some("Port Chamber Ministry of Transport"))
    );
    assert_eq!((written.recent_released, written.release_withheld), (None, None));
    assert_eq!(again.release_withheld["lines"], 1, "91 only");
    assert!(!again.by_rule.contains_key("R0w"));
}

// ---- Issue 573 (rule v14): the dropped-accent repair, planned and applied.

/// Rule v14 end to end: a DROPPED-ACCENT head printed within 3 years of the plan (22441088's shape) is planned
/// `change:R0d` to the accented top spelling, where v13 held it `head_current`; an ordinary rename of the same
/// shape stays `hold:head_current`. The wet run applies it under its own rule, and the re-plan keeps the written
/// head (its top cluster's spelling: `keep:S1`).
#[tokio::test]
async fn a_dropped_accent_head_is_planned_r0d_past_head_current_and_the_written_head_keeps() {
    let (db, conn) = open("ron-v14").await;
    let today = crate_now() / DAY;
    let (head, elected) = ("Tribunal administratif d'Orlans", "Tribunal administratif d'Orléans");
    // 95: the head printed 400 and 399 days ago, the accented cluster 9 mentions newer.
    let mut ninety_five = spread(head, 9_500, today - 400, 2);
    ninety_five.extend(spread(elected, 9_550, today - 300, 9));
    // 96: the control, test 60's shape.
    let mut ninety_six = spread("Acme Works Office", 9_600, today - 400, 2);
    ninety_six.extend(spread("Acme Group Office", 9_650, today - 300, 5));
    for (id, h, rows) in [(95, head, ninety_five), (96, "Acme Works Office", ninety_six)] {
        let mentions: Vec<(&str, &[(i64, i64)])> = rows.iter().map(|(name, v)| (*name, v.as_slice())).collect();
        dated_org(&conn, id, h, &mentions).await;
    }
    let r = plan(&db, 2_000, None, &[]).await;
    assert_eq!(r.epoch, "524u2-v14");
    assert_eq!(ids(&r.planned), vec![95]);
    let e = &r.planned[0];
    assert_eq!((e.verdict, e.reason, e.rule, e.new_name.as_deref()), ("change", "R0d", Some("R0d"), Some(elected)));
    assert_eq!((e.voting, e.top, e.head, e.recent_released, e.release_withheld), (11, 9, 2, None, None));
    assert_eq!(r.by_rule.len(), 1);
    assert_eq!(r.by_rule["R0d"], count(1, 11));
    assert_eq!(ids(&r.held), vec![96]);
    assert_eq!(r.by_hold[store::HEAD_HOLD_CURRENT], count(1, 7));
    assert_eq!(ids(&r.samples["change:R0d"]), vec![95], "sampled per rule like every change");
    let w = db.apply_org_name_refresh(wet(targets(&r), 9)).await.unwrap();
    assert_eq!((w.ready, w.drifted, w.applied), (1, 0, 1));
    let rows = ledger(&conn).await;
    assert_eq!((rows[0][0].clone(), rows[0][2].clone(), rows[0][5].clone()), (Value::Integer(95), text("R0d"), text(elected)));
    assert_eq!(org_changes(&conn).await, vec![95]);
    let again = plan(&db, 2_000, Some(&[95]), &[]).await;
    assert!(again.planned.is_empty() && again.held.is_empty(), "{:?} {:?}", ids(&again.planned), ids(&again.held));
    assert_eq!(again.by_keep["S1"], count(1, 11));
}
