# 374 — real registry numbers wearing a label prefix, found inside the letter-run class

Status: ready-for-agent (filed 2026-09-09 from issue 365 unit 3's composition read)
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
