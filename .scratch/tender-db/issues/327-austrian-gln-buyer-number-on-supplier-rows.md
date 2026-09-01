# 327 — The Austrian 9110 GLN: a buyer's number published in the supplier's block

Status: MEASURED AND WATCHED 2026-09-01 — source-side error, NOT a parse defect.
The actionable conclusion is a NEGATIVE one (no checksum arm for this class), and
the latent risk now has a tripwire live on prod (`f315e2e`, job 554).
Kind: data quality / identity semantics (organization layer)
Relates to: 311 (whose pilot flagged the class), 312 (the same "looks untidy vs
measured false-merge rate" question, same answer shape), 326 (the country
attribution this would have poisoned)

## Where it came from

Issue 311's pilot noted, as an aside: *"Austrian notices widely publish 13-digit
GS1 Austria GLNs (ERsB/USP-issued, 9110-prefix, mod-10-checkable) as
organization identifiers — a real register class idgate/crosswalk do not model.
Worth a census + possible idgate scheme (separate slice)."*

Nobody had run the census. This is it, and it says the opposite of what the
aside expected.

## The class, measured on prod

| | |
| --- | --- |
| 13-digit all-digit identifiers, `AT` | 6,730 — of which **6,712 are 9110-prefixed** |
| 9110 rows corpus-wide | 6,965 |
| distinct 9110 values | 6,915 |
| 13-digit rows corpus-wide | 15,853 |

Mentions per 9110 row: 3,035 carry one, 2,272 carry 2–4, 1,225 carry 5–19, 433
carry 20+. So most of the class is doing real linking work, as issue 312 found
for platform GUIDs.

## But the SHARED values are 67% wrong, and the shape is unmistakable

Only ~50 values are held by more than one row. Of the 46 in the sample:

* **15 are true same-entity** — `Brainlab Sales GmbH` (DE/AT),
  `Hogger GmbH` (DE/AT), `Intuitive Surgical Sàrl` (CH/AT),
  `Bloomberg Finance L.P.` (US/AT), `Mair Wilfried GmbH` (AT/IT).
* **31 are FALSE**, and every one pairs a heavy Austrian public body with a
  one-mention foreign supplier:

```
9110006619920   AT m=315  Bundesministerium für Inneres
                NO m=1    AS Aircontact
9110016030296   AT m=414  WIENER NETZE GmbH
                DE m=1    VTG GmbH Ingenieurbüro
9110010739201   AT m=201  Amt der Oberösterreichischen Landesregierung
                DE m=1    Moog GmbH.
9110010230838   AT m=91   Republik Österreich, vertreten durch die Parla…
                DE m=3    Atelier Brückner GmbH
9110015338201   AT m=218  Tirol Kliniken GmbH
                CH m=1    LEP AG   /   DE m=1  epa-CC GmbH   /   FI m=1  Orion Oyi
```

One buyer's GLN sprayed onto **three** different suppliers in the Tirol Kliniken
case.

## It is NOT our parse. The publisher did it.

Two checks settle it:

1. **Every false pair shares exactly ONE notice** — the Austrian buyer and the
   foreign supplier co-occur once, which is why the foreign row has one mention.
2. **The supplier's own mention carries the buyer's GLN as its raw value.**
   `AS Aircontact`, section `ORG-2` of notice 22273340, has
   `raw_identifier = 9110006619920` — the Interior Ministry's number.
   `VTG GmbH Ingenieurbüro`, section `ORG-7352` of notice 24487219, has Wiener
   Netze's.

The projection recorded what the notice said. **The notice puts the buyer's GLN
in the supplier's organization block.** There is nothing to fix on our side of
the parse, and a "repair" that rewrote these would be inventing data.

## THE ACTIONABLE CONCLUSION IS A NEGATIVE: no checksum arm for this class

A 9110 GLN is mod-10 checkable and 13 digits is an EMPTY shape in
`idgate::uniform_arm`, so an `AT:gln` arm would cost nothing in single-anchor
collisions — the objection that stopped `SK:ico` (issue 326) does not apply.
It looks like a free win.

**It is not, and the reason is this measurement.** An `AT:gln` arm would make
these values read as clean Austrian country evidence. Issue 326's survivor rule
moves a row's country when an anchor names exactly one of its cluster's codes —
so a GLN arm would hand it a class where **67% of the shared values pair
unrelated entities**, and the "correction" would move the foreign supplier to
`AT`, where R2 would then merge it into the Austrian buyer. The Norwegian
aviation firm would become the Austrian Interior Ministry.

Today that path is closed only because `NO`/`AT` and `DE`/`AT` are not one
letter apart. That is luck, not a guard.

So: **the class stays unmodelled, deliberately**, and this issue is the record of
why — the same judgement issue 312 reached for platform GUIDs, from the opposite
direction. There the temptation was to condemn a key that was doing real work;
here it is to bless a key that is right 6,915 times and actively poisonous ~50.

## What could still be done, and what it is worth

* ~~A false-merge tripwire on the shared set~~ — **DONE**, see below.
* **Splitting the 31 false pairs is NOT indicated**: they are already separate
  org rows under different countries, so no merge has occurred. The damage is
  latent, not realised.
* **A corrigendum upstream** is the only real fix, and it is the publisher's.

## The tripwire is live (`f315e2e`, job 554)

Folded into `org-merge-health` beside issue 325's parser-vs-stock gauge, for the
same reason: that census already walks every identifier-bearing org, so the
counters cost nothing.

Two numbers, and only one of them can alarm:

| | |
| --- | --- |
| `shared` | values held by more than one row — the class this issue measured as wrong about two thirds of the time. Reported, never alarms; growth means the publisher-side error is spreading. |
| `shared_one_country` | **must stay ZERO**, alarms on one row, no baseline and no tolerance. |

The asymmetry is the whole design. What keeps the Austrian Interior Ministry
apart from `AS Aircontact` today is **only** that they stand under different
countries, because R2 keys on `(country, kind, identifier)`. A shared GLN whose
rows have collapsed onto a single country is that guard gone — a merge path that
has opened — and one such row is worth a look. The size of the shared set is not.

The alarm rides `parser_vs_stock_alarms` rather than a second mechanism, so it
inherits the test that already exists for the muted-probe lesson (Stage 4
Unit 5), extended with the zero-floor case and with the steady state:
shared-but-multi-country is normal and must **not** alarm.

### First reading, and it cross-checks the hand census exactly

```
rows 6965   distinct 6915   shared 46   shared_one_country 0
```

`shared = 46` is the same 46 values classified by hand above (15 true, 31 false),
computed a second time by a different route — the in-process walk against the
ad-hoc SQL. Two independent computations agreeing is the check worth having on a
number a tripwire will be judged against.

Also confirmed in the same reading: issue 326's 312 country moves did **not**
disturb this class (`shared` unchanged at 46), which is expected — those moves
were 8/9/10/11/14-digit values and this class is 13.
