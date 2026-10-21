"""Issue 551 B6 U7 start rule / D7: may the remaining drain start (or go on) now?

    python3 -I -B start_rule.py --runs-left N [--minutes-per-run 20] [--fixed-minutes 30]
                                [--switchoff-gate-minutes 35] [--now ISO]

Rule (decision U7 "Start rule", D7): do not start unless everything can finish at least 2 h before the next 07:35Z;
once running, if the drain could overrun into the next 07:35Z chain, switch off before the chain. The estimate is
deliberately pessimistic: per run, dry (~1-2 min) + wet/project (2630 took 2 min for 3,502) + PK reads and checks
(~5 min) + slack = 20 min; plus 30 min fixed (deploy, verify, band seeks); plus the switch-off twin's gate (35 min):
the switch-off rev d617a78 is NOT covered by the gate marker once the flip rev is gated (ops/gate-marker.sh keeps one
marker; it names 84c5118), so a switch-off redeploy re-gates it, and GO budgets that gate too (RUNBOOK §0.6).
Also printed: the latest instant a switch-off may START and still finish before the chain (gate + deploy + drain-back
and verify, kitlib.SWITCHOFF_MINUTES). make_wet.py enforces this rule before every wet JSON; dchecks.py check prints it
after every run. Exit 0 GO; 2 NO-GO (wait for the next gap or, if already running, switch off before the chain).
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


def next_chain(now):
    t = now.replace(hour=7, minute=35, second=0, microsecond=0)
    return t if t > now else t + datetime.timedelta(days=1)


def parse_now(s):
    return (datetime.datetime.fromisoformat(s.replace('Z', '+00:00')) if s
            else datetime.datetime.now(datetime.timezone.utc))


def evaluate(runs_left, now=None, minutes_per_run=MINUTES_PER_RUN, fixed_minutes=FIXED_MINUTES,
             switchoff_gate_minutes=None):
    """-> dict: go, chain, deadline (chain - 2 h), finish (with the switch-off gate budgeted), finish_rest (the runs and
    the fixed part only), switchoff_by (the latest switch-off start that still finishes before the chain)."""
    now = now or datetime.datetime.now(datetime.timezone.utc)
    so = dict(K.SWITCHOFF_MINUTES)
    if switchoff_gate_minutes is not None:
        so['gate'] = switchoff_gate_minutes
    chain = next_chain(now)
    deadline = chain - datetime.timedelta(hours=2)
    rest = datetime.timedelta(minutes=runs_left * minutes_per_run + fixed_minutes)
    need = rest + datetime.timedelta(minutes=so['gate'])
    return {'now': now, 'chain': chain, 'deadline': deadline, 'need': need, 'finish': now + need,
            'finish_rest': now + rest, 'go': now + need <= deadline,
            'switchoff_by': chain - datetime.timedelta(minutes=sum(so.values())), 'switchoff_minutes': so}


def line(ev, runs_left):
    f = '%Y-%m-%dT%H:%MZ'
    return (f'now {ev["now"]:{f}}; next chain {ev["chain"]:{f}}; must finish by {ev["deadline"]:{f}}; {runs_left} run(s) '
            f'left, estimate {ev["need"]} (switch-off gate budgeted) -> {ev["finish"]:{f}}: '
            f'{"GO" if ev["go"] else "NO-GO"}; a switch-off must START by {ev["switchoff_by"]:{f}} '
            f'({"+".join(f"{k} {v}" for k, v in ev["switchoff_minutes"].items())} min)')


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--runs-left', type=int, required=True)
    ap.add_argument('--minutes-per-run', type=int, default=MINUTES_PER_RUN)
    ap.add_argument('--fixed-minutes', type=int, default=FIXED_MINUTES)
    ap.add_argument('--switchoff-gate-minutes', type=int)
    ap.add_argument('--now')
    a = ap.parse_args(argv)
    ev = evaluate(a.runs_left, parse_now(a.now), a.minutes_per_run, a.fixed_minutes, a.switchoff_gate_minutes)
    print(line(ev, a.runs_left))
    return 0 if ev['go'] else 2


if __name__ == '__main__':
    sys.exit(K.run_main(main))
