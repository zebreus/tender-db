# 110 — section I verifies the wrong artifact: it greps SSR HTML for a client-hydrated ledger

Status: open — POST-LANDING follow-up. Found during the 98+99 ship, 2026-08-02.
Kind: verification tooling / false-negative gate
Blocked by: — (waits on `issue98-de1x-org-refs` being unfrozen)
Relates to: 107 (snapshot-freshness witness), 102 (baseline naming the wrong source), 98, 100

## Defect

Section I of `de1x_verify.sh` asserts the honesty requirement — that the deployed ledger discloses the
DE-1.x award-winner gap. It implements that by fetching `GET /` and grepping the returned HTML for
`AWARD WINNERS ARE NOT RESOLVED`, `issue 100`, and the 241 sentence.

**The dashboard renders the Resolved-categories table client-side.** No ledger entry appears in the
server-rendered HTML — not this one, not the r208 entry, not the "Resolved categories" heading itself.
The initial hydration payload (`dioxus_hydration_data`, ~6.5 KB decoded) does not carry it either; the
client fetches it afterwards from `/api/dashboard`.

So the gate returns MISSING for a correct deployment. It hard-fails, and its failure is
indistinguishable from the ledger genuinely being absent.

## Why it survived review

It was tested against a stub HTTP server written for the test — which returned the string, because the
same author wrote both sides. The test proved the grep worked, never that the real application serves
that string in that place. **A check validated against a mock of its own assumptions is a mirror, not a
test.**

## Fix

Verify the artifact that carries the claim to the user, not a proxy:

1. **`/api/dashboard`** (the client-fetched payload) — assert the disclosure is in the served JSON.
   This is the real answer: it is what the browser renders from.
2. Optionally also assert it in the **deployed binary** (`grep` the nix-store `server` binary, since the
   ledger is `include_str!`-compiled) as a cheap corroborating check — necessary but not sufficient,
   since a string present in the binary is not proof it reaches a user.
3. **Handle the coverage-disabled case explicitly.** `/api/dashboard` returns all-null fields when the
   coverage refresher is disabled (`TENDER_DISABLE_COVERAGE`) or has not completed a cycle. That is
   *neither* pass nor fail for the disclosure — it means the dashboard has no data at all, which is its
   own alarm. The gate must distinguish three states: disclosure served ✓ / disclosure absent from a
   populated payload ✗ / payload empty → **not a verdict, a different problem**.

## Note

Third instance in this batch of an artifact-vs-proxy confusion, and the first in a gate I wrote myself:
102 (a baseline file naming the wrong source), 107 (a verify blind to which snapshot it read), and now a
gate measuring server-rendered HTML for data that is never server-rendered.

The reasoning was right — *"the ledger is `include_str!`-compiled, so only the deployed binary counts"* —
and the implementation then checked something else entirely. Being right about what matters does not
guarantee measuring it.

## Confirming run — 2026-08-03, live service (sdk-vendor)

Section I had never been run against a service with a populated `.quarantine`; the
prior attempt could only report the null state. Run against
`https://tenders.zebreus.click`:

```
I1 PASS  served Resolved-categories carries the 'eForms-DE 1.x (German dialect)' row
I2 PASS  the gap is stated in caps, in its own sentence
I3 PASS  the gap names its tracking issue (100)
I4 PASS  the 241 residual is stated, not left unexplained on a Resolved row
I5 PASS  the residual's stale reason names its issue (87)
I6 PASS  served row reconciles with F1a/F1b: reclaimed=218635 outstanding=241
I7 EYE   resolved 2026-08-02 · fix: issues 75/78/76/85/98/99 — award winners pending, issue 100
```

I6 reconciled with **no refresher lag** — the served counts and the store agree
exactly, so the 60 s window that would have made a real panel/store split look like
lag was not in play. The preflight independently reported the serving build as
`1830d50c74a26e32e70c1c72ab377b323a93e4d9`.

The disclosure claim of 98/100 is therefore verified against the artifact a browser
actually receives, which is the whole point of this issue.

### `TDB_ONLY=I` — why the run needed changing before it could happen

Section I is two GETs against a public endpoint: no token, no SQL, no data pages.
It was gated behind a box window only because it shares a file with sections A-H,
which are corpus-wide aggregates over 6.9M-row tables. That coupling means the free
honesty check goes unverified for exactly as long as the box is busy — which is when
a stale disclosure is most likely and least likely to be noticed. `TDB_ONLY=I` runs
section I alone, needing neither `TDB_TOKEN` nor `TDB_SNAPSHOT`.

It announces what it skipped in three places (banner, summary, pass message). A quiet
section selector would reintroduce this issue's own failure mode — silence about a
check being indistinguishable from a check that passed — at the level of the whole file.

### The bug the selector exposed: a verdict-less run exited 0

With A-H skipped there is nothing else in the run, so a null `.quarantine` printed
"0 passed, 0 failed" and exited **0**: green to anything reading a status code, from
a run that verified nothing. The EYE line said so in prose, but no consumer reads
prose. Exit codes are now distinct — `0` discloses, `1` fails to disclose, `2` could
not tell.

Falsified against a stub in four states before being pointed at anything real:

| stub serves | result |
|---|---|
| the approved wording | I1-I6 PASS, exit 0 |
| the row, but wording silent about the gap | I2-I5 FAIL, exit 1 |
| no DE-1.x row at all | I1 FAIL, exit 1 |
| `.quarantine: null` | no verdict, exit 2 |

The stub also returned 500 and logged any `POST`. **Zero POSTs were issued** under
`TDB_ONLY=I`, so "this mode touches no data" is measured rather than asserted.

Landed `089e715`.


## The convergence conflict is THIS ISSUE, arriving from the other branch

Preparing the post-deploy `issue62` → `issue115` merge, the one content conflict is
`de1x_verify.sh`, and it is not a coincidence: **both branches independently wrote a
section I for the same obligation.**

```
issue62  (mine, this issue)      adds I0 I1 I6 I7 I8
issue115 (issue 98/100 work)     adds I0 I1  + C12 H3 H7
```

I0 and I1 collide by name. More importantly they disagree about **which artifact
carries the disclosure** — which is the whole subject of this issue.

`issue115`'s section I does:

```sh
page=$(curl -sS "$BASE_URL/")
printf '%s' "$page" | grep -qi "eForms-DE 1" || miss="$miss no-DE-1.x-entry"
printf '%s' "$page" | grep -qi "winner"     || miss="$miss no-mention-of-winners"
printf '%s' "$page" | grep -qi "issue 100"  || miss="$miss no-pointer-to-issue-100"
```

**That is the predecessor gate this issue was filed to replace** — it greps the
server-rendered HTML for a disclosure that is not in the server-rendered HTML.

Verified on `issue115`'s OWN tip (`5c197e7`), not assumed from this branch, because
"the disclosure is client-side" is a claim about a build and the axis here is which
branch:

* `resolved_categories` appears in `crates/app/src/coverage.rs` and
  `crates/app/src/ui.rs` — `ui.rs` being the Dioxus client component;
* the `AWARD WINNERS ARE NOT RESOLVED` text lives in
  `crates/app/data/quarantine-ledger.json`, `include_str!`-compiled;
* the client fetches it from `#[get("/api/dashboard")]` in `api.rs`.

So on `issue115` too, `GET /` does not contain the disclosure, and that gate reports
MISSING for a correct deployment. Its false alarm is indistinguishable from the
ledger genuinely being absent — this issue's opening sentence.

### Resolution (to apply post-deploy, per team-lead's sequencing)

Not symmetric, and not a preference for my own work:

* **Section I — take `issue62`'s.** It asserts the served JSON at the exact path the
  client reads, which is the artifact the user receives. It also asserts strictly
  more: the caps sentence, issue 100, the 241 residual, issue 87, and a
  reconciliation against F1a/F1b. `issue115`'s three greps are a subset of its claims,
  against the wrong artifact.
* **C12, H3, H7 — take `issue115`'s.** Different sections, no collision, additive.
* **Do NOT keep both section I's.** Two gates for one obligation, one of which cannot
  pass on a healthy deployment, is worse than either alone: the red is
  uninterpretable and the natural fix is to stop reading the section.

The one thing worth carrying across from `issue115`'s version is its *reason for
existing* — that a disclosure obligation deserves a standing check at all. That
intent is preserved; only the artifact it reads changes.

### Why this took a merge to surface

Both gates were written for the same obligation, weeks apart, on branches that never
met. Neither author could see the other's. The duplicate is not a process failure so
much as evidence for a single verification owner per obligation — which is what the
convergence establishes.

## The resolution instruction, restated so it cannot invert (2026-08-03)

proj-fix halted the #11 merge on this file and was right to. The instruction had been
recorded as **"take ours"** — written from `issue62`'s vantage. But in a merge,
`ours`/`theirs` are defined by **merge direction**, and the merge runs from the code line,
where `ours` is the *other* branch. Under that reading "take ours" deletes I2–I8.

**Never use `ours`/`theirs` in a cross-branch instruction.** The referent flips with who
runs the command. By name, and this is the resolution:

> **Take `issue62-defer-org-indexes`'s section I. Discard the code line's.**

Verified by branch name rather than pronoun:

| branch | section-I ids | greps HTML | asserts `/api/dashboard` |
|---|---|---|---|
| `issue62-defer-org-indexes` | **I0–I8** | 0 | 9 |
| `issue5-country-filter-restructure` | **I0, I1** | 1 | 0 |

I4–I8 (the 241 residual, issue 87, the F1a/F1b reconciliation, the resolved-date, the
binary corroboration) have **no counterpart at all** on the code line.

### And a correction to how the record was verified

My `comm` check that produced *"nothing of theirs lost"* matched `report … I<n>` — but
**I2–I5 are emitted through the `disc()` helper**, so it never saw them. I recorded
`issue62` as carrying I0/I1/I6/I7/I8 when it carries I0–I8. proj-fix's count was right and
mine was wrong.

The check was too narrow in exactly the way this suite keeps finding: it matched the
*shape* a check usually takes rather than the checks themselves. Had the pronoun pointed
the other way, my grep would not have caught the loss.

### What "resolve from the record" was supposed to mean

The instruction was meant to stop a merger adjudicating invariants they did not write. It
was read — reasonably, because I wrote it loosely — as *the record is authoritative*. It
is not. A record is an input, verifiable like any other, and this one was wrong in two
details while right in substance. The narrower correct form: **do not decide which
invariants are right; do verify the record's factual claims.** proj-fix did exactly that,
and it is why the seven checks still exist.

### Re-verified with a method that catches helper-emitted ids — `issue62` is a STRICT SUPERSET

The earlier `comm` matched only `report … <id>`, so it missed every id emitted through a
helper (`disc`, `hstep`, `eq`, `zero`, `subset`, …). Redone against all emitters:

```
issue62-defer-org-indexes         42 check ids
issue5-country-filter-restructure 35 check ids

in issue62 but NOT in the code line :  I2 I3 I4 I5 I6 I7 I8
in the code line but NOT in issue62 :  (none)
```

**Nothing exists on the code line that `issue62` lacks.** So the resolution is simpler than
a per-section split: **take `issue62-defer-org-indexes`'s `de1x_verify.sh` whole.** No
union, no section-by-section reconciliation — `e8af573` already folded in the code line's
C12/H3/H7 additions, which is why the superset holds.

### A correction to the post-mortem, because the wrong cause implies the wrong remedy

The incident was explained as: *the justification said "their HTML-grep section I
deliberately absent", but the current file greps no HTML, so the record described section I
before your own improvement landed and was never re-derived.*

**That is not what happened.** Measured by branch:

| branch | greps HTML | asserts `/api/dashboard` |
|---|---|---|
| `issue62-defer-org-indexes` | 0 | 9 |
| `issue5-country-filter-restructure` | **1** | 0 |

The HTML-grep gate is on **the code line**, exactly as the justification said. The record's
substance was right and did not go stale. The two things actually wrong were:

1. **an invertible pronoun** — "take ours", whose referent flips with merge direction; and
2. **a verification that matched the shape a check usually takes** rather than the checks
   themselves.

The distinction matters because the remedies differ. "The record went stale under a later
improvement" implies *re-derive records after changes*. The real causes imply *never write
`ours`/`theirs` across branches*, and *verify against the artifact, not against the pattern
you expect the artifact to follow*. Only the second pair would have prevented this.