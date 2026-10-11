# 516 — the altid legal-form gate denies register-confirmed pairs: spelled-out forms, re-registrations, stray heads

Status: ready-for-agent — (a) DONE 2026-10-11: fix f44b159, gated, deployed 136cff8; dry job 2206 (legal-form 24 → 15, 9 verdict-admitted, plan 9) → wet job 2207 merged 9 of 9 (18 tenders touched); all 9 PPON orgs answer 308. NEXT: (b), a decision (7 pairs).
Was: ready-for-agent — filed 2026-10-11 from an audit of issue 469 U1's merges. Part (a) was BUILT and waited for the
next gate; part (b) needs a decision, and that decision is the next step.
Kind: data quality (organization identity)
Relates to: 448 (the altid arm; legal form is a structural gate with no verdict override), 469 U1 (cohort
`altid-469u1-2026-10-11`)

## What was found

U1 posted 80 HIGH `merge` verdicts, and the wet run merged 495 pairs. An audit sampled 10 of the merge verdicts on
the live API. Two pairs still stood as two orgs: Morgan Sindall `04273754` and NatWest `00929027`. Job 2205's stored
`altid-merge-plan` shows why. **16 of the 80 HIGH merge verdicts sit on pairs the `legal-form` gate denies.** That
gate is structural (canonical.rs, `match_org_altid`: "Head against head"), so no verdict can admit them. They fall
into two classes.

### (a) Spelled-out forms read as Ltd — 9 pairs (BUILT)

`gb_legal_family` took the LAST form token. So `PUBLIC LIMITED COMPANY` read as `ltd` (its `limited`), and so did
`LIMITED LIABILITY PARTNERSHIP`. The 9 pairs:
- `00929027` NatWest
- `00935048` Bayer
- `01123045` 3M United Kingdom
- `01372978` KONE
- `01800000` British Telecommunications
- `02317072` Interclass
- `SC139953` Optos
- `OC308311` Rapleys LLP
- `OC310651` Daniel Connal LLP

In each, one head spells the form out and the other abbreviates it.

**Fix (crosswalk.rs):** read `public limited company`, `public limited` and `limited liability partnership` as
`plc` / `llp` before the single tokens. The pin is `spelled_out_forms_are_their_abbreviations`.

The same function feeds four readers, and all of them move in the correct direction:
- the planner's head-against-head veto;
- `altid_corroborates`' FormConflict check over each side's name set;
- the resolver alias's bar;
- the rekey arm (`match_org_rekey`).

A side that names `X PUBLIC LIMITED COMPANY` beside a `X Ltd` side now reads as a conflict, which is correct.

**Drain after deploy:** a dry `match-org-identifiers {"rule":"altid"}` run. The 9 pairs carry HIGH merge verdicts in
`altid-469u1-2026-10-11`, so they should list as verdict-admitted in `plan`. Then a wet run (it holds to the stored
plan's `pairs`).

### (b) Ltd ↔ plc re-registrations and stray heads — 7 pairs (DECISION)

In these 7 pairs the register's own name history, or both sides' name sets, carries BOTH forms. The heads (each a
first-seen election) happen to differ:
- `02174990` Softcat: Softcat Limited, re-registered as Softcat plc. Both sides name both forms.
- `01991637` Axis Europe: both sides name both forms.
- `03784486` Lanes Group: both forms on the coh side.
- `05386273` Oxford Nanopore Technologies: Ltd, then plc in 2021. Both sides name both.
- `01428210` Specialist Computer Centres: the coh side names plc too.
- `04273754` Morgan Sindall Construction & Infrastructure: the coh org's head is the stray `Morgan Sindall plc`;
  its names include `… Ltd`.
- `02255270` Luceco UK Limited: the PPON names `Luceco UK Limited` too.

Reviewer and challenger both read these as HIGH merges against Companies House.

**The decision to take:**
- **(i)** Leave the veto head-against-head and accept the 7 as permanent splits.
- **(ii)** Let a HIGH merge verdict override `legal-form` when the two sides' form SETS overlap (the FormConflict
  test already reads sets). A disjoint plc-vs-Ltd pair, the parent/subsidiary shape the veto exists for, stays
  structural.
- **(iii)** Read the veto over the sides' form sets instead of the heads, for planned pairs too.

The filer's lean is (ii), not yet panelled. It changes nothing without a reviewed HIGH verdict, and (iii) would plan unreviewed pairs
that a stray `plc` name lets through.

**Whatever is chosen:**
- keep a test that pins the disjoint case as structural;
- add a test where a HIGH verdict admits an overlapping-set pair;
- add a test where a HIGH verdict cannot admit a disjoint-set pair.

## Verify

- After (a)'s deploy and drain, `/v1/organizations/<PPON org>` answers 308 for the 9: 30972064 NatWest, then Bayer,
  3M, KONE, BT, Interclass, Optos, Rapleys and Daniel Connal (read their org ids off the plan's `denied` listing).
- `denied_legal_form` in the next dry plan falls from 24 by at least 9.
