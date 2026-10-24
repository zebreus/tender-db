# 518 — the 448 alias reads a narrower name set than the planner, so a merged pair grows back

Status: ready-for-agent — unit 2a (class a) DEPLOYED 2026-10-11 with 469 U5 (469's Status: "U5 DEPLOYED e59fa5e and U6 RUN 2026-10-11", record commit 70f8483). Unit 3 is done: the register alias takes the fixed bar (`../469-registry-pairs/u5-plan.md` D4, applied to both families). First post-deploy counter read (2026-10-24, from the 07:44 UTC `/admin/jobs?limit=100` snapshot `../551-lot-sum/b6/u9/jobs/chain-1024_read.json`, parsed offline with the Verify's own python; the record holds no earlier one): 5 daily folds, 2669 / 2688 / 2716 / 2726 / 2752 (10-20 → 10-24), asked 481, refused 10 (2.1 %), 9 of them `names` (1.9 %), against the 7.6 % / 7.3 % baseline. Every fold refused exactly 2. NEXT: (1) the same read once 7 daily folds are in, plus a dry altid plan showing no regrown pair planned without a verdict (no altid dry has run since 2232, 2026-10-11); (2) unit 1, which classes the remaining `names` refusals (the sampled refusal line is not built), then the class (b) decision on its result.
Was: ready-for-agent — unit 2a (class a) BUILT 2026-10-11 as 469 U5's first commit (83608c2 + review fix 6dae917, gated): the alias bar's corroboration reads the owner's designated names ∪ its newest mention names (capped 4,096 rows / 256 names, per-fold cache, skipped while organization_mentions_org is absent); consortium veto and head-against-head stay on designated names. Ships with U5's deploy. NEXT: after a week of daily folds, read the `[issue 448]` refusal counts against the 7.6% baseline; class (b) (space-insensitive agreement) still needs unit 1's measurement.
Was: ready-for-agent — filed 2026-10-11 from an hourly audit (the alias counters after issue 516's merges).
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

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh '/admin/jobs?limit=200'" | python3 -c "import sys,json,re; m=[re.search(r'issue-448 alias asked (\d+) bound \d+.*? refused (\d+) \(poisoned \d+, no owner \d+, veto \d+, names (\d+)', j.get('counts') or '') for j in json.load(sys.stdin)['recent'] if j['kind']=='project']; t=[tuple(map(int,x.groups())) for x in m if x]; print(len(t), 'folds: asked', sum(x[0] for x in t), 'refused', sum(x[1] for x in t), 'names', sum(x[2] for x in t))"

This sums the `issue-448 alias` clause that every `project` job row carries (`alias_suffix`, supervisor.rs). The clause
is silent when the alias was asked nothing. The command reads unit 2a, which is deployed, against the baseline. Class
(b) still waits on unit 1's measurement.
- **done**: over at least 7 daily folds, `names` is well below the baseline of 168 in 2,294 asks (7.3 %; all refusals
  175, 7.6 %), and no regrown pair is planned without a verdict (`already_one` / `two distinct`, read after a daily
  project).
- **open** (2026-10-24; not run over ssh in this pass. Its python, run offline on the 07:44 UTC `/admin/jobs?limit=100`
  snapshot `../551-lot-sum/b6/u9/jobs/chain-1024_read.json`, prints `5 folds: asked 481 refused 10 names 9`, i.e.
  names 1.9 % and refused 2.1 %. The 100 rows span 4.8 days; at `limit=200` it reaches further back and counts more
  folds.) The counter half of done is met on 5 daily folds, not yet 7. The regrowth half is unread, since no altid
  dry plan has run since 2232 (2026-10-11). Class (b) waits on unit 1. Before unit 2a (2026-10-11 baseline, 82 folds in the diag log): refusals
  7.6 %, 168 of them `names`. Unit 2a (`83608c2`, `6dae917`) shipped with 469 U5's deploy `e59fa5e` (`70f8483`).
