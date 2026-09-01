# 334 — A platform vendor's name sits on 62,080 org rows that hold other bodies' identifiers

Status: NEEDS-TRIAGE 2026-09-01 — surfaced by issue 332's census (job 571), NOT
yet sized or explained.
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
