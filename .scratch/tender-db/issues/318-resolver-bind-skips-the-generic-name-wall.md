# 318 — The resolver's anchor bind applies the R3 bar without the R3 wall

Status: DONE 2026-08-31 (rev 7ae5f6a). Wall live and ARMED on prod, verified
positively. The frequency answer is ZERO anchor binds per day — see the reading
Kind: correctness (organization layer, ingest path)
Relates to: 316 (built the wall), 300 (§4.1, Stage 3 prevention), 234

## The disagreement

Two code paths implement "the R3 bar", and since 76d39e6 they no longer
agree about what corroboration means.

- **The batch merge arm** (`match_org_null_country_r3`) now refuses to
  corroborate on a GENERIC name — one shared by more orgs than the stoplist
  cap — unless the anchor's scheme hard-checksums (design §4.1, issue 316).
- **The resolver's ingest-time prevention path** (`canonical.rs`, the
  Stage-3 pre-probe around the mention bind) binds a country-less mention to
  a standing org "at the R3 merge arm's own bar: unique anchor, sole
  target, exact name corroboration" — its own comment says so — and never
  consults the wall.

So the same evidence that the batch arm now denies still binds at ingest.
The batch arm is the one that leaves an audit row; the resolver is the one
that runs on every notice.

## The class, with the panel's corrected specimen

The disagreement needs a SOFT-scheme anchor (a hard one takes the design's
exemption in both paths and they agree). The panel's first specimen was
wrong — SE:orgnr is hard — and the corrected class is the soft schemes
`checksum_anchors` can emit that `canonical_key_flat` also keys:
FR:siren, PL:nip, BE:kbo, DK:cvr, SI:davcna. A country-less mention
carrying one of those, whose name normalizes to a key over the cap (the
"tribunal administratif de X" / "european commission" shapes are exactly
this), binds at ingest to a standing org the batch arm would refuse to
merge it with.

## Why it is filed rather than fixed

The fix is real but it lands on the INGEST HOT PATH, and it needs its own
dry-first measurement and its own panel round rather than a late edit inside
the issue-316 batch:

1. Thread `hard_scheme` + `stoplist_cap` into the resolver the way the merge
   arm takes them (injected fns; linguistics stay out of store).
2. The wall's LENIENT default has to stay lenient here and cannot become a
   refusal: the resolver has no "refuse" to return, and `org_match_keys` may
   be empty or mid-build during ingest. Unseen key ⇒ not generic ⇒ today's
   behaviour, which is the only safe failure mode on this path.
3. Measure first: how many binds per fold actually take the anchor path, and
   how many of those carry a generic name? The probe is one indexed count
   per anchored bind, so the cost question is the frequency question.

## What it is NOT

Not a false-merge report. Nobody has shown a wrong bind from this class yet;
what is shown is that two implementations of one rule disagree, and the
design says the stricter one is correct. Measure the class before deciding
how hard to close it — the issue-312 discipline.


## MEASURED (2026-08-31, prod job 518, deploy b4942f7)

`anchor-wall-census` walks the N2 key groups the Stage-4 scan already
stoplists — the ones OVER the cap, which that scan discards — and asks of the
orgs standing on them which the resolver could actually reach.

    3,481,572 n2 key groups walked
       55,312 over the stoplist cap, holding 5,204,927 carrier slots
    3,553,602 slots probed (very large groups are sampled)
       15,809 DISTINCT orgs anchor to exactly one scheme
        6,227 HARD — the design's exemption, both paths already agree
        9,582 SOFT — the gap (10,109 (org, generic-name) slots)

    FR:siren 5,962 · PL:nip 2,871 · DK:cvr 416 · BE:kbo 333 · SI:davcna 0

29 seconds. **9,582 is distinct rows reachable, not binds observed** — the
issue is explicit that nobody has shown a wrong bind from this class, and this
does not change that. The first run said 10,109 because it counted an org once
per generic key it stood on; that is fixed and both numbers are now reported,
because "how many rows" and "how many names can reach them" are different
facts.

## The specimens settle it

    org 23095417  DK:cvr    carriers=37   name '0'
    org 22917668  DK:cvr    carriers=55   '1. Vergabekammer des Freistaates Sachsen…'
    org  1844 / 5127  PL:nip  carriers=34  '2. Regionalna Baza Logistyczna'  (both)
    org 9218962 / 10868620  FR:siren  carriers=64  '2c Courtage'  (both)

A row whose entire corroborating name is `"0"`, shared with 36 others, is
reachable at ingest by any country-less mention carrying a CVR-shaped number.
That is precisely the worthless agreement the wall was built to refuse —
"agreement between two names nobody chose to make", in the wall's own comment
— and the batch arm does refuse it today.

## Recommendation: close it

The three reasons the issue asked to weigh, answered:

1. **Frequency/cost.** The genericness probe is one indexed COUNT with
   `LIMIT cap+1` (`name_key_is_generic`), and it fires only on the anchor
   path — which is itself the rare fall-through after E0 exact equality AND
   the Stage-2 canonical hit have both missed. The cost question the issue
   raised resolves as: cheap probe, rare path.
2. **The lenient default stays lenient**, exactly as the issue specifies. An
   unseen key ⇒ not generic ⇒ today's behaviour. `org_match_keys` may be
   empty or mid-build during ingest and the resolver has no "refuse" to
   return; lenient-on-unknown is the only safe failure mode on this path, and
   it is also what the wall's own default already does.
3. **The design says the stricter path is correct.** Two implementations of
   one rule disagreeing is the defect regardless of the count; the count only
   decides urgency, and 9,582 rows with `"0"` among them is not "later".

## Step 2 (open)

Thread `hard_scheme` + `stoplist_cap` into `ResolverArgs` the way
`match_org_null_country_r3` takes them (injected fns; linguistics stay out of
store), and gate the anchor bind's corroboration on the wall. It lands on the
INGEST HOT PATH, so it wants its own panel round and a fold canary — the
issue-316 discipline — not a fast edit on top of this measurement.


## STEP 2 BUILT AND PANELLED (2026-08-31, `6c7f950` then `b1f4395`)

The wall is threaded into `MentionResolver` and gates the anchor bind. The
panel over it returned **46 findings, 23 confirmed, 7 high**, and the first one
refuted a sentence in the implementing commit:

> "one seek on org_match_keys_kk — the index that covers it, checked against
> issue 323's lesson rather than assumed."

**False in exactly the state the design calls load-bearing.**
`org_match_keys_kk` is created by `finish_org_match_keys` at the END of a build
and dropped at its start, so through every window of a ~2-hour wet build — and
DURABLY after an interrupted one, since nothing re-enqueues a cancelled
build — the table has no index and the probe full-scans a ~21M-row satellite
per bind, on the ingest hot path. Measured 3.6 µs/row. The LENIENT verdict is
the expensive case: concluding "0 carriers" means reading everything.

The R3 arm already refuses in that state with a comment saying so. **The commit
message contradicted a comment already committed in this repo**, which is a
sharper way of saying the claim was never checked.

Fixed by resolving the wall's AVAILABILITY once per run and disabling it for
the run when the keyspace cannot answer cheaply — lenient, which is what the
issue mandates there, now at no cost.

### The other highs

- **A denial poisoned the E0 cache** (reproduced). The mint that follows a
  denial claimed the raw identifier triple, so the next mention carrying it
  rode E0 onto the fresh provisional even when its OWN name was specific
  enough to anchor to the standing owner. Which row a mention landed on
  depended on batch ORDER.
- **A probe error aborted the fold.** "Unknown" had two contradictory answers:
  an absent key bound leniently; an unreachable keyspace failed ingestion.
- **The probe took a pooled reader from inside the fold's write transaction** —
  the lock inversion the two pools exist to prevent.

### The counting was worth less than it looked

- Only denials were counted, so 0 denials could not be told apart from 0
  QUESTIONS — **the frequency this issue asked for in step 3, which the
  implementation answered by assertion.** Now (asked, denied, errored).
- Their only surface was `eprintln!` from the projection's isolated worker
  runtime, whose stderr does not reach journald (issues 61/63 — the reason
  `log_diag` exists). Now `log_diag` plus the durable job row.

## The canary, and what it does not say (2026-08-31, rev 1334aba, jobs 519/520)

Deployed, then a controlled six-notice refold before the day's scheduled fold:

    refold-notices: 6 notice(s) named: re-queued 6, stamped 6 tender(s)
    project:        6 notices → 6 tenders (0 islands), 31 versions;
                    6 tenders written, 0 verified unchanged

**No wall suffix**, which by the report's own construction means
`asked = denied = errored = 0`. No fold failure, no behaviour change.

**The wall was live for it**, verified positively rather than by the absence of
a log line: the gate needs `org_match_keys_kk` present and the build watermark
at 0, and `finish_org_match_keys` creates the index and zeroes the watermark in
the same function (canonical.rs:3654 and :3663). Sunday's build reported
"13034812 rows stand indexed" with outcome ok, which is that function's own
message. So both inputs held.

**What the canary does NOT say:** anything about frequency. Six notices were
never going to fire a path that needs a country-less identifier with exactly
one soft-scheme checksum anchor, after E0 exact equality AND the Stage-2
canonical hit have both missed. The reading that answers issue 318's step 3 is
the next full daily fold's job row, which now carries
`asked`/`denied`/`errored`. Read it before calling this closed.


## THE READING (2026-08-31, rev 7ae5f6a)

**The wall is armed on prod**, confirmed from the running system rather than
inferred, in the diag log:

    [issue 318] genericness wall ARMED for this run
                (org_match_keys_kk present, build watermark 0)

That line also retroactively validates the earlier silence: `log_diag` from
this code path demonstrably works, so an absent DISABLED line is evidence
rather than an untested channel.

**The frequency answer issue 318 asked for in step 3, at last:**

    daily fold  3,889 notices → anchor_reached 0
    canaries    6 + 8 + 3 notices → anchor_reached 0
    errored     0 throughout

**The soft-scheme anchor path does not fire in a normal day of ingest.** It
needs a country-less identifier whose digits pass exactly one register
checksum, surviving E0 exact equality AND the Stage-2 canonical hit, corroborated
on an exact cross-language N2 name, past the consortium and legal-form vetoes —
and then a soft scheme. A full day's TED package produced not one.

So the 9,582 standing rows the census found are **historical accumulation, not
daily inflow.** That reframes the fix honestly: it is a correctness repair —
two implementations of one rule no longer disagree, and the design says the
stricter is right — but it prevents approximately nothing per day going
forward. The value was always in closing the disagreement, and the cost is now
measured at zero rather than asserted to be small.

## What it took to get an honest zero

Three rounds, because a zero kept meaning two things:

1. `denied` alone: "nothing was generic" vs "nothing was asked". Panel caught it.
2. `asked` alone: "the path never fired" vs "the wall was switched off". The
   first real fold read `asked 0` over 3,889 notices and could not say which.
3. Silence itself: an armed wall with nothing to do vs a disabled wall on a
   quiet day both reported nothing at all.

`WallCounts { enabled, anchor_reached, asked, denied, errored }` plus an
availability line logged in BOTH directions is what finally makes each zero
say only one thing. Worth remembering the next time a prevention gets a
counter: the first number you reach for is usually the ambiguous one.
