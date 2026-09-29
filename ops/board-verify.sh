#!/usr/bin/env bash
# Runs each open issue's `## Verify` command and prints its output beside the stated done/open
# lines. Decides nothing — the reading is the owner's. See docs/agents/issue-tracker.md.
# Usage: ops/board-verify.sh            every open issue (needs-triage/needs-info/ready-for-agent/REOPENED/…)
#        ops/board-verify.sh --all      every issue that carries a Verify block, closed ones too
#        ops/board-verify.sh 390 393    just those issue numbers, whatever their state
set -u
cd "$(dirname "$0")/.."
python3 - "$@" <<'PY'
import glob, os, re, signal, subprocess, sys
args = [a for a in sys.argv[1:] if not a.startswith('--')]
ALL = '--all' in sys.argv
OPEN = re.compile(r'^Status:\s*\**\s*(needs-triage|needs-info|ready-for-agent|REOPENED|open|BACKLOG|PARKED|DORMANT)\b', re.I)
missing, ran = [], 0
for p in sorted(glob.glob('.scratch/*/issues/*.md')):
    num = re.match(r'(\d+)', p.rsplit('/', 1)[-1]).group(1)
    if args and num not in args:
        continue
    L = open(p).read().split('\n')
    st = next((l for l in L[:8] if l.startswith('Status:')), '')
    if not ALL and not args and not OPEN.match(st):
        continue
    try:
        i = next(i for i, l in enumerate(L) if l.strip() == '## Verify')
    except StopIteration:
        missing.append(num); continue
    cmd = next((l.strip() for l in L[i + 1:i + 8] if l.startswith('    ')), None)
    if not cmd:
        missing.append(num); continue
    expected = [l for l in L[i + 1:i + 40] if re.match(r'- \*\*(done|open)\*\*', l)]
    print(f'\n[{num}] {st[:110]}\n  $ {cmd[:220]}')
    # A build is not a free read (docs/agents/issue-tracker.md): `cargo` here compiles in the
    # shared tree outside ops/check.sh's flags. On 2026-09-29 442's `cargo test` verify ran
    # beside a deploy's gate, waited on the build lock past the timeout, and was still
    # compiling turso's sync kit under its own flags after bash was killed.
    if re.match(r'(\S+=\S+\s+)*cargo\b', cmd):
        out = '(not run: a cargo command is a build, not a check — the gate runs the tests; give this issue a read)'
    else:
        # Own process group, killed whole on timeout: killing bash alone orphans what it started.
        proc = subprocess.Popen(['bash', '-c', cmd], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, start_new_session=True)
        try:
            stdout, _ = proc.communicate(timeout=60)
            out = stdout.strip() or f'(no output, exit {proc.returncode})'
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.communicate()
            out = '(timed out after 60 s; its process group was killed)'
    print('  > ' + out[:600].replace('\n', '\n  > '))
    for e in expected:
        print('  ' + e[:240])
    ran += 1
tail = f'; no `## Verify` on: {", ".join(missing)}' if missing else ''
print(f'\n{ran} verify line(s) run{tail}')
PY
