# 522 — the UK's shared withheld-supplier PPON binds unrelated suppliers into one org

Status: done — VERIFIED 2026-10-14 22:5x UTC. Deployed e2da1e9. The dissolve 2489 (dry 2488) removed 30914328 and 31548264; the Verify reads 0. HISCOX → 2123088, DOC-Labor → 31966281, the four withheld `Individual`s → 31966280 (the residual is issue 529). Was: ready-for-agent — BUILT, REVIEWED, GATED 2026-10-14 (gate green at 3ad1238, 2026 tests; see "Unit 1"); next:
deploy after the 526 unit 3 drain (queue idle), then the `repair-placeholder-orgs` dissolve per the runbook. Filed 2026-10-12 from issue 511 unit 4's item 5 read.
Kind: data quality (organization identity)
Relates to:
- 511 (placeholder names; unit 4 drops the s.94 sentences);
- 452 (identifier verdicts: `wrong`, the per-identifier withhold);
- 466 (related numbers withheld from grouping).

## Verify

    echo "SELECT count(*) FROM organization_mentions WHERE organization_id = 30914328" | ssh root@zebreus.click /root/sq.sh

- **done**: the count drops to the placeholder mentions only (or the org is gone), and HISCOX / DOC-Labor mentions bind
  their own orgs. The same holds for 31548264 (`PGYY-2165-QXNQ`);
- **open**: `9` as on 2026-10-12, with real suppliers among them; `4` on 2026-10-14 (HISCOX 46800936, Individual
  46806664, DOC-Labor 46818757, Individual 46819715 — the s.94 sentences dropped in 511 unit 4).

## What was found

FTS publishers put one PPON, `GB-PPON-PBXY-3232-WNHZ`, on every supplier whose identity a notice withholds under
s.94 of the Procurement Act 2023. Org 30914328 ("Withheld Section94 Supplier") holds it. Its 9 mentions on
2026-10-12, all under section `ORG-GB-PPON-PBXY-3232-WNHZ`:
- "Information withheld due to security" (2);
- "This information has been omitted from this notice in accordance with s.94 Procurement Act 2023" (3);
- "Individual" (2);
- **"HISCOX INSURANCE COMPANY LIMITED"** (notice 46800936);
- **"DOC-Labor GmbH"** (notice 46818757).

Unit 1 of issue 511 already noted the shape (decision line 188: one PPON covers different withheld suppliers on all
123 mentions). The PPON identifies the placeholder, not a supplier. Every party that carries it binds the one org
by identifier, so two real companies are filed under "Withheld Section94 Supplier", and the org gathers
whatever names come next.

Since 511 unit 4, the placeholder sentences drop as `Placeholder`. The real names still bind the PPON org.

## Proposed unit

1. Record a 452 identifier verdict, `wrong`, on (`GBPPONPBXY3232WNHZ`, national, GB) with the rationale above.
   - Per 452, the number then leaves the R2/E0/R3 groups and the resolver's canonical bind.
   - A mention binds the owner only when its names match the owner's.
   - "HISCOX INSURANCE COMPANY LIMITED" would then bind by name, or mint.
2. Re-fold the 9 notices (`refold-notices`). Then read where the real names landed, and that 30914328 keeps only
   placeholder mentions or is swept.
3. Measure first: whether other shared-placeholder PPONs exist. These are PPONs whose holders' mention names include
   both a placeholder sentence and a legal-form company name. Run it as an in-app read or over 511's `identified`
   lists, never a corpus scan from the agent.

## Unit 1 — the proposed verdict cannot work; the gate can (2026-10-14)

**Why not a 452 `wrong` verdict.** The resolver binds the EXACT triple first: `org_of.get(&key)` on
(`GB`, `national`, `GBPPONPBXY3232WNHZ`) in `canonical.rs` (the identified branch, about line 19755). A `wrong` verdict
only guards canonical-key binds and grouping (`guarded`), so HISCOX's mention would still bind org 30914328 by its
literal. Re-folding would not move it either. A recorded mention is kept while it `publishes` the same name, country,
raw identifier and scheme (`RecordedMention::publishes`, issue 434), and none of those change.

**Measure-first (step 3), from the local 2026-09-30 GB census and 511's own reads.** One more s.94 placeholder PPON
exists. 511's `unit1-decision.md` and its u3 and u4+u5 reads named three candidates; review proved one of them is a
real supplier:
- `PBXY-3232-WNHZ`, "Withheld Section94 Supplier" (30914328);
- `PGYY-2165-QXNQ`, "Withheld Section94 supplier - 2nd profile" (31548264). All 14 of its mentions in 511's unit 3
  read carried placeholder names, and no 448 altid case pairs it with a register. It has 2 mentions now, both
  `Individual`, on notices 46806664 and 46819715, beside PBXY's two;
- **not** `PMBM-2912-XLDW`, "Withheld for Security Reasons - Section 94 Supplier" (31567546). It is EPI-USE LIMITED's
  own PPON. On publications `087781-2026` and `035915-2026` one party carries it beside Companies House number
  03707885. The 448 altid review merged it into 23401424 (case `03707885~PMBM2912XLDW`, `merge`, high: "a redaction
  placeholder [name], not a second entity"). That merge is why 31567546 is gone. 511's reads had inferred
  "placeholder" from the head name alone. Condemning it would have split EPI-USE off 23401424 on every new notice,
  and taken it out of the altid pairing for good. It is now a pinned near miss in the test.
  - **Lesson:** a withheld NAME on a PPON is not a placeholder PPON. Only a value the register itself names as a
    withheld-supplier profile, and that is seen covering different suppliers, qualifies.
  - **Audit (2026-10-14 12:5x).** 23401424 "EPI-USE Ltd" holds 8 mentions, 4 under `GB-PPON-PMBM-2912-XLDW`. Two of
    those carry the name "Withheld for Security Reasons - Section 94 Supplier" and two "EPI-USE LIMITED". Both bind
    correctly by the PPON.
  - **Note for 511's tail.** Do not add that phrase to the placeholder list. A `Placeholder` has no identifier guard,
    so it would drop EPI-USE as a party on those two notices.

`ECC Dummy Supplier` (`PMWP-8866-RJWR`, 31547273, 1 mention) is a buyer's dummy profile, not an s.94 one. It is left
out: it has one mention and no fusion.

**What is built.** `idgate::condemns` gains a category class, `withheld_placeholder`. It is a list of the two PPON
bodies, matched in every spelling the corpus keys: `GBPPON…`, bare, and `PPON…` (the 481 token form). The category
proof is the same kind 440 used. The register names each one as a withheld-supplier profile, and 511's unit 1 read
found the first on all 123 of its mentions, covering different suppliers. One gate covers every path:
- the mention (`normalise_identifier` → `None`, so `gated_identifier` too: HISCOX binds or mints by name);
- the altid pairing (`altid_pair_key` / `mention_key`);
- the R2/E0 groups (`condemns` injected);
- the 481 raw-identifier token;
- the stock, through the `repair-placeholder-orgs` dissolve, the path 440 used.

Test: `idgate::tests::the_withheld_supplier_ppon_is_never_an_organization_identifier`. It pins five spellings
condemned, and the end-to-end gate plus altid key as `None`. Near misses keep merge-key status:
- FINYX's neighbouring `PBXY-6699-ZZGN`;
- the identified `Individual`'s own `PMGL-5552-GJZX` on the same notice 46806664 (org 31570158);
- a bare `PBHV7537LCTH`;
- three one-character variants.

**Measured before the dissolve (bounded, 2026-10-14):**
- **Name-reuse targets.** The provisional path reuses only `identifier IS NULL` rows on `(name_norm, country)`:
  - HISCOX → 2123088 (GB, provisional);
  - DOC-Labor GmbH → a fresh GB provisional (its two orgs are DE);
  - the four s.94 `Individual`s (PBXY's two, PGYY's two) → one fresh GB provisional `Individual`.
- **No reuse of 31570158.** The identified `Individual` org is never a reuse target, because it carries a PPON.

**Review (2026-10-14, three lenses: matching / consumers / dissolve).**
- **The one defect:** PMBM (above). It was removed.
- **Consumers, none harmful.** These callers were traced:
  - `repair_renormalised_identifiers` counts the two rows `unexplained` and never plans them;
  - `repair_label_prefixes` and `repair_minted_countries` (VAT only) never touch them;
  - the R2/E0/R3/altid planners only raise `denied_gate`;
  - the 481 token and the 513 tools skip on `None`;
  - no fixture uses the values.
- **Dissolve, claims (a)–(c) confirmed.**
  - Each org has one mention per notice, so every winner row resolves at tier 1 and nothing is skipped.
  - New FTS notices bind by name; no triple, alias or rekey cache is consulted on the identifier-less branch.
- **Pre-dissolve reads, done 2026-10-14:**
  - `organizations` by identifier (indexed): exactly 30914328 and 31548264 hold the two bodies in any spelling.
  - `/admin/case-reviews`, the case / rehoming / country / merge / identifier stores: 0 rows name either id or
    either PPON.
  - The name store is keyed by `name_norm` and does not apply.
  - `organization_merged_identifiers` is not on the `/v1/sql` surface. The 448 campaign files carry no case for
    either PPON.
- **Accepted costs:**
  - On 46806664 and 46819715 the PBXY and PGYY `Individual`s are two different withheld suppliers. Both land on one
    fresh `Individual` row, and their winner rows on a result collapse into one, so those lots lose one distinct
    winner. A fresh fold gives the same; see the residual below.
  - The dissolve writes no `lot changed` rows. This is pre-existing and also true of merges.

**Runbook (after the 526 unit 3 drain, queue idle).** This is 440's sequence. The dry report gives counts, not org
ids, so re-run the bounded identifier read first:

    echo "SELECT id, identifier FROM organizations WHERE identifier IN ('GBPPONPBXY3232WNHZ','GBPPONPGYY2165QXNQ','PBXY3232WNHZ','PGYY2165QXNQ','PPONPBXY3232WNHZ','PPONPGYY2165QXNQ','GBPBXY3232WNHZ','GBPGYY2165QXNQ')" | ssh root@zebreus.click /root/sq.sh
    echo "SELECT organization_id, count(*) FROM organization_mentions WHERE organization_id IN (30914328, 31548264) GROUP BY 1" | ssh root@zebreus.click /root/sq.sh


    /root/aj.sh /admin/jobs '{"kind":"repair-placeholder-orgs","dry_run":true}'
    # expect 2 condemned (30914328, 31548264), 2 dissolved, mentions = the count just read (6 on 2026-10-14; daily
    # folds before the deploy may add more). Dry fresh/reused differ from wet (dry inserts nothing). RECONCILE: every earlier class read 0 after 365/440,
    # so a larger number means drift or an over-reach — read the plan before going wet.
    /root/aj.sh /admin/jobs '{"kind":"repair-placeholder-orgs","dry_run":false}'
    /root/aj.sh /admin/jobs '{"kind":"project","rebuild":false}'
    # (2026-10-14 this step was: backfill-org-name-variants, a whole-corpus walk — job 2491, 3.3 h.)
    # No walk on a re-run: since issue 542 unit 2 the dissolve re-derives its re-bound orgs' name variants itself.
    # Read it in the wet job's summary: "… name variants re-derived over N re-bound org passes".

Then run the Verify (0 rows, or the org is gone). Also read where 46800936's and 46818757's mentions landed.

**Both residuals below are filed as issue 529.** A second finding, also not in this unit: a buyer's PPON on an
`Individual`. On the same framework (46806664,
46819715), a supplier named `Individual` carries `PDQJ-7126-JDHG`. That is UK Research & Innovation's PPON (org
30913706, 3,548 mentions), so the withheld individual binds UKRI. A 452 `wrong` verdict cannot move it, for the same
reason as above (exact triple first). Measure how often an `Individual` (or a placeholder name) carries the PPON of a
differently named org before choosing a rule.

**Residual, not in this unit.** The withheld `Individual`s still share one name-only provisional row, and FTS sends
more of them. `Individual` is FTS's stand-in name for a sole trader, so it does not belong in 511's placeholder list.
A `Placeholder` has no identifier guard, so the identified 31570158 would be dropped with them. As a `Blank` word,
guard 3 would keep an identified `Individual` and drop a bare one. Measure the class first: run
`name-phrase-census` for `individual` (how many, how many identified, which roles).

## Done (2026-10-14)

- **Deployed** e2da1e9 at 22:2x UTC, together with 524 unit 1, 531/535 and 536.
- **Pre-dissolve read.** By identifier, exactly 30914328 and 31548264 hold the two bodies, with 4 + 2 = 6 mentions.
- **Dry run 2488.** It reported 3 condemned orgs and 7 mentions.
  - **Reconciled.** The extra is one stock org in an older condemned class. The latest `org-merge-health` gate census
    (3.8 days old) reads `sequence: 1`, the only non-zero condemned counter, so the extra row is drift from that
    class, not an over-reach of the new rule.
  - 2 + 1 = 3 orgs and 6 + 1 = 7 mentions.
- **Wet run 2489.** 3 dissolved, 7 mentions re-resolved (2 fresh provisionals, 5 reused), 27 party rows, 31 bid-party
  rows, 31 winner rows (2 duplicates removed), 0 skipped, 5 tenders touched.
- **Follow-up jobs.** Project 2490 had nothing pending. Backfill-org-name-variants 2491 started; it is a whole-corpus
  walk for these 7 mentions, could not be cancelled (409), and held the queue for 3.3 h (ok at 01:46 UTC). Issue 542
  made it stoppable and resumable, and its unit 2 removes this runbook step.
- **Verify:** `SELECT count(*) … organization_id IN (30914328, 31548264)` reads 0, and both rows are gone.
- **Where the mentions landed** (bounded reads):

  | notice | mention | now binds |
  | --- | --- | --- |
  | 46800936 | HISCOX INSURANCE COMPANY LIMITED | 2123088 "Hiscox Insurance Company Limited" (GB, provisional) |
  | 46818757 | DOC-Labor GmbH | 31966281, fresh GB provisional |
  | 46806664, 46819715 | four withheld `Individual`s (PBXY and PGYY slots) | 31966280, one fresh GB provisional `Individual` |

  The shared `Individual` row is the documented residual. Issue 529 owns it; its name-phrase census is queued as
  job 2493.
