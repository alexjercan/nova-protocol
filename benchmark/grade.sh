#!/usr/bin/env bash
# Grade one result with a grader agent, in a container that holds the answer
# key and nothing else.
#
# The grader gets no repository. It does not need one: the tier 1 key carries a
# verified citation per question, and the tier 2 key carries the ground-truth
# surface lists. Denying it the tree keeps it grading the answer instead of
# re-deriving the answer and grading its own work.
#
# Usage:
#   ./grade.sh <run-id> <persona> <paper>
#   ./grade.sh <run-id> all                # everything in the run that is ungraded
#
# Writes results/<run>/<persona>/<paper>/grade/grades.json.
# Tier 3 has no grader: the mod either lints and loads or it does not. See
# keys/tier3.md for that verdict procedure.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="nova-bench:base"
CREDS="${HOME}/.claude/.credentials.json"

die() { echo "grade.sh: $*" >&2; exit 1; }

[ $# -ge 2 ] || die "usage: $0 <run-id> <persona> <paper> | $0 <run-id> all"
RUN="$1"

if [ "$2" = "all" ]; then
    for d in "$HERE/results/$RUN"/*/*/; do
        [ -d "$d" ] || continue
        paper="$(basename "$d")"
        persona="$(basename "$(dirname "$d")")"
        [ "$paper" = "tier3" ] && continue
        [ -f "$d/grade/grades.json" ] && continue
        "$0" "$RUN" "$persona" "$paper" || echo "grade.sh: $persona/$paper failed" >&2
    done
    exit 0
fi

[ $# -eq 3 ] || die "usage: $0 <run-id> <persona> <paper>"
PERSONA="$2"; PAPER="$3"

RES="$HERE/results/$RUN/$PERSONA/$PAPER"
[ -d "$RES" ] || die "no result at $RES"
[ "$PAPER" = "tier3" ] && die "tier 3 is verdicted by the linter, not a grader; see keys/tier3.md"

GRADE_IN="$(mktemp -d)"
trap 'rm -rf "$GRADE_IN"' EXIT
OUT="$RES/grade"
rm -rf "$OUT"; mkdir -p "$OUT"

if [ "$PAPER" = "tier1" ]; then
    [ -f "$RES/answers.json" ] || die "no answers.json in $RES"
    # The key is filtered to the questions this persona was actually asked, so
    # the grader never marks a persona down for a question it never saw. Same
    # rule `make-papers.py` generates the paper from - one implementation.
    python3 "$HERE/persona_filter.py" \
        "$HERE/keys/tier1.json" "$PERSONA" "$GRADE_IN/key.json"
    cp "$RES/answers.json" "$GRADE_IN/answers.json"
    PAPER_FILE="$HERE/papers/grade-tier1.md"
else
    [ -f "$RES/NOTES.md" ] || die "no NOTES.md in $RES"
    cp "$HERE/keys/tier2.md" "$GRADE_IN/key.md"
    cp "$RES/NOTES.md" "$GRADE_IN/NOTES.md"
    echo "$PAPER" > "$GRADE_IN/task.txt"
    # The key's Channel scope table drops Required surfaces the persona's image
    # never held, the tier 2 form of the question filtering done above.
    echo "$PERSONA" > "$GRADE_IN/persona.txt"

    # Counted from the transcript, never the agent's self-report. The two
    # differ by up to 2x (blind/tier2a self-reported 14 against 28 actual), and
    # the self-report was what fed Cost of arrival for the whole baseline.
    # `parse_transcript` is aggregate.py's, so there is one counter.
    tc() {
        python3 - "$HERE" "$1/transcript.jsonl" <<'PY'
import pathlib, sys
sys.path.insert(0, sys.argv[1])
from aggregate import parse_transcript
p = pathlib.Path(sys.argv[2])
print(parse_transcript(p)["tool_calls"] if p.exists() else "not recorded")
PY
    }
    tc "$RES" > "$GRADE_IN/tool-calls.txt"
    # The owner has no container and no transcript, and works in an editor
    # rather than a tool loop, so this is "not recorded" by construction - not
    # a missing file to be filled in later. See the Cost of arrival rule in the
    # grader's paper: unanchored means null, never an improvised anchor.
    tc "$HERE/results/$RUN/owner/$PAPER" > "$GRADE_IN/owner-tool-calls.txt"
    PAPER_FILE="$HERE/papers/grade-tier2.md"
fi

CRED_ARGS=()
if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
    CRED_ARGS=(-e "ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY")
elif [ -f "$CREDS" ]; then
    CRED_ARGS=(-v "$CREDS:/run/secrets/credentials.json:ro")
else
    die "no ANTHROPIC_API_KEY and no $CREDS"
fi

MODEL_ARGS=()
[ -n "${NOVA_BENCH_MODEL:-}" ] && MODEL_ARGS=(-e "NOVA_BENCH_MODEL=$NOVA_BENCH_MODEL")

echo "==> grading $PERSONA / $PAPER"
docker run --rm \
    --user "$(id -u):$(id -g)" \
    -v "$PAPER_FILE:/paper.md:ro" \
    -v "$GRADE_IN:/grade:ro" \
    -v "$OUT:/out" \
    "${CRED_ARGS[@]}" "${MODEL_ARGS[@]}" \
    -e NOVA_BENCH_PERSONA=grader \
    "$IMAGE"

[ -f "$OUT/grades.json" ] || die "grader produced no grades.json (see $OUT/transcript.jsonl)"
echo "  -> $OUT/grades.json"
