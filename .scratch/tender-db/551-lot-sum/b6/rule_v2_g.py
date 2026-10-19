"""Issue 551 unit 4b, M1: path W v2 ("lot-sum-awards"), ported to PUBLIC-API facts.

B3 COPY (issue 551 B3, 2026-10-18), written by ../b3/make_port_copy.py from unit4b/rule_v2.py (sha256 39be6d1e...05ce).
Changes, each an asserted replacement (see make_port_copy.py EDITS_RULE): (a) issue 570: a summed or round award's
published lot number is read from the notice's lot section whose 570 remap TARGET is the served key
(remap_port.remap with K3, through 568-u8 combined_port.remap_chain over the version chain in fold order), not from
the section whose own id equals the served key; (b) R10 at every size ("vouched everywhere", M2 fallback PASSED);
(c) reads are offline: /v1/tenders/{id} from the B3 post-drain re-reads only, notice contents from the B3 fetches,
then the M1 cache, then B2's committed bundle (a refold does not change a notice's parsed content); no GET;
(d) facts() is split into facts_from(x, contents, v1) so the 4,745 recorded records can be fed in (b3/records.py).
568 unit 8's refusals need no edit: the port reads the served lot_results, where a refused block is `lot: null`.

B6 COPY (issue 551 B6 U1, 2026-10-19), written by ../b6/make_rule_g.py from b3/rule_v2_b3.py (sha256 5400bc55...).
Changes, each an asserted replacement (make_rule_g.py EDITS):
  (a) the fresh tender layer: /v1/tenders/{id} is read ONLY from scratchpad b6/tenders/{id}.json.gz (filled by
      b6/fetch_b6.py); notice contents from b6/notices/{id}.json.gz first, then the existing layers (B3 raw, the M1
      cache, B2's bundle, B3's verifier, B5's fetches, the 568 read-back band / verify_band / u7 sets). Notice
      contents are immutable; tender JSON is not, so no older tender layer is read. A miss raises FetchError. The
      rule makes no GET (the fetch code is removed, not just unreachable).
  (b) the two candidate guards of issue 551 "B6 -- decision" point 2, worded exactly as decided:
      G-single: a stated total from a notice whose round in this version has exactly ONE LotResult (keyed or not;
                the served lot_results rows with that notice_id) does not vouch.
      G-cover:  a total of kind Award (r3_kind == 'award': BT-161, BT-118, BT-1118, TED-VAL_TOTAL on a CAN-type form)
                or Range (field in ('TED-LOW', 'TED-HIGH')) vouches only if its notice names at least one summed
                key, by R3's own test (results_by_lot[key] holds a row of that notice), keyless results excluded.
                Estimate totals are unchanged.
      Both only filter R10's candidate list (facts()['vouch_candidates']); the refusal name stays R10. Each emits a
      diagnostic (g_single_removed / g_cover_removed, and their union g_removed). R1..R9 and R3 are unchanged.
  (c) the switch: decide(tender_id, guards=frozenset() | {'single'} | {'cover'} | {'single', 'cover'}) and
      decide_facts(f, guards) / decide_all(f), so v2, v2+single, v2+cover and v2+both come from the same facts.
      guards=frozenset() is v2 exactly (v2_decision, unchanged).
  (d) the rates files are unchanged (m551/rates by default) but the directory is a parameter (set_rates_dir(),
      or the RULE_G_RATES_DIR environment variable at import), and decide() records their sha256.

    python3 -I rule_v2.py ID [ID ...]        decide ids (reads/fills the cache; one GET per 1.5 s)
    python3 -I rule_v2.py --port-check       the M1 asserts on the 279 labelled ids (writes M1-port-check.json)

API (for the walk and the port check):
    facts(tender_id, refresh=False) -> dict
                                  every fact v1 and v2 read, JSON-serialisable; cache under SCRATCH/cache (a walk
                                  passes refresh=True so each walked Tender is read fresh; notices are cached by id)
    v1_fires(f) -> bool           path W v1 (unit 3's merge_rule.serve, T' K G0 G5 G5' G6 G7 G8 G8b G1' G2 G3 G4)
    v2_decision(f) -> (ship, first_refusing_check, S, currency, n)

v2 = v1 PLUS refusals only: v2 ships only where v1 fires and none of R1..R10 refuses (so every v2 fire is a v1
fire; nothing in v1 is relaxed, nothing is re-keyed, G8b is unchanged). Decided choices (task, M1): R10 from
EUR 10 m (amendment (a)); the published lot number NEVER takes TED-ITEM; G8b unchanged.

SOURCES. The rule: issue 551 "## Unit 4b -- decided" (the judge's R1..R10) with its ADOPTED AMENDMENTS overriding.
v1: `../unit3/merge_rule.py` (+ `../scripts/elect.py`), imported and sha256-pinned below, fed the head version
rebuilt from /v1/tenders/{id}. v2's facts: adapted from the unit-4b evidence (evidence.tar.gz): score.py's facts()
(R1 Bm_g, R2 C_plus, R3 D_keys, R4 E, R6 C_title, R7 P10, R8 DOM10), score_work/pxscripts/extract.py (the per-notice
facts and published lot numbers), shapes_work/features.py (R5's 15 shape checks), judge/common.py (R9 no_own_bid,
R10 vouch_hits), judge/j7.py + adv/a2.py (the judge's v2 and the amended strict table).

=====================================================================================================
FROZEN FIELD-TO-KIND TABLE (amendment "Freeze one explicit field-to-kind table before the walk"; B1 must carry the
same; any edit to this table or to this file after the first walked id voids the draw)
-----------------------------------------------------------------------------------------------------
Stated totals are read from EVERY version notice of the Tender (detail `versions[].caused_by_notice_id`, plus any
notice a lot result / bid / contract names), from /v1/notices/{id}/content, never a bounded subset.

  field (notice content)                section kind          form                kind for R3         R10 vouch
  BT-161-NoticeResult                   any                   any                 award total         yes
  BT-118-NoticeResult                   any                   any                 award total (read only, never elected; 506) yes
  BT-1118-NoticeResult                  any                   any                 award total (read only)  yes
  TED-VAL_TOTAL                         Notice                CAN-type (not CN)   award total         yes
  TED-VAL_TOTAL                         Notice                CN-type             estimate            yes
  BT-27-Procedure                       any                   any                 estimate            yes
  TED-VAL_ESTIMATED_TOTAL               Notice                any                 estimate            yes
  TED-LOW / TED-HIGH (VAL_RANGE_TOTAL)  Notice                any                 -- (range)          yes
  TED-VALUE, TED-VALUE_COST             --                    --                  NOT READ            no
  BT-271-Procedure                      --                    --                  NOT READ            no
  TED-LOW_VALUE / TED-HIGH_VALUE /
  TED-TOTAL_ESTIMATED.* (estimate ranges) --                  --                  NOT READ            no
  any field in a Modification section (VAL_TOTAL_BEFORE/AFTER, DESCRIPTION_PROCUREMENT.VAL_TOTAL)  NOT READ  no
  CN-type TED forms (old vintage numeric and F-codes alike, the form number with any 'F' and leading zeros
  dropped): 1 2 4 5 7 8 9 10 11 12 16 17 24 (PIN / contract notice / qualification / buyer profile / concession /
  design contest notice). Every other form, and a notice with no form, is CAN-type.
  "A single-lot notice's total equal to its own award does not count": a total whose notice values exactly one
  lot result (API lot_results with a lot and a figure) and is within 1 % of that award is dropped (R3 and R10).
  Correction semantics (as rounds, project.rs): a notice with a `Change` section and a logical notice id
  (BT-701-notice / DE1-ID) that states >= 1 total REPLACES the totals of every earlier version notice with the
  same logical id; otherwise totals accumulate over the chain.
R3 estimates (the 10x branch): the estimate-kind totals above, the head's lot-null API `estimated_value` /
  `framework_maximum` amounts, and each lot's API-keyed `estimated_value` (its max), all in S's currency.
R4 / R5 estimates: API-keyed lot `estimated_value` only (no notice-text TED-VAL_OBJECT / BT-27-Lot fallback).
Published lot number of a lot in a notice (R2): its lot section's TED-LOT_NO / TED-LOT_NUMBER / BT-137-Lot /
  BT-137-Part / BT-137-LotsGroup / DE1-ProcurementProjectLot-ID, else the award block's TED-LOT_NO /
  TED-LOT_NUMBER, else the result's BT-13713-LotResult / DE1-NoticeResult-LotResult-TenderLot-ID, else (FTS) the
  OCDS lot id (the key); else None. NEVER TED-ITEM (the r209 ordinal attribute) and never a title.
  A summed award's number is read from its OWN award notice only.
R9: only bids with a resolved lot key count.
=====================================================================================================

CHECK ORDER (the first that refuses is reported):
  v1:  V (path V fires) | W/T' | W/K | W/G0 | W/G5 | W/G5' | W/G6 | W/G7 | W/G8 | W/G8b | W/G1' | W/G2 | W/G3 | W/G4
       | W/G0-served (M1: G0 against TODAY'S SERVED head -- an id whose served head is not the port's max election
         fails closed; 0 of the 279)
  v2:  R1 | R2a R2b R2c R2d | R3 | R4 | R5.1 .. R5.15 | R6 | R7 | R8 | R9 | R10
  R1  a summed lot's result names >= 2 winners that are not all tenderers of ONE LotTender of that lot in that
      round (consortium on one LotTender = one tender; TED AWARDED_TO_GROUP = one tender; no LotTender rows and no
      group flag: refuse; a winner with no organization id cannot be matched: refuse).
  R2  published lot numbers as token sets (leading zeros dropped, ranges <= 50 expanded, lists split):
      a two summed keys share a token; b one summed award's number is a list of >= 2; c one key's award rounds
      name different numbers; d the summed awards come from >= 2 notices and one has no number.
  R3  a notice's award total (S's currency) below 1/1.01 of the summed awards of the keys it awards; with no
      award total anywhere: S > 10 x the largest estimate.
  R4  a summed award >= 10 x its own lot's estimate.
  R5  the shapes pass's 15 checks, numbered as the rule lists them.
  R6  two summed keys awarded in different notices with the same non-empty winner set and lot title.
  R7  a summed award >= 10 x or <= 1/10 of an earlier valued award on the same key (same currency).
  R8  the largest summed award >= 10 x the second.
  R9  the summed result's round publishes LotTenders, none for the summed lot.
  R10 S_eur >= EUR 10,000,000.00 and no stated total T vouches (R10 table above; |T - S| <= 1 % of S; T equals
      no single summed award).

EUR: RatesLookup at the head's published date, rebuilt from the same public series the production
`currency_rates` table is loaded from (ECB eurofxref-hist 1999->, Eurostat ert_h_eur_d / ert_bil_eur_d before
1999, the irrevocable euro rates; nearest row at or before the date, 7-day window for daily rows), files under
SCRATCH/rates (sha256 in M1-port-check.md). /v1/sql is not used.

KNOWN SIMPLIFICATION (v1): the API serves the head version only, so ScalePartners is built from the head alone
(elect.py SIMPLIFICATION 4). Every scale rule is gated at EUR 1 bn, so this changes nothing below it; facts()
flags an id with any figure >= EUR 1 bn (`flags.scale_rules_head_only`).
"""
import collections
import gzip
import hashlib
import importlib.util
import io
import itertools
import json
import os
import re
import ssl
import sys
import time
import urllib.error
import urllib.request
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
API = 'https://tenders.zebreus.click'
SCRATCH = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551'
CACHE = SCRATCH + '/cache'
RATES_DIR = os.environ.get('RULE_G_RATES_DIR') or SCRATCH + '/rates'      # B6 (d): a parameter, see set_rates_dir()
B3_RAW = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b3/raw'
B3_BUNDLE = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b3/bundle/cache'
# B6 (a): the fresh tender layer (tenders ONLY from here) and the notice layers, first hit wins. Each layer is
# (directory, filename pattern, gzip). The rb8 layers are 568 read-back notice contents, the same
# /v1/notices/{id}/content JSON ({notice_id, sections}) under another file name.
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
B6 = SP + '/b6'
B6_TENDERS = B6 + '/tenders'
B6_NOTICES = B6 + '/notices'
TENDER_LAYERS = [(B6_TENDERS, '{}.json.gz', True)]
NOTICE_LAYERS = [(B6_NOTICES, '{}.json.gz', True),
                 (B3_RAW + '/notices', '{}.json', False),
                 (CACHE + '/notices', '{}.json', False),
                 (B3_BUNDLE + '/notices', '{}.json', False),
                 (SP + '/b3/verify/raw/notices', '{}.json', False),
                 (SP + '/b5/raw/notices', '{}.json', False),
                 (SP + '/rb8/band/notices', 'notices_{}_content.json', False),
                 (SP + '/rb8/verify_band/notices', 'n{}.json', False),
                 (SP + '/rb8/sets/u7notices', 'notices_{}_content.json', False)]

R10_MIN_EUR_CENTS = 0                      # B3: R10 at every size (M2 fallback "vouched everywhere" PASSED)

# ---------------------------------------------------------------- v1: unit 3's port, pinned
PINNED = {
    os.path.join(HERE, '..', 'unit3', 'merge_rule.py'): '111115a7e6500d3af78eed18bc45abf6f2ee0272098222f06ffc05ba825ba899',
    os.path.join(HERE, '..', 'scripts', 'elect.py'): 'b270740e3c8e14b9a44660d7ca959d67b3ef6f8fe77c55974a29b715fe1ece63',
}
for _p, _h in PINNED.items():
    _got = hashlib.sha256(open(_p, 'rb').read()).hexdigest()
    if _got != _h:
        raise SystemExit(f'rule_v2: {_p} changed (sha256 {_got}, pinned {_h}); v1 is no longer the ported rule')
_CP = os.path.join(HERE, '..', '..', '568-r208-lots', 'readback-u8', 'combined_port.py')
_RP = os.path.join(HERE, '..', '..', '570-lot-keys', 'remap_port.py')
for _p, _h in ((_CP, '5363593c663c651bcbf125464ccfd24b96d45b56e1834086352f7d8903d7673c'),
               (_RP, 'abe2b78ae5a0506dd327a412f9753c77a8d2a448fba080d4d73c0aae083bd503')):
    _got = hashlib.sha256(open(_p, 'rb').read()).hexdigest()
    if _got != _h:
        raise SystemExit(f'rule_v2_b3: {_p} changed (sha256 {_got}, pinned {_h})')
_cspec = importlib.util.spec_from_file_location('combined_port_570', _CP)
CP = importlib.util.module_from_spec(_cspec)
_cspec.loader.exec_module(CP)
# B3: the fold's `legacy` is `is_legacy_profile(profile)`; the content endpoint carries no profile, and the FTS parser
# writes `TED-VAL_TOTAL_TAX_BASIS` on OCDS notices (crates/ingest/src/fts/parse.rs), which combined_port's TED-/TXT-
# test reads as legacy. An FTS notice is never legacy (`fts:*`), so that one field does not count here.
CP.is_legacy = lambda content: any(v['field_id'].startswith(('TED-', 'TXT-')) and v['field_id'] != 'TED-VAL_TOTAL_TAX_BASIS'
                                   for s in content.get('sections', []) for v in s.get('values', []))


def remap_inverse(x, contents):
    """Issue 570 (B3): {notice: {served Tender key: the notice's own lot section id}} over the version chain in fold
    order (seq), via combined_port.remap_chain (remap_port.remap + K3 + the carry). A notice with no lot section maps
    nothing; an unremapped one maps each own id to itself."""
    chain = [v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0)
             if v.get('caused_by_notice_id')]
    maps = CP.remap_chain(list(dict.fromkeys(chain)), contents)
    inv = {}
    for nid, c in contents.items():
        # identity over every lot section the port reads (LOT_KINDS), then the 570 moves (K1: injective)
        target = {s['section_id']: s['section_id'] for s in c.get('sections', []) if s.get('kind') in LOT_KINDS}
        target.update((maps.get(nid) or ({}, None))[0])
        inv[nid] = {t: own for own, t in target.items()}
        assert len(inv[nid]) == len(target), (nid, target)
    return inv, {nid: how for nid, (target, how) in maps.items()}


_spec = importlib.util.spec_from_file_location('merge_rule_551', os.path.join(HERE, '..', 'unit3', 'merge_rule.py'))
MR = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(MR)
E = MR.E


# ---------------------------------------------------------------- fetch layer (public API, read-only)
class FetchError(Exception):
    pass


def _layers(kind):
    return TENDER_LAYERS if kind == 'tenders' else NOTICE_LAYERS


def find_cached(kind, key):
    """B6: the path of the first layer holding `key` (kind 'tenders' or 'notices'), or None."""
    for base, pat, _gz in _layers(kind):
        p = os.path.join(base, pat.format(int(key)))
        if os.path.exists(p):
            return p
    return None


def _load(p):
    if p.endswith('.gz'):
        with gzip.open(p, 'rt', encoding='utf-8') as fh:
            return json.load(fh)
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


def _cached(kind, key, path, refresh=False):
    # B6 (a): offline. Tenders: the fresh b6 layer only. Notices: b6, then the older layers (immutable contents).
    p = find_cached(kind, key)
    if p is None:
        raise FetchError(path, 'not cached (the B6 rule never fetches; run b6/fetch_b6.py)')
    x = _load(p)
    got = x.get('id') if kind == 'tenders' else x.get('notice_id')
    if got is not None and int(got) != int(key):
        raise FetchError(path, f'{p} holds {kind} {got}, not {key}')
    return x


def tender_detail(tid, refresh=False):
    return _cached('tenders', int(tid), f'/v1/tenders/{int(tid)}', refresh)


def notice_content(nid, refresh=False):
    return _cached('notices', int(nid), f'/v1/notices/{int(nid)}/content', refresh)


# ---------------------------------------------------------------- EUR: RatesLookup at the head's date
IRREVOCABLE = [('ATS', '1999-01-01', 13.7603), ('BEF', '1999-01-01', 40.3399), ('DEM', '1999-01-01', 1.95583),
               ('ESP', '1999-01-01', 166.386), ('FIM', '1999-01-01', 5.94573), ('FRF', '1999-01-01', 6.55957),
               ('IEP', '1999-01-01', 0.787564), ('ITL', '1999-01-01', 1936.27), ('LUF', '1999-01-01', 40.3399),
               ('NLG', '1999-01-01', 2.20371), ('PTE', '1999-01-01', 200.482), ('GRD', '2001-01-01', 340.750),
               ('SIT', '2007-01-01', 239.640), ('CYP', '2008-01-01', 0.585274), ('MTL', '2008-01-01', 0.429300),
               ('SKK', '2009-01-01', 30.1260), ('EEK', '2011-01-01', 15.6466), ('LVL', '2014-01-01', 0.702804),
               ('LTL', '2015-01-01', 3.45280), ('HRK', '2023-01-01', 7.53450), ('BGN', '2026-01-01', 1.95583)]
LEGACY = {'ECU': 'EUR', 'XEU': 'EUR', 'LIT': 'ITL', 'UKL': 'GBP', 'DKR': 'DKK', 'HFL': 'NLG', 'NFL': 'NLG',
          'SKR': 'SEK', 'NKR': 'NOK', 'BFR': 'BEF', 'PTA': 'ESP', 'LFR': 'LUF', 'FMK': 'FIM', 'ESC': 'PTE',
          'FFR': 'FRF', 'IKR': 'ISK', 'SFR': 'CHF', 'YEN': 'JPY', 'IRL': 'IEP', 'CND': 'CAD'}
_RATES = None


def _day(d):
    y, m, dd = (int(p) for p in d.split('-'))
    y -= m <= 2
    era = (y if y >= 0 else y - 399) // 400
    yoe = y - era * 400
    doy = (153 * (m + (-3 if m > 2 else 9)) + 2) // 5 + dd - 1
    doe = yoe * 365 + yoe // 4 - yoe // 100 + doy
    return era * 146097 + doe - 719468


def _load_rates():
    global _RATES
    if _RATES is not None:
        return _RATES
    rows = {}
    for ds in ('ert_h_eur_d', 'ert_bil_eur_d'):          # production loads h, then bil (REPLACE: the later wins)
        lines = open(os.path.join(RATES_DIR, f'ecu-{ds}.csv'), encoding='utf-8').read().splitlines()
        cols = [c.strip().lower() for c in lines[0].split(',')]
        ci, di, vi = cols.index('currency'), cols.index('time_period'), cols.index('obs_value')
        for line in lines[1:]:
            c = [x.strip() for x in line.split(',')]
            try:
                cur, day, v = c[ci], c[di], float(c[vi])
            except (IndexError, ValueError):
                continue
            if cur and day < '1999-01-01' and v > 0:
                rows[(cur, day)] = (v, False)
    z = zipfile.ZipFile(os.path.join(RATES_DIR, 'eurofxref-hist.zip'))
    csv = z.read(z.namelist()[0]).decode()
    lines = csv.splitlines()
    head = [c.strip() for c in lines[0].split(',')]
    for line in lines[1:]:
        c = [x.strip() for x in line.split(',')]
        for i in range(1, min(len(head), len(c))):
            if not head[i]:
                continue
            try:
                v = float(c[i])
            except ValueError:
                continue
            if v > 0:
                rows[(head[i], c[0])] = (v, False)
    for cur, day, v in IRREVOCABLE:
        rows[(cur, day)] = (v, True)
    by = collections.defaultdict(list)
    for (cur, day), (v, irr) in rows.items():
        by[cur].append((day, v, irr))
    for v in by.values():
        v.sort()
    _RATES = by
    return by


def head_date(published_at):
    """civil_date(head.published_at): the UTC date of the head's publication instant. The API prints a date-only
    publication as its UTC date and a timed one in its own offset (json.rs `published`)."""
    if not published_at:
        return None
    s = str(published_at)
    if len(s) == 10:
        return s
    import datetime
    dt = datetime.datetime.fromisoformat(s.replace('Z', '+00:00'))
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=datetime.timezone.utc)
    return dt.astimezone(datetime.timezone.utc).date().isoformat()


def rate(currency, date):
    """Units of `currency` per EUR on `date` ('YYYY-MM-DD'), or None (RatesLookup::rate)."""
    cur = LEGACY.get(currency, currency)
    if cur == 'EUR':
        return 1.0
    series = _load_rates().get(cur)
    if not series or not date:
        return None
    lo, hi = 0, len(series)
    while lo < hi:
        mid = (lo + hi) // 2
        if series[mid][0] <= date:
            lo = mid + 1
        else:
            hi = mid
    if lo == 0:
        return None
    d, v, irr = series[lo - 1]
    if irr or _day(date) - _day(d) <= 7:
        return v
    return None


def set_rates_dir(path):
    """B6 (d): read the rates files from `path` (ecu-ert_h_eur_d.csv, ecu-ert_bil_eur_d.csv, eurofxref-hist.zip)."""
    global RATES_DIR, _RATES, _RATES_SHA
    RATES_DIR, _RATES, _RATES_SHA = path, None, None


_RATES_SHA = None


def rates_sha256():
    """B6 (d): {'dir': RATES_DIR, 'files': {name: sha256}} of the rates files in use (computed once per dir)."""
    global _RATES_SHA
    if _RATES_SHA is None:
        _RATES_SHA = {'dir': RATES_DIR, 'files': rates_file_hashes()}
    return _RATES_SHA


def rates_file_hashes():
    return {f: hashlib.sha256(open(os.path.join(RATES_DIR, f), 'rb').read()).hexdigest()
            for f in sorted(os.listdir(RATES_DIR))}


# ---------------------------------------------------------------- helpers from the evidence
FLOOR = 1000


def score_sentinel(c):                       # score.py / pxscripts/common.py
    if c is None or c < 0:
        return True
    major = str(c // 100)
    return len(major) >= 7 and len(set(major)) == 1 and c % 100 == 0


def score_figure(c):
    return c is not None and c > FLOOR and not score_sentinel(c)


def sh_sentinel(c):                          # shapes_work/features.py
    if c <= 0 or c in (1, 100):
        return True
    major = str(c // 100)
    return len(major) >= 7 and len(set(major)) == 1 and c % 100 == 0


def near(a, b, tol=0.01):
    return a > 0 and b > 0 and abs(a - b) <= tol * max(a, b)


def lot_nums(p):
    """score.py: a published lot number as a set of tokens, or None."""
    if p is None:
        return None
    s = str(p).lower()
    out = set()
    for a, b in re.findall(r'(?<![\w])(\d+)\s*(?:-|–|to|bis|à|a)\s*(\d+)(?![\w])', s):
        a, b = int(a), int(b)
        if 0 < b - a <= 50:
            out |= {str(i) for i in range(a, b + 1)}
    for t in re.findall(r'\d+[a-z]?(?![a-z])', s):
        m = re.match(r'(\d+)([a-z]?)', t)
        out.add((m.group(1).lstrip('0') or '0') + m.group(2))
    return out or None


def ckey(ids, names):
    """score.py: a winner set, by organization id, else by normalised name."""
    out = set()
    ids = ids or []
    names = names or []
    for i, oid in enumerate(ids):
        if oid is not None:
            out.add(f'id:{oid}')
        elif i < len(names) and names[i]:
            out.add('nm:' + re.sub(r'\W+', ' ', names[i].lower()).strip())
    if not ids:
        for n in names:
            if n:
                out.add('nm:' + re.sub(r'\W+', ' ', n.lower()).strip())
    return frozenset(out)


def norm_title(t):
    return re.sub(r'\W+', ' ', (t or '').lower()).strip()


# ---------------------------------------------------------------- the frozen table, in code
AWARD_TOTAL_ANY_SECTION = ('BT-161-NoticeResult', 'BT-118-NoticeResult', 'BT-1118-NoticeResult')
CN_FORM_NUMBERS = {1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 16, 17, 24}
LOT_NO_FIELDS = ('TED-LOT_NO', 'TED-LOT_NUMBER', 'BT-137-Lot', 'BT-137-Part', 'BT-137-LotsGroup',
                 'DE1-ProcurementProjectLot-ID')
RESULT_LOT_NO_FIELDS = ('TED-LOT_NO', 'TED-LOT_NUMBER')
RESULT_LOT_REF_FIELDS = ('BT-13713-LotResult', 'DE1-NoticeResult-LotResult-TenderLot-ID')
LOT_KINDS = ('Lot', 'Part', 'LotsGroup', 'ProcurementProjectLot')


def cn_type(form):
    if form is None:
        return False
    m = re.fullmatch(r'[Ff]?0*(\d+)', str(form).strip())
    return bool(m) and int(m.group(1)) in CN_FORM_NUMBERS


def stated_kind(field, section_kind, form):
    """-> (r3_kind: 'award' | 'estimate' | None, vouches: bool) -- the frozen table."""
    if field in AWARD_TOTAL_ANY_SECTION:
        return 'award', True
    if field == 'BT-27-Procedure':
        return 'estimate', True
    if section_kind != 'Notice':
        return None, False
    if field == 'TED-VAL_TOTAL':
        return ('estimate' if cn_type(form) else 'award'), True
    if field == 'TED-VAL_ESTIMATED_TOTAL':
        return 'estimate', True
    if field in ('TED-LOW', 'TED-HIGH'):
        return None, True
    return None, False


def _val(v):
    return v['value'] if 'value' in v else v.get('code')


def notice_facts(n):
    """Per notice (adapted from pxscripts/extract.py notice_facts): form, logical id, correction flag, the stated
    totals per the frozen table, per-lot-section and per-result facts."""
    out = {'form': None, 'ted_form': None, 'logical_id': None, 'is_correction': False, 'totals': [], 'lots': {},
           'results': {}}
    for s in n.get('sections', []):
        if s.get('kind') == 'Change':
            out['is_correction'] = True
        fids = {}
        for v in s.get('values', []):
            fids.setdefault(v['field_id'], []).append(v)
        for f in ('TED-FORM', 'BT-02-notice', 'OPP-070-notice', 'DE1-NoticeSubType-SubTypeCode'):
            if f in fids and out['form'] is None:
                out['form'] = _val(fids[f][0])
        if 'TED-FORM' in fids and out['ted_form'] is None:
            out['ted_form'] = _val(fids['TED-FORM'][0])
        for f in ('BT-701-notice', 'DE1-ID'):
            if f in fids and out['logical_id'] is None:
                out['logical_id'] = _val(fids[f][0])
        for f, vs in fids.items():
            for v in vs:
                if v.get('type') != 'amount':
                    continue
                out['totals'].append({'field': f, 'section_kind': s.get('kind'), 'cents': v.get('cents'),
                                      'currency': v.get('currency'), 'section': s.get('section_id')})
        if s.get('kind') in LOT_KINDS:
            L = {'kind': s['kind']}
            for f in LOT_NO_FIELDS:
                if f in fids and 'lot_no' not in L:
                    L['lot_no'] = _val(fids[f][0])
            for f in ('TED-ITEM',):
                if f in fids:
                    L['item'] = _val(fids[f][0])          # recorded, NEVER a lot number
            out['lots'][s['section_id']] = L
        if s.get('kind') == 'LotResult':
            R = {}
            for f in RESULT_LOT_NO_FIELDS:
                if f in fids and 'lot_no' not in R:
                    R['lot_no'] = _val(fids[f][0])
            for f in RESULT_LOT_REF_FIELDS:
                if f in fids and 'lot_ref' not in R:
                    R['lot_ref'] = _val(fids[f][0])
            if 'TED-ITEM' in fids:
                R['item'] = _val(fids['TED-ITEM'][0])
            if 'TED-AWARDED_TO_GROUP' in fids:
                R['group'] = _val(fids['TED-AWARDED_TO_GROUP'][0])
            if 'TED-NO_AWARDED_TO_GROUP' in fids:
                R['no_group'] = _val(fids['TED-NO_AWARDED_TO_GROUP'][0])
            out['results'][s['section_id']] = R
    form = out['ted_form']           # the CN/CAN test reads the TED form only (TED-VAL_TOTAL is a TED field)
    tot = []
    for t in out['totals']:
        k, vouch = stated_kind(t['field'], t['section_kind'], form)
        if k is None and not vouch:
            continue
        tot.append(dict(t, r3_kind=k, vouches=vouch, form=form))
    out['totals'] = tot
    return out


# ---------------------------------------------------------------- v1 entry from the API detail
def _entry(x):
    """unit 3's stored-row entry (merge_rule.serve's input), rebuilt from /v1/tenders/{id} (the head version)."""
    lot_ids, keys, lots_rows, flags = {}, {}, [], []
    for l in x.get('lot_details', []):
        lid = int(l['id'])
        lot_ids[l['lot_key']] = lid
        keys[lid] = l['lot_key']
        lots_rows.append({'lot_id': lid, 'kind': l['kind'], 'lot_key': l['lot_key']})
    syn = [0]

    def lid_of(key, typed):
        if key is None:
            return None
        if key not in lot_ids:
            syn[0] -= 1
            lot_ids[key] = syn[0]
            keys[syn[0]] = key
            if typed:              # an amount on a lot the detail does not list: carry it as a typed Lot
                lots_rows.append({'lot_id': syn[0], 'kind': 'Lot', 'lot_key': key})
                flags.append(f'amount_on_unlisted_lot:{key}')
        return lot_ids[key]

    amounts = []
    for a in x.get('amounts', []):
        v = a.get('value')
        q = a.get('quality')
        if v is None and q is None:
            continue
        amounts.append({'lot_id': lid_of(a.get('lot'), True), 'field': a['field'],
                        'cents': -1 if v is None else int(v['cents']), 'currency': (v or {}).get('currency') or 'EUR',
                        'quality': q, 'tax_basis': None, 'eur_cents': None})
    results = []
    for i, lr in enumerate(x.get('lot_results', [])):
        aw = lr.get('awarded') or {}
        results.append({'lot_result_id': i + 1, 'lot_id': lid_of(lr.get('lot'), False),
                        'awarded_cents': aw.get('cents'), 'awarded_currency': aw.get('currency'),
                        'notice_id': lr['notice_id'], 'result_key': lr.get('key')})
    return {'keys': keys, 'chain': [], 'head': {'amounts': amounts, 'lots': lots_rows, 'results': results}}, flags


def _currencies(x):
    s = set()
    for a in x.get('amounts', []):
        if a.get('value'):
            s.add(a['value']['currency'])
    for lr in x.get('lot_results', []):
        if (lr.get('awarded') or {}).get('currency'):
            s.add(lr['awarded']['currency'])
    for b in x.get('bids', []):
        if (b.get('value') or {}).get('currency'):
            s.add(b['value']['currency'])
    return s


def v1_eval(x):
    entry, flags = _entry(x)
    date = head_date(x.get('published_at'))
    rates = {}
    for c in _currencies(x):
        r = rate(c, date)
        if c != 'EUR' and r is not None:
            rates[c] = r
    out = MR.serve(entry, rates)
    versions, head, partners = MR.build(entry)

    def conv(f):
        return MR.to_eur(rates, f.currency, f.cents) if (f.currency == 'EUR' or f.currency in rates) else None
    today = MR.elect_today(head, partners, conv)
    sv = x.get('value') or {}
    t_fact = None if today is None else {'cents': today[1].cents, 'currency': today[1].currency,
                                         'field': today[1].field, 'scope': list(today[2]), 'eur': today[0]}
    served_ok = ((today is None and not sv) or
                 (today is not None and sv.get('cents') == today[1].cents and sv.get('currency') == today[1].currency))
    # LotSums awards in the chosen currency: the summed set (v1's own)
    by = MR.lot_sums_by_currency(head)
    if today is not None and today[2] == E.LOT(None):
        cur = today[1].currency
    elif today is None:
        cands = [(len(ls.awards), c) for c, ls in by.items() if ls.awards]
        cur = max(cands)[1] if cands else None
    else:
        cur = today[1].currency
    per = {}
    if cur is not None and cur in by:
        per = {lot: nc for lot, nc in by[cur].awards.items()}
    why = out['why']
    if why == 'W' and not served_ok:
        # G0 is checked against TODAY'S SERVED head (M1): a port election that is not the served head fails closed
        why = "W/G0-served: the served head is not the port's max election"
    return {'why': why, 'fires': why == 'W', 'derived': out['derived'], 'eur': out['eur'],
            'today_eur': out['today'], 'today': t_fact, 'served': sv or None,
            'served_derivation': x.get('value_derivation'), 'served_matches_election': served_ok,
            'currency': cur, 'per_lot': {k: list(v) for k, v in per.items()}, 'rates': rates, 'date': date,
            'flags': flags}


# ---------------------------------------------------------------- facts()
def version_notice_ids(x):
    seq = [v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0)
           if v.get('caused_by_notice_id')]
    extra = set()
    for k in ('lot_results', 'bids', 'contracts'):
        extra |= {r['notice_id'] for r in x.get(k, []) if r.get('notice_id')}
    out = list(dict.fromkeys(seq))
    out += sorted(extra - set(out))
    return out


def facts(tender_id, refresh=False):
    x = tender_detail(tender_id, refresh=refresh)
    contents = {nid: notice_content(nid) for nid in version_notice_ids(x)}
    return facts_from(x, contents, v1_eval(x))


def facts_from(x, contents, v1):
    """B3: facts() on a given detail `x`, its notices' contents and a v1 evaluation (v1_eval(x), or b3/records.py's
    entry-based one for the 4,745 records)."""
    tid = int(x['id'])
    nids = version_notice_ids(x)
    NF = {}
    for nid in nids:
        NF[nid] = notice_facts(contents[nid])
    INV, HOW = remap_inverse(x, contents)
    f = {'id': tid, 'source': x.get('source'), 'published_at': x.get('published_at'), 'version': x.get('version'),
         'notices': nids, 'v1': v1, 'flags': list(v1['flags']),
         'remap': {str(n): h for n, h in HOW.items() if h == 'remapped' or h.startswith('refused') or h in ('K3', 'missing')}}
    cur = v1['currency']
    per = v1['per_lot']
    lots_meta = {l['lot_key']: l for l in x.get('lot_details', [])}
    results_by_lot = collections.defaultdict(list)
    for i, lr in enumerate(x.get('lot_results', [])):
        results_by_lot[lr.get('lot')].append((i, lr))
    # the summed lot results: the API row v1's LotSums kept (its latest by notice id, the later offer on a tie)
    summed = []
    for lot, (nid, cents) in per.items():
        rows = [lr for i, lr in results_by_lot.get(lot, []) if lr['notice_id'] == nid
                and (lr.get('awarded') or {}).get('cents') == cents and (lr.get('awarded') or {}).get('currency') == cur]
        lr = rows[-1]
        summed.append({'key': lot, 'notice_id': nid, 'result_key': lr.get('key'), 'cents': cents,
                       'decision': lr.get('decision'),
                       'contractor_ids': [w.get('organization_id') for w in lr.get('winners', [])],
                       'contractor_names': [w.get('organization_name') for w in lr.get('winners', [])]})
    summed.sort(key=lambda s: s['key'])
    S = sum(s['cents'] for s in summed)
    n = len(summed)
    f.update(currency=cur, S=S if summed else None, n=n, summed=summed)
    s_eur = None
    if summed and cur is not None:
        r = 1.0 if cur == 'EUR' else v1['rates'].get(cur)
        s_eur = None if r is None else MR.to_eur(v1['rates'], cur, S)
    f['S_eur'] = s_eur
    # flags: scale rules read the head only; ids near the EUR thresholds
    try:
        mx_eur = 0
        for a in x.get('amounts', []):
            v = a.get('value')
            if v:
                r = rate(v['currency'], v1['date'])
                if r:
                    mx_eur = max(mx_eur, v['cents'] / r)
        for lr in x.get('lot_results', []):
            v = lr.get('awarded') or {}
            if v.get('cents') is not None and v.get('currency'):
                r = rate(v['currency'], v1['date'])
                if r:
                    mx_eur = max(mx_eur, v['cents'] / r)
        if mx_eur >= E.SCALE_ERROR_MIN_EUR_CENTS:
            f['flags'].append('scale_rules_head_only')
    except Exception:          # noqa: BLE001 - a flag, never a decision
        pass
    if s_eur is not None:
        for thr, name in ((R10_MIN_EUR_CENTS, 'eur10m'), (10_000_000_000, 'eur100m')):
            if abs(s_eur - thr) <= 0.02 * thr:
                f['flags'].append(f'within_2pct_of_{name}')
    if not v1['served_matches_election']:
        f['flags'].append('served_head_differs_from_port_election')
    if not summed:
        f['pred'] = {}
        return f

    # ---- per-lot facts (extract.py's lots, from the fresh detail and every version notice)
    def pub_no(nid, key, result_key):
        """The frozen table's published lot number of `key` in notice `nid` (never TED-ITEM, never a title)."""
        nf = NF.get(nid)
        if nf is None:
            return None, f'notice {nid} unread'
        own = INV.get(nid, {}).get(key)          # B3 / issue 570: the section whose remap target is `key`
        L = nf['lots'].get(own) if own is not None else None
        if L and L.get('lot_no') is not None:
            return L['lot_no'], f'lot section {own} of notice {nid}' + ('' if own == key else f' (570: remapped onto {key})')
        R = nf['results'].get(result_key) if result_key is not None else None
        if R and R.get('lot_no') is not None:
            return R['lot_no'], f'result {result_key} LOT_NO of notice {nid}'
        if R and R.get('lot_ref') is not None:
            return R['lot_ref'], f'result {result_key} BT-13713 of notice {nid}'
        if x.get('source') == 'fts':
            return key, f'OCDS lot id {key} (notice {nid})'
        return None, f'none in notice {nid}'

    amt_lot = collections.defaultdict(lambda: collections.defaultdict(list))
    for a in x.get('amounts', []):
        if a.get('lot') and a.get('value'):
            amt_lot[a['lot']][a['field']].append((a['value']['cents'], a['value']['currency']))
    lot_est = {}
    for key, flds in amt_lot.items():
        ests = [c for c, cu in flds.get('estimated_value', []) if cu == cur and score_figure(c)]
        if ests:
            lot_est[key] = max(ests)
    bids_by_lot = collections.defaultdict(list)
    for b in x.get('bids', []):
        v = b.get('value') or {}
        bids_by_lot[b.get('lot')].append({'notice_id': b.get('notice_id'), 'key': b.get('key'), 'cents': v.get('cents'),
                                          'currency': v.get('currency'),
                                          'tenderer_ids': [q.get('organization_id') for q in b.get('parties', [])
                                                           if q.get('role') == 'tenderer']})
    P = {}
    for s in summed:
        p, src = pub_no(s['notice_id'], s['key'], s['result_key'])
        s['published_lot_no'] = p
        s['published_lot_no_src'] = src
    latest = {s['key']: ckey(s['contractor_ids'], s['contractor_names']) for s in summed}

    # R1 (Bm_g, the rule's text: a group flag is one tender with or without LotTender rows)
    r1 = []
    for s in summed:
        if len(latest[s['key']]) < 2:
            continue
        ids = [i for i in s['contractor_ids'] if i is not None]
        R = (NF.get(s['notice_id']) or {}).get('results', {}).get(s['result_key']) or {}
        group = R.get('group') not in (None, 0, '0', False, 'false', 'NO') and 'no_group' not in R
        if group:
            continue
        bids = [b for b in bids_by_lot.get(s['key'], []) if b['notice_id'] == s['notice_id']]
        if not bids:
            r1.append((s['key'], 'no LotTender rows, no group flag'))
            continue
        if len(ids) < len(s['contractor_ids']) or not any(set(ids) <= set(b['tenderer_ids'] or []) for b in bids):
            r1.append((s['key'], 'winners not all tenderers of one LotTender'))
    P['R1'] = r1

    # R2 (C_plus)
    nums = {s['key']: lot_nums(s['published_lot_no']) for s in summed}
    r2a = [(a, b) for a, b in itertools.combinations(sorted(nums), 2) if nums[a] and nums[b] and nums[a] & nums[b]]
    r2b = [k for k, v in nums.items() if v and len(v) >= 2]
    r2c = []
    for s in summed:
        seen = set()
        for i, lr in results_by_lot.get(s['key'], []):
            aw = lr.get('awarded') or {}
            if aw.get('cents') is None and not lr.get('winners'):
                continue
            no, _ = pub_no(lr['notice_id'], s['key'], lr.get('key'))
            t = lot_nums(no)
            if t:
                seen.add(frozenset(t))
        if len(seen) > 1:
            r2c.append(s['key'])
    multi_notice = len({s['notice_id'] for s in summed}) >= 2
    r2d = [k for k, v in nums.items() if v is None] if multi_notice else []
    P['R2a'], P['R2b'], P['R2c'], P['R2d'] = r2a, r2b, r2c, r2d

    # stated totals over every version notice, the frozen table, the correction semantics
    totals = []
    for nid in nids:
        nf = NF[nid]
        mine = [dict(t, notice_id=nid, logical_id=nf['logical_id']) for t in nf['totals']]
        if nf['is_correction'] and nf['logical_id'] is not None and mine:
            totals = [t for t in totals if t['logical_id'] != nf['logical_id']]
        totals += mine

    def own_valued(nid):
        return [lr['awarded']['cents'] for lr in x.get('lot_results', []) if lr['notice_id'] == nid and lr.get('lot')
                and (lr.get('awarded') or {}).get('cents') is not None and score_figure(lr['awarded']['cents'])]
    for t in totals:
        own = own_valued(t['notice_id'])
        t['equals_its_single_lot_award'] = (len(own) == 1 and t['cents'] is not None
                                            and abs(t['cents'] - own[0]) * 100 <= max(own[0], 1))
    f['stated_totals'] = totals
    usable = [t for t in totals if t['currency'] == cur and score_figure(t['cents'])]

    # R3 (D_keys)
    T = [t for t in usable if t['r3_kind'] == 'award' and not t['equals_its_single_lot_award']]
    Est = [t['cents'] for t in usable if t['r3_kind'] == 'estimate']
    for a in x.get('amounts', []):
        v = a.get('value')
        if a.get('lot') is None and v and v['currency'] == cur and a['field'] in ('estimated_value', 'framework_maximum') \
                and score_figure(v['cents']):
            Est.append(v['cents'])
    Est += list(lot_est.values())
    byN = collections.defaultdict(list)
    for t in T:
        byN[t['notice_id']].append(t['cents'])
    r3 = []
    for nid, ts in byN.items():
        tN = max(ts)
        s_keys = sum(s['cents'] for s in summed if any(lr['notice_id'] == nid for i, lr in results_by_lot.get(s['key'], [])))
        if s_keys > tN * 1.01:
            r3.append(('total', nid, tN, s_keys))
    if not T and Est and S > 10 * max(Est):
        r3.append(('estimate', None, max(Est), S))
    P['R3'] = r3

    # R4 (E)
    P['R4'] = [s['key'] for s in summed if s['key'] in lot_est and s['cents'] >= 10 * lot_est[s['key']]]

    # R5: the shapes pass's 15 checks (shapes_work/features.py), on the API detail, in S's currency
    sh = shapes_checks(x, summed, cur, S)
    for i, k in enumerate(R5_FEATURES, 1):
        P[f'R5.{i}'] = bool(sh[k])
    f['shapes'] = sh

    # R6 (C_title)
    r6 = []
    for a, b in itertools.combinations(summed, 2):
        ta = norm_title((lots_meta.get(a['key']) or {}).get('title'))
        tb = norm_title((lots_meta.get(b['key']) or {}).get('title'))
        if a['notice_id'] != b['notice_id'] and latest[a['key']] and latest[a['key']] == latest[b['key']] and ta and ta == tb:
            r6.append((a['key'], b['key']))
    P['R6'] = r6

    # R7 (P10)
    r7 = []
    for s in summed:
        for i, lr in results_by_lot.get(s['key'], []):
            aw = lr.get('awarded') or {}
            c = aw.get('cents')
            if lr['notice_id'] < s['notice_id'] and aw.get('currency') == cur and score_figure(c):
                if s['cents'] >= 10 * c or s['cents'] * 10 <= c:
                    r7.append((s['key'], lr['notice_id'], c))
    P['R7'] = r7

    # R8 (DOM10)
    vals = sorted((s['cents'] for s in summed), reverse=True)
    P['R8'] = len(vals) >= 2 and vals[0] >= 10 * vals[1]

    # R9 (no_own_bid): only bids with a resolved lot key
    nb = collections.defaultdict(set)
    for b in x.get('bids', []):
        if b.get('lot'):
            nb[b.get('notice_id')].add(b['lot'])
    P['R9'] = [s['key'] for s in summed if nb.get(s['notice_id']) and s['key'] not in nb[s['notice_id']]]

    # R10 (the band vouch, from EUR 10 m)
    singles = {s['cents'] for s in summed}
    vouch = [t for t in usable if t['vouches'] and not t['equals_its_single_lot_award'] and t['cents'] not in singles
             and abs(t['cents'] - S) <= 0.01 * S]
    f['vouched_by'] = ({'field': vouch[0]['field'], 'notice_id': vouch[0]['notice_id'], 'cents': vouch[0]['cents']}
                       if vouch else None)
    f['vouches'] = [(t['field'], t['notice_id'], t['cents']) for t in vouch]
    # B6 (b): the facts G-single and G-cover read, per R10 candidate (in the v2 candidate order).
    #   round_results: the served lot_results rows of the candidate's notice, keyed or not (G-single: == 1)
    #   names_summed_keys: the summed keys for which R3's test holds -- results_by_lot[key] holds a row of that
    #                      notice (keyless results excluded: a None or '' key is never a summed key here);
    #                      G-cover removes an award or range candidate when this list is empty
    summed_keys = [s['key'] for s in summed if s['key'] not in (None, '')]
    round_n = collections.Counter(lr.get('notice_id') for lr in x.get('lot_results', []))
    cands = []
    for t in vouch:
        nid = t['notice_id']
        kind = 'range' if t['field'] in ('TED-LOW', 'TED-HIGH') else \
            ('award' if t['r3_kind'] == 'award' else ('estimate' if t['r3_kind'] == 'estimate' else None))
        names = [k for k in summed_keys if any(lr['notice_id'] == nid for i, lr in results_by_lot.get(k, []))]
        c = {'field': t['field'], 'notice_id': nid, 'cents': t['cents'], 'kind': kind, 'form': t.get('form'),
             'round_results': round_n.get(nid, 0), 'content_lot_results': len(NF[nid]['results']),
             'names_summed_keys': names}
        c['g_single'] = c['round_results'] == 1
        c['g_cover'] = kind in ('award', 'range') and not names
        cands.append(c)
    f['vouch_candidates'] = cands
    P['R10'] = s_eur is not None and s_eur >= R10_MIN_EUR_CENTS and not vouch
    f['pred'] = P
    f['bids_by_summed_notice'] = {s['key']: len([b for b in bids_by_lot.get(s['key'], []) if b['notice_id'] == s['notice_id']])
                                  for s in summed}
    return f


R5_FEATURES = ('clos_nw', 'later_closed', 'award_eq_est_total', 'award_eq_whole', 'award_gt_bid_sum', 'gt2_bt709',
               'bt709_sum_lt_S', 'ge15_est_min', 'ge3_est', 'est_sum_lt_S3', 'equal_bids_eq_ceiling',
               'award_n_times_ref', 'tiny', 'proc_est_eq_each_lot_est', 'lot_est_all_equal')


def shapes_checks(x, summed, cur, S):
    """shapes_work/features.py's 15 T0 features, on v1's summed set and currency."""
    results_by_lot = collections.defaultdict(list)
    for lr in x.get('lot_results', []):
        if lr.get('lot'):
            results_by_lot[lr['lot']].append(lr)
    amt = collections.defaultdict(lambda: collections.defaultdict(list))
    lotnull = []
    for a in x.get('amounts', []):
        v = a.get('value')
        if not v or v['currency'] != cur:
            continue
        if a.get('lot') is None:
            lotnull.append((a['field'], v['cents']))
        else:
            amt[a['field']][a['lot']].append(v['cents'])
    est = {k: max(v) for k, v in amt['estimated_value'].items()}
    est_min = {k: min(v) for k, v in amt['estimated_value'].items()}
    bt709 = {k: max(v) for k, v in amt['result_framework_maximum'].items()}
    bt660 = {k: max(v) for k, v in amt['result_framework_reestimate'].items()}
    bt271 = {k: max(v) for k, v in amt['framework_maximum'].items()}
    bids = collections.defaultdict(list)
    for b in x.get('bids', []):
        v = b.get('value')
        if v and v.get('cents') and v['currency'] == cur:
            tenderers = tuple(sorted(str(p.get('organization_id') or p.get('mention_name')) for p in b.get('parties', [])
                                     if p.get('role') == 'tenderer'))
            bids[(b['notice_id'], b['lot'])].append((v['cents'], tenderers))
    per = []
    for s in summed:
        lot, c = s['key'], s['cents']
        lr = next(r for r in results_by_lot[lot] if r['notice_id'] == s['notice_id'] and r.get('key') == s['result_key'])
        wins = {w.get('organization_id') or w.get('organization_name') for w in lr.get('winners', [])}
        bvals = [b[0] for b in bids.get((s['notice_id'], lot), [])]
        later = [o for o in results_by_lot[lot] if o['notice_id'] > s['notice_id']]
        per.append(dict(lot=lot, cents=c, notice=s['notice_id'], decision=lr.get('decision'), n_winners=len(wins),
                        bvals=bvals, award_gt_bid_sum=bool(bvals) and c > sum(bvals) * 1.01,
                        est=est.get(lot), est_min=est_min.get(lot), bt709=bt709.get(lot), bt660=bt660.get(lot),
                        bt271=bt271.get(lot),
                        later_closed=any((o.get('decision') or '').startswith('clos') or
                                         (not o.get('winners') and not (o.get('awarded') or {}).get('cents')) for o in later)))
    g = {}
    g['clos_nw'] = any((p['decision'] or '').startswith('clos') or p['n_winners'] == 0 for p in per)
    g['later_closed'] = any(p['later_closed'] for p in per)
    g['award_gt_bid_sum'] = any(p['award_gt_bid_sum'] for p in per)
    g['gt2_bt709'] = any(p['bt709'] and p['cents'] >= 1.9 * p['bt709'] for p in per)
    g['ge3_est'] = any(p['est'] and p['cents'] >= 3 * p['est'] for p in per)
    est_all = list(est.values())
    g['award_eq_est_total'] = len(est) >= 2 and any(near(p['cents'], sum(est_all), 0.005) for p in per)
    figure_sets = []
    for fld in ('estimated_value', 'result_framework_maximum', 'result_framework_reestimate', 'framework_maximum'):
        d = {k: max(v) for k, v in amt[fld].items()}
        if len(d) >= 2:
            figure_sets.append(sum(d.values()))
    rounds = collections.defaultdict(dict)
    for lr in x.get('lot_results', []):
        a = lr.get('awarded')
        if lr.get('lot') and a and a.get('cents') and a['cents'] > FLOOR and a['currency'] == cur:
            rounds[lr['notice_id']][lr['lot']] = a['cents']
    for nid, d in rounds.items():
        if len(d) >= 2:
            figure_sets.append(sum(d.values()))
    g['award_eq_whole'] = any(near(p['cents'], s, 0.005) for p in per for s in figure_sets)
    g['tiny'] = S < 100000
    g['bt709_sum_lt_S'] = bool(bt709) and sum(bt709.get(p['lot'], 0) for p in per) > 0 and \
        S > 1.5 * sum(bt709.get(p['lot'], 0) for p in per)
    g['est_sum_lt_S3'] = sum(est.get(p['lot'], 0) for p in per) > 0 and \
        S > 3 * sum(est.get(p['lot'], 0) for p in per if est.get(p['lot']))
    g['ge15_est_min'] = any(p['est_min'] and p['cents'] >= 1.5 * p['est_min'] for p in per)
    pe = [c for fld, c in lotnull if fld == 'estimated_value']
    g['proc_est_eq_each_lot_est'] = len(est) >= 2 and any(all(near(v, c, 0.001) for v in est.values()) for c in pe)
    g['lot_est_all_equal'] = len(est) >= 2 and len(set(est.values())) == 1

    def ceil_copy(q):
        bl = q['bvals']
        dup = {v for v in bl if bl.count(v) >= 2}
        refs = [v for v in (q['bt709'], q['bt660'], q['est'], q['est_min'], q['bt271']) if v]
        return bool(dup) and any(near(d, r, 0.001) for d in dup for r in refs)
    g['equal_bids_eq_ceiling'] = any(ceil_copy(q) for q in per)
    g['award_n_times_ref'] = any(q['n_winners'] >= 2 and any(r and near(q['cents'], r * q['n_winners'], 0.001)
                                                             for r in (q['bt709'], q['bt660'], q['est'], q['bt271'])) for q in per)
    return g


# ---------------------------------------------------------------- decisions
V2_ORDER = ['R1', 'R2a', 'R2b', 'R2c', 'R2d', 'R3', 'R4'] + [f'R5.{i}' for i in range(1, 16)] + \
    ['R6', 'R7', 'R8', 'R9', 'R10']


def v1_fires(f):
    return bool(f['v1']['fires'])


def v1_reason(f):
    """merge_rule.serve's reason, reduced to its check name: 'V' (path V served, or a path-V refusal: 'V', 'V0',
    'V1'), else the path-W guard ('W/T'', 'W/K', 'W/G0', ...)."""
    w = f['v1']['why']
    if w == 'W':
        return None
    return w.split(':')[0]


def v2_refusals(f):
    P = f.get('pred') or {}
    return [k for k in V2_ORDER if P.get(k)]


def v2_decision(f):
    """-> (ship, first_refusing_check, S, currency, n). Ships only where v1 fires and no R refuses."""
    if not v1_fires(f):
        return False, v1_reason(f), f.get('S'), f.get('currency'), f.get('n')
    d = f['v1']['derived']
    assert d['cents'] == f['S'] and d['currency'] == f['currency'] and d['lots'] == f['n'], (f['id'], d, f['S'])
    ref = v2_refusals(f)
    return (not ref), (ref[0] if ref else None), f['S'], f['currency'], f['n']


# ---------------------------------------------------------------- B6 (b) + (c): the guards and the switch
GUARDS = ('single', 'cover')
GUARD_SETS = (frozenset(), frozenset({'single'}), frozenset({'cover'}), frozenset({'single', 'cover'}))


def guard_name(guards):
    g = frozenset(guards)
    return 'v2' if not g else 'v2+' + ('both' if g == frozenset(GUARDS) else next(iter(g)))


def _cand_key(c):
    return [c['field'], c['notice_id'], c['cents']]


def guarded_vouches(f, guards):
    """-> (kept candidates, g_single_removed, g_cover_removed). Each guard only removes R10 candidates; a
    candidate both remove is listed under both. With guards=frozenset() nothing is removed (v2)."""
    g = frozenset(guards)
    assert g <= set(GUARDS), g
    kept, rs, rc = [], [], []
    for c in f.get('vouch_candidates') or []:
        drop = False
        if 'single' in g and c['g_single']:
            rs.append(c)
            drop = True
        if 'cover' in g and c['g_cover']:
            rc.append(c)
            drop = True
        if not drop:
            kept.append(c)
    return kept, rs, rc


def decide_facts(f, guards=frozenset()):
    """v2 (guards empty) or v2 + the named guards, on facts `f`. -> dict with ship, first (the first refusing
    check; R10 when a guard empties R10's candidates), S, currency, n, vouched_by (the first kept candidate),
    vouches (every kept candidate, [field, notice_id, cents]), refusals and the guard diagnostics."""
    g = frozenset(guards)
    ship, first, S, cur, n = v2_decision(f)
    kept, rs, rc = guarded_vouches(f, g)
    refusals = v2_refusals(f)
    if g and v1_fires(f) and f.get('pred'):
        P = dict(f['pred'])
        s_eur = f.get('S_eur')
        P['R10'] = s_eur is not None and s_eur >= R10_MIN_EUR_CENTS and not kept
        refusals = [k for k in V2_ORDER if P.get(k)]
        ship, first = (not refusals), (refusals[0] if refusals else None)
    vb = ({'field': kept[0]['field'], 'notice_id': kept[0]['notice_id'], 'cents': kept[0]['cents']} if kept else None)
    if not g:
        assert vb == f.get('vouched_by') and [tuple(_cand_key(c)) for c in kept] == list(f.get('vouches') or []), f['id']
    removed = {tuple(_cand_key(c)): c for c in rs + rc}
    return {'guards': sorted(g), 'name': guard_name(g), 'ship': bool(ship), 'first': first, 'S': S, 'currency': cur,
            'n': n, 'vouched_by': vb, 'vouches': [_cand_key(c) for c in kept], 'refusals': refusals,
            'g_single_removed': [_cand_key(c) for c in rs] if 'single' in g else None,
            'g_cover_removed': [_cand_key(c) for c in rc] if 'cover' in g else None,
            'g_removed': [list(k) for k in removed]}


def decide_all(f):
    """{name: decide_facts(f, guards)} for v2, v2+single, v2+cover, v2+both, on the same facts."""
    return {guard_name(g): decide_facts(f, g) for g in GUARD_SETS}


def decide(tender_id, guards=frozenset()):
    """B6 (c): decide one Tender from the cached facts under `guards`. Records the rates files' sha256 and where
    the tender read came from (its b6 sidecar's fetch time when present)."""
    f = facts(tender_id)
    out = decide_facts(f, guards)
    out.update(id=int(tender_id), S_eur=f.get('S_eur'), v1_why=f['v1']['why'], flags=f['flags'],
               vouch_candidates=f.get('vouch_candidates'), tender_read=tender_read_meta(tender_id),
               rates=rates_sha256())
    return out


def tender_read_meta(tender_id):
    p = find_cached('tenders', tender_id)
    if p is None:
        return None
    meta = os.path.join(os.path.dirname(p), f'{int(tender_id)}.meta.json')
    m = _load(meta) if os.path.exists(meta) else {}
    return {'path': p, 'fetched_at': m.get('fetched_at')}


# ---------------------------------------------------------------- the M1 port check
EV = SCRATCH + '/ev'
RR = os.path.join(HERE, '..', 'postdrain', 'read_result.json')
JUDGE_BAND15 = {683951, 707424, 726354, 1102571, 1147190, 1148297, 5842187, 6017838, 6057925, 6195903, 6370901,
                7001924, 7011414, 8426318, 5278856}
OOS_EXPECT_SHIP = {294968, 467176, 585917, 5995030, 8426318}
OOS_IDS = [585917, 5995030, 467176, 294968, 7256284, 5977060]


def _judge_in_sample():
    """The judge's v2 (judge/j7.py) and the amended strict table (adv/a2.py, all three modes), on the evidence's
    cached features, with the evidence copies' paths (not the session scratchpad the scripts name)."""
    spec = importlib.util.spec_from_file_location('score_551', EV + '/score.py')
    S = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(S)
    S.FEAT = EV + '/px/features_all.json'
    S.SHAPES = EV + '/shapes_work/features.json'
    lab, F, ids = S.load()
    rows = {r['id']: r for r in json.load(open(S.FEAT))}
    sh = json.load(open(S.SHAPES))
    K = {'result_total', 'estimate_total', 'framework_max_total', 'framework_reestimate_total', 'result_range_high',
         'result_range_low'}
    CH = ('Bm_g', 'C_plus', 'D_keys', 'E', 'T0', 'C_title', 'P10', 'DOM10')

    def no_own_bid(r):
        nb = collections.defaultdict(set)
        for l in r['lots']:
            for b in l.get('bids', []):
                nb[b['notice_id']].add(l['key'])
        return any(nb.get(s['notice_id']) and s['key'] not in nb[s['notice_id']] for s in r['summed_lots'])

    def vouch_hits(r, tol=0.01, kinds=None):
        Sv = r['sum_cents']; cur = r['summed_currency']; singles = {s['cents'] for s in r['summed_lots']}
        out = []
        for t in r.get('stated_totals_all_versions') or []:
            if kinds and t['kind'] not in kinds:
                continue
            if t.get('currency') != cur or not S.figure(t.get('cents')):
                continue
            if t.get('equals_its_single_lot_award') or t['cents'] in singles:
                continue
            if abs(t['cents'] - Sv) <= tol * Sv:
                out.append((t['kind'], t.get('field')))
        return out
    band = lambda i: lab[i]['group'].split('-')[0] in ('top', 'mid')

    def v2(f, r, b, kinds=K):
        if any(f[c] for c in CH) or no_own_bid(r):
            return True
        return b and not vouch_hits(r, kinds=kinds)
    import copy
    AWARD_FIELDS = {'BT-161-NoticeResult', 'TED-VAL_TOTAL', 'BT-118-NoticeResult', 'BT-1118-NoticeResult'}
    VOUCH_FIELDS = AWARD_FIELDS | {'BT-27-Procedure', 'TED-VAL_ESTIMATED_TOTAL', 'TED-LOW', 'TED-HIGH'}

    def strict_row(r):
        r = copy.deepcopy(r)
        out = []
        for t in r.get('stated_totals_all_versions') or []:
            fld = t['field']
            if fld in AWARD_FIELDS and t['kind'] in ('result_total', 'framework_max_total', 'framework_reestimate_total'):
                out.append(t)
            elif fld in VOUCH_FIELDS:
                t = dict(t)
                if t['kind'] in ('result_total', 'framework_max_total', 'framework_reestimate_total'):
                    t['kind'] = 'estimate_total'
                out.append(t)
        r['stated_totals_all_versions'] = out
        for s in r['summed_lots']:
            if s.get('published_lot_no') is not None:
                L = next(l for l in r['lots'] if l['key'] == s['key'])
                if 'title' in (L.get('published_lot_no_src') or ''):
                    s['published_lot_no'] = None
        for L in r['lots']:
            if L.get('estimate_src') and not L['estimate_src'].startswith('api keyed'):
                L['estimate_cents'] = None
            L['estimate_by_published_lot_no'] = None
        return r
    judge, strict, checks = {}, {}, {}
    for i in ids:
        judge[i] = not v2(F[i], rows[i], band(i))
        r2 = strict_row(rows[i])
        f2 = S.facts(r2, sh.get(str(i)))
        strict[i] = not v2(f2, r2, band(i))
        checks[i] = {'judge': [c for c in CH if F[i][c]] + (['R9'] if no_own_bid(rows[i]) else []),
                     'strict': [c for c in CH if f2[c]] + (['R9'] if no_own_bid(r2) else []),
                     'vouch_strict': sorted(set(vouch_hits(r2, kinds=K))), 'S': rows[i]['sum_cents'],
                     'cur': rows[i]['summed_currency'], 'n': rows[i]['n_summed'],
                     'pred_judge': {k: F[i][k] for k in ('Bm_g', 'C', 'C_list', 'C_vary', 'C_absent', 'multi_notice', 'D_keys',
                                                         'E', 'T0', 'C_title', 'P10', 'DOM10')},
                     'pred_strict': {k: f2[k] for k in ('Bm_g', 'C', 'C_list', 'C_vary', 'C_absent', 'multi_notice', 'D_keys',
                                                        'E', 'T0', 'C_title', 'P10', 'DOM10')}}
    return lab, judge, strict, checks


def _detail_same(t):
    """The fresh detail against the evidence's cached one, the head value and its derivation set aside (unit 4a
    refolded every path-W head back to its max election, so those two moved by design)."""
    p = f'{SCRATCH}/../d551w/px/tenders/{t}.json'
    if not os.path.exists(p):
        p = f'{SCRATCH}/../d551w/px2/tenders/{t}.json'
    if not os.path.exists(p):
        return None
    a = json.load(open(p))
    b = tender_detail(t)
    drop = ('value', 'value_derivation')
    a = {k: v for k, v in a.items() if k not in drop}
    b = {k: v for k, v in b.items() if k not in drop}
    if a == b:
        return 'identical'
    return 'differs:' + ','.join(sorted(k for k in set(a) | set(b) if a.get(k) != b.get(k)))


def port_check(out_path):
    """The M1 asserts on the 279 (+ the out-of-sample fires), against the judge's in-sample decision and the
    evidence's cached facts, sub-check by sub-check."""
    lab, judge, strict, checks = _judge_in_sample()
    shc = json.load(open(EV + '/shapes_work/features.json'))
    ids = sorted(lab)
    rows = {}
    for t in ids + [i for i in OOS_IDS if i not in lab]:
        f = facts(t)
        ship, why, S, cur, n = v2_decision(f)
        g = lab[t]['group'].split('-')[0] if t in lab else 'oos'
        row = {'id': t, 'group': g, 'verdict': (lab.get(t) or {}).get('verdict'), 'class': (lab.get(t) or {}).get('class'),
               'v1_fires': v1_fires(f), 'v1_why': f['v1']['why'], 'ship': ship, 'first': why,
               'refusals': v2_refusals(f), 'S': S, 'cur': cur, 'n': n, 'S_eur': f.get('S_eur'),
               'judge_ship': judge.get(t), 'strict_ship': strict.get(t), 'flags': f['flags'],
               'vouched_by': f.get('vouched_by'), 'detail_vs_cache': _detail_same(t),
               'today': f['v1']['today'], 'served': f['v1']['served'],
               'served_matches_election': f['v1']['served_matches_election'], 'n_notices': len(f['notices']),
               'head_date': f['v1']['date'], 'rate': (1.0 if cur == 'EUR' else f['v1']['rates'].get(cur))}
        ck = checks.get(t)
        if ck is not None:
            P, ps, gsh = f['pred'], ck['pred_strict'], shc[str(t)]['f']
            pairs = [('R1', bool(P['R1']), ps['Bm_g']), ('R2a', bool(P['R2a']), ps['C']), ('R2b', bool(P['R2b']), ps['C_list']),
                     ('R2c', bool(P['R2c']), ps['C_vary']), ('R2d', bool(P['R2d']), bool(ps['C_absent'] and ps['multi_notice'])),
                     ('R3', bool(P['R3']), ps['D_keys']), ('R4', bool(P['R4']), ps['E'])]
            pairs += [(f'R5.{k}', P[f'R5.{k}'], bool(gsh.get(name))) for k, name in enumerate(R5_FEATURES, 1)]
            pairs += [('R6', bool(P['R6']), ps['C_title']), ('R7', bool(P['R7']), ps['P10']), ('R8', bool(P['R8']), ps['DOM10']),
                      ('R9', bool(P['R9']), 'R9' in ck['strict']), ('vouch', bool(f['vouches']), bool(ck['vouch_strict']))]
            row['subcheck_diffs'] = [(name, a, b) for name, a, b in pairs if a != b]
            row['cached_S'] = [ck['S'], ck['cur'], ck['n']]
        rows[t] = row
    json.dump(rows, open(out_path, 'w'), indent=1, default=str)
    L = [r for r in rows.values() if r['group'] != 'oos']
    band = [r for r in L if r['group'] in ('top', 'mid')]
    sample = [r for r in L if r['group'] == 'sample']
    res = {
        'wrongs_refused': (sum(1 for r in L if r['verdict'] == 'wrong' and not r['ship']), sum(1 for r in L if r['verdict'] == 'wrong')),
        'band_ships': sorted(r['id'] for r in band if r['ship']),
        'band_ships_equal_judge15': {r['id'] for r in band if r['ship']} == JUDGE_BAND15,
        'sample_ships': sum(1 for r in sample if r['ship']),
        'oos_ships': sorted(r['id'] for r in rows.values() if r['ship'] and r['id'] in set(OOS_IDS) | {8426318, 6450467, 6450469, 6450473, 8831876}),
        'decision_diffs_vs_judge': [r['id'] for r in L if r['ship'] != r['judge_ship']],
        'v1_not_firing': [r['id'] for r in rows.values() if not r['v1_fires']],
        'served_not_port_election': [r['id'] for r in rows.values() if not r['served_matches_election']],
        'S_diffs_vs_cache': [r['id'] for r in L if r.get('cached_S') and r['cached_S'] != [r['S'], r['cur'], r['n']]],
        'subcheck_diffs': {r['id']: r['subcheck_diffs'] for r in L if r.get('subcheck_diffs')},
    }
    print(json.dumps(res, indent=1, default=str))
    assert res['wrongs_refused'][0] == res['wrongs_refused'][1] == 105, res['wrongs_refused']
    assert res['band_ships_equal_judge15'], res['band_ships']
    assert res['sample_ships'] == 32, res['sample_ships']
    assert set(res['oos_ships']) == OOS_EXPECT_SHIP, res['oos_ships']
    return rows, res


if __name__ == '__main__':
    a = sys.argv[1:]
    if a and a[0] == '--port-check':
        port_check(a[1] if len(a) > 1 else os.path.join(HERE, 'M1-port-check.json'))
    else:
        # B6: python3 -I -B rule_v2_g.py [--rates-dir DIR] ID [ID ...] -> one JSON line per id, all four variants
        if a and a[0] == '--rates-dir':
            set_rates_dir(a[1])
            a = a[2:]
        for t in a:
            f = facts(int(t))
            print(json.dumps({'id': int(t), 'decisions': decide_all(f), 'flags': f['flags'],
                              'vouch_candidates': f.get('vouch_candidates'), 'rates': rates_sha256()}, default=str))
