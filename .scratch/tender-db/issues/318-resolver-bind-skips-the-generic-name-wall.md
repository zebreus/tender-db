# 318 — The resolver's anchor bind applies the R3 bar without the R3 wall

Status: ready-for-agent (found by the issue-316 adversarial panel, 2026-08-30)
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
