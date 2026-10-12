# 514 unit 2 — review packet and verdicts (2026-10-12)

The workflow was wf_d1abdb0b-672: a packet builder, six reviewers and six challengers
(`review-wf_d1abdb0b.json`, `packet.json`, `packet.md`, `groups.json`).

- **Packet.** The decision's §1.3 rules over census `raw/001-004` give exactly 54 rewrites, 26 E0 groups (52 members)
  and 27 lone movers (743 group mentions).
  - Prod reads: 8 bounded calls, 0 × 408. All 79 orgs are unchanged since the census.
  - Registers: 53 distinct numbers, all found (100 GETs). All 19 FCA societies are Registered, all 27 charities are
    Registered, all 6 UKPRNs are active, and CCNI 102575 Radius is listed.
- **Verdicts.** All 26 groups are **merge** and 26 movers are **move-ok**. 1 mover is **move-hold**: L14 31594820,
  "Birmingham Citizens Advice Bureaux", carries 279057, the NATIONAL Citizens Advice charity. Birmingham's own is
  519639. The challenger agreed on all 53.
- **Hold list (§3.3, S11):** `279057` (L14). One mismatch, so S11 (two or more) does not stop the unit.
- **P0 item 3.** Merge verdicts were read back through `/admin/case-reviews?table=merge&limit=5000` (1,268 rows). The
  only ones touching the 79 orgs are 469 U6's five applied altid merges (a different scheme), so there is no standing
  `keep`. Identifier verdicts: none of the 79 orgs appear in the local verdict files.
- **Anomalies kept for the build:**
  - A3: Birmingham Mind's twin carries a generic label that stays deferred, so that split stays open.
  - A6: 31558903 stays headed "Sheffield Hallam University" from one stray mention. It is the survivor of G24 by lower
    id. A head-name repair is a follow-up.
  - A9: in 8 groups the surviving lower id is the mover.

Posting: the 26 merge verdicts go up in the scheme and keys the U2.3 E0 change reads, so they are posted once U2.2/U2.3
exist. S8 requires them, and the frozen hold list, before the gate.
