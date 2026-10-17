"""Generate crates/store/src/canonical_551_v2_fixtures.rs: issue 551 unit 4b (B2)'s real-row fixtures WITH the
B1 facts, from the M1 port's cached public-API facts (offline; no GET is made).

    python3 -I gen_fixtures.py OUT.rs            write the fixtures from the committed input bundle; print each id's
                                                 port outcome (R10 at every size)
    python3 -I gen_fixtures.py --write-inputs    rebuild fixtures_data/inputs.tar.gz from the session's port cache (the
                                                 exact files a run reads); then update INPUTS_SHA256 and SHA256SUMS
    python3 -I gen_fixtures.py --all-port-check  re-decide EVERY M1-port-check.json id with R10 at every size and assert
                                                 the expected mapping (reads the session's port cache, NOT committed:
                                                 it works only while that scratchpad exists)

Each fixture is one Tender as path W v2 reads it, rebuilt from the same inputs `rule_v2.py` (sha256 39be6d1e...05ce)
decided it on: `/v1/tenders/{id}` (the head version: amounts, lots with kinds and the served lot title, every results
round's lot results with decision, winners and key, every round's bids with their parties) and every version notice's
`/v1/notices/{id}/content`, read through the port's own `notice_facts` (the frozen field-to-kind table), so the B1
facts are the port's facts:
  - LotState.published_lot_no: B1's carry, emulated over the version notices in order -- a notice that publishes a
    lot section (Lot / Part / LotsGroup / ProcurementProjectLot) sets that lot's number to the section's own (None
    included; never TED-ITEM, never a title); FTS: the OCDS lot id (the key). One lots-only version per results-round
    notice that is not the head's (no amount, no award: the scale rules read nothing from it) carries the numbers as of
    that notice, which is where canonical.rs's R2 reads them; the head version carries them as of the head notice.
  - LotResultState.published_lot_no: the award block's own TED-LOT_NO / TED-LOT_NUMBER, else BT-13713-LotResult /
    DE1-NoticeResult-LotResult-TenderLot-ID, in the round notice's LotResult section of the result's key; FTS: the lot.
  - LotResultState.group: TED-AWARDED_TO_GROUP in that section (not 0 / false / NO) with no TED-NO_AWARDED_TO_GROUP.
  - TenderVersion.stated_totals: the port's `stated_totals` (every version notice, the frozen table, its correction
    semantics), kind Award / Estimate / Range from its `r3_kind`, `lots_awarded` the keys the notice's round names.
Rates: one per non-EUR currency, the port's rate at the head's date (rule_v2.rate), seeded at the fixtures' instant.
Amount rows are the port's v1 entry's (`_entry`): a withheld amount kept as -1 with its quality, an amount on a lot the
detail does not list carried on a typed Lot.

Inputs (all committed, all sha256-checked before a fixture is built):
  - rule_v2.py (sha256 pinned below as RULE_V2_SHA256), which pins ../unit3/merge_rule.py and ../scripts/elect.py;
  - fixtures_data/inputs.tar.gz (INPUTS_SHA256, also in SHA256SUMS): the 28 fixtures' `/v1/tenders/{id}` reads and
    their 91 version notices' `/v1/notices/{id}/content` reads (cache/tenders, cache/notices), and the three rate files
    (rates/, each checked against the sha256 M1-port-check.md's "Frozen inputs" records), byte-identical to the M1
    port's cache in the session scratchpad (m551/), from which `--write-inputs` cut it. The generator reads nothing
    else: no GET is made, and the bundle is unpacked to a temporary directory the port's cache paths point at;
  - M1-port-check.json: the port's outcomes with R10 from EUR 10 m. For each FIXTURE id this script re-decides it with
    R10 at every size (the "vouched everywhere" fallback) and asserts the expected mapping: a port refusal keeps its
    first refusing check, a vouched ship stays a ship, an unvouched ship becomes R10, and S, currency and n are equal.
    `--all-port-check` asserts the same mapping over every id of M1-port-check.json, from the session cache.
"""
import collections
import gzip
import hashlib
import importlib.util
import io
import json
import os
import re
import sys
import tarfile
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
RULE_V2_SHA256 = '39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce'
INPUTS = os.path.join(HERE, 'fixtures_data', 'inputs.tar.gz')
INPUTS_SHA256 = '775e5c97bc6d067c4bee22aba6ce9f2235bd534920f5da036928621a64434873'
# M1-port-check.md "Frozen inputs": the rate files the port decided on.
RATES_SHA256 = {
    'eurofxref-hist.zip': '8e925cef1572f8df5c8743229a761b029180f3b5cd4aa2213592192964ebd793',
    'ecu-ert_h_eur_d.csv': '47e941270ac4a0f0f22416bd4b41c9b427d43c3a4f430996f8457dca8f134e5c',
    'ecu-ert_bil_eur_d.csv': '48e2c9f17ae7219b2b145a66ace399aba7c306c88721b7ce8f7c76f4758e131b',
}
MEMBER = re.compile(r'(cache/(tenders|notices)/[0-9]+\.json|rates/(' + '|'.join(re.escape(f) for f in RATES_SHA256) + r'))')


def sha256(path):
    return hashlib.sha256(open(path, 'rb').read()).hexdigest()


_got = sha256(os.path.join(HERE, 'rule_v2.py'))
if _got != RULE_V2_SHA256:
    raise SystemExit(f'gen_fixtures: rule_v2.py changed (sha256 {_got}, pinned {RULE_V2_SHA256})')
spec = importlib.util.spec_from_file_location('rule_v2_b2', os.path.join(HERE, 'rule_v2.py'))
R = importlib.util.module_from_spec(spec)
spec.loader.exec_module(R)
SESSION_CACHE, SESSION_RATES = R.CACHE, R.RATES_DIR


def _offline(path):
    raise SystemExit(f'gen_fixtures: {path} is not in the port cache (this script never fetches)')


R._get = _offline


def use_inputs(tmp):
    """Unpack the committed input bundle into `tmp` (after checking its sha256, each member's name and each rate
    file's sha256) and point the port's cache and rates at it."""
    got = sha256(INPUTS)
    if got != INPUTS_SHA256:
        raise SystemExit(f'gen_fixtures: {INPUTS} changed (sha256 {got}, pinned {INPUTS_SHA256})')
    with tarfile.open(INPUTS, 'r:gz') as tar:
        for m in tar.getmembers():
            if not m.isfile() or not MEMBER.fullmatch(m.name):
                raise SystemExit(f'gen_fixtures: unexpected member {m.name!r} in {INPUTS}')
            dst = os.path.join(tmp, *m.name.split('/'))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            with open(dst, 'wb') as out:
                out.write(tar.extractfile(m).read())
    for name, want in RATES_SHA256.items():
        got = sha256(os.path.join(tmp, 'rates', name))
        if got != want:
            raise SystemExit(f'gen_fixtures: rates/{name} sha256 {got}, M1 decided on {want}')
    R.CACHE, R.RATES_DIR, R._RATES = os.path.join(tmp, 'cache'), os.path.join(tmp, 'rates'), None


# The "vouched everywhere" fallback (issue 551, M2 fallback PASSED 2026-10-17): R10 at every size.
R.R10_MIN_EUR_CENTS = 0

# (id, why it is here)
FIXTURES = [
    # served: the three pins and three sample rights the port ships, each vouched
    (585917, 'pin: served, vouched by BT-118-NoticeResult'),
    (5995030, 'pin: served, vouched by TED-VAL_TOTAL'),
    (8426318, 'pin: served, vouched by TED-HIGH'),
    (6862672, 'sample right: served, vouched by TED-VAL_TOTAL (HUF)'),
    (37368, 'sample right: served, vouched by BT-118-NoticeResult'),
    (6483572, 'sample right: served, vouched by TED-LOW'),
    # refused: one wrong per shape S1a..S10 (shapes.md)
    (70706, 'S1a ceiling copied per supplier'),
    (249185, 'S1b parallel competing offers summed'),
    (793898, 'S2 one multi-lot contract counted per lot'),
    (5747154, 'S3a TED single section restates the framework on LOT-1'),
    (5976305, "S3b TED single section of another lot keyed LOT-1"),
    (6421819, 'S3c merged call-off (and the amendment must-refuse: R2d)'),
    (7239967, 'S3d keys collide inside one notice'),
    (182819, 'S4 eForms modification re-keyed'),
    (8648668, "S5 procedure total booked as one lot's award"),
    (5849667, 'S6 nested lots'),
    (739642, 'S7 pseudo-lots'),
    (993254, 'S8 x10^k slip'),
    (240143, 'S9 closed lot summed'),
    (324582, 'S10 unit rates summed'),
    # refused pins
    (6450467, 'Lyon-Turin: unvouched (R10), keeps EUR 1,660,661,682.94'),
    (6450469, 'Lyon-Turin: unvouched (R10)'),
    (6450473, 'Lyon-Turin: unvouched (R10)'),
    (7256284, 'unit 2 holdout: R2d'),
    (5977060, 'unit 2 holdout: R2d'),
    (8831876, 'band: R1 (its GBP 26 bn total sits on 8831875)'),
    (467176, 'unit 2 holdout: unvouched, now R10'),
    (294968, 'unit 2 holdout: unvouched, now R10'),
]
PORT_CHECK = json.load(open(os.path.join(HERE, 'M1-port-check.json')))
LOT_SECTION_KINDS = set(R.LOT_KINDS)
KIND = {'award': 'StatedTotalKind::Award', 'estimate': 'StatedTotalKind::Estimate', None: 'StatedTotalKind::Range'}


def lit(s):
    if s is None:
        return 'None'
    out = []
    for c in str(s):
        if c == '\\':
            out.append('\\\\')
        elif c == '"':
            out.append('\\"')
        elif 32 <= ord(c) < 127:
            out.append(c)
        else:
            out.append('\\u{%x}' % ord(c))
    return '"' + ''.join(out) + '"'


def opt(s):
    return 'None' if s is None else f'Some({lit(s)})'


def opt_i(x):
    return 'None' if x is None else f'Some({int(x)})'


def decision(d):
    # The no-award code is read through store's one constant (head_election_agreement's
    # `the_no_award_decision_is_written_once` tripwire counts every quoted literal under store/src).
    return 'Some(NO_AWARD)' if d == 'clos-nw' else opt(d)


def amount(field, cents, currency, quality):
    return f'({lit(field)}, {int(cents)}, {lit(currency)}, {opt(quality)})'


def build(tid):
    """-> (versions, rates, port outcome, notes): the fixture chain as plain data."""
    x = R.tender_detail(tid)
    f = R.facts(tid)
    fts = x.get('source') == 'fts'
    notes = []
    vseq = [v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0)
            if v.get('caused_by_notice_id')]
    vseq = list(dict.fromkeys(vseq))
    head_notice = vseq[-1]
    NF = {nid: R.notice_facts(R.notice_content(nid)) for nid in f['notices']}
    round_notices = list(dict.fromkeys(lr['notice_id'] for lr in x.get('lot_results', [])))
    for nid in round_notices:
        if nid not in vseq:
            notes.append(f'round notice {nid} caused no version: R2 reads its awards\' own numbers only')
    order = {nid: i for i, nid in enumerate(vseq)}

    # B1's lot-number carry over the version notices.
    carried = {}
    snapshot = {}
    for nid in vseq:
        for sid, L in NF[nid]['lots'].items():
            no = sid if fts else L.get('lot_no')
            carried[sid] = (L['kind'], None if no is None else str(no))
        snapshot[nid] = dict(carried)

    # The head version: the API detail, as the port's v1 entry reads it.
    listed = [l['lot_key'] for l in x.get('lot_details', [])]
    lot_amounts = collections.defaultdict(list)
    facts = []
    extra_lots = []
    for a in x.get('amounts', []):
        v, q = a.get('value'), a.get('quality')
        if v is None and q is None:
            continue
        row = (a['field'], -1 if v is None else int(v['cents']), (v or {}).get('currency') or 'EUR', q)
        if a.get('lot') is None:
            facts.append(row)
        else:
            if a['lot'] not in listed and a['lot'] not in extra_lots:
                extra_lots.append(a['lot'])
                notes.append(f'amount on unlisted lot {a["lot"]}: carried as a typed Lot (as the port)')
            lot_amounts[a['lot']].append(row)
    head_numbers = snapshot[head_notice]
    lots = []
    for l in x.get('lot_details', []):
        lots.append({'key': l['lot_key'], 'kind': l['kind'], 'no': (head_numbers.get(l['lot_key']) or (None, None))[1],
                     'title': l.get('title'), 'amounts': lot_amounts.get(l['lot_key'], [])})
    for key in extra_lots:
        lots.append({'key': key, 'kind': 'Lot', 'no': (head_numbers.get(key) or (None, None))[1], 'title': None,
                     'amounts': lot_amounts[key]})

    rounds = collections.OrderedDict()
    for nid in sorted(set(round_notices) | {b['notice_id'] for b in x.get('bids', []) if b.get('notice_id')},
                      key=lambda n: (order.get(n, len(order)), n)):
        rounds[nid] = {'results': [], 'bids': []}
    for lr in x.get('lot_results', []):
        aw = lr.get('awarded') or {}
        R_ = NF.get(lr['notice_id'], {}).get('results', {}).get(lr.get('key')) or {}
        no = R_.get('lot_no') if R_.get('lot_no') is not None else R_.get('lot_ref')
        if fts and no is None:
            no = lr.get('lot')
        group = R_.get('group') not in (None, 0, '0', False, 'false', 'NO') and 'no_group' not in R_
        winners = [w.get('organization_id') for w in lr.get('winners', [])]
        if any(w is None for w in winners):
            raise SystemExit(f'{tid}: a winner with no organization id cannot be a LotResultState winner')
        rounds[lr['notice_id']]['results'].append({
            'key': lr.get('key'), 'lot': lr.get('lot'), 'cents': aw.get('cents'), 'currency': aw.get('currency'),
            'decision': lr.get('decision'), 'winners': winners, 'no': None if no is None else str(no), 'group': group})
    for b in x.get('bids', []):
        v = b.get('value')
        parties = []
        for p in b.get('parties', []):
            if p.get('organization_id') is None:
                notes.append(f'bid {b.get("key")}: a party with no organization id is left out')
                continue
            parties.append((p.get('role'), int(p['organization_id'])))
        rounds[b['notice_id']]['bids'].append({
            'key': b.get('key'), 'lot': b.get('lot'), 'cents': None if v is None else v.get('cents'),
            'currency': None if v is None else v.get('currency'), 'quality': b.get('quality'), 'parties': parties})

    keys_of = collections.defaultdict(set)
    for lr in x.get('lot_results', []):
        if lr.get('lot'):
            keys_of[lr['notice_id']].add(lr['lot'])
    totals = []
    for t in f.get('stated_totals') or []:
        totals.append({'notice': t['notice_id'], 'logical': t.get('logical_id'), 'field': t['field'],
                       'kind': KIND[t['r3_kind']], 'cents': int(t['cents']), 'currency': t['currency'],
                       'lots': sorted(keys_of.get(t['notice_id'], ()))})

    versions = []
    for nid in vseq:
        if nid in round_notices and nid != head_notice:
            versions.append({'notice': nid, 'facts': [], 'rounds': [], 'totals': [],
                             'lots': [{'key': k, 'kind': kind, 'no': no, 'title': None, 'amounts': []}
                                      for k, (kind, no) in sorted(snapshot[nid].items())]})
    versions.append({'notice': head_notice, 'facts': facts, 'lots': lots, 'totals': totals,
                     'rounds': [dict(notice=n, **r) for n, r in rounds.items()]})

    # Where B1's carry gives a summed award's round a number other than the port's own-notice reading, say so.
    for s in f.get('summed') or []:
        for lr in x.get('lot_results', []):
            if lr.get('lot') != s['key']:
                continue
            port_no, _ = _port_pub_no(x, NF, lr['notice_id'], s['key'], lr.get('key'))
            snap = snapshot.get(lr['notice_id'], {})
            R_ = NF.get(lr['notice_id'], {}).get('results', {}).get(lr.get('key')) or {}
            res_no = R_.get('lot_no') if R_.get('lot_no') is not None else R_.get('lot_ref')
            if fts and res_no is None:
                res_no = lr.get('lot')
            b1_no = (snap.get(s['key']) or (None, None))[1]
            b1_no = b1_no if b1_no is not None else res_no
            if (None if port_no is None else str(port_no)) != (None if b1_no is None else str(b1_no)):
                notes.append(f'R2 number differs for {s["key"]} in notice {lr["notice_id"]}: port {port_no!r}, B1 {b1_no!r}')

    rates = dict(sorted(f['v1']['rates'].items()))
    ship, first, S, cur, n = R.v2_decision(f)
    return versions, rates, {'ship': ship, 'first': first, 'S': S, 'cur': cur, 'n': n,
                             'vouched_by': f.get('vouched_by'), 'today': f['v1']['today']}, notes


def _port_pub_no(x, NF, nid, key, result_key):
    nf = NF.get(nid)
    if nf is None:
        return None, 'unread'
    L = nf['lots'].get(key)
    if L and L.get('lot_no') is not None:
        return L['lot_no'], 'lot section'
    R_ = nf['results'].get(result_key) if result_key is not None else None
    if R_ and R_.get('lot_no') is not None:
        return R_['lot_no'], 'result LOT_NO'
    if R_ and R_.get('lot_ref') is not None:
        return R_['lot_ref'], 'result BT-13713'
    if x.get('source') == 'fts':
        return key, 'OCDS'
    return None, 'none'


def rust_version(v):
    i = ' ' * 16
    out = [f'V {{', f'{i}notice: {v["notice"]},']
    if v['facts']:
        out.append(f'{i}facts: &[' + ''.join(f'\n{i}    {amount(*a)},' for a in v['facts']) + f'\n{i}],')
    else:
        out.append(f'{i}facts: &[],')
    if v['lots']:
        out.append(f'{i}lots: &[')
        for l in v['lots']:
            am = '&[' + ', '.join(amount(*a) for a in l['amounts']) + ']'
            out.append(f'{i}    L {{ key: {lit(l["key"])}, kind: {lit(l["kind"])}, no: {opt(l["no"])}, title: {opt(l["title"])}, amounts: {am} }},')
        out.append(f'{i}],')
    else:
        out.append(f'{i}lots: &[],')
    if v['rounds']:
        out.append(f'{i}rounds: &[')
        for r in v['rounds']:
            out.append(f'{i}    Rd {{')
            out.append(f'{i}        notice: {r["notice"]},')
            if r['results']:
                out.append(f'{i}        results: &[')
                for x in r['results']:
                    out.append(f'{i}            Res {{ key: {lit(x["key"])}, lot: {opt(x["lot"])}, cents: {opt_i(x["cents"])}, '
                               f'currency: {opt(x["currency"])}, decision: {decision(x["decision"])}, '
                               f'winners: &[{", ".join(str(w) for w in x["winners"])}], no: {opt(x["no"])}, group: {"true" if x["group"] else "false"} }},')
                out.append(f'{i}        ],')
            else:
                out.append(f'{i}        results: &[],')
            if r['bids']:
                out.append(f'{i}        bids: &[')
                for b in r['bids']:
                    parties = ', '.join(f'({lit(role)}, {org})' for role, org in b['parties'])
                    out.append(f'{i}            Bid {{ key: {lit(b["key"])}, lot: {opt(b["lot"])}, cents: {opt_i(b["cents"])}, '
                               f'currency: {opt(b["currency"])}, quality: {opt(b["quality"])}, parties: &[{parties}] }},')
                out.append(f'{i}        ],')
            else:
                out.append(f'{i}        bids: &[],')
            out.append(f'{i}    }},')
        out.append(f'{i}],')
    else:
        out.append(f'{i}rounds: &[],')
    if v['totals']:
        out.append(f'{i}totals: &[')
        for t in v['totals']:
            lots = ', '.join(lit(k) for k in t['lots'])
            out.append(f'{i}    St {{ notice: {t["notice"]}, logical: {opt(t["logical"])}, field: {lit(t["field"])}, '
                       f'kind: {t["kind"]}, cents: {t["cents"]}, currency: {lit(t["currency"])}, lots: &[{lots}] }},')
        out.append(f'{i}],')
    else:
        out.append(f'{i}totals: &[],')
    out.append(' ' * 12 + '}')
    return '\n'.join(out)


HEADER = r'''// @generated by .scratch/tender-db/551-lot-sum/unit4b/gen_fixtures.py from the M1 port's cached public-API reads.
//! Issue 551 unit 4b (B2): the labelled Tenders path W v2 was decided and measured on, as path W reads them, WITH the
//! B1 facts (published lot numbers, the award-to-group flag, the chain's stated totals). Each is rebuilt from the
//! inputs `rule_v2.py` decided it on (`/v1/tenders/{id}` and every version notice's content, cached by unit 4b's M1
//! port check): the head version's amounts, lots (kind, served title, the number B1 carries), every results round's
//! lot results (decision, winners, the award block's own number, the group flag) and bids (value, parties), and the
//! port's stated totals under the frozen field-to-kind table. Before the head, one lots-only version per other
//! results-round notice carries the lot numbers as of that notice (no amount and no award, so the scale rules read
//! nothing from it): canonical.rs's R2 reads a summed award's number from the version its round's notice caused.
//! Rates: the port's rate at the head's date, seeded at the fixtures' instant.
#![allow(clippy::type_complexity)]

use super::{BidParty, BidState, DECISION_NO_AWARD as NO_AWARD, Fact, LotResultState, LotState, Round, StatedTotal, StatedTotalKind, TenderVersion};

/// An amount: (field, cents, currency, quality).
pub(super) type A = (&'static str, i64, &'static str, Option<&'static str>);

/// A lot: key, kind, the number B1 carries for it, its served title, its amounts.
pub(super) struct L {
    pub key: &'static str,
    pub kind: &'static str,
    pub no: Option<&'static str>,
    pub title: Option<&'static str>,
    pub amounts: &'static [A],
}

/// A lot result: key, lot key, award (cents, currency), decision, winners, the block's own number, the group flag.
pub(super) struct Res {
    pub key: &'static str,
    pub lot: Option<&'static str>,
    pub cents: Option<i64>,
    pub currency: Option<&'static str>,
    pub decision: Option<&'static str>,
    pub winners: &'static [i64],
    pub no: Option<&'static str>,
    pub group: bool,
}

/// A bid: key, lot key, value, quality, parties as (role, organization id).
pub(super) struct Bid {
    pub key: &'static str,
    pub lot: Option<&'static str>,
    pub cents: Option<i64>,
    pub currency: Option<&'static str>,
    pub quality: Option<&'static str>,
    pub parties: &'static [(&'static str, i64)],
}

/// A results round: its notice, its lot results, its bids.
pub(super) struct Rd {
    pub notice: i64,
    pub results: &'static [Res],
    pub bids: &'static [Bid],
}

/// A stated total ([`StatedTotal`]).
pub(super) struct St {
    pub notice: i64,
    pub logical: Option<&'static str>,
    pub field: &'static str,
    pub kind: StatedTotalKind,
    pub cents: i64,
    pub currency: &'static str,
    pub lots: &'static [&'static str],
}

/// One version: the notice that caused it, tender-scope amounts, lots, rounds, stated totals.
pub(super) struct V {
    pub notice: i64,
    pub facts: &'static [A],
    pub lots: &'static [L],
    pub rounds: &'static [Rd],
    pub totals: &'static [St],
}

/// One Tender: its id, the rates (units per EUR) of its non-EUR currencies, its chain oldest first.
pub(super) struct Fixture {
    pub id: i64,
    pub rates: &'static [(&'static str, f64)],
    pub chain: &'static [V],
}

/// The fixtures' publication instant (2023-11-14), at which their rates are seeded.
pub(super) const PUBLISHED_AT: i64 = 1_700_000_000;

impl Fixture {
    /// The chain as [`TenderVersion`]s.
    pub fn versions(&self) -> Vec<TenderVersion> {
        let amount = |&(field, cents, currency, quality): &A| Fact::Amount {
            field: field.into(),
            cents,
            currency: currency.into(),
            tax_basis: None,
            quality: quality.map(Into::into),
        };
        self.chain
            .iter()
            .enumerate()
            .map(|(i, v)| TenderVersion {
                caused_by_notice_id: v.notice,
                published_at: PUBLISHED_AT,
                dispatched_at: None,
                notice_subtype: None,
                original_lang: None,
                publication_id: format!("{}-{}", self.id, i + 1),
                facts: v.facts.iter().map(amount).collect(),
                lots: v
                    .lots
                    .iter()
                    .map(|l| LotState {
                        key: l.key.into(),
                        kind: l.kind.into(),
                        facts: l
                            .amounts
                            .iter()
                            .map(amount)
                            .chain(l.title.map(|t| Fact::Text { field: "title".into(), lang: None, value: t.into() }))
                            .collect(),
                        published_lot_no: l.no.map(Into::into),
                    })
                    .collect(),
                rounds: v
                    .rounds
                    .iter()
                    .map(|r| Round {
                        notice_id: r.notice,
                        logical_notice_id: None,
                        lot_results: r
                            .results
                            .iter()
                            .map(|x| LotResultState {
                                key: x.key.into(),
                                lot_key: x.lot.map(Into::into),
                                decision: x.decision.map(Into::into),
                                reason: None,
                                awarded_cents: x.cents,
                                awarded_currency: x.currency.map(Into::into),
                                decided: None,
                                winners: x.winners.to_vec(),
                                buyer_winners: Vec::new(),
                                statistics: Vec::new(),
                                published_lot_no: x.no.map(Into::into),
                                group: x.group,
                            })
                            .collect(),
                        bids: r
                            .bids
                            .iter()
                            .map(|b| BidState {
                                key: b.key.into(),
                                lot_key: b.lot.map(Into::into),
                                cents: b.cents,
                                currency: b.currency.map(Into::into),
                                quality: b.quality.map(Into::into),
                                parties: b
                                    .parties
                                    .iter()
                                    .map(|&(role, organization_id)| BidParty { role: role.into(), organization_id, section_id: String::new() })
                                    .collect(),
                            })
                            .collect(),
                        contracts: Vec::new(),
                    })
                    .collect(),
                group_members: Vec::new(),
                stated_totals: v
                    .totals
                    .iter()
                    .map(|t| StatedTotal {
                        notice_id: t.notice,
                        logical_notice_id: t.logical.map(Into::into),
                        field: t.field.into(),
                        kind: t.kind,
                        cents: t.cents,
                        currency: t.currency.into(),
                        lots_awarded: t.lots.iter().map(|k| (*k).to_owned()).collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// A rates lookup holding the fixture's rates at [`PUBLISHED_AT`]'s date.
    pub fn rates(&self) -> crate::rates::RatesLookup {
        let mut rates = crate::rates::RatesLookup::default();
        let date = crate::rates::civil_date(PUBLISHED_AT);
        for (currency, per_eur) in self.rates {
            rates.insert_for_test(currency, &date, *per_eur);
        }
        rates
    }
}

/// The fixture of Tender `id`.
pub(super) fn fixture(id: i64) -> &'static Fixture {
    FIXTURES.iter().find(|f| f.id == id).unwrap_or_else(|| panic!("no fixture {id}"))
}
'''


def check_against_port(tid, ship, first, S, cur, n):
    """With R10 from EUR 10 m (M1) the port shipped or refused; at every size, a ship with no vouch is R10, and every
    other outcome (a refusal's first check included) and S, currency and n are unchanged."""
    pc = PORT_CHECK.get(str(tid))
    if pc is None:
        return False
    expect_ship = pc['ship'] and pc['vouched_by'] is not None
    expect_first = pc['first'] if not pc['ship'] else (None if expect_ship else 'R10')
    assert (ship, first) == (expect_ship, expect_first), (tid, ship, first, pc['ship'], pc['first'])
    assert [S, cur, n] == [pc['S'], pc['cur'], pc['n']], (tid, S, cur, n, pc)
    return True


def write_inputs():
    """Cut fixtures_data/inputs.tar.gz from the session's port cache: exactly the files a fixture run reads, plus the
    rate files, as a deterministic archive (sorted names, zero times and owners)."""
    read = set()
    cached = R._cached

    def recording(kind, key, path, refresh=False):
        read.add(f'cache/{kind}/{int(key)}.json')
        return cached(kind, key, path, refresh)
    R._cached = recording
    for tid, _ in FIXTURES:
        build(tid)
    R._cached = cached
    files = {name: os.path.join(SESSION_CACHE, *name.split('/')[1:]) for name in read}
    files.update({f'rates/{name}': os.path.join(SESSION_RATES, name) for name in RATES_SHA256})
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode='w', format=tarfile.PAX_FORMAT) as tar:
        for name in sorted(files):
            data = open(files[name], 'rb').read()
            info = tarfile.TarInfo(name)
            info.size, info.mtime, info.mode, info.uid, info.gid, info.uname, info.gname = len(data), 0, 0o644, 0, 0, '', ''
            tar.addfile(info, io.BytesIO(data))
    os.makedirs(os.path.dirname(INPUTS), exist_ok=True)
    with open(INPUTS, 'wb') as f, gzip.GzipFile(fileobj=f, mode='wb', mtime=0, compresslevel=9, filename='') as gz:
        gz.write(raw.getvalue())
    print('wrote', INPUTS, len(files), 'files, sha256', sha256(INPUTS))


def all_port_check():
    """Re-decide every M1-port-check.json id with R10 at every size, from the session's port cache."""
    ids = sorted(int(k) for k in PORT_CHECK)
    moved = collections.Counter()
    for tid in ids:
        f = R.facts(tid)
        ship, first, S, cur, n = R.v2_decision(f)
        check_against_port(tid, ship, first, S, cur, n)
        pc = PORT_CHECK[str(tid)]
        moved[(('ship' if pc['ship'] else 'refuse ' + str(pc['first'])), ('ship' if ship else 'refuse ' + str(first)))] += 1
    print(f'all {len(ids)} M1-port-check ids agree (R10 at every size)')
    for (was, now), k in sorted(moved.items()):
        print(f'  {k:>4}  {was} -> {now}')


def main():
    if sys.argv[1:] == ['--write-inputs']:
        return write_inputs()
    if sys.argv[1:] == ['--all-port-check']:
        return all_port_check()
    if len(sys.argv) != 2 or sys.argv[1].startswith('-'):
        raise SystemExit(__doc__)
    with tempfile.TemporaryDirectory() as tmp:
        use_inputs(tmp)
        generate(sys.argv[1])


def generate(dst):
    out = [HEADER.rstrip('\n'), '', 'pub(super) const FIXTURES: &[Fixture] = &[']
    report = []
    for tid, why in FIXTURES:
        versions, rates, outcome, notes = build(tid)
        assert check_against_port(tid, outcome['ship'], outcome['first'], outcome['S'], outcome['cur'], outcome['n']), \
            f'{tid}: not in M1-port-check.json'
        rl = '&[' + ', '.join(f'({lit(c)}, {repr(float(r))}_f64)' for c, r in rates.items()) + ']'
        out.append(f'    // {tid}: {why}. Port (R10 at every size): '
                   + ('ship' if outcome['ship'] else f'refuse {outcome["first"]}')
                   + f', S {outcome["S"]} {outcome["cur"]} over {outcome["n"]} lots'
                   + (f', vouched by {outcome["vouched_by"]["field"]} {outcome["vouched_by"]["cents"]} (notice {outcome["vouched_by"]["notice_id"]})' if outcome['ship'] else '')
                   + '.')
        out.append('    Fixture {')
        out.append(f'        id: {tid},')
        out.append(f'        rates: {rl},')
        out.append('        chain: &[')
        for v in versions:
            out.append(f'            {rust_version(v)},')
        out.append('        ],')
        out.append('    },')
        report.append((tid, why, outcome, notes))
    out.append('];')
    open(dst, 'w').write('\n'.join(out) + '\n')
    print('wrote', dst, len(out), 'lines')
    for tid, why, o, notes in report:
        vb = o['vouched_by']
        print(f'{tid:>8} ship={o["ship"]!s:5} first={o["first"]!s:6} S={o["S"]} {o["cur"]} n={o["n"]} '
              f'today={o["today"] and (o["today"]["cents"], o["today"]["currency"])} '
              f'vouched_by={vb and (vb["field"], vb["notice_id"], vb["cents"])}  # {why}')
        for n in notes:
            print('           note:', n)


if __name__ == '__main__':
    main()
