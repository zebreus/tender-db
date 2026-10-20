# 560 U-d + 562 unit 5 second half — prepared 2026-10-20 06:3xZ

- 560 U-d: `q_ud_1019.sql` = `scratchpad/d56x/adv/q_adv.sql` with the publication day moved to 2026-10-19
  (published_at in [1792368000, 1792454400)) and the D5 window to 10-20 ([1792454400, 1792540800); "covany" from 1792454400).
  Run ONCE after the 10-20 chain's rederive-eur-recent (D5) has finished, with no refold/drain running. Bar (decision
  amendment (b)): lots neither path covers vs the ~257-lot pre-deploy baseline of 2026-10-14 (TED EUR-only 122, TED non-EUR
  32, FTS 103); (c) a residue version is a stop only when its value moved at FOLD-TIME rates.
- 562 unit 5 (second half): re-run `.scratch/tender-db/562-u5/gen.py` with LO = 958848784 (the cursor before the 10-20 chain;
  nothing writes until 07:35Z) and HI = the cursor after the 10-20 daily `project` job, then the read of
  wf_c31642a7-759 (flips by q1's notice-id form = announced = the project line's `relabelled`; TED-first 0; co-minted 0).
