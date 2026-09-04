# 350 — the genericness wall reads prominent entities as generic, costs the resolver recall, and feeds itself

Status: needs-measurement (filed 2026-09-04 from issue 349's census; the E0 half is fixed there, this is the resolver/R3 half)
Kind: identity semantics (organization layer) — the wall where the evidence is name-only
Relates to: 349 (the measurement), 316/318 (the wall), 331/332 (the statistic), 300 Stage 3 (R3 rescue), the resolver's prevention hook (`name_key_is_generic_on`)

## Observed

Issue 349's census: 315 of 375 `agree-generic` groups are echoes — the key is
over the 20-carrier wall because ONE entity sits in dozens of identifier-less,
mostly NULL-country provisional rows (`stadt burghausen` 157 rows / 7 with an
identifier; `ricoh deutschland gmbh` 115 / 18; `t systems international gmbh`
84 / 15; `hexal ag` 70 / 10). The wall reads each as "a name nobody chose to
make unique".

Where that verdict is consulted with name-only evidence:

1. **The resolver's prevention hook** (`name_key_is_generic_on`, inside the
   fold's write transaction): a mention with a name and no usable identifier
   is not attached by name to a standing org when its key is generic. For an
   echo key that means every further name-only mention of Ricoh Deutschland
   mints or joins a provisional row instead of the identified org — **which
   adds a carrier, which keeps the key generic.** The wall feeds itself; the
   157 Burghausen rows are the loop's output, not its input.
2. **The R3 rescue** (NULL-country candidates → standing target by checksum
   anchor + N2 corroboration): generic → deny. Echo keys deny the rescue of
   exactly the rows the loop created.

Recall is what is lost; precision is not at stake in either place unless the
key is a genuinely shared name — and `caritas` across thirty different Caritas
bodies, or `stadtwerke gmbh`, IS one: an echo criterion that only asks whether
identified carriers are under the cap would admit those too. That is why
issue 349 scoped its fix to E0 (where the members already share a triple) and
left these two walls alone.

## Measure first

1. How many provisional rows on the box carry an echo key (the loop's output),
   and how many mentions sit on them — the recall at stake. The 349 probes
   give the keys; `org_match_keys` gives their carriers; `organization_mentions`
   by org id gives the mentions. Bounded.
2. For the echo keys, how many DISTINCT identified entities the identified
   carriers actually are, once identifier spellings are canonicalised
   (`canonical_key` where an arm exists, else the raw literal): Ricoh's 18
   identifiers are one company if they are one VAT plus tax numbers and
   mangles, and eighteen if they are eighteen VATs. THAT is the statistic that
   separates `ricoh deutschland gmbh` from `caritas`.
3. A 50-sample of echo keys, read by a person or an agent: one entity or many?

## Then, one of

- (a) the wall counts DISTINCT CANONICAL IDENTIFIERS among carriers, not rows
  (issue 332's idea, measured negative on the wrong class — re-measure on this
  one); or
- (b) the resolver attaches a name-only mention to an echo key's identified org
  when there is exactly one such org (a "sole identified carrier" rule), which
  breaks the loop without touching the wall's semantics; or
- (c) leave both as they are and let the E0 fold plus a periodic R3-style
  rescue drain the provisional echo rows behind the wall.

## Done when

- the three measurements are on this issue;
- the chosen change is deployed and the next census shows the echo class
  shrinking rather than growing week over week.
