//! Task 5 / issue 120: which requests must run in the isolated pool.
//!
//! `read::walks` answers "can this request walk?" from the filter shape alone — which
//! is decidable before a row is read, unlike its *cost*, which issue 117 established
//! we cannot predict at all.
//!
//! Two properties matter and only one of them is about any particular filter:
//!
//! 1. **Every unserved shape routes to isolation.** A false negative reintroduces the
//!    defect — an expensive query on the main pool, starving the 8 REST readers.
//! 2. **The classification stays exhaustive as `Filter` grows.** That is enforced by
//!    the destructuring in `walks` itself: adding a field fails to compile until it is
//!    classified. This file cannot test that (a compile error is not observable from a
//!    test), so it is asserted here in prose and by the one thing a test *can* check —
//!    that every field currently on `Filter` is reachable in these cases, so a reader
//!    comparing them against the struct sees nothing missing.
//!
//! The asymmetry is the same one-sided hazard as the short-circuit guard: routing a
//! cheap query to isolation costs it a slot in a rarely-used pool; routing an expensive
//! one to the main pool is the outage. Every uncertain case must fall to isolation.

use store::read::{Collection, Filter, walks};

fn f() -> Filter {
    Filter::default()
}

#[test]
fn every_version_predicate_isolates_on_the_collections_that_apply_them() {
    // These are `EXISTS` subqueries evaluated per row. No index on the driven table
    // helps, because the filter is not a column of it.
    let cases: Vec<(&str, Filter)> = vec![
        ("country", Filter { country: Some("DE".into()), ..f() }),
        ("cpv", Filter { cpv: Some("45".into()), ..f() }),
        ("buyer", Filter { buyer: Some(7), ..f() }),
        ("winner", Filter { winner: Some(7), ..f() }),
        ("bidder", Filter { bidder: Some(7), ..f() }),
        ("status", Filter { status: Some(store::read::Status::Open), ..f() }),
        ("min_value", Filter { min_value: Some(1), ..f() }),
        ("max_value", Filter { max_value: Some(1), ..f() }),
    ];
    for (name, filter) in &cases {
        assert!(walks(Collection::Tenders, filter), "tenders?{name}= must isolate");
        assert!(walks(Collection::Lots, filter), "lots?{name}= must isolate");
    }
}

#[test]
fn the_joined_table_filters_on_lots_isolate() {
    // Isolated for unbounded COST on sparse and absent values, not for lack of an
    // index. Issue 16 made `lots` the driving table with a three-column primary-key
    // probe, so "no index can serve it" is no longer true — but `?kind=` on a value
    // with fewer rows than the page limit still walks all 13.2M lots (132.1s at prod
    // scale) because the work scales with DENSITY.
    //
    // If this assertion ever fails, the fix is not to delete it. A fast `?kind=Lot`
    // is not grounds for de-isolation: the sparse and absent cases are unchanged, and
    // de-isolating returns them to the main reader pool.
    assert!(walks(Collection::Lots, &Filter { source: Some("ted".into()), ..f() }));
    assert!(walks(Collection::Lots, &Filter { kind: Some("Lot".into()), ..f() }));
}

#[test]
fn a_tender_containment_bound_suppresses_isolation_even_with_a_companion_filter() {
    // Issue 212: `tender=X` makes the whole read a bounded containment lookup over one
    // Tender's lot slice (whole-corpus max ~2,604 lots), so no companion predicate can
    // walk — it must run on the main pool, not the shed-only isolated pool. Before the
    // fix, `walks` dropped the bound (`let _ = tender`) and a companion `kind`/`source`
    // isolated the always-cheap read, which then 503'd under isolated-pool saturation.
    for (name, filter) in [
        ("kind=Lot", Filter { tender: Some(1), kind: Some("Lot".into()), ..f() }),
        ("kind=<sparse>", Filter { tender: Some(1), kind: Some("zzz".into()), ..f() }),
        ("source", Filter { tender: Some(1), source: Some("ted".into()), ..f() }),
        ("country", Filter { tender: Some(1), country: Some("DE".into()), ..f() }),
    ] {
        assert!(
            !walks(Collection::Lots, &filter),
            "lots?tender=X&{name} is a bounded containment read — it must stay on the main pool"
        );
    }

    // The fix must NOT de-isolate the density-bounded cases when there is NO `tender`
    // bound — exactly the sparse/absent shapes issue 120 protects.
    assert!(walks(Collection::Lots, &Filter { kind: Some("zzz".into()), ..f() }));
    assert!(walks(Collection::Lots, &Filter { source: Some("ted".into()), ..f() }));
    assert!(walks(Collection::Lots, &Filter { country: Some("DE".into()), ..f() }));
}

#[test]
fn tenders_kind_isolates_because_no_index_covers_it() {
    // `t.kind` is covered by none of tenders_procedure_key / tenders_island /
    // tenders_current_published / tenders_source_id, so a value matching nothing walks
    // 4.26M rows — the same defect issue 117 fixed elsewhere. It was NOT in 117's
    // audit, which is exactly why the routing predicate is derived from the code
    // rather than from that audit's list.
    assert!(walks(Collection::Tenders, &Filter { kind: Some("zzz".into()), ..f() }));
}

#[test]
fn notices_publication_id_always_isolates() {
    // Issue 217: `publication_id` was expected to seek the source-leading
    // `UNIQUE(source, publication_id, …)` index, but the `ORDER BY id LIMIT`
    // pagination makes the planner drive off the id PK / `notices_source_id` and
    // FILTER — a sparse value (≤1 match) then walks the table to fill the page
    // (~10 s measured in prod, WITH or without `source`). Cost decides routing
    // (issue 120), so every `publication_id` lookup isolates.
    assert!(
        walks(Collection::Notices, &Filter { publication_id: Some("00018218-2024".into()), ..f() }),
        "publication_id alone walks — isolate it"
    );
    assert!(
        walks(
            Collection::Notices,
            &Filter {
                publication_id: Some("00018218-2024".into()),
                source: Some("ted".into()),
                ..f()
            }
        ),
        "source + publication_id still walks (ORDER BY id defeats the composite index) — isolate it"
    );
    assert!(
        !walks(Collection::Notices, &Filter { source: Some("ted".into()), ..f() }),
        "source alone is index-served (notices_source_id) and stays on the main pool"
    );
}

#[test]
fn the_index_served_shapes_stay_on_the_main_pool() {
    // Isolating these would push ordinary traffic through a small semaphore for no
    // benefit — the cost of being wrong in the safe direction, which is why the safe
    // direction is not the *only* consideration.
    assert!(!walks(Collection::Tenders, &Filter { source: Some("ted".into()), ..f() }));
    assert!(!walks(Collection::Notices, &Filter { source: Some("ted".into()), ..f() }));
    assert!(!walks(Collection::Notices, &Filter { kind: Some("eforms".into()), ..f() }));
    assert!(!walks(Collection::Organizations, &Filter { country: Some("DE".into()), ..f() }));
    assert!(!walks(Collection::Organizations, &Filter { kind: Some("vat".into()), ..f() }));
    // Issue 217: served by `organizations_identifier_id (identifier, id)`, so a lookup
    // by official id value seeks on the main pool — never the shed-only isolated one.
    assert!(!walks(
        Collection::Organizations,
        &Filter { identifier: Some("DE811907980".into()), ..f() }
    ));
    assert!(!walks(Collection::Lots, &Filter { tender: Some(1), ..f() }));
    assert!(!walks(Collection::Tenders, &f()), "an unfiltered page is index-driven");
}

#[test]
fn a_served_filter_does_not_rescue_an_unserved_one() {
    // The combined case, and the reason the predicate is an OR rather than a choice of
    // driver: `?source=ted&country=MT` seeks `tenders_source_id` to DRIVE the query and
    // still pays the per-row EXISTS for `country` on everything it drives through. The
    // index-served leg makes it no cheaper.
    let both = Filter { source: Some("ted".into()), country: Some("MT".into()), ..f() };
    assert!(
        walks(Collection::Tenders, &both),
        "an index-served driver must not mask an EXISTS-per-row filter"
    );
}

#[test]
fn organizations_and_notices_ignore_the_version_predicates() {
    // `read::organizations` and `read::notices` never call `version_predicates`, so
    // these fields are inert there. Routing on them would isolate requests that cannot
    // walk — conservative, but wrong, and it would put ordinary organization traffic
    // behind the semaphore the first time someone passed a stray `?cpv=`.
    let noise = Filter { cpv: Some("45".into()), winner: Some(7), ..f() };
    assert!(!walks(Collection::Organizations, &noise));
    assert!(!walks(Collection::Notices, &noise));
}
