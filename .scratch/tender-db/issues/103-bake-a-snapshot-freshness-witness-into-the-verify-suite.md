# 103 — bake a snapshot-freshness witness into de1x_verify.sh (a verify must prove its input is the input it thinks)

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
