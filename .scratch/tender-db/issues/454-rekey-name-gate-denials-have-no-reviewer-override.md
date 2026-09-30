# 454 — 16 re-keys the name gate denies are mostly one entity, and nothing lets a reviewer admit them

Status: ready-for-agent — option 1 BUILT (`fee8c61`, gated green) and the 16 REVIEWED 2026-09-30 17:5x UTC: 8 merge + 1 keep verdicts POSTed (cohort `454-rekey-names-2026-09-30`, recorded 9); 7 unsettled stay denied with reasons below. NEXT: deploy `fee8c61` when the queue is idle (after FTS chunk 4 and rekey re-plan 1738), re-plan, and the 8 admitted merges ride issue 453's wet run.
Was status: ready-for-agent — filed 2026-09-30 17:0x UTC from issue 453's first dry plan (job 1726). NEXT: read the 16 against
the register (453's `ch_check.py` does it), then choose between a reviewer override in the rekey arm (the 362 shape)
and leaving them withheld.
Kind: data quality (identifiers)
Relates to: 453 (the re-key arm), 452 (the verdicts), 362 (merge verdicts: the reviewer-override pattern), 448 (the
altid name predicate the gate reuses)

## What is there

The rekey arm merges a wrong-number org into the standing org that carries the right number. It does so only when one
name pair agrees under the altid name key (`altid_keys_agree`). Job 1726 denied 16 of the 174 high verdicts on that gate
(`rekey-plan` `denied`, shape `names`). They stay as they are: withheld from identifier matching, and flagged
`register_mismatch`. That is safe, but their tenders never join the right number's org.

Read by eye, most are the same entity, and the gate's exact name-key equality cannot see it:

| org | name | wrong → right | right number's org |
|---|---|---|---|
| 15500364 | Taskforce Distribution Ltd | 07720853 → 07720852 | TASK FORCE DISTRIBUTION LIMITED |
| 31544041 | Bluestep Solutions Limited | 07393785 → 07392785 | BLUE STEP SOLUTIONS LIMITED |
| 16211817 | Sherbourne Area Schools Trust | 10059883 → 08130468 | Sherborne Area Schools Trust |
| 23708671 | Sony Europe B.V. | 15308001 → FC035527 | Sony Europe BV |
| 16654305 | Marie stopes International | 11022108 → 01102208 | MSI Reproductive Choices (renamed 2020) |
| 31573452 | Atkins Realis | OO688424 → 00688424 | Atkins Limited |
| 31571774 | PSI PRINT MANAGEMENT LIMITED | O2O84294 → 02084294 | PSL Print Management Ltd |
| 31578000 | CGi (UK) Ltd | 00947962 → 00947968 | CGI IT UK Ltd |
| 31556925 | Symology UK Ltd | 06472865 → 01760502 | Symology Limited |
| 31541672 | B Braun | 00077361 → 02296559 | B Braun Medical Ltd |
| 16558214 | Costa Coffee | 01270685 → 01270695 | COSTA LIMITED |

The rest need a real read, because they may be different entities in one group:
- Northern Education Associates → Northern Education;
- Silver Energy Management Solutions → ELEVATE EVERYWHERE LTD;
- Haringey GP Group → North Central London Training Hub;
- Ford Trustford → Ford Retail Ltd;
- VW Commercial Vehicles UK → Volkswagen Group UK Ltd.

Two shapes the gate should arguably see on its own:
- **Letter O for zero in the WRONG number** (`OO688424`, `O2O84294`). The number is the right one mistyped, so the "wrong"
  and right numbers are the same registration. The canonical key does not fold O→0, which is why these are not
  `same_key`. Three merges in the plan have the same shape and passed only because their names agreed: St Annes
  `O1089026`, Choices `O3796191`, Assist Homecare `O7006470`, plus RJ McLeod `SCO28565`.
- **Spacing inside a name** (Taskforce / Task Force, Bluestep / Blue Step), which the altid name key does not collapse.

## Options

1. A reviewer override: the rekey arm honours an `org_merge_verdicts` row under (`GB`, `GB:rekey`, `<wrong>~<right>`),
   `merge` + high admits past the name gate (never past consortium or legal form), and `keep` denies. That is the 362
   pattern, and the verdict table already has the shape.
2. Fold O→0 in a GB company number's canonical key, when the O sits where the register format has a digit. That is a
   key change: an `org_match_keys` epoch bump and an R2 re-census, a separate unit with its own blast radius.
3. Leave them withheld. Nothing is wrong today. The 16 orgs' tenders simply do not join the right number's org.

## Verify

    /root/aj.sh "/admin/reports/rekey-plan" | python3 -c "import json,sys; b=json.loads(json.load(sys.stdin)['body']); print(sum(1 for l in b['denied'] if l['shape']=='names'))"

- **open**: `16`
- **done**: every remaining `names` denial has a recorded reason (a `keep` verdict, or a line here saying why it is a
  different entity).

## 2026-09-30 17:4x–17:5x UTC — option 1 built; the 16 reviewed

**Built** (`fee8c61`). The rekey arm reads `org_merge_verdicts` under (`GB`, `GB:rekey`, `<wrong literal>~<right
number>`).
- A HIGH `merge` whose members are exactly the two org ids admits past the name gate only. It is counted
  `admitted_verdict` and stamped applied by the merge.
- It never admits past a withheld target, a flagged destination, the consortium veto or the legal-form veto. The
  test proves Delta plc stays denied against Delta Ltd.
- A `keep` denies the pair (`verdict-keep`, counted `denied_verdict`).

The test is `a_reviewer_verdict_admits_past_the_name_gate_or_keeps_a_pair_apart`.

**Reviewed** (workflow, 8 agents: a register lens and a refuting skeptic per batch of 4; full reasons in
`453-rekey/name-denials-review-2026-09-30.json`). A pair is admitted only when both lenses say `merge` at high:

| decision | pairs |
|---|---|
| **merge (8)** | Taskforce → TASK FORCE DISTRIBUTION (the register's own former name, 2011–12); Sherbourne → SHERBORNE AREA SCHOOLS' TRUST (a misspelling); Marie Stopes International → MSI Reproductive Choices (its name 1991–2020); Silver Energy Management Solutions → ELEVATE EVERYWHERE LTD (a rename); Sony Europe B.V. → Sony Europe BV (FC035527); Bluestep → BLUE STEP SOLUTIONS; PSI → PSL Print Management (`O2O84294`, O for 0); Atkins Realis → Atkins Limited / ATKINSRÉALIS UK LIMITED (`OO688424`) |
| **keep (1)** | B Braun → B. Braun Medical Ltd. Both lenses: the bare "B Braun" may be the group or another B. Braun company. Posted as a medium `keep`. |
| unsettled (7), left denied | Costa Coffee → COSTA LIMITED (register high / skeptic medium: the brand link rests on the typo); Northern Education Associates (both medium: the target "Northern Education" carries the number but may be a sister company); CGI (UK) → CGI IT UK (both medium); Symology UK → Symology Limited (register merge / skeptic keep: SYNOLOGY UK is a real separate company); Haringey GP Group → "North Central London Training Hub" (register keep: the target org is a training hub hosted by the GP group, and its number may not be its own); Ford Trustford → Ford Retail (both medium: TrustFord is a trading name, but "Ford Trustford Ltd" is not a registered name); VW Commercial Vehicles UK → Volkswagen Group UK (both medium: a division, not a company) |

The 9 verdicts were POSTed as cohort `454-rekey-names-2026-09-30` (recorded 9). They act from the first rekey plan
under `fee8c61`.
