# 107 — bake a snapshot-freshness witness into de1x_verify.sh (a verify must prove its input is the input it thinks)

Status: open — POST-LANDING follow-up (team-lead, 2026-08-02). Deliberately kept manual during the ship.
Kind: verification tooling / false-signal prevention
Blocked by: — (waits only on `issue98-de1x-org-refs` being unfrozen after the ship)
Relates to: 98, 99, 102 (the same class: an artifact that misstates what it describes)

## The failure this prevents

The suite reads a snapshot path handed to it and reports on whatever it finds. It never checks that the
snapshot is the one the operator means. During the 98+99 ship there were **two near-identical ~450 GB
snapshots** on the box — `tender-db-1785661162.db` (pre-refold) and the post-refold one — with names
differing only in a unix timestamp.

Pointing the suite at the stale one produces:

```
H1/H2/H3 = 2185          (the shells "not fixed")
with_OWN_party = 0       (the org class "still dead")
EXPECT_NO_REGROUPING     passes — counts genuinely unchanged
```

That is a **perfect imitation of "the re-fold did nothing"**, on a re-fold that did everything. Every
number is individually correct; the conclusion drawn from them is the exact opposite of the truth. It
would arrive at hour five of a 4h30m run, when the operator is tired and a NO-GO is the expensive
answer, and it would send someone hunting a defect in issue 99 that does not exist.

## The witness

A value only the new binary could have written proves the file contains the run being verified — the
same trick that validated the canary read when `mode=ro` was locked and a blind `immutable=1` would have
reported a stale `0`:

```sql
SELECT COUNT(*)                    AS de_versions,
       SUM(t.projection_epoch = 1) AS epoch_stamped
  FROM notices n
  JOIN tender_versions v ON v.caused_by_notice_id = n.id
  JOIN tenders t         ON t.id = v.tender_id
 WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2');
```

`epoch_stamped ≈ de_versions` → the snapshot contains the re-fold. `epoch_stamped = 0` → wrong or
pre-fold file: **stop, do not run the suite, do not interpret its output.**

## What to build

Run it automatically as the suite's first act, and make a stale/wrong snapshot **exit non-zero
immediately** with an unmistakable message — not a FAIL line among forty others, which is exactly where
it would be misread as a result rather than as "you gave me the wrong file".

Generalise past `projection_epoch`, which only witnesses an epoch-bumping fold. The suite should take an
expected witness for the run it is verifying — a `--expect-witness <sql>=<min>` parameter, or simply
recording the snapshot's mtime/size and requiring it to be newer than the baseline's source. The
principle, not the column, is the point: **a verification must prove its input is the input it thinks it
is, before it reports anything about it.**

## Why it stayed manual for the ship

`issue98-de1x-org-refs` is frozen at `aab6065` and the box copy's checksum is maintained equal to it —
parity we have been actively checking. Breaking that mid-flight to add a check runnable as one command
was the worse trade. It ran as a mandatory manual step 0 instead.

## Note

Third issue in this batch about an artifact that misstates itself: 102 (a baseline naming the wrong
source), the ledger claiming a fold that had not run, and now a verify that cannot tell which snapshot it
read. The recurring lesson is that **the output of a check is only as trustworthy as the check's
knowledge of its own inputs** — and inputs are exactly what nobody re-examines when the numbers look
plausible.

---

## 2026-08-04 — 119 folded in here, and the general form is now built (sdk-vendor)

**Issue 119 belongs under this one.** It was filed as "the snapshot has no producer";
that premise is false (the producer is the daily supervisor `Spec::Snapshot`, live since
`83edbea`, five days before 119 was filed). What survives is 119's *other* half, which is
this issue's sentence exactly: **nothing asserted that the input was the input the check
thought it was.** 119 is rewritten to the silent-staleness gap and folded here; see it for
the cadence half, which stays open.

**The general form asked for above — "a `--expect-witness` parameter, or simply recording
the snapshot's mtime/size and requiring it to be newer than the baseline's source" — is
built.** `canonical-verify/standing_gate.sh` (task #28) states its input's path, size,
mtime and age before reporting anything, and **refuses** past `MAX_AGE_H` (default 30h,
one missed daily): `exit 2`, `VERDICT stale_input`, no check results printed. That is this
issue's requirement — *"exit non-zero immediately with an unmistakable message, not a FAIL
line among forty others"* — and it is exercised in isolation (a 40h input is refused, not
verified).

**One variant this issue did not anticipate, found by proj-fix.** Recording mtime/age
bounds how stale an input may be but **cannot establish which run produced it**. If the
daily's snapshot step fails, "the newest snapshot" is yesterday's file — still inside any
age bound — so the gate would verify it green and report success for a cycle that produced
nothing. Age is a bound, not an identity. The gate therefore accepts a **pinned**
`SNAPSHOT=<path>` from the pipeline and carries the resolution mode (`pinned` vs `newest`)
into its verdict line, so a green from a guessed path can never be read as a green from a
pinned one. The `projection_epoch` witness this issue proposed is the strongest form of the
same idea — a value only the intended run could have written; pinning is the cheap form
available to a check with no epoch to key on.

**Still not generalised to `de1x_verify.sh`.** The witness above is implemented in the
standing gate only. Carrying it into the de1x suite — the run that motivated this issue —
remains open work.
