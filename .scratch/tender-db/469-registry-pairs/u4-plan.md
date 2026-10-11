# Issue 469 U4 — implementation plan (2026-10-11)

Produced by a read-only Plan agent against HEAD 4910f82; ACCEPTED by the owner with every recommendation in §1 (C1–C14) taken as decided: graph gates first (C1), `self` = witness notices whose buyers include either org (C2), `coherent` = `altid_corroborates` over the designated names, listed only (C3), members ascending with `keep`/`loser` and a `side` tag (C4), `registry_scheme` (C5), the conflict test pinned as cross-register plus `registry-multi-ppon` as the real judgment flag (C6), graph-level triple input `ppon_with_coh` (C8), PPON-side single-owner too (C9), the publisher scheme in the verdict key (C10), `witnesses`/`publishers` (C11), operations.md updated in U4 (C12), the register plan carried forward across wet runs (C14).

# Issue 469 U4: dry register planner, implementation plan

I made no edits and ran no cargo. Line numbers are against HEAD `4910f82`, with a clean tree.

Six points need a decision before coding. Each has a recommendation, and the plan below already assumes the recommended option:

- **Gate order (C1).** The decision's numbering would put the merged-holder triple under the wrong gate. Recommend running the graph gates first.
- **`self` and `coherent` (C2, C3).** The decision never defines either flag.
- **`members` order (C4).** Listing the register org first breaks the verdict round trip for 44 of the 124 sampled pairs. Recommend ascending ids plus `keep`/`loser`.
- **The listing's `scheme` field (C5).** In JSON that key already means `GB:altid`. Recommend `registry_scheme` for the register's scheme.
- **Test name (C6).** `a_ppon_beside_two_ukprns_is_a_conflict` describes something gate 8 denies structurally, not a judgment conflict.
- **Carry-forward on wet runs (C14).** A company-number wet run rewrites the stored plan. Recommend copying the last dry plan's register fields forward.

---

## 0. Ground truth checked

- **Arguments struct:** `AltIdMergeArgs` is at canonical.rs:5527-5592. It has `register_identity` at 5543 and nothing for registers yet.
- **Report struct:** `AltIdMergeReport` is at 5719-5878, and unit 1's register fields are at 5756-5759.
- **Unit 1 types:** `RegistryPairCounts` is at 5662-5689, `RegistrySample` / `RegistryMember` at 5694-5710, and `REGISTRY_SAMPLE_PER_SCHEME` at 5713.
- **Planner:** `match_org_altid_pairs` is at 21466.
- **Harvest:**
  - `RegPair` is defined locally at 21542-21547.
  - `reg_pairs`, `first_bound`, `reg_two_values` and `reg_beside_coh` are declared at 21548-21557.
  - `keyed_ppons` is built at 21678-21684.
  - The `first_bound` inserts are at 21685-21698, and the `reg_pairs` insert at 21715-21723.
- **Owner walks:**
  - `reg_owners` and `reg_ppon_owners` are filled at 21803-21876. Withheld orgs are excluded at 21840, 21851 and 21902.
  - The merged-identifiers walk is at 21878-21920. The decision cites 21857-21894; the lines have drifted slightly but the content is the same.
- **Unit 1 classification loop:** 21978-22040. `both_distinct` is the arm at 22024.
- **Nested helpers:** `mention_sides` 21940, `registry_member` 21962, `mention_rows` 22049, `publications` 22079.
- **Company-number gates:**
  - Gate chain 22295-22347.
  - Verdict key `format!("{coh}~{ppon}")` at 22367; members check `v.members == live` at 22364/22384.
  - Judgment gates 22393-22536.
  - Plan push 22543; the loop ends at 22598.
  - Lines 22599-22603: `report.pairs.sort(); if args.dry_run { return Ok(report); } self.altid_wet(...)`.
- **Wet path:** `altid_wet` reads only `report.pairs` and `args.expect_pairs` (22633-22636).
- **Verdict lookup:** `merge_verdict_for` at 23810 returns a keep from any cohort first, else the newest merge. `record_merge_verdicts` at 23739 does not validate the key, but requires members ascending and distinct (23763).
- **Admin endpoint:** `POST /admin/merge-verdicts` (admin.rs:369, 384) caps keys at 64 characters and requires ascending members.
- **Alias preload:** the keep-verdict read (15073-15096) probes only `{coh}~{ppon}`, so register keys are inert there. The coh survivor's verdict stamp (22763) uses coh keys only.
- **Supervisor:**
  - The altid arm is at supervisor.rs:8938, inside `run_match_identifiers_spec` (8118), which `run_spec` reaches through `off_frame` (13959).
  - The arm is already `Box::pin(async move {...}).await`, closing at 9369.
  - Arguments 9039-9061, listing closure 9080-9106, report JSON 9127-9268 (three `json!` objects merged at 9262-9268 because of the macro recursion limit), `put_report` 9269, counts line 9273-9330 (the "not planned" text is at 9329).
  - A wet run re-records `altid-merge-plan` (9269), so the stored plan is overwritten on wet runs too.
- **`AltIdMergeArgs` constructors to update:**
  - supervisor.rs:9039
  - ingest/tests/fts.rs:629 and :739
  - store/tests/altid_merge.rs:162
  - The other test sites use `..args(20)` and need nothing.
- **Exports:** store/src/lib.rs:64-67.
- **Buyer read precedent:** issue 483's `notices_serving_buyer` (canonical.rs:29908-29919) uses two seeks per notice, `tender_versions_notice` then `tender_version_parties_version`, with `role IN ('buyer','Procedure-Buyer')`. This matches `BUYER_ROLES` at project.rs:7365.

---

## 1. Conflicts and ambiguities (item 9)

**C1. Gate order.**
- The decision numbers the structural gates 1 v2, 2 registry-shape, 3 consortium, 4 legal-form, 5 wall, 6 loser-incoherent, 7 triple, 8 cross-register, 9 single-owner.
- With that order, the merged-holder triple (the PPON is held by a 448 survivor, a company-number org whose mentions carry `GB-COH-…` raws) is attributed to loser-incoherent (22342's rule), or even to the wall, never to "triple". The decision's own test `a_ppon_beside_a_company_number_and_a_charity_is_a_triple` (merged-holder form) would then fail its name.
- Gate 1 (v2 on the org literals) also cannot run before gate 9: a holder seen only through `first_bound` may be a provisional org with no identifier.
- All of these gates are structural, so order changes only attribution, never the planned set.
- **Recommendation:** registry-shape, single-owner, triple, cross-register, then the company-number arm's own order (v2, consortium, legal-form, wall, loser-incoherent). This is also the order `u2_synth.py` used to get S3's 78-of-94 figure.

**C2. `self` is undefined.**
- The best reading: B's "self-statement" signal. The organisation stated its own register number and PPON as the notice's buyer.
- Evidence:
  - Decision line 143: "NHS parties are mostly buyers stating their own codes … lowers typo risk".
  - `p` is defined as distinct buyer orgs of the witness notices.
  - 448-altid-design.md:245 (a "distinct-publisher" gate, witnesses from at least 2 buyers).
- **Recommendation:** `self` = the number of witness notices whose buyer orgs include the register org or the PPON org. It is computed from the same per-notice buyer read as `p`, at no extra cost.
- The Rust field must be `self_stated`, because `self` cannot even be a raw identifier. The JSON key stays `"self"`.

**C3. `coherent` is undefined.**
- B's coherence predicate is not in the codebase, and `altid_corroborates` is already a judgment gate.
- **Recommendation:** `coherent` = `altid_corroborates(altid_keyed(org_all_names(a)), altid_keyed(org_all_names(b)))` returns `Corroborated`, using the designated names (head and satellites, witnesses included).
- It is computed for every two-org candidate, before the gates and regardless of any verdict. It is listed, never gated.
- It differs from the gate, which reads witness-free names. It is informative exactly where the gate did not run (verdict-admitted pairs) or denied (`witness-only`).

**C4. `members` order.**
- The caller asked for "register org first, then PPON org", as `RegistrySample` does.
- A register-pair listing must round-trip into `POST /admin/merge-verdicts` with members as listed, as AltIdListing's contract at 5594-5597 requires. Both the endpoint (admin.rs:384) and `record_merge_verdicts` (23763) require ascending ids, and the planner compares against `[min, max]` (22364).
- In the job-2163 sample, 44 of 124 pairs have the register org's id above the PPON org's, so register-first members would be rejected.
- **Recommendation:** `members` ascending; `keep` = the register org; `loser` = the PPON org; each JSON member tagged `"side": "registry" | "ppon"`.
- If register-first order is mandatory, add a separate ascending `verdict_members`.

**C5. JSON field `scheme` is taken.**
- The listing's round-trip `"scheme"` is `"GB:altid"` (supervisor.rs:9084).
- The decision itself moves the ledger's register scheme to `registry_scheme` because `"scheme"` is taken (decision line 58, canonical.rs:22727).
- **Recommendation:** emit the register's scheme as `"registry_scheme"`, and name the Rust field the same.

**C6. Test name.**
- `a_ppon_beside_two_ukprns_is_a_conflict` describes a PPON beside two identities of one scheme. Gate 8 denies that structurally, and U2 made that binding.
- **Recommendation:** keep the name but pin that the pair is denied as `cross-register` and that a HIGH verdict does not admit it.
- Add `a_ukprn_beside_two_ppons_is_a_conflict_a_high_verdict_admits_it` for the real judgment flag, `registry-multi-ppon`.

**C7. Gate 8 citation.** Gate 8 cites 21710 (`reg_two_values`, per party). U2 supersedes this, and gate 8 is built graph-level (§3.4).

**C8. Gate 7 citation.**
- Gate 7 cites 21713 (`reg_beside_coh`, keyed on (scheme, PPON) and only on parties that also carry a register).
- The decision's words, "a party showed the PPON beside a company number", read as any GB party.
- **Recommendation:** a graph-level `ppon_with_coh` set, a superset of `reg_beside_coh`. It is the same principle as U2's ruling for gate 8. It also keeps any PPON that is in the company-number graph out of the register family.
- Note for U5: `a_ppon_org_two_families_would_fold_merges_neither` then needs a loser org that holds two PPONs, one in each family.

**C9. Gate 9 covers only the register side.**
- **Recommendation:** also require the PPON side to be held exactly, by exactly one non-withheld org, through `reg_ppon_owners`.
- Otherwise a PPON holder seen only through `first_bound` can be planned as the loser: a withheld PPON org, or a company-number org the 448 alias bound without recording the PPON. That is the survivor-reversal hazard (decision line 61).

**C10. What `<scheme>` means in the verdict key.**
- **Recommendation:** the publisher scheme, giving `GB-UKPRN:GBUKPRN10007798~PHDQ2359NZMP`. This matches the `report.registry_pairs` keys, `RegistrySample.scheme` and `registry_scheme`, so `key()` can be built from listing fields alone.
- Keys never collide with `<coh>~<ppon>`, which contains no ':'.
- The longest well-formed key is 8+1+15+1+12 = 37 characters, under admin's 64.

**C11. Field names.** The decision's `k` and `p` should be emitted as `witnesses` and `publishers`, the U5 ledger names, documented as k and p.

**C12. Docs.** The decision lists operations.md:2066 under U5 only. U4 changes the counts line that row quotes ("…issue 469, not planned…") and adds stored fields, so update it in U4.

**C13. The decision's round-trip anchor.** It cites the company-number listing closure (supervisor.rs:9080-9111). The register family needs its own serialiser.

**C14. A wet run overwrites the stored plan (9269).**
- If the register planner is dry-only, as recommended, a company-number wet run would drop the stored `registry_pairs_planned`.
- **Recommendation:** carry those fields forward verbatim (§4.3). The invariant becomes: `registry_pairs_planned` always names the latest dry plan's register set.

**C15. Line drift.** The decision's line references have drifted slightly; content verified.

---

## 2. New types, fields and constants

### 2.1 `AltIdMergeArgs` (canonical.rs, insert after `register_identity`, 5543)

```rust
/// Issue 469 U4: `crosswalk::registry_pair_key` — one register side of a register ↔ PPON
/// pairing (publisher scheme, literal, party country) to `(series, identity)`; keys only
/// GB-UKPRN / CHC / MPR / SC / NIC. `None` is the registry-shape gate.
pub registry_pair_key: fn(&str, &str, Option<&str>) -> Option<(&'static str, String)>,
/// Issue 469 U4: `crosswalk::registry_mention_key` — the register channel of the register
/// graph's evidence wall and loser-incoherent gate. The company-number graph never reads it.
pub registry_mention_key: fn(Option<&str>, &str) -> Option<(&'static str, String)>,
```

### 2.2 New public structs (insert after 5713)

`RegistryGateCounts` (`#[derive(Debug, Default, Clone, PartialEq, Eq)]`), one per scheme:

- `candidates` (= unit 1's `both_distinct`).
- Structural, in gate order: `denied_registry_shape`, `denied_single_owner`, `denied_triple`, `denied_cross_register`, `denied_gate`, `denied_consortium`, `denied_legal_form`, `denied_evidence_wall`, `denied_loser_incoherent`.
- Verdict: `denied_verdict`, `admitted_verdict`, `verdict_stale`.
- Judgment: `conflicts`, `conflict_registry_multi_ppon`, `party_ambiguous`, `denied_form_conflict`, `denied_witness_only`, `uncorroborated_disjoint`, `uncorroborated_overlap`, `denied_generic`.
- `planned`.

Methods:
- `pub fn gates(&self) -> [(&'static str, u64); 15]`: (label, count) in gate order, using exactly the listing's gate labels `registry-shape`, `single-owner`, `triple`, `cross-register`, `gate`, `consortium`, `legal-form`, `evidence-wall`, `loser-incoherent`, `verdict-keep`, `form-conflict`, `witness-only`, `uncorroborated-disjoint`, `uncorroborated-overlap`, `generic`.
- `pub fn denied(&self) -> u64`: the sum of `gates()`.
- `fn bump(&mut self, gate: &str)`: a match from label to field, plus `"plan"`; `debug_assert!` on an unknown label.
- Invariant: `candidates == planned + denied() + conflicts`.

`RegistryPairListing` (`#[derive(Debug, Clone, Default, PartialEq, Eq)]`):

```rust
pub struct RegistryPairListing {
    pub registry_scheme: String,     // `GB-UKPRN` (publisher scheme)
    pub registry: String,            // registry_pair_key's identity; for a registry-shape denial the
                                     // resolver identity, else the value's uppercase alphanumerics
    pub registry_literal: String,    // first published (lowest notice)
    pub ppon: String,                // PPON key (serial)
    pub ppon_literal: String,
    pub gate: String,                // plan | structural gate | verdict-keep | flags joined by '+' | judgment class
    pub keep: Option<i64>,           // the register org (U5's survivor)
    pub loser: Option<i64>,          // the PPON org
    pub members: Vec<RegistryMember>,// both orgs, ASCENDING id (verdict-ready; see C4)
    pub witnesses: u64,              // k
    pub witness_publications: Vec<String>, // first three
    pub publishers: u64,             // p: distinct buyer orgs over ALL witness notices
    pub self_stated: u64,            // JSON "self" (C2)
    pub coherent: bool,              // C3
    pub ppon_partners: u64,          // distinct register values beside the PPON, graph-level
    pub registry_partners: u64,      // distinct PPONs beside this register value
    pub registry_names: Vec<String>, // witness-free names corroboration read, first ALTID_LISTING_NAMES
    pub ppon_names: Vec<String>,
    pub corroborated_by: Option<(String, String)>,
    pub cooccurring: u64,            // listed, never gated
}
impl RegistryPairListing {
    /// The verdict key under (`GB`, `GB:altid`): `<registry scheme>:<identity>~<ppon>`.
    pub fn key(&self) -> String { format!("{}:{}~{}", self.registry_scheme, self.registry, self.ppon) }
}
```

### 2.3 `AltIdMergeReport` (insert after `registry_sample`, 5759)

```rust
// ---- Issue 469 U4: the register graph. Dry runs only; the wet path never reads it.
pub registry_gates: std::collections::BTreeMap<String, RegistryGateCounts>,
/// Distinct keyed PPONs partnered with 2+ register values (any scheme) across every GB party — ungated.
pub registry_cross_register_ppons: u64,
/// …of those, distinct PPONs among candidates the cross-register gate denied (U2's "how many it denies").
pub registry_cross_register_denied_ppons: u64,
/// Distinct register-paired PPONs some GB party published beside a company-number row.
pub registry_triple_ppons: u64,
/// Planned `<scheme>:<identity>~<ppon>` keys, sorted, UNCAPPED — the set U5 will hold its family to.
pub registry_pairs_planned: Vec<String>,
pub registry_plan_listing: Vec<RegistryPairListing>,     // cap: args.plan_listing_cap
pub registry_plan_listing_truncated: bool,
pub registry_denied_listing: Vec<RegistryPairListing>,   // cap: R2_PLAN_LISTING_CAP; includes conflicts
pub registry_denied_listing_truncated: bool,
```

`denied_pairs()` (5882) is left unchanged; it covers company-number pairs only. Also annotate `RegistryPairCounts::ppon_beside_two_values` (5684): "per party; not the cross-register gate's input (U2)".

### 2.4 SQL constants (after 5941; export both in lib.rs:66-67)

```rust
/// Issue 469 U4: one witness notice's buyer orgs, for `publishers` (p) and `self` —
/// issue 483's two seeks (`tender_versions_notice`, then the version's parties).
pub const ALTID_WITNESS_BUYERS_SQL: &str = "SELECT p.organization_id FROM tender_versions v \
     JOIN tender_version_parties p ON p.tender_id = v.tender_id AND p.seq = v.seq \
    WHERE v.caused_by_notice_id = ? AND p.mention_notice_id = v.caused_by_notice_id \
      AND p.role IN ('buyer', 'Procedure-Buyer')";
/// Issue 469 U4: one org's merged identifiers (the triple gate's holder form) —
/// a seek on `organization_merged_identifiers_org`.
pub const ALTID_MERGED_IDS_OF_ORG_SQL: &str =
    "SELECT country, identifier_kind, identifier FROM organization_merged_identifiers WHERE org_id = ?";
```

Leave out DISTINCT and dedupe in Rust. lib.rs:64 also exports `RegistryGateCounts` and `RegistryPairListing`.

### 2.5 Module-level private items (near `AltIdDenied`, ~5978)

- **`struct AltIdRegPair`:** hoisted from 21542 with fields `registry_literal`, `ppon_literal: String` (new), `identity: Option<Identifier>`, `registry_key: Option<(&'static str, String)>` (new), and `witnesses: BTreeSet<i64>`.
- **`struct AltIdRegistryGraph<'g>`:** the planner's input, with:
  - `pairs: &'g BTreeMap<(String,String,String), AltIdRegPair>`
  - `candidates: &'g [(String, String, String, i64, i64)]`: (scheme, value, pk, register org, PPON org)
  - `reg_owners: &'g HashMap<(String,String), Vec<i64>>`
  - `reg_ppon_owners: &'g HashMap<String, Vec<i64>>`
  - `ppon_with_coh: &'g HashSet<String>`
  - `tainted: &'g HashSet<(&'static str, String)>`
- **`async fn publications(...)`:** hoisted verbatim from 22079-22093. Delete the nested copy; the company-number call sites are unchanged.
- **`struct AltIdOrgRow { country: Option<String>, kind: Option<String>, identifier: Option<String>, name: String }`** and `async fn altid_org_row(conn: &Connection, org: i64) -> turso::Result<Option<AltIdOrgRow>>` (by primary key). It serves members, the v2 gate and the triple's own-identifier test.
- **`async fn altid_holds_coh(conn, org, row: Option<&AltIdOrgRow>, key: fn(Option<&str>,&str,&str)->Option<(&'static str,String,bool,bool)>) -> turso::Result<bool>`:** the org's own identifier or any merged identifier keys to `GB:coh`, E1 or pad.
- **`async fn altid_registry_evidence(conn, org, org_country: &str, mention_key, registry_mention_key) -> turso::Result<(Vec<(&'static str, String)>, Vec<(i64, String)>)>`:** the same SQL as `mention_rows`. Each raw is keyed through both channels under `mcountry.unwrap_or(org_country)`. It also returns names with their notice ids.
- **`async fn altid_witness_buyers(conn, notice: i64) -> turso::Result<Vec<i64>>`:** sorted and deduplicated.

---

## 3. Planner changes (canonical.rs)

### 3.1 Harvest edits (unit 1's counts must not move)

- Delete the local `RegPair` (21542-21547) and use `AltIdRegPair`.
- At 21557, add `let mut ppon_with_coh: HashSet<String> = HashSet::new();`.
- At 21678-21684, make `keyed_ppons` carry the literal: `Vec<(&String, i64, &String)>` via `Side::E1(k) => Some((k, *o, l))`. Update the patterns at 21693 (`for (pk, ppon_ordinal, _)`) and 21708 (`for (pk, _, pl)`).
- After 21684 (graph-level triple input; same `!coh.is_empty()` rule as 21712):
  ```rust
  if !coh.is_empty() { ppon_with_coh.extend(keyed_ppons.iter().map(|(pk, _, _)| (*pk).clone())); }
  ```
- In the 21717 constructor, add `ppon_literal: (*pl).clone()` and `registry_key: (args.registry_pair_key)(scheme, literal, country.as_deref())`. This uses the first witness party's country, like `identity`.

### 3.2 Candidate collection (unit 1 loop)

At ~21976, declare `let mut registry_candidates: Vec<(String, String, String, i64, i64)> = Vec::new();`. In the `([a], [b]) =>` arm (22024), before the sampling, push `(scheme.clone(), value.clone(), pk.clone(), *a, *b)`. The candidate set is then exactly unit 1's `both_distinct`.

### 3.3 Call site (replace 22600-22602)

```rust
report.pairs.sort();
if args.dry_run {
    // ---- 7. REGISTRY pairs (issue 469 U4): a separate graph, planned DRY only (U5 lifts
    // this). After the company-number loop, so none of its counts can move; boxed (CLAUDE.md).
    Box::pin(self.plan_registry_pairs(conn, &args, AltIdRegistryGraph {
        pairs: &reg_pairs, candidates: &registry_candidates, reg_owners: &reg_owners,
        reg_ppon_owners: &reg_ppon_owners, ppon_with_coh: &ppon_with_coh, tainted: &tainted,
    }, &mut report)).await?;
    return Ok(report);
}
```

Also add step 7 to the doc comment at 21428-21465.

### 3.4 `async fn plan_registry_pairs(&self, conn: &Connection, args: &AltIdMergeArgs<'_>, g: AltIdRegistryGraph<'_>, report: &mut AltIdMergeReport) -> turso::Result<()>`

**Preamble (graph-level maps, from `g.pairs.keys()`):**
- `ppon_registers: HashMap<&str, BTreeSet<(&str,&str)>>`: pk → {(scheme, value)}. Every scheme counts, including NHS and values with no identity; this matches `u2_synth.py`'s `ppon_values`.
- `register_ppons: HashMap<(&str,&str), BTreeSet<&str>>`: (scheme, value) → {pk}.
- `report.registry_cross_register_ppons` = the number of pks with 2+ partners.
- `report.registry_triple_ppons` = the number of pks in `g.ppon_with_coh`.
- Local caches: `names` (`org_all_names`), `rows`, `evidence`, `buyers_of`, `generic`.

**Per candidate `(scheme, value, pk, a, b)` (stop polled each iteration: `report.stopped = true; return Ok(())`):**

Reads:
1. Both org rows and `org_all_names`.
2. Both orgs' evidence through both channels (`org_country` = row country, default `"GB"`).
3. Witness buyers per witness notice, cached. `publishers` = distinct buyer orgs; `self_stated` = witness notices whose buyers include a or b.
4. `coherent` (C3) and `cooccurring` (both orgs' mention notices intersected, witnesses excluded).
5. Witness-free `side()` names, the 22257-22272 logic with `BTreeSet::contains`.

Fill the listing, including `ppon_partners = ppon_registers[pk].len()` and `registry_partners = register_ppons[(scheme,value)].len()`.

**Structural gates, in order (no verdict overrides any of them):**

```rust
let structural: Option<&'static str> = 'gates: {
    // S1 registry-shape: U3 refused it, or its identity is not the one the resolver mints.
    let Some((series, identity)) = pair.registry_key.as_ref().filter(|(_, id)|
        pair.identity.as_ref().is_some_and(|i| i.kind == "national" && &i.value == id))
    else { break 'gates Some("registry-shape") };
    // S2 single-owner: each side held EXACTLY (own or merged identifier) by one non-withheld
    // org — reg_owners/reg_ppon_owners already exclude withheld. A first_bound-only side fails.
    let held: BTreeSet<i64> = g.reg_owners.get(&("national".into(), identity.clone())).into_iter().flatten().copied().collect();
    let ppon_held: BTreeSet<i64> = g.reg_ppon_owners.get(pk).into_iter().flatten().copied().collect();
    if held != BTreeSet::from([a]) || ppon_held != BTreeSet::from([b]) { break 'gates Some("single-owner") }
    // S3 triple: any GB party showed the PPON beside a company number (graph-level), or the
    // PPON-side holder carries GB:coh of its own or through merged identifiers.
    if g.ppon_with_coh.contains(pk) || altid_holds_coh(conn, b, rows[&b].as_ref(), args.key).await? {
        break 'gates Some("triple") }
    // S4 cross-register (U2's binding requirement): the PPON's WHOLE partner set.
    if ppon_registers[pk.as_str()].len() > 1 { break 'gates Some("cross-register") }
    // S5 v2 gate on both orgs' stored identifiers (22296-22300).
    // S6 consortium over every name of both orgs (22302).
    // S7 legal-form, head against head (22310-22317).
    // S8 evidence wall, BOTH channels, stripping ("GB:ppon", pk) and (series, identity):
    //    two non-empty disjoint key sets in one scheme deny.
    // S9 loser-incoherent: b's stripped keys name any "GB:" scheme, register keys included.
    None
};
```

- On a denial: `bump(gate)`, set `listing.gate`, push to the denied listing.
- For S4, also collect the pk into a set for `registry_cross_register_denied_ppons`.
- Optional hardening: in S3, also test `altid_holds_coh(a)`, a charity org that already absorbed a company number.

**Verdict:**
- Key `listing.key()`, read with `self.merge_verdict_for("GB", "GB:altid", &key)`.
- `keep` → `verdict-keep`.
- `high && !applied && members == [min(a,b), max(a,b)]` → admitted (`admitted_verdict += 1`; skip the judgment gates).
- Otherwise `verdict_stale += 1`.

**Judgment (only when not admitted):**
- Conflict flags:
  - `registry-multi-ppon` if `register_ppons[(scheme,value)].len() > 1`.
  - `party-ambiguous` if `g.tainted.contains(&("GB:ppon", pk.clone()))`.
  - If any flag is set: `conflicts += 1`, bump each flag counter, gate = flags joined by `+`, push to the denied listing.
- Then `altid_corroborates(&cs, &ps, names_agree, legal_family)`:
  - `FormConflict` → `form-conflict`.
  - `Uncorroborated` → `witness-only` / `uncorroborated-disjoint` / `uncorroborated-overlap`, with the classification of 22468-22480.
  - `Corroborated` → the generic wall over the pairs (22499-22520) → `generic`, or set `corroborated_by`.
- Duplicate this control flow rather than refactoring the company-number loop, so that loop stays byte-identical. Only the shared pure predicates are reused.

**Plan:**
- `planned += 1`, push `key` to `registry_pairs_planned`, gate `"plan"`, push to the plan listing (cap `args.plan_listing_cap`, with the truncation flag).
- After the loop, `registry_pairs_planned.sort()`.
- Keys are unique: two spellings sharing a PPON are each other's partners, so S4 denies both. Add a `debug_assert!`.
- Listings fill `witness_publications` only for entries that fit under the cap, as `push_listing` does.

---

## 4. Supervisor (supervisor.rs)

### 4.1 Arguments (9039-9061)
Add `registry_pair_key: ingest::crosswalk::registry_pair_key, registry_mention_key: ingest::crosswalk::registry_mention_key`. Make the same addition at ingest/tests/fts.rs:629 and :739.

### 4.2 JSON
Write a sync free fn `fn altid_registry_json(r: &store::AltIdMergeReport) -> serde_json::Map<String, serde_json::Value>`. Keeping it out of the boxed arm's poll frame follows issue 467's lesson. It emits:
- `registry_pairs_planned`
- `registry_plan` / `registry_plan_truncated`
- `registry_denied` / `registry_denied_truncated`
- `registry_gates` (per scheme, every `RegistryGateCounts` field by name)
- `registry_cross_register_ppons`, `registry_cross_register_denied_ppons`, `registry_triple_ppons`

Each listing entry carries:
- `"country": "GB", "scheme": "GB:altid", "key": l.key()`
- `registry_scheme`, `registry`, `registry_literal`, `ppon`, `ppon_literal`
- `gate`, `keep`, `loser`
- `members` (with `org_id`, `kind`, `identifier`, `name`, `side`)
- `witnesses`, `witness_publications`, `publishers`, `"self": l.self_stated`, `coherent`
- `ppon_partners`, `registry_partners`, `registry_names`, `ppon_names`, `corroborated_by`, `cooccurring`

If `json!` hits the recursion limit, split each entry into two objects. Add the map as a fourth part in the 9263 loop.

### 4.3 Carry-forward on wet runs (C14)
- In the wet branch (9002), keep the parsed `v` as `stored_plan`.
- When building the report, if `!dry_run`, copy the keys in `const ALTID_REGISTRY_PLAN_FIELDS: [&str; 9]` verbatim from `stored_plan` instead of `altid_registry_json(&r)`.
- Nothing interprets them, so the wet path still never reads them for merging.

### 4.4 Counts line (replace 9320-9330)
Per scheme: `"{scheme} {both_distinct} of {pairs} two orgs"`. On a dry run add `" (plan {planned}; denied {n gate, …nonzero from gates() in order}; {conflicts} conflicts; {admitted} verdict-admitted)"`, omitting empty clauses. Join schemes with `", "`.

- Dry prefix: `other registers paired with a PPON (issue 469 U4: planned dry, never merged)`, then a tail `; {cross} PPON(s) partnered with two or more register values across parties ({denied} denied cross-register), {triple} beside a company number`.
- Wet: keep unit 1's counts and say `register plan carried from the last dry plan ({n} pairs)`.

### 4.5 Stack (item 8)
The arm is already boxed (8938), inside a family fn reached through `off_frame`. The budget tripwire `run_spec_futures_stay_inside_their_size_budgets` (18218; `run_match_identifiers_spec` at 3,840 B future and 71 KiB poll) does not see the arm's interior, so the U4 supervisor additions do not move its numbers. On the store side, the planner is its own method and is `Box::pin`'d at its call site (§3.3), which keeps it out of `match_org_altid_pairs`'s state and poll frame. That matters because the store tests poll that future directly on test threads. Both `an_execute_without_an_expected_count_is_refused` (17985) and the size-budget test must stay green. Check supervisor edits with `--features server` (CLAUDE.md).

---

## 5. Tests (crates/store/tests/altid_merge.rs unless noted)

### Setup
- **Miniatures** (after 155):
  - `registry_mention_key`: GB only, not condemned; `minted(raw)` leads with `GBUKPRN|GBCHC|GBMPR|GBSC|GBNIC` and the remainder contains a digit → (series, minted).
  - `registry_pair_key`: scheme → series for the five schemes, then filter on `registry_mention_key`.
  - Both are consistent with the existing `register_identity` miniature (minted value). Add both to `args()` (161-183).
- **Constants:** `UKPRN_A "GB-UKPRN-10007798"`, `UKPRN_B "GB-UKPRN-10001111"`, `UKPRN_C "GB-UKPRN-10002222"`, `CHC_X "GB-CHC-1044521"`, `CHC_Y "GB-CHC-207043"`, `CHC_Z "GB-CHC-202918"`, `MPR_M "GB-MPR-30938R"`, `MPR_N "GB-MPR-7522"`.
- **Helpers:**
  - `fn rk(reg: &str, ppon: &str) -> String` gives `format!("{}:{}~{}", scheme_of(reg), minted(reg), k(ppon))`.
  - `Bed::registry_split(notice, (reg_org, reg, name), (ppon_org, ppon, name))`: two orgs, a register-first party bound to the register org on `notice`, and a PPON-first party bound to the PPON org on `notice+1`.
  - `Bed::registry_verdict(key, members, action, confidence)`.
- **Invariant in `plan_capped` (453-461):** for every scheme in `registry_pairs`, `both_distinct == registry_gates[s].candidates` and `candidates == planned + denied() + conflicts`. Every existing test then checks it.

### Tests named by the decision
1. **`an_ukprn_ppon_pair_is_planned_and_keeps_the_ukprn_org`**
   - Fixture: `registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE"))`. The register org has the higher id.
   - Also insert `tender_versions (7,1,100,0,'pub-100')` and `tender_version_parties (7,1,'buyer',2,100,'ORG-R100')`.
   - Expect:
     - `registry_pairs_planned == [rk(UKPRN_A, PPON_P)]`
     - listing: `keep == Some(2)`, `loser == Some(1)`, members `[1,2]`, `registry == "GBUKPRN10007798"`, `registry_literal == UKPRN_A`, `witnesses == 1`, `publishers == 1`, `self_stated == 1`, `coherent`, `corroborated_by.is_some()`
     - `registry_gates["GB-UKPRN"].planned == 1`
     - company-number side untouched: `pairs` empty, `plan_pairs == 0`
2. **`a_ppon_beside_a_company_number_and_a_charity_is_a_triple`**, three sub-cases on distinct PPONs:
   - (a) Party form: `[CHC_X, PPON_P, COH_A]` bound to the charity org; unit 1's `ppon_beside_coh == 1`.
   - (b) Graph form: `[CHC_Y, PPON_Q]` on notice 200, and `[COH_B, PPON_Q]` on notice 202 bound to a provisional org.
   - (c) Merged-holder form: org 80 `COH_A`-style holder with PPON_R inserted into `organization_merged_identifiers` (the SQL from 2001-2008); org 81 `CHC_Z`; party `[CHC_Z, PPON_R]` bound to 81; no party shows R beside a company number.
   - Expect all three listed as gate `triple`, `denied_triple == 3`. Case (c) must not read loser-incoherent; this pins C1.
3. **`a_ppon_beside_two_registers_is_cross_register`**
   - Fixture: `[MPR_M, P]` on notice 100 bound to org 1, `[UKPRN_A, P]` on notice 200 bound to org 3, `[P]` on notice 300 bound to org 9.
   - Expect: both denied `cross-register`, one in each scheme's counts; `registry_cross_register_ppons == 1`; `ppon_partners == 2`.
   - A HIGH merge verdict on one pair still leaves it denied, with `admitted_verdict == 0`.
4. **`a_ppon_beside_two_ukprns_is_a_conflict`** (C6)
   - Fixture: one party `[UKPRN_A, UKPRN_B, P]` bound to org 1; org 3 holds UKPRN_B; PPON org 2.
   - Expect: unit 1's per-party `ppon_beside_two_values == 1`; both pairs `cross-register`; a HIGH verdict does not admit.
5. **`a_register_side_held_only_by_the_name_bound_org_never_plans`**
   - Fixture: provisional org 30 bound by the `[UKPRN_A, P]` party; no org carries `GBUKPRN10007798`; PPON org 31.
   - Expect: unit 1 counts `both_distinct`; U4 gives `single-owner`, with nothing planned.
6. **`a_withheld_register_owner_never_plans`**
   - Control: planned.
   - Then `identifier_verdict(2, &minted(UKPRN_A), "wrong")` → `single-owner`.
   - Second assertion: a withheld PPON org (verdict on `minted(PPON_P)`) also gives `single-owner`. This pins C9.
7. **`the_register_wall_denies_two_different_ukprns`**
   - Base: org 1 UKPRN_A, org 2 PPON_P, planned.
   - Add notice 103 `[UKPRN_C]` bound to org 2 → `loser-incoherent` (register keys are included).
   - Add notice 102 `[UKPRN_B]` bound to org 1 → `evidence-wall`.
   - The company-number channel keys none of these raws, so only the register channel denies.
8. **`registry_pairs_never_change_the_coh_plan`**
   - Fixture: company-number splits that land planned, `legal-form` denied, and a `ppon-multi-coh` conflict.
   - Take `before = coh_view(plan)`. `coh_view` clones the report, zeroes the harvest counters that move with added notices (`fts_notices`, `party_sections`, `scanned`, `unkeyed_scheme`), clears every `registry_*` field, and formats `{:#?}`.
   - Add register pairs of every outcome: planned UKPRN, a CHC beside the planned pair's own PPON_P (a triple), NHS, cross-register. Assert `coh_view` is equal.
   - Assert the same fixture planned with `registry_pair_key: |_, _, _| None` gives an equal `coh_view`.
   - Wet leg: `wet(&dry.pairs)` merges only the company-number pair; the register orgs remain; every `org_merge_log` row has `$.coh`; `registry_pairs_planned` is empty on the wet report.
9. **`an_nhs_ppon_pair_is_counted_and_never_planned` (1883), updated**
   - Keep every existing assertion.
   - Add: `registry_pairs_planned.is_empty()`; `registry_gates.keys() == ["GB-NHS"]` with `(candidates, denied_registry_shape) == (1, 1)`; the denied listing key `"GB-NHS:GBNHSRR8~…"` with gate `registry-shape`.
   - Also add `registry_pairs_planned.is_empty()` to 1937, plus the Barts gate `registry-shape`.
10. **Supervisor round trip** (new, supervisor.rs ~16445, production rules), `an_altid_dry_run_lists_a_register_pair_that_round_trips_into_a_verdict`:
    - Fixture: `altid_fixture` plus org 3 `GBUKPRN10007798` "Leeds City College", org 4 `GBPPONPBZB4962TVLR` "LEEDS CITY COLLEGE", notices 12/13 with `[GB-UKPRN-10007798, GB-PPON-PBZB-4962-TVLR]` bound to 3 and the PPON-first party bound to 4.
    - Dry: body `registry_pairs_planned == ["GB-UKPRN:GBUKPRN10007798~PBZB4962TVLR"]`; `registry_plan[0]` has `scheme "GB:altid"`, `registry_scheme "GB-UKPRN"`, `keep 3`, member ids `[3,4]`, `"self" 0`, `coherent true`; `pairs` is still `["03914810~PHDQ2359NZMP"]`; msg contains `GB-UKPRN 1 of 1 two orgs (plan 1)`.
    - Build indexes and run wet: only the company-number pair merges; orgs 3 and 4 exist; the residual carries `registry_pairs_planned`.
    - Post a `keep` verdict built from the carried listing's `country`/`scheme`/`key`/`members` via `record_merge_verdicts`, re-run dry → `registry_denied[0].gate == "verdict-keep"`.
11. **`a_consortium_ppon_beside_two_members_on_different_notices_is_cross_register`** (U2)
    - Fixture: `[MPR_M, P]` on notice 100 bound to org 1 "Riverside Group Ltd"; `[MPR_N, P]` on notice 200 bound to org 3 "Thirteen Housing Group Ltd"; `[P]` on notice 300 bound to org 9, also headed "Riverside Group Ltd" so pair 1 would corroborate without gate 8. No consortium word anywhere.
    - Expect: unit 1's `ppon_beside_two_values["GB-MPR"] == 0` (U0's blind spot); `registry_cross_register_ppons == 1`; both pairs `cross-register`; a HIGH merge verdict on pair 2 changes nothing.
    - Add a sub-case: a "Northern Housing Consortium Ltd" PPON org beside ONE member → `consortium`.

### Recommended extras
- `a_ukprn_beside_two_ppons_is_a_conflict_a_high_verdict_admits_it`: `registry-multi-ppon` in the denied listing; a HIGH verdict with members `[1,2]` gives `admitted_verdict == 1` and planned.
- Plan guard (1018-1051):
  - `ALTID_WITNESS_BUYERS_SQL` uses `tender_versions_notice` and `tender_version_parties_version`, with no SCAN.
  - `ALTID_MERGED_IDS_OF_ORG_SQL` uses `organization_merged_identifiers_org`.
  - Check the exact EXPLAIN strings on the first run.
- fts.rs:761: add `dry.registry_pairs_planned.is_empty()`.

---

## 6. Sequencing
1. Store types, constants, argument fields, exports. Hoist `AltIdRegPair` and `publications`.
2. Harvest edits and candidate collection, then the planner method and helpers.
3. Store tests: miniatures, `args()`, the invariant, tests 1-9, 11, the extras, the plan guard.
4. ingest/tests/fts.rs constructors.
5. Supervisor: arguments, carry-forward, JSON helper, counts line, test 10.
6. docs/operations.md:2066 (new fields, counts line, gate order, verdict key) and the issue 469 status line.
7. Wait for the running gate, then focused tests with CLAUDE.md's flagged command, then `ops/check.sh`. Read `GATE-EXIT=`. Run `git diff` per file before staging, because other agents edit canonical.rs and supervisor.rs.

## 7. Risks
- **Harvest edits.** These touch unit 1's code (the `keyed_ppons` tuple, the struct hoist). The existing unit-1 tests and test 8 guard it.
- **Planner errors.** A query error in the register planner fails the whole dry run, so no company-number plan is recorded. Recommend propagating the error (a dry run is re-runnable). The alternative is an `edges_error`-style `registry_error: Option<String>`.
- **Yield.** Graph-level triple and cross-register, PPON-side exactness, and counting NHS or no-identity values as partners all deny more pairs than the per-party reading. The `registry_*_ppons` counters measure the effect.
- **Label meanings.** The `self` and `coherent` definitions are interpretations (C2, C3); revisit them after U6 reads the listing.
- **Query plans.** If turso picks a bad join for the buyer read, it scans `tender_version_parties`. Issue 483 uses the same shape successfully; the plan guard pins it.
- **Macros and stack.** The `json!` recursion limit, and stack (§4.5).
- **U5 handoff.** U5 must remove the `if args.dry_run` guard, add the register wet branch keyed on `registry_pairs_planned`, and build the cross-family loser fixture (C8).

### Critical Files for Implementation
- /home/user/tender-db/crates/store/src/canonical.rs
- /home/user/tender-db/crates/store/tests/altid_merge.rs
- /home/user/tender-db/crates/app/src/supervisor.rs
- /home/user/tender-db/crates/ingest/src/crosswalk.rs (read-only: `registry_pair_key` 1262, `registry_mention_key` 1242)
- /home/user/tender-db/crates/store/src/lib.rs (exports 64-67); also /home/user/tender-db/crates/ingest/tests/fts.rs (629, 739) and /home/user/tender-db/docs/operations.md:2066
