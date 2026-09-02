//! Fold-source invariance for Phase-2 (issue 62). The projection now reads the
//! parsed layer for the fold in one of two ways ([`project::Phase2`]): the original
//! `ParsedFold` (each fold batch reads its scattered notices by id, random
//! group_key-order seeks) or the new `Buckets` (read the whole parsed layer once
//! sequentially, spill each resolved notice state to an order-preserving on-disk
//! bucket, fold each bucket sorted in RAM). Both must produce a BYTE-IDENTICAL
//! canonical layer — the bucketed path only changes HOW the parsed layer is read,
//! never WHAT is folded or in what order.
//!
//! Both runs are from-scratch rebuilds on fresh scratch DBs, so every surrogate id
//! starts at 1 in the same fold order → a straight table-by-table equality, ids and
//! all, is the correctness bar. The corpus mirrors `project_equivalence`'s: keyed
//! multi-notice chains, legacy OJS transitive chains ingested far apart, a bridge
//! merging two components, organizations whose VAT ids recur, and real island
//! fixtures — every grouping regime, with notices scattered across the id range.

use ingest::project::Phase2;
use ingest::{eforms, profile, project};
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projfold-{name}-{}.db", std::process::id());
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
/// across notices must resolve to one canonical Organization whichever fold reads it.
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

/// Every grouping regime, with a Tender's notices scattered across the id range.
async fn build_corpus(db: &Db, fetch_id: i64) {
    let vat = |k: u64| format!("NL{:09}B01", k % 37);

    // Keyed procedures: CN → corrigendum → CAN under one BT-04, three interleaved
    // waves so a procedure's notices land far apart in id.
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

    // Legacy OJS chains: a CN and an award referencing it by OJS number, two waves
    // so the chain's notices sit far apart in id.
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
        let award = Parsed {
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
        record(db, fetch_id, SOURCE, &award_num, "ted-export-r209", award).await;
    }

    // A bridge notice merging two legacy chains into one component (a late edge).
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

    // Islands: real fixtures with no procedure key.
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
/// keyed and ordered so it is stable across runs (mirrors `project_equivalence`).
/// Timestamp-only columns are excluded (the two runs stamp `now` at different
/// instants); every derived value — surrogate ids included — is compared.
async fn snapshot(db: &Db) -> String {
    let digests = [
        "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source AS r FROM tenders ORDER BY id)",
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

/// The bucketed sequential fold produces a canonical layer byte-identical to the
/// parsed-fold path — surrogate ids and all — on a rich mixed corpus.
#[tokio::test]
async fn bucketed_fold_matches_the_parsed_fold() {
    // A small Phase-2 budget so `Buckets` spans many buckets and `ParsedFold` many
    // batches — stressing bucket-boundary routing and batch boundaries alike.
    const BATCH: usize = 7;

    let (parsed, fp, path_parsed) = scratch("parsed").await;
    build_corpus(&parsed, fp).await;
    project::project_with_progress_phase2(&parsed, true, BATCH, Phase2::ParsedFold, |_| {})
        .await
        .expect("parsed-fold projection");

    let (buckets, fb, path_buckets) = scratch("buckets").await;
    build_corpus(&buckets, fb).await;
    project::project_with_progress_phase2(&buckets, true, BATCH, Phase2::Buckets { shards: None }, |_| {})
        .await
        .expect("bucketed projection");

    // Sanity: the corpus is non-trivial, so the comparison is meaningful.
    let parsed_tenders = count(&parsed, "SELECT COUNT(*) FROM tenders").await;
    assert!(parsed_tenders > 30, "the corpus should produce many Tenders");
    assert_eq!(
        parsed_tenders,
        count(&buckets, "SELECT COUNT(*) FROM tenders").await,
        "same Tender count"
    );

    assert_eq!(
        parsed.canonical_counts().await.expect("counts"),
        buckets.canonical_counts().await.expect("counts"),
        "every canonical table has the same row count under either fold source"
    );

    assert_eq!(
        snapshot(&parsed).await,
        snapshot(&buckets).await,
        "the canonical layer must be byte-identical whether folded from the parsed \
         layer directly or from the sequential on-disk buckets"
    );

    // The bucketed fold cleans up its scratch directory.
    assert!(
        !buckets.scratch_dir("proj_buckets").exists(),
        "the bucketed fold removes its scratch bucket directory when done"
    );

    for p in [path_parsed, path_buckets] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}

/// The pre-pass reports its sweep through `Progress::PrePass` (issue 65): the
/// count is non-decreasing, every tick lands before the fold starts applying
/// (the pre-pass is a barrier), and the CLOSING tick — the one emitted after all
/// shard workers join, which is the only tick a corpus this small is guaranteed
/// to produce — carries the complete sweep: every parsed notice, exactly once.
///
/// Pinning the closing tick is what makes the variant testable at all: the
/// parent polls the workers' shared counter on a coarse interval sized for
/// multi-hour prod sweeps, so a test corpus finishes before the first poll and
/// the intermediate ticks are timing-dependent. The final one is not.
#[tokio::test]
async fn the_prepass_reports_its_sweep_as_progress() {
    const BATCH: usize = 7;
    let (db, f, path) = scratch("prepass-progress").await;
    build_corpus(&db, f).await;

    let mut prepass = Vec::new();
    let mut applying_seen = false;
    // Issue 339: the FIRST fold tick must arrive at the barrier, before any bucket
    // has been applied — `tenders: 0` — so the phase record names the stage the
    // moment it starts instead of showing the last pre-pass count until the first
    // (biggest) bucket lands.
    let mut first_applying: Option<u64> = None;
    project::project_with_progress_phase2(&db, true, BATCH, Phase2::Buckets { shards: None }, |p| {
        match p {
            project::Progress::PrePass { notices } => {
                assert!(!applying_seen, "the pre-pass is a barrier: no tick after the fold starts");
                prepass.push(notices);
            }
            project::Progress::Applying { tenders, .. } => {
                applying_seen = true;
                first_applying.get_or_insert(tenders);
            }
            _ => {}
        }
    })
    .await
    .expect("bucketed projection");

    assert!(applying_seen, "the corpus folds, so the fold reported too");
    assert_eq!(
        first_applying,
        Some(0),
        "the fold announces itself at the barrier with tenders: 0, before its first bucket lands"
    );
    assert!(!prepass.is_empty(), "the pre-pass reported at least its closing tick");
    assert!(prepass.windows(2).all(|w| w[0] <= w[1]), "the sweep count never goes backwards: {prepass:?}");
    let parsed = count(&db, "SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed'").await;
    assert_eq!(
        prepass.last().copied(),
        Some(parsed as u64),
        "the closing tick is the whole corpus: every parsed notice swept exactly once"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// The sharded pre-pass (issue 66) is byte-identical to the serial one for ANY worker
/// count. Sharding the parsed read by notice-id splits a group's notices across shard
/// files (a keyed procedure's waves, a legacy CN and its award), but routing is by
/// group_key, so they land in one logical bucket that the fold re-concatenates and
/// sorts — the total fold order, and every surrogate id, is unchanged. We rebuild the
/// same rich corpus with 1, 3 and 8 pre-pass workers (8 stripes narrow enough to split
/// even the legacy chains) and assert the whole canonical snapshot is identical.
#[tokio::test]
async fn sharded_prepass_matches_the_serial_prepass() {
    const BATCH: usize = 7;

    async fn run(name: &str, shards: usize) -> String {
        let (db, fetch, path) = scratch(name).await;
        build_corpus(&db, fetch).await;
        project::project_with_progress_phase2(
            &db,
            true,
            BATCH,
            Phase2::Buckets { shards: Some(shards) },
            |_| {},
        )
        .await
        .expect("sharded projection");
        let snap = snapshot(&db).await;
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        snap
    }

    let serial = run("shard1", 1).await;
    assert!(
        serial.contains("--- digest 0 ---"),
        "the corpus must actually project some Tenders for the comparison to bite"
    );
    for shards in [3, 8] {
        assert_eq!(
            serial,
            run(&format!("shard{shards}"), shards).await,
            "the canonical layer must be byte-identical whether the pre-pass runs with \
             1 worker or {shards} — sharding changes only which worker writes a notice, \
             never which bucket it lands in or the order it folds"
        );
    }
}

/// Issue 94: the pre-pass's work partition must balance by parsed-notice COUNT, not
/// by id WIDTH.
///
/// Equal-width striping silently assumes notices are uniformly dense across id
/// space. On prod they are not — `MAX(id)` sits far above the dense region and bulk
/// reclaims append late — so equal-width stripes handed one worker nearly the whole
/// corpus while the others returned instantly on empty id space, and the "sharded"
/// sweep ran at ~1× however many workers it had.
///
/// The skew is reproduced here by asking for stripes over a range far wider than
/// the data occupies: equal-width would put every notice in stripe 0 and leave the
/// rest empty. Count-striping must instead give every stripe ~the same number of
/// parsed notices, while still covering `(lo, hi]` exactly and contiguously — the
/// property `write_shard`'s `id > lo AND id <= hi` window depends on for
/// completeness.
#[tokio::test]
async fn prepass_stripes_balance_by_notice_count_not_id_width() {
    let (db, fetch, path) = scratch("stripes").await;
    build_corpus(&db, fetch).await;

    let max_id = db.max_parsed_notice_id().await.expect("max id");
    let total = db.parsed_notice_count().await.expect("count") as i64;
    assert!(total >= 8, "corpus must be big enough to split ({total} notices)");

    // A range 1000× wider than the data — the prod shape, exaggerated.
    let (lo, hi) = (0i64, max_id * 1000);
    for k in [2usize, 4, 8] {
        let stripes = db.parsed_id_stripes(lo, hi, k).await.expect("stripes");
        assert_eq!(stripes.len(), k, "expected {k} stripes, got {}", stripes.len());

        // Contiguous, gapless, and covering exactly (lo, hi] — a gap would silently
        // drop notices from the fold.
        assert_eq!(stripes[0].0, lo, "first stripe must start at lo");
        assert_eq!(stripes[k - 1].1, hi, "last stripe must end at hi");
        for w in stripes.windows(2) {
            assert_eq!(w[0].1, w[1].0, "stripes must be contiguous: {:?}", stripes);
        }

        // Every stripe carries real work, and the split is even. Equal-width striping
        // over this range would give stripe 0 everything and the rest zero, so the
        // `min > 0` assertion alone is what fails on the old behaviour.
        let mut counts = Vec::new();
        for (s_lo, s_hi) in &stripes {
            let sql = format!(
                "SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed' \
                   AND id > {s_lo} AND id <= {s_hi}"
            );
            counts.push(match db.scalar(&sql).await.expect("stripe count") {
                Some(store::turso::Value::Integer(n)) => n,
                _ => 0,
            });
        }
        let (min, max) = (*counts.iter().min().unwrap(), *counts.iter().max().unwrap());
        assert_eq!(counts.iter().sum::<i64>(), total, "stripes must cover every notice exactly once");
        assert!(min > 0, "every stripe must carry work, got {counts:?} for k={k}");
        assert!(
            max - min <= 1 + total / (k as i64) / 4,
            "stripes must be balanced by notice count, got {counts:?} for k={k}"
        );
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 94 (3): bounding the pre-pass sweep to the PLAN's id range must not change
/// a single byte of the canonical layer.
///
/// `write_shard` already skips notices absent from the plan, so ids outside
/// `[MIN(plan_notice.notice_id), MAX(…)]` can never produce a bucket row — visiting
/// them is pure waste. The optimisation is therefore invisible by construction, and
/// this pins that: a scoped incremental fold whose delta sits at the TOP of the id
/// space (the shape of a late bulk reclaim, where the win is largest) must produce
/// exactly what an unscoped full non-rebuild projection produces, untouched Tenders
/// at low ids included.
///
/// The caveat worth knowing rather than discovering: the touched-Tender expansion
/// can pull merge partners from anywhere in id space, which widens the range back
/// out. That degrades the optimisation to a no-op, never to a wrong answer — which
/// is exactly why it is safe to stack on top of the count-striping.
#[tokio::test]
async fn scoping_the_sweep_to_the_plan_range_changes_nothing() {
    let (full, ff, pf) = scratch("scopefull").await;
    let (scoped, fs, ps) = scratch("scopescoped").await;
    build_corpus(&full, ff).await;
    build_corpus(&scoped, fs).await;
    project::project(&full, false).await.expect("establish full");
    project::project(&scoped, false).await.expect("establish scoped");
    assert_eq!(snapshot(&full).await, snapshot(&scoped).await, "established layers differ");

    // A delta at the TOP of the id space — every planned notice sits far above the
    // established corpus, so the plan range covers a small slice of the whole.
    for (db, fid) in [(&full, ff), (&scoped, fs)] {
        for p in 0..4u64 {
            let key = format!("bt04-late-{p:04}");
            let parsed = Parsed {
                sections: vec![sec("PROC", "Procedure", None)],
                values: vec![
                    id_val("PROC", "BT-04-notice", &key, false),
                    date_val("PROC", "BT-05(a)-notice", 9_000 + p as i64),
                    text_val("PROC", "BT-21-Procedure", &format!("Late procedure {p}")),
                ],
            };
            record(db, fid, SOURCE, &format!("late-{p:04}"), "eforms:eforms-sdk-1.13", parsed).await;
        }
    }

    project::project(&full, false).await.expect("full absorb");
    project::project_incremental(&scoped).await.expect("scoped incremental absorb");

    assert_eq!(
        snapshot(&full).await,
        snapshot(&scoped).await,
        "a plan-range-scoped sweep must be byte-identical to an unscoped full projection"
    );
    assert_eq!(
        scoped.unprojected_parsed_notice_ids().await.unwrap().len(),
        0,
        "the scoped sweep must still mark every planned notice projected"
    );

    for p in [pf, ps] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}
