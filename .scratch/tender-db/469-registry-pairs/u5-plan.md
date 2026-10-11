<!-- ACCEPTED by the owner 2026-10-11 with D1–D4 as recommended: D1 every register merge needs an admitting HIGH verdict (register-confirmed pairs are POSTED as HIGH verdicts under cohort prefix `altid-registry-confirmed-`; reviewer + challenger pairs under `altid-registry-<date>`; a planned pair with no verdict is `registry_unconfirmed`, never merged) — U6's runbook is: confirm offline → post verdicts → dry → capped wet (max_groups 20, per family, D2) → read `registry_merged` → full wet; D3 the register org must carry the identity as its OWN identifier; D4 issue 518 class (a) (owner MENTION names in the bar's corroboration, capped 4,096 rows / 256 names, per-fold cache) lands as U5's FIRST commit for both families; 518 class (b) stays separate. Implementation waits for the 491 unit-3 build to leave the tree (both touch canonical.rs). -->

# Issue 469 U5 implementation plan: register wet path and register alias, in one deploy

I made no edits and ran no cargo. Line numbers are against HEAD `af316b1` (the code there is `a19ec52`'s), with a clean tree. Issue 518 was added mid-task; its section is §7. The rest of the plan is unchanged by it.

**Where you need to decide, and what the plan assumes:**
- **D1 — how the store knows a pair is register-confirmed.** It currently has no way to (C1). The plan assumes every register merge needs an admitting HIGH merge verdict. Register-confirmed pairs are posted under the cohort prefix `altid-registry-confirmed-`. A planned pair with no verdict is held as `registry_unconfirmed` and never merged. This decision changes U6's runbook, so please confirm it.
- **D2 — `max_groups` applies to each family separately.**
- **D3 — the register pair's single-owner gate gets stricter.** The register org must carry the identity as its own `organizations.identifier`. Otherwise the alias can never replay the merge (C2).
- **D4 — the alias name fix (issue 518, class a) lands inside U5 and applies to both families** (§7).

---

## 0. Ground truth (current anchors; the decision's line numbers have drifted)

**`crates/store/src/canonical.rs`**
- Partial index `org_merge_log_e2_altid`: 486-490. DDL comment on `e2-altid` edges: 741-749.
- `AltIdMergeArgs`: 5527-5600 (`known_deferred` at 5587-5593, `max_pairs` at 5579-5581).
- `RegistryGateCounts`: 5731-5827. `RegistryPairListing`: 5834-5890.
- `AltIdMergeReport`: 5896-6076. Register block at 5937-5957; wet block at 6029-6063; `contradictory` doc at 6040-6044.
- `ALTID_ALIAS_LEDGER_SQL`: 6106-6109.
- `ALTID_MERGE_TXN_PAIRS`: 6156. `AltIdPlanned`: 6160-6183. `AltIdDenied`: 6186-6190. `AltIdRegPair`: 6194-6204. `AltIdRegistryGraph`: 6208-6223.
- `AltIdAliasRules`: 6439-6457. `AltIdAliasCounts`: 6463-6509. `AltIdAlias`: 6514-6523. `AltIdRefusal`: 6526-6532.
- `withheld_identifier_orgs`: 6830-6850. It is triple-keyed: every org carrying a verdict triple is withheld.
- `MentionResolver`: 9552-9715.
- `mention_resolver` open scan: 15163-15201. The first-row-wins insert is `org_of.entry(..).or_insert(id)` at 15200. Constructor: 15344-15382.
- `arm_altid_alias`: 15412-15481. The preload skips an empty coh at 15428-15433. The keep-key probe `{coh}~{ppon}` is at 15456.
- `altid_alias_bind`: 15516-15652. The name bar is at 15543-15630: owner names come from designated names only (15553-15565); consortium veto 15566; head-against-head legal form 15571-15577; `altid_corroborates` 15578-15588; generic wall 15589-15629.
- Finish diag line `[issue 448]`: 16021-16042.
- Alias call site in `resolve_one_mention`: 16538-16558. Bind branch with the 460 record: 16580-16602. `org_of` is a `&mut` borrow from 16075.
- Harvest owner walk: 22183-22250 (register rows at 22214). Merged-identifiers walk: 22256-22294.
- Unit-1 classification: 22354-22417.
- Company-number verdict block: 22726-22754. Plan push: 22901-22923.
- Step-7 call site with the dry-only guard: 22962-22985.
- `plan_registry_pairs`: 22987-23404.
  - `deny` helper: 23056-23075.
  - Single-owner gate: 23197-23212.
  - Triple gate: 23217 (holder form checked on `b` only).
  - Verdict block: 23281-23297 (the cohort is discarded).
  - Conflict push: 23311-23328.
  - Plan push: 23380-23392.
- `altid_wet`: 23406-23704.
  - Parity: 23432-23454.
  - Contradictory-loser guard: 23456-23474.
  - Transaction loop: 23476-23620.
  - Ledger evidence: 23525-23540.
  - Verdict stamp: 23559-23573.
  - Edge marked merged: 23576-23585.
  - Residual: 23621-23626.
  - Edges: 23637-23703.
- `repoint_org_references`: 4586. It calls `repoint_merged_identifiers`, so the loser's PPON becomes a merged identifier of the keep.
- `record_alias_identifier`: 4892-4917.
- 460 backfill: `MERGED_BACKFILL_ALTID_SQL` at 4931-4933 reads `$.loser_id` and the survivor's country. No change needed.
- `merge_verdict_for`: 24609-24639 (returns `cohort`).

**`crates/app/src/supervisor.rs`**
- `Spec::MatchOrgIdentifiersAltId`: 409. Enqueue: 1527-1549.
- `alias_suffix`: 2639-2667.
- `ALTID_REGISTRY_PLAN_FIELDS`: 2669-2684. `altid_registry_json`: 2693-2787. `altid_registry_summary`: 2795-2845.
- The altid arm (boxed): 9116-9558.
  - Stored-plan read: 9164-9203.
  - Args: 9217-9245.
  - Stopped-before-merge check: 9251-9261.
  - Register carry-forward: 9444-9465.
  - Summary: 9470-9519.
  - Wet message: 9526-9557.
- Tests:
  - `altid_fixture`: 16475.
  - U4 round trip: 16636-16735.
  - `the_alias_suffix_speaks_only_when_asked`: 17545-17565.
  - Stack canary: 18275.
  - Budget test: 18508. `run_match_identifiers_spec` budget: 3,840 B future, 71 KiB poll. `run_project` budget: 9,000 B.

**Other files**
- `crates/ingest/src/project.rs:1987-1997`: `altid_alias_rules()`. Armed at 2035 and 3351.
- `crates/store/tests/altid_merge.rs`:
  - Miniatures at 30-188. `args()` at 190-214. `wet_args` at 217.
  - U4 helpers: `minted` 252, `k` 257, `scheme_of` 262, `rk` 268, `registry_split` 552, `registry_verdict` 567.
  - Alias helpers: `alias_rules` 1564, `ppon_first` 1571, `merge_all` 1592, `fresh` 1600, `resolve_with` 1617, `resolve` 1638.
  - `identifier_verdict`: 1928. `coh_view`: 2392. `registry_pairs_never_change_the_coh_plan`: 2419 (wet leg at 2476-2485).
- `crates/ingest/tests/fts.rs`: `AltIdMergeArgs` constructors at 629 and 741. End-to-end alias test at 597.
- `crates/app/src/v1/sql.rs:139-206`: the allow-list. `org_merge_log`, `org_merge_verdicts` and `org_candidate_edges` are NOT on it. This matters for S4 (§10).

---

## 1. Conflicts between the decision and the code, with recommendations

**C1. "Register-confirmed" has no representation in the store.**
- What the decision says:
  - Line 11: "a wet run [that] needs register confirmation or a HIGH verdict".
  - Line 130: "No pair merges unless it is register-confirmed or carries a HIGH verdict … no unread lane".
  - Line 94: a ledger field `confirmed` (register or verdict).
- What the code does:
  - 448's register check was offline. In `altid_cases.py`, a confirmed plan pair merged on the plan alone, with ledger `verdict` "none".
  - The planner admits only through `merge_verdict_for` (23285).
  - So a corroboration-planned pair is indistinguishable from an unread one, and writing `confirmed: "register"` would be an unverified claim. That is B's rejected unread lane.
- **Recommendation (D1):**
  - A register pair merges only if the live re-plan admitted it by a HIGH, unapplied merge verdict whose members are exact.
  - Register-confirmed pairs are posted as HIGH verdicts under the cohort prefix `pub const ALTID_REGISTRY_CONFIRMED_COHORT_PREFIX: &str = "altid-registry-confirmed-";`.
  - Reviewer-plus-challenger pairs use `altid-registry-<date>` (decision line 103).
  - `confirmed` = `"register"` if the admitting cohort starts with the prefix, else `"verdict"`.
  - A planned pair with no verdict is counted `registry_unconfirmed`, kept in the residual, and never merged.
  - Side benefit: a regrown pair can never be silently re-merged, because its old verdict is stamped applied (`!v.applied` at 23291).
- Alternative considered: a job parameter listing confirmed keys. Rejected because it is unbounded in the persisted Spec and is not checked against members.
- **U6 consequence:** U6 must post the register-confirmed pairs as verdicts, then run dry again, then wet.

**C2. The planner's single-owner gate admits a merge the alias cannot replay.**
- `held` (23202-23208) includes holders through merged identifiers (22281).
- The decision's alias binds only through the exact row (decision line 98).
- A register org holding the identity only as a merged identifier would merge, and every later PPON-first mention would be refused `no owner`. That is the "silent regrowth" risk (decision line 151).
- Withheld co-holders are not a gap: verdicts are triple-wide (6846-6847), so a withheld row with the same triple withholds the register org too.
- **Recommendation (D3):** in single-owner, also require that `org_rows[&a]` is `country == "GB"`, `kind == "national"` and `identifier == identity`. Label stays `single-owner`; no new counter. It moves U4's `denied_single_owner` by the number of merged-only holders, expected to be about 0.

**C3. "The open scan records … triples of withheld orgs" (decision line 98).**
- The open scan's `withheld` set holds org ids (15159). It is empty in a rebuild, where the table starts empty.
- **Recommendation:** read the verdict triples at arm time instead. Factor the first half of `withheld_identifier_orgs` into `async fn withheld_identifier_triples(conn) -> turso::Result<Vec<(String, String, String)>>`. This equals "triples of withheld orgs" in steady state, because verdicts are triple-wide, and it is also right in a rebuild.
- Multi-row triples do come from the open scan, as the decision says.

**C4. Line drift in the decision's anchors.**

| Decision says | Now |
|---|---|
| preload ~5910 | 15425-15434 |
| skip empty coh 15063-15067 | 15428-15433 |
| keep-verdict disarm 15039-15041 | 15456 |
| bar 15130-15150 | 15543-15630 |
| `canon_of`/`org_of` 14785-14836 | 15136-15201 |
| backfill 4931 | 4931-4933 (unchanged) |
| contradictory guard 22657-22673 | 23456-23474 |
| evidence `scheme` 22727 | 23526 |
| arm boxed at supervisor.rs:8938 | 9116 |

**C5. The decision's ledger field list omits `scheme`.**
- Every `e2-altid` row carries `"scheme":"GB:altid"` (decision line 58; 23526).
- **Recommendation:** keep it on register rows. Nothing reads `$.scheme`, and the invariant "on every e2-altid row" stays true.

**C6. `name_key` is always `""` under D1.**
- Verdict admission skips corroboration (23299), as it does for coh admitted rows ("empty when a verdict stood in", 6177-6179).
- Document this. `coherent` stays in the listing.

**C7. Edges for denied register pairs.**
- The decision's U5 text never says to write them, but its docs item names the edges DDL comment (741-749), and the coh path writes them (23637-23703).
- **Recommendation:** write them, rule `e2-altid`, tier E2, evidence without a `coh` key.
- `write_candidate_edges` upserts evidence on `(org_a, org_b, rule)`, so drop a register edge whose org pair already got a coh edge this run.

**C8. `max_groups` across two families (D2).** Recommendation: the cap applies to each family on its own. With no register pairs this is today's cap. See §2.

**C9. The contradictory guard counted across families is not enough.**
- A register keep could be another live pair's loser. The fixture would be contrived: a register keep holding a PPON as a merged identifier, with the loser-incoherent gate passed.
- A coh family merged first would then delete a keep the register family still needs, and `repoint_org_references` into a deleted org fails with foreign keys ON, mid-run.
- **Recommendation:** an org is contested if it is the loser of 2+ live pairs of any family, or the loser of one and the keep of another.
- Inside the coh family alone a keep cannot be a loser: one `organizations.identifier` keys to one scheme. So coh-only runs stay identical.

**C10. S5's "reappears as both_distinct" (decision line 161) is not what regrowth looks like.**
- After a merge, the survivor holds the PPON as a merged identifier. A re-minted PPON org then makes the PPON side `[survivor, new]`, which is `multi_target` (22415), not `both_distinct`.
- **Recommendation:** an instrument `registry_regrown`: ledger-merged register keys whose live class is not `already_one` (§4.6).

**C11. Two existing tests pin U4's interim contract and must be rewritten.**
- U4's supervisor test step 2 (16704-16714, "carries the plan").
- `registry_pairs_never_change_the_coh_plan`'s wet leg (2476-2485, "the wet path plans no register").

**C12. The carry-forward goes away.**
- `ALTID_REGISTRY_PLAN_FIELDS` and the carry-forward (2669-2684, 9447-9457) are replaced by the live register residual.
- The invariant becomes "`registry_pairs_planned` = the reviewed register pairs still unmerged", the same rule as `pairs`.

**C13. The ledger is not readable through `/v1/sql`.**
- See v1/sql.rs:139-206. So the S4 "ledger read" needs the wet report to carry what it wrote (`registry_merged`, §10).

**C14. A stored plan with no `registry_pairs_planned` (pre-U4).**
- It means an empty expected set: every live register pair is deferred, and over 5 aborts the run.
- That is the safe direction, and every stored plan since a19ec52 carries the field.

---

## 2. How a wet run is invoked

Same job, both families, no selector: `{"kind":"match-org-identifiers","rule":"altid","dry_run":false[,"max_groups":N]}`.

**Why one job:**
- The cross-family contradictory guard needs both families' LIVE plans from one writer-held re-plan. Two jobs would each see a stale other family.
- One stored plan, one residual. Parity is held per family in the same run, and either family over tolerance aborts before any write.
- A family selector would add a persisted Spec field (it would need `#[serde(default)]` for queued jobs). It would also create a mode that either skips one family's parity (a weaker run) or is just a cap of 0.
- `max_groups` per family gives S4 its capped register sample (at most 20 ledger rows) whatever the coh backlog is.

**Byte-identical for coh-only stores:**
- The register planner now runs on wet runs, but only reads.
- Coh parity is checked first, with the same message.
- `loser_uses` and `keeps` reduce to the coh family's.
- The transaction body is moved verbatim (§5.4).
- Edges come in the same order, and the coh message and JSON are unchanged.
- The `altid_registry_summary` suffix is empty when `registry_pairs` is empty.
- Only the diag lines change. They are diag, not wet behaviour.

---

## 3. New types, fields and constants (canonical.rs unless noted)

### 3.1 `AltIdMergeArgs` (after `known_deferred`, 5593)

```rust
/// Issue 469 U5, wet runs only: the stored plan's `registry_pairs_planned` — the reviewed
/// register set, held to its OWN max(2%, 5) parity. Empty on a dry run, and when the stored
/// plan carries none (a plan from before U4): every live register pair is then deferred.
pub expect_registry_pairs: Vec<String>,
/// Issue 469 U5, wet runs only: register pairs a previous wet run already deferred (the
/// residual's `registry_deferred_pairs`) — never merged, not drift again.
pub known_registry_deferred: Vec<String>,
```

Also amend the `max_pairs` doc (5579): "…at most this many pairs OF EACH FAMILY".

### 3.2 `RegistryPairListing` (after `corroborated_by`, 5877)

`pub verdict_cohort: Option<String>,` — the cohort of the HIGH merge verdict that admitted the pair. `None` means it was planned by corroboration alone and a wet run holds it.

### 3.3 New public items (after 5890; export them in lib.rs:64-70)

```rust
/// Issue 469 U5: the cohort prefix under which a register-CONFIRMED pair's HIGH merge verdict
/// is posted; the ledger's `confirmed` reads "register" for it, "verdict" for any other cohort.
pub const ALTID_REGISTRY_CONFIRMED_COHORT_PREFIX: &str = "altid-registry-confirmed-";

/// Issue 469 U5: one register merge a wet run committed — the S4 ledger read
/// (`org_merge_log` is not on /v1/sql's allow-list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryMerged {
    pub key: String,
    pub keep: i64,
    pub loser: i64,
    pub verdict_cohort: String,
    /// The exact `org_merge_log.evidence` written.
    pub evidence: String,
}
```

### 3.4 `AltIdMergeReport`

- Re-document 5937-5938: the register graph is planned on BOTH runs since U5, and `registry_pairs_planned` is the LIVE plan on a wet run.
- After `edges_error` (6063):

```rust
// ---- Issue 469 U5: the register family's wet run; all zero or empty on a dry run.
pub registry_expected_pairs: u64,
pub registry_deferred_unreviewed: u64,
pub registry_deferred_pairs: Vec<String>,
pub registry_expected_not_live: u64,
/// Live ∩ expected register pairs whose PPON org is contested (C9), across families.
pub registry_contradictory: u64,
/// Live ∩ expected register pairs no HIGH merge verdict admitted (D1): held, never merged.
pub registry_unconfirmed: u64,
pub registry_merged_pairs: u64,
/// Live ∩ expected register pairs left unmerged (cap, stop, contested, unconfirmed), sorted.
pub registry_residual_pairs: Vec<String>,
/// Of `edges_written`, the register pairs'.
pub registry_edges_written: u64,
pub registry_merged: Vec<RegistryMerged>,
/// S5 (both runs): register keys the ledger merged whose pair stands split again (not
/// `already_one`) — an alias defect.
pub registry_regrown: Vec<String>,
```

- Amend the `contradictory` doc (6040-6044): "…the loser of ANOTHER live pair of either family, or the keep of one (C9)".
- Shared counters (`removed`, `mentions`, `parties`, `bid_parties`, `winners`, `winner_dups`, `tender_changes`, `edges_written`, `edges_merged`) now sum both families. `merged_pairs` and `residual_pairs` stay coh-only.

### 3.5 Private items (near 6185)

```rust
/// Issue 469 U5: one planned register pair as the wet run merges it: the PPON org folds
/// into the register org.
struct AltIdRegPlanned {
    key: String,              // `<registry scheme>:<identity>~<ppon>`
    registry_scheme: String,  // `GB-UKPRN`
    registry_kind: String,    // the identity's kind (`national`, the shape gate's)
    registry: String,         // identity
    registry_literal: String,
    ppon: String,
    ppon_literal: String,
    keep: i64,
    loser: i64,
    keep_literal: String,     // org rows' identifiers (`keep_id`/`loser_id`), "" if none
    loser_literal: String,
    witnesses: u64,
    publishers: u64,
    witness_notices: Vec<i64>,   // first three, ascending
    name_key: String,            // "" when a verdict stood in (always, under D1)
    verdict_cohort: Option<String>,
}

struct AltIdRegDenied { registry_org: i64, ppon_org: i64, listing: RegistryPairListing }

/// What the register planner hands the wet run (both empty on a dry run).
#[derive(Default)]
struct AltIdRegistryWet { planned: Vec<AltIdRegPlanned>, denied: Vec<AltIdRegDenied> }

/// One merge the wet run commits, whichever family planned it (§5.4).
struct AltIdMergeStep<'p> {
    key: &'p str,
    keep: i64,
    loser: i64,
    evidence: String,
    verdict_cohort: Option<&'p str>,
}
```

### 3.6 Alias types (6439-6532)

```rust
/// Issue 469 U5: where the ledger aimed a PPON.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AltIdTarget {
    /// A company-number `GB:coh` key — 448's, found through `canon_of`.
    Coh(String),
    /// (kind, identity): a register identity, bound ONLY through the exact
    /// (`GB`, kind, identity) row, when exactly one row holds it and no wrong/related
    /// verdict names it.
    Registry(String, String),
}
```

`AltIdAlias` (6514):
- `alias_of: HashMap<String, AltIdTarget>`.
- Add `poisoned_registry: HashSet<String>`: poisoned PPONs whose targets include a register one, counted in the register family.
- Add `withheld_triples: HashSet<(Option<String>, String, String)>`: the verdict triples (C3).
- Add `owner_names: HashMap<i64, AltIdOwnerNames>` (§7).

`AltIdRefusal`: add `Shared` (the register target triple stands on 2+ rows or under a withheld verdict).

`AltIdAliasCounts` (6463): add a nested struct, so the 448 fields keep their meaning and coh lines stay identical.

```rust
/// Issue 469 U5: the alias's REGISTER-target activity, apart from 448's company-number one.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AltIdRegistryAliasCounts {
    pub aliases: u64,
    pub kept: u64,
    pub asked: u64,
    pub bound: u64,
    pub bound_unwalled: u64,
    pub refused: u64,
    pub refused_poisoned: u64,
    pub refused_no_owner: u64,
    pub refused_shared: u64,
    pub refused_veto: u64,
    pub refused_names: u64,
    pub refused_generic: u64,
}
// in AltIdAliasCounts:
pub registry: AltIdRegistryAliasCounts,
/// Issue 518 (§7): owners whose mention names the bar read this fold, and of those how
/// many hit the row cap.
pub owner_name_reads: u64,
pub owner_names_capped: u64,
```

- `poisoned` now means "PPONs the ledger aimed at two or more targets of any kind".
- `ledger_rows` counts usable rows of either family.
- `recorded` stays shared.
- Export `AltIdRegistryAliasCounts`.

### 3.7 `MentionResolver` (after `guarded`, 9685)

`shared_triples: HashSet<(Option<String>, String, String)>` — GB `national` triples standing on two or more org rows at open. A register alias target there binds nobody. Fill it in the constructor at 15344.

### 3.8 `AltIdRegistryGraph` (6208)

No new field is needed: D3 reads `org_rows`.

---

## 4. Planner (canonical.rs)

### 4.1 Removing the dry-only guard (replace 22962-22985)

```rust
report.pairs.sort();
// ---- 7. REGISTER pairs (issue 469 U4/U5): their own graph, planned on BOTH runs since U5 — a
// wet run re-plans it live under the writer, as it re-plans the company-number graph. After the
// company-number loop, so none of its counts move; boxed (CLAUDE.md).
let mut registry = AltIdRegistryWet::default();
Box::pin(self.plan_registry_pairs(conn, &args, AltIdRegistryGraph { /* as today */ }, &mut report, &mut registry)).await?;
// A stop inside the register planner leaves a partial register plan: merge nothing.
if args.dry_run || report.stopped {
    return Ok(report);
}
Box::pin(self.altid_wet(conn, &args, &mut report, planned, denied, registry)).await?;
Ok(report)
```

Why this is safe:
- The planner runs before `altid_wet`, so a query error fails the wet run before any write.
- It already reads through `conn`, which is the writer on a wet run. Its pool reads (`org_all_names`, `merge_verdict_for`, `name_key_is_generic`) cannot deadlock (21847-21853).
- **The `report.stopped` return is load-bearing.** Without it, a stop mid-planner would run `altid_wet` with a partial register plan: a spurious parity abort, or a coh merge of a loser that an unplanned register pair also claims.

Also update the doc comments:
- 21828-21833: step 7 runs on both runs; survivor is the company-number org for coh pairs and the register org for register pairs.
- 22987-23017: "Survivor (U5)…".

### 4.2 `plan_registry_pairs` signature

Add a parameter `wet: &mut AltIdRegistryWet`.

### 4.3 Single-owner tightening (D3, at 23210)

```rust
let own_row = org_rows[&a].as_ref().is_some_and(|r| r.country.as_deref() == Some("GB")
    && r.kind.as_deref() == Some("national") && r.identifier.as_deref() == Some(identity.as_str()));
if held != BTreeSet::from([a]) || ppon_held != BTreeSet::from([b]) || !own_row { break 'gates Some("single-owner"); }
```

Doc: "…and the register org carries the identity as its OWN identifier: the alias replays a register merge only through that exact row." Because verdicts are triple-wide, `held == {a}` plus `own_row` means no other row carries the triple.

### 4.4 Capture the verdict cohort (23285-23297)

`let mut admitted_cohort: Option<String> = None;`. On admission, set `admitted_cohort = Some(v.cohort)` and `listing.verdict_cohort = admitted_cohort.clone()`.

### 4.5 Wet collection

- **Every denial** (the five `deny` call sites at 23277, 23287, 23334, 23347, 23375, and the conflict push at 23318): when `!args.dry_run`, push `AltIdRegDenied { registry_org: a, ppon_org: b, listing: listing.clone() }`. Simplest is to extend `deny` with `wet: &mut Vec<AltIdRegDenied>, orgs: Option<(i64, i64)>`.
- **At the plan push (23380):** when `!args.dry_run`, push an `AltIdRegPlanned` with:
  - `registry_kind` = `pair.identity.kind`, `registry` = `identity`;
  - `keep_literal` / `loser_literal` from `org_rows`, `unwrap_or_default`;
  - `witness_notices` = `pair.witnesses.iter().take(3)`;
  - `publishers` = `publishers.len()`;
  - `name_key` = `listing.corroborated_by.map(|(x, _)| (args.name_key)(&x)).unwrap_or_default()`;
  - `verdict_cohort` = `admitted_cohort`.

### 4.6 S5 instrument `registry_regrown` (recommended; unit-1 loop at 22354-22417)

1. Before the loop, read the ledger's register keys once through the alias SQL (§6.1), which is a seek on the partial index: `merged_registry: HashSet<String>` = `format!("{rs}:{registry}~{ppon}")` for rows with a non-empty `registry`.
2. In the loop: `let one = carried || matches!((r.as_slice(), p.as_slice()), ([a], [b]) if a == b);`.
3. If `pair.identity` is `Some(id)`, its key is in `merged_registry`, and `!one`, push the key to `report.registry_regrown`.
4. Sort it after the loop.

---

## 5. `altid_wet` (23406-23704)

New signature:

`async fn altid_wet(&self, conn, args, report, planned: Vec<AltIdPlanned>, denied: Vec<AltIdDenied>, registry: AltIdRegistryWet) -> turso::Result<()>`

### 5.1 Guard order (all before any write)

1. **Coh parity.** 23432-23454 unchanged, including its message, so a coh abort reads as today.
2. **Register parity.** The same shape:
   - live = `report.registry_pairs_planned` (sorted, live);
   - expect = `args.expect_registry_pairs`;
   - known = `args.known_registry_deferred`;
   - `tolerance = (expect.len() / 50).max(5)`.
   - Over tolerance → `Err("altid register parity abort: the live register plan has {} pairs, the stored register plan {} — {drift} differ ({unforeseen} planned since the dry run, {gone} no longer planned), over the max(2%, 5) tolerance of {tolerance}; nothing was written. Re-run the dry plan and review it")`.
   - Then set `registry_expected_pairs`, `registry_deferred_unreviewed`, `registry_deferred_pairs`, `registry_expected_not_live`.
3. **Contested orgs (C9), over the WHOLE live plans of both families, before either is cut** (the 23459-23462 rule: deferred and unconfirmed pairs contest too):
   ```rust
   let mut loser_uses: HashMap<i64, usize> = HashMap::new();
   for l in planned.iter().map(|p| p.loser).chain(registry.planned.iter().map(|p| p.loser)) { *loser_uses.entry(l).or_default() += 1; }
   let keeps: HashSet<i64> = planned.iter().map(|p| p.keep).chain(registry.planned.iter().map(|p| p.keep)).collect();
   let contested = |loser: i64| loser_uses[&loser] > 1 || keeps.contains(&loser);
   ```
4. **Coh family.** `retain(expect)`, sort by key, then `partition(|p| !contested(p.loser))` gives `(merge, contradictory)`.
5. **Register family.**
   - `retain(expect_registry)`, sort by key.
   - Partition contested out to `registry_contradictory`.
   - Partition `verdict_cohort.is_none()` out to `registry_unconfirmed` (D1).
   - The rest is `reg_merge`.
6. **Caps.** `cap = args.max_pairs.unwrap_or(u64::MAX)`, applied per family.

### 5.2 Merge

- `let mut touched = BTreeSet::new();`
- Coh: `let done = Box::pin(self.altid_merge_steps(conn, args, &coh_steps, cap, &mut touched, report)).await?;` then `report.merged_pairs = done`.
- If `!report.stopped`: the register steps the same way, then `report.registry_merged_pairs = rdone`.
- Then:
  - `registry_merged` = `reg_merge[..rdone]` zipped with their evidence and cohort;
  - `report.tender_changes = touched.len()`;
  - coh residual exactly as 23622-23626;
  - register residual = `reg_merge[rdone..]` + contradictory + unconfirmed keys, sorted;
  - the closing publish if `removed > 0`;
  - `if report.stopped { return Ok(()) }`.

### 5.3 Ledger evidence (flat, no `coh` key, so the 448-era preload's `!coh.is_empty()` drops it)

```json
{"scheme":"GB:altid","registry_scheme":"GB-UKPRN","registry_kind":"national","registry":"GBUKPRN10007798",
 "registry_literal":"GB-UKPRN-10007798","ppon":"PHDQ2359NZMP","ppon_literal":"GB-PPON-PHDQ-2359-NZMP",
 "keep_id":"GBUKPRN10007798","loser_id":"GBPPONPHDQ2359NZMP","name_key":"","witnesses":1,"publishers":1,
 "witness_notices":[100],"verdict":"admitted","confirmed":"register"}
```

- Built by `fn altid_registry_ledger_evidence(p: &AltIdRegPlanned) -> String`, with every string through `altid_json_esc`.
- `verdict` = `"admitted"`, because `verdict_cohort` is always `Some` under D1.
- `confirmed` = `register` or `verdict` per C1.
- Do not name it `altid_registry_evidence`; that name is taken (6300).
- The coh evidence moves verbatim (23525-23540) into `fn altid_coh_ledger_evidence(p: &AltIdPlanned) -> String`.

### 5.4 Shared transaction helper

`async fn altid_merge_steps(&self, conn: &Connection, args: &AltIdMergeArgs<'_>, steps: &[AltIdMergeStep<'_>], cap: u64, touched: &mut BTreeSet<i64>, report: &mut AltIdMergeReport) -> turso::Result<u64>`

- The body of 23483-23620 moved verbatim, with a local `merged` counter in place of `report.merged_pairs`.
- Per step, unchanged: the tender read, `repoint_org_references(.., "e2-altid")`, the DELETE, the ledger INSERT with `step.evidence`, change events, the verdict stamp `WHERE … key = step.key AND cohort = step.verdict_cohort`, and `UPDATE org_candidate_edges SET state='merged'`.
- Stop is polled per chunk, so an empty step list polls nothing (keeps `a_stop_between_transactions_leaves_the_committed_prefix` arithmetic).
- One transaction discipline for both families. The SQL sequence is identical, and the coh wet tests (1177-1559) pin it.
- If you prefer, duplicate the loop for the register family instead. I recommend the helper: the commit/rollback/publish handling is the riskiest code to copy.

### 5.5 Edges (23637-23703)

- `folded` = coh merged losers ∪ register merged losers.
- Rows: the coh rows exactly as today (filtered on `ppon_org`), then register rows. A register row is skipped when either org is folded, or when its `(min, max)` was already written by the coh rows (C7).
- Register evidence:
  `{"registry_scheme":…,"registry":…,"registry_literal":…,"ppon":…,"ppon_literal":…,"registry_org":a,"ppon_org":b,"status":"<gate>","witnesses":k,"publishers":p,"registry_partners":n,"ppon_partners":m}`
- Score = witnesses.
- `registry_edges_written` = `written.saturating_sub(coh_rows.len())`.

---

## 6. The register alias (canonical.rs)

### 6.1 The ledger SQL (6108)

```rust
pub const ALTID_ALIAS_LEDGER_SQL: &str = "SELECT json_extract(evidence, '$.coh'), json_extract(evidence, '$.ppon'), \
     json_extract(evidence, '$.registry_scheme'), json_extract(evidence, '$.registry_kind'), \
     json_extract(evidence, '$.registry') FROM org_merge_log WHERE rule = 'e2-altid'";
```

The WHERE is unchanged, so it still seeks `org_merge_log_e2_altid`. The guard at altid_merge.rs:1153-1154 stays as written.

### 6.2 The open scan (15200)

Replace the first-row-wins insert:

```rust
let triple = (country, kind, value);
match org_of.entry(triple) {
    Entry::Occupied(e) => {
        if e.key().0.as_deref() == Some("GB") && e.key().1 == "national" {
            shared_triples.insert(e.key().clone());
        }
    }
    Entry::Vacant(e) => { e.insert(id); }
}
```

- `keyed` is computed above it, from borrows, so moving the triple is fine.
- In-run mints never duplicate a GB national triple: `org_of` guards the mint, and the anchor path is country-less.
- Log the set's size in the `[issue 452]` open line, or a new `[issue 469]` line.

### 6.3 `arm_altid_alias` (15412-15481)

- **Rows:** `(ppon, target, keep_key)`:
  - coh non-empty → `Coh(coh)`, keep key `{coh}~{ppon}`. Coh first, so old rows behave exactly as today.
  - else `registry_scheme`, `registry_kind` and `registry` all non-empty → `Registry(kind, registry)`, keep key `{rs}:{registry}~{ppon}`.
  - otherwise skip, as today.
- **Keeps:** a kept row bumps `kept` (coh) or `registry.kept`.
- **Targets:** `HashMap<String, BTreeSet<AltIdTarget>>`. One target → `alias_of`. Two or more → `poisoned`, plus `poisoned_registry` if any is `Registry`.
- **Counts:** `aliases` = Coh targets; `registry.aliases` = Registry targets.
- **Withheld triples:** `alias.withheld_triples` from `withheld_identifier_triples` (C3). Country `""` maps to `None`, as at 15244.
- **Diag line (15473), zeros included:**
  "… {} PPON key(s) aliased to a company number and {} to a register identity (issue 469) from {} e2-altid ledger row(s); {} pair(s) dropped by a keep verdict ({} register), {} PPON(s) poisoned (merged into two or more targets, never aliased); {} register target(s) stand on two or more rows or under a wrong/related verdict and bind nobody".

### 6.4 `altid_alias_bind` (15516)

New parameters: `org_of: &HashMap<(Option<String>, String, String), i64>` and `shared_triples: &HashSet<(Option<String>, String, String)>`. Restructure `'bar`:

```rust
let Some(target) = target else { break 'bar Err(AltIdRefusal::Poisoned) };
let ppon_key = ("GB".to_owned(), "GB:ppon", ppon.clone());
if resolver_poisoned.contains(&ppon_key) { break 'bar Err(AltIdRefusal::Poisoned) }
let owner = match &target {
    AltIdTarget::Coh(coh) => {            // 448, unchanged in effect
        let coh_key = ("GB".to_owned(), "GB:coh", coh.clone());
        if resolver_poisoned.contains(&coh_key) { break 'bar Err(AltIdRefusal::Poisoned) }
        let Some(&o) = canon_of.get(&coh_key) else { break 'bar Err(AltIdRefusal::NoOwner) }; o
    }
    AltIdTarget::Registry(kind, identity) => {
        let triple = (Some("GB".to_owned()), kind.clone(), identity.clone());
        if shared_triples.contains(&triple) || alias.withheld_triples.contains(&triple) {
            break 'bar Err(AltIdRefusal::Shared)
        }
        let Some(&o) = org_of.get(&triple) else { break 'bar Err(AltIdRefusal::NoOwner) }; o
    }
};
// …then 448's bar, shared by both target kinds: consortium, head against head,
// altid_corroborates (owner names per §7), generic wall.
```

- A shared or withheld triple is refused before `org_of` is consulted. That fixes "first-row-wins, includes withheld" (decision line 62) for the register alias.
- **Counting:** `let registry = matches!(target, Some(AltIdTarget::Registry(..))) || (target.is_none() && alias.poisoned_registry.contains(&ppon));` decides whether `asked`/`bound`/`bound_unwalled`/`refused`/causes go to `counts.registry.*` or `counts.*`.
- **The 460 record on bind (16594-16601)** is target-agnostic and applies unchanged.

### 6.5 Call site (16543-16556)

```rust
Box::pin(self.altid_alias_bind(conn, m, alias, &resolver.canon_of, &*org_of, &resolver.shared_triples,
    &resolver.poisoned, &mut resolver.generic_memo, resolver.hard_scheme.is_some(), resolver.stoplist_cap)).await?
```

- `&*org_of` reborrows the live `&mut` (16075) as shared for the call. The fields are disjoint and NLL allows it.
- Box it to keep `resolve_one_mention`'s future flat. Precedent: 16198.

### 6.6 Finish diag line (16026)

Add "; issue 469 register targets: {} asked, {} bound ({} unwalled), {} refused ({} poisoned, {} no owner, {} shared/withheld, {} veto, {} names, {} generic); owner names read for {} owner(s), {} capped (issue 518)".

### 6.7 `ingest/src/project.rs`

No change for the register alias itself:
- The target triple comes from the ledger. The planner's shape gate (23190-23196) requires `registry_pair_key`'s identity to equal `normalise_identifier`'s, so the exact lookup finds rows the same normaliser minted.
- The bar reuses the existing rules.
- The only edit is §7's `owner_mention_cap` at 1988-1996.
- A future normaliser change (issue 514) makes the alias refuse `no owner`, which is the safe direction.

---

## 7. The alias's owner names (issue 518): recommendation

**The defect.** The bar compares the asked mention's names against the owner's designated names only (15553-15565). The planner corroborates on mention names: witness-free `side()` names, from `mention_rows` 22426-22451 for coh and `altid_registry_evidence` 6300 for register. After a merge the owner holds both former orgs' mentions, so a PPON-first spelling the planner accepted (Caterlink) is refused, and a new PPON org is minted. Measured: 168 of 2,294 asks, 7.3%.

**The fix (518 unit 2, class a):**
- Owner names for corroboration = designated (head first, then satellites) ∪ DISTINCT non-empty names of the owner's mentions, read once per owner per fold.
- The consortium veto and head-against-head stay on designated names. They are not loosened, and they mirror the planner's own consortium and legal-form gates, which read `org_all_names`.
- Only `altid_corroborates`' owner side widens.
- Its form check can only become stricter when owner forms are added, because disjointness needs both sides non-empty. That matches the planner.

**Is it the same set the planner uses?** Nearly.
- The planner, re-run on a regrown pair, reads the owner's mention names minus that pair's witnesses, and they now include the merged-in PPON org's.
- The alias reads all of the owner's mention names, original witnesses included. That is looser only for names that appear solely on the original pair's witness notices, which a reviewed merge already folded into this org.
- Reading the stored witness notices to exclude them is possible (the ledger keeps three), but not worth it.

**Implementation:**

```rust
/// Issue 518: one owner's names as the bar reads them, cached per fold.
struct AltIdOwnerNames { designated: Vec<String>, mentioned: Vec<String> }

/// Issue 518: an owner's mention names, newest first, capped — a reverse walk of
/// `organization_mentions_org` (no sorter: pinned by the plan guard).
pub const ALTID_ALIAS_OWNER_MENTION_NAMES_SQL: &str =
    "SELECT name FROM organization_mentions WHERE organization_id = ? ORDER BY rowid DESC LIMIT ?";
```

- Bind `LIMIT ?` to `rules.owner_mention_cap`.
- Add `pub owner_mention_cap: usize` to `AltIdAliasRules` (6439). Production value: `pub const ALTID_ALIAS_OWNER_MENTIONS: usize = 4_096;`. It is injectable so a test can show the cut. Set it in `project.rs:1995` and in the test `alias_rules()` (altid_merge.rs:1565).
- Dedupe in Rust and keep at most 256 distinct names, which bounds `altid_agreeing`'s cross product and memory.
- Bump `owner_name_reads`, and `owner_names_capped` when the row cap was hit.
- Cache in `AltIdAlias::owner_names` on the writer `conn`, as the existing owner read does: an owner may be this batch's mint.
- Within a fold the cache misses names recorded later in the fold. That only makes the bar stricter.
- Why newest first: merged-in PPON-org mentions keep their original rowids, and PPON-era notices (2025+) have high ones.
- **If turso's EXPLAIN shows a temp B-tree for the `ORDER BY`, drop it.** A sorter would read every row of the owner before the LIMIT. The guard must catch this.

**Cost on prod:**
- About 28 asks per fold (2,294 over 82 folds). Each distinct owner pays one capped index walk plus row fetches, at most 4,096 rows: tens of ms for a large owner, a few ms for a typical one. Worst case per daily fold is about 28 × 4,096 ≈ 115k row reads on the writer; realistically far fewer.
- A from-archive rebuild asks once per merged PPON's first PPON-first mention: at most about 4,900 owners, each capped. Owners are only partly populated then, which is cheaper.
- Memory is at most (distinct owners asked) × 256 names.
- Before settling the cap, measure: bounded `/v1/sql` `SELECT COUNT(*) FROM organization_mentions WHERE organization_id = ?` (index seek) for about 20 keeps taken from the stored plan's listings.

**Land it with U5, as its first commit (518 unit 2a), applied to both families.**
- The decision gives the register alias "448's bar". The bar is one code path for both target kinds, so fixing it for one family fixes both, and splitting it means two bars, so a merge and its replay would read agreement differently.
- Shipping the register alias on the unfixed bar inherits roughly 7% regrowth from day one (518's point 3), and every refusal is a regrowth S5 has to read.
- The change touches only the alias. The planner already reads mention names, so no dry count moves.
- Landing it first gives its own coh regression test, and its gate run isolates it.
- Keep 518's class (b), the space-insensitive `altid_keys_agree` (`cater link` = `caterlink`), **separate**. It changes `ingest::crosswalk`, moves the planner's coh counts, and needs 518 unit 1's measurement.
- Optional and cheap: 518 unit 1's sampled refusal line — `[issue 448] alias refused <cause>: mention '<name>' vs owner '<head>' (PPON <key>, target <coh|registry key>)`, first 20 per fold. It gives S5 its diagnosis.

---

## 8. Supervisor (supervisor.rs; check with `--features server`)

1. **Stored plan (9164-9203).**
   - Also read `expect_registry` from `v["registry_pairs_planned"]`. Absent means empty (C14); a non-string entry is an error, like `pairs`.
   - If `residual_of_wet_run`, read `known_registry_deferred` from `v["registry_deferred_pairs"]`.
2. **Args (9217-9245):** add `expect_registry_pairs: expect_registry, known_registry_deferred`. Make the same additions at fts.rs:629 and 741 (`Vec::new()`).
3. **Stopped check (9251):** `r.stopped && r.merged_pairs == 0 && r.registry_merged_pairs == 0`.
4. **Delete the carry-forward:** `ALTID_REGISTRY_PLAN_FIELDS` (2669-2684) and 9447-9457.
5. **`fn altid_registry_json(r: &store::AltIdMergeReport, dry_run: bool) -> Map`:**
   - On wet, `registry_pairs_planned` = `r.registry_residual_pairs` and `registry_plan` = the live listing filtered to residual keys. The other plan fields (`registry_denied`, `registry_gates`, …) come from the live plan, as coh's do.
   - Add the listing key `"verdict_cohort"`.
   - Add the U5 fields:
     - `registry_live_plan_pairs` (= `r.registry_pairs_planned.len()`)
     - `registry_expected_pairs`
     - `registry_deferred_unreviewed`
     - `registry_deferred_pairs`
     - `registry_expected_not_live`
     - `registry_contradictory`
     - `registry_unconfirmed`
     - `registry_merged_this_run`
     - `registry_edges_written`
     - `registry_merged`: `[{key, keep, loser, verdict_cohort, evidence: <parsed JSON>}]`
     - `registry_regrown`
   - Update the `debug_assert` count. Keep it a sync free function (issue 467).
6. **`altid_registry_summary(r)`:**
   - Same clauses on dry and wet, since a wet run now plans live.
   - Prefix: `other registers paired with a PPON (issue 469: merged only under a HIGH merge verdict)`.
   - Append `; {n} merged register pair(s) split again (S5)` when `registry_regrown` is non-empty.
7. **`fn altid_registry_wet_clause(r) -> String`:**
   - Empty when `registry_expected_pairs == 0 && registry_pairs_planned.is_empty()`.
   - Otherwise: `; register (issue 469 U5): merged {m} of {live∩expected} reviewed live pairs ({u} awaiting a HIGH verdict, {c} contradictory), held against {e} stored: {d} deferred as unreviewed, {g} no longer planned; {w} register edges written; residual {r} register pairs re-recorded`.
   - Append it to `merged` (9526).
8. **The stored residual after a register wet run:**
   - `residual_of_wet_run: true`;
   - `pairs` / `deferred_pairs`: coh, unchanged;
   - `registry_pairs_planned`: reviewed register pairs still unmerged (cap, stop, contested, unconfirmed), sorted;
   - `registry_deferred_pairs`: the live keys the stored set lacked;
   - `registry_plan`: those entries only, each with `verdict_cohort`;
   - `registry_denied` and `registry_gates`: live;
   - `registry_merged`: this run's ledger rows verbatim.
   - A continuation is held against that residual and `registry_deferred_pairs`.
9. **`alias_suffix` (2639):**
   - Return `""` only if `a.asked == 0 && a.registry.asked == 0`.
   - Emit the 448 clause only if `a.asked > 0`, unchanged.
   - Append `; issue-469 register alias asked A bound B[ (U with the generic wall unable to answer)] refused R (poisoned …, no owner …, shared …, veto …, names …, generic …)` when `a.registry.asked > 0`.
10. **Spec doc (405-409)** and the enqueue comment: wet merges the reviewed `pairs` and the verdict-admitted `registry_pairs_planned`, at most `max_groups` of EACH family.

---

## 9. Docs

- **docs/operations.md:2066 (the altid row):**
  - Replace "never merged: no wet run reads it until U5" and "A wet run plans no register … carried" with the wet register branch: D1 with the cohort prefix, per-family parity, the cross-family contest (C9), the per-family cap, the ledger evidence fields (no `coh`), register edges, the residual fields, `registry_merged`, `registry_regrown`.
  - Add the D3 sentence to `single-owner`.
  - Add a "register alias" paragraph:
    - the ledger's register rows map PPON → (kind, identity);
    - binding goes through the exact (GB, kind, identity) row only when one row holds it and no wrong/related verdict names it (refused `shared`);
    - a PPON aimed at two targets of any kind is poisoned;
    - the bar is 448's, with the §7 owner names;
    - counters, and the 460 record.
- **operations.md:812 and 837-842:** the `issue-469 register alias` clause and how to read `shared`; the issue 518 note on owner names.
- **canonical.rs:**
  - 741-749 (edges): "…a company-number/PPON pair — or (issue 469 U5) a register/PPON pair, evidence `registry_scheme`/`registry` in place of `coh` — …".
  - 486-489: the preload reads both families.
  - 480 rule comment, and 4919-4925: `e2-altid` covers both families; `loser_id` is still the PPON org's identifier.

---

## 10. Stop-rule support

**S4 (capped run, then a ledger read):**
1. Run `{"kind":"match-org-identifiers","rule":"altid","dry_run":false,"max_groups":20}`. That gives at most 20 register merges, plus at most 20 coh.
2. Bounded reads:
   - `GET /admin/reports/altid-merge-plan` → `registry_merged`, each entry the exact `org_merge_log.evidence` with keep, loser and cohort. This is the read, because the ledger is not on `/v1/sql`'s allow-list.
   - `GET /admin/reports/altid-merge-plan/previous` → the reviewed dry plan's `registry_plan` entries for those keys: names, witnesses, publications, members.
   - `GET /admin/case-reviews?table=merge&cohort=altid-registry-confirmed-<date>` → applied stamps (`applied_at`, `applied_action` "merged 1 row(s) into <keep>", `job_id`).
   - `/v1/sql`, on the team lead's word per read: `SELECT id, identifier, name FROM organizations WHERE id IN (…keeps…)` (primary-key seeks), and `SELECT notice_id, name, raw_identifier FROM organization_mentions WHERE organization_id = ? LIMIT 50` (index seek).
3. If one merge is wrong: post a `keep` (the alias disarms at the next fold, `registry.kept`), then unwind by ADR 0003 (delete the ledger row, PK `(loser, at)`, and refold). The unwind is an owner operation on the box.

**S5 (per fold):**
- The `project` job row: `GET /admin/jobs` → `counts` carries `issue-469 register alias asked/bound/refused by cause`.
- The `[issue 448]` lines in the diag log (armed: register aliases, kept, poisoned, shared targets; finish: register counts and owner-name reads).
- Halt when `registry.refused > registry.bound`.
- The next dry plan's `registry_regrown` is the "split again" half (C10).

---

## 11. Tests

### 11.1 `crates/store/tests/altid_merge.rs`: helpers

- `fn wet_all_args<'a>(expect: &[String], expect_registry: &[String]) -> AltIdMergeArgs<'a>` = `{ expect_registry_pairs: expect_registry.to_vec(), ..wet_args(expect) }`. Add `expect_registry_pairs: Vec::new(), known_registry_deferred: Vec::new()` to `args()` (190).
- `Bed::wet_all(&self, expect, expect_registry)`:
  - the coh invariant as in `wet` (472);
  - the register invariants from `plan_capped` (516-526);
  - `registry_merged_pairs + registry_residual_pairs.len() == registry_pairs_planned.len() - registry_deferred_unreviewed`.
- `const CONFIRMED: &str = "altid-registry-confirmed-test"` (it starts with the prefix).
- `Bed::registry_verdict_in(cohort, key, members, action, confidence)`. Keep `registry_verdict` (cohort `469-registry-test`, which gives `confirmed` "verdict").
- `Bed::merge_registry_all()`: plan → post `CONFIRMED` HIGH merge on each `registry_plan_listing` entry (`l.key()`, member ids) → plan again → `wet_all(&dry.pairs, &dry.registry_pairs_planned)` → assert everything planned merged.
- `alias_rules()`: add `owner_mention_cap: store::ALTID_ALIAS_OWNER_MENTIONS`.

### 11.2 Tests the decision names

1. **`a_ukprn_ppon_pair_merges_into_the_ukprn_org_and_the_alias_binds_the_next_ppon_mention`**
   - Fixture: `registry_split(100, (2, UKPRN_A, "Leeds City College"), (1, PPON_P, "LEEDS CITY COLLEGE"))` (the register org has the higher id); `satellite(1, "CYM", "Coleg Dinas Leeds")`.
   - Post a `CONFIRMED` verdict with members [1,2], plan again (`admitted_verdict` 1, listing `verdict_cohort == Some(CONFIRMED)`), then `wet_all(&[], &[rk])`.
   - Expect:
     - `registry_merged_pairs == 1`, `removed == 1`;
     - org 1 is gone and org 2 stands; mentions and the CYM satellite are on 2;
     - the PPON is a merged identifier of 2 (`organization_merged_identifiers` with `identifier = minted(PPON_P)`, `org_id = 2`);
     - the ledger row (`keep 2`, `loser 1`, `job_id 77`) has every §5.3 field: `registry_kind` "national", `keep_id` = minted(UKPRN_A), `loser_id` = minted(PPON_P), `verdict` "admitted", `confirmed` "register", `witness_notices[0] == 100`, and `json_extract(evidence,'$.coh') IS NULL`;
     - the verdict stamp `applied_action = 'merged 1 row(s) into 2'`;
     - change events (org 1 removed, org 2 changed);
     - `registry_merged[0].evidence ==` the ledger text; residual empty.
   - Re-plan: `registry_pairs["GB-UKPRN"].already_one == 1`, `both_distinct == 0`, `registry_regrown` empty.
   - Alias: `fresh(&[300, 301])`, then `resolve(true, [ppon_first(300, PPON_P, "Leeds City College"), ppon_first(301, PPON_P, "Zenith Holdings Ltd")])`.
     - `ids[0] == 2`, `ids[1] != 2`, org count +1.
     - Counts: `registry: {aliases 1, asked 2, bound 1, bound_unwalled 1, refused 1, refused_names 1}`, `aliases 0`, `asked 0`, `recorded 0`, `ledger_rows 1`.
2. **`a_registry_ledger_row_never_reaches_the_coh_alias`**
   - Fixture: as test 1, merged.
   - The 448 SQL verbatim (`SELECT json_extract(evidence,'$.coh'), json_extract(evidence,'$.ppon') … WHERE rule='e2-altid'`) returns `(NULL, k(PPON_P))`, and `ALTID_ALIAS_LEDGER_SQL`'s columns 2-4 are set.
   - Armed: `aliases == 0`, `registry.aliases == 1`.
   - Delete org 2 and its mentions, then resolve: refused, `registry.refused_no_owner == 1`, `asked == 0`. There is no fall-through to `canon_of`.
   - A keep under the coh-shaped key `format!("{}~{}", minted(UKPRN_A), k(PPON_P))` leaves `registry.kept == 0`; the register key `rk(UKPRN_A, PPON_P)` gives `registry.kept == 1`, `registry.aliases == 0`.
3. **`a_ppon_aliased_to_a_company_number_and_a_register_is_poisoned`**
   - Fixture: `split(100, (2, COH_A, "Acme Widgets Ltd"), (1, PPON_P, "ACME WIDGETS LTD"))`, then `merge_all()`.
   - Add `org(5, UKPRN_A, "Acme Widgets Ltd")` and a SQL-inserted register ledger row for PPON_P: `{"scheme":"GB:altid","registry_scheme":"GB-UKPRN","registry_kind":"national","registry":"GBUKPRN10007798","ppon":"<k(PPON_P)>"}`.
   - Resolve `ppon_first(300, PPON_P, "Acme Widgets Ltd")`: id not in {2, 5}; `ledger_rows 2`, `poisoned 1`, `aliases 0`, `registry.aliases 0`, `registry.asked 1`, `registry.refused_poisoned 1`, `asked 0`.
   - Then a keep on the register key: binds to 2 (`registry.kept 1`, `aliases 1`, `bound 1`).
4. **`a_register_target_on_two_rows_or_withheld_binds_nobody`**
   - Fixture: test 1's pair, merged through the wet path. The second row is added after the merge, so D3 does not deny it at plan time.
   - (a) `org_in(7, "GB", &minted(UKPRN_A), "Leeds City College")` → resolve → refused `registry.refused_shared == 1`, id not in {2, 7}.
   - (b) Delete org 7; `identifier_verdict(2, &minted(UKPRN_A), "wrong")` → refused `shared`.
   - (c) A `"right"` verdict → binds to 2.
5. **`registry_drift_aborts_even_when_coh_parity_holds`**
   - Fixture: `split(100, (1, COH_A, "Acme Ltd"), (2, PPON_P, "Acme Ltd"))`; `registry_split(200, (3, UKPRN_A, "Leeds City College"), (4, PPON_Q, "Leeds City College"))`; `CONFIRMED` verdict with members [3,4]; plan.
   - `fake(n)` = `GB-UKPRN:GBUKPRN1000000{i}~FAKE{i:08}`.
   - `wet_all(&dry.pairs, &fake(6))` errs: "register parity abort", "live register plan has 1 pairs, the stored register plan 6", "7 differ", "nothing was written". The snapshot is unchanged: the coh pair was NOT merged although its parity held.
   - With `fake(5)` plus the live key: `merged_pairs 1`, `registry_merged_pairs 1`, `registry_expected_not_live 5`.
6. **`a_ppon_org_two_families_would_fold_merges_neither`** (U4's C8: the loser holds a PPON in each family)
   - Fixture:
     - `org(1, PPON_P, "Acme Widgets Ltd")` (L);
     - an SQL insert into `organization_merged_identifiers` of `(minted(PPON_Q), 'national', 'GB', 1, 999, 'e2-altid')`;
     - `org(2, COH_A, "Acme Widgets Ltd")`, `org(3, UKPRN_A, "Acme Widgets Ltd")`;
     - notice 100: party "C" bound to 2, `[COH_A, PPON_P]`; notice 200: party "R" bound to 3, `[UKPRN_A, PPON_Q]`;
     - `mention(300, "ORG-L", 1, "GB", None, "Acme Widgets Ltd")`: L carries no raw identifier, so neither loser-incoherent gate fires;
     - a `CONFIRMED` verdict on `rk(UKPRN_A, PPON_Q)` with members [1,3].
   - Dry: `pairs == [pk(COH_A, PPON_P)]`, `registry_pairs_planned == [rk(UKPRN_A, PPON_Q)]`.
   - `wet_all(&dry.pairs, &dry.registry_pairs_planned)`: `contradictory 1`, `registry_contradictory 1`, `merged_pairs 0`, `registry_merged_pairs 0`; both residuals carry their key; the snapshot is unchanged.
   - Control: delete the merged-identifier row, plan again (the register side is `no_target_ppon`), `wet_all(&dry.pairs, &[])` → the coh pair merges.
7. **Stay green:** `the_harvest_seeks_notices_profile_and_the_notice_ids_pk` (1122). Add EXPLAIN on `ALTID_ALIAS_OWNER_MENTION_NAMES_SQL` with `vec![Value::Integer(1), Value::Integer(10)]`: assert `organization_mentions_org` and neither `SCAN` nor `TEMP B-TREE`. If it shows a temp B-tree, drop the `ORDER BY` (§7). The supervisor's stack canary `an_execute_without_an_expected_count_is_refused` (18275) must also stay green.

### 11.3 Tests for D1, D2, D3, C9 and S5 (recommended)

8. **`a_register_pair_planned_without_a_high_verdict_is_held_until_one_is_posted`** (D1)
   - `registry_split` planned by corroboration → `wet_all(&[], &[rk])` → `registry_unconfirmed 1`, merged 0, residual `[rk]`, no ledger row.
   - Post `registry_verdict` (cohort `469-registry-test`), no re-plan → `wet_all(&[], &residual)` merges → `confirmed` "verdict".
9. **`a_capped_wet_run_caps_each_family_on_its_own`** (D2 and S4)
   - Two coh splits and two confirmed register splits; `max_pairs: Some(1)`.
   - `merged_pairs 1`, `registry_merged_pairs 1`, each residual 1; `registry_merged.len() == 1` and its `evidence` equals the ledger row's.
   - The continuation `wet_all(&r.residual_pairs, &r.registry_residual_pairs)` merges the rest.
10. **`a_register_org_holding_the_number_only_as_a_merged_identifier_never_plans`** (D3)
    - `org(2, CHC_Y, "Leeds City College")` plus a merged identifier `minted(UKPRN_A)` on 2; `org(1, PPON_P, "Leeds City College")`; the party `[UKPRN_A, PPON_P]` bound to 2; a PPON-first party bound to 1.
    - `denied_single_owner == 1`, nothing planned. U4 would have planned it.
11. **`a_merged_register_pair_that_splits_again_is_counted_regrown`** (C10)
    - After test 1's merge: `org(9, PPON_P, "Leeds City College")` and a PPON-first party bound to 9.
    - Plan: `registry_regrown == [rk(UKPRN_A, PPON_P)]`, `registry_pairs["GB-UKPRN"].multi_target == 1`.
12. Optional C9 chain test: an org that is one register pair's keep and another's loser holds the second pair only.

### 11.4 Fold-level tests (store)

13. **`the_register_alias_survives_the_register_orgs_renumbering`**
    - Test 1's merge, then delete mentions, names and orgs; empty `organization_merged_identifiers` (the rebuild shape); `org(50, UKPRN_A, "Leeds City College")`.
    - Resolve: `ids[0] == 50`, `registry.bound 1`, `recorded 1`; a merged identifier PPON on 50 with rule `altid-alias`.
14. **`a_register_alias_bind_holds_448s_bar_and_refused_mentions_mint`**
    - Three register merges through `merge_registry_all`: (UKPRN_A, "Leeds Trust Ltd", PPON_P), (UKPRN_B, "Harrogate College", PPON_Q), (CHC_X, "Helping Hands Trust", PPON_R).
    - Resolve `(PPON_P, "Leeds Trust plc")`, `(PPON_Q, "Zenith Holdings")`, `(PPON_R, "Helping Hands Consortium")`.
    - Every id differs from its owner; org count +3; `registry.refused 3`, `refused_veto 2`, `refused_names 1`.

### 11.5 Issue 518 tests (store)

15. **`the_alias_reads_the_owners_mention_names_as_the_planner_does`** (the Caterlink shape, on the coh alias)
    - `split(100, (2, COH_A, "Cater Link Limited"), (1, PPON_P, "Caterlink"))`.
    - Add a non-witness coh mention: notice 102, party bound to 2, `[COH_A]`, via `party_named(…, "Caterlink Ltd")`.
    - Dry: `corroborated_by == Some(("Caterlink Ltd", "Caterlink"))`. Then `merge_all`.
    - Resolve `ppon_first(300, PPON_P, "Caterlink")` → `ids[0] == 2`, `bound 1`, `refused_names 0`, `owner_name_reads 1`. It was refused before the fix.
    - Mirror it in the register family: owner "Leeds City College Ltd" whose mentions carry "Leeds College".
16. **`the_owner_name_read_is_capped_newest_first_and_says_so`**
    - An owner with 3 mentions where only the OLDEST carries the agreeing name.
    - `owner_mention_cap: 2` → refused, `owner_names_capped 1`. Cap 3 → bound.
17. **Stays green:** `a_name_that_fails_corroboration_refuses_and_mints` (1807): the veto still reads designated names.

### 11.6 Existing tests to update

- `an_armed_resolver_binds_a_ppon_first_mention_to_the_company_number_org_after_a_merge` (1668-1681): add `owner_name_reads: 1` to the expected struct.
- `registry_pairs_never_change_the_coh_plan`, wet leg (2476-2478): replace the "plans no register" assertion with `wet.registry_pairs_planned == [rk(UKPRN_A, PPON_R)]` (live), `registry_deferred_unreviewed == 1`, `registry_merged_pairs == 0`. Keep the org and ledger assertions.
- `coh_view` (2392): also clear the new `registry_*` wet fields and `registry_regrown`.

### 11.7 `supervisor.rs`

- **Update U4's round trip (16704-16714):**
  - step 2's message: `register (issue 469 U5): merged 0 of 1 reviewed live pairs (1 awaiting a HIGH verdict`;
  - `registry_pairs_planned` still `[key]` (the residual);
  - `registry_unconfirmed` 1.
- **New: `an_altid_wet_run_merges_a_confirmed_register_pair_and_the_next_dry_reads_it_already_one`**
  - Fixture: `altid_fixture` plus 16648-16669's rows.
  - Steps:
    1. Dry.
    2. Record a HIGH merge built from `registry_plan[0]` (`country`/`scheme`/`key`/members), cohort `altid-registry-confirmed-2026-10-12`.
    3. Dry: `registry_gates.GB-UKPRN.admitted_verdict == 1`, `registry_plan[0].verdict_cohort` set.
    4. Build indexes, then wet.
  - Expect:
    - the message has both `WET: merged 1 of 1 reviewed live pairs` and `register (issue 469 U5): merged 1 of 1`;
    - org 4 is gone; the notice-13 mention is on org 3;
    - one ledger row with `keep=3 AND loser=4`, `$.coh IS NULL`, `$.registry_scheme='GB-UKPRN'`, `$.registry_kind='national'`, `$.registry='GBUKPRN10007798'`, `$.registry_literal='GB-UKPRN-10007798'`, `$.ppon='PBZB4962TVLR'`, `$.keep_id='GBUKPRN10007798'`, `$.loser_id='GBPPONPBZB4962TVLR'`, `$.confirmed='register'`, `$.verdict='admitted'`, `$.witnesses=1`, `$.publishers=0`, `$.witness_notices[0]=12`, and the job id;
    - the stored residual: `registry_pairs_planned == []`, `registry_merged_this_run == 1`, `registry_merged[0].evidence.confirmed == "register"`.
  - Re-dry: `registry_pairs.GB-UKPRN.already_one == 1`, `registry_pairs_planned == []`, the message has `GB-UKPRN 0 of 1 two orgs`.
- **`the_alias_suffix_speaks_only_when_asked` (17545):**
  - register-only counts → the suffix contains `issue-469 register alias asked 2 bound 1 refused 1` and `shared 1`, and does not contain `issue-448 alias asked`;
  - coh-only stays as today.

### 11.8 `crates/ingest/tests/fts.rs`

- Add the two `Vec::new()` fields at 629 and 741.
- New end-to-end test after 704: **`a_college_the_altid_arm_merged_by_its_ukprn_stays_one_org_through_the_next_fold_and_a_rebuild`**
  - Modelled on 597, with `party_release` `[("GB-UKPRN","10007798"),("GB-PPON","PBZB-4962-TVLR")]` "Leeds City College" and a PPON-first notice.
  - Dry, then record a `CONFIRMED` verdict from `dry.registry_plan_listing[0]`, dry again, then wet with `expect_registry_pairs`.
  - The next day's PPON-first notice through the incremental fold binds the UKPRN org. A bare rebuild keeps one org.

---

## 12. Stack safety

- **Store side, boxed:**
  - `plan_registry_pairs` (already);
  - `altid_wet` (new, §4.1);
  - both `altid_merge_steps` calls;
  - `altid_alias_bind` at its call site (§6.5);
  - the owner-name read, also boxed or kept in a small helper.
  - The store tests poll `match_org_altid_pairs` and `resolve_mentions` on test threads.
- **Supervisor:** every change is inside the boxed altid arm (9116). The JSON work stays in sync free functions. `run_match_identifiers_spec`'s budgets (3,840 B; 71 KiB poll) are not affected, because the arm's interior is not measured.
- **Watch `run_project` (budget 9,000 B).** `AltIdAliasCounts` grows by about 112 B (12 register u64 plus 2), carried in ingest's `Report` (project.rs:86).
  - If the budget test trips, reduce the counts to the registry struct before the next await in `run_project` (4632-4675), or box `Report.alias`. Do not raise the budget.
- Run `ops/check.sh`, read `GATE-EXIT=`, and use `--features server` for any supervisor check (CLAUDE.md).

---

## 13. Sequencing

1. **Issue 518, unit 2a (§7):** the owner-name read, the `AltIdAliasRules` field, the counters, tests 15-17, and the 1668 update. Gate.
2. **Store types and constants (§3);** exports in lib.rs.
3. **Planner (§4):** guard removal plus the stop return, D3, cohort capture, wet collection, `registry_regrown`.
4. **`altid_wet` (§5):** the helper, per-family parity, contest, D1, per-family caps, edges.
5. **Alias (§6):** SQL, open scan, arm, bind, call site, diag lines.
6. **Store tests** (§11.1-11.6).
7. **Supervisor (§8) and its tests;** then fts.rs.
8. **Docs (§9).**
9. **Focused tests** with the CLAUDE.md flagged command, then `ops/check.sh`. Run `git diff` per file before staging, because canonical.rs and supervisor.rs are contended.

Steps 2-8 are one deploy: the wet path and the alias go together.

---

## 14. Risks

1. **D1 adds a step to U6.** Register-confirmed pairs must be posted as verdicts. A pair someone forgets is held (`awaiting a HIGH verdict`), which is the safe direction.
2. **The cohort prefix is a string contract.** A reviewer cohort named with the prefix by mistake would make the ledger claim `confirmed: "register"`. Keep the constant documented, and have U6's tooling use it.
3. **Refactoring the coh transaction loop.** Move it verbatim. The coh wet tests (1177-1559) and the coh evidence assertions (1229-1242) pin it.
4. **D3 moves U4's dry counts** (`denied_single_owner`) by the number of merged-only holders. Expected to be about 0.
5. **Remaining regrowth paths, all refused `no owner` (safe, and visible to S5):**
   - the register org merged away later (R2, E0, rekey, issue 515's cross-walk), so no exact row remains;
   - a normaliser change (issue 514);
   - a rebuild whose PPON-first notice comes before the register-first one.
6. **The wet run holds the writer longer** for the register re-plan: about 260 candidates' reads, so seconds.
7. **The cross-family contest can hold coh pairs 448 alone would merge.** It only happens when a register pair shares the loser or the keep. Expected 0; counted in `contradictory`.
8. **Rollback hazard.** A pre-U5 binary skips register ledger rows (empty coh). Rolling back after the first register wet run makes those PPONs regrow unless you unwind. Put this in the deploy note.
9. **Issue 518's cost.** The capped read runs per owner per fold on the writer (§7). A large `owner_names_capped` count means the cap binds; raise it only after measuring. The `ORDER BY rowid DESC` must not sort.
10. **Issue 518 is a behaviour change on the coh alias in the same deploy.** Its own commit and gate run, plus 518's Verify (names refusals below 7.6%), separate it.
11. **Stack:** `run_project`'s budget (§12).
12. **Gate time:** 3-5 `ops/check.sh` runs, about 11 minutes each, about 35 minutes from clean.

### Critical Files for Implementation
- /home/user/tender-db/crates/store/src/canonical.rs
- /home/user/tender-db/crates/store/tests/altid_merge.rs
- /home/user/tender-db/crates/app/src/supervisor.rs
- /home/user/tender-db/crates/store/src/lib.rs (exports 64-70)
- /home/user/tender-db/crates/ingest/tests/fts.rs (629, 741, new end-to-end test); also /home/user/tender-db/crates/ingest/src/project.rs:1987-1997 and /home/user/tender-db/docs/operations.md (812, 837-842, 2066)
