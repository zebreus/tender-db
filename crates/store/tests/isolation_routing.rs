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
        ("currency", Filter { currency: Some("EUR".into()), ..f() }),
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
fn a_published_range_isolates_the_id_ordered_tenders_shape() {
    // Issue 216: `published_after/_before` on the ID-ORDERED list (SSE snapshots,
    // explicit sort=id) filter the PK walk — a narrow range walks the corpus to
    // fill its page, the issue-117 sparse class. The REST published-ordered path
    // does NOT consult this arm for the range: `tenders_by_published` rides
    // `tenders_current_published` by construction, and the handler strips the
    // bounds before asking `walks()` about the REMAINING filters.
    assert!(walks(Collection::Tenders, &Filter { published_after: Some(1), ..f() }));
    assert!(walks(Collection::Tenders, &Filter { published_before: Some(1), ..f() }));
    assert!(walks(Collection::Tenders, &Filter { deadline_after: Some(1), ..f() }));
    assert!(walks(Collection::Tenders, &Filter { deadline_before: Some(1), ..f() }));
    // Inert on the other collections — never applied there, so never isolating.
    assert!(!walks(Collection::Notices, &Filter { published_after: Some(1), ..f() }));
    assert!(!walks(Collection::Organizations, &Filter { published_after: Some(1), ..f() }));
}

#[test]
fn a_name_prefix_isolates_the_id_ordered_organizations_shape() {
    // Issue 217-B: the id-ordered application (SSE snapshots) filters the PK walk,
    // so it isolates; the REST search seeks organizations_name_norm_id in name
    // order and never consults this arm with the prefix still set.
    assert!(walks(Collection::Organizations, &Filter { name_prefix: Some("siemens".into()), ..f() }));
    assert!(!walks(Collection::Tenders, &Filter { name_prefix: Some("siemens".into()), ..f() }));
    assert!(!walks(Collection::Notices, &Filter { name_prefix: Some("siemens".into()), ..f() }));
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
fn notices_publication_id_seeks_on_the_main_pool() {
    // Issue 217-A, the fast path. History matters here because this test used to
    // assert the OPPOSITE: without a serving index, `ORDER BY id LIMIT` pagination
    // drove off the id PK / `notices_source_id` and a sparse value walked (~10 s,
    // measured in prod) — so every publication_id lookup isolated. Two things
    // changed, BOTH measured against prod's real file before this assertion flipped
    // (the 88d876a rule: never de-isolate on assumption):
    //   * `notices_publication_id_id (publication_id, id)` now serves the seek —
    //     1 ms present, 0.8 ms absent;
    //   * `notices_query` emits publication_id as the ONLY identity predicate
    //     (companions post-filter in Rust), because emitting `source` alongside let
    //     the planner flatten and drive from `notices_source_id` instead — 7.8 s.
    // If this fails after a query-shape change, check BOTH halves before touching
    // the routing: the index must exist AND the SQL must not re-admit companions.
    assert!(
        !walks(Collection::Notices, &Filter { publication_id: Some("00018218-2024".into()), ..f() }),
        "publication_id seeks its own index — main pool"
    );
    assert!(
        !walks(
            Collection::Notices,
            &Filter {
                publication_id: Some("00018218-2024".into()),
                source: Some("ted".into()),
                ..f()
            }
        ),
        "source + publication_id: the same seek (source post-filters in Rust) — main pool"
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

/// ISSUE 371 unit 1: the routed set and the guard set are ONE set.
///
/// Issue 219 fixed this class for country/cpv/buyer/winner/kind and stated the
/// invariant in PROSE — fold it into one probe "so the guard set and the `walks()` set
/// cannot drift again". Prose is what failed. `currency` was later added to `walks()`
/// (ADR-0014 D5) and `reachable()` never grew a leg, so a code present nowhere was
/// admitted, walked 7.93M rows and held one of four global isolation slots for 29.78 s
/// to answer empty.
///
/// The real fix is in the TYPE SYSTEM and cannot be tested from here: `walks()` is now
/// `!isolation_routed(..).is_empty()`, and `reachable()` consumes `isolation_routed`'s
/// output through an EXHAUSTIVE `match` over `read::Isolated`. Routing a new filter to
/// isolation therefore means adding a variant, and a new variant does not compile until
/// `reachable` gives it a probe or an explicit decline:
///
/// ```text
/// error[E0004]: non-exhaustive patterns: `Isolated::Whatever` not covered
///     --> crates/store/src/read.rs:1090:30
///      |
/// 1090 |         let admitted = match routed {
///      |                              ^^^^^^ pattern `Isolated::Whatever` not covered
///      |
/// note: `Isolated` defined here
///  670 |     Whatever,
///      |     -------- not covered
/// ```
///
/// (Reproduced 2026-09-07 by adding a variant and running `cargo check -p store`, so
/// the shape above is the real message rather than a remembered one.)
///
/// A compile error is not observable from a passing suite, which is why it is written
/// out above. What IS testable is the other half of the coupling — that the two faces
/// cannot disagree about a request, and that each isolating filter is NAMED rather than
/// folded into an anonymous boolean. A `walks()` that stopped delegating (an added
/// `|| new_filter.is_some()`, the exact 371 shape) fails here.
#[test]
fn the_isolated_set_and_the_guard_set_are_one_set() {
    use store::read::{Isolated, isolation_routed};

    let cases: Vec<(&str, Filter, Vec<Isolated>)> = vec![
        ("country", Filter { country: Some("DE".into()), ..f() }, vec![Isolated::Country]),
        ("cpv", Filter { cpv: Some("45".into()), ..f() }, vec![Isolated::Cpv]),
        ("buyer", Filter { buyer: Some(7), ..f() }, vec![Isolated::Buyer]),
        ("winner", Filter { winner: Some(7), ..f() }, vec![Isolated::Winner]),
        ("bidder", Filter { bidder: Some(7), ..f() }, vec![Isolated::Bidder]),
        ("currency", Filter { currency: Some("EUR".into()), ..f() }, vec![Isolated::Currency]),
        ("status", Filter { status: Some(store::read::Status::Open), ..f() }, vec![Isolated::Status]),
        ("min_value", Filter { min_value: Some(1), ..f() }, vec![Isolated::MinValue]),
        ("max_value", Filter { max_value: Some(1), ..f() }, vec![Isolated::MaxValue]),
        ("kind", Filter { kind: Some("procedure".into()), ..f() }, vec![Isolated::Kind]),
        (
            "published_after",
            Filter { published_after: Some(1), ..f() },
            vec![Isolated::PublishedAfter],
        ),
        (
            "deadline_before",
            Filter { deadline_before: Some(1), ..f() },
            vec![Isolated::DeadlineBefore],
        ),
    ];
    for (name, filter, expected) in &cases {
        assert_eq!(
            &isolation_routed(Collection::Tenders, filter),
            expected,
            "tenders?{name}= must route to isolation UNDER ITS OWN NAME — an isolating \
             filter that reaches `reachable()` as an anonymous boolean is how issue 371 \
             happened"
        );
    }

    // The two faces are the same decision. `walks()` delegates, so a filter cannot be
    // routed to isolation without appearing in the list the guard consumes.
    let collections =
        [Collection::Tenders, Collection::Lots, Collection::Organizations, Collection::Notices];
    let matrix: Vec<Filter> = cases
        .iter()
        .map(|(_, filter, _)| filter.clone())
        .chain([
            f(),
            Filter { source: Some("ted".into()), ..f() },
            Filter { name_prefix: Some("siemens".into()), ..f() },
            Filter { tender: Some(1), kind: Some("Lot".into()), ..f() },
            Filter { publication_id: Some("00018218-2024".into()), country: Some("DE".into()), ..f() },
            Filter { identifier: Some("DE811907980".into()), ..f() },
            Filter { source: Some("ted".into()), country: Some("MT".into()), ..f() },
        ])
        .collect();
    for collection in collections {
        for filter in &matrix {
            assert_eq!(
                walks(collection, filter),
                !isolation_routed(collection, filter).is_empty(),
                "{collection:?}: `walks` and `isolation_routed` disagree about {filter:?}. \
                 They are the same decision by construction — if they can differ, the \
                 guard is once again probing a different set from the one being routed"
            );
        }
    }

    // The suppressors keep suppressing, named this time: a bounded read routes NOTHING,
    // so the guard is asked nothing either.
    assert!(
        isolation_routed(Collection::Lots, &Filter { tender: Some(1), country: Some("DE".into()), ..f() })
            .is_empty(),
        "a tender containment bound suppresses isolation entirely (issue 212)"
    );
    assert!(
        isolation_routed(
            Collection::Tenders,
            &Filter { publication_id: Some("00018218-2024".into()), currency: Some("EUR".into()), ..f() }
        )
        .is_empty(),
        "a publication seed suppresses isolation entirely (issue 217-A)"
    );

    // `source` isolates on Lots and not on Tenders, and the guard leg follows the
    // routing rather than a second collection test of its own.
    assert_eq!(
        isolation_routed(Collection::Lots, &Filter { source: Some("ted".into()), ..f() }),
        vec![Isolated::Source]
    );
    assert!(
        isolation_routed(Collection::Tenders, &Filter { source: Some("ted".into()), ..f() }).is_empty(),
        "tenders?source= seeks tenders_source_id — no isolation, so no guard leg is owed"
    );

    // `kind` is routed LAST on both collections that route it: its guard leg is a bare
    // table scan, so every cheaper leg must get to short-circuit before it is paid for.
    let everything = Filter {
        country: Some("DE".into()),
        currency: Some("EUR".into()),
        kind: Some("procedure".into()),
        ..f()
    };
    for collection in [Collection::Tenders, Collection::Lots] {
        assert_eq!(
            isolation_routed(collection, &everything).last(),
            Some(&Isolated::Kind),
            "{collection:?}: the table-scan leg must be probed last"
        );
    }
}
