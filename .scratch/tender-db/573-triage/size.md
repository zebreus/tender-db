# 573 unit 1: sizing the dropped-accent and mojibake heads (2026-10-19)

The input is plan 2653 (`548-triage/org-name-refresh-plan-2026-10-19-v13.json.gz`: epoch `524u2-v13`, rev `dc86e8e`,
as_of 2026-10-19, `held` 11,515 and not truncated, `planned` 1,693). The work was read-only: Python on that one file,
with no network, no box, no code edits and no git. The script is `size.py` (`python3 -I size.py` rewrites
`size.json`). `size.json` carries every hit with all of its plan fields and the per-option readings.

## Result: 17 hits, every one shown below. No salted draw, because 17 ≤ 60.

| verdict:reason | dropped-accent (literal test) | dropped-accent (char-level only) | mojibake marker | total |
|---|---|---|---|---|
| hold:head_current | 12 | 2 | 0 | **14** |
| hold:review_body | 1 (the exhibit) | 0 | 0 | **1** |
| change:R2 | 0 | 0 | 2 | **2** |
| **total** | **13** | **2** | **2** | **17** |

- **The literal test (from the task)** splits on whitespace. Each head word must equal the matching spelling word once
  that word's non-ASCII letters are deleted, compared case-folded. At least one word must differ. It finds 13 heads.
- **The char-level test** catches the same shape when a whole `à` and its no-break space were dropped. It finds 2 more:
  22399787 and 22399788, where `Pointe--Pitre` came from `Pointe-à\xa0-Pitre`. The test deletes every non-ASCII
  character of the spelling, collapses whitespace and compares the result case-folded. It also requires:
  - a word that keeps an ASCII letter lost a letter, so a partial drop rather than an omitted word;
  - no deleted word was made only of 2 or more non-ASCII letters.

  The second guard excludes 11221362 (`Johnson & Johnson Hellas` against `… HELLAS ΑΕΒΕ`) and 22751177 (`ABBOTT
  MEDICAL` against `… ΕΛΛΑΣ ΕΠΕ`). Those heads leave out Greek words; nothing in them was dropped by a broken decoder.
  The char-level test finds all 13 literal hits too, and the literal test finds nothing the char-level test misses.
- **Mojibake markers** are `Ã`/`Â`/`Ä` before a continuation-like character, `€`, `™` and U+FFFD. The head's own unfit
  test already catches both hits: 22504462 reads `mojibake` and 22678565 reads `control`. Both are R2 changes at H = 1.
  They are planned, but no wet run ships them while `exclude_rules` names R2.
  - Three `Ã`/`Â` heads are legitimate and are not markers: 5770 `ASSOCIAÇÃO`, 11109 `ROMÂNA` and 22783706
    `INSTALAÇÕES`.
  - Other near-misses: no head has `?` in place of the accented letters, and no partial drop exists beyond these.
- **The read** covers the whole listing: 16 R, 1 N, 0 W.
  - On the dropped-accent arm, 15 of 15 are R. Each change serves the top spelling of the same name with its accents
    restored.
  - The two Pointe-à-Pitre changes serve `Pointe-à -Pitre`, because the display collapse turns the no-break space into
    a space. That is a cosmetic flaw and the change is not wrong.
  - 22678565 is N: an R2 at T = 4 of V = 10 over 5 clusters, whose new name is the same unit with `ROMA` added.
- **These 15 heads are one batch.** All are FR, with `head_newest` between 2024-01-22 and 2024-02-19 and `top_newest`
  between 2024-05 and 2024-09. No newer head has the shape.

## Every hit

Column key:
- **blocks today**: the hold that holds the change now.
- **then**: the hold that would hold it next if that one were released.
- **(a)**: an R0-style defective-head rule (R0d). It asks only the bind guards.
- **(b)**: the unfit test is taught the shape, so the change becomes an `unfit` junk release. Every wet run drops that
  release with `exclude_released: ["unfit"]`.
- **(b′)**: for comparison only, a narrow release that the wet filters let through.

| org | verdict:reason | head (old_name) | spelling it matches (n) | V/T/H | head_newest | blocks today | then | (a) R0d | (b) unfit | (b′) | read |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 22397161 | hold:head_current | `Comit consultatif interrgional des litiges en matire de marchs publics` | `Comité consultatif interrégional des litiges en matière de marchés publics` (58) | 59/58/1 | 2024-01-23 | head_current | — | yes | no | yes | R |
| 22399787 | hold:head_current | `Tribunal judiciaire de Pointe--Pitre` | `Tribunal judiciaire de Pointe-à\xa0-Pitre` (19) | 22/19/3 | 2024-02-05 | head_current | — | yes | no | yes | R |
| 22399788 | hold:head_current | `Tribunal judiciaire de Pointe--Pitre` | `Tribunal judiciaire de Pointe-à\xa0-Pitre` (19) | 22/19/3 | 2024-02-05 | head_current | — | yes | no | yes | R |
| 22433242 | hold:head_current | `Tribunal administratif de Nmes` | `Tribunal administratif de Nîmes` (36) | 40/36/4 | 2024-01-22 | head_current | — | yes | no | yes | R |
| **22435268** | hold:review_body | `Comit consultatif interrgional du rglement amiable` | `Comité consultatif interrégional du règlement amiable` (18) | 21/18/3 | 2024-02-09 | review_body | head_current | **yes** | no | no | R |
| 22441088 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (9) | 11/9/2 | 2024-02-19 | head_current | — | yes | no | yes | R |
| 22441089 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (9) | 11/9/2 | 2024-02-19 | head_current | — | yes | no | yes | R |
| 22460863 | hold:head_current | `Tribunal judiciaire de Crteil` | `Tribunal judiciaire de Créteil` (9) | 11/9/2 | 2024-02-06 | head_current | — | yes | no | yes | R |
| 22466820 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (23) | 25/23/2 | 2024-02-02 | head_current | — | yes | no | yes | R |
| 22466821 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (23) | 25/23/2 | 2024-02-02 | head_current | — | yes | no | yes | R |
| 22488363 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (17) | 19/17/2 | 2024-02-05 | head_current | — | yes | no | yes | R |
| 22499321 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (4) | 6/4/2 | 2024-02-05 | head_current | — | yes | no | yes | R |
| 22499322 | hold:head_current | `Tribunal administratif d'Orlans` | `Tribunal administratif d'Orléans` (4) | 6/4/2 | 2024-02-05 | head_current | — | yes | no | yes | R |
| 22527505 | hold:head_current | `Tribunal Judiciaire de Chambry` | `Tribunal Judiciaire de Chambéry` (8) | 9/8/1 | 2024-02-07 | head_current | — | yes | no | yes | R |
| 22527506 | hold:head_current | `Tribunal Judiciaire de Chambry` | `Tribunal Judiciaire de Chambéry` (8) | 9/8/1 | 2024-02-07 | head_current | — | yes | no | yes | R |
| 22504462 | change:R2 | `MarchÃ©s sÃ©curisÃ©s - Atline Services` | marker `Ã©`; unfit today: `mojibake` → serves `Atline Services` | 9/4/1 | 2024-02-28 | exclude_rules R2 | — | only with a marker arm | no | no | R |
| 22678565 | change:R2 | `COMANDO LOGISTICO AM - SERVIZIO INFRASTRUTTURE - 3� REPARTO GENIO BARI PALESE` | marker U+FFFD; unfit today: `control` → serves `COMANDO LOGISTICO SERVIZIO INFRASTRUTTURE ROMA - 3° REPARTO GENIO A.M. BARI PALESE` | 10/4/1 | 2024-07-10 | exclude_rules R2 | — | only with a marker arm | no | no | N |

All 15 dropped-accent hits are R1, and in each one the matched spelling is the plan's `new_name`, the top cluster's
display spelling.
- **(a) yes:** every one of these holds sits after the bind guards in `elect_head_counted`. `head_current` comes after
  `serve`, and `review_body` is a serve hold that runs after `head_change_hold`. So the bind guards have already passed,
  and R0d asks nothing else.
- **(b) no, for two reasons:**
  - A `head_current` hold asks the junk exemption. A head the unfit test reads as unfit is released `unfit`, and every
    wet run then drops it with `exclude_released: ["unfit"]` (v12's filter, which stands while its re-judge is failed).
  - `review_body` never asks the junk exemption, so its hold stands whatever the unfit test says.

  Separately, `head_spelling_unfit(candidate, head)` cannot see the cluster's spellings, while the dropped-accent shape
  is a relation between the head and another spelling. So (b) needs a new input as well as a new arm.
- **No top_stale behind them:** every dropped-accent hit has 2 clusters and H ≥ 1, so no rival exists and `top_stale`
  would never hold them.

## The exhibit, 22435268

- **review_body holds it today.** The new name has the phrase `reglement amiable`, and the head is not review-like:
  - its folded words `comit consultatif interrgional du rglement amiable` contain neither that phrase nor the stem
    `reglement amiable`;
  - no other stem of `HEAD_REVIEW_STEMS` appears, and the head is not led by `ta`, `tar` or `vk`.

  So the broken head makes a review body's self-rename look like a review body served for another entity.
- **If review_body were fixed, head_current would hold it.** Its head was printed 2024-02-09, which is within 1,095 days
  until 2027-02-08. Under (b), that `head_current` hold would release it `unfit`, and every wet run drops `unfit`
  releases. `top_stale` would never hold it: there are 2 clusters and H = 3, so no rival exists.
- **Only (a) ships it.** R0d serves `Comité consultatif interrégional du règlement amiable` (18 of 21 votes). The bind
  guards it asks have already passed.

## Lower bound

17 is a lower bound for three reasons:
- **Keep entries are not listed.** That is S1 523,538 orgs, bar 16,996 and D4 1,733. The plan's uniform keep samples
  read 0 of 30 for each of S1, bar and D4 (and E1), and the 59 canaries read 0. That shows the shape is rare, but it
  bounds nothing in absolute terms: 0 of 30 is consistent with up to about 10% of S1.
- **Only the top 5 spellings are listed per entry**, so a matching spelling ranked lower is missed.
- **A head whose own cluster is the top (S1) would not be served its repair by R0d as recommended below.** That rule
  elects the matched cluster only when it is the unique top.

## The options compared on this listing

| path | ships now | the exhibit | note |
|---|---|---|---|
| (a) R0d, dropped-accent only | **15** (all read R) | **yes** | the 2 R2 marker heads stay R2 |
| (a) R0d with a marker arm | 17 | yes | the arm ships 22678565 (N), and on `unfit(head, head)` it would reopen 552 (line-break heads) |
| (b) teach the unfit test (an `unfit` release) | **0** | no | `exclude_released: unfit` drops it on every wet run; review_body never asks |
| (b′) teach it and add an admitted release reason | 14 | no | review_body still holds the exhibit |
| wait | 14, unread, as head_current lifts 2027-01-21 … 2027-02-18 | never | the same "ships unread as it ages out" shape as issue 574 |

## Recommendation for unit 2: decide (a) R0d, narrow

1. **Trigger.** The head is the char-level dropped-accent form of a spelling S of another cluster:
   - S with every non-ASCII character deleted and whitespace collapsed equals the head, compared case-folded;
   - a word that keeps an ASCII letter lost a letter;
   - no deleted word was made only of 2 or more non-ASCII letters.

   It is a pure test over the head and the cluster spellings that `elect_head_counted` already holds. The char-level
   form covers the literal test's 13 and the 2 Pointe-à-Pitre heads, and the last guard excludes the 2 Greek omissions.
2. **Elect** S's cluster only when it is the unique top and passes R1's bars: T × 20 ≥ V × 11, T ≥ 3 and T ≥ 2H. All 15
   pass; the tightest are 22499321 and 22499322 at 4/2 of 6. The rule serves the cluster's display spelling, with the
   twin rule as today. Because S's cluster must be the top and T ≥ 2H, an S1 head never fires and the rule never flips
   toward a minority spelling.
3. **Ask only the bind guards**, as R0n does. Never ask the window, residue, serve, `head_current` or `top_stale` holds,
   or R2's initialism hold. Their reasons are about a real rename that the votes have not caught up with, and they do
   not apply when the head is a byte-dropped copy of the top spelling.
4. **No marker arm.** R0d on `unfit(head, head)` would sweep in control-only line-break heads, the class 552 found wrong
   to release. The two marker heads are R2s at H = 1 and belong with R2's return. A narrower garbled arm (the head's
   cp1252 round trip equals a top-cluster spelling) fires on 0 here: 22504462 repairs to `Marchés sécurisés - Atline
   Services`, which is no listed spelling. It is not needed.
5. **Wet:** add R0d to the values `exclude_rules` accepts, and do not add it to the runbook's
   `["R2","R0w"]`.
6. **Bar: 0 wrong on the read.** This read meets it: 15 R on the dropped-accent arm, and 16 R / 1 N / 0 W over all 17.
   Unit 3's re-plan should produce exactly these 15 R0d changes, or a superset, and every item outside the 15 must be
   read before the wet run.

## Side notes (out of 573's scope)

- **Possible duplicate orgs.** Five pairs of orgs with consecutive identifiers carry identical spellings and counts:
  - 1107949/48, 949579/78, 628524/23, 680375/74, 582261/60;
  - seven orgs, 22441088 … 22499322, are all `Tribunal administratif d'Orléans`.
- **The no-break-space collapse.** The display collapse prints `Pointe-à\xa0-Pitre` as `Pointe-à -Pitre`. That is a
  cosmetic follow-up if wanted.
