#!/usr/bin/env python3
"""Issue 513 U1 (decision "unit 2, second", D3): build crates/ingest/sdk/nhs-ods.tsv,
the frozen ODS table that crates/ingest/src/nhs_ods.rs reads.

    python3 -I gen_nhs_ods_table.py [--offline]

Stdlib plus curl (subprocess). Requests are sequential, with a pause between them, and
every response body is cached under raw/ next to this script, so a re-run fetches
nothing. The first run seeds raw/ from the unit-1 and panel caches
(../u1/ods-lists/, ../u1/ods/, ../u2-panel/ods-extra/), which were fetched the same day
and compared equal to a fresh fetch of the RO197/RO98/RO261/RO213 lists. --offline
refuses to fetch and fails on a cache miss.

Responses are external data. Nothing here interprets them beyond the ORD JSON fields
named below.

ORD paging gotcha: `Offset` is a 0-based row index but the API refuses `Offset=0` (406,
"Suppied Offset must be greater than 1") and `Offset=1` skips the first row. So page 0
is fetched WITHOUT an Offset and page k with `Offset=k*Limit`.

The table:
- every organisation whose primary role is RO197 (NHS trust), RO107 (care trust), RO261
  (strategic partnership: ICBs, regions, networks), RO213 (CSU) or RO98 (CCG / sub-ICB
  location), three- and five-character codes, any status. They are all RIVALS for the
  name check;
- RO172 (independent-sector provider) only for the measured code DQF (Calderdale and
  Huddersfield Solutions, the trust's company). AAV, the other measured IS provider
  (5 mentions), stays out, as the decision records.

ALLOW (who may be a survivor; design A's allowlist): Active, RC1, three characters, a
tail in the digit-rule family (letters only, or its only digits are 0), primary role
RO197, RO107 or RO213, RO261 with a Q lead, or DQF. Never RO98, never Inactive. The
legally-ended codes ODS still answers Active (QJG QMJ QMM QNQ QXU RNK RYV) stay.

PREDECESSORS (allow rows only): each code's ORD record, Succs of Type 'Predecessor'
whose own table row is Inactive. Their names count as the code's own names in the check
(RWW: RKC Warrington Hospital, RJS Halton General). An Active predecessor (RY2
Bridgewater under RWW) is a different body still answering, and a predecessor outside
the table (mostly RO111 directly managed units of 1990s vintage) has no name in it;
both are dropped and counted.

Issue 519 unit 2 (U1, decision `.scratch/tender-db/519-nhs-digit-codes/unit2-decision.md`
§2.1-§2.2): the same run also writes crates/ingest/sdk/nhs-ods-children.tsv, the frozen
DATED CHILD MAP, with its own version (CHILDREN_VERSION). nhs-ods.tsv and VERSION do not
change: the run exits non-zero, writing nothing, when nhs-ods.tsv would come out
different from the checked-in bytes (`--rewrite-main` lets a deliberate re-snapshot
through; bump VERSION with it).
- Children: every RO98 row of the main table (CCG / sub-ICB location), keeping ORD
  `Rels.Rel` with id RE5 whose target's primary role is RO261 (the ICB); and the measured
  RO198 trust sites (MEASURED_RO198), keeping RE6 -> RO197 (the trust). Operational dates
  (Legal when a relation has no Operational start). One row per (child, parent, merged
  Operational interval): adjacent intervals with the same parent (end + 1 day == next
  start) are merged. A child with no such parent gets one `rel -` row.
- The child records are seeded into raw/ from 519's caches (measure/ods-ro98/, 344 RO98
  records fetched 2026-10-12, 03K refetched; ods/ for the 25 sites), never from
  ods-table-codes/ (03K's failed fetch lives there).
- Exits non-zero on: a fetch status other than 200 or 404; an RO98 record that disagrees
  with its main-table row (name, status, record class); a parent that is not a row of
  the main table of the expected role; overlapping intervals for one child; a site whose
  RE6 target is not RO197; an RE2 -> RO261 relation with no RE5 twin of the same target
  and dates (RE2 is dropped as a duplicate).
"""
import collections
import datetime
import json
import os
import subprocess
import sys
import time

VERSION = 'ods-2026-10-11'
# Issue 519 unit 2: the child map's own version (the RO98 / RO198 records' fetch date).
CHILDREN_VERSION = 'ods-children-2026-10-12'
BASE = 'https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations'
LIMIT = 1000
LIST_ROLES = ['RO197', 'RO107', 'RO261', 'RO213', 'RO98']
ROLE_NAMES = {
    'RO197': 'NHS TRUST',
    'RO107': 'CARE TRUST',
    'RO261': 'STRATEGIC PARTNERSHIP (ICB, region)',
    'RO213': 'COMMISSIONING SUPPORT UNIT',
    'RO98': 'CLINICAL COMMISSIONING GROUP (sub-ICB location)',
    'RO172': 'INDEPENDENT SECTOR HEALTHCARE PROVIDER (measured codes only)',
}
MEASURED_RO172 = ['DQF']
# Issue 519 unit 2: the trust-site codes unit 1 measured standing as GB-NHS identity orgs
# (primary role RO198 in 519-nhs-digit-codes/gbnhs-orgs-classified.json).
MEASURED_RO198 = [
    'A2X8Q', 'B4Z5Z', 'P9T5R', 'Q3C0R', 'Q7N9Z', 'R1HKH', 'RBT20', 'RCU51', 'RCX70', 'RGQ67', 'RH8C3', 'RH8H6',
    'RJ128', 'RJ759', 'RJN71', 'RNZ91', 'RP401', 'RTK01', 'RTR54', 'RW5NC', 'RX301', 'RXX1Y', 'RY4HQ', 'RYY13',
    'X7V4Q',
]
ALLOW_ROLES = {'RO197', 'RO107', 'RO213'}
LEGALLY_ENDED_ACTIVE = ['QJG', 'QMJ', 'QMM', 'QNQ', 'QXU', 'RNK', 'RYV']

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'raw')
NHS = os.path.dirname(HERE)                                    # .scratch/tender-db/513-nhs
REPO = os.path.dirname(os.path.dirname(os.path.dirname(NHS)))  # the repo root
OUT = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv')
OUT_CHILDREN = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods-children.tsv')
LEGACY_LISTS = os.path.join(NHS, 'u1', 'ods-lists')
LEGACY_ORGS_WRAPPED = os.path.join(NHS, 'u1', 'ods')            # {"code","http","body"}
LEGACY_ORGS_RAW = os.path.join(NHS, 'u2-panel', 'ods-extra')    # the body itself
U519 = os.path.join(os.path.dirname(NHS), '519-nhs-digit-codes')
SEED_RO98 = os.path.join(U519, 'measure', 'ods-ro98')           # {"code","http","body"}
SEED_RO198 = os.path.join(U519, 'ods')                          # {"code","http","body"}
OFFLINE = '--offline' in sys.argv[1:]
REWRITE_MAIN = '--rewrite-main' in sys.argv[1:]
PAUSE = 0.7
fetched = 0


def load(path):
    with open(path, encoding='utf-8') as f:
        return json.load(f)


def save(path, body):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump(body, f, ensure_ascii=False)
    os.replace(tmp, path)


def curl_json(url):
    """GET url through curl; (http status, parsed body or None)."""
    global fetched
    if OFFLINE:
        sys.exit(f'--offline: cache miss for {url}')
    if fetched:
        time.sleep(PAUSE)
    fetched += 1
    tmp = os.path.join(RAW, '.curl.tmp')
    # The relay drops the odd connection (curl 35, "Connection reset by peer"); retry
    # transport failures a few times with a growing pause, never an HTTP answer.
    for attempt in range(5):
        p = subprocess.run(
            ['curl', '-sS', '-m', '120', '-H', 'Accept: application/json',
             '-o', tmp, '-w', '%{http_code}', url],
            capture_output=True, text=True)
        if p.returncode == 0:
            break
        print(f'  curl {p.returncode} for {url} (attempt {attempt + 1}): {p.stderr.strip()}', file=sys.stderr)
        time.sleep(3 * (attempt + 1))
    status = p.stdout.strip()
    if p.returncode != 0:
        sys.exit(f'curl failed ({p.returncode}) for {url}: {p.stderr.strip()}')
    try:
        body = load(tmp)
    except (OSError, ValueError):
        body = None
    finally:
        if os.path.exists(tmp):
            os.remove(tmp)
    return status, body


def list_page(role, page):
    path = os.path.join(RAW, f'list_{role}_p{page}.json')
    if os.path.exists(path):
        return load(path)
    legacy = os.path.join(LEGACY_LISTS, f'list_{role}.json')
    if page == 0 and os.path.exists(legacy):
        body = load(legacy)
        if len(body.get('Organisations', [])) < LIMIT:   # a complete first page
            save(path, body)
            return body
    url = f'{BASE}?PrimaryRoleId={role}&Limit={LIMIT}' + (f'&Offset={page * LIMIT}' if page else '')
    status, body = curl_json(url)
    if status != '200' or not isinstance(body, dict) or 'Organisations' not in body:
        if page and status == '404':
            body = {'Organisations': []}
        else:
            sys.exit(f'list {role} page {page}: HTTP {status} {str(body)[:200]}')
    save(path, body)
    return body


def role_list(role):
    out, page = [], 0
    while True:
        orgs = list_page(role, page).get('Organisations', [])
        out += orgs
        if len(orgs) < LIMIT:
            return out
        page += 1


def org_record(code):
    """The ORD record body for one code: {"Organisation": {...}} or an error body."""
    path = os.path.join(RAW, f'org_{code}.json')
    if os.path.exists(path):
        return load(path)
    wrapped = os.path.join(LEGACY_ORGS_WRAPPED, f'{code}.json')
    rawp = os.path.join(LEGACY_ORGS_RAW, f'{code}.json')
    if os.path.exists(wrapped):
        d = load(wrapped)
        if d.get('http') == '200' and isinstance(d.get('body'), dict) and 'Organisation' in d['body']:
            save(path, d['body'])
            return d['body']
    if os.path.exists(rawp):
        d = load(rawp)
        if isinstance(d, dict) and 'Organisation' in d:
            save(path, d)
            return d
    status, body = curl_json(f'{BASE}/{code}')
    if status == '200' and isinstance(body, dict) and 'Organisation' in body:
        save(path, body)
        return body
    if status == '404':
        body = {'errorCode': 404, 'errorText': 'Not found'}
        save(path, body)
        return body
    sys.exit(f'org {code}: HTTP {status} {str(body)[:200]}')


def primary_role(o):
    prim = [r['id'] for r in o.get('Roles', {}).get('Role', []) if r.get('primaryRole')]
    return prim[0] if prim else None


def digit_rule_family(code):
    """The tails today's digit and all-zero rules refuse: letters only, or only 0s."""
    return all(not ch.isdigit() or ch == '0' for ch in code)


def o0_variants(code):
    pos = [i for i, ch in enumerate(code) if ch in 'O0']
    out = set()
    for mask in range(1, 1 << len(pos)):
        s = list(code)
        for k, i in enumerate(pos):
            if mask >> k & 1:
                s[i] = '0' if s[i] == 'O' else 'O'
        out.add(''.join(s))
    return out


def clean(text):
    if any(ch in text for ch in '\t\r\n'):
        sys.exit(f'a tab or newline in {text!r}')
    return text.strip()


def child_record(code, role):
    """Issue 519 unit 2: a child's ORD record body, seeded into raw/ from 519's caches
    (wrapped {"code","http","body"}) before anything else, then org_record's own path."""
    path = os.path.join(RAW, f'org_{code}.json')
    if os.path.exists(path):
        return load(path)
    seed = os.path.join(SEED_RO98 if role == 'RO98' else SEED_RO198, f'{code}.json')
    if os.path.exists(seed):
        d = load(seed)
        if d.get('http') == '200' and isinstance(d.get('body'), dict) and 'Organisation' in d['body']:
            save(path, d['body'])
            return d['body']
        if d.get('http') == '404':
            body = {'errorCode': 404, 'errorText': 'Not found'}
            save(path, body)
            return body
    return org_record(code)


def rel_dates(rel, typ):
    for d in rel.get('Date', []):
        if d.get('Type') == typ:
            return d.get('Start') or '', d.get('End') or ''
    return '', ''


def children_table(rows):
    """Issue 519 unit 2 (decision §2.1-§2.2): write OUT_CHILDREN, the dated child map."""
    day = datetime.timedelta(days=1)
    date = lambda s: datetime.date.fromisoformat(s) if s else None
    errors, notes = [], []
    dropped = collections.Counter()
    merged_pairs = 0
    out = []        # (child, status, rc, role, rel, parent, parent_role, start, end, legal_end, name)
    children = [(c, 'RO98') for c in sorted(c for c, r in rows.items() if r['role'] == 'RO98')]
    children += [(c, 'RO198') for c in MEASURED_RO198]
    http = collections.Counter()
    for code, role in children:
        body = child_record(code, role)
        o = body.get('Organisation')
        if not o:
            http['404'] += 1
            if role != 'RO98':
                errors.append(f'{code}: a measured site ORD does not know')
                continue
            t = rows[code]
            out.append((code, t['status'][0], t['rc'], role, '-', '', '', '', '', '', t['name']))
            continue
        http['200'] += 1
        prim = primary_role(o)
        if prim != role:
            errors.append(f'{code}: record primary role {prim}, expected {role}')
            continue
        name = clean(o['Name'])
        status, rc = o['Status'], o.get('orgRecordClass')
        if status not in ('Active', 'Inactive'):
            errors.append(f'{code}: status {status}')
            continue
        if role == 'RO98':
            t = rows[code]
            if (name, status, rc) != (t['name'], t['status'], t['rc']):
                errors.append(f'{code}: record {(name, status, rc)} differs from its table row '
                              f'{(t["name"], t["status"], t["rc"])}')
        want_rel, want_role = ('RE5', 'RO261') if role == 'RO98' else ('RE6', 'RO197')
        ivs = collections.defaultdict(list)
        re2 = []
        for r in o.get('Rels', {}).get('Rel', []):
            target_role = r['Target']['PrimaryRoleId']['id']
            target = r['Target']['OrgId']['extension']
            if r['id'] != want_rel or target_role != want_role:
                if role == 'RO198' and r['id'] == 'RE6':
                    errors.append(f'{code}: RE6 target {target} is {target_role}, not RO197')
                if r['id'] == 'RE2' and target_role == 'RO261':
                    re2.append((target, rel_dates(r, 'Operational')))
                dropped[f'{r["id"]}->{target_role}'] += 1
                continue
            start, end = rel_dates(r, 'Operational')
            legal_start, legal_end = rel_dates(r, 'Legal')
            if not start:
                notes.append(f'{code}: {want_rel}->{target} has no Operational start; Legal {legal_start}..{legal_end}')
                start, end = legal_start, legal_end
            if not start:
                errors.append(f'{code}: {want_rel}->{target} has no start date at all')
                continue
            if target not in rows:
                errors.append(f'{code}: parent {target} is not a row of nhs-ods.tsv')
                continue
            if rows[target]['role'] != want_role:
                errors.append(f'{code}: parent {target} is {rows[target]["role"]} in nhs-ods.tsv, not {want_role}')
                continue
            ivs[target].append([date(start), date(end), legal_end])
        # Measured on 2026-10-12: every RE2 -> RO261 relation duplicates an RE5 one exactly.
        for target, (start, end) in re2:
            if not any(iv[0] == date(start) and iv[1] == date(end) for iv in ivs.get(target, [])):
                errors.append(f'{code}: RE2 -> {target} {start}..{end} has no RE5 twin')
        if not ivs:
            out.append((code, status[0], rc, role, '-', '', '', '', '', '', name))
            continue
        spans = []
        for parent, lst in ivs.items():
            lst.sort(key=lambda iv: iv[0])
            joined = [list(lst[0])]
            for iv in lst[1:]:
                last = joined[-1]
                if last[1] is not None and iv[0] == last[1] + day:
                    notes.append(f'{code}: {parent} {last[0]}..{last[1]} + {iv[0]}..{iv[1] or ""} merged (adjacent)')
                    merged_pairs += 1
                    last[1], last[2] = iv[1], iv[2]
                else:
                    joined.append(list(iv))
            for start, end, legal_end in joined:
                spans.append((start, end, parent))
                out.append((code, status[0], rc, role, want_rel, parent, want_role, start.isoformat(),
                            end.isoformat() if end else '', legal_end, name))
        far = datetime.date(9999, 12, 31)
        for i in range(len(spans)):
            for j in range(i + 1, len(spans)):
                a, b = spans[i], spans[j]
                if a[0] <= (b[1] or far) and b[0] <= (a[1] or far):
                    errors.append(f'{code}: {a[2]} {a[0]}..{a[1]} overlaps {b[2]} {b[0]}..{b[1]}')
    if errors:
        for e in errors:
            print('ERROR', e)
        sys.exit(f'{len(errors)} child-map errors; {OUT_CHILDREN} not written')

    out.sort(key=lambda r: (r[0], r[7], r[5]))
    kinds = collections.Counter((r[3], r[4]) for r in out)
    coded = {r[0] for r in out if r[4] != '-'}
    no_parent = sorted(r[0] for r in out if r[4] == '-')
    per_parent = collections.defaultdict(set)
    for r in out:
        if r[5]:
            per_parent[r[5]].add(r[0])
    head = [
        '# nhs-ods-children.tsv: the frozen DATED CHILD MAP of issue 519 unit 2 (decision §2.1).',
        f'# version: {CHILDREN_VERSION}',
        '# Generated by .scratch/tender-db/513-nhs/u1-table/gen_nhs_ods_table.py; do not edit by hand.',
        f'# endpoint: {BASE}/<code> (the ORD record of each child)',
        '# children: every RO98 row of nhs-ods.tsv (CCG / sub-ICB location), and the measured RO198 trust sites',
        '#   (' + ' '.join(MEASURED_RO198) + ').',
        '# parents: RO98 keeps Rels.Rel id RE5 whose target is RO261 (the ICB); RO198 keeps RE6 -> RO197 (the trust).',
        '#   Operational dates (Legal when a relation has none). One row per (child, parent, merged interval):',
        '#   adjacent intervals with one parent are merged. A child with no such parent has one `rel -` row.',
        '# dropped relations: ' + '; '.join(f'{k} {v}' for k, v in sorted(dropped.items()))
        + ' (every RE2 -> RO261 duplicates an RE5 row exactly; the run exits otherwise)',
        f'# counts: rows {len(out)}; '
        + '; '.join(f'{role} {rel} {kinds[(role, rel)]}' for role, rel in sorted(kinds))
        + f'; children with a parent {len(coded)}; without {len(no_parent)}; adjacent intervals merged {merged_pairs}',
        '# columns: child, child_status (A|I), child record class, child_role (RO98|RO198), rel (RE5|RE6|-), parent,',
        '#   parent_role, op_start, op_end (empty = open), legal_end (the relation\'s ORD Legal End, empty when none),',
        '#   child_name',
    ]
    lines = ['\t'.join(r) for r in out]
    tmp = OUT_CHILDREN + '.tmp'
    with open(tmp, 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(head + lines) + '\n')
    os.replace(tmp, OUT_CHILDREN)
    print(f'wrote {OUT_CHILDREN}: {len(out)} rows {dict(kinds)}; records {dict(http)}')
    print(f'dropped relations: {dict(sorted(dropped.items()))}')
    for n in notes:
        print('note', n)
    allow = sorted(p for p in per_parent if rows[p]['allow'])
    print(f'children per allowlisted parent ({len(allow)} parents):')
    for p in allow:
        print(f'  {p}: {" ".join(sorted(per_parent[p]))}')
    print(f'codes with no parent ({len(no_parent)}): {" ".join(no_parent)}')


def main():
    os.makedirs(RAW, exist_ok=True)
    rows = {}
    skipped_len = collections.Counter()
    for role in LIST_ROLES:
        for o in role_list(role):
            code = o['OrgId']
            if o.get('PrimaryRoleId') != role:
                sys.exit(f'{code} listed under {role} with primary role {o.get("PrimaryRoleId")}')
            if len(code) not in (3, 5):
                skipped_len[(role, len(code))] += 1
                continue
            if code in rows:
                sys.exit(f'{code} listed twice')
            rows[code] = dict(name=clean(o['Name']), status=o['Status'], rc=o['OrgRecordClass'], role=role)
    for code in MEASURED_RO172:
        o = org_record(code).get('Organisation')
        if not o or primary_role(o) != 'RO172':
            sys.exit(f'{code}: not an RO172 record')
        rows[code] = dict(name=clean(o['Name']), status=o['Status'], rc=o.get('orgRecordClass'), role='RO172')

    for code, r in rows.items():
        if r['status'] not in ('Active', 'Inactive'):
            sys.exit(f'{code}: status {r["status"]}')
        r['allow'] = (r['status'] == 'Active' and r['rc'] == 'RC1' and len(code) == 3
                      and digit_rule_family(code)
                      and (r['role'] in ALLOW_ROLES or (r['role'] == 'RO261' and code[0] == 'Q')
                           or code in MEASURED_RO172))
        r['preds'] = []
        r['succs'] = []
        r['legal_end'] = ''

    dropped = collections.Counter()
    mismatch = []
    for code in sorted(c for c, r in rows.items() if r['allow']):
        o = org_record(code).get('Organisation')
        if not o:
            sys.exit(f'allow code {code} has no ORD record')
        r = rows[code]
        got = (clean(o['Name']), o['Status'], o.get('orgRecordClass'), primary_role(o))
        if got != (r['name'], r['status'], r['rc'], r['role']):
            mismatch.append((code, got, (r['name'], r['status'], r['rc'], r['role'])))
        preds = set()
        r['legal_end'] = next((d.get('End', '') for d in o.get('Date', []) if d.get('Type') == 'Legal'), '')
        r['succs'] = sorted({s['Target']['OrgId']['extension'] for s in o.get('Succs', {}).get('Succ', [])
                             if s.get('Type') == 'Successor'})
        for s in o.get('Succs', {}).get('Succ', []):
            if s.get('Type') != 'Predecessor':
                continue
            p = s['Target']['OrgId']['extension']
            if p not in rows:
                dropped['not in table'] += 1
            elif rows[p]['status'] != 'Inactive':
                dropped['active'] += 1
            else:
                preds.add(p)
        r['preds'] = sorted(preds)
    if mismatch:
        for m in mismatch:
            print('record differs from its list row:', m)
        sys.exit('list rows and ORD records disagree; refetch both')

    allow = sorted(c for c, r in rows.items() if r['allow'])
    for c in LEGALLY_ENDED_ACTIVE:
        if c not in allow:
            sys.exit(f'legally-ended Active code {c} is not allowlisted')
    twins_all = sorted({tuple(sorted((a, b))) for a in rows for b in o0_variants(a) if b in rows})
    twins_allow = [t for t in twins_all if t[0] in allow and t[1] in allow]
    if twins_allow:
        sys.exit(f'O/0 twins inside the allowlist: {twins_allow}')

    by_role = collections.Counter((r['role'], r['status'], len(c)) for c, r in rows.items())
    allow_by_role = collections.Counter(rows[c]['role'] for c in allow)
    n_preds = sum(len(r['preds']) for r in rows.values())
    with_preds = sum(1 for r in rows.values() if r['preds'])

    head = [
        '# nhs-ods.tsv: the frozen ODS table of issue 513 U1 (decision "unit 2, second", D3).',
        f'# version: {VERSION}',
        '# Generated by .scratch/tender-db/513-nhs/u1-table/gen_nhs_ods_table.py; do not edit by hand.',
        f'# endpoint: {BASE}?PrimaryRoleId=<role>&Limit={LIMIT}[&Offset=<k*{LIMIT}>]',
        f'# endpoint: {BASE}/<code> (RO172 rows; the predecessors of allow rows)',
        '# roles: ' + '; '.join(f'{k} {v}' for k, v in ROLE_NAMES.items()),
        '# rows: every RO197/RO107/RO261/RO213/RO98 organisation, 3- and 5-character, any status (rivals for',
        '#   the name check), plus RO172 for the measured code DQF only (AAV stays out).',
        '# allow: Active, RC1, 3 characters, tail letters-only or zero-only (the digit-rule family), primary role',
        '#   RO197, RO107 or RO213, RO261 with a Q lead, or DQF. Never RO98, never Inactive.',
        '# predecessors: allow rows only; ORD Succs of Type Predecessor whose own row is Inactive.',
        '# successors: allow rows only; every ORD Succs of Type Successor (in the table or not).',
        '# legal_end: allow rows only; the ORD record\'s Legal date End (YYYY-MM-DD), empty when open.',
        f'# counts: rows {len(rows)}; allow {len(allow)} '
        + '(' + ', '.join(f'{k} {allow_by_role[k]}' for k in ROLE_NAMES if allow_by_role[k]) + ')'
        + f'; predecessor links {n_preds} over {with_preds} codes'
        + f'; allow rows with a legal end {sum(1 for c in allow if rows[c]["legal_end"])}',
        '# counts by role: ' + '; '.join(
            f'{role} ' + ' '.join(f'{st[0]}{ln}={by_role[(role, st, ln)]}' for st in ('Active', 'Inactive')
                                  for ln in (3, 5) if by_role[(role, st, ln)])
            for role in ROLE_NAMES),
        '# columns: code, status (A|I), record class, primary role, allow (0|1), predecessors (comma-separated),',
        '#   successors (comma-separated), legal_end, name',
    ]
    lines = [
        '\t'.join([c, rows[c]['status'][0], rows[c]['rc'], rows[c]['role'], '1' if rows[c]['allow'] else '0',
                   ','.join(rows[c]['preds']), ','.join(rows[c]['succs']), rows[c]['legal_end'], rows[c]['name']])
        for c in sorted(rows)
    ]
    text = '\n'.join(head + lines) + '\n'
    # Issue 519 unit 2: the main table is frozen under VERSION; the child map is a sibling
    # file, so this run must reproduce nhs-ods.tsv byte for byte (513's parse, its VERSION
    # stamp and its rival set stay exactly as they are).
    if os.path.exists(OUT) and not REWRITE_MAIN:
        with open(OUT, encoding='utf-8', newline='') as f:
            standing = f.read()
        if standing != text:
            sys.exit(f'{OUT} would change under version {VERSION}; nothing written '
                     '(a deliberate re-snapshot passes --rewrite-main and bumps VERSION)')
        print(f'{OUT}: byte-identical (version {VERSION})')
    else:
        tmp = OUT + '.tmp'
        with open(tmp, 'w', encoding='utf-8', newline='\n') as f:
            f.write(text)
        os.replace(tmp, OUT)
        print(f'wrote {OUT}')
    children_table(rows)

    print(f'network requests this run: {fetched}')
    print(f'rows {len(rows)}; allow {len(allow)}')
    for role in ROLE_NAMES:
        parts = {f'{st} {ln}-char': by_role[(role, st, ln)] for st in ('Active', 'Inactive') for ln in (3, 5)
                 if by_role[(role, st, ln)]}
        print(f'  {role:6s} rows {sum(parts.values()):4d} {parts} allow {allow_by_role[role]}')
    print(f'predecessor links kept {n_preds} over {with_preds} codes; dropped {dict(dropped)}')
    ended = sorted(c for c in allow if rows[c]['legal_end'])
    print(f'allow rows with a legal end: {len(ended)}: ' + ' '.join(f'{c}({rows[c]["legal_end"]}->{"/".join(rows[c]["succs"]) or "-"})' for c in ended))
    print(f'codes of other lengths skipped: {dict(skipped_len)}')
    print(f'O/0 twin pairs anywhere in the table: {len(twins_all)} {twins_all[:12]}; inside the allowlist: 0')


if __name__ == '__main__':
    main()
