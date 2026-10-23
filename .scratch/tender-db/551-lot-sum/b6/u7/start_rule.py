"""Issue 551 B6 start rule / D7: may the remaining drain start (or go on) now?

    python3 -I -B start_rule.py --runs-left N [--minutes-per-run 20] [--read-minutes 0] [--fixed-minutes 30]
                                [--switchoff-gate covered|uncovered|from-clean | --switchoff-gate-minutes M]
                                [--switchoff-read-statements S] [--now ISO]

Rule (decision U7 "Start rule", D7; W-on drains decision B6/E6 with W2, W3, W9, W11): do not start unless everything
can finish at least 2 h before the next WRITER TICK; once running, if the drain could overrun into it, switch off before
it. The writer ticks (both computed with zoneinfo Europe/Berlin, as supervisor.rs computes them):
  - the daily chain: 09:35 Europe/Berlin (`next_berlin_tick(now, 9, 35)`): 07:35Z through 2026-10-24 (CEST),
    08:35Z from 2026-10-25 (CET; EU DST ends 10-25 01:00Z) -- the kit used to hard-code 07:35 UTC (W2);
  - the weekly report tick: Sunday 03:10 Europe/Berlin (`REPORT_TICK (0, 3, 10)`): 02:10Z on 10-25 and 11-01. It
    queues ~2 h of jobs on the single queue (censuses, data-quality ~100 min, rehash-probe, build/scan-org-match-keys,
    org-merge-health, audit-fts-ids): a writer window (W3). No segment spans it: the deadline is the EARLIER tick.
While the weekly jobs hold the queue (the 2 h after the weekly tick), anything posted -- the runs' own jobs and a
switch-off's drain-back alike -- waits behind them: the start rule then adds their ~2 h as queue wait (B6).
The estimate is deliberately pessimistic: per run --minutes-per-run (job + project + check; the UPPER bound for a
corpus job, W11) plus --read-minutes (the NEW reads that must finish before the next writer, about 45 min for <= 30
heads with reader, skeptic and third reader; W9), plus --fixed-minutes, plus the switch-off twin's gate: 0 when the
twin is covered (`marker_covers <twin>` COVERS), 35 when it is not, from-clean (cargo clean + a ~35 min gate) when a
gate ended GATE-EXIT != 0 (W13/W13'). Also printed: the latest instant a switch-off may START and still finish before
the binding tick (gate + deploy + drain-back and verify, kitlib.SWITCHOFF_MINUTES, + the weekly queue wait when it
applies, + the switch-off's own box reads: --switchoff-read-statements S costs S x 24 s, the 150/h pace of B5 -- with K
frozen over a corpus segment the switch-off re-reads (K, N] twice, build and verify, so S is about 2 x its E1 pages).
make_wet.py (W mode) and segment.py open enforce this rule before every wet POST; gd5.py prints it after every run as
an advisory for the runs left (never a verdict on the run just checked).
Exit 0 GO; 2 NO-GO (wait for the next gap or, if already running, switch off before the tick).
"""
import argparse
import datetime
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'u6'))
import kitlib as K  # noqa: E402

MINUTES_PER_RUN = 20
FIXED_MINUTES = 30
BERLIN = 'Europe/Berlin'
CHAIN_TICK = (9, 35)                     # supervisor.rs: next_berlin_tick(now, 9, 35), every day
WEEKLY_TICK = (6, 3, 10)                 # supervisor.rs REPORT_TICK (0 = Sunday, 3, 10); Python's weekday(): Sunday = 6
WEEKLY_WINDOW_MINUTES = 120              # A10: the weekly jobs' ~2 h on the single queue
GATE_MINUTES = {'covered': 0, 'uncovered': 35, 'from-clean': 40}   # from-clean: cargo clean + ~35 min (CLAUDE.md)
UTC = datetime.timezone.utc
SWITCHOFF_READ_SECONDS = 24              # one statement per 24 s = 150/h (B5), the switch-off box's pace
CLOCK = None                             # tests only: a fixed datetime that parse_now(None) returns


def _berlin():
    try:
        from zoneinfo import ZoneInfo
        return ZoneInfo(BERLIN)
    except Exception as e:  # noqa: BLE001 - no tzdata means no clock: a tool fault, never a GO
        raise SystemExit(f'start rule: zoneinfo {BERLIN} unavailable ({e}); the writer ticks cannot be computed')


def berlin_at(day, hour, minute):
    """The UTC instant at which Berlin wall-clock reads hour:minute on the civil `day` (a date)."""
    return datetime.datetime(day.year, day.month, day.day, hour, minute, tzinfo=_berlin()).astimezone(UTC)


def next_berlin(now, hour, minute, weekday=None):
    """The first instant strictly after `now` at which Berlin reads hour:minute (on `weekday` only, Monday = 0)."""
    d = now.astimezone(_berlin()).date()
    for k in range(-1, 9):
        day = d + datetime.timedelta(days=k)
        if weekday is not None and day.weekday() != weekday:
            continue
        t = berlin_at(day, hour, minute)
        if t > now:
            return t
    raise AssertionError('a matching Berlin tick exists within 8 days')


def prev_berlin(now, hour, minute, weekday=None):
    """The last instant at or before `now` at which Berlin reads hour:minute (on `weekday` only)."""
    d = now.astimezone(_berlin()).date()
    for k in range(1, -9, -1):
        day = d + datetime.timedelta(days=k)
        if weekday is not None and day.weekday() != weekday:
            continue
        t = berlin_at(day, hour, minute)
        if t <= now:
            return t
    raise AssertionError('a matching Berlin tick exists within 8 days')


def next_chain(now):
    """The next daily chain: 09:35 Europe/Berlin (07:35Z in CEST, 08:35Z in CET)."""
    return next_berlin(now, *CHAIN_TICK)


def next_weekly(now):
    """The next weekly report tick: Sunday 03:10 Europe/Berlin."""
    return next_berlin(now, WEEKLY_TICK[1], WEEKLY_TICK[2], weekday=WEEKLY_TICK[0])


def prev_weekly(now):
    return prev_berlin(now, WEEKLY_TICK[1], WEEKLY_TICK[2], weekday=WEEKLY_TICK[0])


def in_weekly_window(t):
    w = prev_weekly(t)
    return w <= t < w + datetime.timedelta(minutes=WEEKLY_WINDOW_MINUTES)


def parse_now(s):
    if not s:
        return CLOCK or datetime.datetime.now(UTC)
    t = datetime.datetime.fromisoformat(s.replace('Z', '+00:00'))
    return t if t.tzinfo else t.replace(tzinfo=UTC)


def evaluate(runs_left, now=None, minutes_per_run=MINUTES_PER_RUN, fixed_minutes=FIXED_MINUTES,
             switchoff_gate_minutes=None, read_minutes=0, switchoff_read_statements=0):
    """-> dict: go, chain (the next daily chain), weekly (the next weekly tick), tick / tick_kind (the earlier: the
    binding tick), deadline (tick - 2 h), need, finish (with the switch-off gate budgeted), finish_rest (the runs and
    the fixed part only), switchoff_by (the latest switch-off start that still finishes before the tick),
    switchoff_minutes, in_weekly_window."""
    now = now or parse_now(None)
    so = dict(K.SWITCHOFF_MINUTES)
    if switchoff_gate_minutes is not None:
        so['gate'] = switchoff_gate_minutes
    if switchoff_read_statements:
        so['box_reads'] = -(-int(switchoff_read_statements) * SWITCHOFF_READ_SECONDS // 60)
    chain, weekly = next_chain(now), next_weekly(now)
    tick, kind = (chain, 'chain') if chain <= weekly else (weekly, 'weekly')
    deadline = tick - datetime.timedelta(hours=2)
    rest_min = runs_left * (minutes_per_run + read_minutes) + fixed_minutes
    in_win = in_weekly_window(now)
    wait = WEEKLY_WINDOW_MINUTES if in_win else 0
    switchoff_by = tick - datetime.timedelta(minutes=sum(so.values()))
    if in_win or in_weekly_window(switchoff_by):
        so['weekly_queue_wait'] = WEEKLY_WINDOW_MINUTES
        switchoff_by = tick - datetime.timedelta(minutes=sum(so.values()))
    rest = datetime.timedelta(minutes=rest_min + wait)
    need = rest + datetime.timedelta(minutes=so['gate'])
    return {'now': now, 'chain': chain, 'weekly': weekly, 'tick': tick, 'tick_kind': kind, 'deadline': deadline,
            'need': need, 'finish': now + need, 'finish_rest': now + rest, 'go': now + need <= deadline,
            'switchoff_by': switchoff_by, 'switchoff_minutes': so, 'in_weekly_window': in_win,
            'minutes_per_run': minutes_per_run, 'read_minutes': read_minutes}


def line(ev, runs_left):
    f = '%Y-%m-%dT%H:%MZ'
    win = '; INSIDE the weekly jobs\' window (+2 h queue wait)' if ev.get('in_weekly_window') else ''
    return (f'now {ev["now"]:{f}}; next chain {ev["chain"]:{f}} (09:35 Berlin); next weekly tick '
            f'{ev["weekly"]:{f}} (Sun 03:10 Berlin); binding: {ev["tick_kind"]}{win}; must finish by '
            f'{ev["deadline"]:{f}}; {runs_left} run(s) left, estimate {ev["need"]} (switch-off gate budgeted) -> '
            f'{ev["finish"]:{f}}: {"GO" if ev["go"] else "NO-GO"}; a switch-off must START by '
            f'{ev["switchoff_by"]:{f}} ({"+".join(f"{k} {v}" for k, v in ev["switchoff_minutes"].items())} min)')


def add_args(ap):
    """The start-rule options shared by make_wet.py (W mode) and gd5.py (D7)."""
    ap.add_argument('--minutes-per-run', type=int, default=MINUTES_PER_RUN)
    ap.add_argument('--read-minutes', type=int, default=0)
    ap.add_argument('--fixed-minutes', type=int, default=FIXED_MINUTES)
    ap.add_argument('--switchoff-read-statements', type=int, default=0,
                    help='the switch-off\'s own box statements (build + verify over (K, N]; x 24 s at 150/h)')
    g = ap.add_mutually_exclusive_group()
    g.add_argument('--switchoff-gate-minutes', type=int)
    g.add_argument('--switchoff-gate', choices=sorted(GATE_MINUTES))


def gate_of(a):
    if getattr(a, 'switchoff_gate', None):
        return GATE_MINUTES[a.switchoff_gate]
    return getattr(a, 'switchoff_gate_minutes', None)


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--runs-left', type=int, required=True)
    add_args(ap)
    ap.add_argument('--now')
    a = ap.parse_args(argv)
    ev = evaluate(a.runs_left, parse_now(a.now), a.minutes_per_run, a.fixed_minutes, gate_of(a), a.read_minutes,
                  a.switchoff_read_statements)
    print(line(ev, a.runs_left))
    return 0 if ev['go'] else 2


if __name__ == '__main__':
    sys.exit(K.run_main(main))
