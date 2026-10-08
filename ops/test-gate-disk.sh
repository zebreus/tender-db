#!/usr/bin/env bash
# Offline pin for ops/gate-disk.sh (issue 475): the gate's free-space preflight. Drives
# the real gate_disk_preflight against stub `df` and `du` on PATH — no cargo, no real
# disk reading, nothing outside a tempdir.
#
# ops/check.sh runs this before it trusts the preflight, the way deploy.sh runs
# test-gate-marker.sh: a preflight that silently answered "fits" would start a build
# that dies mid-link with GATE-EXIT=101 and no FAILED line, the exact face it exists to
# remove.
#
# Every "proceeds" case has its "refuses" twin one KiB away (df's unit), so a check
# hard-wired either way fails here; and the cases vary the axes a fixture tends to hold
# constant (instrument-discipline.md, "both-ways is per-axis"): the threshold's source
# (default, recorded family, env override, a reusable family credited by the inputs
# hash), one filesystem vs two vs one device under two mount points, target/ present vs
# absent, df answering vs erroring vs printing garbage (fail closed), and byte counts
# that are over-long, zero-padded or all zeros. The last section runs a sandboxed copy
# of check.sh itself (stub cargo), so the wiring is pinned by behaviour, not by a grep.
#
# As in test-gate-marker.sh the harness cannot pass by not running its cases: each
# preflight runs in a subshell, and an EXIT trap turns an early exit or a case count
# other than $planned into a failure.
#
# Run: bash ops/test-gate-disk.sh   (exits 0 on success, 1 on any failure)
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
work=$(mktemp -d) && [ -n "$work" ] && [ -d "$work" ] || { echo "FAIL mktemp -d gave no directory"; exit 1; }

planned=78
ran=0
failures=0
finished=0
on_exit() {
    local rc=$?
    rm -rf "$work"
    if [ "$finished" != 1 ]; then
        echo "FAIL the harness ended before its last case (exit $rc after $ran of $planned cases)"
        exit 1
    fi
    exit "$rc"
}
trap on_exit EXIT

pass() { ran=$((ran + 1)); echo "ok   $1"; }
fail() { ran=$((ran + 1)); echo "FAIL $1"; failures=$((failures + 1)); }

# Stub df: `df -Pk <path>` answers from $work/df.mode. Modes: `table` (each line of
# $work/df.table is "<path> <avail KiB> <mount> [<device>]", device defaulting to
# /dev/stub<mount>; the longest path prefix wins), `fail` (exit 1, no output), `garbage`
# (a header and a row whose Available is "-"), `empty` (exit 0, nothing). Any other
# question is refused so a preflight that asked df something else (a different flag
# set) fails loudly instead of reading a default.
mkdir -p "$work/bin"
cat >"$work/bin/df" <<STUB
#!/usr/bin/env bash
[ "\$#" = 2 ] && [ "\$1" = -Pk ] || { echo "stub df: unexpected question: \$*" >&2; exit 3; }
mode=\$(cat "$work/df.mode")
case "\$mode" in
    fail) exit 1 ;;
    empty) exit 0 ;;
    garbage) printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/x 100 50 - 50%% /\n'; exit 0 ;;
esac
case "\$2" in /*) q=\$2 ;; *) q=\$PWD/\$2 ;; esac
best= avail= mount= dev=
while read -r p a m d; do
    case "\$q" in "\$p"|"\$p"/*) [ "\${#p}" -gt "\${#best}" ] && { best=\$p; avail=\$a; mount=\$m; dev=\${d:-/dev/stub\$m}; } ;; esac
done <"$work/df.table"
[ -n "\$best" ] || { echo "stub df: no row for \$2" >&2; exit 1; }
echo "Filesystem 1024-blocks Used Available Capacity Mounted on"
echo "\$dev 999999999 1 \$avail 1% \$mount"
STUB
cat >"$work/bin/du" <<STUB
#!/bin/sh
printf '12G\t%s\n' "\$2"
STUB
# Stub rustc: `rustc -vV` prints $work/rustc.out, or fails when that file is absent.
cat >"$work/bin/rustc" <<STUB
#!/bin/sh
[ "\$*" = -vV ] || { echo "stub rustc: unexpected question: \$*" >&2; exit 3; }
[ -f "$work/rustc.out" ] || exit 1
cat "$work/rustc.out"
STUB
chmod +x "$work/bin/df" "$work/bin/du" "$work/bin/rustc"
echo 'rustc 1.90.0 (stub)' >"$work/rustc.out"
export PATH="$work/bin:$PATH"
unset GATE_DISK_NEED_BYTES GATE_DISK_FAMILY_BYTES GATE_DISK_SCRATCH_BYTES GATE_DISK_RELINK_BYTES CARGO_TARGET_DIR

# shellcheck source=ops/gate-disk.sh
. "$here/gate-disk.sh"

# A fake repository: the preflight hashes its inputs from the current directory.
R="$work/repo" T="$work/repo/target" P="$work/tmp"
mkdir -p "$T" "$P" "$R/crates/a"
printf 'version = 4\n' >"$R/Cargo.lock"
printf '[workspace]\nmembers = ["crates/a"]\n' >"$R/Cargo.toml"
printf '[package]\nname = "a"\n' >"$R/crates/a/Cargo.toml"
KIB13=$((13 * 1024 * 1024))       # the default floor, 13 GiB, in df's KiB
df_mode() { echo "$1" >"$work/df.mode"; }
one_fs() { df_mode table; printf '%s %s /\n' "$work" "$1" >"$work/df.table"; }
two_fs() { df_mode table; printf '%s %s /\n%s %s /tmpfs\n' "$work" "$1" "$P" "$2" >"$work/df.table"; }

out=
run() { out=$( (cd "$R" && gate_disk_preflight "$T" "$P") 2>&1 ); }
proceeds() { if run; then pass "$1"; else fail "$1 — refused: $out"; fi; }
refuses() {
    if run; then fail "$1 — proceeded: $out"; return; fi
    case "$out" in
        "==> GATE REFUSED: disk"*) pass "$1" ;;
        *) fail "$1 — refused without the GATE REFUSED line first: $out" ;;
    esac
}
says() { case "$out" in *"$2"*) pass "$1" ;; *) fail "$1 — no '$2' in: $out" ;; esac; }
with() { local kv=$1; shift; out=$( (cd "$R" && export "$kv" && gate_disk_preflight "$T" "$P") 2>&1 ); }
with_proceeds() { if with "$1"; then pass "$2"; else fail "$2 — refused: $out"; fi; }
with_refuses() {
    if with "$1"; then fail "$2 — proceeded: $out"; return; fi
    case "$out" in "==> GATE REFUSED: disk"*) pass "$2" ;; *) fail "$2 — no GATE REFUSED line first: $out" ;; esac
}

# --- the default floor, one filesystem (this container's shape) -------------------------
one_fs "$KIB13"
proceeds "13 GiB free, nothing recorded: proceeds at the floor"
says "…and reports the free space and the need" "13.0 GiB free on /, needs 13.0 GiB"
one_fs $((KIB13 - 1))
refuses "1 KiB under 13 GiB: refuses"
says "…and names cargo clean as the remedy" "remedy: cargo clean"
says "…and prints du -sh of target/" "target/ holds 12G"
says "…and says cargo was not started" "cargo was NOT started"
one_fs $((12255784))
refuses "the 2026-10-05 reading (11.7 GiB free), no reusable family: refuses"
one_fs "0$KIB13"
proceeds "a zero-padded df Available (0$KIB13 KiB) reads as decimal: proceeds, no octal error"

# --- the threshold's other sources --------------------------------------------------------
echo $((2 * 1073741824)) >"$T/.gate-family-bytes"
one_fs $((3 * 1024 * 1024))
proceeds "recorded 2 GiB family + 1 GiB scratch, 3 GiB free: proceeds"
one_fs $((3 * 1024 * 1024 - 1))
refuses "recorded 2 GiB family, 1 KiB under 3 GiB: refuses"
says "…and names the recorded file as the threshold" ".gate-family-bytes"
echo 'not-a-number' >"$T/.gate-family-bytes"
one_fs $((3 * 1024 * 1024))
refuses "an unreadable recorded size falls back to the 13 GiB floor, not to zero"
# Two appended writes of 12 GiB: 22 digits, which 64-bit arithmetic would wrap negative.
printf '%s%s\n' 12884901888 12884901888 >"$T/.gate-family-bytes"
one_fs $((KIB13 - 1))
refuses "a 22-digit recorded size falls back to the floor (no wrap to a negative need)"
says "…and the need it prints is the floor" "needs 13.0 GiB"
one_fs "$KIB13"
proceeds "…and proceeds at the floor"
echo 08 >"$T/.gate-family-bytes"
one_fs $((KIB13 - 1))
refuses "a recorded '08' (no octal error, under 1 GiB: not believed) refuses at the floor"
says "…without a bash arithmetic error" "needs 13.0 GiB"
echo 0012884901888 >"$T/.gate-family-bytes"
one_fs $((KIB13 - 1))
refuses "a zero-padded recorded 12 GiB reads as 12 GiB: 1 KiB under 13 GiB refuses"
one_fs "$KIB13"
proceeds "…and 13 GiB proceeds"
rm -f "$T/.gate-family-bytes"

one_fs 1024
with_proceeds GATE_DISK_NEED_BYTES=$((1024 * 1024)) "GATE_DISK_NEED_BYTES=1 MiB with 1 MiB free: proceeds"
with_refuses GATE_DISK_NEED_BYTES=$((1024 * 1024 + 1)) "GATE_DISK_NEED_BYTES one byte over 1 MiB free: refuses"
with_proceeds GATE_DISK_NEED_BYTES=0001048576 "GATE_DISK_NEED_BYTES=0001048576 (zero-padded 1 MiB): proceeds, no octal error"
for bad in abc 0 00 -5 1288490188812884901888; do
    with_refuses "GATE_DISK_NEED_BYTES=$bad" "GATE_DISK_NEED_BYTES=$bad: refuses (not a byte count)"
done
one_fs $((50 * 1024 * 1024))
with_refuses GATE_DISK_SCRATCH_BYTES=9223372036854775807 "GATE_DISK_SCRATCH_BYTES=2^63-1: refuses (no wrap), even with 50 GiB free"
with_refuses GATE_DISK_FAMILY_BYTES=00 "GATE_DISK_FAMILY_BYTES=00: refuses"

# --- two filesystems: target/ needs the family, /tmp the scratch --------------------------
two_fs $((12 * 1024 * 1024)) $((1024 * 1024))
proceeds "two filesystems, 12 GiB on target's and 1 GiB on /tmp's: proceeds"
two_fs $((12 * 1024 * 1024 - 1)) $((50 * 1024 * 1024))
refuses "two filesystems, target's 1 KiB short (plenty on /tmp): refuses"
two_fs $((50 * 1024 * 1024)) $((1024 * 1024 - 1))
refuses "two filesystems, /tmp's 1 KiB short (plenty on target's): refuses"
says "…and names /tmp's mount" "on /tmpfs"

# --- one device under two mount points (a bind-mounted /tmp): one pool --------------------
df_mode table
printf '%s %s / /dev/vda\n%s %s /tmp /dev/vda\n' "$work" $((KIB13 - 1)) "$P" $((KIB13 - 1)) >"$work/df.table"
refuses "same device, two mount points, 1 KiB under 13 GiB: refuses (one pool, not 12+1 apart)"
printf '%s %s / /dev/vda\n%s %s /tmp /dev/vda\n' "$work" "$KIB13" "$P" "$KIB13" >"$work/df.table"
proceeds "same device, two mount points, 13 GiB: proceeds"

# --- target/ absent (just after cargo clean): its parent's filesystem is read -------------
rm -rf "$T"
one_fs "$KIB13"
proceeds "target/ absent, 13 GiB free: proceeds"
one_fs $((KIB13 - 1))
refuses "target/ absent, 1 KiB short: refuses"
mkdir -p "$T"

# --- the inputs hash and the reusable-family credit ---------------------------------------
h1=$(cd "$R" && gate_disk_inputs_hash) || h1=
h2=$(cd "$R" && gate_disk_inputs_hash) || h2=
if [ -n "$h1" ] && [ "$h1" = "$h2" ]; then pass "the inputs hash is stable across two reads"; else fail "the inputs hash is stable across two reads ('$h1' vs '$h2')"; fi
printf 'version = 5\n' >"$R/Cargo.lock"
h3=$(cd "$R" && gate_disk_inputs_hash) || h3=
printf 'version = 4\n' >"$R/Cargo.lock"
if [ -n "$h3" ] && [ "$h3" != "$h1" ]; then pass "…and changes when one byte of Cargo.lock changes"; else fail "…and changes when one byte of Cargo.lock changes"; fi
printf '[package]\nname = "b"\n' >"$R/crates/a/Cargo.toml"
h4=$(cd "$R" && gate_disk_inputs_hash) || h4=
printf '[package]\nname = "a"\n' >"$R/crates/a/Cargo.toml"
if [ -n "$h4" ] && [ "$h4" != "$h1" ]; then pass "…and when a member Cargo.toml changes"; else fail "…and when a member Cargo.toml changes"; fi
h5=$(cd "$R" && CARGO_PROFILE_TEST_DEBUG=2 gate_disk_inputs_hash) || h5=
if [ -n "$h5" ] && [ "$h5" != "$h1" ]; then pass "…and when the CARGO_* environment changes"; else fail "…and when the CARGO_* environment changes"; fi
h6=$(cd "$R" && RUST_BACKTRACE=1 RUST_LOG=debug CARGO_TERM_COLOR=always gate_disk_inputs_hash) || h6=
if [ -n "$h6" ] && [ "$h6" = "$h1" ]; then pass "…but NOT when only RUST_BACKTRACE/RUST_LOG/CARGO_TERM_COLOR change (they build nothing)"; else fail "…but NOT when only RUST_BACKTRACE/RUST_LOG/CARGO_TERM_COLOR change ('$h6' vs '$h1')"; fi
rm -f "$work/rustc.out"
if (cd "$R" && gate_disk_inputs_hash >/dev/null); then fail "…and fails (no credit) when rustc -vV fails"; else pass "…and fails (no credit) when rustc -vV fails"; fi
echo 'rustc 1.90.0 (stub)' >"$work/rustc.out"

mkdir -p "$T/debug/deps"
echo "$h1" >"$T/.gate-inputs"
one_fs $((3 * 1024 * 1024))
proceeds "target/ holds the family these inputs build: 2 GiB relinks + 1 GiB scratch, 3 GiB free proceeds"
says "…and says why the need is small" ".gate-inputs matches"
one_fs $((3 * 1024 * 1024 - 1))
refuses "…1 KiB under 3 GiB refuses"
one_fs $((12255784))
proceeds "the 2026-10-05 reading (11.7 GiB) with a reusable family proceeds"
printf 'version = 5\n' >"$R/Cargo.lock"
refuses "the same reading after a Cargo.lock change (a re-hash) refuses"
says "…and says the inputs changed" ".gate-inputs differs"
printf 'version = 4\n' >"$R/Cargo.lock"
rm -rf "$T/debug"
refuses "a matching .gate-inputs with no target/debug/deps gives no credit"
mkdir -p "$T/debug/deps"
rm -f "$work/rustc.out"
refuses "a matching .gate-inputs but rustc failing gives no credit"
echo 'rustc 1.90.0 (stub)' >"$work/rustc.out"
rm -f "$T/.gate-inputs"
GATE_DISK_INPUTS=$h1 gate_disk_record_inputs "$T"
GATE_DISK_INPUTS=$h1 gate_disk_record_inputs "$T"
if [ "$(cat "$T/.gate-inputs")" = "$h1" ]; then pass "gate_disk_record_inputs replaces, never appends (one line after two writes)"; else fail "gate_disk_record_inputs replaces, never appends: $(cat "$T/.gate-inputs")"; fi
rm -rf "$T/debug" "$T/.gate-inputs"

# --- unit 2: cargo clean when the recorded family was built from other inputs --------------
cat >"$work/bin/cargo" <<STUB
#!/bin/sh
echo "\$*" >>"$work/cargo.calls"
STUB
chmod +x "$work/bin/cargo"
cleaned() { rm -f "$work/cargo.calls"; out=$( (cd "$R" && gate_disk_clean_on_rehash "$T") 2>&1 ); [ -f "$work/cargo.calls" ] && grep -qx clean "$work/cargo.calls"; }
record_now() { (cd "$R" && GATE_DISK_MANIFEST=$(gate_disk_inputs_manifest) GATE_DISK_INPUTS=$(gate_disk_inputs_hash) gate_disk_record_inputs "$T"); }
mkdir -p "$T/debug/deps"
record_now
rm -f "$T/.gate-inputs.manifest"
printf 'version = 5\n' >"$R/Cargo.lock"
if cleaned; then fail "a record from before unit 2 (no manifest) never triggers a clean"; else pass "a record from before unit 2 (no manifest) never triggers a clean"; fi
printf 'version = 4\n' >"$R/Cargo.lock"
record_now
if cleaned; then fail "unchanged inputs: no clean"; else pass "unchanged inputs: no clean"; fi
if (cd "$R" && export RUST_BACKTRACE=1 && rm -f "$work/cargo.calls" && gate_disk_clean_on_rehash "$T" 2>/dev/null) && [ ! -f "$work/cargo.calls" ]; then
    pass "only RUST_BACKTRACE differs: no clean"
else
    fail "only RUST_BACKTRACE differs: no clean"
fi
printf 'version = 5\n' >"$R/Cargo.lock"
if cleaned; then pass "a changed Cargo.lock with a family in target/: cargo clean runs"; else fail "a changed Cargo.lock with a family in target/: cargo clean runs — $out"; fi
says "…and the line names the input that moved" "(Cargo.lock)"
if (cd "$R" && export GATE_NO_AUTO_CLEAN=1 && rm -f "$work/cargo.calls" && gate_disk_clean_on_rehash "$T" 2>/dev/null) && [ ! -f "$work/cargo.calls" ]; then
    pass "GATE_NO_AUTO_CLEAN=1 skips the clean"
else
    fail "GATE_NO_AUTO_CLEAN=1 skips the clean"
fi
rm -rf "$T/debug"
if cleaned; then fail "no target/debug/deps (nothing to supersede): no clean"; else pass "no target/debug/deps (nothing to supersede): no clean"; fi
printf 'version = 4\n' >"$R/Cargo.lock"
rm -f "$work/bin/cargo" "$T/.gate-inputs" "$T/.gate-inputs.manifest"

# --- fail closed: a df that cannot answer refuses -----------------------------------------
df_mode fail
refuses "df exits non-zero: refuses"
says "…and says it could not read free space" "could not read free space"
df_mode garbage
refuses "df prints a non-numeric Available: refuses"
df_mode empty
refuses "df prints nothing and exits 0: refuses"

# --- check.sh itself, sandboxed: the preflight decides whether cargo runs -----------------
# A copy of check.sh and the scripts it sources in a scratch repository. cargo is a stub
# that leaves a mark; python3 (the prune and the scratch sweep) is a no-op, and find
# answers nothing for /tmp, so the sandbox never touches the real /tmp or target/.
# ops/test-gate-disk.sh inside the sandbox is a stub (so this does not recurse) whose
# verdict a case sets.
S="$work/sandbox"
mkdir -p "$S/ops" "$work/sbin"
cp "$here/check.sh" "$here/gate-disk.sh" "$here/gate-marker.sh" "$S/ops/"
cp "$R/Cargo.lock" "$R/Cargo.toml" "$S/"
cat >"$S/ops/test-gate-disk.sh" <<STUB
#!/bin/sh
exit "\$(cat "$work/selftest.rc")"
STUB
cat >"$work/sbin/cargo" <<STUB
#!/bin/sh
echo "\$*" >>"$work/cargo.ran"
[ "\$1" = clean ] && exit 0
exit "\$(cat "$work/cargo.rc")"
STUB
cat >"$work/sbin/python3" <<'STUB'
#!/bin/sh
cat >/dev/null
exit 0
STUB
real_find=$(command -v find)
cat >"$work/sbin/find" <<STUB
#!/bin/sh
[ "\$1" = /tmp ] && exit 0
exec "$real_find" "\$@"
STUB
chmod +x "$work/sbin/"*
echo 0 >"$work/selftest.rc"
echo 0 >"$work/cargo.rc"
gate() {
    rm -f "$work/cargo.ran"
    out=$(cd "$S" && PATH="$work/sbin:$PATH" GIT_CEILING_DIRECTORIES="$work" TMPDIR="$P" bash ops/check.sh "$@" 2>&1)
    rc=$?
}
expect() {  # expect <rc> <cargo ran: yes|no> <label>
    local did=no
    [ -f "$work/cargo.ran" ] && did=yes
    if [ "$rc" = "$1" ] && [ "$did" = "$2" ]; then pass "$3"; else fail "$3 — exit $rc, cargo ran: $did; output: $out"; fi
}
rm -rf "$S/target"; mkdir -p "$S/target"
one_fs $((KIB13 - 1))
gate; expect 3 no "check.sh: 1 KiB under the floor exits 3 and cargo never starts"
says "…with the GATE REFUSED line naming the shortfall (not a df failure)" "==> GATE REFUSED: disk — 13.0 GiB free on /"
one_fs "$KIB13"
echo 1 >"$work/selftest.rc"
gate; expect 3 no "check.sh: a failing disk self-test exits 3 and cargo never starts"
echo 0 >"$work/selftest.rc"
echo 101 >"$work/cargo.rc"
gate; expect 101 yes "check.sh: room enough, cargo runs (and its red is its own exit)"
if [ ! -e "$S/target/.gate-inputs" ]; then pass "…and a red run records no .gate-inputs"; else fail "…and a red run records no .gate-inputs"; fi
echo 0 >"$work/cargo.rc"
gate; expect 0 yes "check.sh: room enough and cargo green: exit 0"
if [ -s "$S/target/.gate-inputs" ]; then pass "…and the green run records target/.gate-inputs"; else fail "…and the green run records target/.gate-inputs: $out"; fi
mkdir -p "$S/target/debug/deps"
one_fs $((12255784))
gate; expect 0 yes "check.sh: the next gate at 11.7 GiB credits the recorded family and runs"
if [ -s "$S/target/.gate-inputs.manifest" ]; then pass "…and the green run recorded the manifest beside the hash"; else fail "…and the green run recorded the manifest beside the hash"; fi
printf 'version = 9\n' >"$S/Cargo.lock"
one_fs "$KIB13"
gate; expect 0 yes "check.sh: after a Cargo.lock change the gate runs (from a clean target/)"
if [ "$(head -1 "$work/cargo.ran")" = clean ]; then pass "…and its first cargo call is \`cargo clean\`, before the build"; else fail "…and its first cargo call is cargo clean: $(cat "$work/cargo.ran")"; fi
# CARGO_TARGET_DIR and TMPDIR: the preflight reads the filesystems cargo and the tests use.
df_mode table
printf '%s %s /\n%s %s /small /dev/small\n' "$work" $((50 * 1024 * 1024)) "$work/elsewhere" 1024 >"$work/df.table"
mkdir -p "$work/elsewhere/target"
gate_env() { rm -f "$work/cargo.ran"; out=$(cd "$S" && export "$1" && PATH="$work/sbin:$PATH" GIT_CEILING_DIRECTORIES="$work" bash -c 'TMPDIR=${TMPDIR:-'"$P"'} bash ops/check.sh' 2>&1); rc=$?; }
gate_env CARGO_TARGET_DIR="$work/elsewhere/target"; expect 3 no "check.sh: CARGO_TARGET_DIR on a full filesystem refuses (./target's has 50 GiB)"
says "…and names that filesystem" "free on /small (target/)"
gate_env TMPDIR="$work/elsewhere"; expect 3 no "check.sh: TMPDIR on a full filesystem refuses"
says "…and names that filesystem" "free on /small ($work/elsewhere)"

finished=1
if [ "$ran" != "$planned" ]; then
    echo "FAIL ran $ran cases, planned $planned"
    exit 1
fi
if [ "$failures" != 0 ]; then
    echo "$failures of $ran cases FAILED"
    exit 1
fi
echo "all $ran cases passed"
