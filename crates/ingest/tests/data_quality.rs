//! Issue 27 — the data-quality report end to end, over the real fixture corpus.
//!
//! `bin/data-quality` talks to a live `/v1/sql`, but the risky part is the SQL
//! itself: does it read the canonical schema correctly and assemble into the
//! right rates? So this drives the *identical* queries (`data_quality::queries`)
//! straight against a scratch [`Db`] projected from real notices — the same
//! path the processor takes — and checks the assembled [`Report`]. The HTTP
//! transport is the thin, separately-trusted half (mirrors `bin/verify`).

use ingest::data_quality::{self, Raw};
use ingest::{process, project};
use serde_json::{Value, json};
use store::{Db, Notice, Parse};

/// A scratch database with a fetch row to hang notices off.
async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-dq-{name}-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = Db::open(&path).await.expect("open scratch db");
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
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
    let fetch_id = db.current_packages("ted", "daily", None).await.expect("packages")[0].fetch_id;
    (db, fetch_id, path)
}

/// Run a fixture through the real dispatch + parse chain and store it under a
/// named Source, exactly as `process` would from an archived package.
async fn ingest_from(db: &Db, fetch_id: i64, source: &str, relative: &str) {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let ingest::profile::Disposition::Records(records) = ingest::profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped a fixture");
    };
    let [ingest::profile::Record::Notice(n)] = &records[..] else {
        panic!("{relative}: expected one notice record");
    };
    let parse = process::parse_payload(&n.profile, &bytes);
    assert!(matches!(parse, Parse::Parsed(_)), "{relative}: {parse:?}");
    let (published_at, dispatched_at) = match &parse {
        Parse::Parsed(parsed) => {
            let (p, d) = project::notice_instants(parsed);
            (Some(p), d)
        }
        _ => (None, None),
    };
    db.record_notice(
        &Notice {
            source: source.into(),
            publication_id: n.publication_id.clone(),
            content_hash: n.content_hash.clone(),
            profile: n.profile.clone(),
            declared_version: n.declared_version.clone(),
            fetch_id,
            member_path: n.member_path.clone(),
            ingested_at: 0,
            published_at,
            dispatched_at,
        },
        &parse,
    )
    .await
    .expect("record notice");
}

/// The text era needs its own ingest path: dispatch is keyed on the member NAME
/// (`EN_20050101_001_UTF8_ORG.ZIP!…`, not a fixture path), one member holds many
/// records, each record carries its own byte span, and the parser is
/// `text::parse_payload` rather than the profile-dispatched one (which returns
/// `Pending` for `text`).
async fn ingest_text(db: &Db, fetch_id: i64, fixture: &str, member_path: &str) {
    let path = format!("tests/fixtures/text/{fixture}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let ingest::profile::Disposition::Records(records) = ingest::profile::dispatch(member_path, &bytes)
    else {
        panic!("{fixture}: dispatch skipped a text-era member");
    };
    for record in records {
        let ingest::profile::Record::Notice(n) = record else { continue };
        let (start, end) = n.span.expect("text records carry their span");
        let parse = ingest::text::parse_payload(&n.member_path, &bytes[start..end]);
        assert!(matches!(parse, Parse::Parsed(_)), "{fixture}: {parse:?}");
        let (published_at, dispatched_at) = match &parse {
            Parse::Parsed(parsed) => {
                let (p, d) = project::notice_instants(parsed);
                (Some(p), d)
            }
            _ => (None, None),
        };
        db.record_notice(
            &Notice {
                source: "ted".into(),
                publication_id: n.publication_id.clone(),
                content_hash: n.content_hash.clone(),
                profile: n.profile.clone(),
                declared_version: n.declared_version.clone(),
                fetch_id,
                member_path: n.member_path.clone(),
                ingested_at: 0,
                published_at,
                dispatched_at,
            },
            &parse,
        )
        .await
        .expect("record notice");
    }
}

/// Run one query and return its rows as the JSON matrix the assembler expects —
/// the same shape `/v1/sql` hands the bin.
async fn rows(db: &Db, sql: &str) -> data_quality::Rows {
    let readers = db.readers(1).expect("readers");
    let reader = readers.get().await.expect("reader");
    let mut cursor = reader.query(sql, ()).await.expect("query");
    let width = cursor.column_names().len();
    let mut out = Vec::new();
    while let Some(row) = cursor.next().await.expect("row") {
        let mut cells = Vec::with_capacity(width);
        for i in 0..width {
            cells.push(cell(row.get_value(i).expect("cell")));
        }
        out.push(cells);
    }
    out
}

/// One turso cell as JSON — the same mapping `/v1/sql` applies.
fn cell(value: turso::Value) -> Value {
    match value {
        turso::Value::Null => Value::Null,
        turso::Value::Integer(i) => json!(i),
        turso::Value::Real(f) => json!(f),
        turso::Value::Text(s) => json!(s),
        turso::Value::Blob(b) => json!(b.iter().map(|x| format!("{x:02x}")).collect::<String>()),
    }
}

/// Build the report against the scratch DB by running every query the bin runs.
async fn measure(db: &Db, base_url: &str) -> data_quality::Report {
    let mut results = Vec::new();
    for (label, sql) in data_quality::queries() {
        results.push((label, Some(rows(db, &sql).await)));
    }
    let raw = Raw::from_labelled(results).expect("all result sets present");
    data_quality::assemble(base_url, &raw)
}

/// Issue 230: summing a windowed query across disjoint windows must equal the
/// unwindowed result. This is the property the whole windowed design rests on — if
/// it does not hold, a bounded measurement is a wrong measurement — and it is
/// checked against the real fixture corpus, with a window size of ONE so every
/// tender_id sits in its own window and the seams are maximally exercised.
#[tokio::test]
async fn windowed_sums_equal_the_unwindowed_result() {
    let (db, fetch_id, path) = scratch("windowed").await;
    // Same corpus the full-report test uses: several eras, so the per-profile sum
    // has more than one key to get wrong.
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ] {
        ingest_from(&db, fetch_id, "ted", fixture).await;
    }
    ingest_from(&db, fetch_id, "ted", "doe-ted-pair/ted-cn-00373130-2026.xml").await;
    ingest_from(&db, fetch_id, "doe", "doe-ted-pair/doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml").await;
    project::project(&db, false).await.expect("project");

    let max_id = match db.scalar("SELECT MAX(tender_id) FROM tender_versions").await.unwrap() {
        Some(turso::Value::Integer(i)) => i,
        other => panic!("no versions to window over: {other:?}"),
    };
    assert!(max_id > 1, "the fixture corpus must span several tenders");

    for wq in data_quality::windowed_queries() {
        // Unwindowed: the same query with a range that covers everything.
        let whole = rows(&db, &wq.sql(0, max_id)).await;
        // Windowed: one window per id, summed.
        let mut parts = Vec::new();
        for lo in 0..max_id {
            parts.push(rows(&db, &wq.sql(lo, lo + 1)).await);
        }
        let summed = data_quality::sum_profile_counts(&parts);
        let whole_sorted = data_quality::sum_profile_counts(&[whole]);
        assert_eq!(
            summed, whole_sorted,
            "{}: windowed sum must equal the whole-range result",
            wq.label
        );
        // And the measurement is not vacuous — the fixtures do carry versions.
        if wq.label == "versions" {
            let total: i64 = summed.iter().filter_map(|r| r.get(1).and_then(|v| v.as_i64())).sum();
            assert!(total > 0, "the denominator must count something");
        }
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// The eForms chain (CN → 2 corrigenda → CAN) plus the DÖE↔TED pair: enough to
/// exercise completeness, a materialised winner, results density and the merge.
#[tokio::test]
async fn measures_completeness_results_and_merge_over_real_fixtures() {
    let (db, fetch_id, path) = scratch("full").await;
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ] {
        ingest_from(&db, fetch_id, "ted", fixture).await;
    }
    // A procedure published on both Sources — the merge case (ADR-0003).
    ingest_from(&db, fetch_id, "ted", "doe-ted-pair/ted-cn-00373130-2026.xml").await;
    ingest_from(&db, fetch_id, "doe", "doe-ted-pair/doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml").await;
    project::project(&db, false).await.expect("project");

    let report = measure(&db, "scratch://full").await;

    // Every era that holds versions shows up once, title-complete (title is the
    // one field the projection fills for every notice type).
    assert!(!report.completeness.is_empty(), "some era measured");
    for row in &report.completeness {
        assert_eq!(row.present[0], row.versions, "{}: every version has a title", row.profile);
    }

    // The eForms chain's CAN materialised a winner — so its era's winner count
    // is non-zero (the report can see results the projection actually produced).
    let eforms = report
        .completeness
        .iter()
        .find(|r| data_quality::era_of(&r.profile) == "eForms EU")
        .expect("an eForms EU era row");
    assert!(eforms.present[5] > 0, "the CAN produced at least one named winner");

    // Results density: the CAN is an award notice by its own PUBLISHED subtype
    // (`OPP-070-notice = 29`, issue 235) and it materialised, so the eForms era's
    // density is 100% here — the exact metric that reads ~0% on the
    // pre-results-projection deploy (issue 15).
    let density = report
        .density
        .iter()
        .find(|r| data_quality::era_of(&r.profile) == "eForms EU")
        .expect("an eForms award-notice era");
    assert!(density.award_notices > 0, "the CAN is counted as an award notice");
    assert_eq!(density.with_results, density.award_notices, "every award notice materialised results");
    // The chain's three non-award notices (CN + 2 corrigenda, subtype 16) are NOT
    // in the denominator: a denominator that counted them would measure something
    // else entirely.
    assert_eq!(density.award_notices, 1, "one of the four chain notices is an award");
    // The complement of the text-era case: this CAN published a result block AND it
    // materialised, so nothing is explained away.
    assert_eq!(density.no_award_content, 0, "the eForms CAN published its results: {density:?}");
    // And every version's type was readable, so the rate covers its population.
    for row in &report.doc_types {
        assert_eq!(
            (row.unclassified, row.untyped),
            (0, 0),
            "{}: the fixtures' document types must all be classified",
            row.profile
        );
    }

    // The merge: exactly the one DÖE procedure, and it merged with its TED twin.
    assert_eq!(report.merge.doe_tenders, 1);
    assert_eq!(report.merge.merged, 1);

    // The rendered forms are non-empty and self-consistent.
    let text = data_quality::render_text(&report);
    assert!(text.contains("merged with TED: 1 (100.0%)"), "{text}");
    let value: Value = serde_json::from_str(&data_quality::render_json(&report)).expect("valid json");
    assert_eq!(value["ted_doe_merge"]["merged"], json!(1));

    let _ = std::fs::remove_file(&path);
}

/// A legacy (r2.0.9) contract-award notice materialises results too — proving
/// the density metric is era-agnostic (legacy award blocks synthesise the same
/// `LotResult` section kind that eForms CANs do).
#[tokio::test]
async fn legacy_award_notice_counts_toward_results_density() {
    let (db, fetch_id, path) = scratch("r209").await;
    ingest_from(&db, fetch_id, "ted", "r209/f03-000988-2019.xml").await;
    project::project(&db, false).await.expect("project");

    let report = measure(&db, "scratch://r209").await;
    let density = report
        .density
        .iter()
        .find(|r| data_quality::era_of(&r.profile) == "TED_EXPORT r2.0.9")
        .expect("an r2.0.9 award-notice era");
    // `TD_DOCUMENT_TYPE CODE="7"` — the era's own words for "Contract award",
    // read from the notice rather than from what the projection made of it.
    assert!(density.award_notices > 0, "the F03 is an award notice");
    assert_eq!(density.with_results, density.award_notices, "the legacy award materialised results");

    let _ = std::fs::remove_file(&path);
}

/// Issue 244's fix, end to end: the text era's 2005 CAN now materialises the winners
/// its body publishes, and section 3 counts it in BOTH halves.
///
/// This test used to assert the opposite, and the change is the point. The notice
/// publishes `TD: 7 - Contract award`, and under the old parser nothing came of it:
/// the era publishes no result SECTION, so the projection had nothing to fold and
/// section 3 read 1 award notice, 0 materialised. Issue 235 made that visible (the
/// older definition could not see the notice at all — no result section parsed meant
/// no denominator either), and issue 244 then found WHY: the body carries
/// `V.1.1) Name and address of successful supplier, contractor or service provider:`
/// twice — Grahams Engineering Ltd. and NSG Environmental Ltd. — which the text
/// parser now reads into a `LotResult` per award.
///
/// So the fixture has moved from being the report's evidence of a gap to being the
/// fix's evidence of a repair, and the assertions move with it. The zero-not-absent
/// distinction it used to carry is covered by
/// [`an_award_notice_whose_body_has_no_award_block_reads_zero_not_absent`].
#[tokio::test]
async fn the_2005_text_era_can_materialises_the_winners_its_body_publishes() {
    let (db, fetch_id, path) = scratch("text-can").await;
    ingest_text(
        &db,
        fetch_id,
        "2005-can-154-2005.txt",
        "EN_20050101_001_UTF8_ORG.ZIP!EN_20050101_2005001_UTF8_ORG",
    )
    .await;
    project::project(&db, false).await.expect("project");

    let report = measure(&db, "scratch://text-can").await;
    let density = report
        .density
        .iter()
        .find(|r| r.profile == "text")
        .expect("the text era appears in section 3 on the strength of its own doc type");
    assert_eq!(density.award_notices, 1, "`TD: 7` is an award notice: {density:?}");
    assert_eq!(density.with_results, 1, "and it now materialises: {density:?}");
    // And the barren column empties out, because a result block IS parsed now — the
    // same row that reported the gap reports the repair (issue 242's column, issue
    // 244's fix).
    assert_eq!(density.no_award_content, 0, "a result block is parsed: {density:?}");

    // Both winners, one result each, under the era's own label for the value.
    let results = rows(&db, "SELECT COUNT(*) FROM notice_sections WHERE kind = 'LotResult'").await;
    assert_eq!(results[0][0], serde_json::json!(2), "the body awards two contracts");
    // And they reach the CANONICAL layer as winners, which is the fix's whole point:
    // a name in the parse layer that no projection reads would be invisible.
    let winners = rows(
        &db,
        "SELECT o.name FROM tender_version_parties p JOIN organizations o ON o.id = p.organization_id \
          WHERE p.role = 'winner' ORDER BY o.name",
    )
    .await;
    let winners: Vec<&str> = winners.iter().map(|r| r[0].as_str().expect("a name")).collect();
    assert_eq!(
        winners,
        vec!["Grahams Engineering Ltd", "NSG Environmental Ltd"],
        "both winners are organizations with the winner role"
    );

    // Section 3b gains a denominator for this era, where it had none: the invariant
    // asks "did the fold write what the parse produced", and until issue 244 the text
    // era parsed no result section for it to ask about. Now it does, and the answer is
    // yes — which is the pair of questions section 3 and 3b are meant to answer
    // separately (issue 235).
    let invariant = report
        .invariant
        .iter()
        .find(|r| r.profile == "text")
        .expect("the era now parses result sections");
    assert_eq!(
        (invariant.with_sections, invariant.with_rows),
        (1, 1),
        "parsed one result-bearing notice and folded it: {invariant:?}"
    );

    // And it renders as a rate, not as a dash or a missing row.
    let text = data_quality::render_text(&report);
    let line = text
        .lines()
        .find(|l| l.contains("text 1993") && l.contains("100.0%"))
        .expect("the text era's density is rendered");
    assert!(line.contains(" 1 "), "one award notice, one materialised: {line}");

    let _ = std::fs::remove_file(&path);
}

/// Issue 29 regression, measured through the report itself: the DÖE sdk-0.1 era
/// went from 0 % on every field to real completeness once its `SDK01-*` stems
/// were mapped. This is the tool observing its own motivating anomaly get fixed.
#[tokio::test]
async fn sdk01_era_completeness_is_no_longer_zero() {
    let (db, fetch_id, path) = scratch("sdk01dq").await;
    ingest_from(&db, fetch_id, "doe", "doe/sdk-0.1-numeric-cn-25599482-1.xml").await;
    ingest_from(&db, fetch_id, "doe", "doe/sdk-0.1-uuid-can-427d4645-163c-419d-93a9-5f5ce05ff9b7-1.xml").await;
    project::project(&db, false).await.expect("project");

    let report = measure(&db, "scratch://sdk01").await;
    let _ = std::fs::remove_file(&path);
    let sdk01 = report
        .completeness
        .iter()
        .find(|r| data_quality::era_of(&r.profile) == "DÖE sdk-0.1 island")
        .expect("a DÖE sdk-0.1 era row");
    // title, buyer and winner all now present (were 0 before issue 29).
    assert!(sdk01.present[0] > 0, "title");
    assert!(sdk01.present[1] > 0, "buyer");
    assert!(sdk01.present[5] > 0, "winner");
}
