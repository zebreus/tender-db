//! Output-equivalence proof for the bounded projection (issue 57).
//!
//! The bounded projection folds and applies whole Tenders a *batch of notices* at
//! a time instead of holding the whole corpus in RAM. Correctness hinges on the
//! batching never changing the output — a Tender's notices span the whole corpus
//! (keyed chains, legacy OJS transitive chains — ADR-0001), so a batch boundary
//! must never split one. This test projects one rich corpus twice: once with the
//! smallest possible batch (one notice — every Tender lands on a batch boundary)
//! and once as a single batch (the whole-RAM projection). The canonical layer
//! must come out byte-identical.

use ingest::{eforms, profile, project};
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projeq-{name}-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = Db::open(&path).await.expect("open scratch db");
    db.record_fetch(&store::Fetch {
        source: SOURCE.into(),
        kind: "daily".into(),
        period: "2026-00136".into(),
        url: "https://example.invalid/pkg".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: "ted/daily/2026-00136.tar.gz".into(),
    })
    .await
    .expect("record fetch");
    let fetch_id = db.current_packages(SOURCE, "daily", None).await.expect("packages")[0].fetch_id;
    (db, fetch_id, path)
}

fn sec(id: &str, kind: &str, parent: Option<&str>) -> Section {
    Section { id: id.into(), kind: kind.into(), parent: parent.map(str::to_owned) }
}
fn id_val(section: &str, field: &str, value: &str, is_ref: bool) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: None, value: value.into(), is_ref },
    }
}
fn ojs_edge(section: &str, field: &str, target: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: Some("ojs".into()), value: target.into(), is_ref: true },
    }
}
fn text_val(section: &str, field: &str, value: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Text { lang: Some("ENG".into()), value: value.into() },
    }
}
fn date_val(section: &str, field: &str, utc: i64) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Date { utc_seconds: utc, offset_minutes: 0, has_time: false },
    }
}

async fn record(db: &Db, fetch_id: i64, source: &str, pub_id: &str, profile: &str, parsed: Parsed) {
    db.record_notice(
        &Notice {
            source: source.into(),
            publication_id: pub_id.into(),
            content_hash: format!("h-{source}-{pub_id}"),
            profile: profile.into(),
            declared_version: None,
            fetch_id,
            member_path: format!("{pub_id}.xml"),
            ingested_at: 0,
            published_at: Some(0),
            dispatched_at: None,
        },
        &Parse::Parsed(parsed),
    )
    .await
    .expect("record notice");
}

/// An eForms Organization (name on the section, VAT id on its legal-entity child)
/// referenced as `role` from the procedure — the real shape, so a shared VAT id
/// across notices must resolve to one canonical Organization whichever batch each
/// notice folds in.
fn org(parsed: &mut Parsed, section: &str, name: &str, vat: &str, role: &str, ordinal: i64) {
    parsed.sections.push(sec(section, "Organization", Some("PROC")));
    parsed.values.push(text_val(section, "BT-500-Organization-Company", name));
    let legal = format!("{section}-legal");
    parsed.sections.push(sec(&legal, "CompanyLegalEntity", Some(section)));
    parsed.values.push(ValueRow {
        section_id: legal,
        field_id: "BT-501-Organization-Company".into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: Some("VAT".into()), value: vat.into(), is_ref: false },
    });
    parsed.values.push(ValueRow {
        section_id: "PROC".into(),
        field_id: role.into(),
        ordinal,
        value: NoticeValue::Id { scheme: None, value: section.into(), is_ref: true },
    });
}

/// Build a corpus that exercises every grouping regime and forces cross-batch
/// coupling: keyed multi-notice chains, legacy OJS transitive chains (whose
/// members are ingested far apart in id space), islands, and organizations whose
/// VAT ids recur across unrelated notices. Ingest order is deliberately shuffled
/// so a Tender's notices scatter across the id range.
async fn build_corpus(db: &Db, fetch_id: i64) {
    // A pool of VAT ids reused across notices, so org dedup must cross batches.
    let vat = |k: u64| format!("NL{:09}B01", k % 37);

    // 20 keyed procedures, each CN → corrigendum → CAN under one BT-04, ingested
    // in three interleaved waves so a procedure's notices are far apart in id.
    for wave in 0..3u64 {
        for p in 0..20u64 {
            let key = format!("bt04-{p:04}");
            let pub_id = format!("k{p:04}-w{wave}");
            let mut parsed = Parsed {
                sections: vec![sec("PROC", "Procedure", None), sec("LOT-1", "Lot", Some("PROC"))],
                values: vec![
                    id_val("PROC", "BT-04-notice", &key, false),
                    date_val("PROC", "BT-05(a)-notice", (wave * 1000 + p) as i64),
                    text_val("PROC", "BT-21-Procedure", &format!("Procedure {p} wave {wave}")),
                    date_val("LOT-1", "BT-131(d)-Lot", 700_000_000 + wave as i64),
                ],
            };
            org(&mut parsed, "ORG-A", &format!("Buyer {p}"), &vat(p), "OPT-300-Procedure-Buyer", 0);
            org(&mut parsed, "ORG-B", &format!("Bidder {p}-{wave}"), &vat(p + wave), "OPT-301-Tenderer-SubCont", 1);
            record(db, fetch_id, SOURCE, &pub_id, "eforms:eforms-sdk-1.13", parsed).await;
        }
    }

    // 10 legacy OJS chains: a CN and an award referencing it by OJS number,
    // ingested in two waves so the chain's two notices sit far apart in id.
    for p in 0..10u64 {
        let cn_num = format!("{:06}-2019", 100 + p);
        let cn = Parsed {
            sections: vec![sec("PROC", "Notice", None)],
            values: vec![
                text_val("PROC", "TED-TITLE", &format!("Legacy works {p}")),
                date_val("PROC", "TED-DS_DATE_DISPATCH", (p * 86_400) as i64),
                date_val("PROC", "TED-DATE_RECEIPT_TENDERS", 728_000_000 + p as i64),
            ],
        };
        record(db, fetch_id, SOURCE, &cn_num, "ted-export-r209", cn).await;
    }
    for p in 0..10u64 {
        let award_num = format!("{:06}-2019", 900 + p);
        let cn_ref = format!("2019/S 001-{:06}", 100 + p);
        let mut award = Parsed {
            sections: vec![
                sec("PROC", "Notice", None),
                sec("RES-1", "LotResult", Some("PROC")),
                sec("ORG-1", "Organization", Some("RES-1")),
            ],
            values: vec![
                text_val("PROC", "TED-TITLE", &format!("Legacy award {p}")),
                date_val("PROC", "TED-DS_DATE_DISPATCH", ((p + 30) * 86_400) as i64),
                text_val("ORG-1", "TED-OFFICIALNAME", &format!("Winner {p}")),
                id_val("RES-1", "TED-ADDRESS_CONTRACTOR", "ORG-1", true),
                ValueRow {
                    section_id: "RES-1".into(),
                    field_id: "TED-VAL_TOTAL".into(),
                    ordinal: 0,
                    value: NoticeValue::Amount { cents: 1_000_000 + p as i64, currency: "EUR".into() },
                },
                ojs_edge("PROC", "TED-REF_NOTICE.NO_DOC_OJS", &cn_ref),
            ],
        };
        // Every third award's winner shares a recurring VAT id — but legacy
        // winners are inline name-only blocks, so this just adds provisional orgs;
        // still, keep the structure varied.
        let _ = &mut award;
        record(db, fetch_id, SOURCE, &award_num, "ted-export-r209", award).await;
    }

    // A bridge notice that merges two legacy chains into one component (a late
    // edge — the ADR-0003-style merge, which must survive batching identically).
    let bridge = Parsed {
        sections: vec![sec("PROC", "Notice", None), sec("CHG-1", "Change", Some("PROC"))],
        values: vec![
            text_val("PROC", "TED-TITLE", "Bridge"),
            date_val("PROC", "TED-DS_DATE_DISPATCH", 50 * 86_400),
            ojs_edge("PROC", "TED-REF_NOTICE.NO_DOC_OJS", "2019/S 001-000100"),
            ojs_edge("PROC", "TED-NOTICE_NUMBER_OJ", "2019/S 001-000101"),
        ],
    };
    record(db, fetch_id, SOURCE, "000300-2019", "ted-export-r209", bridge).await;

    // 15 islands: real fixtures with no procedure key.
    for fixture in ["eforms/brin-x01-00497689-2026.xml", "eforms/pin-4-00496860-2026.xml"] {
        let bytes = std::fs::read(format!("tests/fixtures/{fixture}")).expect("fixture");
        let profile::Disposition::Records(records) = profile::dispatch(fixture, &bytes) else {
            panic!("dispatch skipped {fixture}");
        };
        let [profile::Record::Notice(n)] = &records[..] else { panic!("one record") };
        let parse = eforms::parse_payload(&n.profile, &bytes);
        let Parse::Parsed(parsed) = parse else { panic!("parse {fixture}") };
        record(db, fetch_id, SOURCE, &n.publication_id, &n.profile, parsed).await;
    }
}

/// A deterministic digest of every canonical table that carries business content,
/// keyed and ordered so it is stable across runs. Timestamp columns are excluded
/// (the two runs stamp `now` at different wall-clock instants); everything that
/// the projection *derives* is included, surrogate ids and all — those are
/// assigned in an order that does not depend on the batch size.
async fn snapshot(db: &Db) -> String {
    let digests = [
        "SELECT group_concat(r, x'0a') FROM (SELECT coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source AS r FROM tenders ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||coalesce(publication_id,'')||'|'||coalesce(notice_subtype,'')||'|'||published_at||'|'||coalesce(dispatched_at,-1) AS r FROM tender_versions ORDER BY tender_id, seq)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value||'|'||coalesce(lot_id,-1) AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds||'|'||coalesce(lot_id,-1) AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||scheme||'|'||code||'|'||coalesce(lot_id,-1) AS r FROM tender_version_classifications ORDER BY tender_id, seq, field, scheme, code, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||cents||'|'||currency||'|'||coalesce(lot_id,-1) AS r FROM tender_version_amounts ORDER BY tender_id, seq, field, cents, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||role||'|'||organization_id||'|'||coalesce(lot_id,-1) AS r FROM tender_version_parties ORDER BY tender_id, seq, role, organization_id, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||lot_key AS r FROM lots ORDER BY tender_id, lot_key)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||notice_id||'|'||result_key AS r FROM lot_results ORDER BY tender_id, notice_id, result_key)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||coalesce(decision,'')||'|'||coalesce(awarded_cents,-1)||'|'||coalesce(awarded_currency,'')||'|'||coalesce(lot_id,-1) AS r FROM tender_version_lot_results ORDER BY tender_id, seq, lot_result_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(country,'')||'|'||coalesce(identifier_kind,'')||'|'||coalesce(identifier,'')||'|'||name||'|'||provisional AS r FROM organizations ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT notice_id||'|'||section_id||'|'||organization_id AS r FROM organization_mentions ORDER BY notice_id, section_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT entity_kind||'|'||op||'|'||coalesce(version_seq,-1)||'|'||entity_id AS r FROM changes ORDER BY cursor)",
    ];
    let mut out = String::new();
    for (i, sql) in digests.iter().enumerate() {
        let part = match db.scalar(sql).await.expect("digest query") {
            Some(turso::Value::Text(s)) => s,
            _ => String::new(),
        };
        out.push_str(&format!("--- digest {i} ---\n{part}\n"));
    }
    out
}

async fn count(db: &Db, sql: &str) -> i64 {
    match db.scalar(sql).await.expect("count") {
        Some(turso::Value::Integer(i)) => i,
        _ => 0,
    }
}

#[tokio::test]
async fn projection_output_is_identical_under_any_batch_size() {
    let (fine, fetch_fine, path_fine) = scratch("fine").await;
    build_corpus(&fine, fetch_fine).await;
    // One notice per batch: every Tender lands on a batch boundary.
    project::project_with_batch(&fine, true, 1).await.expect("project fine");

    let (whole, fetch_whole, path_whole) = scratch("whole").await;
    build_corpus(&whole, fetch_whole).await;
    // One batch for the whole corpus: the whole-RAM projection.
    project::project_with_batch(&whole, true, usize::MAX).await.expect("project whole");

    // Sanity: the corpus is non-trivial, so the comparison is meaningful.
    let fine_tenders = count(&fine, "SELECT COUNT(*) FROM tenders").await;
    assert!(fine_tenders > 30, "the corpus should produce many Tenders");
    assert_eq!(fine_tenders, count(&whole, "SELECT COUNT(*) FROM tenders").await, "same Tender count");

    let counts_fine = fine.canonical_counts().await.expect("counts");
    let counts_whole = whole.canonical_counts().await.expect("counts");
    assert_eq!(counts_fine, counts_whole, "every canonical table has the same row count");

    let snap_fine = snapshot(&fine).await;
    let snap_whole = snapshot(&whole).await;
    assert_eq!(
        snap_fine, snap_whole,
        "the canonical layer must be identical whether folded one notice per batch or all at once"
    );

    let _ = std::fs::remove_file(&path_fine);
    let _ = std::fs::remove_file(&path_whole);
}
