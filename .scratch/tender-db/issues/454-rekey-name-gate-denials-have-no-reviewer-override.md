# 454 — 16 re-keys the name gate denies are mostly one entity, and nothing lets a reviewer admit them

Status: ready-for-agent — filed 2026-09-30 17:0x UTC from issue 453's first dry plan (job 1726). NEXT: read the 16 against
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
