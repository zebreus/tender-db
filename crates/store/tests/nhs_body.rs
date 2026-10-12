//! Issue 513 U2: `plan_nhs_bodies`, the DRY census of the `nhs-body` rule (decision
//! "unit 2, second", design C's U2), and the verdict schema it reads — a one-member
//! `GB:nhs-body` verdict and its `reviewed_holders`.
//!
//! The injected rules are test-local miniatures in the production fn-pointer SHAPE (the
//! altid_merge.rs convention: store cannot depend on ingest). A nine-row ODS table stands in
//! for `ingest::nhs_ods`, and its "check" admits a name carrying the code's word and names a
//! rival whose word it carries. So these tests pin the store's harvest, holder sets, flags,
//! denials and verdict admission; the REAL table and check are pinned by `nhs_ods`'s own
//! tests, and the production wiring by the supervisor's nhs-body tests.
//!
//! Fixtures are built the way the parser and the fold leave them (altid_merge.rs): a notice
//! under `fts:ocds-1.1`, one `ORG-…` section per party, its identifiers as
//! `BT-501-Organization-Company` rows at ordinal 0, 1, … (`GB-NHS-RTH` under `GB-NHS`), and
//! the fold's one mention per party, carrying the FIRST identifier as `raw_identifier`.

use store::turso::{Connection, Value};
use store::{NhsBodyArgs, NhsBodyCaps, NhsBodyCode, NhsBodyReport, NhsNameCheck, NhsOdsCode};

// ---- The miniature rules.

/// (code, ODS name, role, allowlisted, successors, legal end, the word the check reads).
type Row =
    (&'static str, &'static str, &'static str, bool, &'static [&'static str], Option<&'static str>, &'static str);

const TABLE: [Row; 9] = [
    ("00N", "NHS SUB-ICB LOCATION 00N", "RO98", false, &[], None, "sub-icb only"),
    ("QKK", "NHS SOUTH EAST LONDON INTEGRATED CARE BOARD", "RO261", true, &[], None, "south east london"),
    ("QMF", "NHS NORTH EAST LONDON INTEGRATED CARE BOARD", "RO261", true, &[], None, "north east london"),
    ("QNQ", "NHS FRIMLEY INTEGRATED CARE BOARD", "RO261", true, &["QRL"], Some("2026-03-31"), "frimley"),
    ("QRL", "NHS HAMPSHIRE AND ISLE OF WIGHT INTEGRATED CARE BOARD", "RO261", true, &[], None, "hampshire"),
    ("R0B", "SOUTH TYNESIDE AND SUNDERLAND NHS FOUNDATION TRUST", "RO197", true, &[], None, "sunderland"),
    ("REN", "THE CLATTERBRIDGE CANCER CENTRE NHS FOUNDATION TRUST", "RO197", true, &[], None, "clatterbridge"),
    ("RET", "THE WALTON CENTRE NHS FOUNDATION TRUST", "RO197", true, &[], None, "walton"),
    ("RTH", "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST", "RO197", true, &[], None, "oxford"),
];

fn row(code: &str) -> Option<&'static Row> {
    TABLE.iter().find(|r| r.0 == code)
}

/// `nhs_ods::nhs_tail`, verbatim.
fn tail(value: &str) -> Option<&str> {
    let tail = value.strip_prefix("GBNHS")?;
    let normalised = value.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    (tail.len() == 3 && normalised).then_some(tail)
}

/// `nhs_ods::survivor_code` in small: the tail itself when allowlisted, else its one
/// allowlisted O↔0 spelling.
fn survivor_code(tail: &str) -> Option<(&'static str, bool)> {
    let allowed = |code: &str| row(code).filter(|r| r.3).map(|r| r.0);
    if let Some(code) = allowed(tail) {
        return Some((code, false));
    }
    let bytes = tail.as_bytes();
    let swaps: Vec<usize> = (0..bytes.len()).filter(|&i| matches!(bytes[i], b'O' | b'0')).collect();
    let mut hit = None;
    for mask in 1..(1u32 << swaps.len()) {
        let mut v = bytes.to_vec();
        for (k, &i) in swaps.iter().enumerate() {
            if mask >> k & 1 == 1 {
                v[i] = if v[i] == b'O' { b'0' } else { b'O' };
            }
        }
        if let Some(code) = allowed(std::str::from_utf8(&v).unwrap()) {
            if hit.is_some_and(|h| h != code) {
                return None;
            }
            hit = Some(code);
        }
    }
    hit.map(|code| (code, true))
}

fn ods(code: &str) -> Option<NhsOdsCode> {
    row(code).map(|r| NhsOdsCode {
        code: r.0,
        name: r.1,
        role: r.2,
        active: true,
        allow: r.3,
        predecessors: Vec::new(),
        successors: r.4.to_vec(),
        legal_end: r.5,
    })
}

/// `nhs_ods::check` in small: the name carries the code's word and no rival's (admitted),
/// both (a tie, the first rival named), only a rival's (`better`), or neither (`zero`).
fn check(code: &str, name: &str) -> NhsNameCheck {
    let Some(own) = row(code) else { return NhsNameCheck { verdict: "unknown", rival: None } };
    let lower = name.to_lowercase();
    let rival = TABLE.iter().find(|r| r.0 != code && lower.contains(r.6)).map(|r| r.0);
    let verdict = match (lower.contains(own.6), rival) {
        (true, None) => "admitted",
        (true, Some(_)) => "tie",
        (false, Some(_)) => "better",
        (false, None) => "zero",
    };
    NhsNameCheck { verdict, rival }
}

/// `crosswalk::altid_pair_key` in small: a GB `GB-PPON` row with a 12-character serial.
fn pair_key(scheme: &str, value: &str, country: Option<&str>) -> Option<(&'static str, String, bool, bool)> {
    let serial = minted(value).strip_prefix("GBPPON")?.to_owned();
    (scheme == "GB-PPON" && country == Some("GB") && serial.len() == 12).then_some(("GB:ppon", serial, true, false))
}

/// `crosswalk::canonical_key_flat` in small: only a PPON keys.
fn key(_country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool, bool)> {
    let serial = value.strip_prefix("GBPPON")?;
    (kind == "national" && serial.len() == 12).then(|| ("GB:ppon", serial.to_owned(), true, false))
}

/// `project::normalise_identifier` in small: four characters up with a digit, so a
/// letter-only NHS code mints nothing — and production's two filler rules, so a code whose
/// only digits are zeros (`GB-NHS-R0B`, `0AR`) mints nothing either, as on prod.
fn register_identity(raw: &str, country: Option<&str>) -> Option<store::Identifier> {
    let value = minted(raw);
    if value.len() < 4 || !value.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    // All-zero and single-character fillers, verbatim.
    if value.chars().filter(|c| c.is_ascii_digit()).all(|c| c == '0') {
        return None;
    }
    if value.chars().skip(1).all(|c| c == value.as_bytes()[0] as char) {
        return None;
    }
    Some(store::Identifier { country: country.map(str::to_owned), kind: "national".into(), value })
}

fn consortium(name: &str) -> bool {
    name.to_lowercase().contains("consortium")
}

fn norm(name: &str) -> String {
    name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" ")
}

fn minted(literal: &str) -> String {
    literal.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

fn args<'a>(caps: NhsBodyCaps, stoplist_cap: usize) -> NhsBodyArgs<'a> {
    NhsBodyArgs {
        tail,
        survivor_code,
        ods,
        check,
        ods_version: "ods-test",
        pair_key,
        key,
        register_identity,
        consortium,
        norm,
        stoplist_cap,
        caps,
        stop: &|| false,
    }
}

const PPON: &str = "GB-PPON-PHDQ-2359-NZMP";
const PPON_KEY: &str = "PHDQ2359NZMP";

// ---- Instants (UTC midnights).
const Y2024_03_01: i64 = 1_709_251_200;
const Y2025_02_01: i64 = 1_738_368_000;
const Y2025_06_01: i64 = 1_748_736_000;
const Y2026_05_01: i64 = 1_777_593_600;

struct Bed {
    db: store::Db,
    conn: Connection,
    path: String,
}

async fn bed(name: &str) -> Bed {
    let path = format!("/tmp/tender-db-nhs-body-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
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
    /// A provisional GB org with no identifier: what a letter-only code's party binds by name.
    async fn provisional(&self, id: i64, name: &str) {
        self.org_row(id, None, name, 1).await;
    }

    /// A GB org carrying `identifier` (minted, national).
    async fn org(&self, id: i64, identifier: &str, name: &str) {
        self.org_row(id, Some(identifier), name, 0).await;
    }

    async fn org_row(&self, id: i64, identifier: Option<&str>, name: &str, provisional: i64) {
        self.conn
            .execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, 'GB', ?, ?, ?, ?, ?, 0)",
                (
                    Value::Integer(id),
                    identifier.map_or(Value::Null, |_| Value::Text("national".into())),
                    identifier.map_or(Value::Null, |i| Value::Text(i.into())),
                    Value::Text(name.into()),
                    Value::Text(store::org_name_norm(name)),
                    Value::Integer(provisional),
                ),
            )
            .await
            .unwrap();
    }

    async fn notice(&self, id: i64, published_at: Option<i64>) {
        self.conn
            .execute(
                "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at, parse_state, published_at)
                 VALUES (?, 'fts', ?, 'h', 'fts:ocds-1.1', 1, 'm', 0, 'parsed', ?)",
                (Value::Integer(id), Value::Text(format!("pub-{id}")), published_at.map_or(Value::Null, Value::Integer)),
            )
            .await
            .unwrap();
    }

    /// One FTS party on `notice`: its section, its BT-501 rows in order, and — when
    /// `bound_to` is given — the fold's mention under `country`, carrying the FIRST
    /// identifier and publishing `name`.
    async fn party(
        &self,
        notice: i64,
        party: &str,
        bound_to: Option<i64>,
        country: Option<&str>,
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

    async fn mention(
        &self,
        notice: i64,
        section: &str,
        org: i64,
        country: Option<&str>,
        raw: Option<&str>,
        name: &str,
    ) {
        self.conn
            .execute(
                "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
                 VALUES (?, ?, ?, ?, ?, ?, NULL)",
                (
                    Value::Integer(notice),
                    Value::Text(section.into()),
                    Value::Integer(org),
                    Value::Text(name.into()),
                    country.map_or(Value::Null, |c| Value::Text(c.into())),
                    raw.map_or(Value::Null, |r| Value::Text(r.into())),
                ),
            )
            .await
            .unwrap();
    }

    /// The common shape: a GB party on a fresh notice whose only identifier is `literal`,
    /// bound by name to `org` (its head name published).
    async fn coded(&self, notice: i64, org: i64, literal: &str) {
        self.coded_at(notice, org, literal, None).await;
    }

    async fn coded_at(&self, notice: i64, org: i64, literal: &str, published_at: Option<i64>) {
        let name = self.head(org).await;
        self.notice(notice, published_at).await;
        self.party(notice, &format!("N{notice}"), Some(org), Some("GB"), &[literal], &name).await;
    }

    async fn head(&self, org: i64) -> String {
        let mut rows =
            self.conn.query("SELECT name FROM organizations WHERE id = ?", (Value::Integer(org),)).await.unwrap();
        match rows.next().await.unwrap().map(|r| r.get_value(0).unwrap()) {
            Some(Value::Text(name)) => name,
            _ => "x".to_owned(),
        }
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut rows = self.conn.query(sql, ()).await.unwrap();
        let Value::Integer(n) = rows.next().await.unwrap().unwrap().get_value(0).unwrap() else { panic!("count") };
        n
    }

    async fn plan(&self) -> NhsBodyReport {
        self.plan_with(NhsBodyCaps::PRODUCTION, 20).await
    }

    async fn plan_with(&self, caps: NhsBodyCaps, stoplist_cap: usize) -> NhsBodyReport {
        let r = self.db.plan_nhs_bodies(args(caps, stoplist_cap)).await.expect("census");
        assert!(!r.stopped, "{r:#?}");
        // Every listed holder entry is eligible or denied, and every denial is counted.
        let listed: Vec<_> = r.codes.iter().flat_map(|c| &c.holders).collect();
        assert_eq!(r.holders_listed, listed.len() as u64, "{r:#?}");
        assert_eq!(r.eligible_holders, listed.iter().filter(|h| h.denied.is_empty()).count() as u64, "{r:#?}");
        assert_eq!(r.denied.values().sum::<u64>(), listed.iter().map(|h| h.denied.len() as u64).sum::<u64>(), "{r:#?}");
        assert_eq!(r.admitted_verdict + r.verdict_stale + r.verdict_keep, r.verdict_status.values().sum::<u64>());
        r
    }

    /// A `GB:nhs-body` verdict on `code`, cohort `513-u2-test`.
    async fn verdict(&self, code: &str, members: Vec<i64>, holders: Option<Vec<i64>>, action: &str, confidence: &str) {
        self.db
            .record_merge_verdicts("513-u2-test", &[nhs_verdict(code, members, holders, action, confidence)], 0)
            .await
            .unwrap();
    }
}

fn nhs_verdict(
    code: &str,
    members: Vec<i64>,
    holders: Option<Vec<i64>>,
    action: &str,
    confidence: &str,
) -> store::MergeVerdict {
    store::MergeVerdict {
        country: "GB".into(),
        scheme: store::NHS_BODY_SCHEME.into(),
        key: code.into(),
        members,
        action: action.into(),
        rationale: "fixture: ODS confirms".into(),
        confidence: confidence.into(),
        reviewed_holders: holders,
    }
}

fn code<'r>(r: &'r NhsBodyReport, code: &str) -> &'r NhsBodyCode {
    r.codes.iter().find(|c| c.code == code).unwrap_or_else(|| panic!("{code} not planned: {r:#?}"))
}

fn holder<'c>(c: &'c NhsBodyCode, org: i64) -> &'c store::NhsBodyHolder {
    c.holders.iter().find(|h| h.org_id == org).unwrap_or_else(|| panic!("{org} not a listed holder: {c:#?}"))
}

/// Each listed holder and how it holds the code.
fn vias(c: &NhsBodyCode) -> Vec<(i64, &'static str)> {
    c.holders.iter().map(|h| (h.org_id, h.via)).collect()
}

/// The census's point (D6's "every holder uncapped"): unit 1's listing named at most five
/// register holders per pair, so 35 holders in 11 codes were never read. A code with seven
/// holders — six spelling clusters its coded parties bound by name, and the PPON org a
/// PPON-first party publishes beside the code — lists all seven, each with what a reviewer
/// reads: its mentions' raw identifiers and spellings, its ODS check.
#[tokio::test]
async fn a_body_plan_lists_every_holder_uncapped() {
    let b = bed("uncapped").await;
    let spellings = [
        "Oxford University Hospitals NHS Foundation Trust",
        "Oxford University Hospitals NHS FT",
        "Oxford University Hospitals",
        "OUH Oxford",
        "Oxford Univ Hospitals NHS Trust",
        "John Radcliffe Hospital, Oxford",
    ];
    for (i, name) in spellings.iter().enumerate() {
        let org = i as i64 + 1;
        b.provisional(org, name).await;
        b.coded(100 + org, org, "GB-NHS-RTH").await;
    }
    b.org(7, &minted(PPON), "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST").await;
    b.notice(107, None).await;
    b.party(107, "P", Some(7), Some("GB"), &[PPON, "GB-NHS-RTH"], "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST")
        .await;
    // An uncoded mention of the first cluster (the body's text-era history).
    b.notice(108, None).await;
    b.mention(108, "ORG-U", 1, Some("GB"), None, "Oxford University Hospitals NHS Foundation Trust.").await;

    let r = b.plan().await;
    // Notice 108 is FTS too, but carries no party identifier: walked, no party.
    assert_eq!((r.fts_notices, r.party_sections, r.unfolded_sections), (8, 7, 0), "{r:#?}");
    assert_eq!((r.coded_parties, r.coded_parties_allowlisted, r.ppon_pairs), (6, 6, 1));
    assert_eq!((r.codes_seen, r.holders, r.holders_listed, r.holders_truncated), (1, 7, 7, false));
    let rth = code(&r, "RTH");
    assert!(rth.holder_ids.len() > store::REGISTRY_NHS_REGISTER_HOLDERS, "past unit 1's cap");
    assert_eq!(rth.holder_ids, (1..=7).collect::<Vec<i64>>());
    assert_eq!(rth.holders.iter().map(|h| h.org_id).collect::<Vec<_>>(), rth.holder_ids);
    assert_eq!(rth.eligible, rth.holder_ids, "nothing here is structurally denied");
    assert_eq!(rth.ppons, vec![PPON_KEY.to_owned()]);
    assert_eq!(rth.ods.name, "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST");
    assert_eq!((rth.coded_parties, rth.tails.len(), rth.tails[0].folded), (6, 1, false));
    for h in &rth.holders {
        assert_eq!(h.check.verdict, "admitted", "{h:#?}");
        assert_eq!(h.holder_of, vec!["RTH"]);
    }
    let (first, ppon_org) = (holder(rth, 1), holder(rth, 7));
    assert_eq!(
        (first.via, first.coded_parties, first.provisional, first.identifier.as_deref()),
        ("party", 1, true, None)
    );
    assert_eq!((first.mentions, first.mentions_read, first.mentions_capped), (2, 2, false));
    let raws: Vec<(Option<&str>, u64)> = first.raw_identifiers.iter().map(|x| (x.raw.as_deref(), x.n)).collect();
    assert_eq!(raws, vec![(None, 1), (Some("GB-NHS-RTH"), 1)], "{first:#?}");
    // Two spellings, one 234 key: the trailing full stop is not a different name.
    assert_eq!(first.name_norms.len(), 1, "{first:#?}");
    assert_eq!(
        (first.name_norms[0].value.as_str(), first.name_norms[0].n),
        ("oxford university hospitals nhs foundation trust", 2)
    );
    assert_eq!(first.name_norm, "oxford university hospitals nhs foundation trust");
    assert_eq!(
        (ppon_org.via, ppon_org.ppons.clone(), ppon_org.minted_identity),
        ("ppon", vec![PPON_KEY.to_owned()], false)
    );
    // The PPON org's 234 key is the first cluster's: a standing identifier-NULL row
    // inside the body, so not owned elsewhere (D4).
    assert!(ppon_org.key_owners_elsewhere.is_empty(), "{ppon_org:#?}");
    assert_eq!(r.key_owned_elsewhere, 0);
    assert!(r.off_allowlist.is_empty());
    // Nothing is written: the census is read-only end to end.
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, 7);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 0);
}

/// D3's allowlist decides who may anchor a body. `ICB` (two ICBs typed it) and the sub-ICB
/// `00N` (an RO98 row) anchor nothing: they are listed off the allowlist — parties,
/// holders, the parties' names, and what ODS says of the tail itself — and planned never.
#[tokio::test]
async fn a_tail_off_the_allowlist_is_listed_not_planned() {
    let b = bed("off-allowlist").await;
    b.provisional(1, "NHS Cheshire and Merseyside ICB").await;
    b.coded(101, 1, "GB-NHS-ICB").await;
    b.coded(102, 1, "GB-NHS-ICB").await;
    b.provisional(2, "NHS Lancashire and South Cumbria ICB").await;
    b.coded(103, 2, "GB-NHS-ICB").await;
    b.provisional(3, "NHS South Tyneside Place").await;
    b.coded(104, 3, "GB-NHS-00N").await;

    let r = b.plan().await;
    assert!(r.codes.is_empty(), "{r:#?}");
    assert_eq!((r.coded_parties, r.coded_parties_allowlisted, r.holders), (4, 0, 0));
    assert_eq!(r.off_allowlist_seen, 2);
    let icb = &r.off_allowlist[0];
    assert_eq!((icb.tail.as_str(), icb.value.as_str(), icb.parties, icb.holders), ("ICB", "GBNHSICB", 3, 2));
    assert_eq!(
        icb.top_names.iter().map(|n| (n.value.as_str(), n.n)).collect::<Vec<_>>(),
        vec![("NHS Cheshire and Merseyside ICB", 2), ("NHS Lancashire and South Cumbria ICB", 1)]
    );
    assert_eq!(icb.ods, None, "ODS knows no ICB");
    let sub_icb = &r.off_allowlist[1];
    assert_eq!((sub_icb.tail.as_str(), sub_icb.parties), ("00N", 1));
    assert_eq!(sub_icb.ods.as_ref().map(|o| (o.role, o.allow)), Some(("RO98", false)));

    // The cut is honest: the most-published tail stays, the flag says the rest went.
    let cut = b.plan_with(NhsBodyCaps { off_allowlist: 1, ..NhsBodyCaps::PRODUCTION }, 20).await;
    assert_eq!((cut.off_allowlist.len(), cut.off_allowlist_seen, cut.off_allowlist_truncated), (1, 2, true));
    assert_eq!(cut.off_allowlist[0].tail, "ICB");
}

/// `GB-NHS-ROB` is R0B typed with a letter O. Its only allowlisted O↔0 spelling is R0B, so
/// the tail folds into R0B's body and is recorded as folded.
#[tokio::test]
async fn an_o0_tail_folds_into_its_code() {
    let b = bed("o0").await;
    b.provisional(1, "South Tyneside and Sunderland NHS Foundation Trust").await;
    b.coded(101, 1, "GB-NHS-R0B").await;
    b.provisional(2, "South Tyneside & Sunderland NHS FT").await;
    b.coded(102, 2, "GB-NHS-ROB").await;

    let r = b.plan().await;
    assert!(r.off_allowlist.is_empty(), "{r:#?}");
    assert_eq!((r.coded_parties_allowlisted, r.folded_parties), (2, 1));
    let r0b = code(&r, "R0B");
    let tails: Vec<(&str, u64, bool)> = r0b.tails.iter().map(|t| (t.tail.as_str(), t.parties, t.folded)).collect();
    assert_eq!(tails, vec![("R0B", 1, false), ("ROB", 1, true)]);
    assert_eq!(r0b.holder_ids, vec![1, 2]);
    // A folded spelling is the code's own, not "another code".
    assert!(holder(r0b, 2).other_codes.is_empty(), "{r0b:#?}");
}

/// 362's members-exact rule at code grain: a verdict reviewed over the holders {1, 2} is
/// admitted while they are the live holders, and a newcomer — a third spelling a new
/// coded notice bound — makes it stale, naming the newcomer. Re-posting the same (country,
/// scheme, key, cohort) over the new holder set — U3's step after a newcomer — REPLACES
/// `reviewed_holders`, and the verdict is admitted again.
#[tokio::test]
async fn a_newcomer_holder_makes_the_body_verdict_stale() {
    let b = bed("newcomer").await;
    b.provisional(1, "Oxford University Hospitals NHS Foundation Trust").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.provisional(2, "Oxford University Hospitals NHS FT").await;
    b.coded(102, 2, "GB-NHS-RTH").await;
    b.verdict("RTH", vec![1, 2], Some(vec![1, 2]), "merge", "high").await;

    let r = b.plan().await;
    let v = code(&r, "RTH").verdict.clone().expect("the verdict is read");
    assert_eq!((v.status, v.cohort.as_str(), v.members.clone()), ("admitted", "513-u2-test", vec![1, 2]));
    assert_eq!((r.admitted_verdict, r.verdict_stale), (1, 0));

    b.provisional(3, "Oxford Uni Hospitals").await;
    b.coded(103, 3, "GB-NHS-RTH").await;
    let r = b.plan().await;
    let v = code(&r, "RTH").verdict.clone().unwrap();
    assert_eq!((v.status, v.newcomers.clone(), v.departed.clone()), ("stale", vec![3], vec![]));
    assert_eq!((r.admitted_verdict, r.verdict_stale), (0, 1));
    assert_eq!(r.verdict_status.get("stale"), Some(&1));

    b.verdict("RTH", vec![1, 2], Some(vec![1, 2, 3]), "merge", "high").await;
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_verdicts WHERE key = 'RTH' AND reviewed_holders = '[1,2,3]'").await,
        1,
        "the re-post replaced the stored holder set"
    );
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_verdicts WHERE key = 'RTH'").await, 1, "one row, upserted");
    let r = b.plan().await;
    let v = code(&r, "RTH").verdict.clone().unwrap();
    assert_eq!((v.status, v.reviewed_holders.clone(), v.newcomers.clone()), ("admitted", Some(vec![1, 2, 3]), vec![]));
    assert_eq!((r.admitted_verdict, r.verdict_stale), (1, 0));

    // A medium verdict, however exact, is never admissible.
    b.verdict("RTH", vec![1, 2, 3], Some(vec![1, 2, 3]), "merge", "medium").await;
    let r = b.plan().await;
    assert_eq!(code(&r, "RTH").verdict.as_ref().unwrap().status, "not-high");
}

/// The other half of members-exact: a reviewed holder that is no longer a live holder (a
/// cluster merged away, a stray rehomed) voids the review too. `reviewed_holders` must
/// EQUAL the live set, so a strict superset is `stale`, naming what departed.
#[tokio::test]
async fn a_departed_reviewed_holder_makes_the_body_verdict_stale() {
    let b = bed("departed").await;
    b.provisional(1, "Oxford University Hospitals NHS Foundation Trust").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.provisional(2, "Oxford University Hospitals NHS FT").await;
    b.coded(102, 2, "GB-NHS-RTH").await;
    b.verdict("RTH", vec![1, 2], Some(vec![1, 2, 99]), "merge", "high").await;

    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 2]);
    let v = rth.verdict.clone().expect("the verdict is read");
    assert_eq!((v.status, v.newcomers.clone(), v.departed.clone()), ("stale", vec![], vec![99]), "{v:#?}");
    assert_eq!((r.admitted_verdict, r.verdict_stale), (0, 1));
}

/// A body already one org still records its ODS code (D1): `GB:nhs-body` alone accepts a
/// one-member verdict, which the census admits members-exact. Every other scheme still
/// needs two members, and the body's own shape rules hold.
#[tokio::test]
async fn a_one_member_nhs_body_verdict_is_accepted_and_other_schemes_still_need_two() {
    let b = bed("one-member").await;
    b.provisional(1, "Oxford University Hospitals NHS Foundation Trust").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.verdict("RTH", vec![1], Some(vec![1]), "merge", "high").await;
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_verdicts WHERE scheme = 'GB:nhs-body' AND key = 'RTH' AND members = '[1]' AND reviewed_holders = '[1]'").await,
        1
    );
    let r = b.plan().await;
    assert_eq!(code(&r, "RTH").verdict.as_ref().map(|v| v.status), Some("admitted"));

    let refused = |v: store::MergeVerdict| {
        let db = &b.db;
        async move { db.record_merge_verdicts("513-u2-test", &[v], 0).await.expect_err("refused").to_string() }
    };
    let mut altid = nhs_verdict("RTH", vec![1], None, "merge", "high");
    altid.scheme = "GB:altid".into();
    altid.key = "03914810~PHDQ2359NZMP".into();
    assert!(refused(altid).await.contains("two or more"));
    let mut coh = nhs_verdict("RTH", vec![1], None, "merge", "high");
    coh.scheme = "GB:coh".into();
    assert!(refused(coh).await.contains("two or more"));
    assert!(refused(nhs_verdict("RTH", vec![1], None, "merge", "high")).await.contains("needs reviewed_holders"));
    assert!(
        refused(nhs_verdict("RTH", vec![1, 2], Some(vec![1]), "merge", "high"))
            .await
            .contains("containing every member")
    );
    assert!(refused(nhs_verdict("RTH", vec![1], Some(vec![2, 1]), "merge", "high")).await.contains("ascending"));
    assert!(
        refused(nhs_verdict("GBNHSRTH", vec![1], Some(vec![1]), "merge", "high")).await.contains("three-character")
    );
    let mut fr = nhs_verdict("RTH", vec![1], Some(vec![1]), "merge", "high");
    fr.country = "FR".into();
    assert!(refused(fr).await.contains("keyed (GB, GB:nhs-body"));
    // A keep needs no holder set: it holds the body whatever its holders become.
    b.verdict("RTH", vec![1], None, "keep", "high").await;
    let r = b.plan().await;
    assert_eq!(code(&r, "RTH").verdict.as_ref().map(|v| v.status), Some("keep"));
    assert_eq!((r.verdict_keep, r.admitted_verdict), (1, 0));
}

/// D6: never members — a minted-identity holder (519's identity orgs), a generic or a
/// consortium name. Each is denied structurally and named, a verdict naming one is not
/// admitted, and a PPON holder is no minted identity. The "under review" flag counts every
/// review table but the body's own verdict.
#[tokio::test]
async fn a_member_holding_a_minted_identity_is_denied() {
    let b = bed("minted").await;
    b.provisional(1, "Oxford University Hospitals NHS FT").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    // A typo code's identity org (`GB-NHS-RF1` mints): a coded party bound to it.
    b.org(2, "GBNHSRF1", "Oxford Health RF1").await;
    b.coded(102, 2, "GB-NHS-RTH").await;
    b.provisional(3, "Oxford NHS Consortium").await;
    b.coded(103, 3, "GB-NHS-RTH").await;
    b.provisional(4, "Oxford Hospital").await;
    b.coded(104, 4, "GB-NHS-RTH").await;
    b.org(5, &minted(PPON), "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST").await;
    b.notice(105, None).await;
    b.party(105, "P", Some(5), Some("GB"), &[PPON, "GB-NHS-RTH"], "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST")
        .await;
    // `Oxford Hospital` is over the generic wall: two carriers of its N2 key, cap one.
    b.provisional(90, "Oxford Hospital Elsewhere").await;
    for org in [4, 90] {
        b.conn
            .execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'oxford hospital')",
                (Value::Integer(org),),
            )
            .await
            .unwrap();
    }
    // A case review names org 1; the body's own verdict names 1 and 2 — not "review".
    b.conn
        .execute(
            "INSERT INTO org_case_reviews (case_org_id, cohort, verdict, diagnosis, handling, rationale, confidence, reviewed_at)
             VALUES (1, 'c', 'v', 'd', 'h', 'r', 'high', 0)",
            (),
        )
        .await
        .unwrap();
    b.verdict("RTH", vec![1, 2], Some(vec![1, 2, 3, 4, 5]), "merge", "high").await;

    let r = b.plan_with(NhsBodyCaps::PRODUCTION, 1).await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 2, 3, 4, 5]);
    let denied: Vec<(i64, Vec<&str>)> = rth.holders.iter().map(|h| (h.org_id, h.denied.clone())).collect();
    assert_eq!(
        denied,
        vec![(1, vec![]), (2, vec!["minted-identity"]), (3, vec!["consortium"]), (4, vec!["generic"]), (5, vec![])],
        "{rth:#?}"
    );
    assert_eq!(rth.eligible, vec![1, 5]);
    assert!(holder(rth, 2).minted_identity && !holder(rth, 5).minted_identity);
    assert_eq!(holder(rth, 2).identifier.as_deref(), Some("GBNHSRF1"));
    assert_eq!(
        (r.denied.get("minted-identity"), r.denied.get("consortium"), r.denied.get("generic")),
        (Some(&1), Some(&1), Some(&1))
    );
    assert_eq!(rth.verdict.as_ref().map(|v| v.status), Some("member-denied"), "{rth:#?}");
    assert_eq!((r.admitted_verdict, r.verdict_stale), (0, 1));
    let review: Vec<(i64, bool)> = rth.holders.iter().map(|h| (h.org_id, h.under_review)).collect();
    assert_eq!(review, vec![(1, true), (2, false), (3, false), (4, false), (5, false)]);
}

/// D6's contest: one org named a member by the merge verdicts of two codes (a Walton
/// cluster one stray notice coded REN) is denied in both, and neither verdict is admitted.
/// Before any verdict it is only a holder of two codes, flagged and listed.
#[tokio::test]
async fn an_org_proposed_for_two_codes_is_contested() {
    let b = bed("contested").await;
    b.provisional(1, "The Walton Centre NHS FT").await;
    b.coded(101, 1, "GB-NHS-RET").await;
    b.coded(102, 1, "GB-NHS-REN").await;
    b.provisional(2, "The Clatterbridge Cancer Centre NHS FT").await;
    b.coded(103, 2, "GB-NHS-REN").await;

    let r = b.plan().await;
    assert_eq!((r.multi_code_orgs, r.contested_orgs), (1, 0));
    let (ren, ret) = (code(&r, "REN"), code(&r, "RET"));
    assert_eq!((ren.holder_ids.clone(), ret.holder_ids.clone()), (vec![1, 2], vec![1]));
    assert_eq!(holder(ren, 1).holder_of, vec!["REN", "RET"]);
    assert_eq!(holder(ren, 1).other_codes, vec!["RET".to_owned()]);
    assert_eq!(holder(ret, 1).other_codes, vec!["REN".to_owned()]);
    assert_eq!((holder(ren, 1).check.verdict, holder(ren, 1).check.rival), ("better", Some("RET")));
    assert!(holder(ren, 1).denied.is_empty(), "a second code alone denies nothing");

    b.verdict("REN", vec![1, 2], Some(vec![1, 2]), "merge", "high").await;
    b.verdict("RET", vec![1], Some(vec![1]), "merge", "high").await;
    let r = b.plan().await;
    assert_eq!(r.contested_orgs, 1);
    for c in ["REN", "RET"] {
        let entry = code(&r, c);
        let one = holder(entry, 1);
        assert!(one.contested, "{entry:#?}");
        assert_eq!(one.denied, vec!["contested"]);
        assert_eq!(entry.verdict.as_ref().map(|v| v.status), Some("contested"), "{c}");
    }
    assert_eq!(code(&r, "REN").eligible, vec![2]);
    assert_eq!((r.admitted_verdict, r.verdict_stale, r.denied.get("contested")), (0, 2, Some(&2)));
}

/// D6: a legally-ended code never absorbs its successor's names. QNQ (Frimley) ended on
/// 2026-03-31 with QRL among its successors; a QNQ-coded cluster named as the Hampshire
/// and Isle of Wight ICB is the successor's name and is denied. The census also counts the
/// coded notices published after the legal end (U1's promise) — two before it and one
/// after, so counting the wrong side reads 2.
#[tokio::test]
async fn a_successor_name_member_is_denied() {
    let b = bed("successor").await;
    b.provisional(1, "NHS Frimley ICB").await;
    b.coded_at(101, 1, "GB-NHS-QNQ", Some(Y2025_06_01)).await;
    b.coded_at(103, 1, "GB-NHS-QNQ", Some(Y2025_02_01)).await;
    b.provisional(2, "NHS Hampshire and Isle of Wight Integrated Care Board").await;
    b.coded_at(102, 2, "GB-NHS-QNQ", Some(Y2026_05_01)).await;

    let r = b.plan().await;
    let qnq = code(&r, "QNQ");
    assert_eq!((qnq.ods.legal_end, qnq.ods.successors.clone()), (Some("2026-03-31"), vec!["QRL"]));
    assert_eq!(qnq.coded_parties, 3);
    assert_eq!(qnq.coded_notices_after_legal_end, 1, "notice 102 alone is past the end: {qnq:#?}");
    assert_eq!(holder(qnq, 1).successor_name, None);
    assert_eq!(holder(qnq, 2).successor_name, Some("QRL"));
    assert_eq!(holder(qnq, 2).denied, vec!["successor-name"]);
    assert_eq!(qnq.eligible, vec![1]);
    assert_eq!(r.denied.get("successor-name"), Some(&1));
}

/// A party publishing the code behind another first identifier — a company number (DQF is
/// a company), a charity number, a PPON the crosswalk cannot key — still binds its holder.
/// The fold keeps only the first identifier as the raw one, so without this walk the org
/// its mention bound would hold the code unseen, and a verdict reviewed without it would
/// read admitted. It holds `party-later`; a PPON-led party's org stays the PPON's holder
/// (`ppon`). Such a party with no country or a non-GB one is counted over every ordinal.
#[tokio::test]
async fn a_code_behind_another_first_identifier_still_binds_its_holder() {
    let b = bed("party-later").await;
    let ouh = "Oxford University Hospitals NHS Foundation Trust";
    b.provisional(1, ouh).await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.notice(102, None).await;
    b.party(102, "H", Some(1), Some("GB"), &["GB-CHC-1234567", "GB-NHS-RTH"], ouh).await;
    b.org(2, "GBCOH03914810", "Oxford University Hospitals Services Ltd").await;
    b.notice(103, None).await;
    b.party(103, "C", Some(2), Some("GB"), &["GB-COH-03914810", "GB-NHS-RTH"], "Oxford University Hospitals Services Ltd")
        .await;
    // A PPON the crosswalk cannot key (a short serial): no pair, no PPON-first holder.
    b.provisional(3, "Oxford University Hospitals NHS FT").await;
    b.notice(104, None).await;
    b.party(104, "U", Some(3), Some("GB"), &["GB-PPON-ABC-123", "GB-NHS-RTH"], "Oxford University Hospitals NHS FT")
        .await;
    b.org(7, &minted(PPON), "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST").await;
    b.notice(105, None).await;
    b.party(105, "P", Some(7), Some("GB"), &[PPON, "GB-NHS-RTH"], "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST")
        .await;
    // Behind a company number under no country, and behind a SIRET under FR: counted only.
    b.provisional(4, "Oxford Somewhere").await;
    b.notice(106, None).await;
    b.party(106, "N", Some(4), None, &["GB-COH-01234567", "GB-NHS-RTH"], "Oxford Somewhere").await;
    b.provisional(5, "Hôpital d'Oxford").await;
    b.notice(107, None).await;
    b.party(107, "F", Some(5), Some("FR"), &["FR-SIRET-12345678900011", "GB-NHS-RTH"], "Hôpital d'Oxford").await;

    let r = b.plan().await;
    assert_eq!((r.coded_parties, r.later_coded_parties, r.ppon_pairs), (1, 3, 1), "{r:#?}");
    assert_eq!((r.countryless_parties, r.non_gb_parties), (1, 1));
    assert_eq!(
        (r.countryless_sample[0].publication_id.as_str(), r.countryless_sample[0].value.as_str()),
        ("pub-106", "GB-NHS-RTH")
    );
    assert_eq!((r.non_gb_sample[0].country.as_deref(), r.non_gb_sample[0].value.as_str()), (Some("FR"), "GB-NHS-RTH"));
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 2, 3, 7], "every org a GB party bound by the code, at any ordinal");
    let via: Vec<(i64, &str, u64, u64)> =
        rth.holders.iter().map(|h| (h.org_id, h.via, h.coded_parties, h.later_parties)).collect();
    assert_eq!(
        via,
        vec![(1, "party+party-later", 1, 1), (2, "party-later", 0, 1), (3, "party-later", 0, 1), (7, "ppon", 0, 0)]
    );
    let tails: Vec<(&str, u64, u64)> = rth.tails.iter().map(|t| (t.tail.as_str(), t.parties, t.later_parties)).collect();
    assert_eq!(tails, vec![("RTH", 1, 3)]);
    assert_eq!((rth.coded_parties, rth.later_parties), (1, 3));
    // The company is listed for the reviewer, and D6 still bars it.
    assert_eq!(holder(rth, 2).denied, vec!["minted-identity"]);
    assert_eq!(rth.eligible, vec![1, 3, 7]);
}

/// The four ways a code reaches a PPON holder, each on its own so none hides another's
/// loss: (a) the org carrying the PPON triple as its own identifier, named by no
/// PPON-first party — a code-first party `[GB-NHS-RTH, PPON]` makes the pair; (b) a
/// survivor carrying the PPON only as a merged identifier; (c) a `wrong` identifier verdict
/// on the triple withholds its carrier; (d) a PPON-first party binds an org without the
/// triple, a holder through that party alone.
#[tokio::test]
async fn each_ppon_holder_path_reaches_its_own_holder() {
    let b = bed("ppon-paths").await;
    b.provisional(1, "Oxford University Hospitals NHS FT").await;
    b.notice(101, None).await;
    b.party(101, "A", Some(1), Some("GB"), &["GB-NHS-RTH", PPON], "Oxford University Hospitals NHS FT").await;
    // (a) The triple's own carrier, which no FTS party names.
    b.org(7, &minted(PPON), "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST").await;
    let r = b.plan().await;
    assert_eq!((r.coded_parties, r.later_coded_parties, r.ppon_pairs), (1, 0, 1), "{r:#?}");
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 7]);
    assert_eq!(vias(rth), vec![(1, "party"), (7, "ppon")]);
    assert_eq!(holder(rth, 7).ppons, vec![PPON_KEY.to_owned()]);

    // (b) A survivor that absorbed a PPON org: the triple only as a merged identifier.
    b.provisional(8, "Oxford University Hospitals").await;
    b.conn
        .execute(
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'GB', 8, 99, 'altid')",
            (Value::Text(minted(PPON)),),
        )
        .await
        .unwrap();
    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 7, 8]);
    assert_eq!(vias(rth), vec![(1, "party"), (7, "ppon"), (8, "ppon")]);
    assert_eq!(holder(rth, 8).merged_identifiers, vec![minted(PPON)]);

    // (c) A reviewer found the triple wrong on org 7: withheld, no longer a holder.
    b.conn
        .execute(
            "INSERT INTO org_identifier_verdicts
                 (identifier, identifier_kind, country, org_id, cohort, verdict, rationale, confidence, reviewed_at)
             VALUES (?, 'national', 'GB', 7, 'c', 'wrong', 'r', 'high', 0)",
            (Value::Text(minted(PPON)),),
        )
        .await
        .unwrap();
    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 8], "{rth:#?}");
    assert_eq!(vias(rth), vec![(1, "party"), (8, "ppon")]);

    // (d) A PPON-first party bound by name to an org without the triple.
    b.provisional(9, "Oxford University Hospitals NHS Trust").await;
    b.notice(109, None).await;
    b.party(109, "P", Some(9), Some("GB"), &[PPON, "GB-NHS-RTH"], "Oxford University Hospitals NHS Trust").await;
    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 8, 9]);
    assert_eq!(vias(rth), vec![(1, "party"), (8, "ppon"), (9, "ppon")]);
    assert_eq!((holder(rth, 9).coded_parties, holder(rth, 9).later_parties), (0, 0), "the PPON-first party alone");
    assert_eq!(holder(rth, 9).ppons, vec![PPON_KEY.to_owned()]);
}

/// A holder whose org row no longer stands — merged or swept between the harvest and its
/// read (the reader is no snapshot across statements) — is listed and denied `gone`: its
/// facts are empty, so without the gate it would read eligible. A verdict naming it a
/// member is `member-gone`, never admitted.
#[tokio::test]
async fn a_holder_whose_org_is_gone_is_denied_and_its_verdict_never_admitted() {
    let b = bed("gone").await;
    b.provisional(1, "Oxford University Hospitals NHS Foundation Trust").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.provisional(2, "Oxford University Hospitals NHS FT").await;
    b.coded(102, 2, "GB-NHS-RTH").await;
    // The fold bound this party to org 3, whose row is gone (foreign keys are off here).
    b.notice(103, None).await;
    b.party(103, "G", Some(3), Some("GB"), &["GB-NHS-RTH"], "Oxford Univ Hospitals").await;
    b.verdict("RTH", vec![1, 3], Some(vec![1, 2, 3]), "merge", "high").await;

    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 2, 3]);
    let gone = holder(rth, 3);
    assert_eq!((gone.stands, gone.denied.clone(), gone.check.verdict), (false, vec!["gone"], "unknown"), "{gone:#?}");
    assert_eq!(rth.eligible, vec![1, 2]);
    assert_eq!(r.denied.get("gone"), Some(&1));
    assert_eq!(rth.verdict.as_ref().map(|v| v.status), Some("member-gone"), "{rth:#?}");
    assert_eq!((r.admitted_verdict, r.verdict_stale, r.verdict_status.get("member-gone")), (0, 1, Some(&1)));
}

/// D4: a member's alias key is its own 234 key (head `name_norm`, GB), and one a standing
/// identifier-NULL row OUTSIDE the body owns is `alias_key_owned_elsewhere` at U4 — that
/// member is held. The census names the owner on the holder and holds the verdict
/// (`member-key-owned-elsewhere`, not admitted). A verdict that leaves the holder out of
/// its members is not held by it.
#[tokio::test]
async fn a_member_whose_key_a_row_outside_the_body_owns_holds_the_verdict() {
    let b = bed("key-owner").await;
    b.provisional(1, "Oxford University Hospitals NHS FT").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.org(7, &minted(PPON), "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST").await;
    b.notice(107, None).await;
    b.party(107, "P", Some(7), Some("GB"), &[PPON, "GB-NHS-RTH"], "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST")
        .await;
    // The PPON org's 234 key, on a standing identifier-NULL row that publishes no code.
    b.provisional(50, "Oxford University Hospitals NHS Foundation Trust").await;
    b.verdict("RTH", vec![1, 7], Some(vec![1, 7]), "merge", "high").await;

    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert_eq!(rth.holder_ids, vec![1, 7], "org 50 is no holder");
    assert_eq!(holder(rth, 7).key_owners_elsewhere, vec![50], "{rth:#?}");
    assert!(holder(rth, 1).key_owners_elsewhere.is_empty());
    assert_eq!(r.key_owned_elsewhere, 1);
    assert_eq!(rth.eligible, vec![1, 7], "D4 holds the verdict, not the holder's listing");
    assert_eq!(rth.verdict.as_ref().map(|v| v.status), Some("member-key-owned-elsewhere"));
    assert_eq!((r.admitted_verdict, r.verdict_status.get("member-key-owned-elsewhere")), (0, Some(&1)));

    b.verdict("RTH", vec![1], Some(vec![1, 7]), "merge", "high").await;
    let r = b.plan().await;
    assert_eq!(code(&r, "RTH").verdict.as_ref().map(|v| v.status), Some("admitted"));
}

/// D6's consortium gate reads the HEAD name only: a consortium-shaped head is denied, but
/// a stray consortium-shaped mention spelling on the body's PPON org — one among
/// thousands, perhaps past the 64 names kept — is listed (`consortium_names`, the most
/// frequent first, capped) and denies nothing. The generic wall reads the head name only
/// too: a mention spelling over the wall flags nothing.
#[tokio::test]
async fn a_consortium_mention_spelling_is_listed_and_only_the_head_name_denies() {
    let b = bed("consortium").await;
    b.org(7, &minted(PPON), "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST").await;
    b.notice(107, None).await;
    b.party(107, "P", Some(7), Some("GB"), &[PPON, "GB-NHS-RTH"], "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST")
        .await;
    for (notice, name) in [
        (110, "Oxford Health Consortium"),
        (111, "Oxford Health Consortium"),
        (112, "Thames Valley Consortium"),
        (113, "Oxford Hospital"),
    ] {
        b.notice(notice, None).await;
        b.mention(notice, &format!("ORG-S{notice}"), 7, Some("GB"), None, name).await;
    }
    b.provisional(3, "Oxford NHS Consortium").await;
    b.coded(103, 3, "GB-NHS-RTH").await;
    // `Oxford Hospital` (a mention of org 7) is over the generic wall: two carriers, cap one.
    for org in [80, 90] {
        b.conn
            .execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'oxford hospital')",
                (Value::Integer(org),),
            )
            .await
            .unwrap();
    }

    let r = b.plan_with(NhsBodyCaps::PRODUCTION, 1).await;
    let rth = code(&r, "RTH");
    let (ppon_org, head) = (holder(rth, 7), holder(rth, 3));
    assert_eq!((ppon_org.consortium, ppon_org.generic, ppon_org.denied.clone()), (false, false, vec![]), "{ppon_org:#?}");
    assert_eq!(ppon_org.consortium_names, vec!["Oxford Health Consortium", "Thames Valley Consortium"]);
    assert_eq!((head.consortium, head.denied.clone()), (true, vec!["consortium"]));
    assert_eq!(head.consortium_names, vec!["Oxford NHS Consortium"], "its own mention spells the head");
    assert_eq!(rth.eligible, vec![7]);
    assert_eq!(r.denied.get("consortium"), Some(&1));

    let cut = b.plan_with(NhsBodyCaps { consortium_names: 1, ..NhsBodyCaps::PRODUCTION }, 1).await;
    assert_eq!(holder(code(&cut, "RTH"), 7).consortium_names, vec!["Oxford Health Consortium"]);
}

/// Every cut is said: the holder listing (report-wide) and a holder's mention read are
/// capped, the flags say so, a code's `holder_ids` stay whole, and a verdict whose holders
/// were not all listed is never admitted.
#[tokio::test]
async fn the_census_cuts_are_honest() {
    let b = bed("cuts").await;
    b.provisional(1, "Oxford University Hospitals NHS Foundation Trust").await;
    b.coded(101, 1, "GB-NHS-RTH").await;
    b.notice(110, None).await;
    b.mention(110, "ORG-A", 1, Some("GB"), None, "OUH Oxford").await;
    b.notice(111, None).await;
    b.mention(111, "ORG-B", 1, Some("GB"), None, "Oxford University Hospitals").await;
    b.provisional(2, "Oxford University Hospitals NHS FT").await;
    b.coded(102, 2, "GB-NHS-RTH").await;
    b.provisional(3, "Oxford Univ Hospitals").await;
    b.coded(103, 3, "GB-NHS-RTH").await;
    b.verdict("RTH", vec![1, 2], Some(vec![1, 2, 3]), "merge", "high").await;

    let caps = NhsBodyCaps { holders: 2, mentions: 2, names: 1, ..NhsBodyCaps::PRODUCTION };
    let r = b.plan_with(caps, 20).await;
    let rth = code(&r, "RTH");
    assert_eq!((rth.holder_ids.clone(), rth.holders.len(), rth.holders_truncated), (vec![1, 2, 3], 2, true));
    assert!(r.holders_truncated && r.coded_share_partial);
    let first = holder(rth, 1);
    assert_eq!((first.mentions, first.mentions_read, first.mentions_capped), (3, 2, true));
    assert_eq!((first.name_norms.len(), first.names_capped), (1, true));
    assert_eq!(rth.verdict.as_ref().map(|v| v.status), Some("holders-unlisted"));
    assert_eq!(r.admitted_verdict, 0);
}

/// What the census counts and takes no holder from: a country-less and a non-GB coded
/// party, a GB-NHS value that is no three-character code, a party the fold has not reached.
/// And the coded share by notice year, over the holders' mentions.
#[tokio::test]
async fn the_census_counts_what_it_takes_no_holder_from() {
    let b = bed("extras").await;
    b.provisional(1, "Oxford University Hospitals NHS Foundation Trust").await;
    b.coded_at(101, 1, "GB-NHS-RTH", Some(Y2024_03_01)).await;
    b.notice(102, Some(Y2025_02_01)).await;
    b.mention(102, "ORG-U", 1, Some("GB"), None, "Oxford University Hospitals NHS Foundation Trust").await;
    b.notice(103, None).await;
    b.mention(103, "ORG-V", 1, Some("GB"), None, "Oxford University Hospitals NHS Foundation Trust").await;
    b.provisional(2, "Oxford Somewhere").await;
    b.notice(104, None).await;
    b.party(104, "C", Some(2), None, &["GB-NHS-RTH"], "Oxford Somewhere").await;
    b.provisional(3, "Hôpital d'Oxford").await;
    b.notice(105, None).await;
    b.party(105, "F", Some(3), Some("FR"), &["GB-NHS-RTH"], "Hôpital d'Oxford").await;
    b.provisional(4, "NHS Cheshire and Merseyside").await;
    b.notice(106, None).await;
    b.party(106, "T", Some(4), Some("GB"), &["GB-NHS-NHS Cheshire and Merseyside ICB"], "NHS Cheshire and Merseyside")
        .await;
    b.notice(107, None).await;
    b.party(107, "Z", None, Some("GB"), &["GB-NHS-RTH"], "unfolded").await;

    let r = b.plan().await;
    assert_eq!((r.party_sections, r.unfolded_sections, r.coded_parties), (5, 1, 1), "{r:#?}");
    assert_eq!((r.countryless_parties, r.non_gb_parties, r.other_nhs_values), (1, 1, 1));
    assert_eq!(
        (
            r.countryless_sample[0].publication_id.as_str(),
            r.countryless_sample[0].value.as_str(),
            r.countryless_sample[0].country.clone()
        ),
        ("pub-104", "GB-NHS-RTH", None)
    );
    assert_eq!(r.non_gb_sample[0].country.as_deref(), Some("FR"));
    assert_eq!(r.other_nhs_sample[0].value, "GB-NHS-NHS Cheshire and Merseyside ICB");
    assert_eq!(code(&r, "RTH").holder_ids, vec![1], "a country-less or non-GB party binds no holder");
    let years: Vec<(&str, u64, u64)> =
        r.coded_share_by_year.iter().map(|y| (y.year.as_str(), y.coded, y.mentions)).collect();
    assert_eq!(years, vec![("unknown", 0, 1), ("2024", 1, 1), ("2025", 0, 1)]);
    assert!(!r.coded_share_partial);
    assert_eq!(r.ods_version, "ods-test");
}

/// The census's per-holder and per-party reads are seeks, never a walk of a table of 10⁸
/// mentions or 10⁷ orgs: the party mentions by the notice's ORG range, a PPON's merged
/// holders by the merged table's key, a holder's mention count and newest mentions through
/// `organization_mentions_org` (walked backwards, no sorter), and the 234 key's owners
/// through `organizations_name_country`.
#[tokio::test]
async fn the_census_reads_seek_their_indexes() {
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
    let party = plan(store::NHS_BODY_PARTY_MENTIONS_SQL, vec![Value::Integer(1)]).await;
    assert!(party.contains("SEARCH organization_mentions") && !party.contains("SCAN"), "{party}");
    let merged = plan(store::NHS_BODY_MERGED_HOLDERS_SQL, vec![text("GBPPONPHDQ2359NZMP")]).await;
    assert!(merged.contains("SEARCH organization_merged_identifiers") && !merged.contains("SCAN"), "{merged}");
    let count = plan(store::NHS_BODY_MENTION_COUNT_SQL, vec![Value::Integer(1)]).await;
    assert!(count.contains("organization_mentions_org") && !count.contains("SCAN"), "{count}");
    let newest = plan(store::NHS_BODY_HOLDER_MENTIONS_SQL, vec![Value::Integer(1), Value::Integer(10)]).await;
    assert!(
        newest.contains("organization_mentions_org")
            && !newest.contains("SCAN")
            && !newest.contains("SORTER")
            && !newest.contains("TEMP B-TREE"),
        "{newest}"
    );
    let owners = plan(store::NHS_BODY_KEY_OWNERS_SQL, vec![text("oxford"), text("GB"), Value::Integer(64)]).await;
    assert!(owners.contains("SEARCH organizations") && !owners.contains("SCAN"), "{owners}");
}

/// The column reaches an EXISTING database only through `MIGRATIONS` (the issue-372
/// lesson): an `org_merge_verdicts` in its pre-513 shape gains `reviewed_holders` on open,
/// its standing rows read NULL, and an nhs-body verdict records into it.
#[tokio::test]
async fn an_existing_verdict_table_gains_the_reviewed_holders_column() {
    let path = format!("/tmp/tender-db-nhs-body-migrate-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    {
        let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
        let c = raw.connect().unwrap();
        c.execute(
            "CREATE TABLE org_merge_verdicts (
                 country TEXT NOT NULL, scheme TEXT NOT NULL, key TEXT NOT NULL, cohort TEXT NOT NULL,
                 members TEXT NOT NULL, action TEXT NOT NULL CHECK (action IN ('merge','keep')),
                 rationale TEXT NOT NULL, confidence TEXT NOT NULL CHECK (confidence IN ('high','medium','low')),
                 reviewed_at INTEGER NOT NULL, applied_at INTEGER, applied_action TEXT, job_id INTEGER,
                 PRIMARY KEY (country, scheme, key, cohort)
             ) STRICT",
            (),
        )
        .await
        .unwrap();
        c.execute(
            "INSERT INTO org_merge_verdicts (country, scheme, key, cohort, members, action, rationale, confidence, reviewed_at)
             VALUES ('GB', 'GB:altid', '03914810~PHDQ2359NZMP', 'old', '[1,2]', 'merge', 'r', 'high', 0)",
            (),
        )
        .await
        .unwrap();
    }
    let db = store::Db::open(&path).await.unwrap();
    let c = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
    let mut rows = c
        .query("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'org_merge_verdicts'", ())
        .await
        .unwrap();
    let Value::Text(sql) = rows.next().await.unwrap().unwrap().get_value(0).unwrap() else { panic!("ddl") };
    drop(rows);
    assert!(sql.contains("reviewed_holders"), "the ALTER ran on the old table: {sql}");
    db.record_merge_verdicts("513-u2-test", &[nhs_verdict("RTH", vec![1], Some(vec![1, 3]), "merge", "high")], 1)
        .await
        .expect("records into the migrated table");
    let mut rows =
        c.query("SELECT scheme, reviewed_holders FROM org_merge_verdicts ORDER BY scheme", ()).await.unwrap();
    let mut got = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        got.push((row.get_value(0).unwrap(), row.get_value(1).unwrap()));
    }
    assert_eq!(
        got,
        vec![
            (Value::Text("GB:altid".into()), Value::Null),
            (Value::Text("GB:nhs-body".into()), Value::Text("[1,3]".into())),
        ]
    );
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

// ---- Issue 513 U4: the wet path (`merge_nhs_bodies`), the ledger-armed alias, the unwind.

const RTH_SPELLINGS: [&str; 3] =
    ["Oxford University Hospitals NHS Foundation Trust", "Oxford University Hospitals NHS FT", "OUH Oxford"];
const RTH_PPON_NAME: &str = "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST";

/// `crosswalk::mention_key` in small, for the altid alias the fold arms beside the body
/// alias: a GB `GB-PPON` raw to its PPON key.
fn mention_key(country: Option<&str>, raw: &str) -> Option<(&'static str, String, bool, bool)> {
    pair_key("GB-PPON", raw, country)
}

fn altid_rules() -> store::AltIdAliasRules {
    store::AltIdAliasRules {
        mention_key,
        consortium,
        legal_family: |_| None,
        name_key: norm,
        names_agree: |a, b| a == b,
        trim: |s| s.to_owned(),
        norm,
        owner_mention_cap: store::ALTID_ALIAS_OWNER_MENTIONS,
    }
}

/// A mention as the fold hands it to the resolver: GB, the published raw identifier, and
/// the identity the normaliser mints from it (none for a letter-only NHS code).
fn mention(notice: i64, name: &str, raw: Option<&str>) -> store::Mention {
    store::Mention {
        notice_id: notice,
        section_id: "ORG-N".into(),
        name: name.into(),
        country: Some("GB".into()),
        raw_identifier: raw.map(str::to_owned),
        scheme: raw.map(|r| r.splitn(3, '-').take(2).collect::<Vec<_>>().join("-")),
        identifier: raw.and_then(|r| register_identity(r, Some("GB"))),
        variants: Vec::new(),
    }
}

impl Bed {
    /// Oxford's body as U3 found such bodies: three spelling clusters (1, 2, 3) its
    /// `GB-NHS-RTH` parties bound by name — the first with an uncoded text-era mention too —
    /// the PPON org (7) a PPON-first party names beside the code, and a stray (8,
    /// `University of Oxford` typed RTH) the reviewer keeps out. The verdict: members
    /// {1, 2, 3, 7} over the holders {1, 2, 3, 7, 8}.
    async fn oxford(&self) {
        for (i, name) in RTH_SPELLINGS.iter().enumerate() {
            let org = i as i64 + 1;
            self.provisional(org, name).await;
            self.coded(100 + org, org, "GB-NHS-RTH").await;
        }
        self.notice(108, None).await;
        self.mention(108, "ORG-U", 1, Some("GB"), None, "Oxford University Hospitals NHS Foundation Trust").await;
        self.org(7, &minted(PPON), RTH_PPON_NAME).await;
        self.notice(107, None).await;
        self.party(107, "P", Some(7), Some("GB"), &[PPON, "GB-NHS-RTH"], RTH_PPON_NAME).await;
        self.provisional(8, "University of Oxford").await;
        self.coded(104, 8, "GB-NHS-RTH").await;
        self.verdict("RTH", vec![1, 2, 3, 7], Some(vec![1, 2, 3, 7, 8]), "merge", "high").await;
    }

    /// The Clatterbridge body with no PPON org: two spelling clusters, the larger (11, two
    /// coded mentions) the anchor, the smaller (12) its loser.
    async fn clatterbridge(&self) {
        self.provisional(11, "The Clatterbridge Cancer Centre NHS Foundation Trust").await;
        self.coded(111, 11, "GB-NHS-REN").await;
        self.coded(112, 11, "GB-NHS-REN").await;
        self.provisional(12, "Clatterbridge Cancer Centre").await;
        self.coded(113, 12, "GB-NHS-REN").await;
        self.verdict("REN", vec![11, 12], Some(vec![11, 12]), "merge", "high").await;
    }

    async fn admitted(&self) -> Vec<String> {
        self.plan().await.admitted_codes.iter().map(|c| (*c).to_owned()).collect()
    }

    async fn wet(&self, expect: &[String], cap: Option<u64>) -> store::turso::Result<NhsBodyReport> {
        self.db
            .merge_nhs_bodies(
                args(NhsBodyCaps::PRODUCTION, 20),
                store::NhsBodyWet { expect_codes: expect, known_deferred: &[], max_groups: cap, job_id: Some(77) },
            )
            .await
    }

    /// Dry, then wet over exactly what the dry run admitted.
    async fn fold_all(&self) -> NhsBodyReport {
        let expect = self.admitted().await;
        let r = self.wet(&expect, None).await.expect("the wet run");
        assert!(!r.stopped && r.residual_codes.is_empty() && r.held.is_empty(), "{r:#?}");
        r
    }

    /// FTS notices the fold has not reached: one party section each, no mention.
    async fn fresh(&self, notices: &[i64]) {
        for &n in notices {
            self.notice(n, None).await;
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

    /// One fold's resolver over `mentions` — the canonical key and the consortium veto, as
    /// the fold opens it — armed the way the fold arms it (`arm_altid_alias`, which arms
    /// the body alias too), or not at all.
    async fn resolve(&self, armed: bool, mentions: &[store::Mention]) -> (Vec<i64>, store::NhsBodyAliasCounts) {
        let mut resolver =
            self.db.mention_resolver(Some(key), Some(consortium), None, Some(norm), None, None, 20).await.unwrap();
        if armed {
            self.db.arm_altid_alias(&mut resolver, altid_rules()).await.unwrap();
        }
        let ids = self.db.resolve_mentions(&mut resolver, mentions, 1).await.unwrap();
        let counts = store::Db::nhs_body_alias_counts(&resolver);
        self.db.finish_mention_resolver(resolver).await.unwrap();
        (ids, counts)
    }

    async fn text_of(&self, sql: &str) -> String {
        let mut rows = self.conn.query(sql, ()).await.unwrap();
        let got = match rows.next().await.unwrap().map(|r| r.get_value(0).unwrap()) {
            Some(Value::Text(t)) => t,
            other => panic!("{sql}: {other:?}"),
        };
        while rows.next().await.unwrap().is_some() {}
        got
    }

    /// A hand-written `nhs-body` ledger row, the shape `merge_nhs_bodies` writes, for the
    /// alias's poisoning and target rules.
    async fn ledger(&self, keep: i64, loser: i64, code: &str, loser_key: &str, target: &str, at: i64) {
        self.ledger_named(keep, loser, code, loser_key, target, &format!("Keep {code}"), at).await;
    }

    /// [`Self::ledger`] with the survivor's head name the row records (`keep_name`).
    #[allow(clippy::too_many_arguments)]
    async fn ledger_named(&self, keep: i64, loser: i64, code: &str, loser_key: &str, target: &str, keep_name: &str, at: i64) {
        self.ledger_evidence(
            keep,
            loser,
            &format!(
                "\"code\":\"{code}\",{target},\"keep_name\":\"{keep_name}\",\"loser_name_norm\":\"{loser_key}\",\
                 \"loser_country\":\"GB\""
            ),
            at,
        )
        .await;
    }

    /// An `nhs-body` ledger row carrying `fields` after the scheme.
    async fn ledger_evidence(&self, keep: i64, loser: i64, fields: &str, at: i64) {
        let evidence = format!("{{\"scheme\":\"GB:nhs-body\",{fields}}}");
        self.conn
            .execute(
                "INSERT INTO org_merge_log (keep, loser, rule, evidence, job_id, at) VALUES (?, ?, 'nhs-body', ?, 1, ?)",
                (Value::Integer(keep), Value::Integer(loser), Value::Text(evidence), Value::Integer(at)),
            )
            .await
            .unwrap();
    }

    async fn holders_of(&self, identifier: &str) -> Vec<i64> {
        let f = store::read::Filter { identifier: Some(identifier.into()), ..store::read::Filter::default() };
        store::read::organizations(&self.conn, &f, store::read::Scope::Page { after: 0, limit: 10 })
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect()
    }
}

fn anchor_target(name_norm: &str) -> String {
    format!("\"target\":\"anchor\",\"anchor_name_norm\":\"{name_norm}\",\"anchor_country\":\"GB\"")
}

fn ppon_target(literal: &str) -> String {
    format!(
        "\"target\":\"ppon\",\"target_identity\":\"{}\",\"target_identifier_kind\":\"national\",\"target_country\":\"GB\",\
         \"target_ppon\":\"{}\"",
        minted(literal),
        minted(literal).strip_prefix("GBPPON").unwrap()
    )
}

/// Design C's U3: a reviewed body folds its clusters into its PPON org. The losers are gone
/// and every reference they held is on the survivor (mentions, party rows); each loser has
/// one `nhs-body` ledger row whose evidence is identity-keyed (the alias replays from it)
/// and names what a reader checks the fold by; a loser id answers 308 to the survivor; the
/// code is recorded on the survivor, so `?identifier=GBNHSRTH` answers it; the verdict is
/// stamped applied. The stray the reviewer kept out keeps every mention.
#[tokio::test]
async fn a_reviewed_nhs_body_folds_its_clusters_into_the_ppon_org() {
    let b = bed("fold").await;
    b.oxford().await;
    b.conn
        .execute(
            "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
             VALUES (9, 1, 'buyer', 2, 102, 'ORG-N102')",
            (),
        )
        .await
        .unwrap();
    assert!(b.holders_of("GBNHSRTH").await.is_empty(), "no org answers the code before the fold");
    let r = b.fold_all().await;
    assert_eq!((r.expected_codes, r.deferred_codes.len(), r.expected_not_live), (1, 0, 0));
    assert_eq!((r.merged_bodies, r.recorded_bodies, r.removed, r.mentions, r.parties), (1, 0, 3, 4, 1), "{r:#?}");
    assert_eq!(r.tender_changes, 1);
    assert_eq!(
        r.folded,
        vec![store::NhsBodyFolded { code: "RTH", keep: 7, target: "ppon", losers: vec![1, 2, 3] }]
    );
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (1, 2, 3)").await, 0, "the losers are gone");
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 7").await, 5);
    assert_eq!(b.count("SELECT organization_id FROM tender_version_parties WHERE tender_id = 9").await, 7);
    // The non-member holder is untouched.
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 8").await, 1);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id = 8").await, 1);
    // The ledger, identity-keyed.
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body' AND keep = 7 AND job_id = 77").await, 3);
    let evidence = b.text_of("SELECT evidence FROM org_merge_log WHERE loser = 2").await;
    for (field, value) in [
        ("scheme", "GB:nhs-body"),
        ("code", "RTH"),
        ("ods_name", "OXFORD UNIVERSITY HOSPITALS NHS FOUNDATION TRUST"),
        ("target", "ppon"),
        ("target_identity", "GBPPONPHDQ2359NZMP"),
        ("target_identifier_kind", "national"),
        ("target_country", "GB"),
        ("target_ppon", PPON_KEY),
        ("keep_name", RTH_PPON_NAME),
        ("keep_id", "GBPPONPHDQ2359NZMP"),
        ("loser_id", ""),
        ("loser_kind", "name"),
        ("loser_name", "Oxford University Hospitals NHS FT"),
        ("loser_name_norm", "oxford university hospitals nhs ft"),
        ("loser_country", "GB"),
        ("cohort", "513-u2-test"),
    ] {
        assert!(evidence.contains(&format!("\"{field}\":\"{value}\"")), "{field} in {evidence}");
    }
    assert!(evidence.contains("\"loser_names\":[\"Oxford University Hospitals NHS FT\"]"), "{evidence}");
    assert!(evidence.contains("\"job_id\":77"), "{evidence}");
    assert_eq!(
        b.count("SELECT COUNT(*) FROM org_merge_log WHERE json_extract(evidence, '$.loser_name_norms[0]') = 'oxford university hospitals nhs ft'")
            .await,
        1,
        "the evidence parses as JSON"
    );
    assert_eq!(store::read::resolve_org(&b.conn, 2).await.unwrap(), store::read::OrgResolution::MergedInto(7));
    assert_eq!(b.holders_of("GBNHSRTH").await, vec![7], "the code answers the survivor");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_verdicts WHERE key = 'RTH' AND applied_at IS NOT NULL").await, 1);
    for (kind, id, op) in [("organization", 2, "removed"), ("organization", 7, "changed"), ("tender", 9, "changed")] {
        assert_eq!(
            b.count(&format!("SELECT COUNT(*) FROM changes WHERE entity_kind = '{kind}' AND entity_id = {id} AND op = '{op}'"))
                .await,
            1,
            "{kind} {id} {op}"
        );
    }
    // The report carries this run's ledger rows verbatim, and nothing regrew.
    assert_eq!((r.ledger_rows, r.nhs_body_merged.len()), (3, 3));
    assert!(r.nhs_body_merged.iter().all(|m| m.keep == 7 && m.evidence.contains("\"code\":\"RTH\"")));
    assert!(r.nhs_body_regrown.is_empty(), "{:#?}", r.nhs_body_regrown);
    // The re-dry: the verdict is applied, so nothing is admitted and a wet run folds nothing.
    let again = b.plan().await;
    assert_eq!(code(&again, "RTH").verdict.as_ref().map(|v| v.status), Some("applied"));
    assert_eq!(code(&again, "RTH").holder_ids, vec![7, 8]);
    assert!(again.admitted_codes.is_empty() && again.nhs_body_regrown.is_empty());
}

/// A body with no PPON org keeps its largest cluster as a provisional anchor, and the
/// anchor replays: after the fold the next mention of the loser's spelling binds the
/// anchor through the alias (the 234 probe would find nothing — the loser is gone — and
/// mint), and the anchor's own spelling binds it through 234 as ever.
#[tokio::test]
async fn an_anchor_body_keeps_its_largest_cluster_and_replays_through_the_anchor_key() {
    let b = bed("anchor").await;
    b.clatterbridge().await;
    let r = b.fold_all().await;
    assert_eq!(
        r.folded,
        vec![store::NhsBodyFolded { code: "REN", keep: 11, target: "anchor", losers: vec![12] }]
    );
    let evidence = b.text_of("SELECT evidence FROM org_merge_log WHERE loser = 12").await;
    assert!(
        evidence.contains("\"target\":\"anchor\"")
            && evidence.contains("\"anchor_name_norm\":\"the clatterbridge cancer centre nhs foundation trust\"")
            && evidence.contains("\"anchor_country\":\"GB\""),
        "{evidence}"
    );
    assert_eq!(b.holders_of("GBNHSREN").await, vec![11]);
    b.fresh(&[300, 301]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b
        .resolve(
            true,
            &[
                mention(300, "Clatterbridge Cancer Centre", None),
                mention(301, "The Clatterbridge Cancer Centre NHS Foundation Trust", None),
            ],
        )
        .await;
    assert_eq!(ids, vec![11, 11]);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs, "nothing minted");
    assert_eq!((c.ledger_rows, c.keys, c.asked, c.bound, c.minted, c.refused), (1, 1, 1, 1, 0, 0), "{c:?}");
}

/// S2: a wet run whose live census drifted from the stored plan by more than max(2%, 5)
/// bodies aborts before any write. Within tolerance it folds only live ∩ stored.
#[tokio::test]
async fn a_body_parity_drift_aborts_before_any_write() {
    let b = bed("parity").await;
    b.oxford().await;
    let stale: Vec<String> = ["QKK", "QMF", "QNQ", "QRL", "R0B", "RET"].iter().map(|c| (*c).to_owned()).collect();
    let err = b.wet(&stale, None).await.expect_err("seven bodies differ");
    let msg = err.to_string();
    assert!(msg.contains("nhs-body parity abort") && msg.contains("nothing was written"), "{msg}");
    assert!(msg.contains("1 admitted since the dry run, 6 no longer admitted"), "{msg}");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 0);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, 5);
    // A live body the stored plan lacks is deferred, never folded.
    let r = b.wet(&[], None).await.expect("within tolerance");
    assert_eq!((r.deferred_codes.clone(), r.folded.len()), (vec!["RTH"], 0), "{r:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 0);
}

/// A capped run folds a prefix in code order and leaves the rest as the residual; the
/// continuation, held to the residual, folds the rest.
#[tokio::test]
async fn a_capped_wet_run_leaves_a_residual() {
    let b = bed("capped").await;
    b.oxford().await;
    b.clatterbridge().await;
    let expect = b.admitted().await;
    assert_eq!(expect, vec!["REN", "RTH"]);
    let r = b.wet(&expect, Some(1)).await.unwrap();
    assert_eq!(r.folded.iter().map(|f| f.code).collect::<Vec<_>>(), vec!["REN"]);
    assert_eq!(r.residual_codes, vec!["RTH"]);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (1, 2, 3)").await, 3, "RTH untouched");
    let residual: Vec<String> = r.residual_codes.iter().map(|c| (*c).to_owned()).collect();
    let r = b.wet(&residual, None).await.unwrap();
    assert_eq!(r.folded.iter().map(|f| f.code).collect::<Vec<_>>(), vec!["RTH"]);
    assert!(r.residual_codes.is_empty() && r.deferred_codes.is_empty(), "{r:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body'").await, 4);
}

/// A one-member body (the census already reads it as one org) folds nothing and records
/// its code; two PPON members neither of which, or both of which, the code pairs are held.
#[tokio::test]
async fn a_one_member_body_records_its_code_and_an_ambiguous_survivor_is_held() {
    let b = bed("one-member-fold").await;
    b.provisional(21, "The Walton Centre NHS Foundation Trust").await;
    b.coded(121, 21, "GB-NHS-RET").await;
    b.verdict("RET", vec![21], Some(vec![21]), "merge", "high").await;
    let r = b.fold_all().await;
    assert_eq!((r.merged_bodies, r.recorded_bodies, r.removed), (0, 1, 0));
    assert_eq!(b.holders_of("GBNHSRET").await, vec![21]);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 0, "no loser, no ledger row");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_verdicts WHERE key = 'RET' AND applied_at IS NOT NULL").await, 1);

    let ppon_b = "GB-PPON-WXYZ-1234-ABCD";
    b.org(31, &minted(PPON), "Sunderland Trust A").await;
    b.org(32, &minted(ppon_b), "Sunderland Trust B").await;
    b.provisional(33, "South Tyneside and Sunderland NHS Foundation Trust").await;
    b.coded(131, 33, "GB-NHS-R0B").await;
    for (n, org, literal) in [(132, 31, PPON), (133, 32, ppon_b)] {
        b.notice(n, None).await;
        b.party(n, "P", Some(org), Some("GB"), &[literal, "GB-NHS-R0B"], "Sunderland Trust").await;
    }
    b.verdict("R0B", vec![31, 32, 33], Some(vec![31, 32, 33]), "merge", "high").await;
    let expect = b.admitted().await;
    assert_eq!(expect, vec!["R0B"]);
    let r = b.wet(&expect, None).await.unwrap();
    assert_eq!(
        r.held,
        vec![store::NhsBodyHeld { code: "R0B", reason: "survivor-ambiguous", members: vec![31, 32, 33] }]
    );
    assert_eq!(r.residual_codes, vec!["R0B"]);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (31, 32, 33)").await, 3, "nothing folded");
}

/// The alias's reason to exist (design C's U4). After the fold the loser rows are gone, so
/// the next uncoded mention of a merged spelling misses the 234 probe — which needs
/// `identifier IS NULL`, and the survivor is the PPON org — and the unarmed fold mints the
/// spelling's cluster again. Armed, it binds the survivor, mints nothing, and the code
/// already stands on the survivor, so nothing is recorded twice.
#[tokio::test]
async fn the_next_uncoded_mention_of_a_merged_spelling_binds_the_survivor() {
    let b = bed("next-mention").await;
    b.oxford().await;
    b.fold_all().await;
    b.fresh(&[300, 301]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b.resolve(true, &[mention(300, "Oxford University Hospitals NHS FT", None)]).await;
    assert_eq!(ids, vec![7]);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs);
    assert_eq!((c.ledger_rows, c.keys, c.asked, c.bound, c.minted, c.refused, c.recorded), (3, 3, 1, 1, 0, 0, 0), "{c:?}");
    let (ids, c) = b.resolve(false, &[mention(301, "Oxford University Hospitals NHS FT", None)]).await;
    assert_ne!(ids[0], 7, "unarmed, the regrowth the alias prevents");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 1);
    assert_eq!(c, store::NhsBodyAliasCounts::default(), "not armed, and says so");
    // S4: the regrown spelling is listed by the next census.
    let r = b.plan().await;
    assert_eq!(r.nhs_body_regrown.len(), 1, "{:#?}", r.nhs_body_regrown);
    let g = &r.nhs_body_regrown[0];
    assert_eq!((g.code.as_str(), g.kind, g.key.as_str(), g.org_id, g.survivor), ("RTH", "name", "oxford university hospitals nhs ft|GB", ids[0], Some(7)));
}

/// S5's store test: a rebuild in notice order gives one org per body. The rebuild keeps
/// `org_merge_log` and the verdicts and drops the org layer (ids renumber, the merged
/// identifiers go); the body's history resolves uncoded spellings years before its first
/// PPON mention. The first mention of the PPON body — a merged spelling — makes the alias
/// MINT the PPON org, named by the ledger's `keep_name`; the PPON-first mention then hits
/// it through the exact triple (E0); a coded second spelling binds it too; the code is
/// re-recorded once. The anchor body replays through the anchor key the same way.
#[tokio::test]
async fn the_body_alias_survives_a_rebuild_in_notice_order() {
    let b = bed("rebuild").await;
    b.oxford().await;
    b.clatterbridge().await;
    b.fold_all().await;
    for table in ["organization_mentions", "organization_names", "organization_merged_identifiers", "organizations"] {
        b.conn.execute(&format!("DELETE FROM {table}"), ()).await.unwrap();
    }
    b.fresh(&[400, 401, 402, 403, 404, 405, 406]).await;
    let (ids, c) = b
        .resolve(
            true,
            &[
                mention(400, "Oxford University Hospitals NHS FT", None),
                mention(401, PPON_NAME_FIRST, Some(PPON)),
                mention(402, "OUH Oxford", Some("GB-NHS-RTH")),
                mention(403, "Oxford University Hospitals NHS Foundation Trust", None),
                mention(404, "Clatterbridge Cancer Centre", Some("GB-NHS-REN")),
                mention(405, "The Clatterbridge Cancer Centre NHS Foundation Trust", None),
                mention(406, "University of Oxford", Some("GB-NHS-RTH")),
            ],
        )
        .await;
    let (rth, ren) = (ids[0], ids[4]);
    assert_eq!(&ids[..4], &[rth; 4], "one org for the PPON body: {ids:?}");
    assert_eq!(&ids[4..6], &[ren; 2], "one org for the anchor body: {ids:?}");
    assert!(ids[6] != rth && ids[6] != ren, "the stray stays its own: {ids:?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, 3);
    let head = b.text_of(&format!("SELECT name || '|' || identifier || '|' || provisional FROM organizations WHERE id = {rth}")).await;
    assert_eq!(head, format!("{RTH_PPON_NAME}|GBPPONPHDQ2359NZMP|0"), "the PPON row, named by the ledger");
    let anchor =
        b.text_of(&format!("SELECT name || '|' || name_norm || '|' || provisional FROM organizations WHERE id = {ren}")).await;
    assert_eq!(
        anchor,
        "The Clatterbridge Cancer Centre NHS Foundation Trust|the clatterbridge cancer centre nhs foundation trust|1"
    );
    assert_eq!((c.asked, c.bound, c.minted, c.refused), (4, 2, 2, 0), "{c:?}");
    assert_eq!(c.recorded, 2, "the codes come back once each");
    assert_eq!(b.holders_of("GBNHSRTH").await, vec![rth]);
    assert_eq!(b.holders_of("GBNHSREN").await, vec![ren]);
}

const PPON_NAME_FIRST: &str = "Oxford University Hospitals NHS Foundation Trust (OUH)";

/// Never by code: a coded mention whose name no reviewer read is not asked, and binds by
/// 234 exactly as unarmed — it mints its own cluster, a newcomer holder for the next cohort.
#[tokio::test]
async fn a_coded_mention_with_an_unreviewed_name_still_mints_its_own_cluster() {
    let b = bed("unreviewed").await;
    b.oxford().await;
    b.fold_all().await;
    b.fresh(&[300]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b.resolve(true, &[mention(300, "Oxford Radcliffe Hospitals", Some("GB-NHS-RTH"))]).await;
    assert_ne!(ids[0], 7);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 1);
    assert_eq!((c.asked, c.bound, c.minted), (0, 0, 0), "{c:?}");
    // The party's BT-501 row, as the parser leaves it: the census reads holders by it.
    b.conn
        .execute(
            "INSERT INTO notice_ids (notice_id, section_id, field_id, ordinal, scheme, value, is_ref)
             VALUES (300, 'ORG-N', 'BT-501-Organization-Company', 0, 'GB-NHS', 'GB-NHS-RTH', 0)",
            (),
        )
        .await
        .unwrap();
    let r = b.plan().await;
    let rth = code(&r, "RTH");
    assert!(rth.holder_ids.contains(&ids[0]), "a newcomer holder: {rth:#?}");
    assert_eq!(rth.verdict.as_ref().map(|v| v.status), Some("applied"));
}

/// A key two bodies' ledger rows claim is poisoned: asked, refused, and bound by 234 as
/// unarmed (here: the standing row is gone, so it mints).
#[tokio::test]
async fn a_name_claimed_by_two_bodies_binds_nobody() {
    let b = bed("two-bodies").await;
    b.provisional(40, "Keep RET").await;
    b.provisional(41, "Keep REN").await;
    b.ledger(40, 90, "RET", "walton and clatterbridge", "\"target\":\"anchor\",\"anchor_name_norm\":\"keep ret\",\"anchor_country\":\"GB\"", 1).await;
    b.ledger(41, 91, "REN", "walton and clatterbridge", "\"target\":\"anchor\",\"anchor_name_norm\":\"keep ren\",\"anchor_country\":\"GB\"", 1).await;
    b.ledger(41, 92, "REN", "clatterbridge only", "\"target\":\"anchor\",\"anchor_name_norm\":\"keep ren\",\"anchor_country\":\"GB\"", 1).await;
    b.fresh(&[300, 301]).await;
    let (ids, c) = b
        .resolve(true, &[mention(300, "Walton and Clatterbridge", None), mention(301, "Clatterbridge only", None)])
        .await;
    assert!(ids[0] != 40 && ids[0] != 41, "{ids:?}");
    assert_eq!(ids[1], 41);
    assert_eq!((c.ledger_rows, c.keys, c.poisoned), (3, 1, 1), "{c:?}");
    assert_eq!((c.asked, c.bound, c.refused, c.refused_poisoned), (2, 1, 1, 1), "{c:?}");
}

/// A PPON target binds only through ONE row carrying its exact triple, never under a
/// `wrong` verdict: on two rows, or withheld, the key binds nobody (469 U5's rule), and
/// the mention binds by 234.
#[tokio::test]
async fn a_withheld_or_shared_target_binds_nobody() {
    let b = bed("shared").await;
    b.org(50, &minted(PPON), "Trust One").await;
    b.org(51, &minted(PPON), "Trust One again").await;
    b.ledger(50, 90, "RTH", "a shared spelling", &ppon_target(PPON), 1).await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[mention(300, "A shared spelling", None)]).await;
    assert!(ids[0] != 50 && ids[0] != 51, "{ids:?}");
    assert_eq!((c.asked, c.refused, c.refused_shared, c.bound), (1, 1, 1, 0), "{c:?}");

    let b = bed("withheld").await;
    let other = "GB-PPON-WXYZ-1234-ABCD";
    b.org(52, &minted(other), "Trust Two").await;
    b.ledger(52, 91, "REN", "a withheld spelling", &ppon_target(other), 1).await;
    b.db
        .record_identifier_verdicts(
            "513-test",
            &[store::IdentifierVerdict {
                org_id: 52,
                identifier: minted(other),
                verdict: "wrong".into(),
                correct_identifier: None,
                rationale: "fixture".into(),
                confidence: "high".into(),
            }],
            0,
        )
        .await
        .unwrap();
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[mention(300, "A withheld spelling", None)]).await;
    assert_ne!(ids[0], 52);
    assert_eq!((c.asked, c.refused_shared), (1, 1), "{c:?}");
}

/// A PPON survivor a later altid merge folded into a register org is FOLLOWED through the
/// altid ledger (D5), never re-minted: the body's spelling binds the register org.
#[tokio::test]
async fn a_target_folded_by_a_later_altid_merge_is_followed_not_reminted() {
    let b = bed("followed").await;
    b.oxford().await;
    b.fold_all().await;
    // Issue 469 U5's shape: the PPON org 7 folded into a UKPRN org.
    b.org(60, "GBUKPRN10001234", "Oxford University Hospitals").await;
    b.conn.execute("UPDATE organization_mentions SET organization_id = 60 WHERE organization_id = 7", ()).await.unwrap();
    b.conn.execute("DELETE FROM organizations WHERE id = 7", ()).await.unwrap();
    b.conn
        .execute(
            "INSERT INTO org_merge_log (keep, loser, rule, evidence, job_id, at) VALUES (60, 7, 'e2-altid', ?, 2, 5)",
            (Value::Text(format!(
                "{{\"scheme\":\"GB:altid\",\"ppon\":\"{PPON_KEY}\",\"registry_scheme\":\"GB-UKPRN\",\
                 \"registry_kind\":\"national\",\"registry\":\"GBUKPRN10001234\"}}"
            )),),
        )
        .await
        .unwrap();
    b.fresh(&[300]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b.resolve(true, &[mention(300, "OUH Oxford", None)]).await;
    assert_eq!(ids, vec![60]);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs, "not re-minted");
    assert_eq!((c.asked, c.bound, c.minted), (1, 1, 0), "{c:?}");
}

/// Armed with no `nhs-body` row (every store before the first wet run), the resolver is the
/// pre-513 one: unarmed state, zero counts, and the same binding an unarmed resolver gives.
#[tokio::test]
async fn an_unarmed_resolver_is_byte_identical() {
    let b = bed("byte-identical").await;
    b.provisional(70, "Oxford University Hospitals NHS FT").await;
    b.fresh(&[300, 301, 302]).await;
    let (armed, c) = b.resolve(true, &[mention(300, "Oxford University Hospitals NHS FT", None), mention(301, "New Body", None)]).await;
    assert_eq!(c, store::NhsBodyAliasCounts::default());
    let (unarmed, _) = b.resolve(false, &[mention(302, "Oxford University Hospitals NHS FT", None)]).await;
    assert_eq!(armed[0], 70);
    assert_eq!(unarmed[0], 70);
    let mut resolver =
        b.db.mention_resolver(Some(key), Some(consortium), None, Some(norm), None, None, 20).await.unwrap();
    b.db.arm_nhs_body_alias(&mut resolver).await.unwrap();
    assert_eq!(store::Db::nhs_body_alias_counts(&resolver), store::NhsBodyAliasCounts::default());
    b.db.finish_mention_resolver(resolver).await.unwrap();
}

/// D4 at arming: a loser's key a standing identifier-NULL row owns stays that row's — the
/// alias never takes a key from a row outside the body — and the census lists it regrown.
#[tokio::test]
async fn a_key_a_standing_row_owns_is_not_armed() {
    let b = bed("owned").await;
    b.oxford().await;
    b.fold_all().await;
    b.provisional(80, "OUH Oxford").await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[mention(300, "OUH Oxford", None)]).await;
    assert_eq!(ids, vec![80]);
    assert_eq!((c.keys, c.owned_elsewhere, c.asked), (2, 1, 0), "{c:?}");
}

/// The ledger reads run at every fold and every census: seeks through the partial index
/// `org_merge_log_nhs_body`, never a walk of a ledger of millions of rows.
#[tokio::test]
async fn the_nhs_body_ledger_reads_seek_the_partial_index() {
    let b = bed("ledger-plans").await;
    for sql in [store::NHS_BODY_LEDGER_SQL, store::NHS_BODY_ALIAS_LEDGER_SQL] {
        let mut rows = b.conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            if let Ok(Value::Text(d)) = row.get_value(3) {
                plan.push_str(&d);
                plan.push('\n');
            }
        }
        assert!(plan.contains("org_merge_log_nhs_body") && !plan.contains("SCAN org_merge_log\n"), "{sql}:\n{plan}");
    }
}

/// D7, the round trip: fold, then unwind. Dry, it plans the dissolve and writes nothing.
/// Wet, the merged spellings go back to their own provisional rows (234) and the
/// survivor keeps its PPON mention; the ledger rows are retired and each loser id follows
/// its spelling's new row; the code leaves the survivor; the moved notices are re-queued;
/// and the alias, with no `nhs-body` row left, is not armed — the spelling binds its
/// re-minted row by 234.
#[tokio::test]
async fn the_unwind_round_trip_dissolves_the_body_back_through_234() {
    let b = bed("unwind").await;
    b.oxford().await;
    // A spelling a rehoming once put on the second cluster: not its head key, so the
    // unwind finds it only through the ledger's `loser_name_norms`.
    b.notice(109, None).await;
    b.mention(109, "ORG-R", 2, Some("GB"), None, "OUH NHS FT (Oxford)").await;
    // A party row and a bid-party row naming the second cluster's coded mention: the fold
    // repoints them onto the survivor, and the unwind must take them back with it.
    for sql in [
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (9, 1, 'buyer', 2, 102, 'ORG-N102')",
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (9, 1, 1, 'tenderer', 2, 102, 'ORG-N102')",
    ] {
        b.conn.execute(sql, ()).await.unwrap();
    }
    b.fold_all().await;
    assert_eq!(b.count("SELECT organization_id FROM tender_version_bid_parties WHERE tender_id = 9").await, 7);
    let unwind =
        |dry_run: bool| store::NhsBodyUnwindArgs { code: "RTH", dry_run, register_identity, key, job_id: Some(78) };
    let dry = b.db.unwind_nhs_body(unwind(true)).await.unwrap();
    assert_eq!((dry.ledger_rows, dry.survivor, dry.survivor_mentions), (3, Some(7), 6));
    assert_eq!((dry.moved, dry.by_name, dry.by_identity, dry.minted), (5, 5, 0, 4), "{dry:#?}");
    assert!(dry.targets.iter().any(|t| t.key == "ouh nhs ft (oxford)|GB" && t.mentions == 1), "{dry:#?}");
    assert!(dry.targets.iter().all(|t| t.org_id.is_none() && !t.standing), "{dry:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body'").await, 3, "dry writes nothing");

    let r = b.db.unwind_nhs_body(unwind(false)).await.unwrap();
    assert_eq!((r.moved, r.minted, r.ledger_retired, r.redirects, r.merged_identifiers_dropped), (5, 4, 3, 3, 1), "{r:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 7").await, 1, "its PPON mention");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body'").await, 0);
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body-retired'").await, 3);
    assert!(b.holders_of("GBNHSRTH").await.is_empty(), "the code left the survivor");
    let ft = b
        .count("SELECT organization_id FROM organization_mentions WHERE notice_id = 102")
        .await;
    assert_ne!(ft, 7);
    assert_eq!(
        b.text_of(&format!("SELECT name_norm FROM organizations WHERE id = {ft} AND identifier IS NULL")).await,
        "oxford university hospitals nhs ft"
    );
    assert_eq!(store::read::resolve_org(&b.conn, 2).await.unwrap(), store::read::OrgResolution::MergedInto(ft));
    // The party and bid-party rows went with the mention they name.
    assert_eq!((r.parties, r.bid_parties), (1, 1), "{r:#?}");
    assert_eq!(b.count("SELECT organization_id FROM tender_version_parties WHERE tender_id = 9").await, ft);
    assert_eq!(b.count("SELECT organization_id FROM tender_version_bid_parties WHERE tender_id = 9").await, ft);
    let announced = b.count("SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id = 9").await;
    assert!(announced >= 2, "the fold's and the unwind's: {announced}");
    // The first spelling's coded and uncoded mentions land on one row again.
    assert_eq!(
        b.count("SELECT COUNT(DISTINCT organization_id) FROM organization_mentions WHERE notice_id IN (101, 108)").await,
        1
    );
    assert_eq!(b.count("SELECT COUNT(*) FROM notices WHERE id IN (101, 102, 103, 108) AND projected = 0").await, 4);
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[mention(300, "Oxford University Hospitals NHS FT", None)]).await;
    assert_eq!(ids, vec![ft]);
    assert_eq!(c, store::NhsBodyAliasCounts::default(), "nothing left to arm");
    let after = b.plan().await;
    assert!(after.nhs_body_merged.is_empty() && after.nhs_body_regrown.is_empty());
    assert_eq!(code(&after, "RTH").verdict.as_ref().map(|v| v.status), Some("applied"), "a second fold needs a new verdict");
}

// ---- Issue 513 U4, the review fixes.

/// The unwind's survivor is read from the ledger's IDENTITY-keyed evidence and held to the
/// keep's chain (review finding): a from-archive rebuild re-mints every org id from 1 while
/// `org_merge_log` persists, so a row's `keep` can come to name an unrelated company. Here
/// the PPON body stands at a new id (70) and keep 7 is someone else's — who holds one of
/// the body's spellings, so the old id-only read would have moved that mention off them.
/// Dry and wet alike refuse with nothing written, and the census's regrown entry carries
/// no survivor rather than the unrelated org.
#[tokio::test]
async fn an_unwind_whose_keep_now_names_an_unrelated_org_is_refused() {
    let b = bed("renumbered").await;
    b.oxford().await;
    b.fold_all().await;
    for sql in [
        "UPDATE organizations SET id = 70 WHERE id = 7",
        "UPDATE organization_mentions SET organization_id = 70 WHERE organization_id = 7",
        "UPDATE organization_merged_identifiers SET org_id = 70 WHERE org_id = 7",
    ] {
        b.conn.execute(sql, ()).await.unwrap();
    }
    b.provisional(7, "Acme Facilities Ltd").await;
    b.notice(150, None).await;
    b.mention(150, "ORG-A", 7, Some("GB"), None, "OUH Oxford").await;
    let state = "SELECT (SELECT COUNT(*) FROM organizations) || '|' || (SELECT COUNT(*) FROM changes) || '|' || \
                 (SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body') || '|' || \
                 (SELECT COUNT(*) FROM organization_merged_identifiers) || '|' || \
                 (SELECT group_concat(notice_id || ':' || organization_id, ',') FROM organization_mentions)";
    let before = b.text_of(state).await;
    for dry_run in [true, false] {
        let err = b
            .db
            .unwind_nhs_body(store::NhsBodyUnwindArgs { code: "RTH", dry_run, register_identity, key, job_id: None })
            .await
            .expect_err("the keep names an unrelated org");
        let msg = err.to_string();
        assert!(msg.contains("does not name one live survivor") && msg.contains("nothing was written"), "{msg}");
        assert!(msg.contains("names org 70 but its keep 7 reaches org 7"), "{msg}");
        assert_eq!(b.text_of(state).await, before, "dry_run {dry_run}: nothing written");
    }
    assert_eq!(b.holders_of("GBNHSRTH").await, vec![70]);
    // S4's listing resolves its survivor the same way: none, never Acme.
    b.provisional(80, "Oxford University Hospitals NHS FT").await;
    let r = b.plan().await;
    let g: Vec<_> = r.nhs_body_regrown.iter().filter(|g| g.org_id == 80).collect();
    assert_eq!(g.len(), 1, "{:#?}", r.nhs_body_regrown);
    assert_eq!((g[0].kind, g[0].survivor), ("name", None), "{g:#?}");
}

/// A body whose verdict makes a PPON org a LOSER is held (`loser-ppon-unaliased`, review
/// finding): no alias re-binds a loser's PPON yet, so its next PPON-first mention would
/// mint the org again. Here the code pairs the first PPON org (41) and the second (42)
/// holds the code only through a coded party, so 41 would survive and 42 be deleted.
#[tokio::test]
async fn a_body_with_a_loser_ppon_org_is_held() {
    let b = bed("loser-ppon").await;
    let ppon_b = "GB-PPON-WXYZ-1234-ABCD";
    b.org(41, &minted(PPON), "Clatterbridge Trust A").await;
    b.org(42, &minted(ppon_b), "Clatterbridge Trust B").await;
    b.provisional(43, "The Clatterbridge Cancer Centre NHS Foundation Trust").await;
    b.notice(141, None).await;
    b.party(141, "P", Some(41), Some("GB"), &[PPON, "GB-NHS-REN"], "Clatterbridge Trust A").await;
    b.coded(142, 42, "GB-NHS-REN").await;
    b.coded(143, 43, "GB-NHS-REN").await;
    b.verdict("REN", vec![41, 42, 43], Some(vec![41, 42, 43]), "merge", "high").await;
    let expect = b.admitted().await;
    assert_eq!(expect, vec!["REN"]);
    let r = b.wet(&expect, None).await.unwrap();
    assert_eq!(
        r.held,
        vec![store::NhsBodyHeld { code: "REN", reason: "loser-ppon-unaliased", members: vec![41, 42, 43] }]
    );
    assert_eq!((r.residual_codes.clone(), r.folded.len()), (vec!["REN"], 0), "{r:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (41, 42, 43)").await, 3, "nothing folded");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log").await, 0);
}

/// D7's E0 arm and the redirect order, on a ledger written for the shape the wet run now
/// holds (`loser-ppon-unaliased`) — as a fold before that hold would have left it. RET was
/// folded into PPON org 31: its losers were PPON org 32 (B), B's identifier-NULL twin 34
/// on B's own head key, and a spelling cluster 33; B's PPON and the code were recorded on
/// 31. The unwind sends B's PPON-first mention back through E0 to a row carrying B's
/// triple, and the twin's and the cluster's spellings through 234; loser 32's id follows
/// its PPON (the identity home), not the twin's NULL row its head key also reaches.
#[tokio::test]
async fn the_unwind_returns_a_loser_ppon_org_through_e0_and_redirects_it_by_identity() {
    let b = bed("unwind-e0").await;
    let (ppon_b, twin, cluster) = ("GB-PPON-WXYZ-1234-ABCD", "The Walton Centre B", "Walton Centre Liverpool");
    let head = "THE WALTON CENTRE NHS FOUNDATION TRUST";
    b.org(31, &minted(PPON), head).await;
    // The losers stood once and the fold deleted them, so no mint reuses their ids.
    b.org(32, &minted(ppon_b), twin).await;
    b.provisional(33, cluster).await;
    b.provisional(34, twin).await;
    b.conn.execute("DELETE FROM organizations WHERE id IN (32, 33, 34)", ()).await.unwrap();
    for (n, ids, name) in [
        (131, vec![PPON, "GB-NHS-RET"], head),
        (132, vec![ppon_b], twin),
        (133, vec!["GB-NHS-RET"], twin),
        (134, vec!["GB-NHS-RET"], cluster),
    ] {
        b.notice(n, None).await;
        b.party(n, "N", Some(31), Some("GB"), &ids, name).await;
    }
    let at = 1_000;
    let target = ppon_target(PPON);
    let twin_key = store::org_name_norm(twin);
    let losers = [
        (32, minted(ppon_b), twin_key.clone()),
        (34, String::new(), twin_key.clone()),
        (33, String::new(), store::org_name_norm(cluster)),
    ];
    for (loser, literal, key) in losers {
        let kind = if literal.is_empty() { "name" } else { "ppon" };
        b.ledger_evidence(
            31,
            loser,
            &format!(
                "\"code\":\"RET\",{target},\"keep_name\":\"{head}\",\"loser_id\":\"{literal}\",\"loser_kind\":\"{kind}\",\
                 \"loser_name_norm\":\"{key}\",\"loser_name_norms\":[\"{key}\"],\"loser_country\":\"GB\""
            ),
            at,
        )
        .await;
    }
    for (identifier, loser) in [(minted(ppon_b), 32), ("GBNHSRET".to_owned(), 31)] {
        b.conn
            .execute(
                "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
                 VALUES (?, 'national', 'GB', 31, ?, 'nhs-body')",
                (Value::Text(identifier), Value::Integer(loser)),
            )
            .await
            .unwrap();
    }
    let r = b
        .db
        .unwind_nhs_body(store::NhsBodyUnwindArgs { code: "RET", dry_run: false, register_identity, key, job_id: None })
        .await
        .unwrap();
    assert_eq!((r.survivor, r.moved, r.by_name, r.by_identity, r.minted), (Some(31), 3, 2, 1, 3), "{r:#?}");
    assert_eq!((r.ledger_retired, r.redirects, r.merged_identifiers_dropped), (3, 3, 2), "{r:#?}");
    let b_row = b.count("SELECT organization_id FROM organization_mentions WHERE notice_id = 132").await;
    assert_eq!(
        b.text_of(&format!("SELECT identifier || '|' || name FROM organizations WHERE id = {b_row}")).await,
        format!("{}|{twin}", minted(ppon_b)),
        "B's PPON-first mention is on a row carrying its own triple"
    );
    let twin_row = b.count("SELECT organization_id FROM organization_mentions WHERE notice_id = 133").await;
    assert!(twin_row != b_row && twin_row != 31, "the twin's spelling went by name");
    assert_eq!(b.count("SELECT organization_id FROM organization_mentions WHERE notice_id = 131").await, 31, "A's own");
    assert_eq!(store::read::resolve_org(&b.conn, 32).await.unwrap(), store::read::OrgResolution::MergedInto(b_row));
    assert_eq!(store::read::resolve_org(&b.conn, 34).await.unwrap(), store::read::OrgResolution::MergedInto(twin_row));
}

/// A one-member body writes no ledger row — its fold only recorded the code — so its
/// unwind drops that record (review finding: otherwise no job could ever remove it).
#[tokio::test]
async fn a_one_member_body_unwinds_to_dropping_its_code() {
    let b = bed("one-member-unwind").await;
    b.provisional(21, "The Walton Centre NHS Foundation Trust").await;
    b.coded(121, 21, "GB-NHS-RET").await;
    b.verdict("RET", vec![21], Some(vec![21]), "merge", "high").await;
    b.fold_all().await;
    assert_eq!(b.holders_of("GBNHSRET").await, vec![21]);
    let unwind = |dry_run: bool| store::NhsBodyUnwindArgs { code: "RET", dry_run, register_identity, key, job_id: None };
    let dry = b.db.unwind_nhs_body(unwind(true)).await.unwrap();
    assert_eq!((dry.ledger_rows, dry.survivor, dry.merged_identifiers_dropped), (0, Some(21), 1), "{dry:#?}");
    assert_eq!(b.holders_of("GBNHSRET").await, vec![21], "dry writes nothing");
    let r = b.db.unwind_nhs_body(unwind(false)).await.unwrap();
    assert_eq!((r.ledger_rows, r.survivor, r.merged_identifiers_dropped, r.moved), (0, Some(21), 1, 0), "{r:#?}");
    assert!(b.holders_of("GBNHSRET").await.is_empty(), "the code left the org");
    assert_eq!(b.count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 21").await, 1);
}

/// A `keep` verdict standing on a folded code drops every ledger row of that code at
/// arming: nothing is armed, and the spelling binds by 234 exactly as unarmed (the loser
/// row is gone, so it mints).
#[tokio::test]
async fn a_keep_verdict_on_a_folded_code_drops_its_alias_rows() {
    let b = bed("keep-drop").await;
    b.oxford().await;
    b.fold_all().await;
    b.db.record_merge_verdicts("513-keep-test", &[nhs_verdict("RTH", vec![7], None, "keep", "high")], 1).await.unwrap();
    b.fresh(&[300]).await;
    let orgs = b.count("SELECT COUNT(*) FROM organizations").await;
    let (ids, c) = b.resolve(true, &[mention(300, "Oxford University Hospitals NHS FT", None)]).await;
    assert_eq!((c.ledger_rows, c.kept, c.keys, c.asked), (3, 3, 0, 0), "{c:?}");
    assert_ne!(ids[0], 7);
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations").await, orgs + 1, "bound by 234: minted");
}

/// Chains: an anchor body (REN, its loser key → the anchor's key) whose anchor a later
/// fold took into a PPON org (the anchor's key → the PPON). The loser's spelling follows
/// the chain to the standing PPON org; and in a rebuild, the PPON org is minted under the
/// PPON row's `keep_name` whichever key reaches it first (D5: the chained hop carries its
/// own name, not the first row's).
#[tokio::test]
async fn an_anchor_chain_follows_to_the_ppon_org_and_mints_it_under_one_name() {
    const ANCHOR: &str = "the clatterbridge cancer centre nhs foundation trust";
    const ANCHOR_NAME: &str = "The Clatterbridge Cancer Centre NHS Foundation Trust";
    const PPON_NAME: &str = "CLATTERBRIDGE CANCER CENTRE NHS FOUNDATION TRUST (PPON)";
    async fn rows(b: &Bed) {
        b.ledger_named(11, 12, "REN", "clatterbridge cancer centre", &anchor_target(ANCHOR), ANCHOR_NAME, 1).await;
        b.ledger_named(60, 11, "REN", ANCHOR, &ppon_target(PPON), PPON_NAME, 2).await;
    }
    let (anchor_name, ppon_name) = (ANCHOR_NAME, PPON_NAME);
    let b = bed("chain-standing").await;
    b.org(60, &minted(PPON), ppon_name).await;
    rows(&b).await;
    b.fresh(&[300]).await;
    let (ids, c) = b.resolve(true, &[mention(300, "Clatterbridge Cancer Centre", None)]).await;
    assert_eq!(ids, vec![60], "the loser's spelling follows the chain");
    assert_eq!((c.keys, c.poisoned, c.asked, c.bound, c.minted), (2, 0, 1, 1, 0), "{c:?}");

    let mut heads = Vec::new();
    for (case, order) in [("chain-rebuild-a", [0, 1]), ("chain-rebuild-b", [1, 0])] {
        let b = bed(case).await;
        rows(&b).await;
        b.fresh(&[400, 401]).await;
        let spellings = [("Clatterbridge Cancer Centre", 400), (anchor_name, 401)];
        let mentions: Vec<store::Mention> =
            order.iter().map(|&i| mention(spellings[i].1, spellings[i].0, None)).collect();
        let (ids, c) = b.resolve(true, &mentions).await;
        assert_eq!(ids[0], ids[1], "{case}: one org");
        assert_eq!((c.minted, c.bound), (1, 1), "{case}: {c:?}");
        heads.push(b.text_of(&format!("SELECT name || '|' || identifier FROM organizations WHERE id = {}", ids[0])).await);
    }
    assert_eq!(heads, vec![format!("{ppon_name}|{}", minted(PPON)); 2], "order-independent head");
}

/// Two anchor rows aimed at each other are a broken chain: both keys poisoned, so neither
/// spelling binds through the alias.
#[tokio::test]
async fn a_two_row_anchor_cycle_is_poisoned() {
    let b = bed("anchor-cycle").await;
    b.ledger(40, 90, "RET", "walton a", &anchor_target("walton b"), 1).await;
    b.ledger(41, 91, "RET", "walton b", &anchor_target("walton a"), 1).await;
    b.fresh(&[300]).await;
    let (_, c) = b.resolve(true, &[mention(300, "Walton A", None)]).await;
    assert_eq!((c.ledger_rows, c.keys, c.poisoned), (2, 0, 2), "{c:?}");
    assert_eq!((c.asked, c.refused, c.refused_poisoned, c.bound, c.minted), (1, 1, 1, 0, 0), "{c:?}");
}

/// A body's transaction that fails after an earlier body committed (review finding) ends
/// the folding there: the report comes back with the committed body folded, the failed
/// one and every later one in the residual and the error named — so the caller stores the
/// residual. With nothing committed yet, the error itself, nothing written.
#[tokio::test]
async fn a_body_that_fails_after_a_commit_leaves_the_residual() {
    let b = bed("fold-error").await;
    b.oxford().await;
    b.clatterbridge().await;
    let expect = b.admitted().await;
    assert_eq!(expect, vec!["REN", "RTH"]);
    b.conn
        .execute(
            "CREATE TRIGGER t513 BEFORE DELETE ON organizations WHEN OLD.id = 1
             BEGIN SELECT RAISE(ABORT, 'fixture: RTH refuses'); END",
            (),
        )
        .await
        .unwrap();
    let r = b.wet(&expect, None).await.expect("REN committed, so the run reports");
    assert_eq!(r.folded.iter().map(|f| f.code).collect::<Vec<_>>(), vec!["REN"]);
    assert_eq!(r.residual_codes, vec!["RTH"]);
    let failed = r.fold_error.clone().expect("the failure is named");
    assert!(failed.starts_with("RTH: ") && failed.contains("fixture: RTH refuses"), "{failed}");
    assert_eq!((r.merged_bodies, r.removed, r.mentions), (1, 1, 1), "RTH's rolled-back tallies are not counted: {r:#?}");
    assert_eq!(b.count("SELECT COUNT(*) FROM organizations WHERE id IN (1, 2, 3)").await, 3, "RTH rolled back");
    assert_eq!(b.count("SELECT COUNT(*) FROM org_merge_log WHERE rule = 'nhs-body'").await, 1);
    let residual: Vec<String> = r.residual_codes.iter().map(|c| (*c).to_owned()).collect();
    let err = b.wet(&residual, None).await.expect_err("nothing committed: the error itself");
    assert!(err.to_string().contains("fixture: RTH refuses"), "{err}");
}

/// S5's prod half, in the store: the canary opens a fresh resolver armed as the fold arms
/// it and re-resolves a folded body's recorded mentions — the coded and uncoded spellings
/// (the alias), the PPON-first party (E0), the stray (234) — and every one agrees with the
/// org it is recorded on, inside a transaction that is rolled back: nothing is written.
/// It is a real re-resolve, not the fold's idempotency early return: a mention recorded on
/// the wrong org disagrees.
#[tokio::test]
async fn the_canary_re_resolves_a_folded_body_and_writes_nothing() {
    let b = bed("canary").await;
    b.oxford().await;
    b.fold_all().await;
    let canary = |notices: &'static [i64]| store::NhsBodyCanaryArgs {
        notices,
        identity: |raw, _scheme, country| raw.and_then(|r| register_identity(r, country)),
        canon_key: Some(key),
        consortium: Some(consortium),
        anchors: None,
        norm: Some(norm),
        legal_form: None,
        hard_scheme: None,
        stoplist_cap: 20,
        altid: altid_rules(),
    };
    let state = "SELECT (SELECT COUNT(*) FROM organizations) || '|' || (SELECT COUNT(*) FROM changes) || '|' || \
                 (SELECT COUNT(*) FROM org_merge_log) || '|' || (SELECT COUNT(*) FROM organization_merged_identifiers) \
                 || '|' || (SELECT group_concat(notice_id || ':' || section_id || ':' || organization_id, ',') \
                            FROM organization_mentions)";
    let before = b.text_of(state).await;
    let r = b.db.nhs_body_canary(canary(&[101, 102, 103, 104, 107, 108, 999])).await.unwrap();
    assert_eq!((r.mentions, r.agree, r.disagree, r.minted), (6, 6, 0, 0), "{r:#?}");
    assert_eq!(r.notices_without_mentions, vec![999]);
    for binding in &r.bindings {
        let home = if binding.notice_id == 104 { 8 } else { 7 };
        assert_eq!((binding.stored, binding.resolved, binding.agree), (home, home, true), "{binding:#?}");
    }
    assert_eq!((r.alias.asked, r.alias.bound, r.alias.minted, r.alias.refused), (4, 4, 0, 0), "{:?}", r.alias);
    assert_eq!(b.text_of(state).await, before, "rolled back: nothing written");

    b.conn.execute("UPDATE organization_mentions SET organization_id = 8 WHERE notice_id = 102", ()).await.unwrap();
    let r = b.db.nhs_body_canary(canary(&[102])).await.unwrap();
    assert_eq!((r.mentions, r.agree, r.disagree), (1, 0, 1), "{r:#?}");
    assert_eq!((r.bindings[0].stored, r.bindings[0].resolved, r.bindings[0].agree), (8, 7, false));
    assert_eq!(b.count("SELECT organization_id FROM organization_mentions WHERE notice_id = 102").await, 8, "untouched");
}
