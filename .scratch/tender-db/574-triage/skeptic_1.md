# 574 skeptic 1: the judge's W and REVERT rows

Scope: the judge (read_1.md) marked one row W and none REVERT. That row is 6769473. I judged it as if
`Capita Group Plc -> Care and Custody (Health) Ltd` shipped today (2026-10-19). I tried to show it is R or N.

**Result: W stands (1 of 1). Confidence is medium, up from the judge's medium-low.**

## Budget

- Saved bodies read first: `bodies/org_6769473.json`, the 548 precedent bodies (`548-triage/u3/bodies2654/*11628199*`),
  `548-triage/notes.md`, `548-triage/u1/judgements.json` and the GB census list
  `452-census/gb-orgs-2026-09-30.jsonl` (identifier, org_id, name).
- 2 new public GETs (the limit was 4), both `rc=200`, about 3 s apart, logged in `api_requests.log`, bodies in `bodies/`:
  - `GET /v1/organizations/11504910`
  - `GET /v1/organizations/11621025`
- No /v1/sql, no POST, no ssh, no git. I wrote nothing outside `574-triage/`.
- I did not spend the other 2 GETs. The allowed endpoints do not list an org's tenders or notices, and no saved
  file holds a tender or notice id for 6769473 or 11504910, so I could not open the dated 2015 or 2018 procedures.

## Row 6769473 (GB, identifier 618184140, V/T/H 11/7/1). Final: W

What the entry shows:

| cluster | votes | newest print |
|---|---|---|
| `Capita Group Plc` (head) | 1 | 2015-06-30 |
| `Capita Business Services Ltd (Capita Transformation)` (rival) | 3 | 2015-12-15 |
| `Care and Custody (Health) Ltd` / `Limited` (top) | 6 + 1 = 7 | 2018-04-07 |

The live org still serves `Capita Group Plc`, with 11 mentions and no merged identifiers.

### The case for R (tested, does not hold)

1. The new name is the plurality (7 of 11) and the newest print (2018). The old head is a single stale print, and it
   is the parent's former name: The Capita Group Plc is now Capita plc.
2. The head is the kind of thing the 548 gb_family release accepts: one stale stray (H=1, older than 3 years) with
   the top cluster served instead.
3. 618184140 is nobody's Companies House number (CH numbers have 8 characters), so no register name contradicts the
   top.

Why these fail:

- **The identifier is a VAT number, and two separate Capita companies print it.** 618184140 passes the old-scheme GB
  VAT check: weights 8..2 on the first 7 digits give 154, plus 40 is 194, and 194 mod 97 = 0.
  - A VAT number belongs to one registration. When several legal entities print the same one, that registration is a
    VAT group, and the number is the group's (held by its representative member).
  - Two different Capita legal entities print it, in two clusters: the parent (`Capita Group Plc`) and
    `Capita Business Services Ltd` for its Capita Transformation division, which has nothing to do with custody health.
  - Two independent Capita prints agreeing on a custody-health company's own number by mistake is implausible. So
    the number is Capita's.
- **The 548 gb_family precedent runs the other way.** In 548 the served name became the register entity's own name,
  and the stray print was a subsidiary's:
  - 11628199: CH 02081330 = CAPITA PLC. `Capita Business Services Limited` -> `Capita PLC`.
  - 20957059: a JEOL Nordic AB print on the JEOL (Germany) GmbH VAT number.

  Here the stray-looking head is the umbrella (the parent), and the change would serve one member's name, or another
  company's, for the whole number. That is the label's "a member for the group" shape, not the released one.
- **Care and Custody (Health) Ltd already has its own org under its own register number.** The census lists
  `["05881801", 11504910, "Care and Custody (Health) Ltd"]`. Live, `GET /v1/organizations/11504910` returns that
  name, identifier `05881801` (national) and 2 mentions.
  - So the company is not left without a home if this org keeps a Capita name.
  - Shipping the change would put the same name on a second org whose number Capita's own companies print.
- **Whose company Care and Custody (Health) was cannot rescue the change.** Its name matches Mitie's custody brand:
  `GET /v1/organizations/11621025` returns `Mitie Care and Custody Limited` (identifier 6976230; the census also has
  GBCOH06976230 as org 31573903). I could not establish whether it was a Capita member or a Mitie company in 2018.
  Both cases still give W:
  - **Capita member** (inside Capita's VAT group in 2018): the change serves a member's name for the group number.
    The 4 Capita Business Services and Capita Group mentions would be shown under a custody-health company.
  - **Mitie company** whose 2018 prints carry Capita's VAT number by mistake: the change serves another entity's name
    for Capita's number. The fix for that would be splitting the org, not renaming it.
- **Only one reading makes it R.** If 618184140 were Care and Custody's own VAT registration, the change would be
  right. But that needs both Capita clusters to be strays carrying a foreign number, and the company already has a
  CH-numbered org of its own. The prints do not support it.
- **Weight of the top.** Its 7 votes may come from one or two notices: there are 2 spellings and a short date span,
  and the 548 q02 lesson is that 7 votes can be one notice. Per notice, the top may not outweigh the two Capita
  clusters.

### The case for N (tested, does not hold)

N needs the same entity under both names. These are different legal entities, so N does not apply. Even counting it
as "two imperfect labels", they are not equally imperfect:
- The old label `Capita Group Plc` is a stale spelling of the right umbrella, the parent of the VAT group.
- The new label names a different company for 4 of the 11 mentions, under every ownership reading.

### Not a REVERT

The change moves away from the older name (2015) to a newer one (2018), so it is not a revert. The defect is the
entity, not the age.

### What would flip it

- **To R:** a dated 2018 notice where Care and Custody (Health) Ltd states 618184140 as its own VAT number while it was
  outside Capita's VAT group.
- **Better than either name:** `Capita plc`, the group's current name. That is not the change on offer.

## Verdicts (machine-readable)

```json
{"6769473": "W"}
```
