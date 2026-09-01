# 188 — eforms-sdk-0.1 awards are 98% unchained (136,941/139,684); sdk-1.0 is 99.9%: the issue-27 rule fires

Status: DIAGNOSED-HONEST (2026-08-15 — sdk-0.1 98% is the true number, source publishes no folder key; panel now explains it. sdk-1.0 left for its own check)
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

## Attribution (2026-08-15, orchestrator) — sdk-0.1 is HONEST at the source

Split the 136,941 unchained sdk-0.1 awards on the canonical layer (bounded /v1/sql):
**132,519 (96.8%) are islands** (procedure_key IS NULL); only 5,090 are
keyed-but-single-version. So the cause is a missing grouping key, not the
single-notice-procedure story — candidate 1, but resolved at the SOURCE, not our parse.

Pulled the DÖE 2023-01 monthly (14,300 notices: 11,292 cn-standard, 2,942 can-* awards).
Of the 2,942 award notices, only **454 (15%) carry any ContractFolderID value** — all 454
are valid uuids. The other **2,488 (85%) publish an empty `<ContractFolderID/>`** (self-
closing, no value), and carry no alternative prior-notice reference (the only reference-
shaped element is `RegulatoryDomain`, a legal-basis code). So 85% of DÖE awards have no
key and nothing else to chain on — they are genuinely standalone award records. The 15%
with a real uuid folder chain correctly (they're among the keyed/chained).

Conclusion: the eforms-sdk-0.1 near-100% unchained rate is the HONEST number
(issue-27 candidate 2), NOT a fixable parse gap or an over-strict is_uuid gate. Nothing
to fix in code; the deliverable is documentation. The award-linkage panel now carries a
note distinguishing chain-capable eras (where a high rate is a defect) from sources that
publish awards without a cross-reference, naming sdk-0.1 with this cause (renders on next
deploy). Retires the issue-27 trigger for this era.

Not done here: sdk-1.0 (694 awards, 99.9%, TED) — small, below dashboard-impact
threshold; likely the same "early eForms award without a populated BT-04" shape but
warrants its own one-notice check before asserting. The mid-range eras (sdk-1.7 33%,
1.10 26%, 1.14 56%, r208 32%) are below the >90% trigger and a separate characterization
pass. Left as a smaller follow-up.

## The sdk-1.0 check this issue left open (2026-09-01)

Status line said "sdk-1.0 left for its own check". Done, and **it does not have
sdk-0.1's explanation.**

### Measured, from the live dashboard

```
eforms:eforms-sdk-0.1     141,583 awards   138,832 unchained   98.06%
eforms:eforms-sdk-1.0         713 awards       711 unchained   99.72%   <-
eforms:eforms-sdk-1.3       2,190 awards       615 unchained   28.08%
eforms:eforms-sdk-1.6      18,144 awards     4,246 unchained   23.40%
eforms:eforms-sdk-1.7      90,647 awards    29,395 unchained   32.43%
eforms:eforms-sdk-1.13    173,321 awards    57,628 unchained   33.25%
eforms:eforms-sdk-1.14     15,830 awards     7,962 unchained   50.30%
```

**sdk-1.0 sits with sdk-0.1, not with its own family.** Every other sdk-1.x runs
23–50%; sdk-1.0 runs 99.72%.

### Why sdk-0.1's explanation does not transfer

sdk-0.1's 98% is honest because the source publishes no BT-04 at all — and the
vendored inventory says so:

```
crates/ingest/sdk/fields-sdk-0.1.json   BT-04-notice: ABSENT
crates/ingest/sdk/fields-1.0.0.json     BT-04-notice: PRESENT
crates/ingest/sdk/fields-1.3.0.json     BT-04-notice: PRESENT
```

sdk-1.0 **has a vendored inventory and that inventory defines BT-04**, exactly
like sdk-1.3 which chains 72% of its awards. And the code path confirms there is
no dialect fallback in play: `procedure_key` tries `BT-04-notice` first, then the
national folder ids gated behind `is_sdk01_profile` (exact match on
`eforms:eforms-sdk-0.1`) and `is_de1_profile` (`eforms-de-1.`). **sdk-1.0 matches
neither**, so BT-04 is its only key — and 711 of 713 notices are not producing one.

Two of the 713 DO chain, so the mechanism works when the value is present.

### What is NOT known

Whether the sdk-1.0 notices in the corpus genuinely omit BT-04 in their XML (a
real early-adopter gap, source-side, but for a different reason than sdk-0.1), or
whether something in the sdk-1.0 path — profile detection, inventory selection —
fails to reach a field that is there. **The count cannot tell; only the notices
can.**

### Proportion, stated so nobody over-reacts

713 awards out of roughly 4.2M is **0.017%** of the corpus. This is a correctness
curiosity and a possible parse gap, not a data emergency, and it should be
prioritised as such. The value in resolving it is mostly that sdk-1.0 is the
*earliest real SDK release* — whatever is wrong here may be the same shape as a
gap in a later version that matters more.

### Next step

Inspect the parse layer of a handful of sdk-1.0 notices for `BT-04-notice`
presence — a bounded read. If the field is there and unextracted, it is ours; if
it is absent from the XML, sdk-1.0 joins sdk-0.1 as honest and the panel should
say so for both.
