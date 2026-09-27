# Outside API review, fourth pass (against rev `6ddda76`, 2026-09-27) — verified claim by claim

The owner forwarded an outside agent's review with the instruction to be critical of it ("I don't trust the other
agent to actually understand how to use the API … fix the underlying issues by deleting/fixing code, not bolt on
fixes"). Every claim was re-checked against the live service, the parse layer and the code by four independent
read-only verifiers (2026-09-27 12:0x–12:5x UTC). Verdicts, most important first:

| # | Claim | Verdict | Where it lives now |
|---|---|---|---|
| 2 | "Empty party names although the notice names Transport for London" | **CONFIRMED, NEW, understated ~500×.** R2.0.7 notices (2010-03 → 2011-09) publish names as `<ORGANISATION>` direct text (`TED-ORGANISATION`); the fold reads only `TED-OFFICIALNAME`/`TXT-AU`. ~1.06M nameless mentions, ~1.05M of the 1.415M "nameless provisional" orgs. The 2026-09-15 fan-out had it and filed it under "refuted" wrongly. | issue 435 |
| — | (found while verifying 2 and 3) | **ROOT CAUSE, NEW:** a re-parse keeps a mention whose section id survives, and the fold returns early for a recorded mention — so NO parse or mapping fix ever reaches a standing organization mention (R2.0.7 names, text-era `TXT-CY` country, issue 393's Greek decode). | issue 434 |
| 6 | "Full-table SQL scans got slower … the concurrency fix may have cost throughput" | **REAL REGRESSION, wrong cause.** Issues 425/120's engine deadline makes turso read the clock before every VDBE instruction: local A/B 1.7–2.9× slower on cached scans. Of the reviewer's three 408s, one was a cold cache (3.3 s now) and one a planner trap in the reviewer's own SQL (`GROUP BY +source` → 2.6 s; `prod-box-reads.md` trap 3). The box was idle during their run, so not write load. | issue 438 |
| 5 | "100 newest UK rows: 49 no CPV, 58 no country" | **CONFIRMED, NEW mapping gap for most:** UK5/6/7 award releases publish CPV and region only on `awards[].items`, which the FTS parser drops; its checklist marks them "restate the tender's" — false (those releases have no `tender.items`). 19 of 60 region gaps are source reality. | issue 437 |
| 3 | "Deutsche Bahn keyed by procurement reference numbers" | **CONFIRMED, known (365 exhibit, never given a unit).** The id gate has no class for OJ S notice numbers (403 rows corpus-wide; org 13782393 fuses DB AG, DB Netz and ~30 names). DB's own reference family is left as an open question (publisher-specific pattern = bolt-on). | issue 440 |
| 3 | "VAT DE811335517 shared by eight" | **Count right, framing wrong.** Not a VAT group: one Bavarian state registration used by several authorities; the resolver's preload binds to an ARBITRARY duplicate (no ORDER BY), so the rows now mix authorities. | issue 439 |
| 3 | line breaks in 106,785 org names | **Partly ours, NEW:** the text-era `AU` field is joined with `\n` (the bug issue 397 fixed for titles); issue 330's "publisher-side" verdict rested on comparing two outputs of the same join. | issue 436 |
| 3 | "national id 408712 shared by 15 organizations in six countries" | **WRONG.** 15 rows in 15 countries, one entity (Swiss SDC/DEZA filing from each host country); excluded by design (issue 326). | — |
| 3 | provisional 5.7M / empty 1.4M / no-country 2.6M | **By design as definitions** (provisional = no identifier; nameless minted per issue 234/365) — but ~1.05M of the nameless and much of the text-era no-country are the defects above. | 434, 435 |
| 1 | tender 430681 welds 789 notices | **CONFIRMED, known and decided** (issue 377: a Swiss platform's constant BT-04 for 16 months; three statistical gates each measured and each fails on legitimate DPS). The suggested fix — give that UUID "the same treatment as the all-zero UUID" — is a hard-coded denylist of one string: rejected. | 377 |
| 2 | "4268628 has no title although its content carries the title in 23 languages" | **WRONG.** Those 23 rows are `TED-TI_TEXT`, the OJ heading = the CPV label; the notice publishes no title (issue 368, decided). | 368 |
| 2 | title-less clusters 2016-03-31…04-02, Jan 2015 | **Numbers right, framing wrong.** 1,702 of 1,757 are one Greek publisher's (EETAA) bulk childcare award notices with no title in the source. Source reality. | 368 |
| — | deadline sort puts null-deadline rows first | **CONFIRMED, known (issue 171 rule 12):** three stale `current_deadline` head columns; the read path deliberately not patched; the full fold re-elects them. No code change. | 171 |
| 4 | "CPV mixed across time … three formats" | **STALE** — measured before this morning's issue-394 island refold (all bare 8-digit now across DÖE/TED/FTS samples). One real residue: 42 DÖE `eforms-sdk-1.0` tenders (2023) still serve newline-glued codes, folded before the normaliser; the next full fold fixes them. | 394 |
| 5 | "German portal value on 0.5 %" | **Source reality, known** (issues 231/263: 0 of 50 island members publish money). | 231 |
| 5 | "lot titles null for 48 % at id 12 million" | **Source reality, known** (issue 368: lot id space; undivided single-lot contracts publish no lot title). | 368 |

## What happens next (owner go-ahead to act autonomously, 2026-09-27)

The queued full fold (1597) and 404's dry (1598) were cancelled so the fold runs ONCE with the fixes: build and gate
434/435/436/437/438/439/440, deploy when the XML re-parse (1596) finishes, then text re-parse (436) + FTS re-parse (437)
+ issue 432's name-norm repair + ONE full fold (which now refreshes stale mentions), then 433's reclaim, 404's repair,
440's placeholder repair, and every Verify.
