//! In-place backfills (ADR-0017 D6's route R2) and the correction rows they owe (D5).
//!
//! # The R2 template
//!
//! An R2 walk rewrites a stored value where it sits, with no fold and no planning.
//! It is allowed only when ADR-0017 D6's E1–E6 all hold. In short:
//! - E1: the value is a pure function of the stored canonical rows, reference tables
//!   and the causing notice's own value rows;
//! - E2: there is one implementation (see the fixture below);
//! - E3: nothing is minted or removed;
//! - E4: every stored value derived from the moved one is re-derived too, or its
//!   Tenders are stamped stale and re-queued for the fold;
//! - E5: version N depends only on versions 1..=N;
//! - E6: the fold writes the same value from the deploy onwards.
//!
//! Its mechanics, in order:
//! 1. **Windows over the `tenders` primary key** ([`tender_window`]). Each statement
//!    filters `tender_id > ? AND tender_id <= ?`, the PK or by-version index prefix.
//!    Not a bare rowid range: issue 274's seek lesson, which `rederive-eur` relearned
//!    when its first cut collapsed about 55M rows in.
//! 2. **One `BEGIN IMMEDIATE` per window** ([`Db::immediate`](crate::Db)). Inside it go
//!    the updates, the window's correction rows ([`Moved::announce`]), any stamp and
//!    re-queue that E4 calls for, and the watermark. A crash then redoes at most one
//!    window, and never loses an announcement or a re-queue to it.
//! 3. **A watermark in `projection_state`**, its own column, written in the window
//!    transaction and reset to 0 when the walk ends. A restarted job resumes past it.
//! 4. **A completion flag**, only when a read path relies on the walk having reached
//!    the end (issue 371's currency present-set). It is set after the last window,
//!    never before.
//! 5. **One checkpoint per window**, after its COMMIT (issue 42's WAL discipline).
//! 6. **D5's correction rows.** A walk that moves a value REST serves, or that an
//!    SSE filter reads, announces each moved Tender and lot with seq-less `changed`
//!    rows in the window transaction, and rings the doorbell after COMMIT. A new
//!    additive column, or a stored copy of a value REST already derived, moves
//!    nothing served and stays quiet.
//!
//! The mandatory E2 fixture: a fold under the new logic is byte-identical to a fold
//! under the old logic followed by the walk (and the fold the walk re-queued, for E4),
//! on a fixture corpus that exercises the moved value.
//!
//! The worked example is `rederive-eur` ([`Db::rederive_eur_window`](crate::Db)): four
//! money loci, a moved `eur_cents` announced per D5, the elected head and lot values
//! handed back to the fold by a stamp and a re-queue (E4). Its E2 fixture is
//! `a_rederive_eur_walk_then_its_fold_equals_a_fold_under_the_new_rates` in
//! `crates/ingest/tests/project_golden.rs`.

use std::collections::{BTreeMap, BTreeSet};

use turso::{Connection, Value};

/// ADR-0017 D3's correction rows for one Tender, in the order they are written: rule
/// T's `tender changed`, then one `lot changed` per rule-L lot, ascending. Every row is
/// seq-less. The fold (issue 495 unit 4) and every in-place walk (D5) write through
/// this, so the two announce the same shape.
pub(crate) fn correction_rows(tender_id: i64, lots: &BTreeSet<i64>) -> impl Iterator<Item = (&'static str, i64)> + '_ {
    std::iter::once(("tender", tender_id)).chain(lots.iter().map(|&lot| ("lot", lot)))
}

/// The first `batch` Tender ids past `after`, as `(count, last id)`. A count of 0 ends
/// the walk, and the last id is then `after`.
pub(crate) async fn tender_window(conn: &Connection, after: i64, batch: i64) -> turso::Result<(i64, i64)> {
    let mut rows = conn
        .query(
            "SELECT COUNT(*), MAX(id) FROM
               (SELECT id FROM tenders WHERE id > ? ORDER BY id LIMIT ?)",
            (Value::Integer(after), Value::Integer(batch)),
        )
        .await?;
    let window = match rows.next().await? {
        Some(row) => (crate::int(&row, 0), crate::opt_int_of(&row, 1).unwrap_or(after)),
        None => (0, after),
    };
    while rows.next().await?.is_some() {}
    Ok(window)
}

/// What an in-place walk moved, row by row, for D5's correction rows.
#[derive(Debug, Default)]
pub(crate) struct Moved {
    tenders: BTreeMap<i64, MovedRows>,
}

#[derive(Debug, Default)]
struct MovedRows {
    /// The versions a moved row sits in.
    seqs: BTreeSet<i64>,
    /// The lots whose own rows moved: rule L's second set.
    lots: BTreeSet<i64>,
}

impl Moved {
    /// A served value moved in place in a row of `tender_id`'s version `seq`. `lot` is
    /// the lot the row belongs to: `None` for the Tender's own row, or for a table with
    /// no lot scope.
    pub(crate) fn row(&mut self, tender_id: i64, seq: i64, lot: Option<i64>) {
        let moved = self.tenders.entry(tender_id).or_default();
        moved.seqs.insert(seq);
        moved.lots.extend(lot);
    }

    /// The moved Tenders, ascending.
    pub(crate) fn tender_ids(&self) -> Vec<i64> {
        self.tenders.keys().copied().collect()
    }

    /// Write D3's correction rows for everything moved, inside the caller's transaction,
    /// in Tender id order. Returns how many rows were written.
    ///
    /// Rule T for every moved Tender. Rule L is the lots whose own rows moved, plus
    /// every lot of the head version when a moved row sits in the head, because lots
    /// inherit the Tender's version predicates (ADR-0017 D3). A walk mints and removes
    /// nothing (E3), so rule L has no minted lots to add and no swept lots to drop.
    pub(crate) async fn announce(&self, conn: &Connection, now: i64) -> turso::Result<u64> {
        let mut written = 0u64;
        for (&tender_id, moved) in &self.tenders {
            let mut lots = moved.lots.clone();
            if let Some(head) = head_seq(conn, tender_id).await?
                && moved.seqs.contains(&head)
            {
                head_lots(conn, tender_id, head, &mut lots).await?;
            }
            for (kind, id) in correction_rows(tender_id, &lots) {
                crate::canonical::append_change(conn, kind, id, None, "changed", now).await?;
                written += 1;
            }
        }
        Ok(written)
    }
}

/// The Tender's head: its highest stored seq, off the `tender_versions` primary key.
async fn head_seq(conn: &Connection, tender_id: i64) -> turso::Result<Option<i64>> {
    let mut rows =
        conn.query("SELECT MAX(seq) FROM tender_versions WHERE tender_id = ?", (Value::Integer(tender_id),)).await?;
    let head = match rows.next().await? {
        Some(row) => crate::opt_int_of(&row, 0),
        None => None,
    };
    while rows.next().await?.is_some() {}
    Ok(head)
}

/// Add the lots of `tender_id`'s version `seq` to `lots`, off the `tender_version_lots`
/// primary key.
async fn head_lots(conn: &Connection, tender_id: i64, seq: i64, lots: &mut BTreeSet<i64>) -> turso::Result<()> {
    let mut rows = conn
        .query(
            "SELECT lot_id FROM tender_version_lots WHERE tender_id = ? AND seq = ?",
            (Value::Integer(tender_id), Value::Integer(seq)),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        lots.insert(crate::int(&row, 0));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correction_rows_are_rule_t_then_rule_l_ascending() {
        let lots: BTreeSet<i64> = [12, 3, 7].into_iter().collect();
        assert_eq!(
            correction_rows(5, &lots).collect::<Vec<_>>(),
            vec![("tender", 5), ("lot", 3), ("lot", 7), ("lot", 12)]
        );
        assert_eq!(correction_rows(5, &BTreeSet::new()).collect::<Vec<_>>(), vec![("tender", 5)]);
    }

    #[test]
    fn moved_rows_collect_per_tender_lots_and_seqs() {
        let mut moved = Moved::default();
        moved.row(9, 2, Some(40));
        moved.row(4, 1, None);
        moved.row(9, 1, Some(41));
        moved.row(9, 2, None);
        assert_eq!(moved.tender_ids(), vec![4, 9]);
        let nine = &moved.tenders[&9];
        assert_eq!(nine.seqs.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(nine.lots.iter().copied().collect::<Vec<_>>(), vec![40, 41]);
        assert!(moved.tenders[&4].lots.is_empty());
    }
}
