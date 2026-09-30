# Rubric — a Companies House number and a PPON published together for one supplier (issue 448)

You are reviewing pairs from a public-procurement database (UK Find a Tender notices). In each CASE,
one or more notices ("witnesses") list ONE party with BOTH a Companies House company number (`coh`)
and a PPON (the UK Central Digital Platform's supplier id, `ppon`). The database holds two
organization rows for it: the company-number org (`keep`, which survives a merge) and the PPON
org (which would fold into it). A merge says: this PPON belongs to the legal entity with this
company number.

## What you see per case

- `register`: Companies House's CURRENT name and status for the company number, and its
  PREVIOUS names (renames). This is the ground truth for who the company number is.
- `coh_names`: names other notices publish for the company-number org (outside the witnesses);
  `ppon_names`: names other notices publish under the PPON alone. Both are capped at 8, and
  `*_name_keys` say how many distinct names there are. A few stray names on a side are normal: a
  publisher sometimes attaches one supplier's identifier to another supplier.
- `heads`: each org's displayed name. It is a first-seen election, and one stray mention can set
  it, so do NOT judge by the heads alone.
- `witnesses`: how many notices assert the pair. `cooccurring`: notices OTHER than the witnesses
  where both orgs appear as two separate parties. That happens for a wrong pair, and also for a
  true pair beside a publisher that mislabelled a second supplier.
- `corroborated_by`: the two names whose agreement put the pair in the plan (planned pairs only).
- `gate`: `plan` (would merge) or the reason it was denied (`witness-only`,
  `uncorroborated-overlap`, `uncorroborated-disjoint`, …).

## Verdicts

- `merge`: the PPON org IS the company. Its names (most of them, ignoring an obvious stray)
  are the register's current or a previous name, a trading name or brand of it, or a clear
  spelling or form variant. Renames count: Doosan Babcock → Altrad Babcock, Northgate Public
  Services → NEC Software Solutions UK, Engie Services → Equans Services. A university or public
  body that publishes a consortium or department under its own numbers counts too.
- `keep`: two DIFFERENT legal entities. The typical cases:
  - a parent group beside a subsidiary that has its own company number ("Lloyds Banking Group
    plc" vs "Lloyds Bank plc");
  - two sister companies ("X Services Ltd" vs "X Construction Ltd");
  - the PPON's names belong to a company other than the register's (a wrong number or a wrong
    PPON);
  - the register shows an unrelated company.

  One stray name on a side is not a keep. The question is whose PPON it is.
- `needs-more-evidence`: the evidence cannot tell. Say what would settle it.

## Confidence

- `high`: you would execute it yourself. For a planned pair, a HIGH `keep` stops the merge for
  good. For a denied pair, a HIGH `merge` executes the merge. Give HIGH only when the register
  or the names make it plain.
- `medium`: probably right.
- `low`: a guess.

Output exactly one entry per case (the `case` string is the key), each with a two-sentence
rationale that names the evidence you used (a register name, a rename date, a name match, or a
stray).
