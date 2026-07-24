//! Resume-from-plan salvage proof (issue 60). An interrupted rebuild that already
//! finished Phase-1 leaves a COMPLETE grouping plan on disk; a subsequent
//! `project(db, true)` must detect it, SKIP Phase-1, and re-run only grouping
//! (path-B union-find) + Phase-2 — producing a canonical layer BYTE-IDENTICAL to a
//! from-scratch rebuild. The corpus includes legacy OJS chains and a bridge notice
//! that merges two components, so path-B grouping is exercised under resume.

use ingest::{eforms, profile, project};
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projresume-{name}-{}.db", std::process::id());
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

async fn record(db: &Db, fetch_id: i64, pub_id: &str, profile: &str, parsed: Parsed) {
    db.record_notice(
        &Notice {
            source: SOURCE.into(),
            publication_id: pub_id.into(),
            content_hash: format!("h-{pub_id}"),
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

/// A corpus that exercises every grouping regime path-B must reproduce under
/// resume: keyed chains, legacy OJS chains ingested far apart, a bridge notice
/// merging two legacy components (the union-find merge), and islands.
async fn build_corpus(db: &Db, fetch_id: i64) {
    // 6 keyed procedures, CN + corrigendum interleaved so a procedure's notices
    // scatter across id space.
    for wave in 0..2u64 {
        for p in 0..6u64 {
            let key = format!("bt04-{p:04}");
            let pub_id = format!("k{p:04}-w{wave}");
            let parsed = Parsed {
                sections: vec![sec("PROC", "Procedure", None), sec("LOT-1", "Lot", Some("PROC"))],
                values: vec![
                    id_val("PROC", "BT-04-notice", &key),
                    date_val("PROC", "BT-05(a)-notice", (wave * 100 + p) as i64),
                    text_val("PROC", "BT-21-Procedure", &format!("Procedure {p} wave {wave}")),
                    date_val("LOT-1", "BT-131(d)-Lot", 700_000_000 + wave as i64),
                ],
            };
            record(db, fetch_id, &pub_id, "eforms:eforms-sdk-1.13", parsed).await;
        }
    }

    // Two legacy OJS chains: a CN and an award referencing it by OJS number.
    for p in 0..2u64 {
        let cn_num = format!("{:06}-2019", 100 + p);
        let cn = Parsed {
            sections: vec![sec("PROC", "Notice", None)],
            values: vec![
                text_val("PROC", "TED-TITLE", &format!("Legacy works {p}")),
                date_val("PROC", "TED-DS_DATE_DISPATCH", (p * 86_400) as i64),
            ],
        };
        record(db, fetch_id, &cn_num, "ted-export-r209", cn).await;
    }
    for p in 0..2u64 {
        let award_num = format!("{:06}-2019", 900 + p);
        let cn_ref = format!("2019/S 001-{:06}", 100 + p);
        let award = Parsed {
            sections: vec![sec("PROC", "Notice", None)],
            values: vec![
                text_val("PROC", "TED-TITLE", &format!("Legacy award {p}")),
                date_val("PROC", "TED-DS_DATE_DISPATCH", ((p + 30) * 86_400) as i64),
                ojs_edge("PROC", "TED-REF_NOTICE.NO_DOC_OJS", &cn_ref),
            ],
        };
        record(db, fetch_id, &award_num, "ted-export-r209", award).await;
    }
    // A bridge notice merging the two legacy chains into one component (the
    // union-find merge path-B must resolve to the minimum OJS key).
    let bridge = Parsed {
        sections: vec![sec("PROC", "Notice", None)],
        values: vec![
            text_val("PROC", "TED-TITLE", "Bridge"),
            date_val("PROC", "TED-DS_DATE_DISPATCH", 50 * 86_400),
            ojs_edge("PROC", "TED-REF_NOTICE.NO_DOC_OJS", "2019/S 001-000100"),
            ojs_edge("PROC", "TED-NOTICE_NUMBER_OJ", "2019/S 001-000101"),
        ],
    };
    record(db, fetch_id, "000300-2019", "ted-export-r209", bridge).await;

    // Islands: real fixtures with no procedure key.
    for fixture in ["eforms/brin-x01-00497689-2026.xml", "eforms/pin-4-00496860-2026.xml"] {
        let bytes = std::fs::read(format!("tests/fixtures/{fixture}")).expect("fixture");
        let profile::Disposition::Records(records) = profile::dispatch(fixture, &bytes) else {
            panic!("dispatch skipped {fixture}");
        };
        let [profile::Record::Notice(n)] = &records[..] else { panic!("one record") };
        let parse = eforms::parse_payload(&n.profile, &bytes);
        let Parse::Parsed(parsed) = parse else { panic!("parse {fixture}") };
        record(db, fetch_id, &n.publication_id, &n.profile, parsed).await;
    }
}

/// Deterministic digest of every canonical table with business content, keyed and
/// ordered so it is stable across runs. Mirrors `project_equivalence::snapshot`.
async fn snapshot(db: &Db) -> String {
    let digests = [
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source AS r FROM tenders ORDER BY id)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||coalesce(publication_id,'')||'|'||published_at AS r FROM tender_versions ORDER BY tender_id, seq)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds)",
        "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||lot_key AS r FROM lots ORDER BY tender_id, lot_key)",
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

async fn plan_notice_count(db: &Db) -> i64 {
    match db.scalar("SELECT COUNT(*) FROM plan_notice").await.expect("count") {
        Some(store::turso::Value::Integer(i)) => i,
        _ => 0,
    }
}

/// A rebuild that resumes from a complete on-disk plan produces the same canonical
/// layer as a from-scratch rebuild — byte-identical, surrogate ids included.
#[tokio::test]
async fn resume_from_complete_plan_matches_a_full_rebuild() {
    let (full, ff, pf) = scratch("full").await;
    let (resume, fr, pr) = scratch("resume").await;
    build_corpus(&full, ff).await;
    build_corpus(&resume, fr).await;

    // Baseline: a from-scratch full rebuild.
    project::project(&full, true).await.expect("full rebuild");

    // Interrupted rebuild: build the plan and STOP (leaves a complete plan on disk,
    // canonical not yet applied, org indexes stripped).
    project::project_plan_only(&resume).await.expect("plan only");
    assert!(resume.plan_is_complete().await.unwrap(), "plan-only leaves a complete plan");
    assert!(plan_notice_count(&resume).await > 0, "plan populated");

    // Restart with a rebuild → it detects the complete plan and RESUMES (skips
    // Phase-1), re-running grouping (path-B) + Phase-2.
    project::project(&resume, true).await.expect("resume rebuild");
    assert!(!resume.plan_is_complete().await.unwrap(), "the resume clears the plan when done");

    assert_eq!(
        snapshot(&full).await,
        snapshot(&resume).await,
        "a resumed rebuild must be byte-identical to a from-scratch rebuild"
    );

    for p in [pf, pr] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// A normal rebuild leaves NO resumable plan on disk — so a rebuild only ever
/// resumes a genuinely-interrupted run, never a completed one. (A fresh DB and a
/// just-completed rebuild both report plan-incomplete, the guard against a normal
/// rebuild being mistaken for a salvage.)
#[tokio::test]
async fn a_normal_rebuild_leaves_no_resumable_plan() {
    let (db, fid, path) = scratch("normal").await;
    build_corpus(&db, fid).await;
    assert!(!db.plan_is_complete().await.unwrap(), "fresh DB: no plan on disk");

    project::project(&db, true).await.expect("rebuild");
    // The run cleared its plan at the end, so a subsequent rebuild starts fresh
    // (from-scratch) rather than resuming — the plan-complete signal is false.
    assert!(!db.plan_is_complete().await.unwrap(), "plan cleared after a completed rebuild");
    assert_eq!(plan_notice_count(&db).await, 0, "no plan rows left behind");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
