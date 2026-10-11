//! Issue 448: `match_org_altid_pairs`, the `altid` rule — the Companies House ↔
//! PPON pairing FTS parties publish in `additionalIdentifiers`, which the fold
//! drops. Unit 1 is its DRY planner; unit 2 its WET run (set parity against the
//! stored plan, merges of the reviewed pairs, `e2-altid` edges for the denied).
//!
//! The injected rules are test-local miniatures in the production fn-pointer
//! SHAPE (the r3_merge.rs convention: store cannot depend on ingest). So these
//! tests pin the store's harvest, graph, owner map, gate order and merge
//! machinery; the REAL rule content — `altid_pair_key`, `mention_key`,
//! `altid_name_key`, `gb_legal_family` — is pinned by `ingest::crosswalk`'s own
//! tests, and the production wiring end to end by the supervisor's
//! `an_altid_wet_run_merges_the_stored_plan`, which plans and merges a real FTS
//! pair through the real fns.
//!
//! Fixtures are built the way the parser and the fold leave them: a notice
//! under profile `fts:ocds-1.1`, one `ORG-…` section per party, its identifiers
//! as `BT-501-Organization-Company` rows at ordinal 0, 1, … with the
//! publisher's scheme (`GB-COH-03914810` under `GB-COH`), and the fold's one
//! mention per party, bound to the org its FIRST identifier keys to and
//! carrying that identifier as `raw_identifier`.

use std::sync::atomic::{AtomicUsize, Ordering};

use store::turso::{Connection, Value};

// ---- The miniature rules.

/// The GB arm in small: `GBPPON` + 12 → PPON; `GBCOH` (or a bare `GB`) then
/// eight characters → company number at E1, six or seven digits → padded, E2.
fn gb_key(value: &str) -> Option<(&'static str, String, bool, bool)> {
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    if let Some(serial) = norm.strip_prefix("GBPPON") {
        return (serial.len() == 12).then(|| ("GB:ppon", serial.to_owned(), true, false));
    }
    let coh = norm.strip_prefix("GBCOH").or_else(|| norm.strip_prefix("GB")).unwrap_or(&norm);
    let digits = !coh.is_empty() && coh.bytes().all(|b| b.is_ascii_digit());
    match coh.len() {
        8 if digits
            || (coh[..2].bytes().all(|b| b.is_ascii_alphabetic())
                && coh[2..].bytes().all(|b| b.is_ascii_digit())) =>
        {
            Some(("GB:coh", coh.to_owned(), true, false))
        }
        6 | 7 if digits => Some(("GB:coh", format!("{coh:0>8}"), false, false)),
        _ => None,
    }
}

/// The v2 gate in small: a run of eight nines is a placeholder.
fn condemns(_country: Option<&str>, _kind: &str, value: &str) -> bool {
    value.contains("99999999")
}

/// `crosswalk::mention_key` in small: the gate first (the normaliser's), then
/// the GB arm, under GB only.
fn mention_key(country: Option<&str>, raw: &str) -> Option<(&'static str, String, bool, bool)> {
    if country != Some("GB") || condemns(country, "national", raw) {
        return None;
    }
    gb_key(raw)
}

/// `crosswalk::altid_pair_key` in small: the scheme names the series.
fn pair_key(scheme: &str, value: &str, country: Option<&str>) -> Option<(&'static str, String, bool, bool)> {
    let series = match scheme {
        "GB-COH" => "GB:coh",
        "GB-PPON" => "GB:ppon",
        _ => return None,
    };
    mention_key(country, value).filter(|k| k.0 == series)
}

/// `crosswalk::canonical_key_flat` in small, for the standing orgs. A VAT kind
/// keys nothing — which is what makes the R2/R3 two-letter-lead inference blind
/// to FTS raws, and what `the_evidence_wall_keys_fts_raws` relies on.
fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool, bool)> {
    if kind != "national" {
        return None;
    }
    mention_key(country, value)
}

fn norm(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `crosswalk::altid_trim` in small: a trading-as clause is cut.
fn trim(name: &str) -> String {
    match name.to_ascii_lowercase().find(" t/a ") {
        Some(at) => name[..at].to_owned(),
        None => name.to_owned(),
    }
}

fn name_key(name: &str) -> String {
    norm(&trim(name))
        .split(' ')
        .filter(|t| !t.is_empty() && *t != "the" && *t != "and")
        .map(|t| match t {
            "ltd" | "limited" => "§ltd",
            "plc" => "§plc",
            "llp" => "§llp",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `crosswalk::altid_keys_agree` in small: equal, or equal but for a legal form
/// only one side carries.
fn names_agree(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let strip = |key: &str| {
        let mut tokens: Vec<&str> = key.split(' ').collect();
        let mut formed = false;
        while tokens.last().is_some_and(|t| t.starts_with('§')) {
            tokens.pop();
            formed = true;
        }
        (tokens.join(" "), formed)
    };
    let ((a, a_formed), (b, b_formed)) = (strip(a), strip(b));
    a_formed != b_formed && !a.is_empty() && a == b
}

fn legal_family(name: &str) -> Option<&'static str> {
    norm(name)
        .split(' ')
        .filter_map(|t| match t {
            "ltd" | "limited" => Some("ltd"),
            "plc" => Some("plc"),
            "llp" => Some("llp"),
            _ => None,
        })
        .last()
}

/// `project::normalise_identifier` in small: the uppercase alphanumerics, minted
/// only from four characters up and with a digit among them — so a letter-only
/// NHS code (`GB-NHS-QWO`) mints none and its mention binds by name.
fn register_identity(raw: &str, country: Option<&str>) -> Option<store::Identifier> {
    let value = minted(raw);
    (value.len() >= 4 && value.chars().any(|c| c.is_ascii_digit())).then(|| store::Identifier {
        country: country.map(str::to_owned),
        kind: "national".into(),
        value,
    })
}

fn consortium(name: &str) -> bool {
    norm(name).split(' ').any(|t| t == "consortium")
}

/// `crosswalk::registry_mention_key` in small (issue 469 U4): under GB only and never a
/// condemned value, a minted value led by one of the five register tokens with a digit
/// after it keys to that register's series — as the identity `register_identity` mints.
fn registry_mention_key(country: Option<&str>, raw: &str) -> Option<(&'static str, String)> {
    if country != Some("GB") || condemns(country, "national", raw) {
        return None;
    }
    let value = minted(raw);
    [("GBUKPRN", "GB:ukprn"), ("GBCHC", "GB:chc"), ("GBMPR", "GB:mpr"), ("GBSC", "GB:sc"), ("GBNIC", "GB:nic")]
        .iter()
        .find_map(|(token, series)| {
            value.strip_prefix(token).filter(|n| n.chars().any(|c| c.is_ascii_digit())).map(|_| (*series, value.clone()))
        })
}

/// `crosswalk::registry_pair_key` in small: the publisher's scheme names the series, and
/// only the five registers key (NHS waits for U7).
fn registry_pair_key(scheme: &str, literal: &str, country: Option<&str>) -> Option<(&'static str, String)> {
    let series = match scheme {
        "GB-UKPRN" => "GB:ukprn",
        "GB-CHC" => "GB:chc",
        "GB-MPR" => "GB:mpr",
        "GB-SC" => "GB:sc",
        "GB-NIC" => "GB:nic",
        _ => return None,
    };
    registry_mention_key(country, literal).filter(|(s, _)| *s == series)
}

fn args<'a>(stoplist_cap: usize) -> store::AltIdMergeArgs<'a> {
    store::AltIdMergeArgs {
        pair_key,
        key,
        mention_key,
        register_identity,
        registry_pair_key,
        registry_mention_key,
        condemns,
        consortium,
        legal_family,
        name_key,
        names_agree,
        trim,
        norm,
        stoplist_cap,
        plan_listing_cap: store::ALTID_PLAN_LISTING_CAP,
        dry_run: true,
        max_pairs: None,
        expect_pairs: None,
        known_deferred: Vec::new(),
        expect_registry_pairs: Vec::new(),
        known_registry_deferred: Vec::new(),
        job_id: Some(77),
        stop: &|| false,
    }
}

/// A wet run held against `expect` (the stored plan's `pairs`).
fn wet_args<'a>(expect: &[String]) -> store::AltIdMergeArgs<'a> {
    store::AltIdMergeArgs { dry_run: false, expect_pairs: Some(expect.to_vec()), ..args(20) }
}

/// Issue 469 U5: a wet run held against both stored sets — `pairs` and
/// `registry_pairs_planned`.
fn wet_all_args<'a>(expect: &[String], expect_registry: &[String]) -> store::AltIdMergeArgs<'a> {
    store::AltIdMergeArgs { expect_registry_pairs: expect_registry.to_vec(), ..wet_args(expect) }
}

/// Issue 469 U5 (D1): a cohort under the register-confirmed prefix — the ledger's
/// `confirmed` reads `register` for it.
const CONFIRMED: &str = "altid-registry-confirmed-test";

/// Issue 469 U4/U5: every register split is a register candidate, every candidate lands
/// in exactly one place, and the planned keys are the planned count — on a dry plan and,
/// since U5 re-plans live, on a wet run too.
fn assert_registry_accounting(r: &store::AltIdMergeReport) {
    for (scheme, c) in &r.registry_pairs {
        let g = r.registry_gates.get(scheme).cloned().unwrap_or_default();
        assert_eq!(c.both_distinct, g.candidates, "{scheme}: {r:#?}");
        assert_eq!(g.candidates, g.planned + g.denied() + g.conflicts, "{scheme}: {g:#?}");
    }
    assert!(r.registry_gates.keys().all(|s| r.registry_pairs.contains_key(s)), "{r:#?}");
    assert_eq!(
        r.registry_pairs_planned.len() as u64,
        r.registry_gates.values().map(|g| g.planned).sum::<u64>(),
        "{r:#?}"
    );
}

/// Issue 513 unit 1, on every dry plan: the GB-NHS listing is one entry per pair unit 1
/// counted, in harvest order (value, then PPON key), its classes are the counts' classes
/// and its null identities are `no_identity` — the listing carries the loop's decision.
fn assert_nhs_listing_matches_counts(r: &store::AltIdMergeReport) {
    let c = r.registry_pairs.get("GB-NHS").cloned().unwrap_or_default();
    let l = &r.registry_nhs_listing;
    assert!(!r.registry_nhs_listing_truncated, "{r:#?}");
    assert_eq!(l.len() as u64, c.pairs, "{r:#?}");
    let class = |name: &str| l.iter().filter(|e| e.class == name).count() as u64;
    assert_eq!(
        [
            class("already_one"),
            class("both_distinct"),
            class("no_target_registry"),
            class("no_target_ppon"),
            class("no_target_both"),
            class("multi_target"),
        ],
        [c.already_one, c.both_distinct, c.no_target_registry, c.no_target_ppon, c.no_target_both, c.multi_target],
        "{r:#?}"
    );
    assert_eq!(l.iter().filter(|e| e.identity.is_none()).count() as u64, c.no_identity, "{r:#?}");
    assert!(l.windows(2).all(|w| (&w[0].value, &w[0].ppon) < (&w[1].value, &w[1].ppon)), "{r:#?}");
}

/// A pair's stored-plan key, from the literals.
fn pk(coh: &str, ppon: &str) -> String {
    format!("{}~{}", k(coh), k(ppon))
}

// ---- The literals: five company numbers, four PPONs.

const COH_A: &str = "GB-COH-03914810";
const COH_B: &str = "GB-COH-SC305103";
const COH_C: &str = "GB-COH-07495895";
const COH_D: &str = "GB-COH-NI012345";
const COH_E: &str = "GB-COH-OC301234";
const PPON_P: &str = "GB-PPON-PHDQ-2359-NZMP";
const PPON_Q: &str = "GB-PPON-PBZB-4962-TVLR";
const PPON_R: &str = "GB-PPON-PDTR-3338-MNPG";
const PPON_S: &str = "GB-PPON-PYDR-3797-LZLJ";
const PPON_T: &str = "GB-PPON-PTTT-1111-AAAA";
const PPON_U: &str = "GB-PPON-PUUU-2222-BBBB";

// ---- Issue 469 U4: register literals, each minting its register's identity.
const UKPRN_A: &str = "GB-UKPRN-10007798";
const UKPRN_B: &str = "GB-UKPRN-10001111";
const UKPRN_C: &str = "GB-UKPRN-10002222";
const CHC_X: &str = "GB-CHC-1044521";
const CHC_Y: &str = "GB-CHC-207043";
const CHC_Z: &str = "GB-CHC-202918";
const MPR_M: &str = "GB-MPR-30938R";
const MPR_N: &str = "GB-MPR-7522";

/// A literal's stored org identifier, the way the resolver mints it: the
/// normaliser's uppercase alphanumerics.
fn minted(literal: &str) -> String {
    literal.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

/// A literal's key, for asserting on listings.
fn k(literal: &str) -> String {
    gb_key(literal).expect("fixture literal keys").1
}

/// The publisher's scheme of an FTS literal (`GB-UKPRN-10007798` → `GB-UKPRN`).
fn scheme_of(literal: &str) -> String {
    literal.splitn(3, '-').take(2).collect::<Vec<_>>().join("-")
}

/// Issue 469 U4: a register pair's verdict key, from the literals —
/// `<scheme>:<identity>~<ppon>`.
fn rk(reg: &str, ppon: &str) -> String {
    format!("{}:{}~{}", scheme_of(reg), minted(reg), k(ppon))
}

struct Bed {
    db: store::Db,
    conn: Connection,
    path: String,
}

async fn bed(name: &str) -> Bed {
    let path = format!("/tmp/tender-db-altid-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
    // The parsed layer's parents (fetches, sections) are not what is under
    // test; the r3_merge.rs precedent.
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    Bed { db, conn, path }
}

impl Drop for Bed {
    fn drop(&mut self) {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{s}", self.path));
        }
    }
}

impl Bed {
    /// A standing GB org keyed by `literal` (minted as the resolver would).
    async fn org(&self, id: i64, literal: &str, name: &str) {
        self.org_in(id, "GB", &minted(literal), name).await;
    }

    /// A provisional GB org with no identifier: what a mention binds by name.
    async fn provisional(&self, id: i64, name: &str) {
        self.conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, 'GB', NULL, NULL, ?, ?, 1, 0)",
                (Value::Integer(id), Value::Text(name.into()), Value::Text(name.to_lowercase())),
            )
            .await
            .unwrap();
    }

    async fn org_in(&self, id: i64, country: &str, identifier: &str, name: &str) {
        self.conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, ?, 'national', ?, ?, ?, 0, 0)",
                (
                    Value::Integer(id),
                    Value::Text(country.into()),
                    Value::Text(identifier.into()),
                    Value::Text(name.into()),
                    Value::Text(name.to_lowercase()),
                ),
            )
            .await
            .unwrap();
    }

    async fn satellite(&self, org: i64, lang: &str, name: &str) {
        self.conn
            .execute(
                "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (?, ?, ?, ?)",
                (Value::Integer(org), Value::Text(lang.into()), Value::Text(name.into()), Value::Text(name.to_lowercase())),
            )
            .await
            .unwrap();
    }

    async fn notice(&self, id: i64, profile: &str) {
        self.conn
            .execute(
                "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at, parse_state)
                 VALUES (?, 'fts', ?, 'h', ?, 1, 'm', 0, 'parsed')",
                (Value::Integer(id), Value::Text(format!("pub-{id}")), Value::Text(profile.into())),
            )
            .await
            .unwrap();
    }

    /// One FTS party on `notice`: its section, its BT-501 rows in order, and —
    /// when `bound_to` is given — the fold's mention, bound to that org and
    /// carrying the FIRST identifier, under `country`. The party publishes the
    /// bound org's head name, the way the fold elected it.
    async fn party(&self, notice: i64, party: &str, bound_to: Option<i64>, country: &str, ids: &[&str]) {
        let name = match bound_to {
            Some(org) => self.head(org).await,
            None => "x".to_owned(),
        };
        self.party_named(notice, party, bound_to, country, ids, &name).await;
    }

    async fn head(&self, org: i64) -> String {
        let mut rows = self
            .conn
            .query("SELECT name FROM organizations WHERE id = ?", (Value::Integer(org),))
            .await
            .unwrap();
        match rows.next().await.unwrap() {
            Some(row) => match row.get_value(0).unwrap() {
                Value::Text(name) => name,
                _ => "x".to_owned(),
            },
            None => "x".to_owned(),
        }
    }

    /// [`Self::party`] publishing `name`, whatever the bound org is called.
    async fn party_named(
        &self,
        notice: i64,
        party: &str,
        bound_to: Option<i64>,
        country: &str,
        ids: &[&str],
        name: &str,
    ) {
        let section = format!("ORG-{party}");
        self.conn
            .execute(
                "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
                 VALUES (?, ?, 'Organization', 'PROCEDURE')",
                (Value::Integer(notice), Value::Text(section.clone())),
            )
            .await
            .unwrap();
        for (ordinal, literal) in ids.iter().enumerate() {
            // The FTS form is `<scheme>-<id>`, and the scheme is two parts.
            let scheme = literal.splitn(3, '-').take(2).collect::<Vec<_>>().join("-");
            self.conn
                .execute(
                    "INSERT INTO notice_ids (notice_id, section_id, field_id, ordinal, scheme, value, is_ref)
                     VALUES (?, ?, 'BT-501-Organization-Company', ?, ?, ?, 0)",
                    (
                        Value::Integer(notice),
                        Value::Text(section.clone()),
                        Value::Integer(ordinal as i64),
                        Value::Text(scheme),
                        Value::Text((*literal).into()),
                    ),
                )
                .await
                .unwrap();
        }
        if let Some(org) = bound_to {
            self.mention(notice, &section, org, country, ids.first().copied(), name).await;
        }
    }

    async fn mention(&self, notice: i64, section: &str, org: i64, country: &str, raw: Option<&str>, name: &str) {
        self.conn
            .execute(
                "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
                 VALUES (?, ?, ?, ?, ?, ?, NULL)",
                (
                    Value::Integer(notice),
                    Value::Text(section.into()),
                    Value::Integer(org),
                    Value::Text(name.into()),
                    Value::Text(country.into()),
                    raw.map_or(Value::Null, |r| Value::Text(r.into())),
                ),
            )
            .await
            .unwrap();
    }

    /// The common shape: a supplier published company-number-first (both ids
    /// on one party, bound to the company-number org) on `notice`, and
    /// PPON-first (bound to the PPON org) on `notice + 1`. Both orgs stand.
    async fn split(
        &self,
        notice: i64,
        (coh_org, coh, coh_name): (i64, &str, &str),
        (ppon_org, ppon, ppon_name): (i64, &str, &str),
    ) {
        self.org(coh_org, coh, coh_name).await;
        self.org(ppon_org, ppon, ppon_name).await;
        self.notice(notice, "fts:ocds-1.1").await;
        self.party(notice, &format!("C{notice}"), Some(coh_org), "GB", &[coh, ppon]).await;
        self.notice(notice + 1, "fts:ocds-1.1").await;
        self.party(notice + 1, &format!("P{notice}"), Some(ppon_org), "GB", &[ppon]).await;
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut rows = self.conn.query(sql, ()).await.unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
        n
    }

    async fn plan(&self) -> store::AltIdMergeReport {
        self.plan_capped(20).await
    }

    /// A wet run held against `expect`, checked for the same accounting as a
    /// plan.
    async fn wet(&self, expect: &[String]) -> store::AltIdMergeReport {
        let r = self.db.match_org_altid_pairs(wet_args(expect)).await.expect("wet run");
        assert_eq!(
            r.both_distinct,
            r.plan_pairs + r.denied_pairs() + r.conflicts,
            "every both-distinct pair lands in exactly one place: {r:#?}"
        );
        r
    }

    /// The row counts a write would move, table by table.
    async fn snapshot(&self) -> Vec<(&'static str, i64)> {
        let mut out = Vec::new();
        for t in [
            "organizations",
            "organization_mentions",
            "organization_names",
            "org_merge_log",
            "org_candidate_edges",
            "changes",
        ] {
            out.push((t, self.count(&format!("SELECT COUNT(*) FROM {t}")).await));
        }
        out
    }

    async fn text(&self, sql: &str) -> String {
        let mut rows = self.conn.query(sql, ()).await.unwrap();
        let row = rows.next().await.unwrap().unwrap();
        match row.get_value(0).unwrap() {
            Value::Text(s) => s,
            other => panic!("text: {other:?}"),
        }
    }

    async fn plan_capped(&self, cap: usize) -> store::AltIdMergeReport {
        let r = self.db.match_org_altid_pairs(args(cap)).await.expect("dry plan");
        assert_eq!(
            r.both_distinct,
            r.plan_pairs + r.denied_pairs() + r.conflicts,
            "every both-distinct pair lands in exactly one place: {r:#?}"
        );
        // Issue 469 U4: every register split is a register candidate, and every register
        // candidate lands in exactly one place too.
        assert_registry_accounting(&r);
        // Issue 513 unit 1: and every counted GB-NHS pair is listed, under its counted class.
        assert_nhs_listing_matches_counts(&r);
        r
    }

    /// Issue 469 U5: a wet run held against both stored sets, checked for both families'
    /// accounting: the company-number plan's, the register plan's (planned live since U5),
    /// and every live ∩ stored register pair either merged or in the residual.
    async fn wet_all(&self, expect: &[String], expect_registry: &[String]) -> store::AltIdMergeReport {
        let r = self.db.match_org_altid_pairs(wet_all_args(expect, expect_registry)).await.expect("wet run");
        assert_eq!(
            r.both_distinct,
            r.plan_pairs + r.denied_pairs() + r.conflicts,
            "every both-distinct pair lands in exactly one place: {r:#?}"
        );
        assert_registry_accounting(&r);
        assert_eq!(
            r.registry_merged_pairs + r.registry_residual_pairs.len() as u64,
            r.registry_pairs_planned.len() as u64 - r.registry_deferred_unreviewed,
            "every reviewed live register pair merged or carried: {r:#?}"
        );
        assert_eq!(r.registry_merged.len() as u64, r.registry_merged_pairs, "{r:#?}");
        r
    }

    async fn verdict(&self, coh: &str, ppon: &str, members: Vec<i64>, action: &str, confidence: &str) {
        self.db
            .record_merge_verdicts(
                "448-altid-test",
                &[store::MergeVerdict {
                    country: "GB".into(),
                    scheme: "GB:altid".into(),
                    key: format!("{}~{}", k(coh), k(ppon)),
                    members,
                    action: action.into(),
                    rationale: "fixture".into(),
                    confidence: confidence.into(),
                    reviewed_holders: None,
                }],
                0,
            )
            .await
            .unwrap();
    }

    /// Issue 469 U4, the register pair's common shape: a register-first party bound to
    /// the register org on `notice` (both ids), and a PPON-first party bound to the PPON
    /// org on `notice + 1`. Both orgs stand.
    async fn registry_split(
        &self,
        notice: i64,
        (reg_org, reg, reg_name): (i64, &str, &str),
        (ppon_org, ppon, ppon_name): (i64, &str, &str),
    ) {
        self.org(reg_org, reg, reg_name).await;
        self.org(ppon_org, ppon, ppon_name).await;
        self.notice(notice, "fts:ocds-1.1").await;
        self.party(notice, &format!("R{notice}"), Some(reg_org), "GB", &[reg, ppon]).await;
        self.notice(notice + 1, "fts:ocds-1.1").await;
        self.party(notice + 1, &format!("P{notice}"), Some(ppon_org), "GB", &[ppon]).await;
    }

    /// A merge verdict under (`GB`, `GB:altid`) on a register key ([`rk`]), cohort
    /// `469-registry-test` — a reviewer's, so a merge it admits reads `confirmed`
    /// `verdict`.
    async fn registry_verdict(&self, key: &str, members: Vec<i64>, action: &str, confidence: &str) {
        self.registry_verdict_in("469-registry-test", key, members, action, confidence).await;
    }

    /// Issue 469 U5: [`Self::registry_verdict`] under `cohort` ([`CONFIRMED`] for a
    /// register-confirmed pair).
    async fn registry_verdict_in(&self, cohort: &str, key: &str, members: Vec<i64>, action: &str, confidence: &str) {
        self.db
            .record_merge_verdicts(
                cohort,
                &[store::MergeVerdict {
                    country: "GB".into(),
                    scheme: "GB:altid".into(),
                    key: key.into(),
                    members,
                    action: action.into(),
                    rationale: "fixture".into(),
                    confidence: confidence.into(),
                    reviewed_holders: None,
                }],
                0,
            )
            .await
            .unwrap();
    }
}

/// The issue's shape, planned: one supplier split across a company-number org
/// and a PPON org, names agreeing. The survivor is the company-number org even
/// though the PPON org has the LOWER id — R2's min-id rule does not apply here.
/// And a notice under another profile carrying a perfect pair is not FTS, so it
/// is never harvested.
#[tokio::test]
async fn a_clean_pair_with_agreeing_names_is_planned_and_keeps_the_company_number_org() {
    let b = bed("clean").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    // Not FTS: an eForms notice publishing a pair the planner would take.
    b.org(3, COH_B, "Beta Ltd").await;
    b.org(4, PPON_Q, "Beta Ltd").await;
    b.notice(200, "eforms-sdk-1.10").await;
    b.party(200, "X", Some(3), "GB", &[COH_B, PPON_Q]).await;

    let r = b.plan().await;
    assert_eq!(r.fts_notices, 2, "the eForms notice is not walked");
    assert_eq!((r.party_sections, r.unfolded_sections, r.literal_pairs, r.pairs_seen), (2, 0, 1, 1));
    assert_eq!((r.scanned, r.owners, r.both_distinct), (4, 2, 1));
    assert_eq!(r.plan_pairs, 1);
    assert_eq!(r.pairs, vec![format!("{}~{}", k(COH_A), k(PPON_P))]);
    let l = &r.plan_listing[0];
    assert_eq!(l.gate, "plan");
    assert_eq!(l.keep, Some(2), "the company-number org survives, whatever the ids say");
    assert_eq!(l.members.iter().map(|m| m.0).collect::<Vec<_>>(), vec![1, 2], "ascending, verdict-ready");
    assert_eq!(l.members[0].2, minted(PPON_P));
    assert_eq!((l.coh_literal.as_str(), l.ppon_literal.as_str()), (COH_A, PPON_P));
    assert_eq!((l.witnesses, l.first_coh, l.first_ppon), (1, 1, 0));
    assert_eq!((l.coh_partners, l.ppon_partners), (1, 1));
    assert_eq!(l.witness_publications, vec!["pub-100".to_owned()]);
    assert_eq!(r.mentions, 1, "the PPON org's one mention is what a merge would move");
}

/// `Ltd` and `Limited` are one legal form spelled two ways, so the names
/// corroborate.
#[tokio::test]
async fn ltd_and_limited_corroborate() {
    let b = bed("ltd-limited").await;
    b.split(100, (1, COH_A, "Acme Widgets Ltd"), (2, PPON_P, "ACME WIDGETS LIMITED")).await;
    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.uncorroborated_overlap, r.uncorroborated_disjoint), (1, 0, 0));
}

/// A plc beside a Ltd is the parent/subsidiary shape a publisher writes by
/// mistake (the parent's company number on the subsidiary's party). The veto is
/// head against head, so a satellite that WOULD corroborate cannot carry it.
#[tokio::test]
async fn a_parent_plc_beside_a_subsidiary_ppon_is_denied() {
    let b = bed("plc-ltd").await;
    b.split(100, (1, COH_A, "Acme Holdings plc"), (2, PPON_P, "Acme Holdings Ltd")).await;
    b.satellite(2, "CYM", "Acme Holdings plc").await;
    let r = b.plan().await;
    assert_eq!((r.denied_legal_form, r.plan_pairs), (1, 0));
    assert_eq!(r.denied_listing[0].gate, "legal-form");
    assert_eq!(r.denied_listing[0].keep, Some(1));
}

/// `Acme UK Ltd` beside `Acme Ltd` is a sister company, and a core-token rule
/// that drops two-letter tokens would read them as one (the hole in design 1).
/// The altid key keeps `uk`, so the pair is uncorroborated — and the shared
/// core `acme` makes it the review shape, not a disjoint error.
#[tokio::test]
async fn a_uk_suffixed_sister_name_is_uncorroborated() {
    let b = bed("uk-sister").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme UK Ltd")).await;
    let r = b.plan().await;
    assert_eq!((r.uncorroborated_overlap, r.uncorroborated_disjoint, r.plan_pairs), (1, 0, 0));
}

/// Issue 447's Energinet shape: names that share a core but are not one name
/// are LISTED for review as `overlap`, never merged; names with nothing in
/// common are the probable publisher error and list as `disjoint`.
#[tokio::test]
async fn an_overlapping_sister_name_lists_as_overlap_not_merge() {
    let b = bed("overlap").await;
    b.split(100, (1, COH_A, "Acme Water Services Ltd"), (2, PPON_P, "Acme Water Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Zenith Ltd")).await;
    let r = b.plan().await;
    assert_eq!((r.uncorroborated_overlap, r.uncorroborated_disjoint, r.plan_pairs), (1, 1, 0));
    let gate_of = |coh: &str| {
        r.denied_listing.iter().find(|l| l.coh == k(coh)).map(|l| l.gate.clone()).expect("listed")
    };
    assert_eq!(gate_of(COH_A), "uncorroborated-overlap");
    assert_eq!(gate_of(COH_B), "uncorroborated-disjoint");
}

/// Unit 1b: witnesses publish another supplier's name beside a company number,
/// company number first. The fold binds those mentions to the company-number
/// org and records the witness name as its satellite, so its designated names
/// "agree" with the PPON org's. Every name either org carries from ANY OTHER
/// notice disagrees, so the pair lists as `witness-only`, never as a merge. A
/// reviewer's HIGH merge verdict still admits it, which is the path a real
/// rename takes. (Unit 1 read this shape into dry job 1681's Amentum pair. The
/// backfill showed that pair TRUE — PBDC-7744-BTPG is Amentum's own PPON, and
/// the Altrad name was a publisher's mislabel — but the circularity is real.)
#[tokio::test]
async fn a_name_only_the_witnesses_recorded_never_corroborates() {
    let b = bed("witness-only").await;
    b.org(1, COH_A, "Amec Foster Wheeler Nuclear UK Limited").await;
    b.org(2, PPON_P, "Altrad Babcock Limited").await;
    // Amentum's own history under its company number.
    b.notice(90, "fts:ocds-1.1").await;
    b.party(90, "H", Some(1), "GB", &[COH_A]).await;
    // The witness: Altrad Babcock's name, Amentum's number first, then the PPON.
    b.notice(100, "fts:ocds-1.1").await;
    b.party_named(100, "W", Some(1), "GB", &[COH_A, PPON_P], "Altrad Babcock Limited").await;
    b.satellite(1, "ENG", "Altrad Babcock Limited").await;
    // Altrad Babcock's own PPON-first history.
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(2), "GB", &[PPON_P]).await;

    let r = b.plan().await;
    assert_eq!((r.denied_witness_only, r.plan_pairs, r.uncorroborated_overlap), (1, 0, 0), "{r:#?}");
    assert_eq!(r.denied_listing[0].gate, "witness-only");
    assert_eq!(r.denied_listing[0].keep, Some(1));
    // Unit 3b: the reviewer sees the names that disagree, and nothing cleared.
    assert_eq!(r.denied_listing[0].coh_names, vec!["Amec Foster Wheeler Nuclear UK Limited".to_owned()]);
    assert_eq!(r.denied_listing[0].ppon_names, vec!["Altrad Babcock Limited".to_owned()]);
    assert_eq!(r.denied_listing[0].corroborated_by, None);

    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    let r = b.plan().await;
    assert_eq!((r.admitted_verdict, r.plan_pairs, r.denied_witness_only), (1, 1, 0));
}

/// Unit 3b, the prod shape the backfill showed (2026-09-30): Amentum's PPON
/// org is headed `Altrad Babcock Limited` because one publisher listed Altrad
/// Babcock under Amentum's PPON, as a SECOND party beside Amentum itself. The
/// listing carries what a reviewer reads that by: each side's witness-free
/// names (the stray among them), the names that cleared, and the notice where
/// both orgs are distinct parties. The co-occurrence is counted, never gated.
#[tokio::test]
async fn a_listing_carries_the_names_a_reviewer_reads_and_the_cooccurring_notices() {
    let b = bed("reviewer-evidence").await;
    b.org(1, COH_A, "Amentum Clean Energy Limited").await;
    b.org(2, PPON_P, "Altrad Babcock Limited").await;
    for n in [90, 100, 101, 102] {
        b.notice(n, "fts:ocds-1.1").await;
    }
    b.party(90, "H", Some(1), "GB", &[COH_A]).await;
    b.party(100, "W", Some(1), "GB", &[COH_A, PPON_P]).await;
    b.party_named(101, "P", Some(2), "GB", &[PPON_P], "Amentum Clean Energy Ltd").await;
    // The mislabel: Amentum by its number, and Altrad Babcock by Amentum's PPON.
    b.party(102, "A", Some(1), "GB", &[COH_A]).await;
    b.party(102, "B", Some(2), "GB", &[PPON_P]).await;

    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.plan_cooccurring), (1, 1), "{r:#?}");
    let l = &r.plan_listing[0];
    assert_eq!(l.coh_names, vec!["Amentum Clean Energy Limited".to_owned()]);
    assert_eq!(l.ppon_names, vec!["Altrad Babcock Limited".to_owned(), "Amentum Clean Energy Ltd".to_owned()]);
    assert_eq!(
        l.corroborated_by,
        Some(("Amentum Clean Energy Limited".to_owned(), "Amentum Clean Energy Ltd".to_owned()))
    );
    assert_eq!((l.coh_name_keys, l.ppon_name_keys), (1, 2));
    assert_eq!((l.cooccurring, l.cooccur_publications.clone()), (1, vec!["pub-102".to_owned()]));
    assert_eq!(l.witness_publications, vec!["pub-100".to_owned()]);

    // The review's case: the witness notice ALSO lists the supplier under its
    // PPON alone, as a second party bound to the PPON org. The notice asserts
    // the pair, so that is one supplier listed twice, not two parties.
    b.party(100, "W2", Some(2), "GB", &[PPON_P]).await;
    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.plan_cooccurring), (1, 1), "{r:#?}");
    assert_eq!(r.plan_listing[0].cooccur_publications, vec!["pub-102".to_owned()]);
}

/// The other side of unit 1b's rule: an org made of the witness mentions ALONE
/// (the company-number org was minted by the very notices that pair it) lets
/// those names stand in, and they agree with the PPON org's own history. And a
/// legal form only one side publishes is no disagreement.
#[tokio::test]
async fn an_org_made_only_of_witness_mentions_corroborates_with_its_own_names() {
    let b = bed("witness-made").await;
    b.split(100, (1, COH_A, "Carnall Farrar Ltd"), (2, PPON_P, "Carnall Farrar")).await;
    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.denied_witness_only, r.uncorroborated_overlap), (1, 0, 0), "{r:#?}");
}

/// The review's case against unit 1b's first cut: an outside mention with NO
/// name still means the org is not made of the witnesses, so their name stays
/// out. Deciding on keyed names instead let a blank or unkeyable name bring the
/// circular witness name back.
#[tokio::test]
async fn a_nameless_outside_mention_still_keeps_the_witness_name_out() {
    let b = bed("nameless-outside").await;
    b.org(1, COH_A, "Amec Foster Wheeler Nuclear UK Limited").await;
    b.org(2, PPON_P, "Altrad Babcock Limited").await;
    b.notice(90, "fts:ocds-1.1").await;
    b.party_named(90, "H", Some(1), "GB", &[COH_A], "").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party_named(100, "W", Some(1), "GB", &[COH_A, PPON_P], "Altrad Babcock Limited").await;
    b.satellite(1, "ENG", "Altrad Babcock Limited").await;
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(2), "GB", &[PPON_P]).await;
    let r = b.plan().await;
    assert_eq!((r.denied_witness_only, r.plan_pairs), (1, 0), "{r:#?}");
}

/// A formless name agrees with `Acme plc` and with `Acme Ltd`, so it could
/// bridge a parent's company number to a subsidiary's PPON that the
/// head-against-head veto never sees (the PPON org's head is formless).
/// Every name the agreement reads is checked: plc on one side, Ltd on the
/// other, nothing shared.
#[tokio::test]
async fn a_formless_name_never_bridges_a_plc_and_a_ltd() {
    let b = bed("form-bridge").await;
    b.org(1, COH_A, "Acme plc").await;
    b.org(2, PPON_P, "Acme").await;
    b.notice(90, "fts:ocds-1.1").await;
    b.party(90, "H", Some(1), "GB", &[COH_A]).await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party_named(100, "W", Some(1), "GB", &[COH_A, PPON_P], "Acme Ltd").await;
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(2), "GB", &[PPON_P]).await;
    b.notice(102, "fts:ocds-1.1").await;
    b.party_named(102, "P2", Some(2), "GB", &[PPON_P], "Acme Ltd").await;
    let r = b.plan().await;
    assert_eq!((r.denied_form_conflict, r.plan_pairs, r.denied_legal_form), (1, 0, 0), "{r:#?}");
    assert_eq!(r.denied_listing[0].gate, "form-conflict");
}

/// The generic wall reads the words the agreement read. A trading-as clause
/// is cut before keying, so it is cut before the wall too: otherwise the
/// clause's extra words make a generic name look unique.
#[tokio::test]
async fn a_trading_as_clause_does_not_carry_a_generic_name_past_the_wall() {
    let b = bed("ta-wall").await;
    b.split(100, (1, COH_A, "Acme"), (2, PPON_P, "Acme t/a Acme Scaffolding")).await;
    // The wall counts carriers that still stand as orgs.
    b.org_in(7, "GB", "GBCOH00000007", "Acme").await;
    b.org_in(8, "GB", "GBCOH00000008", "Acme").await;
    for org in [1i64, 7, 8] {
        b.conn
            .execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'acme')", (Value::Integer(org),))
            .await
            .unwrap();
    }
    let r = b.plan_capped(2).await;
    assert_eq!((r.denied_generic, r.plan_pairs), (1, 0), "{r:#?}");
}

/// One PPON beside two company numbers: one of the statements is wrong and the
/// arm cannot say which, so both pairs are conflicts — whatever the names say.
#[tokio::test]
async fn a_ppon_paired_with_two_company_numbers_is_a_conflict() {
    let b = bed("ppon-two-coh").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.org(3, COH_B, "Acme Ltd").await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "B", Some(3), "GB", &[COH_B, PPON_P]).await;
    let r = b.plan().await;
    assert_eq!((r.both_distinct, r.conflicts, r.conflict_ppon_multi_coh, r.plan_pairs), (2, 2, 2, 0));
    assert_eq!(r.conflict_coh_multi_ppon, 0);
    assert!(r.conflict_listing.iter().all(|l| l.gate == "ppon-multi-coh" && l.ppon_partners == 2));
}

/// One company number beside two PPONs: the mirror conflict.
#[tokio::test]
async fn a_company_number_with_two_ppons_is_a_conflict() {
    let b = bed("coh-two-ppon").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.org(3, PPON_Q, "Acme Ltd").await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "Q", Some(1), "GB", &[COH_A, PPON_Q]).await;
    let r = b.plan().await;
    assert_eq!((r.conflicts, r.conflict_coh_multi_ppon, r.conflict_ppon_multi_coh, r.plan_pairs), (2, 2, 0, 0));
    assert!(r.conflict_listing.iter().all(|l| l.gate == "coh-multi-ppon" && l.coh_partners == 2));
}

/// A party that names two company numbers (the 89 COH→COH pairs) is not one
/// supplier's statement, so every key it names is tainted — its PPON included,
/// and the taint follows a key into OTHER notices' pairs.
#[tokio::test]
async fn a_party_listing_two_company_numbers_taints_its_ppon() {
    let b = bed("ambiguous").await;
    for (id, lit) in [(1, COH_A), (2, COH_B), (3, PPON_P), (4, COH_C), (5, PPON_Q), (6, COH_E), (7, PPON_S)] {
        b.org(id, lit, "Acme Ltd").await;
    }
    // Two company numbers and a PPON on one party.
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "X", Some(1), "GB", &[COH_A, COH_B, PPON_P]).await;
    // Two company numbers and no PPON: pairs nothing, taints COH_C…
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "Y", Some(4), "GB", &[COH_C, COH_D]).await;
    // …so COH_C's clean-looking pair on another notice is tainted too.
    b.notice(102, "fts:ocds-1.1").await;
    b.party(102, "Z", Some(4), "GB", &[COH_C, PPON_Q]).await;
    // The control: a pair between keys no ambiguous party named.
    b.notice(103, "fts:ocds-1.1").await;
    b.party(103, "W", Some(6), "GB", &[COH_E, PPON_S]).await;

    let r = b.plan().await;
    assert_eq!(r.ambiguous_parties, 2);
    assert_eq!(r.pairs_seen, 4, "(A,P), (B,P), (C,Q), (E,S)");
    assert_eq!(r.party_ambiguous, 3, "A, B and P through X's party; C through Y's");
    assert_eq!(r.conflict_ppon_multi_coh, 2, "P also stands beside two company numbers");
    let c_q = r.conflict_listing.iter().find(|l| l.coh == k(COH_C)).expect("listed");
    assert_eq!(c_q.gate, "party-ambiguous", "tainted, and nothing else is wrong with it");
    let a_p = r.conflict_listing.iter().find(|l| l.coh == k(COH_A)).expect("listed");
    assert_eq!(a_p.gate, "ppon-multi-coh+party-ambiguous");
    assert_eq!(r.pairs, vec![format!("{}~{}", k(COH_E), k(PPON_S))]);
}

/// A 6-7 digit company number keys only by a pad (E2), and a pad collides by
/// construction, so it is counted and never paired — even with both orgs
/// standing and the names agreeing. Its siblings in the breakdown of the
/// issue's 490 "neither found" pairs each land in a class of their own: a
/// condemned value, a malformed one, a non-GB party, and a party the fold has
/// not reached.
#[tokio::test]
async fn a_padded_company_number_is_counted_never_paired() {
    let b = bed("pad").await;
    b.org(1, COH_A, "Acme Ltd").await;
    b.org(2, PPON_P, "Acme Ltd").await;
    b.notice(100, "fts:ocds-1.1").await;
    // 03914810 published as 3914810: pads onto org 1's key.
    b.party(100, "PAD", Some(1), "GB", &["GB-COH-3914810", PPON_P]).await;
    b.party(100, "BAD", Some(1), "GB", &["GB-COH-99999999", PPON_Q]).await;
    b.party(100, "ODD", Some(1), "GB", &["GB-COH-12AB", PPON_R]).await;
    b.party(100, "IE", Some(1), "IE", &[COH_B, PPON_S]).await;
    b.party(100, "NEW", None, "GB", &[COH_C, PPON_S]).await;
    let r = b.plan().await;
    assert_eq!((r.pad_side, r.condemned, r.unkeyed_value, r.non_gb_pairs), (1, 1, 1, 1));
    assert_eq!((r.party_sections, r.unfolded_sections), (5, 1));
    assert_eq!(r.literal_pairs, 3, "the GB parties' pairs, keyed or not");
    assert_eq!((r.pairs_seen, r.both_distinct, r.plan_pairs), (0, 0, 0));
}

/// Only `GB-COH` and `GB-PPON` rows are sides of a pair. An 8-digit value under
/// another register's scheme would key as a company number through the GB
/// arm's bare-GB strip — here it would reach org 1 — so it is counted by scheme
/// and never keyed.
#[tokio::test]
async fn an_unkeyed_scheme_is_never_keyed() {
    let b = bed("scheme").await;
    b.org_in(1, "GB", "12345678", "Acme Ltd").await;
    b.org(2, PPON_P, "Acme Ltd").await;
    assert!(key(Some("GB"), "national", "12345678").is_some(), "the value alone WOULD key");
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "CHC", Some(2), "GB", &[PPON_P, "GB-CHC-12345678"]).await;
    b.conn
        .execute(
            "INSERT INTO notice_ids (notice_id, section_id, field_id, ordinal, scheme, value, is_ref)
             VALUES (100, 'ORG-CHC', 'BT-501-Organization-Company', 2, 'GB-NHS', '12345678', 0)",
            (),
        )
        .await
        .unwrap();
    let r = b.plan().await;
    assert_eq!(r.unkeyed_scheme.get("GB-CHC"), Some(&1));
    assert_eq!(r.unkeyed_scheme.get("GB-NHS"), Some(&1), "even a bare digits-only value");
    assert_eq!((r.literal_pairs, r.pairs_seen, r.plan_pairs), (0, 0, 0));
}

/// Two standing rows share the company number's key: that is a family R2
/// declined to merge, and this arm never picks a side of it.
#[tokio::test]
async fn a_key_with_two_owners_is_multi_target() {
    let b = bed("multi").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    // The same company number, stored bare.
    b.org_in(3, "GB", "03914810", "Acme Ltd").await;
    let r = b.plan().await;
    assert_eq!((r.multi_target, r.both_distinct, r.plan_pairs), (1, 0, 0));
    assert_eq!(r.owners, 3);
}

/// A consortium-named side publishes its lead member's ids; the veto reads
/// every name of both orgs, satellites included, and no verdict overrides it.
#[tokio::test]
async fn a_consortium_named_side_denies() {
    let b = bed("consortium").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "ACME LTD")).await;
    b.satellite(1, "ENG", "Acme Consortium").await;
    let r = b.plan().await;
    assert_eq!((r.denied_consortium, r.plan_pairs), (1, 0));
    assert_eq!(r.denied_listing[0].gate, "consortium");
}

/// A corroborating name more than `stoplist_cap` orgs carry is agreement nobody
/// chose to make unique: denied, with no hard-scheme exemption. Generic means
/// MORE than the cap.
#[tokio::test]
async fn a_generic_corroborating_name_denies() {
    let b = bed("generic").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "ACME LTD")).await;
    b.org_in(9, "GB", "GBCOH00000009", "Acme Ltd").await;
    for org in [1i64, 2, 9] {
        b.conn
            .execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'acme ltd')",
                (Value::Integer(org),),
            )
            .await
            .unwrap();
    }
    let at_cap = b.plan_capped(3).await;
    assert_eq!((at_cap.denied_generic, at_cap.plan_pairs), (0, 1), "three carriers, cap three");
    let over = b.plan_capped(2).await;
    assert_eq!((over.denied_generic, over.plan_pairs), (1, 0));
    assert_eq!(over.denied_listing[0].gate, "generic");
}

/// The wall reads mention raws through `mention_key`. The R2/R3 inference — a
/// two-letter lead means VAT — would send every `GB-COH-…` raw to a VAT kind
/// the GB arm never keys, and the wall would be blind to exactly the values
/// this arm is about. Each org here also carries a DIFFERENT second company
/// number: two registrations in one scheme, denied. Then the control: the same
/// second number on both is no conflict, but the PPON org naming another GB
/// registration at all is `loser_incoherent`.
#[tokio::test]
async fn the_evidence_wall_keys_fts_raws() {
    let b = bed("wall").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    // Mentions the fold bound to each org, published under another number.
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "W1", Some(1), "GB", &[COH_B]).await;
    b.party(300, "W2", Some(2), "GB", &[COH_C]).await;
    let r = b.plan().await;
    assert_eq!((r.denied_evidence_wall, r.denied_loser_incoherent, r.plan_pairs), (1, 0, 0));
    assert_eq!(r.denied_listing[0].gate, "evidence-wall");

    b.conn
        .execute(
            "UPDATE organization_mentions SET raw_identifier = ? WHERE section_id = 'ORG-W2'",
            (Value::Text(COH_B.into()),),
        )
        .await
        .unwrap();
    let r = b.plan().await;
    assert_eq!((r.denied_evidence_wall, r.denied_loser_incoherent, r.plan_pairs), (0, 1, 0));

    // And the pair's OWN keys on either side are the statement under test, never
    // evidence: with the extra mentions gone, it plans.
    b.conn.execute("DELETE FROM organization_mentions WHERE notice_id = 300", ()).await.unwrap();
    b.party(301, "W3", Some(2), "GB", &[COH_A]).await;
    let r = b.plan().await;
    assert_eq!(r.plan_pairs, 1, "{r:#?}");
}

/// A PPON whose org no longer stands, while the company-number org's mentions
/// carry it, is already one supplier — the state a merge leaves behind (the
/// loser's mentions move to the survivor). It plans nothing and is not a
/// no-target pair.
///
/// The design's `already_one` also names two keys owned by ONE row, which
/// cannot happen through `organizations.identifier` (one key per row); this is
/// the reachable form of the same fact.
#[tokio::test]
async fn an_already_unified_pair_plans_nothing() {
    let b = bed("unified").await;
    b.org(1, COH_A, "Acme Ltd").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "C", Some(1), "GB", &[COH_A, PPON_P]).await;
    // The PPON-first party, bound to the company-number org.
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(1), "GB", &[PPON_P]).await;
    let r = b.plan().await;
    assert_eq!((r.already_one, r.no_target_ppon, r.plan_pairs), (1, 0, 0));

    // Without that mention the PPON never stood: a no-target pair, sampled.
    b.conn.execute("DELETE FROM organization_mentions WHERE notice_id = 101", ()).await.unwrap();
    let r = b.plan().await;
    assert_eq!((r.already_one, r.no_target_ppon), (0, 1));
    let s = &r.no_target_sample[0];
    assert_eq!((s.gate.as_str(), s.ppon_literal.as_str()), ("no-target-ppon", PPON_P));
    assert_eq!(s.members.iter().map(|m| m.0).collect::<Vec<_>>(), vec![1]);
}

/// A reviewer's `keep` denies the pair for good, however well the names agree.
#[tokio::test]
async fn a_keep_verdict_denies() {
    let b = bed("keep").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "ACME LTD")).await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "keep", "high").await;
    let r = b.plan().await;
    assert_eq!((r.denied_verdict, r.plan_pairs), (1, 0));
    assert_eq!(r.denied_listing[0].gate, "verdict-keep");
}

/// A HIGH `merge` over exactly the live pair stands in for the judgment gates —
/// here an uncorroborated pair plans — but never for a structural one.
#[tokio::test]
async fn a_high_merge_verdict_with_exact_members_admits_an_uncorroborated_pair_but_never_past_structural_gates() {
    let b = bed("admit").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Zenith Ltd")).await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    // A plc beside a Ltd, with the same verdict: the legal-form veto holds.
    b.split(200, (3, COH_B, "Northgate plc"), (4, PPON_Q, "Northgate Ltd")).await;
    b.verdict(COH_B, PPON_Q, vec![3, 4], "merge", "high").await;
    let r = b.plan().await;
    assert_eq!((r.admitted_verdict, r.denied_legal_form, r.uncorroborated_disjoint), (1, 1, 0));
    assert_eq!(r.pairs, vec![format!("{}~{}", k(COH_A), k(PPON_P))]);
}

/// A verdict read over another member set, or not HIGH, is stale: the pair is
/// judged as if it did not exist.
#[tokio::test]
async fn a_verdict_for_another_member_set_is_stale() {
    let b = bed("stale").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Zenith Ltd")).await;
    b.verdict(COH_A, PPON_P, vec![1, 99], "merge", "high").await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Southgate Ltd")).await;
    b.verdict(COH_B, PPON_Q, vec![3, 4], "merge", "medium").await;
    let r = b.plan().await;
    assert_eq!((r.verdict_stale, r.admitted_verdict), (2, 0));
    assert_eq!((r.uncorroborated_disjoint, r.plan_pairs), (2, 0));
}

/// Issue 513 U2: `reviewed_holders` belongs to the nhs-body census alone. An altid
/// verdict records it NULL, and the altid planner never reads it: a column filled behind
/// its back admits and stales exactly the pairs it did before (the R2 half is
/// `r2_verdicts_ignore_reviewed_holders` in r2_name_gate.rs).
#[tokio::test]
async fn altid_verdicts_ignore_reviewed_holders() {
    let b = bed("reviewed-holders").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Zenith Ltd")).await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Southgate Ltd")).await;
    b.verdict(COH_B, PPON_Q, vec![3, 99], "merge", "high").await;
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_verdicts WHERE reviewed_holders IS NULL").await,
        2,
        "an altid verdict carries no holder set"
    );
    let before = b.plan().await;
    assert_eq!((before.admitted_verdict, before.verdict_stale, before.plan_pairs), (1, 1, 1));
    b.conn.execute("UPDATE org_merge_verdicts SET reviewed_holders = '[999]'", ()).await.unwrap();
    let after = b.plan().await;
    assert_eq!(
        (after.admitted_verdict, after.verdict_stale, after.plan_pairs, after.pairs),
        (before.admitted_verdict, before.verdict_stale, before.plan_pairs, before.pairs),
        "the column is not altid's"
    );
    // And recording one on an altid verdict is refused: it would read as reviewed.
    let refused = b
        .db
        .record_merge_verdicts(
            "448-altid-test",
            &[store::MergeVerdict {
                country: "GB".into(),
                scheme: "GB:altid".into(),
                key: pk(COH_A, PPON_P),
                members: vec![1, 2],
                action: "merge".into(),
                rationale: "fixture".into(),
                confidence: "high".into(),
                reviewed_holders: Some(vec![1, 2]),
            }],
            0,
        )
        .await
        .expect_err("reviewed_holders is nhs-body's");
    assert!(refused.to_string().contains("reviewed_holders is for GB:nhs-body verdicts only"), "{refused}");
}

/// The dry plan writes nothing — not even the denied pair's edge, which only a
/// wet run records — and a wet run WITHOUT the stored plan's pairs is refused
/// before anything is read: it could only merge unreviewed pairs.
#[tokio::test]
async fn the_dry_run_writes_nothing() {
    let b = bed("dry").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "ACME LTD")).await;
    b.split(200, (3, COH_B, "Northgate plc"), (4, PPON_Q, "Northgate Ltd")).await;
    let before = b.snapshot().await;
    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.denied_legal_form), (1, 1));
    assert_eq!((r.merged_pairs, r.removed, r.edges_written), (0, 0, 0));
    let mut wet = args(20);
    wet.dry_run = false;
    let err = b.db.match_org_altid_pairs(wet).await.expect_err("no stored plan, no wet run");
    assert!(err.to_string().contains("no expected pair set") && err.to_string().contains("nothing was"), "{err}");
    assert_eq!(b.snapshot().await, before);
}

/// The harvest reads one range of `notices_profile` per FTS profile and one
/// primary-key range of `notice_ids` per notice — the party sections only,
/// never the lot, result or contract rows; the alias preload reads the e2-altid
/// ledger rows through their partial index. Asserted on the plans, not a clock.
#[tokio::test]
async fn the_harvest_seeks_notices_profile_and_the_notice_ids_pk() {
    let b = bed("plans").await;
    b.db.build_organization_indexes().await.unwrap();
    let plan = |sql: &'static str, p: Vec<Value>| {
        let conn = &b.conn;
        async move {
            let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), p).await.unwrap();
            let mut out = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                if let Ok(Value::Text(d)) = row.get_value(3) {
                    out.push_str(&d);
                    out.push('\n');
                }
            }
            out
        }
    };
    let text = |s: &str| Value::Text(s.into());
    let next = plan(store::ALTID_NEXT_PROFILE_SQL, vec![text("fts:")]).await;
    assert!(next.contains("SEARCH notices USING") && next.contains("notices_profile (profile>?"), "{next}");
    let ids = plan(store::ALTID_PROFILE_NOTICES_SQL, vec![text("fts:ocds-1.1")]).await;
    assert!(ids.contains("SEARCH notices USING") && ids.contains("notices_profile (profile=?)"), "{ids}");
    let party = plan(store::ALTID_PARTY_IDS_SQL, vec![Value::Integer(1)]).await;
    assert!(
        party.contains("SEARCH notice_ids USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id>"),
        "a primary-key RANGE on the party sections, not every id row of the notice:\n{party}"
    );
    let mentions = plan(store::ALTID_PARTY_MENTIONS_SQL, vec![Value::Integer(1)]).await;
    assert!(mentions.contains("SEARCH organization_mentions") && !mentions.contains("SCAN"), "{mentions}");
    // The alias preload runs on every fold: a seek through the partial index
    // over the e2-altid rows, never a walk of the whole merge ledger.
    let ledger = plan(store::ALTID_ALIAS_LEDGER_SQL, vec![]).await;
    assert!(ledger.contains("org_merge_log_e2_altid") && !ledger.contains("SCAN org_merge_log\n"), "{ledger}");
    // Issue 469 U4: the register planner's two reads — a witness notice's buyers by
    // issue 483's two seeks, and a PPON holder's merged identifiers by its org.
    b.db.build_tender_indexes().await.unwrap();
    let buyers = plan(store::ALTID_WITNESS_BUYERS_SQL, vec![Value::Integer(1)]).await;
    assert!(
        buyers.contains("tender_versions_notice")
            && buyers.contains("tender_version_parties_version")
            && !buyers.contains("SCAN"),
        "{buyers}"
    );
    let merged = plan(store::ALTID_MERGED_IDS_OF_ORG_SQL, vec![Value::Integer(1)]).await;
    assert!(merged.contains("organization_merged_identifiers_org") && !merged.contains("SCAN"), "{merged}");
    // Issue 518: the alias bar's owner-name read, on the writer inside every fold — a
    // seek of the owner's index range walked backwards (`SeekLE`/`Prev`, 2026-10-11),
    // never a sorter that reads every mention of a large owner before the LIMIT. Turso
    // names one `USE SORTER FOR ORDER BY` (SQLite: `USE TEMP B-TREE`); either fails.
    let owner = plan(store::ALTID_ALIAS_OWNER_MENTION_NAMES_SQL, vec![Value::Integer(1), Value::Integer(10)]).await;
    assert!(
        owner.contains("organization_mentions_org")
            && !owner.contains("SCAN")
            && !owner.contains("SORTER")
            && !owner.contains("TEMP B-TREE"),
        "{owner}"
    );
}

// ---- Unit 2: the wet run.

/// The issue's shape, merged: the PPON org — here the LOWER id — folds into the
/// company-number org. Its mention, party, bid-party and winner rows and its
/// name variants move; a winner row the survivor already holds collapses to
/// one; the loser row is gone; the ledger names the rule and the statement; the
/// change feed hears the removal, the survivor and the touched tender. A re-run
/// finds the pair already one.
#[tokio::test]
async fn a_wet_run_merges_the_reviewed_pair_into_the_company_number_org() {
    let b = bed("wet").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.satellite(1, "CYM", "Acme Widgets Cyf").await;
    for sql in [
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 'winner', 1, 101, 'ORG-P100')",
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 70, 'tenderer', 1, 101, 'ORG-P100')",
        // One award both orgs hold (a doubled winner), one only the loser holds.
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (7, 1, 70, 1)",
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (7, 1, 70, 2)",
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (7, 1, 71, 1)",
    ] {
        b.conn.execute(sql, ()).await.unwrap();
    }
    let dry = b.plan().await;
    assert_eq!(dry.pairs, vec![pk(COH_A, PPON_P)]);

    let r = b.wet(&dry.pairs).await;
    assert_eq!((r.plan_pairs, r.expected_pairs, r.deferred_unreviewed, r.expected_not_live), (1, 1, 0, 0));
    assert_eq!((r.merged_pairs, r.removed, r.tender_changes), (1, 1, 1));
    assert_eq!(
        (r.mentions, r.parties, r.bid_parties, r.winners, r.winner_dups),
        (1, 1, 1, 1, 1),
        "what was actually repointed, the doubled award collapsed"
    );
    assert!(r.residual_pairs.is_empty() && r.deferred_pairs.is_empty() && !r.stopped);
    assert!(r.plan_listing.iter().all(|l| l.keep == Some(2)));

    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 1").await, 0, "the loser row is gone");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 2").await, 1, "the company-number org survives");
    for table in ["organization_mentions", "tender_version_parties", "tender_version_bid_parties", "organization_names"] {
        let col = if table == "organization_names" { "org_id" } else { "organization_id" };
        assert_eq!(b.count(&format!("SELECT COUNT(*) FROM {table} WHERE {col} = 1")).await, 0, "{table}");
    }
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 2").await, 2);
    assert_eq!(b.count("SELECT organization_id FROM organization_mentions WHERE notice_id = 101").await, 2);
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_names WHERE org_id = 2 AND lang = 'CYM'").await, 1);
    assert_eq!(
        b.count("SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id = 2").await,
        2,
        "lot results 70 (once) and 71"
    );

    // The ledger: one flat, json_extract-readable row a targeted unwind can find.
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'e2-altid' AND keep = 2 AND loser = 1 AND job_id = 77").await,
        1
    );
    let evidence = b.text("SELECT evidence FROM org_merge_log WHERE loser = 1").await;
    for (field, value) in [
        ("scheme", "GB:altid".to_owned()),
        ("coh", k(COH_A)),
        ("ppon", k(PPON_P)),
        ("coh_literal", COH_A.to_owned()),
        ("ppon_literal", PPON_P.to_owned()),
        ("keep_id", minted(COH_A)),
        ("loser_id", minted(PPON_P)),
        ("verdict", "none".to_owned()),
        ("name_key", "acme widgets §ltd".to_owned()),
    ] {
        assert!(evidence.contains(&format!("\"{field}\":\"{value}\"")), "{field} in {evidence}");
    }
    assert!(evidence.contains("\"witnesses\":1") && evidence.contains("\"witness_notices\":[100]"), "{evidence}");
    assert_eq!(
        b.count(&format!(
            "SELECT COUNT(*) FROM org_merge_log WHERE json_extract(evidence, '$.ppon') = '{}' \
               AND json_extract(evidence, '$.witness_notices[0]') = 100",
            k(PPON_P)
        ))
        .await,
        1,
        "the evidence parses as JSON"
    );
    // The change feed.
    for (kind, id, op) in [("organization", 1, "removed"), ("organization", 2, "changed"), ("tender", 7, "changed")] {
        assert_eq!(
            b.count(&format!(
                "SELECT COUNT(*) FROM changes WHERE entity_kind = '{kind}' AND entity_id = {id} AND op = '{op}'"
            ))
            .await,
            1,
            "{kind} {id} {op}"
        );
    }

    // Restart safety: the survivor's mentions carry the PPON now, so the pair
    // is already one, and the (empty) residual holds nothing to merge.
    let again = b.wet(&r.residual_pairs).await;
    assert_eq!((again.already_one, again.both_distinct, again.plan_pairs, again.merged_pairs), (1, 0, 0, 0));
}

/// Only reviewed pairs merge. A pair the live plan carries and the stored plan
/// does not — planned since the dry run — is deferred and counted, its orgs
/// untouched, and it never enters the residual: the next DRY plan puts it up
/// for review.
#[tokio::test]
async fn a_live_pair_the_stored_plan_lacks_is_deferred_never_merged() {
    let b = bed("deferred").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Northgate Ltd")).await;
    assert_eq!(b.plan().await.plan_pairs, 2);
    let r = b.wet(&[pk(COH_A, PPON_P)]).await;
    assert_eq!((r.plan_pairs, r.merged_pairs, r.deferred_unreviewed), (2, 1, 1));
    assert_eq!(r.deferred_pairs, vec![pk(COH_B, PPON_Q)]);
    assert!(r.residual_pairs.is_empty(), "a deferred pair is never carried as reviewed");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (3, 4)").await, 2);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 2").await, 0);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 1);
}

/// Set parity: the live plan and the stored plan may differ by at most
/// max(2% of the stored set, 5) pairs, counted as a symmetric difference.
/// Over it, the run aborts before any write and names both counts; at it, the
/// run goes ahead and merges only the reviewed pair it still plans.
#[tokio::test]
async fn a_wet_run_whose_live_plan_drifted_past_tolerance_aborts_before_any_write() {
    let b = bed("drift").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    let fake = |n: usize| -> Vec<String> { (0..n).map(|i| format!("0000000{i}~FAKE{i:08}")).collect() };
    let before = b.snapshot().await;
    // Six stored pairs gone from the live plan, one live pair not stored: 7 > 5.
    let err = b.db.match_org_altid_pairs(wet_args(&fake(6))).await.expect_err("drift aborts");
    let msg = err.to_string();
    assert!(
        msg.contains("live plan has 1 pairs, the stored plan 6") && msg.contains("7 differ"),
        "{msg}"
    );
    assert!(msg.contains("nothing was written"), "{msg}");
    assert_eq!(b.snapshot().await, before, "an abort writes nothing");

    // Five stale stored pairs and the live one: a drift of exactly 5 passes.
    let mut expect = fake(5);
    expect.push(pk(COH_A, PPON_P));
    expect.sort();
    let r = b.wet(&expect).await;
    assert_eq!((r.expected_pairs, r.expected_not_live, r.deferred_unreviewed, r.merged_pairs), (6, 5, 0, 1));
    assert!(r.residual_pairs.is_empty(), "a stored pair the live plan lost is not carried either");
}

/// A HIGH merge verdict that admitted a pair (here past an uncorroborated name)
/// is stamped applied in the merge's own transaction, and the ledger says the
/// verdict carried it. Stamped, it cannot admit anything again.
#[tokio::test]
async fn a_high_admitting_verdict_is_stamped_applied_by_the_merge() {
    let b = bed("stamp").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Zenith Ltd")).await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!((dry.admitted_verdict, dry.plan_pairs), (1, 1));
    let r = b.wet(&dry.pairs).await;
    assert_eq!(r.merged_pairs, 1);
    assert_eq!(
        b.count(&format!(
            "SELECT COUNT(*) FROM org_merge_verdicts WHERE scheme = 'GB:altid' AND key = '{}' \
               AND applied_at IS NOT NULL AND job_id = 77 AND applied_action = 'merged 1 row(s) into 1'",
            pk(COH_A, PPON_P)
        ))
        .await,
        1
    );
    let evidence = b.text("SELECT evidence FROM org_merge_log WHERE loser = 2").await;
    assert!(evidence.contains("\"verdict\":\"admitted\"") && evidence.contains("\"name_key\":\"\""), "{evidence}");
}

/// The pairs a gate denied become open `e2-altid` edges, tier E2, scored by
/// their witnesses and carrying the gate as `status` — the record of what was
/// NOT merged. An earlier run's edge for a pair merged now turns `merged`
/// instead of going away; nothing else is touched, and a re-run refreshes an
/// edge without resetting its `first_seen` or its state. Nothing is deleted.
#[tokio::test]
async fn denied_pairs_get_open_e2_edges_and_nothing_is_deleted() {
    let b = bed("edges").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Zenith Ltd")).await;
    for (a, bb, rule, tier) in [(50i64, 51i64, "e3-name", "E3"), (1, 2, "e2-altid", "E2")] {
        b.conn
            .execute(
                "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen, job_id)
                 VALUES (?, ?, ?, ?, 1.0, '{}', 1, 1, NULL)",
                (Value::Integer(a), Value::Integer(bb), Value::Text(rule.into()), Value::Text(tier.into())),
            )
            .await
            .unwrap();
    }
    let dry = b.plan().await;
    assert_eq!((dry.plan_pairs, dry.uncorroborated_disjoint), (1, 1));
    assert_eq!(b.count("SELECT COUNT(*) FROM org_candidate_edges").await, 2, "the dry plan writes no edge");

    let r = b.wet(&dry.pairs).await;
    assert_eq!((r.merged_pairs, r.edges_written, r.edges_merged), (1, 1, 1));
    assert_eq!(b.count("SELECT COUNT(*) FROM org_candidate_edges").await, 3);
    assert_eq!(b.text("SELECT state FROM org_candidate_edges WHERE org_a = 1 AND org_b = 2").await, "merged");
    assert_eq!(b.text("SELECT state FROM org_candidate_edges WHERE org_a = 50 AND org_b = 51").await, "open");
    assert_eq!(
        b.count(
            "SELECT COUNT(*) FROM org_candidate_edges WHERE org_a = 3 AND org_b = 4 AND rule = 'e2-altid' \
               AND tier = 'E2' AND state = 'open' AND score = 1.0 AND job_id = 77"
        )
        .await,
        1
    );
    let evidence = b.text("SELECT evidence FROM org_candidate_edges WHERE org_a = 3 AND org_b = 4").await;
    for (field, value) in [
        ("status", "uncorroborated-disjoint".to_owned()),
        ("coh", k(COH_B)),
        ("ppon", k(PPON_Q)),
        ("coh_literal", COH_B.to_owned()),
        ("ppon_literal", PPON_Q.to_owned()),
    ] {
        assert!(evidence.contains(&format!("\"{field}\":\"{value}\"")), "{field} in {evidence}");
    }
    for field in ["\"coh_org\":3", "\"ppon_org\":4", "\"witnesses\":1", "\"coh_partners\":1", "\"first_coh\":1", "\"first_ppon\":0"] {
        assert!(evidence.contains(field), "{field} in {evidence}");
    }
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (3, 4)").await, 2, "a denied pair's orgs stand");

    // A re-run refreshes the denied pair's edge in place.
    b.conn.execute("UPDATE org_candidate_edges SET first_seen = 5 WHERE org_a = 3", ()).await.unwrap();
    let again = b.wet(&r.residual_pairs).await;
    assert_eq!((again.merged_pairs, again.edges_written), (0, 1));
    assert_eq!(b.count("SELECT COUNT(*) FROM org_candidate_edges").await, 3, "an upsert, never a second row");
    assert_eq!(b.count("SELECT first_seen FROM org_candidate_edges WHERE org_a = 3").await, 5);
    assert_eq!(b.text("SELECT state FROM org_candidate_edges WHERE org_a = 1 AND org_b = 2").await, "merged");
}

/// The stop is polled between merge transactions (50 pairs each), so a stopped
/// run stands on a committed prefix: whole pairs merged, the rest untouched,
/// the residual naming exactly the reviewed pairs still to go, and no edge
/// written. The continuation, held against that residual, finishes the job.
#[tokio::test]
async fn a_stop_between_transactions_leaves_the_committed_prefix() {
    let b = bed("stop").await;
    let n = 51i64;
    for i in 0..n {
        b.split(
            1000 + 2 * i,
            (1000 + i, &format!("GB-COH-{:08}", 20_000_000 + i), &format!("Supplier {i} Ltd")),
            (2000 + i, &format!("GB-PPON-PAAA-{i:04}-ZZZZ"), &format!("Supplier {i} Ltd")),
        )
        .await;
    }
    // The planning polls, counted on the dry run: the wet run polls the same
    // ones before its first transaction.
    let calls = AtomicUsize::new(0);
    let counting = || {
        calls.fetch_add(1, Ordering::SeqCst);
        false
    };
    let dry = b.db.match_org_altid_pairs(store::AltIdMergeArgs { stop: &counting, ..args(20) }).await.unwrap();
    assert_eq!(dry.plan_pairs, 51);
    let planning = calls.load(Ordering::SeqCst);
    // Let the first transaction's check pass and stop at the second's.
    let wet_calls = AtomicUsize::new(0);
    let stop = || wet_calls.fetch_add(1, Ordering::SeqCst) > planning;
    let r = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs { stop: &stop, ..wet_args(&dry.pairs) })
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!((r.merged_pairs, r.removed, r.edges_written), (50, 50, 0));
    assert_eq!(r.residual_pairs, vec![dry.pairs[50].clone()], "the one reviewed pair still to go");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, 2 * n - 50);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 50);
    assert_eq!(
        b.count(
            "SELECT COUNT(*) FROM organization_mentions m \
              WHERE NOT EXISTS (SELECT 1 FROM organizations o WHERE o.id = m.organization_id)"
        )
        .await,
        0,
        "no mention points at a deleted org"
    );
    assert_eq!(b.count("SELECT COUNT(*) FROM changes WHERE op = 'removed'").await, 50);
    assert_eq!(b.count("SELECT COUNT(*) FROM changes WHERE op = 'changed'").await, 50);

    let rest = b.wet(&r.residual_pairs).await;
    assert_eq!((rest.already_one, rest.merged_pairs, rest.deferred_unreviewed), (50, 1, 0));
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, n);
}

/// The job's `max_groups` caps the merges; the capped run's residual is the
/// reviewed rest, in the stored plan's order, and a continuation under it
/// passes parity.
#[tokio::test]
async fn a_capped_wet_run_merges_a_prefix_and_its_residual_continues() {
    let b = bed("capped").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Northgate Ltd")).await;
    b.split(300, (5, COH_C, "Southgate Ltd"), (6, PPON_R, "Southgate Ltd")).await;
    let dry = b.plan().await;
    assert_eq!(dry.plan_pairs, 3);
    let r = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs { max_pairs: Some(1), ..wet_args(&dry.pairs) })
        .await
        .unwrap();
    assert_eq!((r.merged_pairs, r.stopped), (1, false));
    assert_eq!(r.residual_pairs, dry.pairs[1..].to_vec());
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 1);
    let rest = b.wet(&r.residual_pairs).await;
    assert_eq!((rest.merged_pairs, rest.deferred_unreviewed), (2, 0));
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, 3);
}

/// One PPON beside two company numbers is a conflict, and only a verdict can
/// plan either pair. Two HIGH verdicts that admit BOTH would fold one PPON org
/// into two company-number orgs; the arm never picks a side, so neither merges
/// and both stay in the residual for the reviewer to settle.
#[tokio::test]
async fn two_admitting_verdicts_over_one_ppon_merge_neither() {
    let b = bed("contradictory").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.org(3, COH_B, "Acme Ltd").await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "B", Some(3), "GB", &[COH_B, PPON_P]).await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    b.verdict(COH_B, PPON_P, vec![2, 3], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!((dry.admitted_verdict, dry.plan_pairs), (2, 2));
    let before = b.snapshot().await;
    let r = b.wet(&dry.pairs).await;
    assert_eq!((r.contradictory, r.merged_pairs, r.edges_written), (2, 0, 0));
    assert_eq!(r.residual_pairs, dry.pairs);
    assert_eq!(b.snapshot().await, before, "nothing merged, nothing denied");
}

/// The review's case: a HIGH verdict posted AFTER the dry run plans a second
/// pair on the same PPON org. That pair is not in the stored set, so it is
/// deferred, but it still contests the org: the reviewed pair must not fold it
/// into its own company-number org and leave the later verdict silently stale.
#[tokio::test]
async fn a_later_admitting_verdict_on_a_deferred_pair_still_contests_the_ppon_org() {
    let b = bed("contested-deferred").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.org(3, COH_B, "Acme Ltd").await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "B", Some(3), "GB", &[COH_B, PPON_P]).await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!(dry.pairs, vec![pk(COH_A, PPON_P)], "B~P is a conflict at dry time");
    b.verdict(COH_B, PPON_P, vec![2, 3], "merge", "high").await;
    let before = b.snapshot().await;
    let r = b.wet(&dry.pairs).await;
    assert_eq!((r.deferred_unreviewed, r.contradictory, r.merged_pairs), (1, 1, 0), "{r:#?}");
    assert_eq!(r.residual_pairs, dry.pairs);
    assert_eq!(b.snapshot().await, before, "the contested org stays where it is");
}

/// A capped or stopped continuation holds its live plan against the residual,
/// whose tolerance is smaller than the first run's. Pairs the earlier run
/// already deferred are not drift a second time: without `known_deferred`, six
/// of them abort a one-pair residual; with it, the reviewed pair merges and the
/// six stay deferred.
#[tokio::test]
async fn a_continuation_does_not_recount_pairs_an_earlier_run_deferred() {
    let b = bed("known-deferred").await;
    let mut keys = Vec::new();
    for i in 0..7i64 {
        let coh = format!("GB-COH-1000000{i}");
        let ppon = format!("GB-PPON-PAAA-000{i}-AAAA");
        let name = format!("Firm{i} Ltd");
        b.split(100 + 10 * i, (10 + 2 * i, &coh, &name), (11 + 2 * i, &ppon, &name)).await;
        keys.push(pk(&coh, &ppon));
    }
    let dry = b.plan().await;
    assert_eq!(dry.plan_pairs, 7);
    let residual = vec![keys[0].clone()];
    let earlier: Vec<String> = keys[1..].to_vec();
    let before = b.snapshot().await;
    let err = b.db.match_org_altid_pairs(wet_args(&residual)).await.expect_err("six unforeseen pairs");
    assert!(err.to_string().contains("parity abort"), "{err}");
    assert_eq!(b.snapshot().await, before);
    let r = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs { known_deferred: earlier, ..wet_args(&residual) })
        .await
        .expect("the continuation proceeds");
    assert_eq!((r.merged_pairs, r.deferred_unreviewed), (1, 6), "{r:#?}");
}

// ---- Unit 3: the resolver alias, and the campaign's full plan listing.

/// The alias's rules, the same miniatures the arm takes above.
fn alias_rules() -> store::AltIdAliasRules {
    store::AltIdAliasRules {
        mention_key,
        consortium,
        legal_family,
        name_key,
        names_agree,
        trim,
        norm,
        owner_mention_cap: store::ALTID_ALIAS_OWNER_MENTIONS,
    }
}

/// A PPON-first mention as the fold hands it to the resolver: the party's
/// FIRST identifier as published, and normalised (the org identifier a mint
/// would store), under GB.
fn ppon_first(notice: i64, literal: &str, name: &str) -> store::Mention {
    store::Mention {
        notice_id: notice,
        section_id: "ORG-N".into(),
        name: name.into(),
        country: Some("GB".into()),
        raw_identifier: Some(literal.into()),
        scheme: Some("GB-PPON".into()),
        identifier: Some(store::Identifier {
            country: Some("GB".into()),
            kind: "national".into(),
            value: minted(literal),
        }),
        variants: Vec::new(),
    }
}

impl Bed {
    /// The issue's shape, then the wet run over its plan: the PPON org is gone,
    /// its mentions are on the company-number org, and the ledger holds the
    /// `e2-altid` row the alias replays.
    async fn merge_all(&self) -> store::AltIdMergeReport {
        let dry = self.plan().await;
        let r = self.wet(&dry.pairs).await;
        assert_eq!(r.merged_pairs, dry.plan_pairs, "{r:#?}");
        r
    }

    /// FTS notices the fold has not reached: one party section each, no mention.
    async fn fresh(&self, notices: &[i64]) {
        for &n in notices {
            self.notice(n, "fts:ocds-1.1").await;
            self.conn
                .execute(
                    "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
                     VALUES (?, 'ORG-N', 'Organization', 'PROCEDURE')",
                    (Value::Integer(n),),
                )
                .await
                .unwrap();
        }
    }

    /// One fold's resolver over `mentions`, the fold's shape in small (the
    /// canonical key and the consortium veto; no anchors), armed with the alias
    /// or not, the wall injected or not.
    async fn resolve_with(
        &self,
        armed: bool,
        hard_scheme: Option<fn(&str) -> bool>,
        cap: usize,
        mentions: &[store::Mention],
    ) -> (Vec<i64>, store::AltIdAliasCounts) {
        self.resolve_ruled(armed.then(alias_rules), hard_scheme, cap, mentions).await
    }

    /// Issue 518: [`Self::resolve`] armed with the owner-name read capped at
    /// `owner_mention_cap` rows.
    async fn resolve_capped(&self, owner_mention_cap: usize, mentions: &[store::Mention]) -> (Vec<i64>, store::AltIdAliasCounts) {
        let rules = store::AltIdAliasRules { owner_mention_cap, ..alias_rules() };
        self.resolve_ruled(Some(rules), None, 20, mentions).await
    }

    async fn resolve_ruled(
        &self,
        rules: Option<store::AltIdAliasRules>,
        hard_scheme: Option<fn(&str) -> bool>,
        cap: usize,
        mentions: &[store::Mention],
    ) -> (Vec<i64>, store::AltIdAliasCounts) {
        let mut resolver = self
            .db
            .mention_resolver(Some(key), Some(consortium), None, Some(norm), None, hard_scheme, cap)
            .await
            .unwrap();
        if let Some(rules) = rules {
            self.db.arm_altid_alias(&mut resolver, rules).await.unwrap();
        }
        let ids = self.db.resolve_mentions(&mut resolver, mentions, 1).await.unwrap();
        let counts = store::Db::altid_alias_counts(&resolver);
        self.db.finish_mention_resolver(resolver).await.unwrap();
        (ids, counts)
    }

    async fn resolve(&self, armed: bool, mentions: &[store::Mention]) -> (Vec<i64>, store::AltIdAliasCounts) {
        self.resolve_with(armed, None, 20, mentions).await
    }
}

/// The alias's reason to exist. After the merge the PPON org is gone, so a new
/// PPON-first mention misses the exact triple AND the canonical key — and the
/// unarmed fold mints the PPON org again. Armed, it binds to the company-number
/// org, mints nothing, and records the mention there.
///
/// Never cached (issue 318's rule): the next mention of the same PPON in the
/// SAME batch is asked again, and its own name — a different company — is
/// refused and mints, as it would unarmed. Had the first bind claimed the
/// PPON's triple, the second would have ridden it onto the company-number org
/// without a name check.
#[tokio::test]
async fn an_armed_resolver_binds_a_ppon_first_mention_to_the_company_number_org_after_a_merge() {
    let b = bed("alias-bind").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    b.fresh(&[300, 301]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;

    let (ids, c) = b
        .resolve(true, &[ppon_first(300, PPON_P, "Acme Widgets Limited"), ppon_first(301, PPON_P, "Zenith Holdings Ltd")])
        .await;
    assert_eq!(ids[0], 2, "the PPON-first mention binds to the company-number org");
    assert_ne!(ids[1], 2, "the same PPON under another company's name is refused");
    assert_eq!(b.count("SELECT organization_id FROM organization_mentions WHERE notice_id = 300").await, 2);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 1, "only the refused mention minted");
    assert_eq!(
        c,
        store::AltIdAliasCounts {
            armed: true,
            ledger_rows: 1,
            aliases: 1,
            asked: 2,
            bound: 1,
            // No wall injected: the bind is lenient, and says so.
            bound_unwalled: 1,
            refused: 1,
            refused_names: 1,
            // Issue 518: one owner, its mention names read once for both asks.
            owner_name_reads: 1,
            ..Default::default()
        }
    );
}

/// Issue 460 review (D5): a from-archive rebuild empties
/// `organization_merged_identifiers` (it names re-minted org ids), and under the
/// alias the rebuilt fold binds the PPON's mentions straight to the
/// company-number org — no PPON org is minted, so no merge ever writes the PPON
/// back and `?identifier=` answered an empty page for good. The bind itself
/// records the PPON as a merged identifier of the org it bound to, once.
#[tokio::test]
async fn an_alias_bind_records_the_ppon_as_a_merged_identifier_of_its_org() {
    let b = bed("alias-records").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    let holders = async || -> Vec<i64> {
        let f = store::read::Filter { identifier: Some(minted(PPON_P)), ..store::read::Filter::default() };
        store::read::organizations(&b.conn, &f, store::read::Scope::Page { after: 0, limit: 10 })
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect()
    };
    assert_eq!(holders().await, vec![2], "the merge wrote the PPON");
    // What a rebuild leaves: the table empty, the ledger (and so the alias) intact.
    b.conn.execute("DELETE FROM organization_merged_identifiers", ()).await.unwrap();
    assert!(holders().await.is_empty());
    let changed_before = b
        .count("SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 2 AND op = 'changed'")
        .await;

    b.fresh(&[300, 301]).await;
    let (ids, c) = b
        .resolve(
            true,
            &[ppon_first(300, PPON_P, "Acme Widgets Limited"), ppon_first(301, PPON_P, "Acme Widgets Ltd")],
        )
        .await;
    assert_eq!(ids, vec![2, 2], "both mentions bind to the company-number org");
    assert_eq!((c.bound, c.recorded), (2, 1), "{c:?}");
    assert_eq!(holders().await, vec![2], "the bind recorded the PPON on the org it bound to");
    assert_eq!(
        b.count("SELECT COUNT(*) FROM organization_merged_identifiers").await,
        1,
        "once, however many mentions bind"
    );
    assert_eq!(
        b.count("SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 2 AND op = 'changed'")
            .await
            - changed_before,
        1,
        "the org's served merged_identifiers grew: one `organization changed`"
    );
    // A fold the merge already covered records nothing: the row stands.
    b.fresh(&[302]).await;
    let (_, c) = b.resolve(true, &[ppon_first(302, PPON_P, "Acme Widgets Ltd")]).await;
    assert_eq!((c.bound, c.recorded), (1, 0), "{c:?}");
}

/// Unarmed, the resolver is the pre-448 one: the same mention after the same
/// merge mints the PPON org again, and the alias reports nothing at all.
#[tokio::test]
async fn an_unarmed_resolver_mints_as_before() {
    let b = bed("alias-unarmed").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    b.fresh(&[300]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b.resolve(false, &[ppon_first(300, PPON_P, "Acme Widgets Limited")]).await;
    assert_ne!(ids[0], 2);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 1, "the re-mint the alias prevents");
    assert_eq!(
        b.text(&format!("SELECT identifier FROM organizations WHERE id = {}", ids[0])).await,
        minted(PPON_P)
    );
    assert_eq!(c, store::AltIdAliasCounts::default(), "not armed, and says so");
}

/// A reviewer's `keep` over the pair — posted after the merge, as the marker of
/// an unwind — drops it from the alias: the PPON is not asked about, and mints.
#[tokio::test]
async fn a_keep_verdict_disables_the_alias() {
    let b = bed("alias-keep").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    b.verdict(COH_A, PPON_P, vec![1, 2], "keep", "high").await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Acme Widgets Limited")]).await;
    assert_ne!(ids[0], 2);
    assert_eq!((c.ledger_rows, c.kept, c.aliases, c.asked, c.bound), (1, 1, 0, 0, 0), "{c:?}");
}

/// A PPON the ledger merged into two company numbers is poisoned: the arm never
/// picks a side of a contradiction, so neither does the fold. Asked, refused,
/// and minted.
#[tokio::test]
async fn a_ppon_merged_into_two_company_numbers_never_aliases() {
    let b = bed("alias-poison").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    // A second e2-altid merge of the same PPON into another company number.
    b.org(3, COH_B, "Acme Widgets Ltd").await;
    b.conn
        .execute(
            "INSERT INTO org_merge_log (keep, loser, rule, evidence, job_id, at) VALUES (3, 4, 'e2-altid', ?, 78, 1)",
            (Value::Text(format!("{{\"scheme\":\"GB:altid\",\"coh\":\"{}\",\"ppon\":\"{}\"}}", k(COH_B), k(PPON_P))),),
        )
        .await
        .unwrap();
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Acme Widgets Limited")]).await;
    assert!(ids[0] != 2 && ids[0] != 3, "neither company-number org takes it");
    assert_eq!((c.ledger_rows, c.poisoned, c.aliases), (2, 1, 0), "{c:?}");
    assert_eq!((c.asked, c.refused, c.refused_poisoned, c.bound), (1, 1, 1, 0), "{c:?}");
}

/// The bind is the arm's bar, never looser. Four merged suppliers, and a
/// PPON-first mention of each whose names fail it:
/// - another company's name;
/// - a plc beside the owner's Ltd (GB legal forms, head against head);
/// - a consortium name;
/// - a formless head whose variant is a plc, beside the owner's Ltd: the form
///   check reads every name, so a formless name cannot bridge them.
/// Each is refused, and each mints exactly as it would unarmed.
#[tokio::test]
async fn a_name_that_fails_corroboration_refuses_and_mints() {
    let b = bed("alias-names").await;
    b.split(100, (1, COH_A, "Acme Widgets Ltd"), (2, PPON_P, "Acme Widgets Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Northgate Ltd")).await;
    b.split(300, (5, COH_C, "Beta Services Ltd"), (6, PPON_R, "Beta Services Ltd")).await;
    b.split(400, (7, COH_D, "Gamma Works Ltd"), (8, PPON_S, "Gamma Works Ltd")).await;
    assert_eq!(b.merge_all().await.merged_pairs, 4);
    b.fresh(&[500, 501, 502, 503]).await;
    let mut formless = ppon_first(503, PPON_S, "Gamma Works");
    formless.variants = vec![("ENG".into(), "Gamma Works plc".into())];
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b
        .resolve(
            true,
            &[
                ppon_first(500, PPON_P, "Zenith Ltd"),
                ppon_first(501, PPON_Q, "Northgate plc"),
                ppon_first(502, PPON_R, "Beta Services Consortium Ltd"),
                formless,
            ],
        )
        .await;
    assert!(ids.iter().all(|id| ![1, 3, 5, 7].contains(id)), "{ids:?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 4);
    assert_eq!((c.asked, c.bound, c.refused), (4, 0, 4), "{c:?}");
    assert_eq!((c.refused_names, c.refused_veto), (2, 2), "{c:?}");
}

/// Identity to identity: the alias maps the PPON key to the company-number KEY,
/// not to an org id, so it finds the company-number org however it was
/// renumbered — a bare rebuild re-mints every org, and keeps `org_merge_log`.
#[tokio::test]
async fn the_alias_survives_the_company_number_orgs_renumbering() {
    let b = bed("alias-renumber").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    // The rebuild's shape: the org layer re-minted under new ids; the ledger
    // still names keep 2 / loser 1, which no longer exist.
    b.conn.execute("DELETE FROM organization_mentions", ()).await.unwrap();
    b.conn.execute("DELETE FROM organization_names", ()).await.unwrap();
    b.conn.execute("DELETE FROM organizations", ()).await.unwrap();
    b.org(50, COH_A, "Acme Widgets Limited").await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "ACME WIDGETS LTD")]).await;
    assert_eq!(ids[0], 50, "found by its key, not by the id the ledger recorded");
    assert_eq!((c.asked, c.bound), (1, 1));
    // And with no standing owner of the company number at all, it is refused.
    b.conn.execute("DELETE FROM organization_mentions", ()).await.unwrap();
    b.conn.execute("DELETE FROM organizations", ()).await.unwrap();
    b.fresh(&[301]).await;
    let (_, c) = b.resolve(true, &[ppon_first(301, PPON_P, "ACME WIDGETS LTD")]).await;
    assert_eq!((c.asked, c.refused_no_owner), (1, 1), "{c:?}");
}

/// The generic wall, through the resolver's own availability rule. Armed (the
/// key index present, no build in flight), a corroborating name more orgs than
/// the cap carry is refused, and a specific one binds with the wall answering.
#[tokio::test]
async fn the_alias_holds_the_generic_wall_when_it_is_armed() {
    let b = bed("alias-wall").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Northgate Ltd")).await;
    assert_eq!(b.merge_all().await.merged_pairs, 2);
    // Three live carriers of `acme ltd`, over a cap of two.
    b.org_in(9, "GB", "GBCOH00000009", "Acme Ltd").await;
    b.org_in(10, "GB", "GBCOH00000010", "Acme Ltd").await;
    for org in [1i64, 9, 10] {
        b.conn
            .execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'acme ltd')",
                (Value::Integer(org),),
            )
            .await
            .unwrap();
    }
    b.conn
        .execute("CREATE INDEX IF NOT EXISTS org_match_keys_kk ON org_match_keys(key_kind, key, org_id)", ())
        .await
        .unwrap();
    b.fresh(&[300, 301]).await;
    let (ids, c) = b
        .resolve_with(
            true,
            Some(|_: &str| false),
            2,
            &[ppon_first(300, PPON_P, "Acme Ltd"), ppon_first(301, PPON_Q, "Northgate Limited")],
        )
        .await;
    assert_ne!(ids[0], 1, "generic: refused, and minted");
    assert_eq!(ids[1], 3, "specific: bound");
    assert_eq!((c.asked, c.bound, c.bound_unwalled, c.refused_generic), (2, 1, 0, 1), "{c:?}");
}

/// The campaign's prerequisite: the `plan` listing carries every planned pair a
/// wet run would merge, up to its own cap — far above R2's 500 in production —
/// with the witnesses' publication ids, and the cut stays honest when it binds.
/// The denied and conflict listings keep R2's cap.
#[tokio::test]
async fn the_plan_listing_carries_every_planned_pair_up_to_its_own_cap() {
    assert!(store::ALTID_PLAN_LISTING_CAP >= 20_000, "well above any realistic plan");
    assert_eq!(store::R2_PLAN_LISTING_CAP, 500, "R2's, E0's and R3's listings are untouched");
    let b = bed("listing-cap").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Northgate Ltd")).await;
    b.split(300, (5, COH_C, "Beta Ltd"), (6, PPON_R, "Beta Ltd")).await;
    let full = b.plan().await;
    assert_eq!((full.plan_pairs, full.plan_listing.len(), full.plan_listing_truncated), (3, 3, false));
    assert!(full.plan_listing.iter().all(|l| l.witness_publications.len() == 1), "{:#?}", full.plan_listing);
    let cut = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs { plan_listing_cap: 2, ..args(20) })
        .await
        .unwrap();
    assert_eq!((cut.plan_pairs, cut.pairs.len(), cut.plan_listing.len()), (3, 3, 2));
    assert!(cut.plan_listing_truncated, "a cut listing says so");
}

// ---- Issue 452: a wrong-number verdict withholds the company number.

impl Bed {
    /// Record one issue-452 identifier verdict on `org`'s `identifier`.
    async fn identifier_verdict(&self, org: i64, identifier: &str, verdict: &str) -> store::IdentifierVerdictReport {
        self.db
            .record_identifier_verdicts(
                "452-test",
                &[store::IdentifierVerdict {
                    org_id: org,
                    identifier: identifier.into(),
                    verdict: verdict.into(),
                    correct_identifier: None,
                    rationale: "fixture".into(),
                    confidence: "high".into(),
                }],
                0,
            )
            .await
            .unwrap()
    }
}

/// The 448 campaign's Harvey Nash shape: the company-number org is keyed by a
/// transposed number, so folding the PPON org (which carries the RIGHT identity)
/// into it would put a correct supplier under someone else's number. A `wrong`
/// verdict on that number takes the org out of the owner walk: the pair finds no
/// company-number org and is never planned. A verdict on a number the org does
/// not carry is inert. A `related` verdict withholds too (issue 466): the number is a
/// related company's, so folding the PPON org into the org under it would put the
/// supplier under its parent's or sister's number — until the re-key arm moves the
/// org onto its own number, which ends the withholding (`applied_at`).
#[tokio::test]
async fn a_wrong_number_verdict_keeps_the_company_number_org_from_owning_its_pair() {
    let b = bed("withheld-owner").await;
    b.split(100, (2, COH_A, "Harvey Nash Ltd"), (1, PPON_P, "HARVEY NASH LTD")).await;
    assert_eq!(b.plan().await.plan_pairs, 1, "the control: planned");

    let stale = b.identifier_verdict(2, "GBCOH99999999", "wrong").await;
    assert_eq!((stale.recorded, stale.stale), (0, 1), "not the number org 2 carries");
    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.withheld), (1, 0), "a verdict on another number changes nothing");

    let live = b.identifier_verdict(2, &minted(COH_A), "wrong").await;
    assert_eq!((live.recorded, live.stale), (1, 0));
    let r = b.plan().await;
    assert_eq!((r.withheld, r.owners, r.both_distinct, r.no_target_coh), (1, 1, 0, 1), "{r:#?}");
    assert_eq!(r.plan_pairs, 0, "no org owns the withheld number, so nothing folds into it");

    b.identifier_verdict(2, &minted(COH_A), "related").await;
    let r = b.plan().await;
    assert_eq!((r.plan_pairs, r.withheld), (0, 1), "a re-review replaces the verdict; related withholds too");
}

/// The alias finds the company-number org through the resolver's canonical map,
/// and a withheld org is not in it: after a merge, a wrong-number verdict on the
/// survivor stops the alias handing it new PPON-first mentions.
#[tokio::test]
async fn the_alias_never_binds_to_a_withheld_company_number_org() {
    let b = bed("withheld-alias").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    b.identifier_verdict(2, &minted(COH_A), "wrong").await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Acme Widgets Limited")]).await;
    assert_ne!(ids[0], 2, "refused: the number is not trusted to name an owner");
    assert_eq!((c.asked, c.bound, c.refused, c.refused_no_owner), (1, 0, 1, 1), "{c:?}");
}

/// Issue 469 unit 1: the other GB registers are measured, never planned. Leeds Teaching
/// Hospitals publishes its NHS code and its PPON on one party, and each keys its own org:
/// counted `both_distinct` under `GB-NHS`, sampled with both names, and left alone. A
/// UKPRN pair one org already serves, a PPON beside two charity numbers, and the
/// company-number control beside them all read as before.
#[tokio::test]
async fn an_nhs_ppon_pair_is_counted_and_never_planned() {
    let b = bed("registry-pairs").await;
    // The control: a company-number pair, planned exactly as without the registers.
    b.split(100, (2, COH_A, "Harvey Nash Ltd"), (1, PPON_P, "HARVEY NASH LTD")).await;
    // Leeds: NHS-first on one notice (bound to the NHS org), PPON-first on the next.
    b.org(10, "GB-NHS-RR8", "Leeds Teaching Hospitals NHS Trust").await;
    b.org(11, PPON_Q, "Leeds Teaching Hospitals NHS Trust").await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "L1", Some(10), "GB", &["GB-NHS-RR8", PPON_Q]).await;
    b.notice(201, "fts:ocds-1.1").await;
    b.party(201, "L2", Some(11), "GB", &[PPON_Q]).await;
    // Bradford College: one org already serves its UKPRN, and no org carries its PPON.
    b.org(20, "GB-UKPRN-10000840", "Bradford College").await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "B1", Some(20), "GB", &["GB-UKPRN-10000840", PPON_R]).await;
    // A party with two charity numbers beside one PPON: the first one stands (the
    // mention bound to it), the second does not, and no org carries the PPON.
    b.org(21, "GB-CHC-216250", "Barnardo's").await;
    b.notice(400, "fts:ocds-1.1").await;
    b.party(400, "C1", Some(21), "GB", &["GB-CHC-216250", "GB-CHC-1234567", PPON_S]).await;

    let r = b.plan().await;
    assert_eq!(r.plan_pairs, 1, "only the company-number pair is planned: {r:#?}");
    assert_eq!((r.pairs_seen, r.both_distinct), (1, 1), "the COH counters see no register");
    let nhs = r.registry_pairs.get("GB-NHS").expect("NHS counted");
    assert_eq!((nhs.literal_pairs, nhs.pairs, nhs.no_identity, nhs.both_distinct), (1, 1, 0, 1));
    let ukprn = r.registry_pairs.get("GB-UKPRN").expect("UKPRN counted");
    assert_eq!((ukprn.pairs, ukprn.no_target_ppon, ukprn.both_distinct), (1, 1, 0));
    let chc = r.registry_pairs.get("GB-CHC").expect("CHC counted");
    assert_eq!(
        (chc.pairs, chc.ppon_beside_two_values, chc.no_target_ppon, chc.no_target_both),
        (2, 1, 1, 1)
    );
    assert_eq!(r.registry_sample.len(), 1);
    let s = &r.registry_sample[0];
    assert_eq!((s.scheme.as_str(), s.key.clone()), ("GB-NHS", format!("GB-NHS-RR8~{}", k(PPON_Q))));
    let members: Vec<(i64, Option<&str>, &str)> =
        s.members.iter().map(|m| (m.org_id, m.identifier.as_deref(), m.name.as_str())).collect();
    assert_eq!(
        members,
        vec![
            (10, Some("GBNHSRR8"), "Leeds Teaching Hospitals NHS Trust"),
            (11, Some(minted(PPON_Q).as_str()), "Leeds Teaching Hospitals NHS Trust"),
        ],
        "R2's listing shape: the register org first"
    );
    assert_eq!(s.notices, vec!["pub-200".to_owned()]);
    // Issue 469 U4: NHS waits for U7, so the register planner denies it on shape and
    // plans nothing; the other schemes have no split to plan.
    assert!(r.registry_pairs_planned.is_empty(), "{r:#?}");
    assert_eq!(r.registry_gates.keys().collect::<Vec<_>>(), vec!["GB-NHS"]);
    let g = &r.registry_gates["GB-NHS"];
    assert_eq!((g.candidates, g.denied_registry_shape), (1, 1), "{g:#?}");
    assert_eq!(r.registry_denied_listing.len(), 1);
    let d = &r.registry_denied_listing[0];
    assert_eq!((d.key(), d.gate.as_str()), (format!("GB-NHS:GBNHSRR8~{}", k(PPON_Q)), "registry-shape"));
    // Issue 513 unit 1: the NHS pair is listed with its identity, the class it was
    // counted under and both holders; no other scheme's pair is listed.
    assert_eq!(r.registry_nhs_listing.len(), 1, "{r:#?}");
    assert!(!r.registry_nhs_listing_truncated);
    let e = &r.registry_nhs_listing[0];
    assert_eq!(
        (e.literal.as_str(), e.value.as_str(), e.identity.as_deref(), e.class, e.ppon.clone()),
        ("GB-NHS-RR8", "GBNHSRR8", Some("GBNHSRR8"), "both_distinct", k(PPON_Q))
    );
    let leeds = "Leeds Teaching Hospitals NHS Trust";
    assert_eq!(holders(&e.registry_holders), vec![(10, Some("GBNHSRR8".to_owned()), leeds.to_owned(), false)]);
    assert_eq!(holders(&e.ppon_holders), vec![(11, Some(minted(PPON_Q)), leeds.to_owned(), false)]);
    assert_eq!((e.registry_holder_count, e.ppon_holder_count, e.witnesses), (1, 1, 1));
    assert_eq!(e.witness_publications, vec!["pub-200".to_owned()]);
}

/// Issue 513 unit 1: a listed holder as (org id, identifier, name, provisional).
fn holders(hs: &[store::RegistryNhsHolder]) -> Vec<(i64, Option<String>, String, bool)> {
    hs.iter().map(|h| (h.org_id, h.identifier.clone(), h.name.clone(), h.provisional)).collect()
}

/// Issue 469 review: a register value the resolver mints no identity for (a
/// letter-only NHS code) is held by the org its register-first mention bound to
/// by name — never looked up as an identifier, which no org carries. And a pair
/// one org already serves, by binding or by its mentions, is `already_one`.
#[tokio::test]
async fn a_register_value_with_no_identity_is_held_by_the_org_its_mention_bound() {
    let b = bed("registry-identity").await;
    // Barts: `GB-NHS-QWO` mints nothing, so the party bound a provisional org by name.
    b.provisional(30, "Barts Health NHS Trust").await;
    b.org(31, PPON_P, "Barts Health NHS Trust").await;
    b.notice(500, "fts:ocds-1.1").await;
    b.party(500, "Q1", Some(30), "GB", &["GB-NHS-QWO", PPON_P]).await;
    // Brighton: the code's mention bound the PPON org itself — one org already.
    b.org(40, PPON_Q, "Brighton Hospitals").await;
    b.notice(600, "fts:ocds-1.1").await;
    b.party(600, "R1", Some(40), "GB", &["GB-NHS-RXH", PPON_Q]).await;
    // A college whose UKPRN org was merged into its PPON org: the pair is witnessed
    // PPON-first, no org carries the UKPRN, and the PPON org's mentions carry it.
    b.org(50, PPON_R, "Leeds College").await;
    b.notice(700, "fts:ocds-1.1").await;
    b.party(700, "U1", Some(50), "GB", &[PPON_R, "GB-UKPRN-10003841"]).await;
    b.notice(701, "fts:ocds-1.1").await;
    b.party(701, "U2", Some(50), "GB", &["GB-UKPRN-10003841"]).await;
    // A charity whose PPON org was merged into its charity org: register-first, no
    // org carries the PPON, and the charity org's mentions carry it.
    b.org(60, "GB-CHC-1112223", "Oxfam").await;
    b.notice(800, "fts:ocds-1.1").await;
    b.party(800, "H1", Some(60), "GB", &["GB-CHC-1112223", PPON_S]).await;
    b.notice(801, "fts:ocds-1.1").await;
    b.party(801, "H2", Some(60), "GB", &[PPON_S]).await;

    let r = b.plan().await;
    assert_eq!(r.plan_pairs, 0, "{r:#?}");
    let nhs = r.registry_pairs.get("GB-NHS").expect("NHS counted");
    assert_eq!(
        (nhs.pairs, nhs.no_identity, nhs.both_distinct, nhs.already_one, nhs.no_target_registry),
        (2, 2, 1, 1, 0),
        "{nhs:#?}"
    );
    let ukprn = r.registry_pairs.get("GB-UKPRN").expect("UKPRN counted");
    assert_eq!((ukprn.pairs, ukprn.no_identity, ukprn.already_one), (1, 0, 1), "{ukprn:#?}");
    let chc = r.registry_pairs.get("GB-CHC").expect("CHC counted");
    assert_eq!((chc.pairs, chc.already_one, chc.no_target_ppon), (1, 1, 0), "{chc:#?}");
    assert_eq!(r.registry_sample.len(), 1);
    let s = &r.registry_sample[0];
    assert_eq!(s.key, format!("GB-NHS-QWO~{}", k(PPON_P)));
    let members: Vec<(i64, Option<&str>)> = s.members.iter().map(|m| (m.org_id, m.identifier.as_deref())).collect();
    assert_eq!(members, vec![(30, None), (31, Some(minted(PPON_P).as_str()))]);
    // Issue 469 U4: a value that mints no identity fails the shape gate, its listing
    // keyed by the value's uppercase alphanumerics.
    assert!(r.registry_pairs_planned.is_empty(), "{r:#?}");
    let barts = r.registry_denied_listing.iter().find(|l| l.registry_literal == "GB-NHS-QWO").expect("Barts listed");
    assert_eq!((barts.gate.as_str(), barts.registry.as_str()), ("registry-shape", "GBNHSQWO"));
    // Issue 513 unit 1: BOTH NHS pairs are listed, in value order — the split one the
    // sample showed and the one-org one it did not — each with no identity and the
    // class unit 1 counted; Barts's register side is the provisional org its mention
    // bound by name, carrying no identifier.
    let listed: Vec<(&str, Option<&str>, &str, String)> = r
        .registry_nhs_listing
        .iter()
        .map(|e| (e.literal.as_str(), e.identity.as_deref(), e.class, e.ppon.clone()))
        .collect();
    assert_eq!(
        listed,
        vec![("GB-NHS-QWO", None, "both_distinct", k(PPON_P)), ("GB-NHS-RXH", None, "already_one", k(PPON_Q))],
        "{r:#?}"
    );
    let (barts, brighton) = (&r.registry_nhs_listing[0], &r.registry_nhs_listing[1]);
    assert_eq!(barts.value, "GBNHSQWO");
    assert_eq!(holders(&barts.registry_holders), vec![(30, None, "Barts Health NHS Trust".to_owned(), true)]);
    assert_eq!(holders(&barts.ppon_holders), vec![(31, Some(minted(PPON_P)), "Barts Health NHS Trust".to_owned(), false)]);
    assert_eq!(barts.witness_publications, vec!["pub-500".to_owned()]);
    let one = vec![(40, Some(minted(PPON_Q)), "Brighton Hospitals".to_owned(), false)];
    assert_eq!((holders(&brighton.registry_holders), holders(&brighton.ppon_holders)), (one.clone(), one));
}

/// Issue 513 unit 1: a letter-only code bound by name to several provisional orgs —
/// spellings of one trust's name, each its own org — is `multi_target`, which no sample
/// shows. The listing names every holder up to its cap of five, and counts all six, so
/// the owner can tell name variants of one body from different bodies under one code.
#[tokio::test]
async fn a_multi_target_nhs_code_lists_each_holder() {
    let b = bed("registry-nhs-multi").await;
    let names = [
        "Oxford University Hospitals NHS Foundation Trust",
        "Oxford University Hospitals NHS FT",
        "OUH NHS Foundation Trust",
        "Oxford University Hospitals",
        "Oxford Univ Hospitals NHS Trust",
        "John Radcliffe Hospital",
    ];
    for (i, name) in names.iter().enumerate() {
        let (org, notice) = (90 + i as i64, 1000 + i as i64);
        b.provisional(org, name).await;
        b.notice(notice, "fts:ocds-1.1").await;
        b.party(notice, &format!("O{i}"), Some(org), "GB", &["GB-NHS-RTH"]).await;
    }
    // The trust's PPON org, and the one notice pairing the two (PPON-first).
    b.org(99, PPON_P, "Oxford University Hospitals NHS Foundation Trust").await;
    b.notice(1100, "fts:ocds-1.1").await;
    b.party(1100, "P", Some(99), "GB", &[PPON_P, "GB-NHS-RTH"]).await;

    let r = b.plan().await;
    let nhs = r.registry_pairs.get("GB-NHS").expect("NHS counted");
    assert_eq!((nhs.pairs, nhs.no_identity, nhs.multi_target), (1, 1, 1), "{nhs:#?}");
    assert!(r.registry_sample.is_empty(), "a multi-target pair is never sampled: {r:#?}");
    assert_eq!(r.registry_nhs_listing.len(), 1, "{r:#?}");
    let e = &r.registry_nhs_listing[0];
    assert_eq!(
        (e.literal.as_str(), e.identity.as_deref(), e.class, e.ppon.clone()),
        ("GB-NHS-RTH", None, "multi_target", k(PPON_P))
    );
    let expected: Vec<(i64, Option<String>, String, bool)> =
        names.iter().take(5).enumerate().map(|(i, n)| (90 + i as i64, None, (*n).to_owned(), true)).collect();
    assert_eq!(holders(&e.registry_holders), expected, "the first five, ascending");
    assert_eq!(e.registry_holder_count, 6, "the cap is honest about the sixth");
    assert_eq!(
        holders(&e.ppon_holders),
        vec![(99, Some(minted(PPON_P)), "Oxford University Hospitals NHS Foundation Trust".to_owned(), false)]
    );
    assert_eq!((e.ppon_holder_count, e.witnesses), (1, 1));
    assert_eq!(e.witness_publications, vec!["pub-1100".to_owned()]);
    // A measurement: nothing plans.
    assert!(r.registry_pairs_planned.is_empty(), "{r:#?}");
}

/// Issue 469 review 2: a side is held by every org a mention bound where that side
/// came FIRST, on any notice — a letter-only code's own notices bind it by name even
/// when every pair witness lists the PPON first — and by the survivor a merge left
/// the side on (`organization_merged_identifiers`).
#[tokio::test]
async fn a_side_is_held_through_its_own_notices_and_through_a_merge() {
    let b = bed("registry-holders").await;
    // Whittington: `GB-NHS-RKE` alone binds a provisional org by name; the only pair
    // witness lists the PPON first and binds the PPON org.
    b.provisional(70, "Whittington Health").await;
    b.org(71, PPON_P, "Whittington Health").await;
    b.notice(900, "fts:ocds-1.1").await;
    b.party(900, "W1", Some(70), "GB", &["GB-NHS-RKE"]).await;
    b.notice(901, "fts:ocds-1.1").await;
    b.party(901, "W2", Some(71), "GB", &[PPON_P, "GB-NHS-RKE"]).await;
    // Oxfam: the 448 arm merged its PPON org into its company-number org, which now
    // holds the PPON only as a merged identifier; its charity org stands apart.
    b.org(80, COH_A, "Oxfam GB").await;
    b.org(81, "GB-CHC-202918", "Oxfam").await;
    b.conn
        .execute(
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'GB', 80, 999, 'e2-altid')",
            (Value::Text(minted(PPON_Q)),),
        )
        .await
        .unwrap();
    b.notice(910, "fts:ocds-1.1").await;
    b.party(910, "X1", Some(80), "GB", &[COH_A, "GB-CHC-202918", PPON_Q]).await;

    let r = b.plan().await;
    let nhs = r.registry_pairs.get("GB-NHS").expect("NHS counted");
    assert_eq!(
        (nhs.pairs, nhs.no_identity, nhs.both_distinct, nhs.no_target_registry),
        (1, 1, 1, 0),
        "{nhs:#?}"
    );
    let chc = r.registry_pairs.get("GB-CHC").expect("CHC counted");
    assert_eq!((chc.pairs, chc.both_distinct, chc.no_target_ppon), (1, 1, 0), "{chc:#?}");
    let held: Vec<(&str, Vec<i64>)> = r
        .registry_sample
        .iter()
        .map(|s| (s.scheme.as_str(), s.members.iter().map(|m| m.org_id).collect()))
        .collect();
    assert_eq!(held, vec![("GB-CHC", vec![81, 80]), ("GB-NHS", vec![70, 71])]);
}

// ---- Issue 469 U4: the dry register planner.

/// The issue's register shape, planned: one college split across its UKPRN org and its
/// PPON org, names agreeing. The register org survives (U5's rule) though it has the
/// HIGHER id, and the members still list ascending so the listing round-trips into a
/// merge verdict. The college is its own witness notice's buyer: one publisher, and a
/// self-statement. The company-number side sees nothing.
#[tokio::test]
async fn an_ukprn_ppon_pair_is_planned_and_keeps_the_ukprn_org() {
    let b = bed("registry-plan").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE")).await;
    for sql in [
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
         VALUES (7, 1, 100, 0, 'pub-100')",
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 'buyer', 2, 100, 'ORG-R100')",
    ] {
        b.conn.execute(sql, ()).await.unwrap();
    }

    let r = b.plan().await;
    assert_eq!(r.registry_pairs_planned, vec![rk(UKPRN_A, PPON_P)], "{r:#?}");
    assert_eq!(rk(UKPRN_A, PPON_P), format!("GB-UKPRN:GBUKPRN10007798~{}", k(PPON_P)));
    let l = &r.registry_plan_listing[0];
    assert_eq!((l.gate.as_str(), l.keep, l.loser), ("plan", Some(2), Some(1)), "{l:#?}");
    assert_eq!(l.members.iter().map(|m| m.org_id).collect::<Vec<_>>(), vec![1, 2], "ascending, verdict-ready");
    assert_eq!(l.key(), rk(UKPRN_A, PPON_P));
    assert_eq!(
        (l.registry_scheme.as_str(), l.registry.as_str(), l.registry_literal.as_str()),
        ("GB-UKPRN", "GBUKPRN10007798", UKPRN_A)
    );
    assert_eq!((l.ppon.clone(), l.ppon_literal.as_str()), (k(PPON_P), PPON_P));
    assert_eq!((l.witnesses, l.publishers, l.self_stated), (1, 1, 1), "{l:#?}");
    assert_eq!((l.ppon_partners, l.registry_partners, l.cooccurring), (1, 1, 0));
    assert!(l.coherent && l.corroborated_by.is_some(), "{l:#?}");
    assert_eq!(l.witness_publications, vec!["pub-100".to_owned()]);
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.candidates, g.planned, g.denied(), g.conflicts), (1, 1, 0, 0), "{g:#?}");
    assert!(r.registry_denied_listing.is_empty(), "{r:#?}");
    assert!(r.pairs.is_empty() && r.plan_pairs == 0 && r.both_distinct == 0, "the company-number side is untouched");
}

/// A PPON beside a company number AND a register is the charitable-company shape, a hard
/// deny whichever way it shows: (a) on one party; (b) across the graph, one party pairing
/// the PPON with the charity and another with a company number; (c) through the holder,
/// a 448 survivor that holds the PPON as a merged identifier, though no party shows the
/// two together. (c)'s holder also carries company-number mentions, so the gate ORDER is
/// pinned: the graph gates run first and it reads `triple`, not `loser-incoherent`.
#[tokio::test]
async fn a_ppon_beside_a_company_number_and_a_charity_is_a_triple() {
    let b = bed("registry-triple").await;
    // (a) The party form.
    b.org(1, CHC_X, "Helping Hands Trust").await;
    b.org(2, PPON_P, "Helping Hands Trust").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "A1", Some(1), "GB", &[CHC_X, PPON_P, COH_A]).await;
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "A2", Some(2), "GB", &[PPON_P]).await;
    // (b) The graph form.
    b.org(3, CHC_Y, "Riverside Care").await;
    b.org(4, PPON_Q, "Riverside Care").await;
    b.provisional(5, "Riverside Care Ltd").await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "B1", Some(3), "GB", &[CHC_Y, PPON_Q]).await;
    b.notice(201, "fts:ocds-1.1").await;
    b.party(201, "B2", Some(4), "GB", &[PPON_Q]).await;
    b.notice(202, "fts:ocds-1.1").await;
    b.party(202, "B3", Some(5), "GB", &[COH_B, PPON_Q]).await;
    // (c) The merged-holder form.
    b.org(80, COH_C, "Oxfam GB").await;
    b.org(81, CHC_Z, "Oxfam").await;
    b.conn
        .execute(
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'GB', 80, 999, 'e2-altid')",
            (Value::Text(minted(PPON_R)),),
        )
        .await
        .unwrap();
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "C1", Some(81), "GB", &[CHC_Z, PPON_R]).await;
    b.notice(301, "fts:ocds-1.1").await;
    b.party(301, "C2", Some(80), "GB", &[COH_C]).await;

    let r = b.plan().await;
    assert_eq!(r.registry_pairs["GB-CHC"].ppon_beside_coh, 1, "unit 1 reads the party form only");
    assert_eq!(r.registry_triple_ppons, 2, "the party and graph forms show a company number beside the PPON");
    let g = &r.registry_gates["GB-CHC"];
    assert_eq!((g.candidates, g.denied_triple, g.denied_loser_incoherent, g.planned), (3, 3, 0, 0), "{g:#?}");
    let mut gates: Vec<(String, &str)> =
        r.registry_denied_listing.iter().map(|l| (l.ppon.clone(), l.gate.as_str())).collect();
    gates.sort();
    let mut want = vec![(k(PPON_P), "triple"), (k(PPON_Q), "triple"), (k(PPON_R), "triple")];
    want.sort();
    assert_eq!(gates, want);
    let held = r.registry_denied_listing.iter().find(|l| l.ppon == k(PPON_R)).expect("listed");
    assert_eq!((held.keep, held.loser), (Some(81), Some(80)), "the 448 survivor would have been the loser");
}

/// One PPON beside two registers (a housing association's number on one notice, a
/// college's on another) is two statements of which one is wrong: a structural deny for
/// both pairs, which no verdict overrides.
#[tokio::test]
async fn a_ppon_beside_two_registers_is_cross_register() {
    let b = bed("registry-cross").await;
    b.org(1, MPR_M, "Riverside Housing").await;
    b.org(3, UKPRN_A, "Riverside College").await;
    b.org(9, PPON_P, "Riverside Housing").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "M", Some(1), "GB", &[MPR_M, PPON_P]).await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "U", Some(3), "GB", &[UKPRN_A, PPON_P]).await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "P", Some(9), "GB", &[PPON_P]).await;
    b.registry_verdict(&rk(MPR_M, PPON_P), vec![1, 9], "merge", "high").await;

    let r = b.plan().await;
    assert_eq!((r.registry_cross_register_ppons, r.registry_cross_register_denied_ppons), (1, 1));
    for scheme in ["GB-MPR", "GB-UKPRN"] {
        let g = &r.registry_gates[scheme];
        assert_eq!((g.candidates, g.denied_cross_register, g.admitted_verdict, g.planned), (1, 1, 0, 0), "{scheme}");
    }
    assert_eq!(r.registry_denied_listing.len(), 2);
    assert!(
        r.registry_denied_listing.iter().all(|l| l.gate == "cross-register" && l.ppon_partners == 2),
        "{:#?}",
        r.registry_denied_listing
    );
    assert!(r.registry_pairs_planned.is_empty());
}

/// One party publishing two UKPRNs beside a PPON (unit 1's per-party
/// `ppon_beside_two_values`). The decision named this a conflict; U2 made the
/// PPON's partner set a STRUCTURAL gate, so both pairs are `cross-register` and a
/// reviewer's HIGH verdict cannot admit either. (The judgment flag is the mirror,
/// `registry-multi-ppon`: `a_ukprn_beside_two_ppons_is_a_conflict_a_high_verdict_admits_it`.)
#[tokio::test]
async fn a_ppon_beside_two_ukprns_is_a_conflict() {
    let b = bed("registry-two-ukprns").await;
    b.org(1, UKPRN_A, "Leeds City College").await;
    b.org(2, PPON_P, "Leeds City College").await;
    b.org(3, UKPRN_B, "Leeds City College").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "X", Some(1), "GB", &[UKPRN_A, UKPRN_B, PPON_P]).await;
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(2), "GB", &[PPON_P]).await;
    b.registry_verdict(&rk(UKPRN_A, PPON_P), vec![1, 2], "merge", "high").await;

    let r = b.plan().await;
    assert_eq!(r.registry_pairs["GB-UKPRN"].ppon_beside_two_values, 1, "unit 1's per-party reading");
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!(
        (g.candidates, g.denied_cross_register, g.admitted_verdict, g.conflicts, g.planned),
        (2, 2, 0, 0, 0),
        "{g:#?}"
    );
    assert!(r.registry_denied_listing.iter().all(|l| l.gate == "cross-register"));
}

/// A register side held only by the org a mention bound to by name — no org carries
/// the identity — is a split unit 1 counts, but it never plans: the register org U5
/// would keep must hold the number itself.
#[tokio::test]
async fn a_register_side_held_only_by_the_name_bound_org_never_plans() {
    let b = bed("registry-name-bound").await;
    b.provisional(30, "Leeds City College").await;
    b.org(31, PPON_P, "Leeds City College").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "R", Some(30), "GB", &[UKPRN_A, PPON_P]).await;
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(31), "GB", &[PPON_P]).await;

    let r = b.plan().await;
    assert_eq!(r.registry_pairs["GB-UKPRN"].both_distinct, 1, "unit 1 counts the split");
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.denied_single_owner, g.planned), (1, 0), "{g:#?}");
    assert_eq!(r.registry_denied_listing[0].gate, "single-owner");
}

/// Issue 452's withholding reaches the register family on both sides: a `wrong`
/// verdict on the register org's number, or on the PPON org's, leaves that side held
/// only by the org its own notice bound — still a split, never planned.
#[tokio::test]
async fn a_withheld_register_owner_never_plans() {
    let b = bed("registry-withheld").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "Leeds City College")).await;
    assert_eq!(b.plan().await.registry_pairs_planned, vec![rk(UKPRN_A, PPON_P)], "the control: planned");

    b.identifier_verdict(2, &minted(UKPRN_A), "wrong").await;
    let r = b.plan().await;
    assert_eq!(r.registry_pairs["GB-UKPRN"].both_distinct, 1, "its own notice still binds it: {r:#?}");
    assert_eq!((r.registry_gates["GB-UKPRN"].denied_single_owner, r.registry_pairs_planned.len()), (1, 0));

    // A re-review clears it; then the PPON org's number is the wrong one.
    b.identifier_verdict(2, &minted(UKPRN_A), "right").await;
    assert_eq!(b.plan().await.registry_pairs_planned.len(), 1);
    b.identifier_verdict(1, &minted(PPON_P), "wrong").await;
    let r = b.plan().await;
    assert_eq!((r.registry_gates["GB-UKPRN"].denied_single_owner, r.registry_pairs_planned.len()), (1, 0), "{r:#?}");
}

/// The register channel of the evidence wall: the company-number channel keys no
/// register raw, so without it two different UKPRNs pass. The PPON org naming another
/// UKPRN is `loser-incoherent`; the register org naming a third one as well is two
/// registrations in one series, `evidence-wall`.
#[tokio::test]
async fn the_register_wall_denies_two_different_ukprns() {
    let b = bed("registry-wall").await;
    b.registry_split(100, (1, UKPRN_A, "Leeds City College"), (2, PPON_P, "Leeds City College")).await;
    assert_eq!(b.plan().await.registry_gates["GB-UKPRN"].planned, 1, "the control");

    b.notice(103, "fts:ocds-1.1").await;
    b.party(103, "W2", Some(2), "GB", &[UKPRN_C]).await;
    let r = b.plan().await;
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.denied_loser_incoherent, g.denied_evidence_wall, g.planned), (1, 0, 0), "{g:#?}");

    b.notice(102, "fts:ocds-1.1").await;
    b.party(102, "W1", Some(1), "GB", &[UKPRN_B]).await;
    let r = b.plan().await;
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.denied_evidence_wall, g.denied_loser_incoherent, g.planned), (1, 0, 0), "{g:#?}");

    let blind = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs { registry_mention_key: |_, _| None, ..args(20) })
        .await
        .unwrap();
    assert_eq!(blind.registry_gates["GB-UKPRN"].planned, 1, "only the register channel sees them: {blind:#?}");
}

/// The company-number half of a report: everything but the register fields and the
/// harvest counters that added notices and orgs move.
fn coh_view(r: &store::AltIdMergeReport) -> String {
    let mut r = r.clone();
    r.fts_notices = 0;
    r.party_sections = 0;
    r.scanned = 0;
    r.unkeyed_scheme.clear();
    r.registry_pairs.clear();
    r.registry_sample.clear();
    // Issue 513 unit 1: the GB-NHS listing, a register field.
    r.registry_nhs_listing.clear();
    r.registry_nhs_listing_truncated = false;
    r.registry_gates.clear();
    r.registry_cross_register_ppons = 0;
    r.registry_cross_register_denied_ppons = 0;
    r.registry_triple_ppons = 0;
    r.registry_pairs_planned.clear();
    r.registry_plan_listing.clear();
    r.registry_plan_listing_truncated = false;
    r.registry_denied_listing.clear();
    r.registry_denied_listing_truncated = false;
    // Issue 469 U5: the register family's wet fields, and the edges they add.
    r.registry_expected_pairs = 0;
    r.registry_deferred_unreviewed = 0;
    r.registry_deferred_pairs.clear();
    r.registry_expected_not_live = 0;
    r.registry_contradictory = 0;
    r.registry_unconfirmed = 0;
    r.registry_merged_pairs = 0;
    r.registry_residual_pairs.clear();
    r.edges_written -= r.registry_edges_written;
    r.registry_edges_written = 0;
    r.registry_merged.clear();
    r.registry_regrown.clear();
    format!("{r:#?}")
}

/// Unit 1's contract, kept by U4: the register graph never moves a company-number count.
/// A company-number fixture with a planned pair, a legal-form denial and a
/// `ppon-multi-coh` conflict reads the same before and after register pairs of every
/// outcome land beside it — one of them on the planned pair's own PPON — and the same
/// again with the register key disabled. The wet run merges only the company-number
/// pair: since U5 it re-plans the register family live, and a register pair the stored
/// plan does not carry (this run holds an empty register set) is deferred, never merged.
#[tokio::test]
async fn registry_pairs_never_change_the_coh_plan() {
    let b = bed("registry-coh-parity").await;
    b.split(100, (1, COH_A, "Acme Widgets Ltd"), (2, PPON_P, "ACME WIDGETS LTD")).await;
    b.split(200, (3, COH_B, "Northgate plc"), (4, PPON_Q, "Northgate Ltd")).await;
    b.split(300, (5, COH_C, "Southgate Ltd"), (6, PPON_S, "Southgate Ltd")).await;
    b.org(7, COH_D, "Southgate Ltd").await;
    b.notice(302, "fts:ocds-1.1").await;
    b.party(302, "D", Some(7), "GB", &[COH_D, PPON_S]).await;
    let first = b.plan().await;
    assert_eq!(
        (first.plan_pairs, first.denied_legal_form, first.conflict_ppon_multi_coh),
        (1, 1, 2),
        "{first:#?}"
    );
    assert!(first.registry_pairs.is_empty());
    let before = coh_view(&first);

    // A planned UKPRN pair.
    b.registry_split(400, (10, UKPRN_A, "Leeds City College"), (11, PPON_R, "Leeds City College")).await;
    // A charity beside the planned company-number pair's own PPON: a triple.
    b.org(12, CHC_X, "Acme Widgets Trust").await;
    b.notice(500, "fts:ocds-1.1").await;
    b.party(500, "H", Some(12), "GB", &[CHC_X, PPON_P]).await;
    // An NHS pair: registry-shape.
    let leeds = "Leeds Teaching Hospitals NHS Trust";
    b.registry_split(600, (13, "GB-NHS-RR8", leeds), (14, PPON_T, leeds)).await;
    // A PPON beside two registers: cross-register.
    b.org(15, MPR_M, "Riverside Housing").await;
    b.org(16, UKPRN_C, "Riverside College").await;
    b.org(17, PPON_U, "Riverside Housing").await;
    b.notice(700, "fts:ocds-1.1").await;
    b.party(700, "M", Some(15), "GB", &[MPR_M, PPON_U]).await;
    b.notice(701, "fts:ocds-1.1").await;
    b.party(701, "U", Some(16), "GB", &[UKPRN_C, PPON_U]).await;
    b.notice(702, "fts:ocds-1.1").await;
    b.party(702, "P", Some(17), "GB", &[PPON_U]).await;

    let dry = b.plan().await;
    assert_eq!(dry.registry_pairs_planned, vec![rk(UKPRN_A, PPON_R)], "{dry:#?}");
    let outcome = |s: &str| {
        let g = &dry.registry_gates[s];
        (g.candidates, g.planned, g.denied_triple, g.denied_registry_shape, g.denied_cross_register)
    };
    assert_eq!(outcome("GB-UKPRN"), (2, 1, 0, 0, 1));
    assert_eq!(outcome("GB-CHC"), (1, 0, 1, 0, 0));
    assert_eq!(outcome("GB-NHS"), (1, 0, 0, 1, 0));
    assert_eq!(outcome("GB-MPR"), (1, 0, 0, 0, 1));
    // Issue 513 unit 1: the NHS listing stands beside the plan, and moves none of it.
    let listed: Vec<(&str, &str)> = dry.registry_nhs_listing.iter().map(|e| (e.literal.as_str(), e.class)).collect();
    assert_eq!(listed, vec![("GB-NHS-RR8", "both_distinct")], "{dry:#?}");
    assert_eq!(coh_view(&dry), before, "the register graph moved a company-number count");

    let unkeyed = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs { registry_pair_key: |_, _, _| None, ..args(20) })
        .await
        .unwrap();
    assert!(unkeyed.registry_pairs_planned.is_empty(), "{unkeyed:#?}");
    assert_eq!(coh_view(&unkeyed), before);

    let wet = b.wet(&dry.pairs).await;
    assert_eq!(wet.merged_pairs, 1, "{wet:#?}");
    assert!(wet.registry_nhs_listing.is_empty(), "issue 513: a wet run lists no NHS pair: {wet:#?}");
    assert_eq!(wet.registry_pairs_planned, vec![rk(UKPRN_A, PPON_R)], "the live register plan: {wet:#?}");
    assert_eq!((wet.registry_deferred_unreviewed, wet.registry_merged_pairs), (1, 0), "{wet:#?}");
    assert_eq!(wet.registry_deferred_pairs, vec![rk(UKPRN_A, PPON_R)]);
    assert!(wet.registry_residual_pairs.is_empty(), "a deferred register pair is never carried as reviewed");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id BETWEEN 10 AND 17").await, 8);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 1);
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_log WHERE json_extract(evidence, '$.coh') IS NULL").await,
        0,
        "every ledger row is a company-number merge"
    );
}

/// U2's consortium agent: Northern Housing Consortium states each member's register
/// number beside its own PPON, one member per notice — so unit 1's per-party
/// `ppon_beside_two_values` reads 0, and only the PPON's partner set across parties
/// sees it. The agent's PPON org is headed like the first member, so that pair would
/// corroborate; no consortium word appears anywhere, and a HIGH verdict changes
/// nothing. The named form (a consortium PPON org beside ONE member) is the consortium
/// veto's.
#[tokio::test]
async fn a_consortium_ppon_beside_two_members_on_different_notices_is_cross_register() {
    let b = bed("registry-consortium").await;
    b.org(1, MPR_M, "Riverside Group Ltd").await;
    b.org(3, MPR_N, "Thirteen Housing Group Ltd").await;
    b.org(9, PPON_P, "Riverside Group Ltd").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "M1", Some(1), "GB", &[MPR_M, PPON_P]).await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "M2", Some(3), "GB", &[MPR_N, PPON_P]).await;
    b.notice(300, "fts:ocds-1.1").await;
    b.party(300, "A", Some(9), "GB", &[PPON_P]).await;
    b.registry_verdict(&rk(MPR_N, PPON_P), vec![3, 9], "merge", "high").await;
    b.registry_split(400, (20, UKPRN_A, "Gateshead College"), (21, PPON_Q, "Northern Housing Consortium Ltd")).await;

    let r = b.plan().await;
    assert_eq!(r.registry_pairs["GB-MPR"].ppon_beside_two_values, 0, "U0's blind spot: one member per party");
    assert_eq!((r.registry_cross_register_ppons, r.registry_cross_register_denied_ppons), (1, 1));
    let mpr = &r.registry_gates["GB-MPR"];
    assert_eq!(
        (mpr.candidates, mpr.denied_cross_register, mpr.admitted_verdict, mpr.planned),
        (2, 2, 0, 0),
        "{mpr:#?}"
    );
    let ukprn = &r.registry_gates["GB-UKPRN"];
    assert_eq!((ukprn.denied_consortium, ukprn.planned), (1, 0), "{ukprn:#?}");
    assert!(r.registry_pairs_planned.is_empty(), "{r:#?}");
}

/// The mirror of cross-register is judgment, not structure: one UKPRN beside two PPONs
/// (a college's own PPON and, say, its trading company's) is a `registry-multi-ppon`
/// conflict, listed for review — and a reviewer's HIGH verdict over exactly the live
/// pair admits it.
#[tokio::test]
async fn a_ukprn_beside_two_ppons_is_a_conflict_a_high_verdict_admits_it() {
    let b = bed("registry-multi-ppon").await;
    b.registry_split(100, (1, UKPRN_A, "Leeds City College"), (2, PPON_P, "Leeds City College")).await;
    b.org(3, PPON_Q, "Leeds City College").await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "R2", Some(1), "GB", &[UKPRN_A, PPON_Q]).await;
    b.notice(201, "fts:ocds-1.1").await;
    b.party(201, "P2", Some(3), "GB", &[PPON_Q]).await;

    let r = b.plan().await;
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.candidates, g.conflicts, g.conflict_registry_multi_ppon, g.planned), (2, 2, 2, 0), "{g:#?}");
    assert!(
        r.registry_denied_listing.iter().all(|l| l.gate == "registry-multi-ppon" && l.registry_partners == 2),
        "{:#?}",
        r.registry_denied_listing
    );

    b.registry_verdict(&rk(UKPRN_A, PPON_P), vec![1, 2], "merge", "high").await;
    let r = b.plan().await;
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.admitted_verdict, g.planned, g.conflicts), (1, 1, 1), "{g:#?}");
    assert_eq!(r.registry_pairs_planned, vec![rk(UKPRN_A, PPON_P)]);
    assert_eq!(r.registry_plan_listing[0].corroborated_by, None, "a verdict stood in for the judgment");
}

// ---- Issue 518 unit 2a: the alias bar reads the owner's mention names, as the planner does.

/// The Caterlink regression (job 2215). The planner corroborates a pair on each org's
/// MENTION names, so it merged `Caterlink` into `Cater Link Limited` on the strength of the
/// company-number org's own `Caterlink Ltd` mention. The alias bar used to read the owner's
/// DESIGNATED names only — `Cater Link Limited` — so the next PPON-first `Caterlink` had
/// nothing to agree with, minted a new PPON org, and the merged pair grew back. Reading the
/// owner's mention names too, it binds; with the read capped to nothing (the old bar) it is
/// refused, so the test sees the difference.
#[tokio::test]
async fn the_alias_reads_the_owners_mention_names_as_the_planner_does() {
    let b = bed("alias-owner-mentions").await;
    b.split(100, (2, COH_A, "Cater Link Limited"), (1, PPON_P, "Caterlink")).await;
    // A company-number mention outside the pair's witnesses, under the other spelling.
    b.notice(102, "fts:ocds-1.1").await;
    b.party_named(102, "C102", Some(2), "GB", &[COH_A], "Caterlink Ltd").await;
    let dry = b.plan().await;
    assert_eq!(dry.pairs, vec![pk(COH_A, PPON_P)], "{dry:#?}");
    assert_eq!(
        dry.plan_listing[0].corroborated_by,
        Some(("Caterlink Ltd".to_owned(), "Caterlink".to_owned())),
        "the planner agreed on mention names: {dry:#?}"
    );
    b.merge_all().await;
    assert_eq!(b.text("SELECT name FROM organizations WHERE id = 2").await, "Cater Link Limited");
    b.fresh(&[300, 301]).await;

    // The old bar: no owner mention name read, only `Cater Link Limited` — refused.
    let (ids, c) = b.resolve_capped(0, &[ppon_first(300, PPON_P, "Caterlink")]).await;
    assert_ne!(ids[0], 2, "the designated names alone do not agree");
    assert_eq!((c.asked, c.refused_names, c.owner_name_reads, c.owner_names_capped), (1, 1, 1, 1), "{c:?}");
    b.conn.execute("DELETE FROM organization_mentions WHERE notice_id = 300", ()).await.unwrap();
    b.conn.execute(&format!("DELETE FROM organizations WHERE id = {}", ids[0]), ()).await.unwrap();

    // The fixed bar: the owner's `Caterlink` (the merged-in PPON org's mention) agrees.
    let (ids, c) = b.resolve(true, &[ppon_first(301, PPON_P, "Caterlink")]).await;
    assert_eq!(ids[0], 2, "the PPON-first spelling the planner merged now binds");
    assert_eq!((c.asked, c.bound, c.refused, c.refused_names), (1, 1, 0, 0), "{c:?}");
    assert_eq!((c.owner_name_reads, c.owner_names_capped), (1, 0), "{c:?}");
}

/// Issue 518 (the U5 review): a from-archive rebuild strips `organization_mentions_org` for
/// all of Phase 1, and without it the owner-name read would scan the whole mention table per
/// owner. The armed alias sees the index absent and reads the designated names only, as before
/// issue 518: the Caterlink mention is refused again, and no owner-name read runs.
#[tokio::test]
async fn the_owner_name_read_is_skipped_while_its_index_is_absent() {
    let b = bed("alias-owner-unindexed").await;
    b.split(100, (2, COH_A, "Cater Link Limited"), (1, PPON_P, "Caterlink")).await;
    b.notice(102, "fts:ocds-1.1").await;
    b.party_named(102, "C102", Some(2), "GB", &[COH_A], "Caterlink Ltd").await;
    let dry = b.plan().await;
    assert_eq!(dry.pairs, vec![pk(COH_A, PPON_P)], "{dry:#?}");
    b.merge_all().await;
    b.fresh(&[300]).await;
    b.conn.execute("DROP INDEX organization_mentions_org", ()).await.unwrap();

    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Caterlink")]).await;
    assert_ne!(ids[0], 2, "the designated names alone do not agree");
    assert_eq!((c.asked, c.refused_names, c.owner_name_reads), (1, 1, 0), "no mention scan without the index: {c:?}");
}

/// The owner-name read is capped and walks newest first: an owner whose ONLY agreeing name
/// is on its OLDEST mention binds when the cap reaches it and is refused when it does not,
/// and the cut is counted. An owner with exactly the cap's mentions is not counted as cut.
#[tokio::test]
async fn the_owner_name_read_is_capped_newest_first_and_says_so() {
    let b = bed("alias-owner-cap").await;
    b.org(2, COH_A, "Zeta Group Ltd").await;
    b.conn
        .execute(
            "INSERT INTO org_merge_log (keep, loser, rule, evidence, job_id, at) VALUES (2, 1, 'e2-altid', ?, 77, 1)",
            (Value::Text(format!("{{\"scheme\":\"GB:altid\",\"coh\":\"{}\",\"ppon\":\"{}\"}}", k(COH_A), k(PPON_P))),),
        )
        .await
        .unwrap();
    // Oldest first, by rowid: the agreeing name, then two that do not agree.
    for (notice, name) in [(100, "Caterlink"), (101, "Zeta Group Ltd"), (102, "Zeta Group Limited")] {
        b.mention(notice, "ORG-Z", 2, "GB", Some(COH_A), name).await;
    }
    b.fresh(&[300, 301]).await;

    let (ids, c) = b.resolve_capped(2, &[ppon_first(300, PPON_P, "Caterlink")]).await;
    assert_ne!(ids[0], 2, "the two newest mentions carry no agreeing name");
    assert_eq!((c.asked, c.refused_names, c.owner_name_reads, c.owner_names_capped), (1, 1, 1, 1), "{c:?}");
    b.conn.execute("DELETE FROM organization_mentions WHERE notice_id = 300", ()).await.unwrap();
    b.conn.execute(&format!("DELETE FROM organizations WHERE id = {}", ids[0]), ()).await.unwrap();

    let (ids, c) = b.resolve_capped(3, &[ppon_first(301, PPON_P, "Caterlink")]).await;
    assert_eq!(ids[0], 2, "a cap of three reaches the oldest mention");
    assert_eq!((c.bound, c.owner_name_reads, c.owner_names_capped), (1, 1, 0), "exactly the cap is no cut: {c:?}");
}

// ---- Issue 469 U5: the register wet path and the register alias.

impl Bed {
    /// Plan, post a register-confirmed HIGH merge verdict ([`CONFIRMED`]) over every
    /// planned register pair as its listing names it, plan again, and merge both stored
    /// sets — the U6 runbook in small. Every planned register pair merges.
    async fn merge_registry_all(&self) -> store::AltIdMergeReport {
        let first = self.plan().await;
        for l in &first.registry_plan_listing {
            let members = l.members.iter().map(|m| m.org_id).collect();
            self.registry_verdict_in(CONFIRMED, &l.key(), members, "merge", "high").await;
        }
        let dry = self.plan().await;
        assert!(dry.registry_plan_listing.iter().all(|l| l.verdict_cohort.as_deref() == Some(CONFIRMED)), "{dry:#?}");
        let r = self.wet_all(&dry.pairs, &dry.registry_pairs_planned).await;
        assert_eq!(r.registry_merged_pairs, dry.registry_pairs_planned.len() as u64, "{r:#?}");
        assert!(r.registry_residual_pairs.is_empty(), "{r:#?}");
        r
    }

    /// The register alias's owner, for asserting: the org carrying `literal` as its own
    /// identifier, if one stands.
    async fn holder(&self, literal: &str) -> i64 {
        self.count(&format!("SELECT COALESCE(MAX(id), 0) FROM organizations WHERE identifier = '{}'", minted(literal)))
            .await
    }

    /// Undo a refused mention's mint, so the next resolve of the same PPON is asked again
    /// rather than riding the minted row's exact triple.
    async fn unmint(&self, notice: i64, org: i64) {
        self.conn.execute(&format!("DELETE FROM organization_mentions WHERE notice_id = {notice}"), ()).await.unwrap();
        self.conn.execute(&format!("DELETE FROM organizations WHERE id = {org}"), ()).await.unwrap();
    }
}

/// The decision's headline test. A college split across its UKPRN org (the HIGHER id)
/// and its PPON org; a register-confirmed HIGH verdict admits the pair; the wet run folds
/// the PPON org into the UKPRN org — mentions, satellites, the PPON as a merged identifier
/// — writes a register ledger row with no `coh`, stamps the verdict, and says what it wrote.
/// The next dry plan reads the pair `already_one`. And the alias binds the next PPON-first
/// mention to the UKPRN org through the exact triple, at 448's bar: another name is refused
/// and mints.
#[tokio::test]
async fn a_ukprn_ppon_pair_merges_into_the_ukprn_org_and_the_alias_binds_the_next_ppon_mention() {
    let b = bed("u5-merge").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE")).await;
    b.satellite(1, "CYM", "Coleg Dinas Leeds").await;
    let key = rk(UKPRN_A, PPON_P);
    assert_eq!(b.plan().await.registry_plan_listing[0].verdict_cohort, None, "planned on corroboration alone");
    b.registry_verdict_in(CONFIRMED, &key, vec![1, 2], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!(dry.registry_gates["GB-UKPRN"].admitted_verdict, 1, "{dry:#?}");
    assert_eq!(dry.registry_plan_listing[0].verdict_cohort.as_deref(), Some(CONFIRMED));

    let r = b.wet_all(&dry.pairs, &dry.registry_pairs_planned).await;
    assert_eq!((r.registry_merged_pairs, r.removed, r.merged_pairs), (1, 1, 0), "{r:#?}");
    assert_eq!((r.registry_expected_pairs, r.registry_unconfirmed, r.registry_contradictory), (1, 0, 0));
    assert!(r.registry_residual_pairs.is_empty() && r.registry_deferred_pairs.is_empty(), "{r:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 1").await, 0, "the PPON org is gone");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 2").await, 1, "the UKPRN org survives");
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 2").await, 2);
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_names WHERE org_id = 2 AND lang = 'CYM'").await, 1);
    assert_eq!(
        b.count(&format!(
            "SELECT COUNT(*) FROM organization_merged_identifiers WHERE identifier = '{}' AND org_id = 2",
            minted(PPON_P)
        ))
        .await,
        1,
        "the PPON is a merged identifier of the UKPRN org"
    );
    // The ledger: flat, no `coh`, every field the alias and an unwind read.
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'e2-altid' AND keep = 2 AND loser = 1 AND job_id = 77").await,
        1
    );
    let evidence = b.text("SELECT evidence FROM org_merge_log WHERE loser = 1").await;
    for (field, value) in [
        ("scheme", "GB:altid".to_owned()),
        ("registry_scheme", "GB-UKPRN".to_owned()),
        ("registry_kind", "national".to_owned()),
        ("registry", minted(UKPRN_A)),
        ("registry_literal", UKPRN_A.to_owned()),
        ("ppon", k(PPON_P)),
        ("ppon_literal", PPON_P.to_owned()),
        ("keep_id", minted(UKPRN_A)),
        ("loser_id", minted(PPON_P)),
        ("name_key", String::new()),
        ("verdict", "admitted".to_owned()),
        ("confirmed", "register".to_owned()),
    ] {
        assert!(evidence.contains(&format!("\"{field}\":\"{value}\"")), "{field} in {evidence}");
    }
    assert!(evidence.contains("\"witnesses\":1") && evidence.contains("\"publishers\":0"), "{evidence}");
    assert_eq!(
        b.count(
            "SELECT COUNT(*) FROM org_merge_log WHERE loser = 1 AND json_extract(evidence, '$.coh') IS NULL \
               AND json_extract(evidence, '$.witness_notices[0]') = 100"
        )
        .await,
        1,
        "parses as JSON, and carries no coh"
    );
    assert_eq!(
        b.count(&format!(
            "SELECT COUNT(*) FROM org_merge_verdicts WHERE scheme = 'GB:altid' AND key = '{key}' AND cohort = '{CONFIRMED}' \
               AND applied_at IS NOT NULL AND job_id = 77 AND applied_action = 'merged 1 row(s) into 2'"
        ))
        .await,
        1
    );
    for (id, op) in [(1, "removed"), (2, "changed")] {
        assert_eq!(
            b.count(&format!(
                "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = {id} AND op = '{op}'"
            ))
            .await,
            1,
            "organization {id} {op}"
        );
    }
    assert_eq!(
        r.registry_merged,
        vec![store::RegistryMerged { key: key.clone(), keep: 2, loser: 1, verdict_cohort: CONFIRMED.into(), evidence }],
        "the report carries what the ledger holds (S4)"
    );

    // The next dry plan: one org, nothing regrown.
    let again = b.plan().await;
    let ukprn = &again.registry_pairs["GB-UKPRN"];
    assert_eq!((ukprn.already_one, ukprn.both_distinct), (1, 0), "{ukprn:#?}");
    assert!(again.registry_regrown.is_empty() && again.registry_pairs_planned.is_empty(), "{again:#?}");

    // The alias: the PPON-first mention binds to the UKPRN org; another company's name
    // under the same PPON is refused and mints.
    b.fresh(&[300, 301]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b
        .resolve(true, &[ppon_first(300, PPON_P, "Leeds City College"), ppon_first(301, PPON_P, "Zenith Holdings Ltd")])
        .await;
    assert_eq!(ids[0], 2, "bound through the exact UKPRN triple");
    assert_ne!(ids[1], 2);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 1, "only the refused mention minted");
    assert_eq!(
        c,
        store::AltIdAliasCounts {
            armed: true,
            ledger_rows: 1,
            owner_name_reads: 1,
            registry: store::AltIdRegistryAliasCounts {
                aliases: 1,
                asked: 2,
                bound: 1,
                bound_unwalled: 1,
                refused: 1,
                refused_names: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        "register counts only, nothing recorded (the merge wrote the PPON)"
    );
}

/// A register ledger row carries no `coh`, so 448's alias read — the SQL a pre-U5 binary
/// runs — sees an empty company number and drops the row; the U5 read names the register.
/// The register target never falls through to the canonical map: with the UKPRN org gone it
/// is refused `no owner`. A keep verdict disarms it only under the REGISTER key.
#[tokio::test]
async fn a_registry_ledger_row_never_reaches_the_coh_alias() {
    let b = bed("u5-no-coh").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE")).await;
    b.merge_registry_all().await;
    let mut rows = b
        .conn
        .query("SELECT json_extract(evidence, '$.coh'), json_extract(evidence, '$.ppon') FROM org_merge_log WHERE rule = 'e2-altid'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!((row.get_value(0).unwrap(), row.get_value(1).unwrap()), (Value::Null, Value::Text(k(PPON_P))));
    drop(rows);
    let mut rows = b.conn.query(store::ALTID_ALIAS_LEDGER_SQL, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let cols: Vec<Value> = (0..5).map(|i| row.get_value(i).unwrap()).collect();
    assert_eq!(
        cols,
        vec![
            Value::Null,
            Value::Text(k(PPON_P)),
            Value::Text("GB-UKPRN".into()),
            Value::Text("national".into()),
            Value::Text(minted(UKPRN_A)),
        ]
    );
    drop(rows);

    b.conn.execute("DELETE FROM organization_mentions WHERE organization_id = 2", ()).await.unwrap();
    b.conn.execute("DELETE FROM organizations WHERE id = 2", ()).await.unwrap();
    b.fresh(&[300, 301, 302]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Leeds City College")]).await;
    assert_eq!((c.aliases, c.registry.aliases, c.asked), (0, 1, 0), "{c:?}");
    assert_eq!((c.registry.asked, c.registry.refused_no_owner), (1, 1), "no fall-through to canon_of: {c:?}");
    b.unmint(300, ids[0]).await;

    // A keep under a company-number-shaped key names no register merge.
    b.registry_verdict(&format!("{}~{}", minted(UKPRN_A), k(PPON_P)), vec![1, 2], "keep", "high").await;
    let (ids, c) = b.resolve(true, &[ppon_first(301, PPON_P, "Leeds City College")]).await;
    assert_eq!((c.kept, c.registry.kept, c.registry.aliases), (0, 0, 1), "{c:?}");
    b.unmint(301, ids[0]).await;
    b.registry_verdict(&rk(UKPRN_A, PPON_P), vec![1, 2], "keep", "high").await;
    let (_, c) = b.resolve(true, &[ppon_first(302, PPON_P, "Leeds City College")]).await;
    assert_eq!((c.registry.kept, c.registry.aliases, c.registry.asked), (1, 0, 0), "{c:?}");
}

/// A PPON the ledger merged into a company number AND into a register is poisoned across
/// kinds: the arm never picks a side, so neither does the fold — asked, refused in the
/// register family (a register target is among its targets), and minted. A keep on the
/// register merge leaves one target, and the PPON aliases to the company number again.
#[tokio::test]
async fn a_ppon_aliased_to_a_company_number_and_a_register_is_poisoned() {
    let b = bed("u5-poison").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.merge_all().await;
    b.org(5, UKPRN_A, "Acme Widgets Ltd").await;
    b.conn
        .execute(
            "INSERT INTO org_merge_log (keep, loser, rule, evidence, job_id, at) VALUES (5, 6, 'e2-altid', ?, 78, 1)",
            (Value::Text(format!(
                "{{\"scheme\":\"GB:altid\",\"registry_scheme\":\"GB-UKPRN\",\"registry_kind\":\"national\",\
                  \"registry\":\"{}\",\"ppon\":\"{}\"}}",
                minted(UKPRN_A),
                k(PPON_P)
            )),),
        )
        .await
        .unwrap();
    b.fresh(&[300, 301]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Acme Widgets Ltd")]).await;
    assert!(ids[0] != 2 && ids[0] != 5, "neither target takes it: {ids:?}");
    assert_eq!((c.ledger_rows, c.poisoned, c.aliases, c.registry.aliases), (2, 1, 0, 0), "{c:?}");
    assert_eq!((c.asked, c.registry.asked, c.registry.refused_poisoned), (0, 1, 1), "{c:?}");
    b.unmint(300, ids[0]).await;

    b.registry_verdict(&rk(UKPRN_A, PPON_P), vec![5, 6], "keep", "high").await;
    let (ids, c) = b.resolve(true, &[ppon_first(301, PPON_P, "Acme Widgets Ltd")]).await;
    assert_eq!(ids[0], 2, "one target left: the company number");
    assert_eq!((c.registry.kept, c.aliases, c.bound, c.poisoned), (1, 1, 1, 0), "{c:?}");
}

/// A register target binds only through a triple exactly one row carries and no
/// `wrong`/`related` verdict names: (a) a second row with the same UKPRN, added after the
/// merge, makes it `shared` and it binds neither; (b) a `wrong` verdict on the survivor's
/// number does too, even with one row; (c) a re-review to `right` lifts it.
#[tokio::test]
async fn a_register_target_on_two_rows_or_withheld_binds_nobody() {
    let b = bed("u5-shared").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE")).await;
    b.merge_registry_all().await;
    b.fresh(&[300, 301, 302]).await;

    b.org_in(7, "GB", &minted(UKPRN_A), "Leeds City College").await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "Leeds City College")]).await;
    assert!(ids[0] != 2 && ids[0] != 7, "two rows carry the triple: {ids:?}");
    assert_eq!((c.registry.asked, c.registry.refused_shared), (1, 1), "{c:?}");
    b.unmint(300, ids[0]).await;
    b.conn.execute("DELETE FROM organizations WHERE id = 7", ()).await.unwrap();

    b.identifier_verdict(2, &minted(UKPRN_A), "wrong").await;
    let (ids, c) = b.resolve(true, &[ppon_first(301, PPON_P, "Leeds City College")]).await;
    assert_ne!(ids[0], 2, "a withheld triple binds nobody");
    assert_eq!((c.registry.asked, c.registry.refused_shared), (1, 1), "{c:?}");
    b.unmint(301, ids[0]).await;

    b.identifier_verdict(2, &minted(UKPRN_A), "right").await;
    let (ids, c) = b.resolve(true, &[ppon_first(302, PPON_P, "Leeds City College")]).await;
    assert_eq!(ids[0], 2);
    assert_eq!((c.registry.asked, c.registry.bound), (1, 1), "{c:?}");
}

/// Parity is held per family: register drift over max(2%, 5) aborts the whole run before
/// any write, even though the company-number plan matches its stored set exactly. At the
/// tolerance both families merge.
#[tokio::test]
async fn registry_drift_aborts_even_when_coh_parity_holds() {
    let b = bed("u5-drift").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.registry_split(200, (3, UKPRN_A, "Leeds City College"), (4, PPON_Q, "Leeds City College")).await;
    b.registry_verdict_in(CONFIRMED, &rk(UKPRN_A, PPON_Q), vec![3, 4], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!((dry.pairs.len(), dry.registry_pairs_planned.len()), (1, 1), "{dry:#?}");
    let fake = |n: usize| -> Vec<String> { (0..n).map(|i| format!("GB-UKPRN:GBUKPRN1000000{i}~FAKE{i:08}")).collect() };
    let before = b.snapshot().await;
    let err = b.db.match_org_altid_pairs(wet_all_args(&dry.pairs, &fake(6))).await.expect_err("register drift aborts");
    let msg = err.to_string();
    assert!(msg.contains("register parity abort"), "{msg}");
    assert!(msg.contains("live register plan has 1 pairs, the stored register plan 6") && msg.contains("7 differ"), "{msg}");
    assert!(msg.contains("nothing was written"), "{msg}");
    assert_eq!(b.snapshot().await, before, "the company-number pair was not merged either");

    let mut expect = fake(5);
    expect.extend(dry.registry_pairs_planned.iter().cloned());
    expect.sort();
    let r = b.wet_all(&dry.pairs, &expect).await;
    assert_eq!((r.merged_pairs, r.registry_merged_pairs, r.registry_expected_not_live), (1, 1, 5), "{r:#?}");
}

/// The cross-family contest (C9; U4's C8): one PPON org is the loser of a company-number
/// pair (its own PPON) and of a register pair (a PPON it holds as a merged identifier).
/// Each family alone would fold it; together they would fold it twice, so neither merges
/// and both keys stay in their residuals. With the register side gone, the company-number
/// pair merges.
#[tokio::test]
async fn a_ppon_org_two_families_would_fold_merges_neither() {
    let b = bed("u5-two-families").await;
    b.org(1, PPON_P, "Acme Widgets Ltd").await;
    b.conn
        .execute(
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'GB', 1, 999, 'e2-altid')",
            (Value::Text(minted(PPON_Q)),),
        )
        .await
        .unwrap();
    b.org(2, COH_A, "Acme Widgets Ltd").await;
    b.org(3, UKPRN_A, "Acme Widgets Ltd").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "C", Some(2), "GB", &[COH_A, PPON_P]).await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "R", Some(3), "GB", &[UKPRN_A, PPON_Q]).await;
    // L's own mention carries no raw identifier, so neither loser-incoherent gate fires.
    b.notice(300, "fts:ocds-1.1").await;
    b.mention(300, "ORG-L", 1, "GB", None, "Acme Widgets Ltd").await;
    b.registry_verdict_in(CONFIRMED, &rk(UKPRN_A, PPON_Q), vec![1, 3], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!(dry.pairs, vec![pk(COH_A, PPON_P)], "{dry:#?}");
    assert_eq!(dry.registry_pairs_planned, vec![rk(UKPRN_A, PPON_Q)], "{dry:#?}");

    let before = b.snapshot().await;
    let r = b.wet_all(&dry.pairs, &dry.registry_pairs_planned).await;
    assert_eq!((r.contradictory, r.registry_contradictory), (1, 1), "{r:#?}");
    assert_eq!((r.merged_pairs, r.registry_merged_pairs), (0, 0), "{r:#?}");
    assert_eq!((r.residual_pairs.clone(), r.registry_residual_pairs.clone()), (dry.pairs.clone(), dry.registry_pairs_planned.clone()));
    assert_eq!(b.snapshot().await, before, "the contested org stays where it is");

    // The control: without the merged identifier the register side has no PPON holder.
    b.conn.execute("DELETE FROM organization_merged_identifiers", ()).await.unwrap();
    let dry = b.plan().await;
    assert!(dry.registry_pairs_planned.is_empty(), "{dry:#?}");
    assert_eq!(dry.registry_pairs["GB-UKPRN"].no_target_ppon, 1, "{dry:#?}");
    let r = b.wet_all(&dry.pairs, &[]).await;
    assert_eq!((r.merged_pairs, r.contradictory), (1, 0), "{r:#?}");
}

/// D1: a register pair planned on corroboration alone carries no verdict, so the wet run
/// HOLDS it — counted `registry_unconfirmed`, carried in the residual, no ledger row. A
/// reviewer's HIGH verdict posted afterwards admits it on the continuation's live re-plan,
/// and the ledger says a verdict, not a register, confirmed it.
#[tokio::test]
async fn a_register_pair_planned_without_a_high_verdict_is_held_until_one_is_posted() {
    let b = bed("u5-unconfirmed").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "Leeds City College")).await;
    let dry = b.plan().await;
    let key = rk(UKPRN_A, PPON_P);
    assert_eq!(dry.registry_pairs_planned, vec![key.clone()]);
    assert_eq!(dry.registry_plan_listing[0].verdict_cohort, None);
    let r = b.wet_all(&dry.pairs, &dry.registry_pairs_planned).await;
    assert_eq!((r.registry_unconfirmed, r.registry_merged_pairs, r.removed), (1, 0, 0), "{r:#?}");
    assert_eq!(r.registry_residual_pairs, vec![key.clone()]);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 0, "nothing merged on corroboration");

    b.registry_verdict(&key, vec![1, 2], "merge", "high").await;
    let r = b.wet_all(&[], &r.registry_residual_pairs).await;
    assert_eq!((r.registry_unconfirmed, r.registry_merged_pairs), (0, 1), "{r:#?}");
    let evidence = b.text("SELECT evidence FROM org_merge_log WHERE loser = 1").await;
    assert!(evidence.contains("\"confirmed\":\"verdict\"") && evidence.contains("\"verdict\":\"admitted\""), "{evidence}");
}

/// D2 and S4: `max_groups` caps EACH family, so a capped run samples the register family
/// whatever the company-number backlog — one of each merges, `registry_merged` carries the
/// one ledger row verbatim — and the continuation held against both residuals merges the
/// rest.
#[tokio::test]
async fn a_capped_wet_run_caps_each_family_on_its_own() {
    let b = bed("u5-capped").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Northgate Ltd")).await;
    b.registry_split(300, (5, UKPRN_A, "Leeds City College"), (6, PPON_R, "Leeds City College")).await;
    b.registry_split(400, (7, UKPRN_B, "Harrogate College"), (8, PPON_S, "Harrogate College")).await;
    for (key, members) in [(rk(UKPRN_A, PPON_R), vec![5, 6]), (rk(UKPRN_B, PPON_S), vec![7, 8])] {
        b.registry_verdict_in(CONFIRMED, &key, members, "merge", "high").await;
    }
    let dry = b.plan().await;
    assert_eq!((dry.pairs.len(), dry.registry_pairs_planned.len()), (2, 2), "{dry:#?}");
    let r = b
        .db
        .match_org_altid_pairs(store::AltIdMergeArgs {
            max_pairs: Some(1),
            ..wet_all_args(&dry.pairs, &dry.registry_pairs_planned)
        })
        .await
        .unwrap();
    assert_eq!((r.merged_pairs, r.registry_merged_pairs, r.removed), (1, 1, 2), "{r:#?}");
    assert_eq!((r.residual_pairs.len(), r.registry_residual_pairs.len()), (1, 1), "{r:#?}");
    assert_eq!(r.registry_residual_pairs, dry.registry_pairs_planned[1..].to_vec(), "the stored order's prefix merged");
    assert_eq!(r.registry_merged.len(), 1);
    let m = &r.registry_merged[0];
    assert_eq!(
        b.text(&format!("SELECT evidence FROM org_merge_log WHERE keep = {} AND loser = {}", m.keep, m.loser)).await,
        m.evidence
    );
    let rest = b.wet_all(&r.residual_pairs, &r.registry_residual_pairs).await;
    assert_eq!((rest.merged_pairs, rest.registry_merged_pairs, rest.registry_deferred_unreviewed), (1, 1, 0), "{rest:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, 4);
}

/// D3: the register org must carry the identity as its OWN identifier. One holding the
/// UKPRN only as a merged identifier is the pair's single owner — U4 planned it — but the
/// alias replays a register merge only through the exact row, so every later PPON-first
/// mention would be refused `no owner`: denied `single-owner`. Re-keyed onto the UKPRN, it
/// plans.
#[tokio::test]
async fn a_register_org_holding_the_number_only_as_a_merged_identifier_never_plans() {
    let b = bed("u5-own-row").await;
    b.org(2, CHC_Y, "Leeds City College").await;
    b.conn
        .execute(
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'GB', 2, 999, 'r2')",
            (Value::Text(minted(UKPRN_A)),),
        )
        .await
        .unwrap();
    b.org(1, PPON_P, "Leeds City College").await;
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "R", Some(2), "GB", &[UKPRN_A, PPON_P]).await;
    b.notice(101, "fts:ocds-1.1").await;
    b.party(101, "P", Some(1), "GB", &[PPON_P]).await;
    let r = b.plan().await;
    let g = &r.registry_gates["GB-UKPRN"];
    assert_eq!((g.candidates, g.denied_single_owner, g.planned), (1, 1, 0), "{g:#?}");
    assert!(r.registry_pairs_planned.is_empty());

    b.conn.execute(&format!("UPDATE organizations SET identifier = '{}' WHERE id = 2", minted(UKPRN_A)), ()).await.unwrap();
    assert_eq!(b.plan().await.registry_pairs_planned, vec![rk(UKPRN_A, PPON_P)], "its own row: planned");
}

/// S5 (C10): after a register merge, a re-minted PPON org makes the PPON side two holders
/// — the survivor (by its merged identifier) and the new org — so the pair reads
/// `multi_target`, never `both_distinct`. `registry_regrown` names it.
#[tokio::test]
async fn a_merged_register_pair_that_splits_again_is_counted_regrown() {
    let b = bed("u5-regrown").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE")).await;
    b.merge_registry_all().await;
    b.org(9, PPON_P, "Leeds City College").await;
    b.notice(500, "fts:ocds-1.1").await;
    b.party(500, "N", Some(9), "GB", &[PPON_P]).await;
    let r = b.plan().await;
    assert_eq!(r.registry_regrown, vec![rk(UKPRN_A, PPON_P)], "{r:#?}");
    assert_eq!(r.registry_pairs["GB-UKPRN"].multi_target, 1, "{r:#?}");
}

/// C9's chain: org 3 is one register pair's keep (its UKPRN) and the other's loser (a PPON
/// it holds as a merged identifier). The second pair sorts first; merging it would delete
/// org 3 before the first pair repoints into it. The arm holds the contested pair and
/// merges the other.
#[tokio::test]
async fn a_keep_that_another_register_pair_would_fold_is_contested() {
    let b = bed("u5-chain").await;
    b.org(3, UKPRN_A, "Leeds College").await;
    b.conn
        .execute(
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'GB', 3, 999, 'e2-altid')",
            (Value::Text(minted(PPON_Q)),),
        )
        .await
        .unwrap();
    b.org(4, PPON_P, "Leeds College").await;
    b.org(5, UKPRN_B, "Leeds College").await;
    // PPON-first parties, each bound to its PPON's holder.
    b.notice(100, "fts:ocds-1.1").await;
    b.party(100, "A", Some(4), "GB", &[PPON_P, UKPRN_A]).await;
    b.notice(200, "fts:ocds-1.1").await;
    b.party(200, "B", Some(3), "GB", &[PPON_Q, UKPRN_B]).await;
    let (first, second) = (rk(UKPRN_A, PPON_P), rk(UKPRN_B, PPON_Q));
    assert!(second < first, "the pair that would delete the keep sorts first");
    b.registry_verdict_in(CONFIRMED, &first, vec![3, 4], "merge", "high").await;
    b.registry_verdict_in(CONFIRMED, &second, vec![3, 5], "merge", "high").await;
    let dry = b.plan().await;
    assert_eq!(dry.registry_pairs_planned, vec![second.clone(), first.clone()], "{dry:#?}");
    let r = b.wet_all(&dry.pairs, &dry.registry_pairs_planned).await;
    assert_eq!((r.registry_merged_pairs, r.registry_contradictory), (1, 1), "{r:#?}");
    assert_eq!(r.registry_residual_pairs, vec![second]);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (3, 5)").await, 2, "the keep stands");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 4").await, 0);
}

/// Identity to identity: the register alias maps the PPON to (kind, identity), never an org
/// id, so after a rebuild — the org layer re-minted under new ids, the merged-identifier
/// table emptied, the ledger kept — it finds the re-minted UKPRN org by its exact triple,
/// and records the PPON there (issue 460's rule, target-agnostic).
#[tokio::test]
async fn the_register_alias_survives_the_register_orgs_renumbering() {
    let b = bed("u5-renumber").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE")).await;
    b.merge_registry_all().await;
    for table in ["organization_mentions", "organization_names", "organizations", "organization_merged_identifiers"] {
        b.conn.execute(&format!("DELETE FROM {table}"), ()).await.unwrap();
    }
    b.org(50, UKPRN_A, "Leeds City College").await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(300, PPON_P, "LEEDS CITY COLLEGE")]).await;
    assert_eq!(ids[0], 50, "found by its triple, not by the id the ledger recorded");
    assert_eq!((c.registry.bound, c.recorded), (1, 1), "{c:?}");
    assert_eq!(
        b.count(&format!(
            "SELECT COUNT(*) FROM organization_merged_identifiers WHERE identifier = '{}' AND org_id = 50 AND rule = 'altid-alias'",
            minted(PPON_P)
        ))
        .await,
        1
    );
}

/// The register alias holds 448's bar, never looser: a plc beside the owner's Ltd (head
/// against head), another company's name, and a consortium name are each refused — counted
/// in the register family — and each mints exactly as it would unarmed.
#[tokio::test]
async fn a_register_alias_bind_holds_448s_bar_and_refused_mentions_mint() {
    let b = bed("u5-bar").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds Trust Ltd"), (1, PPON_P, "Leeds Trust Ltd")).await;
    b.registry_split(200, (4, UKPRN_B, "Harrogate College"), (3, PPON_Q, "Harrogate College")).await;
    b.registry_split(300, (6, CHC_X, "Helping Hands Trust"), (5, PPON_R, "Helping Hands Trust")).await;
    assert_eq!(b.merge_registry_all().await.registry_merged_pairs, 3);
    b.fresh(&[500, 501, 502]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b
        .resolve(
            true,
            &[
                ppon_first(500, PPON_P, "Leeds Trust plc"),
                ppon_first(501, PPON_Q, "Zenith Holdings"),
                ppon_first(502, PPON_R, "Helping Hands Consortium"),
            ],
        )
        .await;
    for (id, owner) in ids.iter().zip([UKPRN_A, UKPRN_B, CHC_X]) {
        assert_ne!(*id, b.holder(owner).await, "{owner}: {ids:?}");
    }
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 3);
    assert_eq!((c.registry.asked, c.registry.bound, c.registry.refused), (3, 0, 3), "{c:?}");
    assert_eq!((c.registry.refused_veto, c.registry.refused_names), (2, 1), "{c:?}");
    assert_eq!(c.asked, 0, "nothing in the company-number family: {c:?}");
}

/// Issue 518's fix holds for the register family too (one bar for both target kinds): the
/// owner's designated name `Leeds City College Ltd` does not agree with `Leeds College`, its
/// mentions do — the PPON org's, folded in by the merge — so the PPON-first `Leeds College`
/// binds; with the owner-name read capped to nothing it is refused.
#[tokio::test]
async fn the_register_alias_reads_the_owners_mention_names_too() {
    let b = bed("u5-owner-mentions").await;
    b.registry_split(100, (2, UKPRN_A, "Leeds City College Ltd"), (1, PPON_P, "Leeds College")).await;
    b.notice(102, "fts:ocds-1.1").await;
    b.party_named(102, "R102", Some(2), "GB", &[UKPRN_A], "Leeds College").await;
    b.merge_registry_all().await;
    b.fresh(&[300, 301]).await;
    let (ids, c) = b.resolve_capped(0, &[ppon_first(300, PPON_P, "Leeds College")]).await;
    assert_ne!(ids[0], 2, "the designated names alone do not agree");
    assert_eq!((c.registry.asked, c.registry.refused_names), (1, 1), "{c:?}");
    b.unmint(300, ids[0]).await;
    let (ids, c) = b.resolve(true, &[ppon_first(301, PPON_P, "Leeds College")]).await;
    assert_eq!(ids[0], 2);
    assert_eq!((c.registry.bound, c.owner_name_reads), (1, 1), "{c:?}");
}

/// The hard rule of U5: on a store with no register pairs, the company-number family's dry
/// AND wet runs are what they were — the register planner runs on the wet run but finds
/// nothing, its parity holds two empty sets, the contest reduces to the company-number
/// family's, and the ledger and edge evidence keep their exact text. Pinned here verbatim.
#[tokio::test]
async fn a_store_with_no_register_pairs_merges_exactly_as_before() {
    let b = bed("u5-coh-only").await;
    b.split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD")).await;
    b.split(200, (3, COH_B, "Northgate Ltd"), (4, PPON_Q, "Zenith Ltd")).await;
    let dry = b.plan().await;
    let wet = b.wet(&dry.pairs).await;
    for r in [&dry, &wet] {
        assert!(r.registry_pairs.is_empty() && r.registry_gates.is_empty() && r.registry_pairs_planned.is_empty(), "{r:#?}");
        assert!(r.registry_regrown.is_empty() && r.registry_merged.is_empty() && r.registry_residual_pairs.is_empty());
        assert_eq!(
            (r.registry_expected_pairs, r.registry_deferred_unreviewed, r.registry_expected_not_live),
            (0, 0, 0)
        );
        assert_eq!(
            (r.registry_contradictory, r.registry_unconfirmed, r.registry_merged_pairs, r.registry_edges_written),
            (0, 0, 0, 0)
        );
    }
    assert_eq!((wet.merged_pairs, wet.removed, wet.edges_written, wet.contradictory), (1, 1, 1, 0), "{wet:#?}");
    assert_eq!(
        b.text("SELECT evidence FROM org_merge_log WHERE loser = 1").await,
        format!(
            "{{\"scheme\":\"GB:altid\",\"coh\":\"{}\",\"ppon\":\"{}\",\"coh_literal\":\"{COH_A}\",\
             \"ppon_literal\":\"{PPON_P}\",\"keep_id\":\"{}\",\"loser_id\":\"{}\",\
             \"name_key\":\"acme widgets §ltd\",\"witnesses\":1,\"witness_notices\":[100],\"verdict\":\"none\"}}",
            k(COH_A),
            k(PPON_P),
            minted(COH_A),
            minted(PPON_P)
        ),
        "the 448 ledger row, byte for byte"
    );
    assert_eq!(
        b.text("SELECT evidence FROM org_candidate_edges WHERE org_a = 3 AND org_b = 4").await,
        format!(
            "{{\"coh\":\"{}\",\"ppon\":\"{}\",\"coh_literal\":\"{COH_B}\",\"ppon_literal\":\"{PPON_Q}\",\
             \"coh_org\":3,\"ppon_org\":4,\"status\":\"uncorroborated-disjoint\",\"witnesses\":1,\
             \"coh_partners\":1,\"ppon_partners\":1,\"first_coh\":1,\"first_ppon\":0}}",
            k(COH_B),
            k(PPON_Q)
        ),
        "the 448 edge row, byte for byte"
    );
}

/// C7: a register pair a gate denied gets an open `e2-altid`/`E2` edge like a
/// company-number one, its evidence naming the register in place of `coh`.
#[tokio::test]
async fn a_denied_register_pair_gets_an_open_e2_edge() {
    let b = bed("u5-edge").await;
    b.registry_split(100, (1, UKPRN_A, "Leeds City College"), (2, PPON_P, "Zenith Holdings")).await;
    let dry = b.plan().await;
    assert_eq!(dry.registry_gates["GB-UKPRN"].uncorroborated_disjoint, 1, "{dry:#?}");
    let r = b.wet_all(&dry.pairs, &dry.registry_pairs_planned).await;
    assert_eq!((r.edges_written, r.registry_edges_written), (1, 1), "{r:#?}");
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_candidate_edges WHERE org_a = 1 AND org_b = 2 AND rule = 'e2-altid' AND tier = 'E2' AND state = 'open' AND job_id = 77").await,
        1
    );
    let evidence = b.text("SELECT evidence FROM org_candidate_edges WHERE org_a = 1 AND org_b = 2").await;
    for field in [
        "\"registry_scheme\":\"GB-UKPRN\"".to_owned(),
        format!("\"registry\":\"{}\"", minted(UKPRN_A)),
        format!("\"ppon\":\"{}\"", k(PPON_P)),
        "\"registry_org\":1".to_owned(),
        "\"ppon_org\":2".to_owned(),
        "\"status\":\"uncorroborated-disjoint\"".to_owned(),
    ] {
        assert!(evidence.contains(&field), "{field} in {evidence}");
    }
    assert!(!evidence.contains("\"coh\""), "{evidence}");
}
