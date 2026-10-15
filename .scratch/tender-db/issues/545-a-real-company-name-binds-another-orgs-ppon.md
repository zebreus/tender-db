# 545 — a real company name on another organization's PPON binds that organization (Jeol on UKRI)

Status: needs-triage — filed 2026-10-15 from issue 529's design panel. Next: measure the class before choosing
anything.
Kind: data quality (organization identity)
Relates to:
- 529 (unit 3: the FASTER `Individual` on UKRI's PPON, accepted);
- 452 and 466 (identifier verdicts; the wrong-identifier class);
- 522 (the exact-triple bind comes before verdicts).

## Verify

    ssh root@zebreus.click "curl -s 'http://127.0.0.1:8080/v1/tenders/8664993'" | head -c 600

- **done**: the supplier `Jeol (U.K.) Limited` on notice 46834978 is served under its own organization, not UK
  Research & Innovation (30913706).
- **open (2026-10-15)**: tender 8664993 serves UKRI as a bidder. The mention `Jeol (U.K.) Limited` carries
  `GB-PPON-PDQJ-7126-JDHG`, UKRI's PPON, on 46834978, whose buyer is UKHSA.
  Confirmed from the public API on 2026-10-15 04:5x UTC: `/v1/tenders/8664993` ("Service and Maintenance x JEOL",
  060996-2026) serves the Tenderer as org 30913706 `UK Research & Innovation` with `mention_name` `Jeol (U.K.)
  Limited`. Its one bid names UKRI, and the buyer is 30914819 UK Health Security Agency.

## What was found

The resolver binds the exact identifier triple first (canonical.rs ~20063). A supplier that publishes another
organization's PPON lands on that organization, whatever its name says. Here a real company name, an instrument
maker, sits on UKRI's PPON, probably a copy-paste in the buyer's form. Unlike 529's stand-in `Individual`, the name
is a real, different organization. So this is a name-versus-identifier disagreement, not a placeholder.

A 452 `wrong` verdict cannot move the bind, for the reason 522 unit 1 gives. `org-head-drift` (issue 524) counts
`stray_mentions` by scheme: 1,162 GB-PPON stray mentions on 955 orgs (job 2492). Some share of those is this class.

## Units

1. **Measure.** From 524's census samples, or a bounded per-scheme read: how many GB-PPON (and COH) mentions carry a
   name that agrees with neither the org's head nor any other mention of it, and how many of those name a company
   that exists elsewhere in the corpus under its own identifier.
2. **Decide** from the measurement. Options to weigh:
   - a reviewed rehoming of the measured mentions (issue 317), which moves recorded mentions only;
   - a resolver rule that refuses the triple when the name clearly names a different, identified org;
   - accept as publisher error.
