//! Output-identity proof for the incremental projection (issue 58).
//!
//! The daily projection re-derives only the Tenders TOUCHED by notices parsed
//! since the last run. Correctness hinges on it producing EXACTLY what a full
//! non-rebuild projection over the whole corpus produces — untouched Tenders
//! left byte-identical, touched ones re-derived in full. Each test builds two
//! DBs with an identical established layer, applies the SAME delta, absorbs it
//! one way with a full non-rebuild projection and the other way incrementally,
//! and asserts the canonical layer is byte-identical (surrogate ids included —
//! the reconcile reuses ids by natural key, so they match).

use ingest::project;
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projincr-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
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
fn id_val(section: &str, field: &str, value: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: None, value: value.into(), is_ref: false },
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

/// An eForms Organization (name on the section, VAT id on its legal-entity child)
/// referenced as a role from the procedure.
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

async fn record(db: &Db, fetch_id: i64, pub_id: &str, parsed: Parsed) {
    db.record_notice(
        &Notice {
            source: SOURCE.into(),
            publication_id: pub_id.into(),
            content_hash: format!("h-{pub_id}"),
            profile: "eforms:eforms-sdk-1.13".into(),
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

/// A keyed notice under BT-04 `key`, published at `pub_at`, with a buyer org.
fn keyed(key: &str, pub_at: i64, title: &str) -> Parsed {
    let mut parsed = Parsed {
        sections: vec![sec("PROC", "Procedure", None), sec("LOT-1", "Lot", Some("PROC"))],
        values: vec![
            id_val("PROC", "BT-04-notice", key),
            date_val("PROC", "BT-05(a)-notice", pub_at),
            text_val("PROC", "BT-21-Procedure", title),
            date_val("LOT-1", "BT-131(d)-Lot", 700_000_000 + pub_at),
        ],
    };
    org(&mut parsed, "ORG-A", "Buyer One", "NL000000001B01", "OPT-300-Procedure-Buyer", 0);
    parsed
}

/// An island notice — no procedure key.
fn island(title: &str) -> Parsed {
    Parsed {
        sections: vec![sec("PROC", "Procedure", None)],
        values: vec![
            date_val("PROC", "BT-05(a)-notice", 5),
            text_val("PROC", "BT-21-Procedure", title),
        ],
    }
}

/// Build the SAME established corpus on `db`: a 2-notice keyed Tender, an
/// unrelated 1-notice keyed Tender, and an island — then project it fully.
async fn establish(db: &Db, fetch_id: i64) {
    record(db, fetch_id, "A-cn", keyed("bt04-0001", 1, "Alpha CN")).await;
    record(db, fetch_id, "A-corr", keyed("bt04-0001", 2, "Alpha corrigendum")).await;
    record(db, fetch_id, "B-cn", keyed("bt04-0002", 1, "Beta CN")).await;
    record(db, fetch_id, "ISL", island("Island one")).await;
    project::project(db, false).await.expect("establish projection");
}

/// A deterministic digest of every canonical table with business content, keyed
/// and ordered so it is stable across runs. Timestamps excluded; surrogate ids
/// included (the reconcile reuses them by natural key). Mirrors
/// `project_equivalence::snapshot`.
async fn snapshot(db: &Db) -> String {
    let digests = [
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source AS r FROM tenders ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||coalesce(publication_id,'')||'|'||published_at AS r FROM tender_versions ORDER BY tender_id, seq)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value||'|'||coalesce(lot_id,-1) AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds||'|'||coalesce(lot_id,-1) AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||role||'|'||organization_id||'|'||coalesce(lot_id,-1) AS r FROM tender_version_parties ORDER BY tender_id, seq, role, organization_id, lot_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||lot_key AS r FROM lots ORDER BY tender_id, lot_key)",
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(country,'')||'|'||coalesce(identifier_kind,'')||'|'||coalesce(identifier,'')||'|'||name||'|'||provisional AS r FROM organizations ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT notice_id||'|'||section_id||'|'||organization_id AS r FROM organization_mentions ORDER BY notice_id, section_id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT entity_kind||'|'||op||'|'||coalesce(version_seq,-1)||'|'||entity_id AS r FROM changes ORDER BY cursor)",
    ];
    let mut out = String::new();
    for (i, sql) in digests.iter().enumerate() {
        let part = match db.scalar(sql).await.expect("digest query") {
            Some(store::turso::Value::Text(s)) => s,
            _ => String::new(),
        };
        out.push_str(&format!("--- digest {i} ---\n{part}\n"));
    }
    out
}

async fn count(db: &Db, sql: &str) -> i64 {
    match db.scalar(sql).await.expect("count") {
        Some(store::turso::Value::Integer(i)) => i,
        _ => 0,
    }
}

/// A late award notice attaching to an existing keyed Tender is absorbed
/// identically by a full non-rebuild projection and an incremental one.
#[tokio::test]
async fn incremental_late_attach_to_keyed_tender_matches_full() {
    let (full, ff, pf) = scratch("full").await;
    let (incr, fi, pi) = scratch("incr").await;
    establish(&full, ff).await;
    establish(&incr, fi).await;

    // The established layers must already match.
    assert_eq!(snapshot(&full).await, snapshot(&incr).await, "established layers differ");
    // Everything is projected: the incremental change-set is empty.
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "nothing left to project");

    // Delta: a later notice under the SAME key bt04-0001 (a late attach).
    record(&full, ff, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;
    record(&incr, fi, "A-award", keyed("bt04-0001", 3, "Alpha award")).await;

    // Exactly one notice is now unprojected on the incremental DB.
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap(), vec![5], "one changed notice");

    // Absorb: full re-scan vs incremental.
    project::project(&full, false).await.expect("full absorb");
    let report = project::project_incremental(&incr).await.expect("incremental absorb");
    assert_eq!(report.notices, 1, "incremental touched exactly the delta");

    // Byte-identical canonical layer.
    assert_eq!(
        snapshot(&full).await,
        snapshot(&incr).await,
        "incremental late-attach must match a full non-rebuild projection"
    );
    // The Tender count did not grow (attach, not a new Tender), and the change-set
    // is drained.
    assert_eq!(count(&incr, "SELECT COUNT(*) FROM tenders").await, 3, "still three Tenders");
    assert_eq!(incr.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "change-set drained");

    for p in [pf, pi] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}
