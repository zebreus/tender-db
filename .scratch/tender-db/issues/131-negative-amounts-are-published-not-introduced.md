# 131 — the `cents < 0` rows: the fold cannot introduce them, and the parser makes them on purpose

Status: ANALYSIS — the parse-vs-fold half of task #33 (sdk-vendor owns the distribution half). Code-path
determination, no prod contact. **Carries four predictions their snapshot data can falsify, including one
that would make this my bug.**
Kind: data-quality / verification
Relates to: 33 (the triage), 28 (the gate that caught it), `run_light` 3.7 (the invariant asserted)
Owner: proj-fix

## The question

The standing gate caught **17,738 rows with `cents < 0`** in `tender_version_amounts` — an invariant
agreed as a hard fail ten days ago that went unenforced because its carrier (four pinned totals) rotted.
Three possible sources: the parser wrote a negative, the fold introduced one, or **the source published
one and both layers are right** (sdk-vendor's third arm, and the one worth keeping open).

## 1. The fold cannot introduce a negative

The path from parsed value to stored row is a **pure copy**, with no arithmetic and no sign
transformation at any step:

- `project.rs:1551-1552` — `NoticeValue::Amount { cents, currency }` → `Fact::Amount { cents: *cents, … }`
- `canonical.rs:2800-2801` — `Fact::Amount { cents, … }` → `Value::Integer(*cents)`
- `canonical.rs:3422` — that value inserted into `tender_version_amounts(…, cents, …)`

The **only** arithmetic on cents anywhere in the projection is `single_currency_total`
(`project.rs:2412-2427`), which sums *bid* values. It writes `tender_version_lot_results.awarded_cents`
and the bid totals — **different columns**, not `tender_version_amounts.cents`. A sum of non-negatives
cannot be negative in any case.

So: if a row in that table is negative, the value arrived negative.

## 2. The parser produces negatives deliberately

`eforms/value.rs:81-103` (`value::cents`) strips a leading `-`, keeps the sign, and applies it:

```rust
let (sign, digits) = match text.strip_prefix('-') { Some(rest) => (-1i64, rest), … };
…
whole.checked_mul(100).and_then(|c| c.checked_add(fraction)).map(|c| sign * c)
```

`checked_mul`/`checked_add` guard overflow, so a negative is **not** a wraparound artifact — it is a
faithful reading of a published `-` value.

## 3. Which eras can produce one

Both XML eras share that one parser, so this is not era-specific:

- eForms — `eforms/value.rs`, `Decision::Amounts`
- legacy TED r208/r209 — `r209/parse.rs` `Rule::Amount`, and the early-R2.0.8 `FMTVAL` arm, both calling
  `value::cents`
- internal-ojs (2008) — rides the r209 walker, same path

**The text era emits no amounts at all.** `text/parse.rs` produces only `Classification`, `Code`, `Date`,
`Id`, `Integer`, `Text` — no `Amount` variant — so text-profile notices contribute **zero** rows to
`tender_version_amounts`.

> My first instinct was "probably eForms-only, so a single-profile cluster points at parsing". That was
> wrong and I checked it before sending it: r209 routes through the identical parser. Recorded because
> the wrong version is the intuitive one and someone will have it again.

## Conclusion

**The third arm is the live one**: the source published negative amounts, and both layers recorded them
faithfully. The thing that is wrong is the **invariant** — `run_light` 3.7 asserting `cents >= 0` — not
the data. Negative published amounts are plausible on their face (corrections, credit adjustments,
withdrawn or reduced awards).

That is a conclusion about code paths. It is **not** yet a conclusion about these 17,738 rows, which is
what the predictions below are for.

## Predictions — what sdk-vendor's distribution should show, and what would refute me

- **P1.** No `text`-profile rows among the negatives — indeed none in the table at all. A text-profile row
  here would mean amounts reach that table by a path I have not found.
- **P2.** Negatives spread across XML-era profiles rather than clustered in one. A **single-profile
  cluster** would point instead at an era-specific mapping fault — e.g. a field whose published semantic
  is a *delta* being mapped to an absolute amount — which is a real defect and not "the source said so".
- **P3.** Magnitudes look like money, and mirror siblings (equal-and-opposite on the same `tender,seq`)
  would indicate correction pairs — supporting the published-negative reading.
- **P4 — the decisive one, and the one that makes this my bug.** For any negative row, the corresponding
  **parse-layer** value (`notice_amounts` for that notice + field) must be **equally negative**. If a
  parsed value is non-negative while its folded row is negative, then the fold *did* transform it, my
  analysis above is wrong, and the defect is mine. This needs the snapshot, so it is sdk-vendor's query
  to run, not mine.

  > **P4 must compare against the version CHAIN, not the version's own causing notice.** The fold carries
  > facts forward: each version's stored set is `carried ∪ published` (`project.rs:1887`, `supersede()` at
  > `:1931`), so an amount first published at seq 1 is re-written at every later seq, whose causing
  > notices contain no such value. Comparing a row against its own seq's notice therefore flags every
  > carried-forward negative as fold-introduced — a false positive, in the direction that blames the fold,
  > on any multi-version tender. Compare instead against every notice in the chain with `seq <= a.seq`.
  > Caught in sdk-vendor's queries 9–11 before their results were read (2026-08-04); their single-notice
  > fixture could not have surfaced it, since carry-forward needs ≥2 versions.
  >
  > And even corrected, **P4 returning 0 does not prove the fold correct** — only that no negative
  > appeared on a chain carrying none. A sign flip on a tender whose chain legitimately holds another
  > negative stays invisible.

## If the predictions hold

The fix is to the invariant, not the data: `run_light` 3.7 should assert what is actually true of
published procurement amounts. Worth deciding deliberately rather than by relaxation — "negative amounts
exist" and "negative amounts are fine anywhere" are different claims, and the second is probably false
(a negative *estimated total value* is likelier a defect than a negative correction line).
