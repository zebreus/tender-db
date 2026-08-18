# 236 — eForms EU awards chain to their contract notice at 44–77 %, where eForms-DE manages 98–100 %

Status: needs-triage — measured 2026-08-18 in the first complete eleven-query report (job 732)
Kind: identity/chaining gap, suspected cross-era boundary
Blocked by: —
Relates to: 187 (INTERNAL_OJS 100 % unchained), 188 (sdk-0.1 98 % unchained), 58 (legacy OJS closure —
the chaining machinery for the eras on the other side of the suspected boundary), 12 (procedure keys),
230 (the measurement)

## What

Section 2 of the first complete data-quality report, award Tenders chained back to a contract notice:

    era                                awards  unchained   linked
    eforms-de-1.1                      65,207         18   100.0%
    eforms-de-1.2                      29,364         70    99.8%
    eforms-de-2.0                      33,658        502    98.5%
    eforms-de-2.1                      37,151        818    97.8%
    ---
    eforms-sdk-1.10                    66,748     17,045    74.5%
    eforms-sdk-1.11                    39,597     13,495    65.9%
    eforms-sdk-1.12                    94,251     37,382    60.3%
    eforms-sdk-1.13                   172,450     60,831    64.7%
    eforms-sdk-1.14                    15,129      8,407    44.4%
    eforms-sdk-1.7                     89,750     29,460    67.2%
    eforms-sdk-1.8                     61,732     20,289    67.1%
    eforms-sdk-1.9                     36,243     13,720    62.1%
    eforms-sdk-1.6                     18,136      4,269    76.5%
    eforms-sdk-1.3                      2,190        616    71.9%
    eforms-sdk-1.0                        713        711     0.3%

Both families are eForms. Both key on BT-04. Yet the German customization chains essentially
everything and the EU profile loses a third to a half — **roughly 246,000 unchained award Tenders**
across the sdk-1.x eras. That is not a small tail, and it is not the two known island cases (issues
187 and 188 cover internal-ojs and sdk-0.1, both visible in the same table at 3.5 % and 2.0 %).

## The leading hypothesis, and how to kill or confirm it

**A cross-era boundary rather than an eForms defect.** eForms became mandatory for EU publication in
late 2023, so a 2024–2026 eForms CAN routinely belongs to a procedure whose CONTRACT NOTICE was
published earlier as TED r2.0.9. The legacy notice carries no BT-04, so the eForms CAN has nothing to
chain to on the eForms key — the CN is reachable only through the legacy OJS closure, which keys
differently. eForms-DE would not show this because DÖE's German channel started publishing eForms-DE
CNs and CANs together, so both ends of a DE procedure are usually in the same vocabulary.

Two predictions that make this falsifiable:
1. **Unchained share should fall as the SDK version rises** — later SDKs mean later notices, and a
   later CAN is more likely to have an eForms CN behind it. The table does NOT obviously show that
   (1.14 at 44.4 % is the WORST, 1.6 at 76.5 % among the best), so either the hypothesis is wrong or
   SDK version is a poor proxy for date. Check publication dates directly before believing either.
2. **An unchained eForms CAN should have a findable r2.0.9 CN** for the same procedure — same buyer,
   same CPV, an OJS reference in its own text. Pull ten unchained CANs from sdk-1.13 and look.

If it holds, the fix is a cross-vocabulary bridge (eForms CAN → legacy CN), which is a real design
question, not a mapping tweak — and it should be an ADR, since it decides whether one procedure spanning
two publication regimes is one Tender.

If it does not hold, the cause is inside eForms and the next thing to check is whether these CANs carry
BT-04 at all (a missing key looks identical to an unmatched one in this metric).

## Note on `eforms-sdk-1.0` at 0.3 %

713 awards, 711 unchained. Tiny, and almost certainly its own story — sdk-1.0 is an early-adopter
trickle. Worth a glance while investigating the rest, not worth its own issue yet.

## Acceptance

- The publication-date distribution of unchained vs chained eForms EU award Tenders, recorded here —
  that alone confirms or kills the cross-era hypothesis.
- For a sample of ten unchained CANs: does the procedure have an earlier legacy CN in the corpus, and
  does the CAN carry a BT-04?
- A recorded decision: bridge the vocabularies (ADR), or accept the split with the reason stated in the
  report's own text so the number stops looking like an unexplained defect.
