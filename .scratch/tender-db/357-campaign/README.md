# Cluster-country review campaign — the tooling (issue 357, cohort `cluster-country-2026-09-05`)

The review loop issue 311 asked for, run at scale: every organization row that carries one
register number under several country codes is read by an agent against a rubric, an
adversarial second agent tries to refute every move, a blind third reader re-reads a sample,
deterministic floors take the deny direction, and only HIGH moves reach the database through
`apply-country-verdicts` (issue 355's verdict store). Seven slices of ≤600 clusters ran on
2026-09-05/06 — 1,730 clusters, 1,233 rows moved, 779 folds; the per-slice records are `../357-cluster-country-verdicts-slice*.json` and the
issue file carries the numbers.

Files here:

- `rubric-v3c.md` — what the reviewer and challenger read first. Evidence fields, the spray
  shape, the four verdicts, the four move evidence classes, the HIGH bar. The version that ran.
- `split.py` — cuts a `country-cluster-packet` report into 35-case review batches stratified
  by census verdict × spray/balanced shape, every-10th-case sample files for the blind readers,
  `index.json`, and `args.json` for the workflow.
- `review.js` — the Workflow-tool script: per review batch a reviewer then a challenger
  (pipeline, so challenges start while later batches still review); the sample batches in
  parallel on the session model. Structured output schemas for both.
- `post.py` — joins reviews, challenges and samples by identifier set, prints the agreement
  statistics, applies the floors, and writes the `POST /admin/country-verdicts` body.

## One slice, end to end

```sh
# 1. the packet (a job; ~2 s) and its report
admin.sh enqueue country-cluster-packet '{"max_groups":600}'
admin.sh raw GET /admin/reports/country-cluster-packet < /dev/null > packet.json

# 2. batches — excluding clusters already in a committed slice record is a belt; the packet
#    itself leaves out clusters that carry a country verdict (c4fd5f8)
python3 split.py packet.json ./batches --exclude ../357-cluster-country-verdicts-slice*.json

# 3. the review: Workflow tool, script review.js, args = ./batches/args.json
#    (dir, rubric, model, review [[id, stratum]], samples [id], roots {id: [identifiers]}).
#    Two agents at a time on a 4-CPU container; ~2 h for 600 cases with sonnet reviewers.
#    Results land in <session>/subagents/workflows/<run>/journal.jsonl.

# 4. join + floors + the POST body (prints the stats; read the diff: lines by hand)
CAMPAIGN_DIR=. python3 post.py <journal.jsonl> cluster-country-2026-09-05 ./batches
#    -> 357-post-body.json (the verdicts) and 357-campaign.json (everything joined)

# 5. record, plan, compare, execute, fold, project
admin.sh raw POST /admin/country-verdicts < 357-post-body.json
admin.sh enqueue apply-country-verdicts                          # dry: plan stored as report
admin.sh raw GET /admin/reports/country-verdict-plan < /dev/null # diff its tuples vs the HIGH rows
echo '{"kind":"apply-country-verdicts","dry_run":false}' | admin.sh raw POST /admin/jobs
admin.sh enqueue match-org-identifiers                            # R2 dry: the moved rows now
echo '{"kind":"match-org-identifiers","rule":"r2","dry_run":false}' | admin.sh raw POST /admin/jobs
admin.sh enqueue project                                          # repoints derived rows: 0 notices
```

Then save the slice record — `{"cohort", "slice", "rubric", "cases": [{identifier, stratum,
verdict, confidence, rationale, moves, challenger, blind_sample}]}`, one entry per reviewed
cluster with the recorded moves and what the challenger and blind reader said — append the
slice's section to the issue, commit.

## What the floors are for

The reviewers over-claim HIGH in shapes the rubric did not name, and the challenger catches
most but not all of it. `post.py` turns each shape into a rule in the DENY direction only
(a floor demotes HIGH to MEDIUM, never promotes): an agreeing row never moves; a name that
says "branch"/"embassy" without naming the destination is a foreign filing; one register
serving two codes (Åland/FI, DOM/FR, Faroes/DK — issue 358) is policy, not contamination;
arithmetic claimed without an anchor for the destination; the CZ/SK/SI shared mod-11 on an
unprobed row; weight moves only for genuine strays (≤2 mentions, destination named or 10×);
a mover whose name carries ITS OWN country's unambiguous legal form (a French SAS with an
Italian VAT id is a parent carrying its branch's number, not a mis-tag); a mover with ≥10
mentions has standing unless its name says it is the destination's branch or carries the
destination's legal form. `HAND_HIGH`/`HAND_MEDIUM` are one-by-one adjudications of heavy
movers from the slices. A reviewed cluster without a move gets a `keep` on its heaviest
member so the next packet skips it.

Each new shape found while reading the heavy movers was added here AND written to the issue
("A shape the rubric missed"). If the campaign runs again, read that section first.
