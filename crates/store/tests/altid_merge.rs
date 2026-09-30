//! Issue 448 unit 1: `match_org_altid_pairs`, the DRY planner of the `altid`
//! rule — the Companies House ↔ PPON pairing FTS parties publish in
//! `additionalIdentifiers`, which the fold drops.
//!
//! The injected rules are test-local miniatures in the production fn-pointer
//! SHAPE (the r3_merge.rs convention: store cannot depend on ingest). So these
//! tests pin the store's harvest, graph, owner map and gate order; the REAL rule
//! content — `altid_pair_key`, `mention_key`, `altid_name_key`,
//! `gb_legal_family` — is pinned by `ingest::crosswalk`'s own tests, and the
//! production wiring end to end by the supervisor's
//! `an_altid_wet_run_is_refused_until_unit_2`, whose dry step plans a real FTS
//! pair through the real fns.
//!
//! Fixtures are built the way the parser and the fold leave them: a notice
//! under profile `fts:ocds-1.1`, one `ORG-…` section per party, its identifiers
//! as `BT-501-Organization-Company` rows at ordinal 0, 1, … with the
//! publisher's scheme (`GB-COH-03914810` under `GB-COH`), and the fold's one
//! mention per party, bound to the org its FIRST identifier keys to and
//! carrying that identifier as `raw_identifier`.

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

fn name_key(name: &str) -> String {
    norm(name)
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

fn args(stoplist_cap: usize) -> store::AltIdMergeArgs<'static> {
    store::AltIdMergeArgs {
        pair_key,
        key,
        mention_key,
        condemns,
        consortium,
        legal_family,
        name_key,
        names_agree,
        norm,
        stoplist_cap,
        dry_run: true,
        stop: &|| false,
    }
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

/// Unit 1b, the shape dry job 1681 planned on prod: witnesses publish Altrad
/// Babcock's name and PPON beside AMENTUM's company number, company number
/// first. The fold binds those mentions to the company-number org and records
/// the witness name as its satellite, so its designated names "agree" with the
/// PPON org's. Every name either org carries from ANY OTHER notice disagrees,
/// so the pair lists as `witness-only`, never as a merge. A reviewer's HIGH
/// merge verdict still admits it, which is the path a real rename takes.
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

    b.verdict(COH_A, PPON_P, vec![1, 2], "merge", "high").await;
    let r = b.plan().await;
    assert_eq!((r.admitted_verdict, r.plan_pairs, r.denied_witness_only), (1, 1, 0));
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

/// Unit 1 is a planner: nothing it does writes, and a wet run is refused before
/// anything is read.
#[tokio::test]
async fn the_dry_run_writes_nothing() {
    let b = bed("dry").await;
    b.split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "ACME LTD")).await;
    b.split(200, (3, COH_B, "Northgate plc"), (4, PPON_Q, "Northgate Ltd")).await;
    let tables = ["organizations", "organization_mentions", "org_merge_log", "org_candidate_edges", "changes"];
    let mut before = Vec::new();
    for t in tables {
        before.push(b.count(&format!("SELECT COUNT(*) FROM {t}")).await);
    }
    let r = b.plan().await;
    assert_eq!(r.plan_pairs, 1);
    let mut wet = args(20);
    wet.dry_run = false;
    let err = b.db.match_org_altid_pairs(wet).await.expect_err("no wet path yet");
    assert!(err.to_string().contains("448 unit 2"), "{err}");
    for (t, n) in tables.iter().zip(before) {
        assert_eq!(b.count(&format!("SELECT COUNT(*) FROM {t}")).await, n, "{t}");
    }
}

/// The harvest reads one range of `notices_profile` per FTS profile and one
/// primary-key range of `notice_ids` per notice — the party sections only,
/// never the lot, result or contract rows. Asserted on the plans, not a clock.
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
}
