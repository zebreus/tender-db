# 321 — A re-homed mention leaves its name variant on the origin org

Status: DONE 2026-08-31 — measured, built, panelled (9 confirmed), deployed,
and applied on prod. 66 dropped, orphans-at-target 72 → 0, undo verified live
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

## What the repair needed (all done)

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


## APPLIED ON PROD (2026-08-31, deploy fa574c8, jobs 513–516)

    DRY  (513): 66 variant(s) would be dropped
    WET  (514): 66 candidate(s) matched the recorded plan exactly;
                dropped 66, skipped 0 on the in-transaction re-check
    RE-MEASURE (515): 80 origins, 80 standing, holding 19 name variants;
                6 supported by NO remaining mention
                (0 of those already stand on a row this origin re-homed to)
    RESTORE DRY (516): 66 outstanding pre-image(s), 66 would be restored;
                0 occupied, 0 superseded, 0 whose org no longer exists

The second and third lines are the acceptance. `orphans_at_target` went 66 → 0
and the orphan count 72 → 6: exactly the class this was built for went, exactly
the 6 the measurement said must stay remain, and not one of them has a
destination carrying it. Variants across the origins fell 85 → 19.

The fourth line is the one that matters most, because it is the claim that
made the drop permissible in the first place: the undo holds all 66, every one
restorable, nothing occupied or stranded. It was run live rather than argued.

**Residual, and deliberately not chased:** 66 `org_match_keys` rows still
carry a dropped key. The key store is rebuilt from zero by
`build-org-match-keys` on the Sunday cadence, and the E3 edges those keys
would feed are only re-derived by `scan-org-match-keys` in the same run — so
nothing regenerates before the rebuild corrects it, and the honest move is to
let the scheduled job do its own work rather than hand-deleting rows another
job owns.

## The panel (9 confirmed, 3 high) — all in the UNDO

The adversarial round the plan called for found nothing wrong with the drop's
selection and three high findings in the half whose whole job is reversibility:
scan order deciding which of two pre-images came back; a merged-away org
aborting the entire pass on a foreign key, permanently, since a re-run met the
same row again; and the restore running with no transaction at all, so a crash
between the INSERT and its `restored_at` stamp left a row that every later pass
read as `occupied`. Plus six more, including a `stale_keys` count taken over a
prefix of the candidate list while the skips are interleaved through it. See
`fa574c8`.

## Near-miss worth keeping (2026-08-31)

`39a327a` was committed WITHOUT the `org_name_drops` INSERT — a concurrent
agent was mutation-testing this very function in the shared worktree to check
the tests fail when each guard is removed, and `git add crates/store/src/canonical.rs`
took the mutation. That build reached prod and stood for ~40 minutes: a
`drop-orphan-satellites --wet` run against it would have deleted 66 satellites
with no pre-image and no undo — the issue-312 shape exactly, and worse.

Nothing was lost. The wet arm was being held for the panel, so only the
read-only dry pass ever ran on that build, and the co-worker caught and
restored the 22 lines (`4ac4271`) before any of this reached a write.

The lesson is not "check the diff" — CLAUDE.md already says that, and I did
run `git diff` on the small files. It is that I ran it on the SMALL files and
grepped the big one, which is the file another agent was in. The rule earns its
keep exactly where it is least convenient: **read the whole diff of the
contended file, or take a separate worktree.** Two gate runs in the same hour
also died on a co-worker's transient `zz_probe_*.rs`, so the contention was
visible before it cost anything.
