# 304 — acquire the text era's missing language editions (the un-downloaded TED zips)

Status: READY (filed 2026-08-27 on Lennart's question "we were missing some TED
download for historical multi language — what do we do?"; supersedes the WRONG
line in 291 gap #4 that called text-era non-EN "recoverable by re-dispatch")
Kind: acquisition campaign (staged, capacity-gated)
Relates to: 291 (language capability), ADR-0013 (supersession stays wholesale —
safe under more languages), 232 (the CO archive study is a separate question),
docs/research/ted-access-channels.md (the per-era channel map).

## The facts (ted-access-channels.md, re-read 2026-08-27)

- The text era's bulk archive ships ONE ZIP PER LANGUAGE per daily package
  (1993–1999 flat zips, initially EN-only with languages added through the
  90s; 2000–2007 `{LG}_…_ISO_ORG.ZIP`, ~38 zips/daily; 2008–2010 utf8+meta
  pairs per language, ~46 zips/daily).
- v1 deliberately fetched **EN only** (20× size saving). The other editions
  were NEVER DOWNLOADED — this is a real archive gap, unlike r208/r209 where
  translations sit inside XML we already store.
- The EN edition is often a TRANSLATION: the `OL:` field names the original
  language. So 1993–2010 currently serves translated text as its only text —
  the authenticity gap is the product cost, not just coverage breadth.
- Sizes (from the research): all-language text era ≈ +136 GB (2004–2010) /
  ~150 GB total archive; whole all-language history ≈ 188 GB compressed.
- **The "does not fit" premise is STALE**: /data measured 2026-08-27 at
  1.7 TB with 721 GB free. The archive fits with ~570 GB headroom; the open
  question is the PARSED-layer growth (multilingual text measured ~86% of
  parsed size), which must be measured, not assumed.

## Plan (staged, each stage gated)

1. **r208/r209 first, no download needed**: parser-policy change to keep all
   language variants from the stored XML → re-parse → refold (the proven
   DE-1.x/251 machinery). Measure parsed+canonical growth on ONE month before
   the corpus-wide run.
2. **Text-era acquisition**: fetch the missing language zips through the
   ordinary fetch registry, staged per year (newest text years first —
   2008–2010 have the most structure), with a disk checkpoint after each year.
   Decide breadth at stage start: all languages (+~150 GB, the flexible
   choice) vs original-language-focused subset (zips are per-language per-day,
   so a subset means choosing LANGUAGES, not notices).
3. **Flip the dispatch policy in the same stage** — today's dispatcher
   policy-skips text-era non-EN members (`text-era-non-english`, 1,571 rows),
   so newly fetched editions would land straight in the skip ledger unless the
   policy flips with the campaign.
4. **Refold rides the era-scoped machinery** after each stage's re-parse; the
   ADR-0013 fallback chain then serves the new variants automatically
   (`?lang=` already shipped, 7f89576).

Not in scope: the CO archive (232's winner-half study) — a different corpus
question. Sequencing: after the running epoch refold lands and its deploy
batch (D5 flip + backfill-values + ?lang=) is out.
