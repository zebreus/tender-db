# 263 — eForms-DE value completeness (30–44 %) lags the EU SDK eras (55–80 %): extraction gap or publication reality?

Status: CLOSED 2026-08-22 — PUBLICATION REALITY on every sampled prong; the extraction is
acquitted. Archive-sampled prefix-agnostically (`currencyID=` marks ANY UBL money element):

- **DÖE sdk-0.1**: 0 of 50 sampled members carry any money element — the 667k-version island's
  0.0 % is the source's own shape (the pre-release tooling never published values).
- **eforms-de-1.1**: 8 of 50 (16 %) at source vs 30.1 % in the layer — the LAYER READS HIGHER
  than the raw member rate (version chains merge several notices; CANs state values more often
  than CNs), which is the opposite of an extraction gap. German buyers simply rarely state
  estimated values — consistent with national practice.
- **EU-SDK control**: 40 of 80 (50 %) at source vs 55-71 % in the layer — same layer-above-source
  shape; the method validates.

Section 1 of the report now carries the caveat with the sampled rates, so the DE-vs-EU value gap
stops reading as a mapping suspicion. One method note for the record: de-1.1 members do NOT ride
the TED archive at all (0 found in three TED daily bundles) — German eForms flow exclusively
through DÖE, which is where the sample had to come from. Was: needs-triage — filed 2026-08-21
(owner, requested by Lennart: investigate data quality).
Kind: data-quality investigation (field completeness, `value`)
Blocked by: —
Relates to: 230 (the measurement this reads), 195 (the last value-mount campaign — its lesson:
"missing value" has repeatedly turned out to be an unmapped MOUNT, not an absent fact), 251 (the
adjacent amount work), 172 (cross-era caveats)

## The anomaly (weekly data-quality run, 2026-08-21, section 1)

Share of versions carrying ANY amount, side by side:

    eforms-de-1.1   30.1 %      eforms-sdk-1.10   62.5 %
    eforms-de-1.2   40.8 %      eforms-sdk-1.12   64.3 %
    eforms-de-2.0   36.4 %      eforms-sdk-1.13   71.0 %
    eforms-de-2.1   43.9 %      eforms-sdk-1.8    80.1 %
    eforms-de-1.0    0.0 %      DÖE sdk-0.1        0.0 %

Same eForms family, same era of procurement, HALF the value coverage — and the German dialect is
consistently the low side, which is exactly the shape issue 195 kept finding: a dialect mounting a
known element at an unmapped position. But it is ALSO the shape of a real publication difference
(German buyers may genuinely publish values less often — national law does not require estimated
values on many notice types). The two explanations have opposite consequences and the number
cannot distinguish them; that is the investigation.

Separately: DÖE sdk-0.1 at 0.0 % over 667,502 versions and eforms-de-1.0 at 0.0 % over 31 are
absolute zeros, which smell like unmapped mounts full stop (a population that NEVER states a value
across two-thirds of a million notices is not plausible publisher behaviour).

## How to investigate (the issue-195 method, era-scoped)

1. Sample ~50 raw payloads per low era from the archive (value-less per the layer), grep for the
   money-bearing elements (`cbc:EstimatedOverallContractAmount`, `efbc:` value extensions, the
   DE-specific mounts) prefix-agnostically — the issue-100 lesson: a namespace-prefix grep lies.
2. Split the sample: payloads with NO money element (publication reality — record the rate) vs
   payloads WITH one the parse layer missed (extraction gap — file the mount, per 195's shape).
3. For sdk-0.1's absolute zero: same, but 20 payloads suffice — a zero is cheap to falsify.

## Acceptance

Either a filed mount-gap issue per finding (with the fixture the grep produced), or a recorded
"publication reality" verdict per era with the sampled rate — so the completeness table stops
being ambiguous about whose gap the number is. The verdict lands in this file and a caveat line in
the report render if warranted (like section 6's VAT-mix caveat).
