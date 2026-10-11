# 518 — the 448 alias reads a narrower name set than the planner, so a merged pair grows back

Status: ready-for-agent — filed 2026-10-11 from an hourly audit (the alias counters after issue 516's merges).
NEXT: unit 1, measure the refusal classes.
Kind: data quality (organization identity; regrowth)
Relates to:
- 448 unit 3 (the resolver alias and its bar);
- 469 U5 (the register alias takes "448's name bar" verbatim, so this defect would carry over);
- 469's stop rule S5 ("alias refusals exceed binds … an alias defect").

## What was found

**One measured regrowth.** Today's daily `project` (job 2215, 07:35 UTC) logged `issue-448 alias asked 1 bound 0
refused 1 (… names 1 …)`. The next dry plan (job 2217) moved `already_one` 5,068 → 5,067 and two distinct orgs
242 → 243. It also planned the regrown pair again at once: `03732298~PWPW6942JXBP`, Cater Link Limited
(org 10015573) beside a new PPON org 31919217 "Caterlink", minted from notice 47384480's PPON-first party. The
planner corroborated it with `Caterlink ~ Caterlink`.

**Why the alias refused what the planner admits.** The two read different name sets:
- The **planner** reads each org's names from its MENTIONS. The coh side's `coh_names` are `Cater Link Ltd`,
  `Caterlink` and `Caterlink Ltd`.
- The **alias bar** (canonical.rs, `arm_altid_alias`'s bar) reads the owner's DESIGNATED names:
  `SELECT name FROM organizations WHERE id = ? UNION ALL SELECT name FROM organization_names WHERE org_id = ?`.
  For 10015573 those are only `Cater Link Limited` and `CATER LINK LIMITED` (a bounded `/v1/sql` read,
  2026-10-11).

`Caterlink` therefore has nothing to agree with, `altid_corroborates` returns `Uncorroborated`, and the mention mints a
new PPON org. A merge the planner made grows back on the next PPON-first mention that uses the other spelling, and a
wet run must merge it again. Note that `altid_keys_agree` also treats `caterlink` and `cater link` as different
words: it strips forms but keeps the space.

**Scale.** Across the 82 folds in `/data/db/tender-db.db.diag.log`, the alias was asked 2,294 times:
- 2,119 bound;
- **175 refused** (7.6%): 168 names disagree, 4 consortium/legal-form veto, 3 generic, 0 poisoned, 0 no owner.

Each refusal is a regrowth, and stays a split until the next reviewed wet run. The bar's refusals are by design
("a refused mention mints exactly as it would without the alias"). The defect is the asymmetry: the bar is stricter
than the planner on the same pair, so it refuses pairs the planner would merge again unreviewed. Refusing a name the
planner would also refuse is correct.

## Proposed units

1. **Measure the classes.** The diag line carries counts only.
   - Add the refused mention's name and the owner's head to a sampled `[issue 448]` refusal line (the first 20 per
     fold), or read the regrown pairs off each dry plan after a fold. A pair that reappears planned with no verdict
     is a regrowth the planner admits.
   - Class each one:
     - (a) a mention spelling that the owner's MENTIONS carry but its designated names lack, as Caterlink does;
     - (b) a spacing or joined-word variant that neither side carries (`Cater Link` / `Caterlink`);
     - (c) a genuine stray (someone else's name under this PPON), where the refusal is right.
2. **Decide.**
   - For (a): read the owner's mention names in the bar, as the planner does. Use a bounded DISTINCT read of
     `organization_mentions` by `organization_id`, capped as the planner's listing is, and cached per fold per owner.
   - For (b): add a space-insensitive agreement to `altid_keys_agree`, so that `cater link` = `caterlink` once forms
     are stripped. This is a fold for the planner and the alias together, so a merge and its replay never read
     agreement differently.
   - Neither may loosen the consortium or legal-form veto.
3. **Before 469 U5 ships its register alias**, decide whether the register alias takes the fixed bar or the current
   one. Otherwise the register family inherits a ~7% regrowth rate from day one.

## Verify

- After the fix, the alias's `names` refusals per fold fall well below the 7.6% baseline.
- No regrown pair reappears in a dry plan without a verdict across a week of daily folds. Read
  `already_one` / `two distinct` after each daily project.
