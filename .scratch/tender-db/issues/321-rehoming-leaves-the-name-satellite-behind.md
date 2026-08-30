# 321 — A re-homed mention leaves its name variant on the origin org

Status: ready-for-agent (found by the issue-317 Unit A panel, 2026-08-30)
Kind: correctness (organization layer)
Relates to: 317 (Unit A built the move), 300 Stage 4 (name keys), ADR-0013 D4

## What the move does not move

`apply_rehoming` corrects `organization_mentions` and lets the fold re-derive
everything downstream. `organization_names` — the per-org language-variant
satellite the resolver writes on mention capture — is NOT part of that
derivation in the same way, so after re-homing a mention from a consortium
vehicle to its member:

- the VEHICLE keeps the member's name as a satellite variant;
- the MEMBER may not gain it.

## Why it is not cosmetic

Satellite names feed the Stage-4 name-key build (`org_match_keys` reads head
+ satellites), so the vehicle keeps a key that matches the member's name and
the pair goes on generating E3 candidate edges — the exact edges a reviewer
just spent a verdict resolving. It can also carry cross-language
corroboration into R3, which is a merge path.

## Why it was left alone rather than fixed in the move

The vehicle **was published under that name** — that is why the variant
exists. Deciding whether a consortium vehicle should keep a member's name is
a judgement about identity, and the re-homing job is a mechanical move that
must not make one. Two candidate answers, both needing a measurement first:

1. **Move the variant with the mention**, if that variant came only from the
   re-homed mention and no other mention of the vehicle published it. That
   is checkable: count the vehicle's remaining mentions whose name
   normalizes to the variant.
2. **Leave it and let the review say so**, adding a verdict action that
   explicitly drops a satellite name from the origin.

## First step

Measure: after the first re-homing wet run, count how many re-homed cases
leave a satellite name on the origin that no remaining mention supports.
That number decides whether this needs machinery or a line in the review
schema. The `fusion-census` job already reads the same rows and is the
natural place to add the counter.
