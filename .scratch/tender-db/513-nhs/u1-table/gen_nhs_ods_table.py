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
"""
import collections
import json
import os
import subprocess
import sys
import time

VERSION = 'ods-2026-10-11'
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
ALLOW_ROLES = {'RO197', 'RO107', 'RO213'}
LEGALLY_ENDED_ACTIVE = ['QJG', 'QMJ', 'QMM', 'QNQ', 'QXU', 'RNK', 'RYV']

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'raw')
NHS = os.path.dirname(HERE)                                    # .scratch/tender-db/513-nhs
REPO = os.path.dirname(os.path.dirname(os.path.dirname(NHS)))  # the repo root
OUT = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv')
LEGACY_LISTS = os.path.join(NHS, 'u1', 'ods-lists')
LEGACY_ORGS_WRAPPED = os.path.join(NHS, 'u1', 'ods')            # {"code","http","body"}
LEGACY_ORGS_RAW = os.path.join(NHS, 'u2-panel', 'ods-extra')    # the body itself
OFFLINE = '--offline' in sys.argv[1:]
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
        f'# counts: rows {len(rows)}; allow {len(allow)} '
        + '(' + ', '.join(f'{k} {allow_by_role[k]}' for k in ROLE_NAMES if allow_by_role[k]) + ')'
        + f'; predecessor links {n_preds} over {with_preds} codes',
        '# counts by role: ' + '; '.join(
            f'{role} ' + ' '.join(f'{st[0]}{ln}={by_role[(role, st, ln)]}' for st in ('Active', 'Inactive')
                                  for ln in (3, 5) if by_role[(role, st, ln)])
            for role in ROLE_NAMES),
        '# columns: code, status (A|I), record class, primary role, allow (0|1), predecessors (comma-separated), name',
    ]
    lines = [
        '\t'.join([c, rows[c]['status'][0], rows[c]['rc'], rows[c]['role'], '1' if rows[c]['allow'] else '0',
                   ','.join(rows[c]['preds']), rows[c]['name']])
        for c in sorted(rows)
    ]
    tmp = OUT + '.tmp'
    with open(tmp, 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(head + lines) + '\n')
    os.replace(tmp, OUT)

    print(f'wrote {OUT}')
    print(f'network requests this run: {fetched}')
    print(f'rows {len(rows)}; allow {len(allow)}')
    for role in ROLE_NAMES:
        parts = {f'{st} {ln}-char': by_role[(role, st, ln)] for st in ('Active', 'Inactive') for ln in (3, 5)
                 if by_role[(role, st, ln)]}
        print(f'  {role:6s} rows {sum(parts.values()):4d} {parts} allow {allow_by_role[role]}')
    print(f'predecessor links kept {n_preds} over {with_preds} codes; dropped {dict(dropped)}')
    print(f'codes of other lengths skipped: {dict(skipped_len)}')
    print(f'O/0 twin pairs anywhere in the table: {len(twins_all)} {twins_all[:12]}; inside the allowlist: 0')


if __name__ == '__main__':
    main()
