#!/usr/bin/env bash
# Build the per-persona benchmark images.
#
# Each image is a sealed filesystem holding exactly one information channel.
# There is no honor system and no transcript audit: a persona cannot read what
# is not in its image, because the repo is not mounted and the container has no
# path to it.
#
# Usage:
#   ./sandbox.sh build [persona ...]     # default: all five sandboxed personas
#   ./sandbox.sh list                    # what is built, and from which commit
#   ./sandbox.sh inspect <persona>       # the payload file list
#   ./sandbox.sh clean                   # remove all nova-bench images
#
# `owner` has no image. It works in the repo, and its whole purpose is to be
# the unrestricted control.
#
# NixOS: rustdoc is generated through `nix develop --command cargo`.

set -euo pipefail

SANDBOXED=(blind docs tree rustdoc modder)
IMAGE_PREFIX="nova-bench"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$HERE" rev-parse --show-toplevel)"
COMMIT="$(git -C "$REPO" rev-parse --short HEAD)"

say() { printf '  %s\n' "$*"; }
die() { echo "sandbox.sh: $*" >&2; exit 1; }

# Directories that never enter any image, and the single chokepoint that drops
# them. `tasks/` holds records naming exactly what moved, which would answer
# most of tier 1 outright; `benchmark/` holds this harness, its question sets
# and `keys/`, so shipping it would hand a persona the answer key.
#
# Both the tar payload and TREE.txt go through `repo_files`, so one list covers
# both. Do NOT add a second filter anywhere downstream.
EXCLUDED_PREFIXES=(tasks/ benchmark/)

# The file set is git's, not find's: tracked files plus untracked ones git would
# not ignore. That is what a fresh clone shows. A hand-maintained prune list
# would drift, and drift here silently changes what the benchmark measures.
repo_files() {
    local prefix filter=''
    for prefix in "${EXCLUDED_PREFIXES[@]}"; do
        filter+="${filter:+|}^${prefix}"
    done
    git -C "$REPO" ls-files --cached --others --exclude-standard "$@" \
        | grep -Ev "$filter" \
        | LC_ALL=C sort
}

copy_repo_files() {
    local dest="$1"; shift
    repo_files "$@" | tar -C "$REPO" -cf - -T - | tar -C "$dest" -xf -
}

# ---------------------------------------------------------------- payloads

# blind: the code and the tree, with every .md physically deleted. Previously
# this was "must not read .md" on trust. Now the files are not there.
stage_blind() {
    local p="$1"
    copy_repo_files "$p"
    find "$p" -name '*.md' -type f -delete
    say "$(find "$p" -type f | wc -l) files, $(find "$p" -name '*.rs' | wc -l) .rs, 0 .md"
}

stage_docs() {
    local p="$1"
    for f in AGENTS.md README.md; do
        if [ -f "$REPO/$f" ]; then
            cp "$REPO/$f" "$p/"
            say "$f"
        else
            say "MISSING: $f - note it on the run record"
        fi
    done
    mkdir -p "$p/wiki"
    cp -R "$REPO/web/src/wiki/." "$p/wiki/"
    find "$p" -type d -name tasks -prune -exec rm -rf {} +
    say "wiki/ ($(find "$p/wiki" -name '*.md' | wc -l) pages)"
}

# tree: filenames and nothing else. No line counts - owner ruling 2026-08-07.
# Counts would be a second information channel and would flatter the baseline,
# shrinking the delta this persona exists to measure.
stage_tree() {
    local p="$1"
    {
        echo "# Nova Protocol - file listing"
        echo "# Every file a fresh clone has (git-tracked plus non-ignored),"
        echo "# minus tasks/ and benchmark/. Names only. No file contents exist"
        echo "# in this sandbox; there is nothing else to open."
        echo
        repo_files
    } > "$p/TREE.txt"
    say "TREE.txt ($(wc -l < "$p/TREE.txt") lines)"
}

# rustdoc: the rendered public API. The `src/` pages are stripped - rustdoc
# ships the entire annotated source behind every [source] link, which would
# hand this persona the codebase and collapse it into `blind`.
stage_rustdoc() {
    local p="$1"
    # A dedicated target dir, not the shared `target/`. `--no-deps` governs what
    # cargo GENERATES, not what is already sitting in the doc root - a shared
    # target/doc accumulates every dependency's pages from earlier builds, which
    # buried the 19 workspace crates under 430 others and made a 3 GB image.
    # Deps still compile; only workspace members are documented.
    local docroot="$REPO/target/bench-doc/doc"
    say "cargo doc --workspace --no-deps into target/bench-doc (slow, cached)"
    (cd "$REPO" && CARGO_TARGET_DIR="$REPO/target/bench-doc" \
        nix develop --command cargo doc --workspace --no-deps >&2)
    [ -d "$docroot" ] || die "no rustdoc output at $docroot"

    cp -R "$docroot/." "$p/"
    # rustdoc puts the entire annotated source one click behind every symbol.
    # Left in, this persona would collapse into `blind`.
    rm -rf "$p/src"
    find "$p" -name '*.rs.html' -delete
    # Deleting the pages leaves the [source] hrefs pointing at them, and the
    # href itself carries file AND line - `../src/nova_mod_format/lib.rs.html#139`
    # is a tier 1 answer at the grain the paper asks for. Dead links, but a
    # readable file:line map of the whole public API. Rewrite them to nothing.
    find "$p" -name '*.html' -exec \
        sed -i -E 's#href="[^"]*src/nova_[^"]*\.rs\.html[^"]*"#href="#g' {} +

    local crates
    crates=$(find "$p" -maxdepth 1 -type d -name 'nova_*' | wc -l)
    say "$(find "$p" -name '*.html' | wc -l) pages, $crates crates, src/ stripped"
    [ "$crates" -gt 0 ] || die "no nova_* crate docs staged"
}

stage_modder() {
    local p="$1"
    mkdir -p "$p/wiki/modding"
    cp "$REPO/web/src/wiki/modding.md" "$p/wiki/"
    cp -R "$REPO/web/src/wiki/modding/." "$p/wiki/modding/"
    mkdir -p "$p/webmods"
    cp -R "$REPO/webmods/." "$p/webmods/"
    # Binary thumbnails carry no authoring information.
    find "$p/webmods" -type d -name thumbnails -prune -exec rm -rf {} +
    # A real modder can read the base mod from the game's assets folder, and
    # the base prototype ids exist nowhere else in this channel - the after-run
    # controller-cube swap was unknowable without them (owner ruling,
    # 20260809-213441; GAPS.md gap 3 predicted it verbatim).
    copy_repo_files "$p" assets/base
    say "$(find "$p/wiki" -type f | wc -l) wiki pages, webmods/ ($(find "$p/webmods" -type f | wc -l) files), assets/base/ ($(find "$p/assets" -type f | wc -l) files)"
}

# ---------------------------------------------------------------- build

build_base() {
    echo "==> base runtime"
    docker build -q -f "$HERE/docker/Dockerfile.base" -t "$IMAGE_PREFIX:base" "$HERE/docker" >/dev/null
    say "$IMAGE_PREFIX:base"
}

build_persona() {
    local persona="$1"
    local ctx payload
    ctx="$(mktemp -d)"
    payload="$ctx/payload"
    mkdir -p "$payload"

    echo "==> $persona"
    "stage_$persona" "$payload"

    cp "$HERE/docker/Dockerfile.persona" "$ctx/Dockerfile"
    docker build -q \
        --build-arg "BASE=$IMAGE_PREFIX:base" \
        --build-arg "PERSONA=$persona" \
        --build-arg "BUILT_FROM=$COMMIT" \
        -t "$IMAGE_PREFIX:$persona" "$ctx" >/dev/null
    rm -rf "$ctx"
    say "$IMAGE_PREFIX:$persona (from $COMMIT)"
}

cmd_build() {
    local personas=("$@")
    [ ${#personas[@]} -eq 0 ] && personas=("${SANDBOXED[@]}")
    command -v docker >/dev/null || die "docker not found"
    build_base
    for persona in "${personas[@]}"; do
        # `owner` is checked before the case: the case subject is the padded
        # SANDBOXED list, so an `owner)` arm there could never match.
        if [ "$persona" = owner ]; then
            echo "==> owner works in the repo; no image"
            continue
        fi
        case " ${SANDBOXED[*]} " in
            *" $persona "*) build_persona "$persona" ;;
            *) die "unknown persona: $persona" ;;
        esac
    done
    echo
    echo "Built from $COMMIT. Run one with: ./run.sh <run-id> <persona> <paper>"
}

cmd_list() {
    printf '%-10s %-10s %-10s %s\n' PERSONA COMMIT SIZE CREATED
    docker images --filter "reference=$IMAGE_PREFIX:*" \
        --format '{{.Tag}}|{{.Size}}|{{.CreatedSince}}' \
        | LC_ALL=C sort \
        | while IFS='|' read -r tag size created; do
            # `docker images --format` cannot read labels; inspect can.
            commit="$(docker image inspect "$IMAGE_PREFIX:$tag" \
                --format '{{index .Config.Labels "nova.bench.built_from"}}' 2>/dev/null)"
            printf '%-10s %-10s %-10s %s\n' "$tag" "${commit:--}" "$size" "$created"
        done
}

cmd_inspect() {
    [ $# -eq 1 ] || die "usage: sandbox.sh inspect <persona>"
    docker run --rm --entrypoint find "$IMAGE_PREFIX:$1" /work -type f
}

cmd_clean() {
    local ids
    ids="$(docker images -q --filter "reference=$IMAGE_PREFIX:*" | LC_ALL=C sort -u)"
    [ -n "$ids" ] || { echo "nothing to remove"; return; }
    # shellcheck disable=SC2086
    docker rmi -f $ids
}

case "${1:-}" in
    build)   shift; cmd_build "$@" ;;
    list)    cmd_list ;;
    inspect) shift; cmd_inspect "$@" ;;
    clean)   cmd_clean ;;
    *) sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 2 ;;
esac
