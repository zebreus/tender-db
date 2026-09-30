# 448 — design (panel of three + judge, 2026-09-30)

Produced by a design workflow: three independent designs (reuse-edges, resolver-first, risk-first), each grounded in
file:line evidence, and a judge who checked the load-bearing claims against the code. The judge picked **risk-first**
and grafted parts of the other two. The build follows this file; the issue carries the status.

## Chosen design

TIER: E2. The pairing is a publisher's statement, never a key. It never enters canonical_key, canon_of or R2 grouping. All three designs agree, and the code agrees:
- crosswalk.rs:338-344 calls the pairing 'E2 evidence for R3, not a key'.
- PPON is a platform registration.
- Issue 447 shows publishers write a sister company's number.
- Design §1 (issue-300 design, line 711) defines E2 as auto-merge under R3's full condition stack, otherwise an edge.

CONSUMER: a new dedicated arm, Db::match_org_altid_pairs(AltIdMergeArgs) -> AltIdMergeReport, in crates/store/src/canonical.rs.
- It runs as match-org-identifiers with rule 'altid'. The dispatch at supervisor.rs:1426 gains 'altid' => Spec::MatchOrgIdentifiersAltId{dry_run,max_groups}. The unknown-rule message lists altid.
- The run_spec arm body is wrapped in Box::pin(async move{..}).await.
- The ledger rule is 'e2-altid'.
- It is not R3. R3's pool is NULL-country rows with a checksum anchor, and adding pairs would shift r3-merge-plan parity.
- It is not an edit to R2, which stays untouched.
- It reuses repoint_org_references, org_all_names, name_key_is_generic, merge_verdict_for, org_merge_verdicts with POST /admin/merge-verdicts, append_change, publish_cursor and refuse_without_org_fk_indexes.

INJECTED FNS (in crates/ingest/src/crosswalk.rs):
- altid_pair_key(scheme, value, country) -> Option<(scheme, key, is_e1)>.
  - It keys only when notice_ids.scheme is exactly 'GB-COH' or 'GB-PPON' (graft from design 1). This keeps other schemes away from the bare-GB strip at crosswalk.rs:360-364.
  - It then calls project::normalise_identifier, then canonical_key_flat, and keeps only GB:coh and GB:ppon.
  - A 6-7 digit COH comes back with is_e1=false. It is counted as pad_side and never paired.
- altid_name_key(name): n3_key, then fold ltd|limited->§ltd, plc->§plc, llp->§llp, lp->§lp, cic->§cic, then drop 'the' and 'and'. It keeps 'uk', 'group' and 'holdings'.
- gb_legal_family(name): Ltd, plc or LLP.
- Also canonical_key_flat, idgate::condemns, consortium_name, match_norm (for the generic wall) and STOPLIST_CAP.
- mention_key: normalise_identifier then canonical_key_flat. The evidence wall keys FTS raw values through this. The R2/R3 wall's two-letter-lead-means-VAT rule (canonical.rs:10934, 13532) sends 'GB-COH-..' to kind vat, and the GB arm returns None for vat. So without mention_key the wall never sees these values.
- Nothing shared changes: family_token, NAME_STOP_TOKENS, name_cores_disjoint, NAME_KEY_EPOCH, R2 and R3 all stay as they are.

HARVEST (every run re-derives from the immutable notice layer):
- FTS notice ids come from a keyset walk of notices_profile for each fts:% profile, in IN_CHUNK windows.
- For each window, read notice_ids rows with field_id='BT-501-Organization-Company' and section_id in ['ORG-','ORG.'). This is a PK range.
- Read the organization_mentions rows (name, country, organization_id) of those sections by PK.
- Group the rows by (notice, section). For each section, collect the E1 COH set and the E1 PPON set.
  - More than one COH or more than one PPON marks the party ambiguous and taints every key in it. This covers the 89 COH->COH pairs.
  - A non-GB party is skipped and counted.
  - Unkeyed schemes (CHC, NHS, UKPRN, MPR, SC) are counted per scheme.
- Build the bipartite COH<->PPON graph over every pair, owned or not.
  - partners = the number of distinct partners per key.
  - The flags are conflict_ppon_multi_coh, conflict_coh_multi_ppon and party_ambiguous.
  - Witness counts are distinct notices. first_coh and first_ppon count which side came at ordinal 0 ('first', not 'primary', because of the parse.rs:203 skip).

TARGETS:
- Walk organizations in id order, R2-preload shape, identifier NOT NULL.
- Keep rows that register_jurisdiction puts in GB and that carry an E1 GB:coh or GB:ppon key. Map each key to its owner ids.
- Classify each pair as one of:
  - already_one;
  - multi_target (a key with two or more owners: a denied family, skipped);
  - no_target_coh, no_target_ppon or no_target_both, with a sample of literals (this answers the '490 neither found' question);
  - both_distinct, which is the issue's Verify figure.

STRUCTURAL GATES (a verdict cannot override these):
- condemns on both literals.
- Consortium veto on every head and satellite name of both orgs.
- gb_legal_family veto, head against head.
- Evidence wall: the mention raw values of both orgs are keyed through mention_key with the two pair keys removed. Any same-scheme disjoint key denies.
- loser_incoherent: the PPON org's mentions carry any other GB key.

VERDICT:
- Lookup: merge_verdict_for('GB','GB:altid','<coh>~<ppon>').
- keep -> denied_verdict.
- A HIGH, unapplied merge whose members equal the live [min,max] -> admitted_verdict. It skips the judgment gates only.
- Anything else -> verdict_stale.

JUDGMENT GATES:
- The conflict flags.
- Positive corroboration: some altid_name_key must be equal across (the head and satellites of the COH org) x (the head and satellites of the PPON org). Otherwise the pair is 'uncorroborated'.
  - Listed in two sub-classes (graft from design 1): 'overlap', the review shape such as Energinet, and 'disjoint', a probable publisher error.
  - name_cores_disjoint is used only to pick the sub-class, never to admit.
- Generic wall: the N2 key of the corroborating name is above STOPLIST_CAP -> denied_generic. There is no hard-scheme exemption.
- The wall needs org_match_keys to be readable, so the supervisor applies R3's refusals:
  - a key build in flight, or epoch drift, refuses both dry and wet;
  - an empty satellite refuses wet only.

SURVIVOR: always the org keyed by the company number (COH). The PPON org is the loser.

PLAN REPORT 'altid-merge-plan':
- Every counter above.
- pairs: the sorted, uncapped list of planned '<coh>~<ppon>' keys, used for set parity.
- plan, denied (with the failing gate), conflicts and no_target_sample listings, in R2's {country, scheme:'GB:altid', key, members:[{org_id, kind, identifier, name}]} shape plus up to 3 witness publication ids, so a listing round-trips into verdict uploads.

WET RUN:
- Needs the stored plan.
- expect_pairs is read from the plan. If more than max(2%, 5) live pairs differ from it, the run aborts before any write.
- Only live ∩ expect merges, in sorted order. Pairs that appeared after the dry run are counted as deferred_unreviewed.
- Merges run 50 pairs per BEGIN IMMEDIATE transaction with foreign keys ON. Each transaction:
  - repoint_org_references(keep = COH org, loser = PPON org);
  - DELETE the loser;
  - INSERT org_merge_log rule 'e2-altid' with flat json_field-readable evidence {scheme:'GB:altid', coh, ppon, coh_literal, ppon_literal, keep_id, loser_id, name_key, witnesses, witness_notices (up to 3, graft from design 2 for a targeted unwind), verdict};
  - append_change for the removed and changed organizations and the touched tenders;
  - stamp the verdict;
  - UPDATE the edge to state='merged' where one exists;
  - COMMIT, TRUNCATE checkpoint, publish_cursor.
- The residual is recorded again afterwards.
- Then an upsert writes open e2-altid/E2 edges for the two-org pairs a gate denied. It uses write_candidate_edges, extracted from scan_org_match_keys (canonical.rs:6834-6905) with the tier as a bound parameter (graft from design 1).
  - evidence: flat JSON {coh, ppon, literals, status = the failed gate, witnesses, partners, first counts}.
  - score: the witness count.
  - It never deletes an edge, so tripwire 6 cannot report SHRUNK.

EDGE READERS (must land before the first E2 row exists):
- census_org_candidate_edges (canonical.rs:19710) currently counts every rule other than 'e3-xlang' as e3_name. It gains e2_altid and e2_altid_merged, and only e3-* edges join components.
- xb_same_name_packet (15490) skips rules that are not e3.
- The DDL comment at 650 is updated.
- The OrgEdgeCensus message prints the e2 count.

PREVENTION (ships in the same deploy as the wet path, before the first wet run):
- A new Db::arm_altid_alias(&mut MentionResolver, AltIdAliasRules{..}), called after mention_resolver at project.rs:1546 and 2280. This is design 2's arming shape: the 7-argument mention_resolver and its roughly 47 call sites stay untouched, and an unarmed resolver behaves byte-identically.
- Preload:
  - read org_merge_log WHERE rule='e2-altid', a scan of a small log;
  - build alias_of: (GB, GB:ppon, P) -> (GB, GB:coh, C);
  - drop pairs that carry a GB:altid keep verdict;
  - poison a PPON that maps to two COHs.
- Binding, in resolve_one_mention, only after the E0 lookup and the canon_hit lookup both miss for an E1 GB:ppon key:
  - the owner is canon_of[alias], and it must not be poisoned;
  - the bind must pass the arm's own corroboration predicate: one shared pure fn altid_corroborates(mention name and variants, owner names) plus the consortium veto, gb_legal_family and the generic wall through the resolver's memo (lenient and counted when the wall is disabled);
  - a bind is never cached in org_of (issue 318);
  - on failure the mention mints as it does today, and the next arm run decides.
- The alias is keyed identity to identity, so it survives the org-id renumbering of a bare rebuild (the rebuild keeps org_merge_log, canonical.rs:6985-7005).
- Counters alias_asked, alias_bound and alias_refused are written through log_diag as '[issue 448]', in both directions.

SCHEDULE: run by an operator, never scheduled at first. After two hand-read runs at 100% precision, consider a weekly dry run as a regrowth tripwire.

NO NEW TABLE OR COLUMN: notice_ids stays the durable source of truth. Option (b), mention-level altids, is deferred to the regrowth reading after the backfill.

## First build unit

UNIT 1: the dry-only planner. It writes nothing and delivers the issue's measurement.

FILES:
(1) crates/ingest/src/crosswalk.rs
- Add altid_pair_key, altid_name_key and gb_legal_family, with unit tests:
  - altid_pair_key_reads_the_fts_literal_forms:
    - GB-COH-03914810 and GB-COH-SC305103 -> GB:coh E1;
    - GB-PPON-PHDQ-2359-NZMP -> GB:ppon PHDQ2359NZMP E1;
    - GB-COH-3914810 -> E2 (pad);
    - scheme GB-CHC, GB-SC, GB-UKPRN or GB-NHS -> None, even for digits-only ids;
    - a GB VAT -> None.
  - altid_name_key_folds_ltd_and_limited_but_keeps_plc_llp_and_uk.
  - gb_legal_family_separates_ltd_plc_llp.

(2) crates/store/src/canonical.rs
- Add AltIdMergeArgs, AltIdMergeReport and Db::match_org_altid_pairs, with harvest, graph, targets, structural gates, verdict consult, judgment gates and listings.
- On wet it returns Err('altid wet path not built (448 unit 2)').
- Re-export from crates/store/src/lib.rs.

(3) New crates/store/tests/altid_merge.rs:
- a_clean_pair_with_agreeing_names_is_planned_and_keeps_the_company_number_org
- ltd_and_limited_corroborate
- a_parent_plc_beside_a_subsidiary_ppon_is_denied
- a_uk_suffixed_sister_name_is_uncorroborated ('Acme UK Ltd' vs 'Acme Ltd', the hole in design 1)
- an_overlapping_sister_name_lists_as_overlap_not_merge (the Energinet shape)
- a_ppon_paired_with_two_company_numbers_is_a_conflict
- a_company_number_with_two_ppons_is_a_conflict
- a_party_listing_two_company_numbers_taints_its_ppon
- a_padded_company_number_is_counted_never_paired
- an_unkeyed_scheme_is_never_keyed
- a_key_with_two_owners_is_multi_target
- a_consortium_named_side_denies
- a_generic_corroborating_name_denies
- the_evidence_wall_keys_fts_raws (a COH org whose mentions also carry a second GB-COH raw is denied, which proves the wall is not blind)
- an_already_unified_pair_plans_nothing
- a_keep_verdict_denies
- a_high_merge_verdict_with_exact_members_admits_an_uncorroborated_pair_but_never_past_structural_gates
- a_verdict_for_another_member_set_is_stale
- the_dry_run_writes_nothing (organizations, org_merge_log and org_candidate_edges counts unchanged)
- the_harvest_seeks_notices_profile_and_the_notice_ids_pk (plan shape)

(4) crates/app/src/supervisor.rs
- Add the Spec::MatchOrgIdentifiersAltId variant and the 'altid' dispatch.
- The Box::pin'd arm applies R3's org_match_keys refusals, records 'altid-merge-plan' on a dry run, and refuses a wet run.
- Tests: an_altid_wet_run_is_refused_until_unit_2 and the_unknown_rule_message_lists_altid. The existing an_execute_without_an_expected_count_is_refused must stay green; it is the stack canary.

GATE AND COMMIT:
- Run ops/check.sh with output redirected to a file and read GATE-EXIT=0. Never pipe it.
- Run git diff on each file, stage only these files, check git show --stat, then push HEAD:main and the handover branch.

VERIFY:
- Deploy, then enqueue {kind:'match-org-identifiers', rule:'altid'} (dry by default).
- Read altid-merge-plan. both_distinct (plan + denied + conflicts, two distinct GB owners) should be about 161 plus whatever the running 2025-07..2026-02 backfill chunk has added. A large gap is diagnosed before unit 2.
- Record on issue 448: the per-gate counts, the overlap/disjoint split, and the breakdown of the 490 no-target pairs (pad_side, unkeyed scheme, non-GB party, condemned, no owner).

LATER UNITS:
- Unit 2: the wet path, the extracted write_candidate_edges, and scoping of the edge readers.
- Unit 3: the resolver alias. It deploys together with unit 2, and no wet run happens before both are live.
- Unit 4: the campaign:
  - hand-read the full dry plan with a reviewer and a challenger;
  - post verdicts under cohort 448-altid-<date>;
  - run a dry re-plan;
  - run wet with max_groups=20, read the ledger, then run wet uncapped, then run project;
  - Verify: both_distinct equals the number of keep-verdict pairs, and after the next daily FTS fold alias_bound > 0.
- Unit 5: docs (CONTEXT.md, the DDL comment) and a follow-up issue for the blind two-letter-lead wall in R2/R3.

## Grafted from the runners-up

- Design 1: key only on notice_ids.scheme exactly GB-COH or GB-PPON, so no other scheme reaches the COH arm's bare-GB strip (crosswalk.rs:360-364).
- Design 1: harvest by a notice_ids PK range on section_id ['ORG-','ORG.') instead of reading every id row of each notice.
- Design 1: explicit classification counters for the 490 'neither found' pairs (pad_side, unkeyed_scheme, non-GB, condemned, no owner) and partners counts that include partners with no standing org.
- Design 1: extract write_candidate_edges out of scan_org_match_keys with the tier as a bound parameter, rather than copying the upsert. It keeps first_seen and state (canonical.rs:6849-6860).
- Design 1: scope the edge readers before any E2 row exists (the census default arm at canonical.rs:19710 and the xb packet union at 15490), with the test that an e2 edge bridging two e3 components leaves the E3 census unchanged.
- Design 1: split the uncorroborated listing into 'overlap' (the review shape, e.g. Energinet) and 'disjoint' (a probable publisher error). name_cores_disjoint is used only as the classifier.
- Design 1: the evidence says 'first' rather than 'primary', because of the ordinal-0 caveat (parse.rs:203 skips a scheme-less primary).
- Design 1 and design 3: the resolver alias maps identity to identity (PPON key to COH key), so it survives the org-id renumbering of a bare rebuild while org_merge_log persists.
- Design 2: arm the alias through a separate Db::arm_altid_alias(&mut MentionResolver, ..) call after mention_resolver. This avoids changing mention_resolver's signature at about 47 call sites, and an unarmed resolver stays byte-identical.
- Design 2: witness notice provenance in the org_merge_log evidence, so one bad publisher statement's merges can be found and unwound.
- Design 2: the refresh-storm analysis of RecordedMention::publishes (canonical.rs:5231) is the documented reason no mention column or table is added now.

## Rejected, and why

- Design 1 (reuse-edges) as winner. Its load-bearing safety claims fail against the code.
(a) Its 'head-vs-head legal-form veto' does nothing for GB names. family_token (crosswalk.rs:787-812) has no ltd/plc/llp, so legal_form_family returns None and never vetoes.
(b) Its positive name rule is core-token-set equality, tokenised like name_cores_disjoint (canonical.rs:23480). That tokenisation drops tokens of 2 characters or fewer, so 'Acme UK Ltd' equals 'Acme Ltd' and auto-merges.
(c) It adds 'plc' and 'llp' to NAME_STOP_TOKENS, so 'Acme plc' equals 'Acme Ltd' and auto-merges. That is exactly the parent/subsidiary publisher-error shape. It also silently changes R2's name gate for every country.
(d) Its hoisted evidence_keys wall is blind to FTS raw values: the two-letter-lead rule makes them kind vat, and the GB arm returns None (canonical.rs:13532-13540).
(e) Its alias binds with no name check, which is more aggressive than the verified merge.
(f) It is weaker than the E2 contract, which asks for R3's exact N2/N3 any-name match and a generic wall. It also needs two new jobs (a scan and a consumer) plus weekly scheduling.
- Design 2 (resolver-first) as winner:
- It reverses the issue's stated default ((a) first, then decide (b) after the backfill).
- It is the largest build, and it touches the hot fold path: the Mention struct and 23 literals, the resolver's per-batch preload, the re-parse clear path, the rebuild, and a new table.
- It carries a corpus-wide refresh-storm risk through RecordedMention::publishes.
- It binds at ingest at E2 before any reviewed dry plan exists.
- It edits the R2 arm in place: alias grouping, the survivor rule and the parity rule.
- It reuses E0's rule that all heads share one N3 key. With no GB forms in family_token, that rule denies every Ltd/Limited pair, and its legal-form agreement check does nothing for GB.
- Its real advantage, preventing the split for the 2,454 COH-only pairs, is a decision for after the backfill's regrowth reading.
- Tier E1, an unconditional PPON<->COH key. It would feed R2 grouping and the resolver's name-blind canonical bind (canonical.rs:9300-9325) from a publisher assertion. It contradicts the GB arm's own contract (crosswalk.rs:338-356) and the class that platform_guids_are_never_a_merge_key guards.
- R3 as the consumer. R3's pool is NULL-country rows rescued by a unique checksum anchor (canonical.rs:13429-13600), while an altid pair is two country-ful GB rows with no checksum. Mixing them would move r3-merge-plan parity and fail its anchor precondition.
- Reusing R2 with an alias key (design 2's consumer). It couples the most important merge arm to FTS-specific grouping, survivor rules and parity. A dedicated arm that shares the primitives costs about the same and leaves R2 byte-identical.
- Changing family_token or NAME_STOP_TOKENS to learn GB legal forms. That would bump NAME_KEY_EPOCH or change R2's decisions on Ltd-to-plc re-registrations that share one company number. The GB fold lives only in altid_name_key and gb_legal_family, and the divergence is documented.
- A separate scan-org-altid writer job whose edges the merge arm then consumes (design 1). That means two parities and an edge-staleness ladder for evidence the arm can re-derive cheaply from immutable notice_ids each run. Edges stay the record of denied pairs, as in design §4.2, but they are not the arm's input.
- Refactoring the resolver's anchor-path corroboration block into a shared helper (design 3's step 7). That is hot-path churn across a different rule (the N2 anchor bar). Instead, one pure altid_corroborates predicate is shared by the altid arm and the alias, so prevention and repair cannot drift.
- Using R2's count parity tolerance of max(2%, 50). On a plan of about 161 pairs that is roughly 31% slack. Set parity with a max(2%, 5) abort, merging only live ∩ reviewed, replaces it.

## Open questions

- Is the issue's 161 reproduced by canonical-key owners, or did the prod count use exact literals? The unit-1 dry run settles this. A gap larger than the backfill's growth is diagnosed before unit 2.
- Should a conflict (a PPON beside two COHs, or a COH beside two PPONs) be overridable by a HIGH merge verdict with exact members, as chosen here, or be a hard deny, as in design 1? Revisit after the first campaign's conflict listing is read.
- Is the strict altid_name_key equality (no legal-form stripping, which is E4 material) too costly in recall, for example 'Acme Services Limited' against 'Acme Services'? Measure the 'overlap' listing and the verdict throughput before widening anything.
- Should the alias ship live, or in design 2's shadow mode (count would_bind first)? It replays only ledger-verified merges, which argues for live. Shadow costs a week of re-mint churn.
- Is option (b), mention-level capture of additional identifiers, needed for the 2,454 COH-only pairs? Decide from the post-backfill regrowth: both_distinct on repeated dry runs, and alias_refused.
- Harvest cost after the full 2021-onward backfill: measure the dry run's wall time once the corpus reaches several hundred thousand FTS notices.
- Scope growth to NHS->PPON (381), UKPRN->PPON (179) and CHC->PPON (108). These are exact-literal targets with no canonical key, so a different target map is needed. They are a separate follow-up.
- Should a distinct-publisher gate (witnesses from at least 2 buyers) be added, as design 3 raised? It is deferred until the first hand-read plan shows whether single-publisher pairs carry the errors.
- Lookup by PPON after a merge: /v1/organizations?identifier=GBPPON... matches organizations.identifier exactly and will miss merged suppliers. Decide whether to file a follow-up.
