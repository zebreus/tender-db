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
        Parse::Parsed(parsed) => project::notice_stamps(parsed),
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
            Parse::Parsed(parsed) => project::notice_stamps(parsed),
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


/// Issue 109's gate, driven the way the incident ran: strip one era's versions
/// bare — satellite rows deleted, `tender_versions` rows SURVIVING, which is
/// exactly why G2 and every other row-counting gate stayed green through
/// 218,635 eForms-DE 1.x shells — and the presence probe must read that era
/// 100 % factless while the untouched eras stay at zero. The version count
/// itself must NOT move, restating the blindness this measure exists to cover.
#[tokio::test]
async fn a_stripped_cohort_reads_factless_while_every_row_count_stays_green() {
    let (db, fetch_id, path) = scratch("factless").await;
    for fixture in
        ["eforms-chain/1-cn-16-831374-2025.xml", "eforms-chain/4-can-29-380868-2026.xml"]
    {
        ingest_from(&db, fetch_id, "ted", fixture).await;
    }
    project::project(&db, false).await.expect("project");

    let before = measure(&db, "http://x").await;
    assert!(!before.presence.is_empty(), "the corpus must have presence rows");
    assert!(
        before.presence.iter().all(|r| r.factless == 0),
        "real fixtures carry facts: {:?}",
        before.presence
    );

    // Strip the first era bare — by (tender_id, seq) of ITS versions only, since
    // a chained tender carries versions from several eras.
    let victim = before.presence[0].profile.clone();
    for table in [
        "tender_version_texts",
        "tender_version_classifications",
        "tender_version_dates",
        "tender_version_parties",
        "tender_version_amounts",
        "tender_version_lots",
    ] {
        db.execute_for_test(&format!(
            "DELETE FROM {table} WHERE EXISTS (SELECT 1 FROM tender_versions v \
               JOIN notices n ON n.id = v.caused_by_notice_id \
              WHERE n.profile = '{victim}' AND v.tender_id = {table}.tender_id \
                AND v.seq = {table}.seq)"
        ))
        .await
        .expect("strip");
    }

    let after = measure(&db, "http://x").await;
    let row = after.presence.iter().find(|r| r.profile == victim).expect("era still present");
    assert!(row.versions > 0);
    assert_eq!(row.factless, row.versions, "every stripped version must read factless");
    assert_eq!(
        row.versions, before.presence[0].versions,
        "the version COUNT must not move — that blindness is what this probe covers"
    );
    for r in &after.presence {
        if r.profile != victim {
            assert_eq!(r.factless, 0, "untouched eras must stay clean: {r:?}");
        }
    }
    let text = data_quality::render_text(&after);
    assert!(text.contains("== 7. Content presence"), "the section must render:\n{text}");

    // The cohort floor: fixture-sized eras stay under the alarm's ≥1,000-version
    // bar, so even a wholesale strip is quiet HERE — the alarm's own unit test
    // covers the firing shapes at scale.
    let prev: Vec<(String, u64, u64)> =
        before.presence.iter().map(|r| (r.profile.clone(), r.versions, r.factless)).collect();
    assert!(data_quality::presence_step_changes(&prev, &after.presence).is_empty());

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 402 unit B: the continuity window must be PUBLICATION time, because an
/// ingest-ordered window cannot see the hole this section exists for.
///
/// The shape is issue 402's exactly, in miniature. The corpus was built in two
/// sittings: the monthly packages (publication through 2026-06-29) were fetched
/// first and so hold the LOW notice ids, and the daily packages (publication from
/// 2026-07-16) were fetched after and hold the HIGH ones. A window over the newest
/// N ids therefore contains only the dailies, so the oldest publication day IN THE
/// WINDOW is 2026-07-16 — the day the hole ends. `publication_gaps` reports
/// interior silences only (a stretch lies between two days that both carry
/// notices, and the caption says so), so with no earlier day inside the window
/// there was no `before` bracket, no stretch, and a 12-day blackout rendered as
/// "none". Neither the 4-day threshold nor the ordinary-bracket floor could have
/// helped: the rows were not in the result set to be judged.
///
/// This test asserts both halves — that the time window SEES the stretch, and
/// that the id window it replaced does NOT. The counterfactual is executed rather
/// than described, so the change is provably load-bearing.
#[tokio::test]
async fn the_continuity_window_is_publication_time_not_ingest_order() {
    let (db, fetch_id, _path) = scratch("pubwindow").await;
    let now = 1_789_600_000_i64;
    let midnight = now / 86_400 * 86_400;
    let day = |back: i64| midnight - back * 86_400;

    // Ingest order is the variable under test, so it is explicit: the "monthly"
    // block is recorded FIRST and takes the low ids, exactly as the real corpus
    // was built. Four notices a day so the median day is 4 and the ordinary
    // bracket floor (50%) is 2 — every day here is an ordinary one.
    //
    // The third block is what makes this test discriminate at fixture scale. A
    // deep backfill ingests OLD publications LAST, so it holds the newest ids of
    // all — and `MAX(id) - 2_000_000` is negative on a 24-row fixture, meaning the
    // id window this replaced would admit every row here and the forward half
    // would pass under either implementation. Publishing this block 600 days back,
    // outside the 550-day window, is what separates them: the time window excludes
    // it (5 days, one stretch) and any id window includes it (6 days, two).
    let mut seq = 0;
    for (label, backs) in [
        ("monthly", vec![40_i64, 39]),
        ("daily", vec![10_i64, 9, 8]),
        ("ancient-backfill", vec![600_i64]),
    ] {
        for back in backs {
            for _ in 0..4 {
                seq += 1;
                db.record_notice(
                    &Notice {
                        source: "ted".into(),
                        publication_id: format!("{label}-{seq:05}"),
                        content_hash: format!("hash{seq:05}"),
                        profile: "eforms-sdk-1.6".into(),
                        declared_version: None,
                        fetch_id,
                        member_path: format!("{label}/{seq:05}.xml"),
                        ingested_at: 0,
                        published_at: Some(store::Stamp::utc(day(back))),
                        dispatched_at: None,
                    },
                    &Parse::Pending,
                )
                .await
                .expect("record notice");
            }
        }
    }

    // The window under test, evaluated at a fixed instant so the fixture days sit
    // where a real corpus would put them and the test cannot rot as time passes.
    let days = rows(&db, &data_quality::publication_days_sql_at(now)).await;
    let as_pairs = |rs: &data_quality::Rows| -> Vec<(String, i64)> {
        rs.iter()
            .map(|r| (r[1].as_str().expect("day").to_owned(), r[2].as_i64().expect("count")))
            .collect()
    };
    let pairs = as_pairs(&days);
    assert_eq!(
        pairs.len(),
        5,
        "the five days inside the publication window, and NOT the 600-day-old backfill \
         whose ids are the newest in the corpus: {pairs:?}"
    );
    assert_eq!(pairs[0].1, 4, "and each carries its four notices: {pairs:?}");

    let gaps = data_quality::publication_gaps(&pairs, data_quality::PUBLICATION_GAP_MIN_DAYS);
    assert_eq!(
        gaps.len(),
        1,
        "one stretch, between the monthly and daily blocks — the ancient backfill is out \
         of the window, so it does not add a second, 559-day one: {gaps:?}"
    );
    assert_eq!(gaps[0].2, 28, "day-39 to day-10 is 28 silent days: {gaps:?}");
    assert_eq!((gaps[0].3, gaps[0].4), (4, 4), "both brackets are ordinary days: {gaps:?}");

    // The counterfactual: the id window this replaced. Twelve ids is exactly the
    // "daily" block, so the "monthly" block falls outside it — which is the real
    // corpus's situation, at the real corpus's scale.
    let id_windowed = rows(
        &db,
        "SELECT n.source AS source, \
                strftime('%Y-%m-%d', n.published_at, 'unixepoch') AS day, \
                COUNT(*) AS notices \
           FROM notices n \
          WHERE n.id > (SELECT MAX(id) FROM notices) - 16 \
            AND n.id <= (SELECT MAX(id) FROM notices) - 4 \
            AND n.published_at IS NOT NULL \
          GROUP BY n.source, day ORDER BY n.source, day",
    )
    .await;
    let blind = as_pairs(&id_windowed);
    assert_eq!(blind.len(), 3, "the id window holds only the last-ingested block: {blind:?}");
    assert!(
        data_quality::publication_gaps(&blind, data_quality::PUBLICATION_GAP_MIN_DAYS).is_empty(),
        "THIS is the defect: with the earlier block outside the window there is no `before` \
         bracket, so the 28-day blackout reports as no gap at all — {blind:?}"
    );
}


/// Issue 243: the result-section probe must be two point seeks on
/// `notice_sections(kind, notice_id)`, never a walk of the notice's sections by
/// the primary key's `notice_id` prefix. An eForms notice carries ~57 sections
/// per version, and that walk made `sections_can` (1,173 s) and `awards`
/// (1,880 s) the two most expensive queries of the 6,057 s weekly run on
/// 2026-09-20. turso's own planner is the authority on which shape it picks,
/// so the plan is pinned here the way the store pins its re-queue seeks
/// (`the_requeue_statements_seek_notices_by_rowid`).
#[tokio::test]
async fn the_result_section_probe_seeks_kind_then_notice() {
    let (db, _fetch_id, path) = scratch("plans243").await;
    db.ensure_unprojected_index().await.expect("prod's partial index");
    let queries = data_quality::windowed_queries();
    let probed: Vec<_> =
        queries.iter().filter(|q| q.label == "sections_can" || q.label == "awards").collect();
    assert_eq!(probed.len(), 2, "both carriers of the probe are windowed");
    for q in probed {
        let sql = q.sql(0, 250_001);
        let rows = db.measure_rows(&format!("EXPLAIN QUERY PLAN {sql}")).await.expect("plan");
        let plan = rows.iter().map(|r| format!("{r:?}")).collect::<Vec<_>>().join("\n");
        assert!(
            plan.contains("notice_sections_kind_notice (kind=? AND notice_id=?"),
            "{}: the result-section probe must seek (kind, notice_id); plan:\n{plan}",
            q.label
        );
        assert!(
            !plan.contains("sqlite_autoindex_notice_sections_1 (notice_id=?"),
            "{}: the probe walks the notice's sections by the primary-key prefix again \
             (issue 243's 10× windows); plan:\n{plan}",
            q.label
        );
    }
    let _ = std::fs::remove_file(&path);
}

/// Issue 386 unit 1: the FTS weld gauge counts versions that DISAGREE on the
/// buyer, not Tenders with several buyers. After the 2026-09-27 FTS re-fold the
/// first form read 949, of which 946 were frameworks naming the same 18 or 140
/// buyers in every version — joint procurements the per-buyer split rightly keeps
/// together — so its "expected 0" could never be met. Driven on hand-built rows
/// (foreign keys off: the gauge reads `tenders` and `tender_version_parties` only),
/// with the old count executed beside it so the difference is on record.
#[tokio::test]
async fn the_fts_weld_gauge_counts_disagreeing_versions_not_joint_procurements() {
    let (db, _, path) = scratch("weld-fts").await;
    db.set_foreign_keys(false).await.expect("fk off");
    // (tender id, source, [(seq, organization id)]) — role alternates between the
    // two buyer vocabularies, which the gauge must treat alike.
    let tenders: [(i64, &str, &[(i64, i64)]); 5] = [
        // Joint procurement: the same two buyers in both versions. NOT a weld.
        (1, "fts", &[(1, 11), (1, 12), (2, 11), (2, 12)]),
        // A weld: one buyer per version, a different one each time.
        (2, "fts", &[(1, 21), (2, 22)]),
        // A later version ADDS a buyer: its widest version holds the union. NOT a weld.
        (3, "fts", &[(1, 31), (1, 32), (2, 31), (2, 32), (2, 33)]),
        // The same disagreement on TED is out of scope for the FTS arm.
        (4, "ted", &[(1, 41), (2, 42)]),
        // One version, one buyer.
        (5, "fts", &[(1, 51)]),
    ];
    for (id, source, parties) in tenders {
        db.execute_for_test(&format!(
            "INSERT INTO tenders (id, source, kind, created_at) VALUES ({id}, '{source}', 'procedure', 0)"
        ))
        .await
        .expect("tender");
        for (i, (seq, org)) in parties.iter().enumerate() {
            let role = if i % 2 == 0 { "buyer" } else { "Procedure-Buyer" };
            db.execute_for_test(&format!(
                "INSERT INTO tender_version_parties \
                   (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) \
                 VALUES ({id}, {seq}, NULL, '{role}', {org}, 0, 's{id}-{i}')"
            ))
            .await
            .expect("party");
        }
    }
    // A review body alone on tender 5's second version: to a role-blind count that
    // is a version disagreeing with the first (buyer 51 vs org 99) — a false weld.
    db.execute_for_test(
        "INSERT INTO tender_version_parties \
           (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id) \
         VALUES (5, 2, NULL, 'Lot-ReviewOrg', 99, 0, 'review')",
    )
    .await
    .expect("reviewer");

    let count = |rows: data_quality::Rows| rows[0][0].as_i64().expect("a count");
    assert_eq!(
        count(rows(&db, &data_quality::weld_fts_sql()).await),
        1,
        "only tender 2 — its versions disagree on the buyer"
    );
    // The first form, for the record: it also counted the joint procurement and the
    // amendment, which is how 946 frameworks filled a line that expects zero.
    let naive = "SELECT COUNT(*) FROM (SELECT p.tender_id \
       FROM tenders t JOIN tender_version_parties p ON p.tender_id = t.id \
      WHERE t.source = 'fts' AND p.role IN ('buyer', 'Procedure-Buyer') \
      GROUP BY p.tender_id HAVING COUNT(DISTINCT p.organization_id) >= 2)";
    assert_eq!(count(rows(&db, naive).await), 3, "the old gauge's reading on the same rows");
    let _ = std::fs::remove_file(&path);
}

/// Issue 394 (b): the identity-string recurrence detector finds a placeholder
/// cohort through the real SQL and names nothing else. Twelve DÖE notices elected
/// `00000000-1900` (the shape the parser produced for 7,158 before the guard);
/// an honest id re-issued once carries two contents and must stay off the list.
#[tokio::test]
async fn the_repeated_id_listing_names_a_placeholder_cohort_and_nothing_else() {
    let (db, fetch_id, path) = scratch("repeated-ids").await;
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");
    let insert = |key: &str, i: usize| {
        format!(
            "INSERT INTO notices (source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
             VALUES ('doe', '{key}', 'h-{key}-{i}', 'eforms', {fetch_id}, 'm-{key}-{i}.xml', 0)"
        )
    };
    for i in 0..12 {
        conn.execute(&insert("00000000-1900", i), ()).await.expect("placeholder");
    }
    for i in 0..2 {
        conn.execute(&insert("a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01", i), ()).await.expect("re-issue");
    }

    let listed = rows(&db, &data_quality::repeated_ids_sql()).await;
    assert_eq!(listed, vec![vec![json!("doe"), json!("00000000-1900"), json!(12)]]);

    let report = measure(&db, "http://x").await;
    assert_eq!(report.repeated_ids.len(), 1);
    assert_eq!(report.repeated_ids[0].notices, 12);
    let text = data_quality::render_text(&report);
    assert!(text.contains("00000000-1900"), "{text}");
    assert!(!text.contains("fcd05fc6fd5c"), "an ordinary re-issue is not listed: {text}");

    drop(conn);
    let _ = std::fs::remove_file(&path);
}

/// Issue 394 (c): the CPV spelling census counts each shape through the real SQL,
/// leaves bare codes and other schemes alone, and reads ONLY the `(scheme, code)`
/// index — a covering scan, so the weekly pass is one index walk with no table fetch
/// per row. The plan is turso's to choose, so it is pinned here (issue 243's rule).
#[tokio::test]
async fn the_cpv_shape_census_counts_each_shape_and_reads_only_its_index() {
    let (db, _fetch_id, path) = scratch("cpv-shapes").await;
    db.build_tender_indexes().await.expect("tender indexes");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");
    let seeded = [
        ("cpv", "45000000"),
        ("cpv", "09123000"),
        ("cpv", "45421146-9"),
        ("cpv", "09123000-7"),
        ("cpv", "45324000-4  45421146-9"),
        ("cpv", "50"),
        ("cpv", "4542"),
        ("cpv", "45.42"),
        ("cpv", "123456789"),
        ("nuts", "DE300"),
        ("nuts", "DE3"),
    ];
    for (i, (scheme, code)) in seeded.iter().enumerate() {
        conn.execute(
            &format!(
                "INSERT INTO tender_version_classifications (tender_id, seq, field, scheme, code) \
                 VALUES ({}, 1, 'main', '{scheme}', '{code}')",
                i + 1
            ),
            (),
        )
        .await
        .expect("seed");
    }

    let mut listed = rows(&db, data_quality::CPV_SHAPES_SQL).await;
    listed.sort_by(|a, b| a[0].as_str().cmp(&b[0].as_str()));
    assert_eq!(
        listed,
        vec![
            vec![json!("check-digit"), json!(2), json!("09123000-7")],
            vec![json!("division"), json!(2), json!("4542")],
            vec![json!("glued"), json!(1), json!("45324000-4  45421146-9")],
            vec![json!("other"), json!(2), json!("123456789")],
        ],
        "the two bare codes and both NUTS rows are not listed"
    );

    let plan = db
        .measure_rows(&format!("EXPLAIN QUERY PLAN {}", data_quality::CPV_SHAPES_SQL))
        .await
        .expect("plan");
    let plan = plan.iter().map(|r| format!("{r:?}")).collect::<Vec<_>>().join("\n");
    let code = db
        .measure_rows(&format!("EXPLAIN {}", data_quality::CPV_SHAPES_SQL))
        .await
        .expect("bytecode");
    assert!(
        plan.contains("USING INDEX tender_version_classifications_code (scheme=?)"),
        "the census must seek the cpv range of the (scheme, code) index; plan:\n{plan}"
    );
    // turso's plan text never says COVERING for a SEARCH, so the bytecode is the
    // witness: one read cursor, on the index, and no seek into the table by rowid.
    let opcodes: Vec<String> =
        code.iter().filter_map(|r| r.get(1).and_then(|v| v.as_text()).cloned()).collect();
    let code = code.iter().map(|r| format!("{r:?}")).collect::<Vec<_>>().join("\n");
    assert_eq!(opcodes.iter().filter(|o| *o == "OpenRead").count(), 1, "one read cursor:\n{code}");
    assert!(code.contains("index=tender_version_classifications_code"), "{code}");
    for seek in ["SeekRowid", "DeferredSeek", "IdxRowId"] {
        assert!(!opcodes.iter().any(|o| o == seek), "a table fetch ({seek}) per row:\n{code}");
    }

    let report = measure(&db, "http://x").await;
    assert_eq!(report.cpv_shapes.len(), 4, "{:?}", report.cpv_shapes);
    let text = data_quality::render_text(&report);
    assert!(text.contains("== 15. CPV spellings") && text.contains("45324000-4  45421146-9"), "{text}");

    drop(conn);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 471 unit 1 (366 unit 6), pinned by the name the issue gives it: a head figure
/// that occurs ONCE in the corpus reaches the band listing (section 16) and does not
/// reach section 10, which keeps only values repeating [`SENTINEL_MIN_REPEATS`] times.
///
/// The same scratch corpus pins the rest of the unit through the real SQL:
/// - the THRESHOLD: €9,999,999,999.99 is out, exactly €10 bn is in, a NULL head is out;
/// - the GROUPING: rows come back under their published currency, one per Tender even
///   when the elected figure is published at tender and lot scope alike;
/// - the SIGNALS: the exact-10³ lot estimate, the smallest sibling, and the F14 value
///   corrigendum earlier in the chain (`II.1.7 )`: the trailing `)` with a space before
///   it, normalised the way `project::f14_coordinate` does); a 1.00 placeholder sibling
///   is neither a 10ᵏ partner nor the smallest sibling (issue 380's ceiling);
/// - ONE ROW PER TENDER from the SQL itself (so the LIMIT counts Tenders), the tie
///   between two head rows sharing the elected `eur_cents` broken the read layer's way
///   (`cents DESC, currency`), and a STALE head (no amount row at the head version
///   matches the head column) listed with no published figure rather than dropped;
/// - the ORDER: top of the band first, so a capped listing cuts the lowest heads;
/// - the BOUND: the plan drives from a range seek on `tenders_current_value_eur`, never
///   a scan of `tenders`, every satellite read is a seek (issue 243's rule), and there
///   is no temp B-tree (no sorter) — on an EMPTY `sqlite_stat1` and again after
///   `store::ANALYZE_TABLES` are analyzed (issue 429's job, issue 428's finding that
///   turso plans with the stats). The second check is fixture statistics, not prod's:
///   a plan-probe on an analyzed prod snapshot is still owed before 429's schedule.
#[tokio::test]
async fn the_band_listing_shows_a_head_that_repeats_nowhere() {
    let (db, fetch_id, path) = scratch("band").await;
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");
    let exec = |sql: String| {
        let conn = &conn;
        async move { conn.execute(&sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}")) }
    };
    // Notices 1..=7, one per (tender, version) below.
    let notices = [
        ("071343-2017", "ted-export-r209"),  // 1: tender 1's F14 value corrigendum
        ("123456-2017", "ted-export-r209"),  // 2: tender 1's award, the £9 bn head
        ("00447172-2025", "eforms:eforms-sdk-1.13"), // 3: tender 2, 20905's class
        ("200003-2020", "ted-export-r208"),  // 4: tender 3, just under the floor
        ("200004-2020", "ted-export-r209"),  // 5: tender 4, exactly on the floor
        ("200005-2020", "ted-export-r209"),  // 6: tender 5, the repeated sentinel
        ("200007-2020", "ted-export-r209"),  // 7: tender 7, SEK
        ("200008-2020", "ted-export-r209"),  // 8: tender 8, a tie on the elected eur_cents
        ("200009-2020", "ted-export-r209"),  // 9: tender 9, a stale head column
    ];
    for (i, (pubid, profile)) in notices.iter().enumerate() {
        exec(format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at) \
             VALUES ({}, 'ted', '{pubid}', 'h{i}', '{profile}', {fetch_id}, 'm{i}.xml', 0)",
            i + 1
        ))
        .await;
    }
    // (tender, current_seq, head eur_cents)
    let tenders: [(i64, i64, Option<i64>); 8] = [
        (1, 2, Some(1_040_000_000_000)),
        (2, 1, Some(3_326_050_000_000)),
        (3, 1, Some(999_999_999_999)),
        (4, 1, Some(1_000_000_000_000)),
        (5, 1, None),
        (7, 1, Some(4_400_000_000_000)),
        (8, 1, Some(2_000_000_000_000)),
        (9, 1, Some(1_200_000_000_000)),
    ];
    for (id, seq, eur) in tenders {
        let eur = eur.map_or("NULL".to_owned(), |e| e.to_string());
        exec(format!(
            "INSERT INTO tenders (id, source, kind, created_at, current_seq, current_value_eur_cents) \
             VALUES ({id}, 'ted', 'procedure', 0, {seq}, {eur})"
        ))
        .await;
    }
    // (tender, seq, notice)
    for (tender, seq, notice) in [(1, 1, 1), (1, 2, 2), (2, 1, 3), (3, 1, 4), (4, 1, 5), (5, 1, 6), (7, 1, 7), (8, 1, 8), (9, 1, 9)] {
        exec(format!(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id) \
             VALUES ({tender}, {seq}, {notice}, 0, '{}')",
            notices[notice as usize - 1].0
        ))
        .await;
    }
    let amount = |tender: i64, seq: i64, lot: &str, cents: i64, cur: &str, eur: Option<i64>| {
        let eur = eur.map_or("NULL".to_owned(), |e| e.to_string());
        format!(
            "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency, eur_cents) \
             VALUES ({tender}, {seq}, {lot}, 'result_value', {cents}, '{cur}', {eur})"
        )
    };
    // Tender 1: the £9 bn head over a £9 M lot estimate (10³), plus a £45 M sibling.
    exec(amount(1, 2, "NULL", 900_000_000_000, "GBP", Some(1_040_000_000_000))).await;
    exec(amount(1, 2, "1", 900_000_000, "GBP", Some(1_040_000_000))).await;
    exec(amount(1, 1, "NULL", 4_500_000_000, "GBP", Some(5_200_000_000))).await;
    // Tender 2: one figure, published in two slots, nowhere else in the corpus.
    exec(amount(2, 1, "NULL", 3_326_050_000_000, "EUR", Some(3_326_050_000_000))).await;
    exec(amount(2, 1, "1", 3_326_050_000_000, "EUR", Some(3_326_050_000_000))).await;
    // Tender 3: a cent under the floor. Tender 4: exactly on it.
    exec(amount(3, 1, "NULL", 999_999_999_999, "EUR", Some(999_999_999_999))).await;
    exec(amount(4, 1, "NULL", 1_000_000_000_000, "EUR", Some(1_000_000_000_000))).await;
    // ... beside a EUR 1.00 placeholder exactly 10¹² below it: NOT a partner (issue 380).
    exec(amount(4, 1, "1", 100, "EUR", Some(100))).await;
    // Tender 8: two head rows share the elected eur_cents. The read layer serves
    // `cents DESC, currency` first — the DKK row — and so must the listing. Inserted
    // EUR first so "keep the first joined row" would pick the wrong one.
    exec(amount(8, 1, "NULL", 2_000_000_000_000, "EUR", Some(2_000_000_000_000))).await;
    exec(amount(8, 1, "1", 14_920_000_000_000, "DKK", Some(2_000_000_000_000))).await;
    // Tender 9: the head column says 12 bn, the only head row converts to 11 bn — a
    // `rederive-eur` move not yet refolded (issue 375).
    exec(amount(9, 1, "NULL", 1_100_000_000_000, "EUR", Some(1_100_000_000_000))).await;
    // Tender 5: a REPEATED sentinel, so section 10 is not vacuously empty — and its head
    // is NULL (the election refused it), so it is not in the band.
    for _ in 0..10 {
        exec(amount(5, 1, "NULL", 2_222_222_222_222, "PLN", Some(500_000_000_000))).await;
    }
    // Tender 7: SEK 500 bn, its own currency group; a lot award 10⁴ below it.
    exec(amount(7, 1, "NULL", 50_000_000_000_000, "SEK", Some(4_400_000_000_000))).await;
    exec(
        "INSERT INTO tender_version_lot_results (tender_id, seq, lot_result_id, awarded_cents, awarded_currency) \
         VALUES (7, 1, 1, 5000000000, 'SEK')"
            .to_owned(),
    )
    .await;
    // Tender 1's corrigendum: a CHG-1 block naming II.1.7 (with the trailing `)`) and
    // its NEW_VALUE.TEXT, plus an unrelated CHG-2 naming IV.2.2.
    for (section, field, value) in [
        ("CHG-1", "TED-SECTION", "II.1.7 )"),
        ("CHG-1", "TED-NEW_VALUE.TEXT", "9 000 000.00 GBP"),
        ("CHG-2", "TED-SECTION", "IV.2.2"),
    ] {
        exec(format!(
            "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, value) \
             VALUES (1, '{section}', '{field}', 0, '{value}')"
        ))
        .await;
    }

    // The deferred indexes after the seed, as on prod (and as issue 429's test does: a
    // bulk build rather than 40k one-row index inserts).
    db.build_tender_indexes().await.expect("tender indexes");
    let report = measure(&db, "http://x").await;
    let ids: Vec<i64> = report.band.iter().map(|r| r.tender_id).collect();
    assert_eq!(ids, vec![7, 2, 8, 9, 1, 4], "top-down index order, one row per Tender; under the floor and NULL are out");
    let row = |id: i64| report.band.iter().find(|r| r.tender_id == id).expect("row");
    assert_eq!((row(1).currency.as_str(), row(1).cents), ("GBP", 900_000_000_000));
    assert_eq!(row(1).profile, "ted-export-r209");
    assert_eq!(row(1).head_notice, "123456-2017");
    assert_eq!(row(1).pow10_partner, Some(900_000_000), "the exact 10³ lot estimate");
    assert_eq!(row(1).smallest_sibling, Some(900_000_000));
    assert_eq!(row(1).value_corrigendum.as_deref(), Some("071343-2017"));
    assert_eq!(row(2).currency, "EUR");
    assert_eq!((row(2).pow10_partner, row(2).smallest_sibling, row(2).value_corrigendum.clone()), (None, None, None));
    assert_eq!(row(7).pow10_partner, Some(5_000_000_000), "a lot award 10⁴ below");
    assert_eq!(row(4).value_corrigendum, None, "no F14 in its chain");
    assert_eq!((row(4).pow10_partner, row(4).smallest_sibling), (None, None), "a 1.00 placeholder is no sibling");
    assert_eq!((row(8).currency.as_str(), row(8).cents), ("DKK", 14_920_000_000_000), "the read layer's tiebreak");
    assert_eq!((row(9).currency.as_str(), row(9).eur_cents), ("", 1_200_000_000_000), "a stale head is listed, figure-less");

    let text = data_quality::render_text(&report);
    let s10 = text.find("== 10.").expect("section 10");
    let s11 = text.find("== 11.").expect("section 11");
    let s16 = text.find("== 16.").expect("section 16");
    assert!(text[s10..s11].contains("22,222,222,222.22"), "section 10 sees the repeat:\n{text}");
    assert!(!text[s10..s11].contains("33,260,500,000.00"), "a unique figure is not a sentinel:\n{text}");
    let band = &text[s16..];
    assert!(band.contains("issue 471"), "{band}");
    assert!(band.contains("33,260,500,000.00"), "the unique head is in the band:\n{band}");
    for group in ["-- EUR (2 Tender(s)) --", "-- GBP (1 Tender(s)) --", "-- SEK (1 Tender(s)) --"] {
        assert!(band.contains(group), "{group}:\n{band}");
    }
    assert!(!band.contains("9,999,999,999.99"), "{band}");
    assert!(band.contains("1 with no elected row found"), "{band}");

    // The bound. Driving read: a range seek on the value index, not a scan of tenders.
    let sql = data_quality::band_listing_sql();
    // `(parent, detail)` per plan line: a sorter under parent 0 sorts the whole
    // listing; one under a correlated subquery sorts that Tender's few head rows.
    let plan_of = |c: turso::Connection| {
        let sql = sql.clone();
        async move {
            let mut rows = c.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.expect("plan");
            let mut out = Vec::new();
            while let Some(r) = rows.next().await.expect("plan row") {
                let parent = r.get_value(1).ok().and_then(|v| v.as_integer().copied()).unwrap_or(-1);
                out.push((parent, r.get_value(3).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default()));
            }
            out
        }
    };
    let check = |label: &str, plan: &[(i64, String)]| {
        let text = plan.iter().map(|(p, d)| format!("{p:>4} {d}")).collect::<Vec<_>>().join("\n");
        assert!(
            plan.first().is_some_and(|(_, d)| d == "SEARCH t USING INDEX tenders_current_value_eur (current_value_eur_cents>=?)"),
            "{label}: the band must drive from a seek on the value index; plan:\n{text}"
        );
        for (_, d) in plan.iter().filter(|(_, d)| d.contains("SCAN")) {
            panic!("{label}: every read must be a SEARCH, found a scan: {d}\nplan:\n{text}");
        }
        for (_, d) in plan.iter().filter(|(p, d)| *p == 0 && (d.contains("SORTER") || d.contains("TEMP B-TREE"))) {
            panic!("{label}: walked in index order, no sorter over the listing: {d}\nplan:\n{text}");
        }
    };
    check("empty sqlite_stat1", &plan_of(raw.connect().expect("connect")).await);

    // Filler, so statistics have a shape to plan with: 2,000 ordinary Tenders under
    // the floor (added AFTER the measurement above, which it would only slow down), each with a head version, two
    // amounts and a lot award — the "many rows, few per Tender" proportions of prod.
    exec("INSERT INTO tenders (id, source, kind, created_at, current_seq, current_value_eur_cents) \
          SELECT 100 + value, 'ted', 'procedure', 0, 1, 500000 + value FROM generate_series(1, 2000)"
        .to_owned()).await;
    exec("INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id) \
          SELECT 100 + value, 1, 3, 0, 'f' || value FROM generate_series(1, 2000)"
        .to_owned()).await;
    for lot in ["NULL", "1"] {
        exec(format!(
            "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency, eur_cents) \
             SELECT 100 + value, 1, {lot}, 'estimated_value', 500000 + value, 'EUR', 500000 + value \
               FROM generate_series(1, 2000)"
        )).await;
    }
    exec("INSERT INTO tender_version_lot_results (tender_id, seq, lot_result_id, awarded_cents, awarded_currency) \
          SELECT 100 + value, 1, 1, 400000 + value, 'EUR' FROM generate_series(1, 2000)"
        .to_owned()).await;

    // The same plan with statistics (issue 428: turso plans with `sqlite_stat1`; issue
    // 429's job writes it for exactly `ANALYZE_TABLES`). Fixture statistics over the
    // 2,000-Tender filler, not prod's — see the doc comment.
    for table in store::ANALYZE_TABLES {
        db.analyze_table(table).await.unwrap_or_else(|e| panic!("{table}: {e}"));
    }
    db.finish_analyze().await.expect("finish analyze");
    check("after ANALYZE", &plan_of(raw.connect().expect("reconnect")).await);

    drop(conn);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
