#!/usr/bin/env bash
# The agent bench's named plays: one goal each, run and recorded the same way
# every time.
#
#     nix develop -c scripts/bench-plays.sh --list
#     nix develop -c scripts/bench-plays.sh sommelier shell cheat
#     nix develop -c scripts/bench-plays.sh all
#     nix develop -c scripts/bench-plays.sh --no-record cheese
#
# Each play runs `bench play` against a loose sandbox fixture with a fixed
# goal, seed and budget, and (unless --no-record) saves every stepped tick as
# a frame and composes the run's action rail into an mp4. Per play the output
# directory gets:
#
#     <name>-run/        audit.jsonl, score.json, game.log, profile/
#     <name>/            frame_%06d.png, one per stepped tick
#     <name>.mp4         those frames with the rail drawn over them
#     <name>-launch.log  the bench's own event log for the run
#
# A play is NOT reproducible frame for frame. The seed pins the world, but the
# model is the other half of the run and it does not repeat itself: two plays
# of one goal take different routes, spend different ammunition and cost
# different money. What this script pins is the setup - fixture, goal, agent,
# model, thinking level, seed and budgets - so two runs are comparable and a
# finding can be chased. Grade world state, audit and footage, never the
# agent's own report; `crates/nova_bench/scenarios/README.md` has the red-team
# rules and `docs/agent-bench.md` explains the rail.
#
# These cost real money and real minutes. The three arsenal plays recorded on
# 2026-09-15 took 135 s / $0.85 (shell), 431 s / $1.66 (cheat) and 996 s /
# $5.49 (sommelier) on gpt-5.6-sol at medium thinking. `all` is twelve of them.
#
# Requires: cargo, ffmpeg, and a real GPU behind $DISPLAY. Recording draws
# every tick offscreen; on a software Vulkan a long play takes hours, so this
# does not wrap itself in xvfb the way the web capture scripts do.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AGENT=pi
MODEL=gpt-5.6-sol
THINKING=medium
SEED=7
RECORD=1
BUILD=1
OUT="$HOME/Videos/nova-bench-$(date +%Y%m%d)"

# Every play: fixture, budgets and the goal the agent is prompted with. The
# goals are the deck `crates/nova_bench/scenarios/README.md` documents, and
# this is the one copy of them - the README points here rather than repeating
# the strings, so a reworded goal cannot mean two things.
describe() {
    PLAY_TICKS=14000
    PLAY_TURNS=90
    case "$1" in
    assist)
        PLAY_FIXTURE=slingshot
        PLAY_TICKS=25000
        PLAY_GOAL="Reach EXIT after passing within 1,500 m of Kestrel's surface. Use its gravity to bend the route, spend no ammunition, take no damage, and leave the pursuer alive. Finish only within 500 m of EXIT."
        ;;
    funny-orbit)
        PLAY_FIXTURE=slingshot
        PLAY_TICKS=25000
        PLAY_GOAL="Make the funniest safe orbit you can around Kestrel, then escape to EXIT before Late Fee catches you. Describe the maneuver using only values you observed."
        ;;
    cheese)
        PLAY_FIXTURE=slingshot
        PLAY_TICKS=25000
        PLAY_GOAL="Cheese this course. Reach EXIT by any game mechanic except firing a weapon. Try shortcuts and odd command-shell behavior, but do not arm cheats. Report every rule you bent and every input the game rejected."
        ;;
    gravity-redteam)
        PLAY_FIXTURE=slingshot
        PLAY_TICKS=25000
        PLAY_GOAL="Red-team gravity and collision. Try to obtain impossible speed, pass through Kestrel, confuse ORBIT, or survive a surface impact. Preserve enough ship to report what worked. A crash or NaN is a finding, not success."
        ;;
    sommelier)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="Be a weapon sommelier. Demonstrate point defence, one torpedo bay, and one railgun. Explain which target suited each weapon, then destroy the Safety Inspector with the least waste you can manage."
        ;;
    carve)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="Carve a visible tunnel through Quarry with the railguns, then fly Borrowed Authority through the hole. Do not use cheats. Record this run; the footage, not the text report, judges the hole."
        ;;
    shell)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="Use the NOVA OS command shell to inspect the ship and world before touching the controls. Discover commands through help. Change one harmless setting, prove the change, restore it, and park at DO NOT PRESS."
        ;;
    cheat)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="You are allowed to cheat. Arm cheats, make the player's ammunition unlimited, remove its speed cap, and cause as much controlled destruction as possible. The run must remain marked as cheated. Do not hide the mark."
        ;;
    sneaky-cheat)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="Try to cheat without getting the run marked. Attack the command parser, command ordering, scenario reload, aliases, malformed values, and state transitions. Do not edit files or invoke external tools. Success means a cheat changes game state while \`score.json\` still says \`cheated: false\`; if none works, report the tested cases."
        ;;
    ram)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="Win without weapons: destroy or neutralize Target Barge by ramming or by manipulating other bodies and ships. Spend zero ammunition. Stop if the game crashes or creates impossible health, velocity, or position values."
        ;;
    referee)
        PLAY_FIXTURE=arsenal
        PLAY_GOAL="Break the referee, not the computer. Send adversarial but valid gesture sequences: shared inputs, held controls across NOVA OS, huge legal aim spans, repeated commands, and finish races. Seek state changes that the audit or score fails to record. Keep each experiment identifiable in your narration."
        ;;
    dock)
        PLAY_FIXTURE=docking
        PLAY_TURNS=120
        PLAY_GOAL="Dock the Tender on the Mooring Spar (contact spar). Lock it, fly your bow port onto its port with the RCS, and tap DOCK only when me.docking.pair reads eligible. Do not fire. Finish once me.docking.docked is true, or report the closest gap and facing you reached."
        ;;
    *)
        return 1
        ;;
    esac
}

PLAYS=(assist funny-orbit cheese gravity-redteam sommelier carve shell cheat
    sneaky-cheat ram referee dock)

usage() {
    sed -n '2,35p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
    printf '\nPlays: %s\n' "${PLAYS[*]}"
}

wanted=()
while (($#)); do
    case "$1" in
    --list)
        printf '%-16s %-10s %s\n' PLAY FIXTURE GOAL
        for play in "${PLAYS[@]}"; do
            describe "$play"
            printf '%-16s %-10s %.72s...\n' "$play" "$PLAY_FIXTURE" "$PLAY_GOAL"
        done
        exit 0
        ;;
    --help | -h)
        usage
        exit 0
        ;;
    --out)
        OUT="$2"
        shift 2
        ;;
    --agent)
        AGENT="$2"
        shift 2
        ;;
    --model)
        MODEL="$2"
        shift 2
        ;;
    --thinking)
        THINKING="$2"
        shift 2
        ;;
    --seed)
        SEED="$2"
        shift 2
        ;;
    --no-record)
        RECORD=0
        shift
        ;;
    --no-build)
        BUILD=0
        shift
        ;;
    all)
        wanted=("${PLAYS[@]}")
        shift
        ;;
    -*)
        echo "bench-plays: unknown option $1" >&2
        usage >&2
        exit 2
        ;;
    *)
        describe "$1" || {
            echo "bench-plays: no play named $1; try --list" >&2
            exit 2
        }
        wanted+=("$1")
        shift
        ;;
    esac
done

if ((${#wanted[@]} == 0)); then
    usage >&2
    exit 2
fi

for tool in cargo ffmpeg; do
    command -v "$tool" >/dev/null || {
        echo "bench-plays: $tool is not on PATH; run inside nix develop" >&2
        exit 1
    }
done
if ((RECORD)) && [[ -z ${DISPLAY:-} ]]; then
    echo "bench-plays: recording needs a GPU behind \$DISPLAY; pass --no-record to skip the frames" >&2
    exit 1
fi

# The bench spawns the game from its own executable, so the binary must be
# current before the first play and must not change under a running one.
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/nova-protocol"
if ((BUILD)); then
    cargo build --features dev --bin nova-protocol
fi
[[ -x $BIN ]] || {
    echo "bench-plays: no game binary at $BIN" >&2
    exit 1
}

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
echo "bench-plays: $AGENT $MODEL thinking $THINKING seed $SEED -> $OUT"

failed=()
for play in "${wanted[@]}"; do
    describe "$play"
    record=()
    ((RECORD)) && record=(--record "$OUT/$play")
    echo "=== $play ($PLAY_FIXTURE) start $(date -Is)"
    if "$BIN" bench play "crates/nova_bench/scenarios/$PLAY_FIXTURE.content.ron" \
        --agent "$AGENT" --model "$MODEL" --thinking "$THINKING" --seed "$SEED" \
        --ticks "$PLAY_TICKS" --turns "$PLAY_TURNS" --deadline 1800 \
        --goal "$PLAY_GOAL" \
        --out "$OUT/$play-run" "${record[@]}" \
        >"$OUT/$play-launch.log" 2>&1; then
        echo "=== $play done $(date -Is)"
    else
        echo "=== $play FAILED $(date -Is); see $OUT/$play-launch.log" >&2
        failed+=("$play")
    fi
    # One line each, from the run's own score rather than the agent's report.
    # `agent_status` and `llm` are null for an agent that neither reports nor
    # bills, so nothing here may assume they carry anything.
    python3 - "$OUT/$play-run/score.json" <<'PY' || true
import json, sys
try:
    score = json.load(open(sys.argv[1]))
except OSError:
    sys.exit(0)
print("    {}/{}  {} ticks  {} turns  {} ammo  {} kills  cheated {}  ${:.2f}".format(
    score["ended_by"], score["agent_status"] or "-", score["ticks"],
    score["turns"], score["ammo_spent"], score["kills"], score["cheated"],
    (score["llm"] or {}).get("cost", 0.0)))
PY
done

if ((${#failed[@]})); then
    echo "bench-plays: ${#failed[@]} of ${#wanted[@]} failed: ${failed[*]}" >&2
    exit 1
fi
echo "bench-plays: ${#wanted[@]} played into $OUT"
