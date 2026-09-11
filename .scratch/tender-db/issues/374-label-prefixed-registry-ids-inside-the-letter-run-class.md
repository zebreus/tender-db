# 374 — real registry numbers wearing a label prefix, found inside the letter-run class

Status: DONE 2026-09-11 — unit 1 shipped and repaired, units 3 and 4 measured and declined, and the
only thing left is owned by issue 329, which records it (the `HRA16270` key that became shared once
this issue's label strip landed, and the 3,215 exact duplicate `(DE, vat, DEnnnnnnnnn)` rows). A
handoff the receiving issue does not know about is not a handoff; this one it does.
Was: ready-for-agent — **UNIT 1 SHIPPED, DEPLOYED AND REPAIRED 2026-09-09 (`a398a4d`)**;
**UNITS 3 AND 4 MEASURED AND DECLINED** the same firing. See "Unit 1 DONE" and "Units 3+4
DECLINED". What remains is the German exact-duplicate residue the strip made visible, which is
E0's business (issue 329) rather than this issue's.
Kind: defect (organization layer — identifier RECOVERY, the additive mirror of 365's prevention)
Blocked by: —
Relates to: 365 (unit 3's read turned these up while deciding the letter-run class), 328/359/363
(the label-strip vocabulary and the `repair-label-prefixes` job that already do exactly this for
NIP/PIVA/CIF/NIF/YTUNNUS/…), 345 (`repair-renormalised-identifiers`, the standing-stock catch-up
shape), 327 (the Austrian GLN class, a neighbouring wrong-scope class)

## What the read found

Deciding whether to condemn the letter-run class (365 unit 3) required classifying it, and the
classification turned up a sub-class that wants the OPPOSITE treatment from the rest: identifiers
that are perfectly good registry numbers with the register's NAME glued to the front. Grouping
the class by its leading letter run over `id <= 3000000`:

| prefix | orgs (window) | what it is | wanted |
| --- | --- | --- | --- |
| `BERICHTSEINHEITID` | 38 | statistical reporting unit | condemned in 365 unit 3 ✔ |
| `LEITWEGID` | 36 | e-invoicing routing address | condemned in 365 unit 3 ✔ |
| `SPRAWA` | 24 | Polish *sprawa* = case/matter number | **unread** |
| `CVRNR` | 11 | Danish CVR number under "CVR-nr" | **strip** |
| `CSKDZMIZP` | 9 | Polish procurement-unit abbreviation? | **unread** |
| `AMAZPEPN` | 9 | ditto | **unread** |
| `SIRET` | 8 | French SIRET under its own label | **strip** |
| `OSZP` / `ZDWNZP` / `PZDWWZP…` / `MSWPN` | 3–6 each | Polish `ZP` (zamówienia publiczne) unit refs | **unread** |
| `EKRSZ` | 4 | Hungarian? | **unread** |
| `REGISTRIERUNGSNUMMER` | 3 | German "registration number" label | **strip** |
| `KODNUTSPL` | 3 | a **NUTS region code** under a label | **condemn** (location-scoped) |
| `HANDELSREGISTERHRB` | 3 | German commercial register HRB under its label | **strip** |

These are window counts (`id <= 3000000`), so treat them as shape evidence and relative
frequency, not corpus totals — the corpus figure for the whole letter-run class is 21,375 after
365 unit 3.

## Why it matters, and why it is NOT 365

365 is about refusing values that must not be merge keys. This is the mirror: `CVRNR29189498` and
`SIRET78467169500087` carry a **real, checksummable, jurisdiction-scoped registry number**, and
today the label glued to the front makes them their own private key. Every such row is a MISSED
merge against the same body published without the label — the failure mode 328/359/363 were built
to fix, for a vocabulary that simply does not yet contain these four entries.

Note the trap this sits next to: every one of these values has a ≥4-character letter run, so a
blanket letter-run condemnation (the tempting reading of 300's parked rule) would have destroyed
recoverable registry evidence. 365 unit 3 records that reasoning; this issue is the other half.

## Unit 1 DONE (2026-09-09, `a398a4d`) — and the Handelsregister half was not what this issue said

I filed unit 1 as "extend the strip vocabulary with four entries". That was right for three of
them and **wrong for the fourth**, which is worth correcting rather than quietly fixing:
`HANDELSREGISTER` was **already in the vocabulary**, and `label_prefix_stripped` already returned
the right answer.

The strip was being computed and then **thrown away**. The `recognisable` guard admits a
non-national kind, pure digits, or (since 359) a Spanish CIF. `HANDELSREGISTERHRB93017` strips to
`HRB93017`, which is none of those — so the labelled form survived as its own live merge key, and
the same company published bare as `HRB93017` got a second org row. Widened by one shape, exactly
as 359 did for the CIF: an **anchored** German register division plus digits. The anchoring is
what keeps it safe — the two leftover fragments the guard exists for, `NRHRB64128` and
`ARNHEM09155985`, do not start with a division and still fail.

The five divisions are the ones the corpus carries in bare form, so a labelled row stripping to
one of them rejoins real rows rather than a guessed shape: **HRB 11,439, HRA 2,132, VR 326,
PR 258, GnR 22**.

`CVRNR`, `SIRET` and `REGISTRIERUNGSNUMMER` were genuinely just absent. Each strips to pure
digits, which the guard has always admitted.

### A committed conclusion corrected

`the_handelsregister_strip_is_correct_and_currently_inert` asserted that both the labelled and
bare forms return `None`, and concluded from that pair that "the v2 gate condemns `HRB…` values
on their own account… the class is simply out of reach until the gate's view of `HRB` changes".

That conclusion was false, and the cause was its fixture. `HRB12345`'s digits are an ascending
run, so `suspicious_digit_run` condemns **both** forms whatever the strip does — the test was
measuring the sequence rule and reading the result as a fact about `HRB`. `HRB93017` is not
condemned at all. Renamed and rewritten with a realistic register number.

**This fixture trap is now three-for-three**: it produced this wrong committed conclusion, and it
invalidated three of my own negative controls during 365 units 2 and 3. Any test asserting "X is
not condemned" must use non-sequential digits or it asserts nothing. Noted in the tests
themselves so the next reader does not rediscover it.

One fixture MOVED rather than re-pointed: `HANDELSREGISTERNRHRB64128` sat in the
leftover-fragment test, cited as producing `NRHRB64128`. True when the vocabulary's longest match
was `HANDELSREGISTER` (15) — but the list has since gained `HANDELSREGISTERNR` (17), so
longest-match consumes the label's `NR` and the remainder has been the clean register value for a
while; only the guard kept the row labelled. The value reads "Handelsregister-Nr. HRB 64128", so
`HRB64128` is correct. The other four fragments in that test still fail, including both prod
values from the issue-328 dry plan.

### The repair, and the caveat it prints itself

`repair-label-prefixes` dry (842) then wet (843): **7,085 rows carry a publisher label, 715
planned, APPLIED 715, skipped 0**, 6,370 already agreed with the re-parse. **431 of the 715 land
on an identity that ALREADY stands** — those are the reunions this issue was filed for.

The job's own note is the important caveat and it is not a defect: for the German class this is
**not a merge R2 will perform**, because `crosswalk::canonical_key` has no DE arm at all
("court-scoped registers", a pinned negative — the same HRB number in two courts is two
companies). So those rows become **visible exact duplicates rather than folded ones**. That is
still a strict improvement: an invisible split becomes a same-triple duplicate that the
duplicate-identity census (347) surfaces and E0 (329) can fold when the names agree. Recorded as
the remaining work, on 329 rather than here.

Verified after the wet run — the labelled forms are gone except where the guard rightly refuses:

| prefix | before | after |
| --- | --- | --- |
| `CVRNR` | 165 | 1 |
| `SIRET` | 540 | 32 |
| `REGISTRIERUNGSNUMMER` | 31 | 10 |
| `HANDELSREGISTER` | 87 | 65 |

The 108 survivors are the mangling the guard exists to prevent — `HANDELSREGISTERARNHEM09155985`
and `HANDELSREGISTERAMTSGERICHTESSENHRB11082` carry a court NAME where digits should be, so they
keep what the publisher wrote. And the published string is untouched throughout:
`organization_mentions.raw_identifier` still reads `Cvr-nr.: 29 77 69 38` and
`CVR. nr. 26 77 06 45`, spacing and punctuation intact.

## Units 3+4 DECLINED by measurement (2026-09-09)

Every remaining candidate prefix was measured corpus-wide with the method that settled 365 unit 1
(share of rows carrying ≥2 distinct mention names, against the **14.7 %** corpus baseline, plus
the worst row). **All of them decline**, and two of my own filed hypotheses are falsified:

| prefix | orgs | ≥2 names | max | mentions | verdict |
| --- | --- | --- | --- | --- | --- |
| `EKRSZ` | **6,569** | 17.6 % | 8 | 125,887 | leave — at baseline, and doing real linking |
| `SPRAWA` | 56 | **1.8 %** | 2 | 71 | leave — *more* unique per body than average |
| `KODNUTS` | 12 | 16.7 % | 4 | 19 | leave — at baseline, 19 mentions total |
| `ZDWNZP` | 12 | 50.0 % | 3 | 28 | leave — ratio high, but 12 rows / 28 mentions |
| `OSZP` | 7 | 0 % | 1 | 8 | leave |
| `MSWPN` | 3 | 0 % | 1 | 3 | leave |

- **`SPRAWA` was my own guess and it was wrong.** I filed it as "almost certainly a case-file
  reference (condemn)". At 1.8 % multi-name it is *eight times more* one-body-per-key than the
  average identifier. A Polish case number is apparently issued per counterparty, so it works as
  a key. Condemning it would have been a pure loss.
- **`EKRSZ` is 1,600× bigger than the window suggested** — 4 rows in the `id <= 3000000` sample,
  **6,569** corpus-wide (a Hungarian EKR e-procurement number). Barely above baseline and
  carrying 125,887 mentions. Exactly the hex-class shape that issue 312 spared.
- **`KODNUTSPL` was an a-priori-obvious condemn that the data declines.** A NUTS code *is* a
  region, not a party, so the reasoning of 365 unit 3 says refuse it — but the class is 12 rows
  and 19 mentions at baseline diversity, so a rule would prevent approximately nothing. Declined
  on size, with the reasoning recorded so it can be revisited if the class grows.

The unit-3/4 lesson is the same one that came out of 365 unit 3: **the window is not the corpus,
and the a priori argument is not the measurement.** Both prefixes I was most confident about
(`SPRAWA` condemn, `KODNUTSPL` condemn) failed, and the one I had no opinion on (`EKRSZ`) turned
out to be the biggest class in the set.


## Units

1. **Extend the strip vocabulary** in the label-prefix table with `CVRNR`, `SIRET`,
   `REGISTRIERUNGSNUMMER` and `HANDELSREGISTERHRB`, each shape-guarded the way 359/363 guarded
   theirs (CVR is 8 digits, SIRET 14, HRB is `HRB` + digits — so `HANDELSREGISTERHRB93017` must
   strip to `HRB93017`, not to `93017`, or it collides with a bare serial). Red-first tests with
   real-shaped digits: an ascending run like `CVRNR12345678` is condemned by the `sequence` rule
   whatever the strip does, so it asserts nothing — this trap caught two tests during 365's work.
2. **Run `repair-label-prefixes` dry → wet**, then R2, then `project`, and record the merges the
   strip reunites. Expect the same reconciliation discipline as 365: the dry/wet fresh-provisional
   split differs by construction, so compare the totals and the class counts, not that line.
3. **`KODNUTSPL`** — a NUTS code is a location, not a party. Small (3 in-window), and the natural
   home is 365's `ROUTING_SCOPE_PREFIXES`, which already exists. Measure it corpus-wide the way
   365 unit 3 did (rows, ≥2-name share against the 14.7 % baseline, and the max-names row, which
   is the number that actually decides it), then add or record why not.
4. **The unread Polish/Hungarian sub-classes** — `SPRAWA`, `CSKDZMIZP`, `AMAZPEPN`, `OSZP`,
   `ZDWNZP`, `PZDWWZP…`, `MSWPN`, `EKRSZ`. `SPRAWA` is almost certainly a case-file reference
   (condemn), and the `ZP` family looks like procurement-unit abbreviations, but none of that is
   measured. Same read as unit 3: name diversity against baseline, plus the worst row.

## Done when

- `CVRNR`/`SIRET`/`REGISTRIERUNGSNUMMER`/`HANDELSREGISTERHRB` rows carry the register number the
  publisher meant, pinned by tests, and the repair has run so standing rows caught up;
- `KODNUTSPL` and the `SPRAWA`/`ZP` families each have a measured verdict on this issue — wired or
  explicitly declined with the number that declined it;
- the weekly org report still shows `letter_run` shrinking only by classes that were decided, so
  nothing in it moves unobserved.

*One issue because:* every row here reached its current state through the same admission rule as
365 — a label-bearing string surviving the shape filters and becoming a key — but the remedy is
recovery rather than refusal, and it routes through machinery (328/359/363) that already exists.
