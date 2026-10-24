# 591 — a reparse deletes every party row citing the notice, so its Tenders serve without those parties until the re-fold, and the re-fold announces every one as corrected

Status: ready-for-agent — **BUILT 2026-10-24 ~22:00Z in 3e055d4 (deploy #4's tree, with 546 + 561 U4; gate running)**: `clear_parsed` deletes party and bid-party rows only for the sections the new parse does not re-create, by the FK's full key inside the mention loop (child before parent, `_mention_key` seek); the issue-404 re-key path in `record_notice` also stamps its Tenders epoch-stale (the diagnosis skeptic's sibling case: `projected = 0` alone hits the fold's early return, so the stale `publication_id` never got rewritten). Tests: reparse_fk `a_projected_notice_can_be_reparsed` (kept section keeps its party row) + `a_vanishing_sections_party_rows_go_and_a_kept_sections_stay`; mention_fk_probe `the_per_section_party_delete_seeks`; the in-crate re-key test pins the stamp; project 118/118 and project_incremental 51/51 green. CHANGELOG entry (issue 591). NEXT: deploy #4, then re-enqueue 577 slice 3 (sdk probe) and read its compare (Verify). Was: ready-for-agent — filed 2026-10-24 ~20:50Z from 577 unit 3 slice 1's diagnosis (wf_feca2058-025,
`577-u3/s1-diagnosis/`: code and data lines agree). Next: unit 1 BUILD — in `Db::clear_parsed` delete the party and bid-party
rows only for the sections the new parse does NOT re-create (the same `keep` set issue 248 already applies to
`organization_mentions`), by the FK's full key; a store test that re-parses a folded notice and keeps its party rows;
then deploy (rides deploy #4 if it is still open) and re-enqueue 577's slice 3 and sdk rounds under it.
Kind: served data gap + false correction announcements (N for values; W-ish while the window is open: a Tender serves no
buyer)
Relates to: 577 (unit 3 slices: the 0-correction bar is unreadable while this stands), 495 (the compare that reports the
restore as corrections; its design names "the re-parse's party deletes", issues/495:217, :308), 248 (keeps surviving
sections' mentions), 247 / 441 (the delete costs and the FK index), 539 (run b was a refold, so it never hit this).

## What happens

- `crates/store/src/lib.rs` `clear_parsed` (≈:2480-2510 at 6cb4408), called from `reparse_notice`, runs
  `DELETE FROM tender_version_parties WHERE mention_notice_id = ?` and the same for `tender_version_bid_parties` for every
  re-parsed notice. Its comment says only what the FK forces is deleted; that stopped being true when 248 began keeping the
  mentions of sections the new parse re-creates. The FK now forces only the rows whose section goes away.
- Between the reparse and the paired `project`, every Tender citing such a notice serves without those parties (577
  slice 1: 2773 → 2774, about 60 min, 126,812 Tenders; slice 2: 2776 → 2777 now).
- The project's compare (issue 495) reads the short stored tables, rewrites them, and announces each as a correction:
  2774 corrected 126,812 Tenders (0 verified) with 326,839 correction rows (tender 126,812, lot 200,027) — only
  `tender_version_parties` (220,649 version tables) and `tender_version_bid_parties` (186,010) differed; the other 13 tables
  were 100% identical. 539 run b, a refold of the same notices, corrected 2,211.
- The restored rows match the notices: the archive XML of 26213590 and 26198825 declares exactly the stored roles (8/8,
  11/11), the TED copies (not re-parsed) kept their 10-08 rowids, and where rowid gaps allow, old count = new count.

## Verify

    ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=20"' | python3 -c "import sys,json;[print(r['job_id'],r['counts'][:200]) for r in json.load(sys.stdin)['recent'] if r['kind']=='project']"

- **open (2026-10-24)**: 2774 reads `compare (issue 495): 0 tenders verified, 126812 corrected`.
- **done**: the first reparse slice after the fix pairs with a project whose compare corrects only Tenders whose parse
  really changed (slice-3 probe expectation: corrected ≪ written; parties tables mostly identical).
