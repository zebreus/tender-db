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
fn gb_key(value: &str) -> Option<(&'static str, String, bool)> {
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    if let Some(serial) = norm.strip_prefix("GBPPON") {
        return (serial.len() == 12).then(|| ("GB:ppon", serial.to_owned(), true));
    }
    let coh = norm.strip_prefix("GBCOH").or_else(|| norm.strip_prefix("GB")).unwrap_or(&norm);
    let digits = !coh.is_empty() && coh.bytes().all(|b| b.is_ascii_digit());
    match coh.len() {
        8 if digits
            || (coh[..2].bytes().all(|b| b.is_ascii_alphabetic())
                && coh[2..].bytes().all(|b| b.is_ascii_digit())) =>
        {
            Some(("GB:coh", coh.to_owned(), true))
        }
        6 | 7 if digits => Some(("GB:coh", format!("{coh:0>8}"), false)),
        _ => None,
    }
}

/// The v2 gate in small: a run of eight nines is a placeholder.
fn condemns(_country: Option<&str>, _kind: &str, value: &str) -> bool {
    value.contains("99999999")
}

/// `crosswalk::mention_key` in small: the gate first (the normaliser's), then
/// the GB arm, under GB only.
fn mention_key(country: Option<&str>, raw: &str) -> Option<(&'static str, String, bool)> {
    if country != Some("GB") || condemns(country, "national", raw) {
        return None;
    }
    gb_key(raw)
}

/// `crosswalk::altid_pair_key` in small: the scheme names the series.
fn pair_key(scheme: &str, value: &str, country: Option<&str>) -> Option<(&'static str, String, bool)> {
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
fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
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

fn consortium(name: &str) -> bool {
    norm(name).split(' ').any(|t| t == "consortium")
}

fn args<'a>(stoplist_cap: usize) -> store::AltIdMergeArgs<'a> {
    store::AltIdMergeArgs {
        pair_key,
        key,
        mention_key,
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
        job_id: Some(77),
        stop: &|| false,
    }
}

/// A wet run held against `expect` (the stored plan's `pairs`).
fn wet_args<'a>(expect: &[String]) -> store::AltIdMergeArgs<'a> {
    store::AltIdMergeArgs { dry_run: false, expect_pairs: Some(expect.to_vec()), ..args(20) }
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

/// A literal's stored org identifier, the way the resolver mints it: the
/// normaliser's uppercase alphanumerics.
fn minted(literal: &str) -> String {
    literal.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

/// A literal's key, for asserting on listings.
fn k(literal: &str) -> String {
    gb_key(literal).expect("fixture literal keys").1
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
    store::AltIdAliasRules { mention_key, consortium, legal_family, name_key, names_agree, trim, norm }
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
        let mut resolver = self
            .db
            .mention_resolver(Some(key), Some(consortium), None, Some(norm), None, hard_scheme, cap)
            .await
            .unwrap();
        if armed {
            self.db.arm_altid_alias(&mut resolver, alias_rules()).await.unwrap();
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
            ..Default::default()
        }
    );
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
/// not carry is inert, and a `related` verdict flags without withholding.
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
    assert_eq!((r.plan_pairs, r.withheld), (1, 0), "a re-review replaces the verdict; related only flags");
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
