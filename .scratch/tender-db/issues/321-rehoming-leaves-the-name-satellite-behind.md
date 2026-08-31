# 321 — A re-homed mention leaves its name variant on the origin org

Status: MEASURED 2026-08-31 (prod job 511) — the measurement says MACHINERY,
and names the safe subset. The repair itself is the open half
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


## MEASURED (2026-08-31, prod job 511, deploy 2fc2ce1)

`satellite-orphans` compares, per re-homing ORIGIN, the keys it still has
evidence for — its own head name plus every mention still sitting on it —
against every variant in `organization_names`:

    80 re-homing origins, 80 still standing, holding 85 name variants;
    72 are supported by NO remaining mention
    (66 of those already stand on a row this origin re-homed to),
    over 68 origins

**85% of the variants are orphans, and 92% of the orphans are the
destination's own name.** That settles the question the issue posed: this is
machinery, not a line in the review schema. 72 cases is too many to hand-judge
one at a time, and — more to the point — the evidence for 66 of them is
structural rather than a judgement about identity:

- org 9610149 `Bietergemeinschaft Dobler / Oberall` keeps
  `Dobler GmbH & Co.KG Bauunternehmung`, whose key stands on org 1711879,
  the row its 14 mentions moved to;
- org 15053079 keeps `Held & Francke Baugesellschaft m.b.H.` → org 3517077;
- org 1572, 14534470, 16232653 the same shape.

Each is a key the next `build-org-match-keys` re-creates on a row holding not
one mention that publishes it, and each one generates an E3 edge between
exactly the pair a reviewer just spent a verdict separating.

Language spread: DEU 68, FRA 3, ITA 1 — this is overwhelmingly the German
consortium shape the whole 311/317 line has been about.

## The safe subset the measurement names

**Drop a variant from the origin when all three hold:**

1. no mention still on that org normalizes to its key (it is unsupported —
   the origin has no evidence left for the name), AND
2. the org's own head name does not normalize to its key (never drop the row's
   own identity), AND
3. the key already stands on a row this origin re-homed a mention TO, on that
   row's head or its own satellites (the name is demonstrably the other
   company's, and the destination already carries it, so nothing is lost).

That is **66 of 72**. Condition 3 is what makes it safe: the variant is not
being deleted from the corpus, it is being removed from a row that has no
claim on it while the row that does keeps it.

**The remaining 6 stay.** They are unsupported here but no destination carries
them — `AMBERG ENGINEERING` (FRA and ITA on org 22197923), `a+r Architekten`,
`Ing.-Büro Grote GmbH`, `ASZ-Linz GmbH`, `Gebrüder Haider & Co, Hoch- u.
Tiefbau GmbH`. Dropping those WOULD lose a spelling the corpus has nowhere
else, which is the invention this line of work keeps refusing. They are the
`missing_target` residue by another name, and they belong with issue 317's 64.

## What the repair still needs

- A **pre-image**, like every other apply job in this campaign: dropping a row
  from `organization_names` must be undoable (issue 312 is what happens when
  an apply outruns its evidence).
- **Dry-first with a recorded plan and T4 parity**, compared as tuples
  `(org, lang, key)` — a count cannot show that a variant's destination
  changed between the plan and the run.
- A **rebuild of `org_match_keys` afterwards**, or the orphan keys stand until
  the next weekly build anyway. The measurement is of the SATELLITE; the harm
  is in the derived key table.
- Its own adversarial panel round. It writes an entity table.
