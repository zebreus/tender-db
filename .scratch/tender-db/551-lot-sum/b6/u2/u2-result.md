# Issue 551 B6 U2 -- result (decision points 4 and 7)

Produced by `run_u2.py` (offline; network disabled; reproduces every output byte for byte; `--check` verifies). Public-API reads by `fetch_u2_api.py` (15 GETs, `api_requests.log`).

## Inputs

- Port of record `b6/rule_v2_g.py` **sha256 23cbd8508a3afa35b195f8a2ee91a75bb3b8f08aa2e2f3f9abbd0ea80454452f** (matches `b6/SHA256SUMS`; not edited).
- Rates `m551/rates/ecu-ert_bil_eur_d.csv` sha256 48e2c9f17ae7219b2b145a66ace399aba7c306c88721b7ce8f7c76f4758e131b (the 10-17 set U1 recorded; not refreshed -- refresh is before U6).
- Rates `m551/rates/ecu-ert_h_eur_d.csv` sha256 47e941270ac4a0f0f22416bd4b41c9b427d43c3a4f430996f8457dca8f134e5c (the 10-17 set U1 recorded; not refreshed -- refresh is before U6).
- Rates `m551/rates/eurofxref-hist.zip` sha256 8e925cef1572f8df5c8743229a761b029180f3b5cd4aa2213592192964ebd793 (the 10-17 set U1 recorded; not refreshed -- refresh is before U6).
- Fresh U2 layer: 3502 tenders (digest cda6154ab88077d069ffe1b58553bc381a9debb91fddb38bf1b446c459568a1f), 15537 notices (digest d943e0b862b2157e9acef12e61af78246848c736374b190bbc5776f71ef3d266); notice layers used: `b6/notices` 15246; `cache/notices` 291. Port errors / misses: 0 (no notice needed the deleted rb8 layers).
- Tender fetch times 2026-10-19T05:47:56Z .. 2026-10-19T07:41:30.381Z.
- `served_lsa_rows.json` sha256 1407b38da2a14ddf4ce057652dad3ad502ecfed87b0e2f464d2b4fdfc68fb93a
- `l_changed_571_ids.json` sha256 c07bb97fc8d2abd9a61ddf7a0fb4f553fe32f8cfe96194eadb12e1399e769a05
- `l_changed_545_ids.json` sha256 334c15b9674cacf715d5bcec494879387d00b736c9a3aa6c4c0ace6c2f66e9bc
- `api/manifest.json` sha256 9af3494d312f0beb4900feb339a24a7a80720598296fcbbccc109b24d2193964

## 1. The provisional L_changed (fixed per id at fetch time, before comparing)

- Feed: `GET /v1/changes?since=958787061&entity=tender&limit=1000`, 9 pages to the head (last_cursor 958848784, more=false), 8573 tender rows. By job stamp:
  - 2026-10-19T03:34:07Z: 3502 rows, 3502 ids (3502 of the 3,502), cursor 958787062..958809773, {'changed/version-null': 3502}
  - 2026-10-19T04:23:30Z: 126 rows, 126 ids (0 of the 3,502), cursor 958809779..958809905, {'changed/version-null': 126}
  - 2026-10-19T05:53:39Z: 66 rows, 66 ids (0 of the 3,502), cursor 958809906..958810036, {'changed/version-null': 66}
  - 2026-10-19T07:35:55Z: 4784 rows, 4081 ids (0 of the 3,502), cursor 958811147..958848404, {'added/versioned': 2023, 'changed/versioned': 2648, 'changed/version-null': 102, 'removed/version-null': 11}
  - 2026-10-19T11:06:08Z: 48 rows, 48 ids (0 of the 3,502), cursor 958848498..958848545, {'changed/version-null': 48}
  - 2026-10-19T11:06:21Z: 47 rows, 47 ids (0 of the 3,502), cursor 958848546..958848656, {'changed/version-null': 47}
- **Literal reading** (a change row after 958787061 up to the fetch time): **3502 of the 3,502** -- every id, through its row from the switch-off drain-back 2630 (cursor (958787061, 958809778], 3502 rows stamped 2026-10-19T03:34:07Z, exactly the 3,502). The parity would compare nothing.
- **Operative reading** (2630 excluded): Read literally, criterion (a) puts every one of the 3,502 in L_changed: project 2630 (2026-10-19 03:34-03:36Z, refold-lot-sum by the 3,502 at W=false) wrote one version-null `changed` row for each of exactly the 3,502, after 958787061 and before every fetch, so the parity would compare nothing. Those rows are path W's own drain-back (each served head going from lot-sum-awards back to its max election; issue 551 "B5 -- switched off and drained back"), not a change of any fact the port reads: the decision excludes them itself where it builds the complement ("minus 2630's own correction rows") and lists the post-2627 fact changes as 541, the daily chains and 571 only. The one code change 2630 carried besides W=false is 541 unit 2 (923172d: T02 MOVE awards write a RES-T02), inert on the 3,502: none of their 15,537 notices is a T02 or carries TED-COST_PARAMETERS (one T01 prior-information notice, 18002514 on 5278856, which 541 does not read). The literal reading is recorded in literal_reading.
- Criterion (a), 2630 excluded: **0** ids. No id of the 3,502 has any non-2630 tender row even up to the head (0).
- Criterion (b), notices: **0** ids. The decision words this three ways: "a notice folded since 2026-10-18 20:30Z" (point 4, amendment B5), "the tender ids of every notice ingested since 2026-10-18 20:30Z" (synthesis) and "notices folded since 2627, not ingested_at (notices fetched before the cutoff but parsed later fall through)" (C3). Wider reading taken: first folded (parsed and folded for the first time) since the EARLIER cutoff 2026-10-18T20:30:00Z. Re-folds of already-folded notices are not counted (2627, 2630 and the zero-drift probe re-folded every notice of the 3,502, which would make the criterion vacuous). On the public API the only per-notice instant is ingested_at; it is assigned at insert in id order, so the probes binary-searched the boundary over the 15,537 named notices: the newest named notice, 47406725, was ingested 2026-10-16T07:35:45Z, before the cutoff, so no named notice was ingested since it. A notice ingested earlier but first parsed later (a quarantine reclaim or a re-parse keeps its ingested_at) is not visible per notice without a detail GET of each; no reclaim or re-parse job is on record in the window (the daily chain does not reclaim), and such a fold that moved facts, rounds, groups, closure or value writes a tender change row (canonical.rs append_version_changes), which (a) reads; only a silent (projection-only) version would escape. Recorded as a residual. Probes (id, ingested_at): [[47396127, '2026-10-14T07:35:02Z'], [47398305, '2026-10-15T07:35:03Z'], [47400399, '2026-10-15T07:35:03Z'], [47401649, '2026-10-15T07:35:03Z'], [47402365, '2026-10-15T07:35:32Z'], [47406725, '2026-10-16T07:35:45Z']].
- Sets: 541 2632 corrections 126 (feed range (958809778, 958809905]), 571 66 (= the feed's 05:53:39Z rows), 545 47 (= the feed's 11:06:21Z rows).
- **Provisional L_changed: 239 ids** (sha256 5b5eb683f22b2cf7a2d29fc7ff314dbed2008b35d635d0b60bddb08487305605), **0 of them among the 3,502**, so all 3502 are compared.

## 2. v2 parity (port guards=frozenset() vs served_lsa_rows.json)

- Compared **3502** (outside L_changed), **agree 3502, disagree 0**, unread 0, port errors 0. Kinds: none.
- vouched_by a member of the port's list but not its first: 0 (A4's known naming difference; not a disagreement).

## 3. P and the guard sets

- v2 ships 3502 of 3,502 (refuses 0).
- **v2+single: P = 3468** (ids sha256 91c7c55c842286ef3ea076fd2124991aacfab7ee9d7c697c8cfd3025f47ed69f), g = 34; vouched_by moved but still ships 0; B_P (>= EUR 100 m) 29: 683951, 707424, 726354, 1102571, 1147190, 1148297, 5278856, 5769515, 5842187, 6017838, 6057925, 6195903, 6370901, 6435162, 6548237, 6569442, 6603588, 6679375, 6718092, 7001924, 7011414, 7139761, 7167278, 7196993, 7461921, 7542368, 8407303, 8426318, 8460630; within 2 % of EUR 100 m: 899272, 5842187; 5816379 shape 286 (lot-less later award 144: 5557595, 5574008, 5673713, 5816379, 5826698, 5847452, 5859852, 5863173, 5907365, 5914032, 5924297, 5935164, 5944341, 5970225, 5992450, 5993394, 5999026, 6015175, 6041010, 6043463, 6052980, 6096892, 6113606, 6129027, 6130837, 6154617, 6168075, 6173282, 6177790, 6210355, 6214381, 6230981, 6253467, 6273141, 6296643, 6297927, 6298078, 6306585, 6345927, 6357976, ... (144 in all)).
- **v2+cover: P = 3489** (ids sha256 8a6b5a9b7348f27b367e6218d424cb635bb71090f722737b2c3de0e9d2df3bd7), g = 13; vouched_by moved but still ships 0; B_P (>= EUR 100 m) 29: 683951, 707424, 726354, 1102571, 1147190, 1148297, 5278856, 5769515, 5842187, 6017838, 6057925, 6195903, 6370901, 6435162, 6548237, 6569442, 6603588, 6679375, 6718092, 7001924, 7011414, 7139761, 7167278, 7196993, 7461921, 7542368, 8407303, 8426318, 8460630; within 2 % of EUR 100 m: 899272, 5842187; 5816379 shape 296 (lot-less later award 144: 5557595, 5574008, 5673713, 5816379, 5826698, 5847452, 5859852, 5863173, 5907365, 5914032, 5924297, 5935164, 5944341, 5970225, 5992450, 5993394, 5999026, 6015175, 6041010, 6043463, 6052980, 6096892, 6113606, 6129027, 6130837, 6154617, 6168075, 6173282, 6177790, 6210355, 6214381, 6230981, 6253467, 6273141, 6296643, 6297927, 6298078, 6306585, 6345927, 6357976, ... (144 in all)).
- **v2+both: P = 3467** (ids sha256 a2619fd9e380d3d5672b3417d807e504b3a85965b8e5ea319425564e3cd21872), g = 35; vouched_by moved but still ships 0; B_P (>= EUR 100 m) 29: 683951, 707424, 726354, 1102571, 1147190, 1148297, 5278856, 5769515, 5842187, 6017838, 6057925, 6195903, 6370901, 6435162, 6548237, 6569442, 6603588, 6679375, 6718092, 7001924, 7011414, 7139761, 7167278, 7196993, 7461921, 7542368, 8407303, 8426318, 8460630; within 2 % of EUR 100 m: 899272, 5842187; 5816379 shape 286 (lot-less later award 144: 5557595, 5574008, 5673713, 5816379, 5826698, 5847452, 5859852, 5863173, 5907365, 5914032, 5924297, 5935164, 5944341, 5970225, 5992450, 5993394, 5999026, 6015175, 6041010, 6043463, 6052980, 6096892, 6113606, 6129027, 6130837, 6154617, 6168075, 6173282, 6177790, 6210355, 6214381, 6230981, 6253467, 6273141, 6296643, 6297927, 6298078, 6306585, 6345927, 6357976, ... (144 in all)).
- **R_single = 34** (sha256 65224f5239d68b4ed1fec108b2b03024cc4a24ee436a1fa03ec4b50d44b4aa5d): 402503, 472292, 5581720, 5663701, 5884304, 6003114, 6234738, 6342738, 6356382, 6403567, 6465510, 6582856, 6595330, 6784258, 6815094, 6838776, 6852228, 6937514, 7014656, 7102361, 7163120, 7166738, 7179291, 7348648, 7354861, 7380025, 7394272, 7423850, 7600895, 7605188, 7608791, 7651470, 7675062, 7785319. Shape: {'removed_field': {'BT-161-NoticeResult': 3, 'TED-VAL_TOTAL': 35}, 'removed_kind': {'award': 38}, 'v2_vouched_by_field': {'BT-161-NoticeResult': 2, 'TED-VAL_TOTAL': 32}, 'year': {'2016': 1, '2017': 1, '2018': 3, '2019': 1, '2020': 3, '2021': 4, '2022': 4, '2023': 14, '2024': 2, '2025': 1}, 'eur_band': {'<100k': 4, '<100m': 5, '<10k': 3, '<10m': 12, '<1m': 10}}.
- **R_excess = 1** (G-cover refuses beyond G-single; sha256 39165ec9cbe9a7a28340d73fe7695bbb7c9e1c94ca6920ea7632041c702b3b46): 7202436. Shape: {'removed_field': {'TED-VAL_TOTAL': 1}, 'removed_kind': {'award': 1}, 'v2_vouched_by_field': {'TED-VAL_TOTAL': 1}, 'year': {'2022': 1}, 'eur_band': {'<100k': 1}}. All G-cover refusals 13, of which in R_single 12; R_both minus R_single = 1 (equal to R_excess: True); refused only with both guards: none.
- Adoption reads (U4, not done here): min(|R|, 30) of each set in `sha256("551-b6-gcost-"+id)` order, M2 protocol; adopt iff >= 10 % non-right.

## 4. Estimate-vouch flags (point 7 / D4; recorded only, flagged to B-fresh readers)

- Definition: Every estimate-kind R10 candidate of every v2 ship, tested: from a CN-type notice with exactly ONE lot section
    whose served key (the 570 remap target of that section) is not a summed key. Then, per guard set, the ships whose
    KEPT vouch list holds such a candidate (any), whose vouched_by is one (first), or whose every kept vouch is one
    (only: S has no other support). -> (is_cn, reason). CN-type when the notice's form says so: a TED-FORM in the port's frozen CN set
    (cn_type), an eForms subtype (OPP-070-notice / DE1-NoticeSubType-SubTypeCode) 1..24 or E1..E3, or a BT-02-notice
    cn-*/pin-*/qu-*/subco*; with no form field at all, a notice holding no LotResult section.
- Estimate candidates examined (v2 ships): 605; ids with a flagged candidate: 0.
- v2: any 0 (none); first (vouched_by) 0; only (no other kept vouch) 0 (none).
- v2+single: any 0 (none); first (vouched_by) 0; only (no other kept vouch) 0 (none).
- v2+cover: any 0 (none); first (vouched_by) 0; only (no other kept vouch) 0 (none).
- v2+both: any 0 (none); first (vouched_by) 0; only (no other kept vouch) 0 (none).

## Outputs (sha256)

- `u2/l_changed_provisional.json` bc893c3da0e649a8600f3b0c683cfe0646bf033f513bc824ce3df1a4c3495cf0
- `u2/decisions.jsonl.gz` 0add92e36c70ca76b81049e9faf678e91305e790d1fd914ff4952a0f1d0749cb
- `u2/parity_v2.json` 8457029c6a19d54293b986e5cb90e2b8dfd5c7d4b917cbb1403dda075a003034
- `u2/P.json` 5f8b7f7ce50b07d9a474539d0aa7a162a6a1933e638ec9914218c22727f4f75b
- `u2/R_single.json` c1b413e034dc8bbc57229cc6b89b4eacc7be60f700dc6d93890fefd0afa62c60
- `u2/R_excess.json` 3d134390f82269a5c568e3a82269264b5f16648bcfc5cb63372cc8f711b8e252
- `u2/estimate_flags.json` c53d91720ada09e778b85f449ed24c835afc907925f3534b1138572421e36734
- `u2/u2-result.json` b608b8812b88f4cbfa3b393f599b66ad823f9463edd915daa1ae137affb02ee5
- `u2/run_u2.py` 376e8652c80eae1215656ce4e4c914b52eedce9c85ab5fb115097a3728a0cc0a; `u2/fetch_u2_api.py` 7c38605aa6be0e9100082371d953a363fcb24b85ff84dcec54183c1211979163
