# 334 — A platform vendor's name sits on 62,080 org rows that hold other bodies' identifiers

Status: MEASURED AND CLOSED — SOURCE-SIDE 2026-09-01 (job 574, `27386b5`, 8 s).
**The notices said it.** 120 of 120 sampled `avenue web systèmes` carriers agree
with their own mentions; 0 differ. Nothing downstream replaced anything, so there
is no parser fix and no repair we can make from our own data.
Kind: data quality (organization layer)
Relates to: 332 (where it surfaced), 300 Stage 4 (`org_match_keys` is built from
these names), 330 (the other "wrong thing in the name field" class)
Blocked by: nothing

## The finding

Issue 332's census lists over-cap name keys widest-first. The single widest name
key in the corpus is not a government body or a large company:

```
carriers=62084  with_identifier=62080  distinct_identities=62080
  avenue web systèmes
```

**62,080 organization rows carry the name "Avenue Web Systèmes", and they hold
62,080 DIFFERENT identifiers.** Avenue Web Systèmes is a French e-procurement
platform vendor. So the name field appears to have received the *platform's*
name while the identifier field received the *actual buyer's* — 62,080 distinct
buyers wearing one vendor's name.

For contrast, the same listing shows what a real shared name looks like
(`tribunal administratif de lyon`, 16,892 carriers / 5,995 identities) and what a
placeholder looks like (`tendsign` 16,600, `sans suite` 2,805, `infructueux`
6,098 — all with zero identifiers, which is the honest shape for a placeholder).
`avenue web systèmes` is neither: it is a real vendor name attached to rows that
carry real, distinct, other-party identifiers.

## Why it matters

The name is load-bearing. `n2_key` / `n3_key` are computed from it, so all 62,080
rows share one key — by far the largest carrier count in the corpus. That means:

* the genericness wall correctly refuses it (62,080 > 20), so no merge arm will
  act on the name — the immediate false-merge risk is nil;
* but every one of those 62,080 organizations is **named wrong** in the canonical
  layer, and anything a reader or API consumer does with `organizations.name` for
  them is wrong too.

So this is a correctness problem in the projected data rather than a merge-safety
problem.

## What is NOT known — do not "fix" anything yet

1. **Is the identifier the buyer's, or is the whole row the platform's?** If
   62,080 rows genuinely are the platform under 62,080 buyer identifiers, the
   name is right and the identifier is wrong — the opposite repair. The count
   alone cannot tell.
2. **Where does it enter?** A parse-side mount picking the platform block instead
   of the buyer block would be a real parser defect; a publisher putting its own
   platform name in the buyer element would not. Issue 330's lesson applies
   directly: compare against `organization_mentions.name`, which holds what each
   notice said, and the answer is decisive rather than sampled.
3. **Are there other vendors in the same shape?** `tendsign` (16,600 carriers,
   zero identifiers) is another platform name in the name field, behaving
   differently. A cut of over-cap keys by "many carriers, nearly as many distinct
   identifiers" would name the whole class rather than these two.

## Suggested first step

Reuse the issue-330 machinery, which already answers question 2 corpus-wide:
compare each affected org's stored name against its own mentions' names. If the
mentions say "Avenue Web Systèmes" too, it is published and no parser change
would help; if the mentions name the real buyer, something downstream replaced
it, and that is ours.

Only then decide, and per issue 328's precedent the published mention string
stays whatever happens.

## The probe (job 574, `27386b5`, 8 seconds)

Walked 3,481,572 name keys to find the **60 widest** over the cap, sampled up to
120 carriers of each, and asked each carrier's own `organization_mentions` what
the notice called it. A probe over those 60 keys — not a corpus tally.

**7,200 sampled: 7,197 agree with at least one of their own mentions, 0 differ, 3
have no mentions. All 60 keys are `published`.**

The specimen that opened the issue, first row of the listing:

```
carriers=62084  sampled=120  agrees=120  differs=0  silent=0  published
  avenue web systèmes
```

## The answer: source-side, and unfixable from here

The name is not a parse defect and not a downstream substitution — **the notices
themselves put "Avenue Web Systèmes" in the organization element.** Every one of
the 120 sampled rows says so. Same conclusion as issue 330 reached for embedded
addresses, by the same method, which is some corroboration that the method is
sound rather than that the answer was foregone.

That closes the actionable part. It also closes the *repair* question, and for a
sharper reason than "publisher's fault": a repair would need a better name to
write, and the mentions are where a better name would come from. They agree with
what is already stored. **There is nothing in our own data to promote.**

Question 1 from the filing — "is the identifier the buyer's, or is the whole row
the platform's?" — is therefore the wrong question to chase here. Whichever it is,
the notice published a vendor name against that identifier, and we have no
independent evidence to overturn either field.

## What this does NOT mean

The projected data really is odd: 62,080 organizations named after one
e-procurement vendor. Anything an API consumer does with
`organizations.name` for those rows inherits that. Worth knowing, worth
documenting in the caveats, **not** worth a repair — and the genericness wall
already prevents the one dangerous consequence, since 62,080 carriers is far over
the cap so no merge arm will ever act on that name.

`tendsign` (16,600 carriers) is in the same list with the same verdict, so this is
a class rather than one vendor: platform names published as organization names.
Recording it as a known source-side characteristic rather than a defect to chase.

## The wider reading, which is the useful output

All 60 of the widest name keys in the corpus are published as stored. So the
widest-key population is not contaminated by anything we did — it is what the
publishers sent. Combined with issue 332's finding (99.5% of decidable over-cap
keys are genuinely shared names), the picture of the wide tail is now settled:
**it is real, published, and mostly legitimately shared.** No further probing of
this population is warranted without new evidence.
