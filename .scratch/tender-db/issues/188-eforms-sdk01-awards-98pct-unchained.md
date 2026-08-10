# 188 — eforms-sdk-0.1 awards are 98% unchained (136,941/139,684); sdk-1.0 is 99.9%: the issue-27 rule fires

Status: needs-triage
Kind: reference-resolution defect (pre-registered trigger)
Blocked by: —
Relates to: 27 (the >90%-at-full-data rule), 12 (DÖE source and the empirical sdk-0.1 profile), 34 (sdk01 ContractFolderID key), 100/101 (the DE-1.x winner gap — a different metric, do not conflate)

## Why

Same pre-registered trigger as issue 187, different eras (public `/api/dashboard`, 2026-08-10):

    eforms:eforms-sdk-0.1   awards=139,684   unchained=136,941   ratio=0.980
    eforms:eforms-sdk-1.0   awards=694       unchained=693       ratio=0.999

sdk-0.1 is the DÖE empirical profile (issue 12): 139,684 awards is not a corner — it is the bulk
of DÖE's award history reading as never-linked. Candidate causes: the empirical field inventory
may not map whatever DÖE uses as the procedure key where TED eForms use BT-04/ContractFolderID
(issue 34 established that key for sdk01 — verify it actually populates the grouping for this
profile at full data); or DÖE genuinely publishes awards without a resolvable prior reference, in
which case ~98% is the *honest* number and belongs beside a documented explanation, not silence.
sdk-1.0 (694 awards, TED) is small but maximally broken and may share a cause with its
0.1-adjacent vintage.

For context, not covered by this issue: mid-range eras sit at 23–56% unchained (sdk-1.7 33%,
1.10 26%, 1.14 56%, r208 32%) against the research-predicted ~17% for r209. Below the
pre-registered threshold, unexplained, worth a characterization pass once the two >90% eras are
attributed — the same fix may move them.

## What

1. Attribute per era: take 10 known award/prior pairs from the raw notices and trace where the
   chain key diverges (parse layer vs grouping vs referent-not-held).
2. Fix or document. If a fix lands, the affected eras need a refold (see issue 179's cost note).
3. Either way the panel's story gets written down — a 98% red row with no explanation on the
   dashboard is exactly the unowned-gap shape this audit exists to end.
