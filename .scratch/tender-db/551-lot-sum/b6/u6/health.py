"""Issue 551 B6 kit: read the public /health once, paced through the kit's shared GET stamp.

    python3 -I -B health.py [--label C0|C1|CB-stage1|...]

The GET goes through fetch_b6's LockedHttp on cohort_parity's stamp (at most one GET per 1.5 s across fetch_b6,
confirm_draw and the kit's API band reads), so a cursor read can never crowd a running fetch_b6. Prints rev, the
change cursor, and the last job (kind, outcome, started/finished), and appends the line with its label to
u6/health.log -- the RUNBOOK's C0 / C1 / CB / CA cursors are read here.
Exit 0; 3 no answer (not a 200 JSON body).
"""
import argparse
import json
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402

LOG = os.path.join(K.U6, 'health.log')


def read(http=None):
    code, body = (http or K.api_http()).get('/health')
    if code != 200:
        return None, f'HTTP {code} {(body or b"")[:200]!r}'
    return json.loads(body), None


def summary(d):
    # The public /health (2026-10-21) is flat: {"cursor":"958919213","ok":true,"rev":"…"} -- the cursor is a top-level
    # string and no last_job is served. The nested shape is kept as a fallback (the kit's fake box serves it).
    ch = d.get('checks') or {}
    lj = ch.get('last_job') or {}
    cur = d.get('cursor')
    if cur is None:
        cur = (ch.get('database') or {}).get('cursor')
    return {'rev': d.get('rev'), 'cursor': int(cur) if cur is not None else None, 'last_job_kind': lj.get('kind'),
            'last_job_outcome': lj.get('outcome'), 'last_job_started_at': lj.get('started_at'),
            'last_job_finished_at': lj.get('finished_at')}


def main(argv=None, http=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--label', default='')
    a = ap.parse_args(argv)
    d, err = read(http)
    if d is None:
        print(f'/health: NO ANSWER ({err})')
        return 3
    s = summary(d)
    line = f'{K.now_iso()} {a.label or "-"} ' + ' '.join(f'{k}={v}' for k, v in s.items())
    with open(LOG, 'a', encoding='utf-8') as fh:
        fh.write(line + '\n')
    print(line)
    return 0


if __name__ == '__main__':
    sys.exit(K.run_main(main))
