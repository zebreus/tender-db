# 575 U2: adversarial critique, code feasibility and parity (2026-10-24)

Scope: `u2/decision.md` §2 (the guard), §3 (placement), §4 (bars), §6 (U3 build spec), and §6.9 where it constrains U3.
I checked them against the code at `af959ff`. No crates/ change has landed between `af959ff` and HEAD `c1d8dc0`.
Read-only: no cargo, no box, no `/v1/sql`. Scratch emulation: `cf35_emul.py` in this session's scratchpad (not
committed).

## Verdict

The U3 spec can be built, and the core claims hold:
- **Caller list.** Complete. `grep -rln 'gated_identifier\|guid_twin_gated_v4\|GuidTwinGatedV4\|NhsBodyCanaryArgs'` over the
  repo finds the 8 files §6.3 names and no others. Every line in §6.3 matches `af959ff`.
- **Stock safety.** `RecordedMention::publishes` compares name, country, raw and scheme (`canonical.rs:15758-15763`
  tree). The resolver returns early on it (`:23161` tree), so an unchanged recorded mention keeps its org.
- **One name everywhere.** The stored `organization_mentions.name` is `m.name` (`canonical.rs:23970-23983` tree). So the
  canary (`:33082` tree), the 530 recurrence (`MentionRow.name`, `guid_twins.rs:3705`), the planner (`TwinView.name`)
  and the fold (`project.rs:6269`, after the renamed-heads loop `:6243`) all see the same name.
- **Fixtures.** The only cf35-shaped value under `crates/*/tests` is the cn-fa fixture, and that fixture is parse-only.
- **Buyer tokens.** 481's `r:` token reads the raw value (`project.rs:11115`), so the overlap of re-emitted values
  survives. The dropped `FR:national:<hex>` token was unique per notice.
- **Coverage numbers reproduce.** My emulation of §2's guard gives the same figures as the decision:
  - 497 judged keys: 335 R / 5 N / 0 W refused, 157 kept;
  - random 60: 49 refused, all R, and all 8 W kept;
  - 16 multi-Tender rows: 15 refused, `CCIRAL` 23126078 kept.

  The guard as written is the guard that was measured. All 50 list entries fold distinct with reason `list`, so R0 #5
  can pass.

One defect is major and six are minor. None blocks the U3 build.

## Defects

### D1 (major): a CF35 list addition after the deploy shows up as 530 reverse drift and blocks 530's wet path

- **Claim.** §5 plans compiled `CF35_UNSAFE_NAMES` additions after the deploy (any W in R2b(a), (b) or (c)), with the
  mentions folded in the meantime "moved back by U4". After such a commit, every uuid_v4 cf35 mention (1/64 of them)
  that was folded onto the pooled identifier-NULL row under the newly listed name gets `guid_twin_gated_v4(..., name) =
  Some`. 530's recurrence arm then counts it as reverse drift.
- **Evidence.**
  - `guid_twins.rs:3720-3731`: every unrefused section with `gated_v4` Some.
  - `:3743-3757`: sections bound to an identifier-NULL row go to `since_candidates`, so `pass` is false.
  - `supervisor.rs:15459-15476`: 530's wet run (step 5b) refuses while the reverse drift since the boundary is non-zero.
  - `supervisor.rs:15170-15179`: 530's restore reads this drift and restores it value by value. That would re-split only
    the uuid_v4 1/64, by the wrong job.
  - §6.9's R3 itself requires "the 530 recurrence quiet". U4 is a one-shot blocked on R2b, 579 and 551, so nothing
    quiets the arm between a list commit and U4, or after U4 for any later addition.
- **Fix.**
  - State the interplay in §5 and §6.8.
  - Make the two-direction rehome a standing arm, run after every list commit and before anyone reads the 530
    recurrence. A one-shot U4 is not enough.
  - Or have the recurrence arm count since-drift on raws with `store::is_cf35` in their own bucket, outside `pass`,
    naming 575's rehome as the remedy.
  - Either way, forbid 530's restore on cf35 raws.

### D2 (minor): guard 3 is not "unaffected", because the census blank metric is not 511's Blank class

- **Claim.** §2 row 1 and §6.3 #2 say guard 3's verdicts cannot move because `cf35_blank_name_mentions` = 0 and a Blank
  party's name is blank or void. Neither holds.
  - The census counts `org_name_norm(name)` empty (`hex_census.rs:1123-1126`). That excludes `N/A`, `néant` and
    `sans objet`.
  - 511's `NotAName::Blank` includes FR whole values with two content words: `sans objet`, `non concerne`, `non
    applicable` (`partyname.rs:627`, `:635`, `:636`). None of those words is in `PREFIX_FUNCTION_WORDS`
    (`project.rs:11397`).
  - Under the guard as written, all three are **refused** (emulated).
- **Consequence.**
  - Guard 3 (`project.rs:5191-5206` at `af959ff`) stops keeping such a party. On any refold its recorded mention is
    retired: the 525 retire keys (`:6133`) act before the resolver, so `publishes` does not protect them. That moves a
    standing mention, which contradicts §6.4.
  - If the party survived, its mention would pool onto a `("sans objet", FR)` row.
  - How many mentions this hits is unmeasured. The name is not among the 497 judged keys or the census's listed keys.
- **Fix.**
  - Add a first reason, `not_a_name`: `crate::partyname::not_a_name(name).is_some()` keeps the key.
  - Add `Sans objet` to R0 #2 and #9.
  - Correct the two texts.

### D3 (minor): the generic test misses article-led kind-of-body names

- **Claim.** §2's normalisation note says the list compares content words, "so `Le tribunal administratif` … meet one
  entry". That holds for test 5 only.
  - Test 3 calls `generic_body_name(words)`, which matches `words.join(" ")` exactly, function words included
    (`project.rs:11307-11327`).
  - So `Le Centre hospitalier`, `La Communauté de communes`, `Le centre communal d'action sociale` and `CCAS de la
    ville` are all refused (emulated). They are not on the 50-entry list either, because the list relies on test 3 for
    the base forms.
- **Evidence in data.** No reader row hits it today. The only article-led one is the seat-named `le centre communal
  d'action sociale de vendôme`. R2b(c) would catch such names after the fact.
- **Fix.** Run test 3 on `words` with leading `PREFIX_FUNCTION_WORDS` dropped. Add `Le Centre hospitalier` to R0 #2.

### D4 (minor): gaps in the census guard-mode spec (§6.5)

- **(a) Extra names.** Store cannot fold `extra_unsafe_names` the way the gate does. `match_norm`, `merge_initials`
  and the legal-form drop live in ingest (`project.rs:10662`), and store does not depend on ingest. 530 solves this by
  passing the extras into the injected fn (`GuidTwinRule = fn(&[TwinView], &GuidKeep, &[String])`,
  `guid_twins.rs:283`). Make it `keep_name: fn(&str, &[String]) -> Option<&'static str>`.
- **(b) The request.** `JobRequest` is `#[serde(default)]` with no `deny_unknown_fields` (`supervisor.rs:1229-1233`),
  and it has no `guard` or `salt` field.
  - As specified, `{"guard": true, "salt": 575002}` is silently dropped and the job runs in U1 mode.
  - The spec must also extend `JobRequest`, the parse arm (`:4946-4967`) and the params string.
  - `Spec` has no `extra_unsafe_names`, so the args field has no source.
- **(c) Tests that will not compile.** They build `Spec::HexCensus { stable_mentions, samples }` without `..`
  (`supervisor.rs:25600`, `:25694`), and the `hex_census.rs` tests build `HexCensusArgs` literals. Add them to the
  site list.
- **(d) The cap.** `refused_keys` is capped at 5,000, but R2b(c) is called exhaustive, and job 2770 already had 4,847
  target keys. Report the total and a `truncated` flag, and fail R2b(c) on truncation.
- **(e) Per mention.** A row with 2–3 mentions can carry different names. Judge each mention, as the gate does, not
  "its cf35 mention".

### D5 (minor): the served-churn bar can read 0 after the churn, and its rationale is incomplete

- **Timing.**
  - `buyer_refused_key_tenders` reads the stored `tenders.procedure_key` "right after the deploy".
  - Any daily or `project` job that runs before it re-plans class Tenders under the new `buyer_key`. The 577 slice
    project jobs are queued.
  - Such a job lifts a refusal before the census looks. The bar would then read 0 after the churn it exists to catch.
- **Rationale.** §3/§4 say `buyer_key` reaches a served key "only for a refused key, which needs `key_shaped = 1`".
  That is wrong. 482's UUID-hub refusal has `key_shaped = 0` and `is_uuid_key`, which is exactly Avenue-Web's random
  v4 BT-04 class, and it labels with the first member's `buyer_key`:
  - `uuid_hub_label`, `canonical.rs:2486-2491` (tree);
  - `refused:{key}:{label}` at `:20310` (tree).

  The `refused:` prefix still catches both, so the bar's test is right.
- **Fix.**
  - Queue the guard census ahead of any `project` or daily job after the deploy. The queue serializes, so this costs no
    wait.
  - Correct the rationale to name 369 and 482.

### D6 (minor): "A hit never folds the mention into a name row" fails for 530-T twins

- **Claim.** A guard-kept cf35 uuid_v4 value under a name that is safe for 530 stays a 530 candidate
  (`guid_twin.rs:287-288`). Committee names are not on `GUID_TWIN_UNSAFE_NAMES` (`guid_twin.rs:74-87`).
- **Consequence.** A twin of two v4 values is refused by 530-T and pooled onto, for example, 1163289 `comité
  consultatif interrégional`, which the readers judged W.
- **Size.** Rare: about (1/64)² per twin group. It is 530's existing behaviour.
- **Fix.** Correct the sentence in §2 and §6.1's module doc.

### D7 (minor): U4's two-direction rehome needs 530's verdict and merge carve-outs

- **Claim.** §6.9 "rehomes every mention whose bound org disagrees with that identity". Without a carve-out, that undoes:
  - reviewed rehomings (`org_mention_rehoming`), which 530 skips (`reverse_drift_verdict_skipped`,
    `guid_twins.rs:3450`; `guid_twin_has_verdict`, `:3527`);
  - merges, where the mention sits on the survivor of a hex row that was merged away.
- **Fix.** Write both carve-outs into §6.9 now, so U3's census guard mode can count them.

## Notes, not defects

- **Line drift.** `project.rs` now also carries another agent's uncommitted edits (+62 lines above `:5193`, all in
  `project_incremental_chunked_observed` and the tests), so the tree is 62 lines below the `af959ff` numbers. §6.3
  names only `canonical.rs` as dirty.
- **Re-emitted values.** For the 46 re-emitted multi-notice rows, the `COUNT(DISTINCT buyer_key)` per key "can only
  fall" only if a re-sent section keeps its name spelling. That is not checked, but it is harmless under `refused:`
  bar D5.
- **The `o:` token.** It moves to the name row and can only widen 481 overlap: 530's F6 argument, which applies
  unchanged.
