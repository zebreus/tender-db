# shellcheck shell=bash
# queue_verdict <probe exit status> <probe output> — what a caller of
# tender-db-queue-probe.sh may act on, as exactly one line (issue 459). Sourced, never
# run, by the two callers that must not act on a busy box:
#
#   deploy.sh               after piping the probe over ssh (the status is ssh's, which
#                           passes the remote exit status through, or 255 of its own);
#   tender-db-snapshot.sh   installed beside it in /usr/local/bin by install.sh.
#
# One definition, because the rule is the whole safety of both callers and it used to
# exist twice — once in deploy.sh's `case`, where nothing tested it (review of 459).
# test-watchdogs.sh table-tests this function and drives the snapshot through it.
#
# The verdict:
#   idle                       the probe exited 0 and printed exactly `idle`
#   down                       the probe exited 0 and printed exactly `down`
#   busy <id> <kind> <params>  the probe exited 0 and printed `busy ` and more
#   error <what>               everything else, named. The probe's own `error …` line
#                              with its exit 1; or no answer, an answer over two lines,
#                              a known answer with the wrong exit status (`idle` with
#                              exit 1 is not idle: the probe broke after printing), a
#                              trailing blank or carriage return, anything unforeseen.
# The exit status of the probe is half of its answer, so it is checked first.
#
# It always returns 0 and prints one line: a caller under `set -e` that wrote
# `v=$(queue_verdict …)` would otherwise die on an `error` without saying why. Callers
# proceed ONLY on `idle` or `down`, and must treat anything they do not recognise as
# `error` — the last arm of their `case`, never a default that proceeds.
queue_verdict() {
    local rc=${1:-} out=${2-} flat
    flat=$(printf '%s' "$out" | tr '\r\n\t' '   ' | cut -c1-200)
    case "$out" in
        *$'\n'*|*$'\r'*)
            printf 'error the probe answered more than one line: %s (exit %s)\n' "$flat" "${rc:-?}"
            return 0
            ;;
    esac
    case "$rc:$out" in
        0:idle|0:down)   printf '%s\n' "$out" ;;
        "0:busy "?*)     printf '%s\n' "$out" ;;
        1:"error "?*)    printf '%s\n' "$out" ;;
        *)
            if [ -z "$out" ]; then
                printf 'error no answer (exit %s)\n' "${rc:-?}"
            else
                printf "error unexpected answer '%s' (exit %s)\n" "$flat" "${rc:-?}"
            fi
            ;;
    esac
    return 0
}
