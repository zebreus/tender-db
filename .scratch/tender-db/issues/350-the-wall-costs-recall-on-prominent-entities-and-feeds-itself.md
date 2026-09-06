# 350 — the genericness wall reads prominent entities as generic, costs the resolver recall, and feeds itself

Status: DONE 2026-09-06 (owner board sweep) — the three units this measurement filed as issue 351 all shipped: census (unit 1), country-less reuse under the wall (unit 2), the p0 provisional fold (unit 3, 5,764,011 rows folded by 2026-09-05); the wall's semantics were left alone as decided here. Was: MEASURED 2026-09-04 07:0x — **the premise was half wrong: the wall does not feed itself; country-less mentions do.** The resolver consults the wall only on the anchor-corroboration path (soft-scheme identifier + N2 agreement); name-only mentions never attach to identified orgs. The echo rows are the post-234 rule "country-less mentions mint fresh rows": `Stadt Burghausen` × 92 identical NULL-country provisional rows. Over 168 echo keys, 68% of the entities' mentions sit on such rows. Next unit filed below: a provisional-echo census, then country-less reuse gated by the wall, then a stock fold. Was: needs-measurement (filed 2026-09-04 from issue 349's census; the E0 half is fixed there, this is the resolver/R3 half)
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

## Measurement (07:0x UTC, 168 echo keys through `/admin/name-key` + mention counts)

Carriers bounded at the first 100 shown per key; mention counts by org id
(20 keys re-run after `/v1/sql` 429s, so the totals are complete).

| | |
| --- | --- |
| carriers shown | 9,003 |
| provisional (no identifier) | **7,856 (87%)** — 7,384 of them NULL-country too |
| identified carriers | 1,147 |
| mentions on provisional carriers | **44,340** |
| mentions on identified carriers | 21,064 |

So for these 168 prominent entities, **68% of their mentions point at echo
rows** rather than at the identified organization: that is the recall at
stake, and it is not small.

**What the echo rows are.** Not spelling variants: `Stadt Burghausen` has 92
provisional rows with the IDENTICAL name and NULL country; `Stadt Mannheim` 59
× `STADT MANNHEIM` + 38 × `Stadt Mannheim`; `IBM Deutschland GmbH` 83 identical
NULL-country rows. The post-234 provisional path reuses `(name_norm, country)`
and — its own words — "nameless or country-less mentions mint fresh rows". One
country-less mention, one row, forever.

**Identifier diversity among the identified carriers** (approximate cores:
alphanumeric, VAT prefix stripped): 1 core for 23 keys, 2–5 for 83, 6–9 for
27, ≥10 for 34 (Dell, IBM, Fujitsu, Olympus, Microsoft — HRB numbers, VATs,
Leitweg-IDs, tax numbers, mangles of ONE company). Distinct identifiers do not
separate one entity from many; they count the schemes a big supplier gets
published under.

**50-sample read: 48 of 50 are one entity** (Stadt Netphen, Hochtaunus
Kliniken, Rohde & Schwarz, Entsorgungsbetriebe Wiesbaden, Landkreis Ebersberg,
Kreis Wesel, Medtronic, Zöller-Kipper, Dekra Automobil, …). The two others are
the shape the wall exists for: `gemeinde taufkirchen` (Taufkirchen (Vils),
Taufkirchen bei München, Taufkirchen an der Pram) and `gemeinde haibach` (two
Haibachs) — identical names, different municipalities, each publishing its own
number when it publishes one.

**Where the wall is actually consulted** (`canonical.rs`, the resolver): only
when a mention's identifier reaches a standing org by canonical key under a
scheme that does not checksum hard, and the bind then needs N2 agreement — the
wall refuses that agreement for a generic key. Name-only mentions are never on
that path. So the wall's cost is anchor-corroborated binds on soft schemes for
echo keys (real but bounded), plus the R3 rescue; the 44k mentions above were
never the wall's to lose.

## Decision

Leave the wall's semantics alone (this issue's (a) is withdrawn: distinct
identifiers measure scheme variety, not entity count). The fix is upstream of
it, in three bounded units, filed as **issue 351**:

1. **Census**: walk `provisional = 1 AND country IS NULL` rows grouped by
   `name_norm`; report groups ≥ 2, rows in them, mentions on them, and the top
   groups — the corpus-wide size of what the 168 keys sampled.
2. **Prevention**: reuse the standing `(name_norm, NULL)` provisional row for a
   country-less mention when its N2 key is under the wall — the Taufkirchen
   shape (over the wall today) keeps minting, the Burghausen shape stops.
3. **Repair**: fold the standing identical-name NULL-country provisional rows
   into one per name (dissolve-adjacent machinery, `org_merge_log` rule `p0`),
   which also drops those keys back under the wall so the E0 rule, the R3
   rescue and the corroboration path all see them as what they are.
