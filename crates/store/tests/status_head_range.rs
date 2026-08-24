//! Issue 273 step 1 pin: `status` on the TENDERS shapes is a head-range on the
//! indexed `current_deadline` column, never the per-row submission-deadline
//! EXISTS. The EXISTS form combined with a sparse second filter walked the whole
//! deadline-ordered stream to the 30s service bound (`status=open&country=LU` →
//! 503, four of them brown out the walk pool); the range bounds the scan to the
//! open head (0.13s validated on prod, 2026-08-24). Lots shapes keep the EXISTS
//! — no head column there — so this also pins that the split stays split.

use store::read::{tenders_ordered_statement, lots_statement, Filter, HeadOrder, Scope, Status};

fn f() -> Filter {
    Filter { status: Some(Status::Open), country: Some("LU".into()), now: 1_756_000_000, ..Filter::default() }
}

#[test]
fn tenders_status_is_a_current_deadline_range() {
    let (sql, _) = tenders_ordered_statement(&f(), HeadOrder::Deadline, false, None, 25);
    println!("TENDERS SQL:\n{sql}");
    assert!(sql.contains("t.current_deadline > ?"), "Open must be the head range: {sql}");
    assert!(
        !sql.contains("d.field = 'submission_deadline'"),
        "the walking EXISTS form must be gone from the Tenders shape: {sql}"
    );

    let closed = Filter { status: Some(Status::Closed), ..f() };
    let (sql, _) = tenders_ordered_statement(&closed, HeadOrder::Deadline, false, None, 25);
    assert!(
        sql.contains("(t.current_deadline IS NULL OR t.current_deadline <= ?)"),
        "Closed is NULL-or-past — a Tender that never published a deadline cannot be bid on: {sql}"
    );
}

#[test]
fn lots_status_keeps_the_exists_form() {
    let (sql, _) = lots_statement(&f(), Scope::Page { after: 0, limit: 25 });
    assert!(
        sql.contains("d.field = 'submission_deadline'"),
        "lots have no head deadline column; the EXISTS stays: {sql}"
    );
}
