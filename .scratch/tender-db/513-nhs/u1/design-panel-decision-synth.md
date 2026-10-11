## Decision (unit 2, second) — 2026-10-11

Replaces the withdrawn D1–D4. All three judges ranked design C first. Counts are over distinct orgs (352 SAME orgs, 129 bodies).

**D1. No mint. Consolidate each body by review (C).** A HIGH, members-exact verdict per allowlisted code folds the body's SAME clusters and SAME PPON org into one survivor. The survivor is the PPON org (for RYA, the real code's PPON), or else the largest SAME cluster as a provisional anchor. Each loser gets `org_merge_log` `nhs-body` (308), and the survivor records `GBNHS<code>` as a merged identifier. `normalise_identifier` does not change.
*Why:* orgs go 352 → 173 now → 129, with 0 bodies worse, 0 strays moved, 0 identities minted; a mint holds ~9.5% of a body. The code never binds, so unseen strays cannot bind wrong (B4 admits 'University of Oxford' under RTH).

**D2. Amend 300 §10 in writing:** a reviewed, ODS-confirmed, members-exact body verdict may fold provisional clusters into a canonical row and alias its reviewed keys. Nothing else captures by name.

**D3. A frozen ODS table and check (ingest, pure).**
- The table is B's: 1,053 codes, with RO98, Inactive and five-character codes as rivals, plus predecessor names, RO107/RO172 and a version.
- A's 213-code allowlist decides who may be a survivor.
- The B4 check also refuses non-NHS institutions (university, council, ltd with no NHS role token), and a tie refuses.
- A member the check refuses needs HIGH with cited ODS rename or predecessor evidence; RCX 'Queen Elizabeth Hospital FT' is held.
*Why:* review answers to ODS; the check never binds.

**D4. Alias keys are the members' own 234 keys** (head `name_norm`, GB). A key owned by a standing identifier-NULL row outside the body is denied (`alias_key_owned_elsewhere`), its member held. *Why:* the 234 path matches head names (canonical.rs ~17357), so the alias never binds wider than today.

**D5. The alias reaches its target through `Some(identifier)`, on the synthesised PPON** (org_of → rekey → canon_of → altid alias → mint). Any mint of a ledger target takes the ledger's `keep_name`. *Why:* a survivor a later fold absorbs is followed, not re-minted, and the head name is order-independent.

**D6. Never members:** HOSTED_OR_SITE, NOT_ORG and DIFFERENT holders; minted-identity holders (519's); successor names under the 7 legally-ended codes; generic or consortium names; flagged rehomed or case-moved members. The QKK/QMF cross-assignment is dropped (it fails members ⊆ live), so both stay anchors. Verdicts come from the live listing only: adjudication-rows.json gives TAJ's PPON as 31542073, which is QMM's.

**D7. A scoped `nhs-body` unwind ships with the wet path and is tested before any wet run.** It retires a body's ledger rows and then dissolves the body through 234/E0. *Why:* nothing can undo these 223 merges today.

**D8. Any future forward arm goes in the resolver, never in `normalise_identifier`, and binds to the survivor's merged identifier, never to a twin.** *Why:* the identifier feeds buyer_key and the FTS ocid gate (canonical.rs:14088).

### Units
- **U0** This record and the 300 amendment. u1 corrections: NHS TRUST is 99; uncoded is 47,467; the TAJ id.
- **U1** D3. No behaviour change.
- **U2** The census, a dry `nhs-body` run: every holder uncapped, key ownership, check verdicts, D6 flags, country-less and bare GB-NHS values, non-allowlisted tails, coded share by notice year. Adds `reviewed_holders` and 1-member verdicts. Deploy dry.
- **U3** Adjudicate the 35 holders (reviewer plus challenger) and POST the verdicts. Also post 469 keep verdicts for the QKK, QMF, RXX, RTF, RGT, 0CX and RAL PPON pairs.
- **U4** One deploy, after 491 3(b) leaves canonical.rs and supervisor.rs: the wet path, the ledger-armed alias (boxed), the D7 unwind, the `nhs_body_merged` and `nhs_body_regrown` reports, and the re-resolve read.
- **U5** Dry → capped wet (20) → read → full wet.
- **U6 (conditional)** D8's arm. It opens only if the last-12-month coded share is above 9.5% and S4 shows coded new spellings regrowing. It ships only if its own dry census admits ≤ 0.5% non-SAME names.

### Stop rules
- **S0** No verdict for a body with an unread holder (11 such bodies today). 0 elsewhere-owned keys admitted.
- **S1** SAME agreement ≥ 98%. 0 flagged members without evidence. 0 successor-name members.
- **S2** Parity within max(2%, 5) bodies, otherwise abort before writing.
- **S3** Capped read: reviewed survivors, non-members untouched, survivor mentions = members' sum, losers 308, `?identifier=GBNHS<code>` answers the survivor for 20 codes.
- **S4** On every fold, alias refusals ≤ binds. On every dry run, `nhs_body_regrown` = 0.
- **S5** A store test that rebuilds in notice order yields one org per body. On prod, 3 notices × 3 bodies (TED uncoded, FTS coded, PPON-first) re-resolved through a fresh resolver must agree 100%.
- **S6** No rollback past U4 without the unwind first.

### Handoffs
- **469 U7:** digit-bearing NHS codes only. ODS confirmation and the witness check use D3's table. No letter-only REGISTRIES shape. Survivors are holders via merged identifiers. Keep verdicts deny the cross-body pairs.
- **519:** RO98 sub-ICB codes (incl. 10 Active `00[A-Z]`) and typo codes. Their orgs never join a body. D3's table classifies them, and A's (triple, name_norm) pin is a candidate fix. Lands before or with U7.

### Trade-offs accepted
- No `GBNHS` identity; lookups go through merged identifiers.
- Clusters that never carry the code stay split (E3).
- New coded spellings regrow until the next cohort, and a newcomer makes the verdicts stale.
- 13 anchor bodies stay provisional.
- Review costs ~2× 469 U6.
- Hosted sites stay separate.
- The legally-ended codes have no date fence.
