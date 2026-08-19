# 244 — 1.3M text-era award notices publish their winners in prose, and nothing extracts them

Status: needs-triage — measured 2026-08-19 on prod (job 751, rev `e2ad213`), the first full run of the
repaired section 3
Kind: extraction gap, the largest single cohort in the corpus
Blocked by: — (wants to ride along with the text-era re-parse already planned for the AU→buyer fix)
Relates to: 235 (the denominator that made this visible), 242 (the column it landed in), 13 (results
layer), 100 (winners unresolved, the eForms half of the same question), 41/202 (text-era ingestion),
the pending text-era re-parse for the `AU:`→buyer mapping

## What

Section 3 of the first full-corpus run, text era:

    era                            award-notices with lot_results  density  no block parsed
    text 1993–2010                  1,306,514              0     0.0%        1,306,514

**1,306,514 award notices, zero materialised results.** By count that is the largest cohort in the
report — bigger than r2.0.8 (1,088,292) and r2.0.9 (2,098,599 award notices at 100.0 %) put together
in terms of what is missing.

And the content is published. From the committed 2005 fixture
(`tests/fixtures/text/2005-can-154-2005.txt`, `TD: 7 - Contract award`):

    CO: Name and address of successful supplier, contractor or service provider:
        Grahams Engineering Ltd.
        NSG Environmental Ltd.

Two named winners, in the era's ordinary labelled-line format with continuation lines — the same shape
every other text-era field uses. The parser maps `CO` as `Prose(None)` (`text/rules.rs`), so the text
is claimed and retrievable as `TXT-CO` under ADR-0004, and then nothing turns it into a result block, a
`lot_results` row, or a winner mention.

## Why it was invisible until now

The old section 3 drew its denominator from notices that already carried a parsed result SECTION, so
an era that parses no result sections at all fell out of both halves and simply did not appear. Issue
235 moved the denominator onto the notice's own published document type (`TXT-TD = 7` here), and the
era appeared — with the largest gap in the report. This is the exact failure class issue 235 was
opened to expose, at a scale nobody had guessed.

## What is not yet known

1. **How much award content each notice carries.** The fixture has winner NAMES. Whether the era also
   publishes award values, dates or contract numbers in adjacent labelled lines is unmeasured — the
   text-era inventory work would answer it, and the answer decides whether this is a winner-only graft
   or a full result block.
2. **How reliable the `CO:` shape is across 17 years.** 1993–2010 is a long span with format drift
   (issues 41 and 202 both hit era-internal variation). The fixture is 2005. A bounded probe of `TXT-CO`
   presence per year, against `TXT-TD = 7`, sizes the extractable subset before any mapping is written —
   the sdk-0.1 lesson from issue 231: check what the era publishes before mapping it.
3. **Whether names in prose can be attributed.** Two names on two lines are two winners here, but a
   single line may hold a name AND an address ("Name and address of…"), so the parse boundary is a real
   question rather than a split on newline.

## Steps

1. Bounded probe: `TXT-CO` presence by year among `TXT-TD = 7` versions, plus a sample of raw values —
   sizes the cohort and shows the format drift.
2. Decide the target shape: synthesise a `LotResult` section per award notice with the winner as a
   party mention, or start with a winner-only text fact. The former makes the era answer the same
   questions as every other; the latter is cheaper and still moves 1.3M notices off zero.
3. Implement with fixtures from at least three eras of the span (the 1990s, 2005, and the 2008–2010
   tail), since the label vocabulary drifts.
4. Land it in the same re-parse pass as the `AU:`→buyer fix — both need every text-era notice re-read
   from the archive, and doing that twice for one era is the expensive way.
5. Re-run the report: the text era's density is the acceptance number, and section 3's "no block
   parsed" total should fall by whatever the probe in step 1 sized.

## Note on the report's own wording

The first render of this column said "Nothing can project those" for the whole of it, which was true
for r2.0.8's 14,532 and false for these 1.3M. Corrected in the same commit that files this issue: the
column measures the PARSE, and the two causes — publisher shipped nothing, or we do not extract it yet
— now get named separately. A correct number under a confident wrong sentence is worse than no
sentence.
