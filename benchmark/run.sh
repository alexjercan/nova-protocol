#!/usr/bin/env bash
# Run one paper against one persona, in that persona's sealed image.
#
# Usage:
#   ./run.sh <run-id> <persona> <paper>
#   ./run.sh baseline tree tier1
#   ./run.sh baseline docs tier2a
#   ./run.sh baseline modder tier3
#   ./run.sh baseline all tier1          # every persona that is asked this paper
#
# Results land in results/<run-id>/<persona>/<paper>/ :
#   transcript.jsonl   every message and tool call, machine-readable
#   payload.txt        what the persona actually had
#   answers.json       tier 1 only
#   NOTES.md           tier 2 only
#   GAPS.md, salvage-run/   tier 3 only
#   meta.json          self-reported tool calls and confidence
#
# Env:
#   NOVA_BENCH_MODEL   passed to `claude --model` (default claude-opus-5)
#   NOVA_BENCH_EFFORT  passed to `claude --effort` (default medium)
#   ANTHROPIC_API_KEY  used instead of the host credentials file if set

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_PREFIX="nova-bench"
CREDS="${HOME}/.claude/.credentials.json"

# Pinned, not left to the CLI's default: a navigability benchmark is only
# comparable across runs on the same agent, and "whatever the CLI defaulted to
# that week" is not a recorded fact. Override per run via the env vars.
NOVA_BENCH_MODEL="${NOVA_BENCH_MODEL:-claude-opus-5}"
NOVA_BENCH_EFFORT="${NOVA_BENCH_EFFORT:-medium}"

die() { echo "run.sh: $*" >&2; exit 1; }

# Which personas each paper is put to. `owner` is run by hand against the same
# papers; it has no image, so it is not listed here.
paper_personas() {
    case "$1" in
        tier1)               echo "blind docs tree rustdoc" ;;
        tier2a|tier2b|tier2c) echo "blind docs tree rustdoc" ;;
        tier3)               echo "modder" ;;
        *) die "unknown paper: $1" ;;
    esac
}

[ $# -eq 3 ] || die "usage: $0 <run-id> <persona> <paper>"
RUN="$1"; PERSONA="$2"; PAPER="$3"

if [ "$PERSONA" = "all" ]; then
    for p in $(paper_personas "$PAPER"); do
        "$0" "$RUN" "$p" "$PAPER"
    done
    exit 0
fi

# tier 1 papers are generated per persona: a persona is never shown a question
# it is not asked, because seeing the question is itself a hint.
PAPER_FILE="$HERE/papers/$PAPER-$PERSONA.md"
[ -f "$PAPER_FILE" ] || PAPER_FILE="$HERE/papers/$PAPER.md"
[ -f "$PAPER_FILE" ] || die "no paper: papers/$PAPER.md (run ./make-papers.py?)"

OUT="$HERE/results/$RUN/$PERSONA/$PAPER"
[ -e "$OUT" ] && die "$OUT already exists; pick a new run id or remove it"
mkdir -p "$OUT"

# `owner` has no image - it is the unrestricted control and it works in the
# repo. Set the result dir up and hand over the paper; the same files an agent
# would write get filled in by hand, so everything downstream is identical.
if [ "$PERSONA" = "owner" ]; then
    printf '{\n  "run": "%s",\n  "persona": "%s",\n  "paper": "%s",\n  "model": "human",\n  "exit_code": 0\n}\n' \
        "$RUN" "$PERSONA" "$PAPER" > "$OUT/run.json"
    echo "==> $PERSONA works in the repo. No container."
    echo "  paper:  $PAPER_FILE"
    echo "  answer: $OUT/$([ "$PAPER" = tier1 ] && echo answers.json || echo NOTES.md)"
    echo
    echo "  There is no transcript for a hand-entered run, so tool calls are"
    echo "  self-reported only. The report says so on the row."
    exit 0
fi

docker image inspect "$IMAGE_PREFIX:$PERSONA" >/dev/null 2>&1 \
    || die "no image $IMAGE_PREFIX:$PERSONA (run ./sandbox.sh build $PERSONA)"

CRED_ARGS=()
if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
    CRED_ARGS=(-e "ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY")
elif [ -f "$CREDS" ]; then
    CRED_ARGS=(-v "$CREDS:/run/secrets/credentials.json:ro")
else
    die "no ANTHROPIC_API_KEY and no $CREDS"
fi

MODEL_ARGS=(
    -e "NOVA_BENCH_MODEL=$NOVA_BENCH_MODEL"
    -e "NOVA_BENCH_EFFORT=$NOVA_BENCH_EFFORT"
)

echo "==> $PERSONA / $PAPER -> results/$RUN/$PERSONA/$PAPER"

# --rm: the container is the sandbox. Anything the agent writes outside /out is
# gone when it exits, so no run can contaminate the next one.
#
# The network stays up because the agent talks to the API over it. That is the
# one hole left, and it is a small one: the repo is a private GitHub remote and
# the container holds no SSH key or token for it. aggregate.py still flags any
# transcript that shows a clone or a fetch.
set +e
docker run --rm \
    --name "bench-$RUN-$PERSONA-$PAPER" \
    --user "$(id -u):$(id -g)" \
    -v "$PAPER_FILE:/paper.md:ro" \
    -v "$OUT:/out" \
    "${CRED_ARGS[@]}" "${MODEL_ARGS[@]}" \
    "$IMAGE_PREFIX:$PERSONA"
status=$?
set -e

{
    echo "{"
    echo "  \"run\": \"$RUN\","
    echo "  \"persona\": \"$PERSONA\","
    echo "  \"paper\": \"$PAPER\","
    echo "  \"image_built_from\": \"$(docker image inspect -f '{{index .Config.Labels "nova.bench.built_from"}}' "$IMAGE_PREFIX:$PERSONA")\","
    echo "  \"model\": \"$NOVA_BENCH_MODEL\","
    echo "  \"effort\": \"$NOVA_BENCH_EFFORT\","
    echo "  \"exit_code\": $status"
    echo "}"
} > "$OUT/run.json"

exit "$status"
