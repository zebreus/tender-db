# shellcheck shell=bash
# The test gate's disk checks (issue 475). Sourced, never run: `ops/check.sh` calls
# gate_disk_preflight before cargo starts, and `ops/test-gate-disk.sh` drives the same
# function against a stubbed `df`/`du`.
#
# Why a preflight. The container's disk filled THROUGH the gate on 2026-09-10, 09-27 and
# 09-29 (issue 475's table): a run that could not fit started anyway and died mid-link,
# and its signature is GATE-EXIT=101 with no `test result: FAILED` line, or
# `ld … signal 7 [Bus error]` — a red that reads like a test or compiler failure. The
# prune at the top of check.sh frees only what is ALREADY superseded; a run that
# re-hashes (the lockfile, `[patch]`, a profile, features, the toolchain) writes a whole
# new family beside the old one, so the peak is two families. This check asks the only
# question that catches every fill mode, including a raw `cargo test` family or a second
# worktree's target/: would this run's build fit right now?
#
# The threshold, in bytes:
#   * GATE_DISK_NEED_BYTES, if set, overrides everything (a positive integer);
#   * else, when target/ already holds the family THESE inputs build — the inputs hash
#     below equals target/.gate-inputs, which the last green gate wrote, and
#     target/debug/deps exists — the run reuses it: GATE_DISK_RELINK_BYTES (default 2 GiB,
#     an estimate: the prune's ~0.5 GiB of sibling executables relinked every gate, plus
#     parallel link outputs) plus the scratch. Without this credit the floor below
#     refused every gate after a green one here: the allowance with target/ empty is
#     ~24.8 GiB, a family takes ~13, so ~11.7 GiB is left (review of 475, 2026-10-05);
#   * else the family size the last green run recorded in target/.gate-family-bytes
#     (unit 3 of 475 writes it) plus GATE_DISK_SCRATCH_BYTES of /tmp scratch;
#   * else GATE_DISK_FAMILY_BYTES (default 12 GiB — one family measured 12.0 GiB,
#     12,840,753,202 bytes of target/debug/deps, on 2026-10-01) plus the scratch.
#   GATE_DISK_SCRATCH_BYTES defaults to 1 GiB (a run leaves ~0.6 GB in /tmp, issue 260).
# So with nothing set, nothing recorded and no reusable family the floor is 13 GiB.
#
# The inputs hash (gate_disk_inputs_hash) covers what sets every crate's metadata hash:
# Cargo.lock, every Cargo.toml (features, `[patch]`, `[profile]`), .cargo/config(.toml),
# rust-toolchain(.toml), `rustc -vV` and the CARGO_*/RUST* environment. When any of them
# differs from the last green run's, the build writes a new family beside the old one,
# so no credit is given. When the hash cannot be computed (no rustc, no sha256sum), no
# credit either. Unit 2 of 475 adds the `cargo clean` on a mismatch.
#
# Byte counts are digits only, at most 15 of them (999 TB) after leading zeros are
# stripped, so the sums below cannot wrap 64-bit arithmetic and "08" is decimal 8, not
# a bash octal error; zero is refused. A recorded family under 1 GiB is not believed
# (one family is 12 GiB) and falls back to the floor, like any other garbage.
#
# target/ and /tmp are one pool when df names the same device OR the same mount point
# (a bind mount shows the device twice under two mount points); then the whole
# threshold must fit in the smaller of the two readings. Two genuinely separate
# filesystems that df names alike (two tmpfs) are treated as one — the stricter answer.
# Otherwise target/'s needs the family and /tmp's the scratch.
#
# Fails CLOSED: a df that errors, prints nothing, or prints a non-numeric Available
# column refuses the run. A preflight that answered "fits" when it could not tell is the
# permissive default docs/agents/instrument-discipline.md ledger #6 forbids.
#
# The refusal's first line starts `==> GATE REFUSED: disk`, which no cargo output
# resembles, and the caller exits non-zero before cargo runs, so GATE-EXIT reads red
# without a `test result: FAILED` line to be mistaken for a test failure.

GATE_DISK_GIB=1073741824

# Echo a positive decimal integer of at most 15 digits (leading zeros stripped), or fail.
_gate_disk_int() {
    local v=${1:-}
    case "$v" in '' | *[!0-9]*) return 1 ;; esac
    v=${v#"${v%%[!0]*}"}
    [ -n "$v" ] && [ "${#v}" -le 15 ] || return 1
    printf '%s\n' "$v"
}

# "<available bytes> <device> <mount point>" for the filesystem holding $1, or fail.
# `df -Pk`: POSIX format, one data row per path (a long device name cannot wrap), 1024-byte
# blocks; field 1 is the device, field 4 Available, field 6 the mount point.
_gate_disk_free() {
    local out row avail dev mount
    out=$(df -Pk "$1" 2>/dev/null) || return 1
    row=$(printf '%s\n' "$out" | awk 'NR==2 {print $4, $1, $6}')
    read -r avail dev mount <<<"$row"
    case "$avail" in '' | *[!0-9]*) return 1 ;; esac
    [ "${#avail}" -le 15 ] && [ -n "$dev" ] && [ -n "$mount" ] || return 1
    printf '%s %s %s\n' "$((10#$avail * 1024))" "$dev" "$mount"
}

_gate_disk_gib() { awk -v b="$1" 'BEGIN { printf "%.1f GiB", b / 1073741824 }'; }

# gate_disk_inputs_hash: the sha256 of what sets the build's metadata hashes, read from
# the current directory (the repository root, where check.sh stands). Fails when it
# cannot read one of them, so a failure means "no credit", never "unchanged".
gate_disk_inputs_hash() {
    local f rv
    rv=$(rustc -vV 2>/dev/null) && [ -n "$rv" ] || return 1
    {
        for f in Cargo.lock .cargo/config.toml .cargo/config rust-toolchain rust-toolchain.toml; do
            if [ -e "$f" ]; then printf '%s %s\n' "$f" "$(sha256sum <"$f" | cut -d' ' -f1)"; else printf '%s absent\n' "$f"; fi
        done
        find . \( -path ./target -o -path ./.git -o -path ./.scratch \) -prune -o -name Cargo.toml -type f -print \
            | LC_ALL=C sort | while read -r f; do printf '%s %s\n' "$f" "$(sha256sum <"$f" | cut -d' ' -f1)"; done
        printf '%s\n' "$rv"
        env | LC_ALL=C grep -E '^(CARGO_|RUST)' | LC_ALL=C sort
    } | sha256sum | cut -d' ' -f1 | grep -xE '[0-9a-f]{64}'
}

# gate_disk_record_inputs [target_dir]: after a GREEN run, record the inputs hash the
# preflight computed (GATE_DISK_INPUTS) in target/.gate-inputs, replacing the old one
# (a temp file and a rename — never an append). No hash, no record.
gate_disk_record_inputs() {
    local target=${1:-target}
    [ -n "${GATE_DISK_INPUTS:-}" ] && [ -d "$target" ] || return 1
    printf '%s\n' "$GATE_DISK_INPUTS" >"$target/.gate-inputs.tmp.$$" \
        && mv -f "$target/.gate-inputs.tmp.$$" "$target/.gate-inputs"
}

# gate_disk_preflight [target_dir] [tmp_dir]
# Returns 0 when this run's build plus its scratch fits; otherwise prints the refusal
# and the remedy to stderr and returns 1. Sets GATE_DISK_INPUTS (empty when the hash
# could not be computed) for gate_disk_record_inputs.
gate_disk_preflight() {
    local target=${1:-${CARGO_TARGET_DIR:-target}} tmp=${2:-${TMPDIR:-/tmp}}
    local family scratch relink need source probe recorded
    local t_free t_dev t_mount p_free p_dev p_mount

    GATE_DISK_INPUTS=$(gate_disk_inputs_hash) || GATE_DISK_INPUTS=

    scratch=$(_gate_disk_int "${GATE_DISK_SCRATCH_BYTES:-$GATE_DISK_GIB}") || {
        echo "==> GATE REFUSED: disk — GATE_DISK_SCRATCH_BYTES='${GATE_DISK_SCRATCH_BYTES:-}' is not a positive byte count (at most 15 digits)" >&2
        return 1
    }
    recorded=
    [ -r "$target/.gate-inputs" ] && recorded=$(tr -d '[:space:]' <"$target/.gate-inputs")
    if [ -n "${GATE_DISK_NEED_BYTES:-}" ]; then
        need=$(_gate_disk_int "$GATE_DISK_NEED_BYTES") || {
            echo "==> GATE REFUSED: disk — GATE_DISK_NEED_BYTES='$GATE_DISK_NEED_BYTES' is not a positive byte count (at most 15 digits)" >&2
            return 1
        }
        family=$need scratch=0 source="GATE_DISK_NEED_BYTES"
    elif [ -n "$GATE_DISK_INPUTS" ] && [ "$recorded" = "$GATE_DISK_INPUTS" ] && [ -d "$target/debug/deps" ]; then
        relink=$(_gate_disk_int "${GATE_DISK_RELINK_BYTES:-$((2 * GATE_DISK_GIB))}") || {
            echo "==> GATE REFUSED: disk — GATE_DISK_RELINK_BYTES='${GATE_DISK_RELINK_BYTES:-}' is not a positive byte count (at most 15 digits)" >&2
            return 1
        }
        family=$relink source="target/ holds the family these inputs build ($target/.gate-inputs matches): relinks + scratch"
    elif [ -r "$target/.gate-family-bytes" ] && family=$(_gate_disk_int "$(tr -d '[:space:]' <"$target/.gate-family-bytes")") \
        && [ "$family" -ge "$GATE_DISK_GIB" ]; then
        source="the family the last green run recorded ($target/.gate-family-bytes) + scratch"
    else
        family=$(_gate_disk_int "${GATE_DISK_FAMILY_BYTES:-$((12 * GATE_DISK_GIB))}") || {
            echo "==> GATE REFUSED: disk — GATE_DISK_FAMILY_BYTES='${GATE_DISK_FAMILY_BYTES:-}' is not a positive byte count (at most 15 digits)" >&2
            return 1
        }
        source="one build family (12.0 GiB measured 2026-10-01) + 1 GiB /tmp scratch"
        [ -z "$recorded" ] || source="$source (target/.gate-inputs differs: the inputs changed since the last green gate)"
    fi
    need=$((family + scratch))
    # Unreachable with 15-digit parts; kept so a future edit that loosens the parse
    # cannot turn a wrapped sum into "fits".
    if [ "$need" -le 0 ] || [ "$need" -lt "$family" ]; then
        echo "==> GATE REFUSED: disk — the threshold arithmetic overflowed ($family + $scratch); cargo was NOT started" >&2
        return 1
    fi

    # target/ may not exist yet (after `cargo clean`); its filesystem is then the one the
    # directory holding it is on.
    probe=$target
    [ -e "$probe" ] || probe=$(dirname "$target")
    if ! read -r t_free t_dev t_mount <<<"$(_gate_disk_free "$probe")" || [ -z "${t_mount:-}" ]; then
        echo "==> GATE REFUSED: disk — could not read free space for $probe (df -Pk failed or printed no number); refusing rather than guessing" >&2
        return 1
    fi
    if ! read -r p_free p_dev p_mount <<<"$(_gate_disk_free "$tmp")" || [ -z "${p_mount:-}" ]; then
        echo "==> GATE REFUSED: disk — could not read free space for $tmp (df -Pk failed or printed no number); refusing rather than guessing" >&2
        return 1
    fi

    local short="" pool
    if [ "$t_mount" = "$p_mount" ] || [ "$t_dev" = "$p_dev" ]; then
        pool=$t_free
        [ "$p_free" -ge "$pool" ] || pool=$p_free
        [ "$pool" -ge "$need" ] || short="$(_gate_disk_gib "$pool") free on $t_mount (target/ and $tmp, one device: $t_dev), needs $(_gate_disk_gib "$need")"
    else
        [ "$t_free" -ge "$family" ] || short="$(_gate_disk_gib "$t_free") free on $t_mount (target/), needs $(_gate_disk_gib "$family")"
        [ -n "$short" ] || [ "$p_free" -ge "$scratch" ] || short="$(_gate_disk_gib "$p_free") free on $p_mount ($tmp), needs $(_gate_disk_gib "$scratch")"
    fi
    if [ -z "$short" ]; then
        printf '==> disk preflight: %s free on %s, needs %s (%s)\n' "$(_gate_disk_gib "$t_free")" "$t_mount" "$(_gate_disk_gib "$need")" "$source"
        return 0
    fi

    local held
    held=$(du -sh "$target" 2>/dev/null | awk '{print $1}')
    {
        echo "==> GATE REFUSED: disk — $short; cargo was NOT started"
        echo "    threshold: $source"
        echo "    target/ holds ${held:-unknown (du failed)} (du -sh $target)"
        echo "    remedy: cargo clean   (removes target/; the next gate builds from clean, ~867 s)"
        echo "            then run ops/check.sh again. Also look for a second worktree's target/"
        echo "            and stale /tmp/tender-db-* scratch. GATE_DISK_NEED_BYTES=<bytes> overrides"
        echo "            the threshold for one run (e.g. the first gate over a target/ built"
        echo "            before target/.gate-inputs existed)."
    } >&2
    return 1
}
