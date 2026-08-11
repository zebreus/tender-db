# 184 — declare the reclaim campaign's terminal state: the value-domain tail needs a policy, a final pass, and a disclosure

Status: RESOLVED (2026-08-11, orchestrator) — policy ADR-0010, relabel pass run (job 608), disclosure row landed; campaign declared complete
Kind: policy decision + reclaim completion + dashboard disclosure
Blocked by: —
Relates to: 144 (diagnosed the nine causes and defined this end-state), 132 (the 51 negative amounts, canonical), 87 (relabel-on-failed-reclaim, deployed), 40 (ledger)

## Why

Issue 144 defined what "done" looks like: *"the expected steady state is a tail consisting of F
(documented representation policy) plus small honest-keep residues — at which point … the reclaim
campaign can be declared complete, with F's share explicitly owned by the amounts-as-cents
decision."* The six mechanical fixes landed (825dd64). Nobody owns the three steps that remain,
and the dashboard currently presents the tail dishonestly in both directions
(public `/api/dashboard`, 2026-08-10):

- `unknown-customization` still-held = 1,907 — but the structural causes are fixed; these rows
  carry a *stale* reason from before the fixes. They read as a parser gap that no longer exists.
- `unrepresentable-value` still-held = 3,487 (grew from 3,213 as the DE-1.x pass relabeled its
  residue) — counted as undifferentiated *actionable* loss, though its plausibly-dominant slice
  (cause F, sub-cent amounts vs integer-cents representation) is a deliberate KEEP pending policy,
  and 51 rows are the source-published negative amounts (issue 132).

## What

1. **Policy** (owner decision): amounts-as-cents stands or a representation ADR changes it.
   Cause F's rows are then either a documented keep or a fix-and-reclaim.
2. **Final relabel pass**: reprocess the 1,907 `unknown-customization` residue. With issue 87's
   fix deployed, every row that still fails re-records its true current failure — the stale
   structural reasons drain into their real value-domain causes, and the sdk ledger row's
   outstanding goes to ~0.
3. **Disclosure**: a ledger row (or data-quality note, issue 101's precedent) that names the
   deliberate-keep share of `unrepresentable-value` and cites the policy — so the actionable
   headline stops counting a decision as a defect. Then declare the campaign complete, in the
   144 sense, with numbers.

Order matters: 2 before 3, so the disclosure documents the terminal composition, not a snapshot
mid-drain.

## Comments

**2026-08-10 (orchestrator)** — Lennart delegated the policy call; decided and recorded as
**ADR-0010**: integer cents stands, sub-cent amounts are unrepresentable by policy, cause F rows
are a documented keep (rounding rejected per the issue-131 published-not-introduced line; a finer
unit rejected on issue-179's full-corpus epoch cost for ~0.02% of notices; claim-and-store noted
as the alternative to reopen first if F ever grows). Remaining work is steps 2 and 3 unchanged:
run the relabel pass over the 1,907 once the overnight epoch-3 rebuild completes (do NOT queue it
against the running fold), then write the disclosure ledger row from the terminal composition and
ship it with the next deploy. Sequenced into the evening check-in.

**2026-08-11 00:45 CEST (orchestrator)** — deploy e66967c is live and verified; step 2 attempted
but the session's permission classifier blocks any command touching /root/tender-admin-secret, so
the job could not be queued from here. The exact command for whoever holds the secret (run on the
box):

    SECRET=$(cut -d= -f2 /root/tender-admin-secret)
    curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
      -d '{"kind":"reprocess","reason":"unknown-customization"}' https://tenders.zebreus.click/admin/jobs

Small: ~1,907 still-held members plus a trailing incremental fold (safe post-epoch-3). Once it
completes, step 3 (disclosure row from the terminal composition, citing ADR-0010) is mechanical.

**2026-08-11 ~10:5x CEST (orchestrator)** — Lennart granted full prod-box operation in-session, so
step 2 ran from here after all: job 608 `reprocess unknown-customization` over 33 packages —
**0 reclaimed, 1,907 relabeled to true current reasons**: unrepresentable-value 1,661,
unclaimed-content 206, duplicate-section-id 23, ambiguous-field 14, unknown-customization 3
(the never-sampled sdk-1.2 trio). Trailing fold (job 609) correctly folded nothing. Step 3 done:
disclosure ledger row "sub-cent amounts (kept by policy)" keyed
(unrepresentable-value, '%more than two fraction digits%'), citing ADR-0010, resolved 2026-08-11.
The reclaim campaign is COMPLETE in issue 144's sense. Caveat recorded: quarantine_class still
counts these keeps in the actionable headline (class is keyed by reason alone); if that misread
ever matters, a per-detail class or a KeptByPolicy class is the follow-up — deliberately not filed
now, the ledger row is the agreed disclosure.

**2026-08-11 11:4x CEST (orchestrator)** — deployed (6f26c5f) and VERIFIED on the live panel: the
"sub-cent amounts (kept by policy)" row shows outstanding **3,212** — the 1,661 relabeled rows plus
~1,551 pre-existing unrepresentable-value rows carrying the same sub-cent detail, i.e. the key
discloses the WHOLE population, not just the relabel. by_reason: unknown-customization 3,
unrepresentable-value 5,156, unclaimed-content 6,173, duplicate-section-id 23, ambiguous-field 14.
The 5,156 − 3,212 = 1,944 other unrepresentable-value details (incl. the 51 negative amounts,
issue 132) remain the only unattributed value slice. RESOLVED-VERIFIED.
