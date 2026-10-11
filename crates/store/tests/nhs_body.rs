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
/// letter-only NHS code mints nothing.
fn register_identity(raw: &str, country: Option<&str>) -> Option<store::Identifier> {
    let value = minted(raw);
    (value.len() >= 4 && value.chars().any(|c| c.is_ascii_digit())).then(|| store::Identifier {
        country: country.map(str::to_owned),
        kind: "national".into(),
        value,
    })
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
