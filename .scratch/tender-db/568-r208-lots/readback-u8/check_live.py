"""568 unit 8 + 570 read-back, bar (i) PARITY: read Tenders from the live API and compare every served block with the
combined replay (combined_port.py: 570's remap + 568's match with G1 / G2 + the amended guards, in fold's order).

Reads the public API only, at most ONE GET per 1.5 s (`--gap`, never below 1.5), a 408 retried once and nothing else
retried. Every raw read lands in the cache dir as tenders_<id>.json, notices_<id>_content.json and
notices_tender_<id>.json, and an existing file is never re-fetched, so a re-run of the same dir costs no GET. Use a NEW
dir for a read after the drain: a tender file read before it would be compared as if it were current. Notice contents
are the parsed values the fold reads (/v1/notices/{id}/content), read for the Tender's CHAIN only: a notice that
appears only in lot_results is never read (the replay has no use for it; its blocks are reported as served outside the
chain). `--reuse-notices DIR` (repeatable, refused in bar mode) takes a notice from an older read-only cache first,
which saves GETs and is right only while no re-parse has touched those notices since.

Legacy. combined_port.py (unchanged) tells a legacy notice by `TED-` / `TXT-` fields in its content, which carries no
profile. Each Tender's chain profiles are read with ONE /v1/notices?tender={id} (json::notice's `profile`, the column
fold's `is_legacy_profile` reads; the same field /v1/notices/{id} returns), and checked against that prefix test per
chain notice. A disagreement means the replay modelled the notice wrongly: its Tender is adjudicated. While they agree,
the replay's `non-legacy` reason IS the profile's classification. Outside bar mode a Tender whose listing was not read
(offline and uncached, or a failed GET) falls back to the prefix test, recorded in `profile_from_prefix` (adjudicate).

Bar mode (`--bar`), the read-back of record (decision-u7.md "Amendment (unit 8, 2026-10-18)", "The read-back bar"):
  - IDS must be draw_fresh30.py's output with `complete: true`, exactly 30 distinct ids and a `cursor_range`;
  - `--cache` must be a new or empty dir: every Tender, listing and notice is read fresh, after the drain;
  - no `--reuse-notices`, no `--offline`;
  - `--drain-job ID` and `--drain-finished TIME` (RFC 3339 with an offset or Z) are required and recorded in the
    summary, and the read refuses to start before TIME;
  - a Tender whose profile listing is not read is incomplete (no prefix fallback).
Outside bar mode a draw file must still have `complete: true` (an id list or comma list is taken as is), and
`pass_bar_i` is false by construction.

Verdict: `parity_verdict`, and `pass_bar_i` = bar mode and parity_verdict == "pass".
  fail        a parity break (a legacy block whose served lot is not the replay's), an incomplete Tender, or fewer
              Tenders read than listed;
  adjudicate  nothing failed, but some block the replay cannot decide: computed but not served
              (`computed_not_served`); served from a notice outside the chain (`served_outside_chain`); a non-legacy
              block whose served lot is not its BT-13713 (`non_legacy_mismatches`, reported apart and never counted
              as a parity break); a chain notice whose profile and prefix legacy tests disagree
              (`legacy_test_disagrees`); a Tender whose profiles were not read (`profile_from_prefix`);
  pass        none of these.

G2 stratum: when IDS is a draw, for each drawn Tender with `g2: true`, the blocks the replay moves from a pre-drain-rule
(base) lot of null to a lot with cause `570`, and of those the ones whose base was G2's own refusal (`568 none: g2`):
does the stratum exercise 570's G2 narrowing at all?

Usage:
  python3 -I -B check_live.py --cache DIR IDS [--gap 1.6] [--reuse-notices DIR]... [--offline] [--out summary.json]
  python3 -I -B check_live.py --bar --cache NEW_DIR DRAW.json --drain-job ID --drain-finished TIME [--out summary.json]
IDS is a file (one id per line, or a JSON list, or draw_fresh30.py's output with its `draw`) or a comma list.
--offline makes no GET at all: a Tender or notice missing from the caches is reported, not fetched.

Prints, per Tender, every block whose served lot differs from the expected one (a parity break), non-legacy mismatches
and blocks to adjudicate, the G2 stratum, then one JSON summary line set: tenders read, blocks compared, breaks (with
the replay's reason and what moved the block relative to the pre-drain rule: 570 / 568-u8 / both / unchanged), blocks
outside the model, incomplete Tenders, fetch errors, the verdict and its reasons.
"""
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import combined_port as C  # noqa: E402

BASE = 'https://tenders.zebreus.click'
MIN_GAP = 1.5
N_BAR = 30


def is_legacy_profile(profile):
    """crates/ingest/src/project.rs `is_legacy_profile`."""
    return profile in ('text', 'internal-ojs') or profile.startswith('ted-export')


class Fetcher:
    def __init__(self, cache, gap, offline, reuse):
        self.cache, self.gap, self.offline = cache, max(MIN_GAP, gap), offline
        self.reuse = [C.Cache([d]) for d in reuse]
        self.last = 0.0
        self.gets = 0
        self.errors = []
        self.last_code = None  # of the latest _get: 'cached', 'offline', or the HTTP status curl reported

    def _get(self, path, out):
        if os.path.exists(out):
            self.last_code = 'cached'
            return True
        if self.offline:
            self.last_code = 'offline'
            self.errors.append(f'{path}: not cached (offline)')
            return False
        for attempt in (1, 2):
            wait = self.last + self.gap - time.time()
            if wait > 0:
                time.sleep(wait)
            self.last = time.time()
            self.gets += 1
            tmp = out + '.part'
            r = subprocess.run(['curl', '-sS', '--max-time', '60', '-o', tmp, '-w', '%{http_code}', BASE + path],
                               capture_output=True, text=True)
            code = r.stdout.strip()
            self.last_code = code
            if code == '200':
                os.replace(tmp, out)
                return True
            if os.path.exists(tmp):
                os.remove(tmp)
            if code == '408' and attempt == 1:
                continue
            self.errors.append(f'{path}: HTTP {code} {r.stderr.strip()}')
            return False
        return False

    def tender(self, tid):
        out = os.path.join(self.cache, f'tenders_{tid}.json')
        return C._ld(out) if self._get(f'/v1/tenders/{tid}', out) else None

    def notice(self, nid):
        out = os.path.join(self.cache, f'notices_{nid}_content.json')
        if not os.path.exists(out):
            for rc in self.reuse:
                if nid in rc.notices:
                    return C._ld(rc.notices[nid])
        return C._ld(out) if self._get(f'/v1/notices/{nid}/content', out) else None

    def profiles(self, tid):
        """{notice id: profile} of the Tender's chain notices (/v1/notices?tender={id}), or None when not read.
        Offline and uncached is None without an error: the caller records the prefix fallback."""
        out = os.path.join(self.cache, f'notices_tender_{tid}.json')
        if self.offline and not os.path.exists(out):
            return None
        if not self._get(f'/v1/notices?tender={tid}', out):
            return None
        return {int(x['id']): x.get('profile') for x in C._ld(out).get('items', [])}


def read_ids(arg):
    """(ids, draw): draw is draw_fresh30.py's output when IDS is one (a JSON object with `draw`), else None.
    A draw whose `complete` is not true is refused, in every mode."""
    if os.path.exists(arg):
        text = open(arg, encoding='utf-8').read()
        try:
            d = json.loads(text)
        except ValueError:
            return [int(x) for x in text.replace(',', ' ').split() if x.strip().isdigit()], None
        draw = None
        if isinstance(d, dict):
            if 'draw' in d:
                draw = d
                if d.get('complete') is not True:
                    sys.exit(f'{arg}: refusing an incomplete draw (complete: {d.get("complete")!r}, held '
                             f'{d.get("held")}, out {d.get("out")}, g2 {d.get("g2")}, stopped: {d.get("stopped")!r})')
            d = d.get('draw') or d.get('ids') or []
        return [int(x['id'] if isinstance(x, dict) else x) for x in d], draw
    return [int(x) for x in arg.split(',') if x.strip()], None


def parse_time(s):
    t = datetime.fromisoformat(s.replace('Z', '+00:00'))
    if t.tzinfo is None:
        raise ValueError('no offset')
    return t


def bar_refusals(cache, ids, draw, reuse, offline, drain_job, drain_finished, now):
    problems = []
    if reuse:
        problems.append('--reuse-notices is not allowed: a bar read takes every notice fresh')
    if offline:
        problems.append('--offline is not allowed: a bar read is a fresh read')
    if os.path.exists(cache) and (not os.path.isdir(cache) or os.listdir(cache)):
        problems.append(f'--cache {cache} exists and is not an empty dir: a bar read needs a new, empty one')
    if draw is None:
        problems.append("IDS is not draw_fresh30.py's output (a JSON object with `draw` and `complete`)")
    else:
        if len(ids) != N_BAR or len(set(ids)) != N_BAR:
            problems.append(f'the draw holds {len(ids)} ids ({len(set(ids))} distinct); bar mode needs exactly {N_BAR}')
        if not draw.get('cursor_range'):
            problems.append('the draw records no cursor_range (draw_fresh30.py --from-cursor / --to-cursor)')
    if not drain_job:
        problems.append('--drain-job is required (the drain job id)')
    if not drain_finished:
        problems.append('--drain-finished is required (the drain job completion time, RFC 3339)')
    else:
        try:
            if parse_time(drain_finished) > now:
                problems.append(f'--drain-finished {drain_finished} is after now ({now.isoformat()}): a bar read is '
                                'read after the drain')
        except ValueError:
            problems.append(f'--drain-finished {drain_finished!r} is not an RFC 3339 time with an offset or Z')
    return problems


def check(tender, contents):
    """Per served block: (served, expected row, key); the computed keys not served; every computed row.
    Expected rows come from combined_port.attribute()."""
    rows = C.attribute(tender, contents)
    out, served_keys = [], set()
    for r in tender.get('lot_results', []):
        k = (r['notice_id'], r['key'])
        served_keys.add(k)
        out.append((r.get('lot'), rows.get(k), k))
    extra = [k for k in rows if k not in served_keys]
    return out, extra, rows


def g2_moves(rows):
    """Blocks moved from a base (pre-drain rule) of null to a lot with cause 570, and those whose base was G2's."""
    moved = {k: r for k, r in rows.items() if r['base_lot'] is None and r['expected_lot'] is not None
             and r['cause'] == '570'}
    via = [k for k, r in moved.items() if r['base_reason'].startswith('568 none: g2')]
    return ([f"{k[0]}/{k[1]} -> {moved[k]['expected_lot']}" for k in sorted(moved)],
            [f'{k[0]}/{k[1]}' for k in sorted(via)])


def main():
    args = sys.argv[1:]

    def opt(name, default=None, many=False):
        vals = []
        while name in args:
            i = args.index(name)
            vals.append(args[i + 1])
            del args[i:i + 2]
        return vals if many else (vals[-1] if vals else default)
    cache = opt('--cache')
    gap = float(opt('--gap', '1.6'))
    out_path = opt('--out')
    reuse = opt('--reuse-notices', many=True)
    drain_job, drain_finished = opt('--drain-job'), opt('--drain-finished')
    offline, bar = '--offline' in args, '--bar' in args
    args = [a for a in args if a not in ('--offline', '--bar')]
    if not cache or len(args) != 1:
        sys.exit(__doc__)
    ids, draw = read_ids(args[0])
    started = datetime.now(timezone.utc)
    if bar:
        problems = bar_refusals(cache, ids, draw, reuse, offline, drain_job, drain_finished, started)
        if problems:
            sys.exit('bar mode refused:\n  ' + '\n  '.join(problems))
    os.makedirs(cache, exist_ok=True)
    f = Fetcher(cache, gap, offline, reuse)
    g2_ids = [x['id'] for x in (draw or {}).get('draw', []) if isinstance(x, dict) and x.get('g2')]
    summary = {'mode': 'bar' if bar else 'check', 'drain': {'job': drain_job, 'finished': drain_finished},
               'read_started': started.isoformat(timespec='seconds'), 'cache': os.path.abspath(cache),
               'ids_from': os.path.abspath(args[0]) if os.path.exists(args[0]) else args[0],
               'draw_cursor_range': (draw or {}).get('cursor_range'), 'reuse_notices': reuse,
               'tenders': len(ids), 'read': 0, 'blocks': 0, 'parity_ok': 0, 'breaks': [], 'tenders_with_breaks': [],
               'non_legacy_ok': 0, 'non_legacy_mismatches': [], 'served_outside_chain': [], 'computed_not_served': [],
               'legacy_test_disagrees': [], 'profile_from_prefix': [], 'incomplete': [], 'by_cause': {},
               'by_reason': {}}
    g2_report = {}
    for tid in ids:
        t = f.tender(tid)
        if t is None:
            summary['incomplete'].append({'tender_id': tid, 'why': 'tender not read'})
            print(f'TENDER {tid}: INCOMPLETE (tender not read)')
            continue
        chain = C.chain_of(t)
        contents, missing = {}, []
        for n in chain:  # the chain only: a notice seen only in lot_results is never used
            c = f.notice(n)
            if c is None:
                missing.append(n)
            else:
                contents[n] = c
        if missing:
            summary['incomplete'].append({'tender_id': tid, 'why': f'chain notices not read: {missing}'})
            print(f'TENDER {tid}: INCOMPLETE (chain notices not read: {missing})')
            continue
        prof = f.profiles(tid)
        unknown = [n for n in chain if not (prof or {}).get(n)]
        if unknown and bar:
            why = 'profile listing not read' if prof is None else f'chain notices without a profile: {unknown}'
            summary['incomplete'].append({'tender_id': tid, 'why': why})
            print(f'TENDER {tid}: INCOMPLETE ({why})')
            continue
        if unknown:
            summary['profile_from_prefix'].append(tid)
        else:
            for n in chain:
                lp, lc = is_legacy_profile(prof[n]), C.is_legacy(contents[n])
                if lp != lc:
                    summary['legacy_test_disagrees'].append(
                        f"{tid}/{n}: profile {prof[n]!r} is {'legacy' if lp else 'non-legacy'}, the content prefix "
                        f"test says {'legacy' if lc else 'non-legacy'}")
        summary['read'] += 1
        pairs, extra, rows = check(t, contents)
        breaks, nl_bad, outside = [], [], []
        for served, row, k in pairs:
            summary['blocks'] += 1
            if row is None:
                outside.append(f'{tid}/{k[0]}/{k[1]}: served {served} (notice not in the chain)')
                continue
            reason = ' '.join(re.sub(r'\b(?:LOT|PAR|GRP)-[^\s,()]+', '', row['reason']).split(' (')[0].split())
            summary['by_reason'][reason] = summary['by_reason'].get(reason, 0) + 1
            summary['by_cause'][row['cause']] = summary['by_cause'].get(row['cause'], 0) + 1
            if row['reason'].startswith('non-legacy'):
                if served == row['expected_lot']:
                    summary['non_legacy_ok'] += 1
                else:
                    nl_bad.append({'tender_id': tid, 'notice_id': k[0], 'key': k[1], 'served': served,
                                   'bt_13713': row['expected_lot']})
                continue
            if served == row['expected_lot']:
                summary['parity_ok'] += 1
                continue
            b = {'tender_id': tid, 'notice_id': k[0], 'key': k[1], 'served': served, 'expected': row['expected_lot'],
                 'printed': row['printed'], 'reason': row['reason'], 'cause_vs_pre_drain_rule': row['cause'],
                 'pre_drain_rule_lot': row['base_lot'], 'remap_only_lot': row['remap_only_lot'],
                 'guards_only_lot': row['guards_only_lot'], 'notice_remap': row['notice_remap']}
            breaks.append(b)
        not_served = [f'{tid}/{k[0]}/{k[1]}' for k in extra]
        summary['computed_not_served'] += not_served
        summary['served_outside_chain'] += outside
        summary['non_legacy_mismatches'] += nl_bad
        summary['breaks'] += breaks
        if breaks:
            summary['tenders_with_breaks'].append(tid)
        if tid in g2_ids:
            moved, via = g2_moves(rows)
            g2_report[tid] = {'read': True, 'exercises_570': bool(moved), 'null_to_lot_by_570': moved,
                              'of_which_base_was_g2_refusal': via}
        print(f'TENDER {tid}: {len(pairs)} blocks, {len(breaks)} parity break(s)'
              + (f', {len(nl_bad)} non-legacy mismatch(es)' if nl_bad else '')
              + (f', ADJUDICATE {len(not_served)} computed-not-served + {len(outside)} served-outside-chain'
                 if not_served or outside else ''))
        for b in breaks:
            print(f"  BREAK {b['notice_id']}/{b['key']} printed {b['printed']!r}: served {b['served']} expected "
                  f"{b['expected']} ({b['reason']}; vs pre-drain rule {b['pre_drain_rule_lot']}: {b['cause_vs_pre_drain_rule']})")
        for b in nl_bad:
            print(f"  NON-LEGACY {b['notice_id']}/{b['key']}: served {b['served']}, BT-13713 {b['bt_13713']} "
                  '(not a parity break; adjudicate)')
        for x in not_served:
            print(f'  ADJUDICATE computed, not served: {x}')
        for x in outside:
            print(f'  ADJUDICATE {x}')
    for tid in g2_ids:
        g2_report.setdefault(tid, {'read': False})
    if draw is not None:
        ex = [t for t, r in g2_report.items() if r.get('exercises_570')]
        summary['g2_stratum'] = {
            'tenders': len(g2_ids), 'read': sum(1 for r in g2_report.values() if r['read']),
            'exercising_570': len(ex),
            'exercising_570_via_g2_refusal': sum(1 for r in g2_report.values() if r.get('of_which_base_was_g2_refusal')),
            'rule': 'a block whose base (pre-drain rule) lot is null and whose expected lot is not, with cause 570',
            'per_tender': g2_report}
        for tid, r in g2_report.items():
            if not r['read']:
                print(f'G2 TENDER {tid}: not read')
            else:
                print(f"G2 TENDER {tid}: {len(r['null_to_lot_by_570'])} block(s) base null -> lot by 570 "
                      f"({len(r['of_which_base_was_g2_refusal'])} of them G2's refusal at base)"
                      + (f": {r['null_to_lot_by_570'][:5]}" if r['null_to_lot_by_570'] else ' -- does NOT exercise 570'))
        print(f'G2 STRATUM: {len(ex)} of {len(g2_ids)} Tender(s) carry a block 570 moves from null to a lot')
    else:
        summary['g2_stratum'] = None
    summary['read_finished'] = datetime.now(timezone.utc).isoformat(timespec='seconds')
    summary['gets'] = f.gets
    summary['fetch_errors'] = f.errors
    summary['parity_breaks'] = len(summary['breaks'])
    fail, adj = [], []
    if summary['parity_breaks']:
        fail.append(f"{summary['parity_breaks']} parity break(s) in {len(summary['tenders_with_breaks'])} Tender(s)")
    if summary['incomplete']:
        fail.append(f"{len(summary['incomplete'])} Tender(s) incomplete")
    if summary['read'] == 0 or summary['read'] != len(ids):
        fail.append(f"read {summary['read']} of {len(ids)} Tenders")
    for key, what in (('computed_not_served', 'block(s) computed by the replay but not served'),
                      ('served_outside_chain', 'served block(s) whose notice is outside the chain'),
                      ('non_legacy_mismatches', "non-legacy block(s) whose served lot is not their BT-13713"),
                      ('legacy_test_disagrees', 'chain notice(s) whose profile and content-prefix legacy tests disagree'),
                      ('profile_from_prefix', 'Tender(s) whose profiles were not read (legacy from the prefix test)')):
        if summary[key]:
            adj.append(f'adjudicate: {len(summary[key])} {what}')
    summary['parity_verdict'] = 'fail' if fail else ('adjudicate' if adj else 'pass')
    summary['verdict_reasons'] = fail + adj
    summary['pass_bar_i'] = bar and summary['parity_verdict'] == 'pass'
    if not bar:
        summary['pass_bar_i_note'] = 'not a bar run (--bar not given): pass_bar_i is false by construction'
    if out_path:
        with open(out_path, 'w', encoding='utf-8') as fh:
            json.dump(summary, fh, ensure_ascii=False, indent=1)
    print(json.dumps({k: v for k, v in summary.items() if k not in ('breaks', 'g2_stratum')}, ensure_ascii=False))
    print(json.dumps({'breaks': summary['breaks']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
