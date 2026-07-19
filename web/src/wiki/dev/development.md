# Development

## Toolchain

- **Rust nightly**, pinned by `rust-toolchain.toml` (with rustfmt + clippy).
- **NixOS**: `nix develop` gives the toolchain, the `wasm32-unknown-unknown`
  target, all system libs Bevy needs (udev, alsa, vulkan, X11/wayland), and
  `trunk`. Without Nix, install those yourself.

## Everyday commands

```sh
cargo run                         # the game (boots into the main menu)
cargo run --features dev          # + debug tooling (inspector, wireframe)
cargo run --example 08_scenario   # run an example
cargo build --release             # release profile: opt=s, lto, stripped
cargo check && cargo fmt          # before committing
cargo test --workspace            # full suite (CI runs this; skip locally unless asked)
```

Notes that keep the suite honest and fast:

- Use `cargo test --workspace`, never bare `cargo test`: unit tests live in the
  member crates, so the bare form runs almost nothing and gives false comfort.
- `cargo test` takes ONE filter and one `-p` per invocation; separate runs for
  separate filters or packages.
- For a timed headless example run, build first, then time only the run
  (`cargo build --example X --features debug`, then `BCS_AUTOPILOT=1 timeout N
  cargo run --example X ...`). A cold build inside the timeout burns the window.
- Struct-field changes: `cargo check --workspace --all-targets`, or examples and
  tests stay silently broken.

The dev profile uses `opt-level = 1` for our code, `3` for dependencies: slow
first build, fast iteration. `split-debuginfo = "unpacked"` +
`debug = "line-tables-only"` keep link-time RAM around 20 GB instead of 40
(one Bevy-sized binary per test/example target); set `debug = true` temporarily
if you need a debugger.

**Worktree builds**: a fresh sprout worktree has an empty `target/`, so the
first build is a cold Bevy compile. Do NOT point `CARGO_TARGET_DIR` at the main
checkout's cache: both checkouts hold the same crates at the same versions, so
their artifacts overwrite each other and a worktree binary can silently link
the main checkout's code. Accept the cold build.

## Features

- `debug` - the whole `nova_debug` plugin (inspector, wireframe, overlays) plus
  `bevy/track_location`.
- `dev` - alias for `debug`.

### Debug tooling

`cargo run --features dev` compiles in `nova_debug`'s `DebugPlugin`
(`crates/nova_debug/src/lib.rs`), which adds the inspector, the wireframe
toggle, and the section/gravity debug overlays. The overlays are gated on a
`DebugEnabled` resource toggled at runtime with **F11**
(`DEBUG_TOGGLE_KEYCODE`), so they can be flipped off without a rebuild. Note the
feature is spelled `debug`, with `dev` as an alias for it (root `Cargo.toml`);
`--features dev` and `--features debug` are interchangeable.

`DebugPlugin` also binds **F12** (`SCREENSHOT_KEYCODE`,
`crates/nova_debug/src/screenshot.rs`) to a screenshot: it captures the primary
window and saves it to your Downloads directory as `<unix-millis>.png`. The
capture is intentionally not gated on `DebugEnabled`, so it works whether or not
the overlays are shown.

Two debug-only CLI flags exist, both parsed in `src/main.rs` and both compiled
in only under the `debug` feature:

- `--norender` - build the app with rendering off (`editor_app(false)`), for
  headless runs.
- `--debugdump` - print the system schedule graph (via `bevy_mod_debugdump`)
  and exit. It dumps the `Update` schedule (`debugdump` in
  `crates/nova_debug/src/lib.rs`).

## Examples

`examples/` exercises one subsystem each, end to end; this repo prefers
runnable examples over isolated unit tests. The set reads as a curriculum in
four blocks:

- Sections: `01_controller_section` (PD attitude), `02_thruster_section`
  (burn -> thrust + plume shader), `03_hull_section` (damage -> destroy ->
  ship survives), `04_turret_section` and `05_torpedo_section` (the weapon
  test ranges), `06_torpedo_guidance` (PN deep-dive), `07_com_range` (mass
  properties under section destruction).
- Scenario: `08_scenario` (the scenario language - variables, events,
  filters, actions - built in code and asserted live).
- Editor: `09_editor` (the shipped editor flow).
- Playable: `10_playable` (a scenario played through the real input
  pipeline: lock, kill, GOTO, arrive - watched by its own handlers),
  `11_hud_range` (screen-projected HUD indicators, velocity sphere
  included), `12_menu_newgame` (the shipped boot flow).
- Screenshots: `13_screenshot_reel`, `14_screenshot_ui`,
  `15_screenshot_combat`, `16_screenshot_sections`, `17_screenshot_juice`,
  `18_screenshot_orbit` (drive the shipped scenes headless to capture the
  wiki and marketing frames).
- Slice: `19_broadside` (the chapter-two scenario end to end through the
  Scenarios picker: defeat -> Retry reload -> the full act machine -> the
  Victory overlay, all staged on scenario state).

When adding a substantial feature, add or extend the example that drives it.
(Consolidated over time: 01_scene/03_scenario merged into 08_scenario;
02_thruster_shader into 02_thruster_section; 05_directional into
11_hud_range; 10_gameplay into 03_hull_section + 10_playable; 07b_slicer's
subject lives in bevy-common-systems; 04_asteroids' slider tuning tool was
dropped.)

Every example is HARNESSED: it drives itself under `BCS_AUTOPILOT=1`, and
`tests/examples_smoke.rs` (the `HARNESSED_EXAMPLES` list) runs all eighteen
headless as a regression suite - each must reach `Playing` and exit without
panic. The gameplay examples (01-12) additionally carry panic-on-failure
behavior assertions with completion backstops (a stalled script fails instead
of passing vacuously), except `06_torpedo_guidance` and `09_editor`, which
assert at the scenario-load / reach-gameplay level; the screenshot examples
(13-18) drive the shipped scenes to capture frames. Keep list and disk in
sync: a new example joins the list with a harness, or it does not merge.

### Examples as bug pins

When a bug is fixed, prefer pinning it where it lives: a unit/App test for a
system-level mechanism, an example assertion when the bug only manifests in a
composed scene (for example, `12_menu_newgame` runs the shipped boot flow with
the ECS fallback error handler swapped to panic, so unhandled command errors on
those transitions fail CI). An example pin is an autopilot-script assertion
(`.input(...)` closure, staged by elapsed time - see `07_com_range`/`11_hud_range`
for the style); the smoke suite runs it on every push. Caveat: the handler swap
does NOT catch `remove`/`despawn` command warns (they bake in the WARN handler
at queue time).

## Web build

WASM via **Trunk** (`Trunk.toml`, `index.html`):

```sh
trunk serve            # serve on http://localhost:8080
trunk build --release
```

`.cargo/config.toml` sets `--cfg=web_sys_unstable_apis` for wasm; `bevy_rand`
uses its `wasm_js` feature there. Trunk only supports the `release` profile.
The GitHub Pages deploy (`.github/workflows/deploy-page.yaml`) builds the
landing site (`web/`) at the root and the game under `/play/`.

The same sources fan out into three build targets that combine into one
published site:

```mermaid
flowchart LR
  src[Sources]
  src -->|cargo| native[Native game]
  src -->|web build| landing[Landing + wiki]
  src -->|trunk| wasm[Bevy WASM game]
  landing --> pages[GitHub Pages]
  wasm --> pages
  pages --> root["/ (landing)"]
  pages --> play["/play/ (game)"]
```

### Regenerating the web screenshots

The site's `.figure` blocks ship as placeholders; the real screenshots are
captured in-engine and packaged into `web/src/assets/` by
`scripts/gen-web-screenshots.py`. Each figure auto-upgrades to its image at
runtime once the asset exists (progressive enhancement in `web/src/site.ts`), so
no HTML edit is needed - just drop the file in.

Capture (needs a display + GPU; headless CI-style is Xvfb + lavapipe) into a
staging dir, then package into `web/src/assets/`:

```sh
export NOVA_SHOT_DIR=target/reel
BCS_REEL=1                cargo run --example 13_screenshot_reel   --features debug
BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example 14_screenshot_ui     --features debug
BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example 15_screenshot_combat --features debug
python3 scripts/gen-web-screenshots.py   # validate + copy; build composites; write the 44x44 icons
```

The example examples run headless under `BCS_AUTOPILOT`; the reel poses a
free-fly camera per beat and captures 1920x1080 PNGs. The Python step validates
each shot is 16:9, copies it in, builds the composite shots a single capture
cannot make (e.g. `devlog5-radar-stance-slots`, two lock stances side by side)
with a stdlib PNG codec, generates the section icons, and reports which shots
have no capture example yet. Commit the resulting PNGs (they are content, like
`banner.png`). Run `python3 scripts/gen-web-screenshots.py --self-test` to check
the PNG codec (decode/resize/compose) in isolation.

## Performance

Frame-time is measured with `nova_perf` (`crates/nova_perf/`), an env-gated
capture plugin that drives the real gameplay app to `Playing`, warms up, records
the wall-clock delta of every frame for a fixed window, and writes percentile
stats. It is inert unless `NOVA_PERF` is set, so `20_perf_baseline` can carry it
permanently. See the crate docs for the full knob list (`NOVA_PERF_*`).

The sweep script builds the example once and runs it across the heavy scenes x
graphics presets, writing one `<label>.json` per run plus an aggregated
`frametime.csv` into a results dir:

```sh
scripts/perf-baseline.sh gpu               # real GPU, headless Xvfb (default)
scripts/perf-baseline.sh sw                # lavapipe software-raster floor
REPORT=1 scripts/perf-baseline.sh gpu      # + render an HTML report at the end
```

`perf_report` turns a results dir into one self-contained HTML report (inline
CSS + inline SVG, opens offline, attachable to a task/PR): per scene x preset it
shows frame count and window, mean and p50/p95/p99/p999/max frame times, the
derived mean / 1%-low FPS, a bar chart (mean bar + p99 marker, one common scale,
a 60 fps budget line), and - against a baseline dir - per-run deltas so a
regression is obvious:

```sh
cargo run -p nova_perf --bin perf_report -- <results-dir>            # -> <results-dir>/report.html
cargo run -p nova_perf --bin perf_report -- <new-dir> --baseline <old-dir> -o report.html
```

The report reads only the aggregated `frametime.csv` (schema
`nova_perf::CSV_HEADER`), so the reader and the capture writer share one column
contract; a fixture-rendered test (`crates/nova_perf/tests/fixtures/`) pins it.

The web target captures the same way through Trunk; `scripts/perf-web.sh` drives
the wasm build under headless Chromium and scrapes the summary line (no fs in the
browser).

## Versioning and release

- Version: `workspace.package.version` in root `Cargo.toml`; crates inherit it.
- `nova_info::APP_VERSION` comes from the `APP_VERSION` env var via `build.rs`.
- Packaging assets (icons, installer, .app) live under `build/`.

### Cutting a release

Pushing a tag `v[0-9]+.[0-9]+.[0-9]+*` triggers `release-flow`
(`.github/workflows/release.yaml`). Steps, on `master`:

1. Bump `workspace.package.version` in root `Cargo.toml`.
2. Refresh `Cargo.lock`: `cargo metadata --format-version 1 >/dev/null`.
3. Update `CHANGELOG.md` (Keep a Changelog, one concise line per entry):
   promote `[Unreleased]` to `[<version>] - <YYYY-MM-DD>`, leave a fresh empty
   `## [Unreleased]` on top, merge any duplicate section headings that grew
   during the cycle, and update the compare links at the bottom (repoint
   `[unreleased]`, add the new `[<version>]` line).
4. Commit exactly those three files:
   `git add Cargo.toml Cargo.lock CHANGELOG.md && git commit -m "chore(release): vX.Y.Z"`.
5. `git tag vX.Y.Z` (CI reads the tag for the release name).
6. `git push origin master && git push origin vX.Y.Z`.
7. Watch the run (`gh run watch`), then check the GitHub release page and
   consider adding summarized release notes (`gh release edit vX.Y.Z --notes-file ...`).
8. Write or expand the release News post (see "Writing the release news post"
   below) and land it in `web/`; sync any wiki pages the cycle changed (see
   [Keeping docs in sync](../keeping-docs-in-sync/)).

The workflow uploads four assets to a release named after the tag: macOS
universal `.dmg`, Linux `.tar.gz`, Windows `.zip`, and a wasm-opt'd web zip.
It can also be re-run via `workflow_dispatch` with a `version` input.

### Writing the release news post

Every release cycle gets one **News** post on the site (`/news/`, markdown under
`web/src/news/`). News is the merged devlog + release notes: **one post per
FEATURE release** (`v0.1.0`, `v0.2.0`, ... `v0.6.0`). Patch releases do NOT get
their own post - they fold into the parent feature post's `## Point releases`
section (`v0.5.0`'s post covers `v0.5.1` and `v0.5.2`). The terse per-version
list stays in `CHANGELOG.md`; source the post's content from the cycle's
`CHANGELOG.md` sections.

A News post follows the spirit of Factorio's Friday Facts: a narrative lead,
then a handful of feature-by-feature `##` sections written candidly (the
reasoning, the dead-ends, the piece you are proudest of), leaning on screenshots,
and - where a devlog video exists - an optional `## Watch the devlog` companion
near the top (the written highlights must stand on their own; the video is an
extra). Do not just restate the terse `CHANGELOG.md`.

Adding a post touches three places (mirror an existing post such as
`web/src/news/0.5.0.md`):

1. Write the post at `web/src/news/<version>.md` (e.g. `0.6.0.md`). The page
   shell (`newsPostShell` in `web/markdown.js`) renders the H1, the
   `<date> // v<version>` meta line, and the footer (the Discussions prompt plus
   the `CHANGELOG.md` pointer and "All news" link), so the markdown is just the
   body: the H1 (`# vX.Y.0 - <title>`), the lead, the `##` sections, `.figure`
   placeholder blocks for screenshots to capture later, an optional
   `.video-embed` companion, a `.callout.callout--breaking` block for any format
   break, and a closing `## Point releases` section for the cycle's patches.
   Do not add a footer or a `CHANGELOG.md` link yourself - the shell adds them.
2. Register it in `web/webpack.config.js`: add an entry to `NEWS_POSTS`
   (newest-first) with `slug`/`version`/`date`/`description`. The plugin list and
   the `historyApiFallback` rewrite both derive from `NEWS_POSTS`, so no other
   wiring is needed.
3. Add a `.post-card` to `web/src/news.html` at the top of `.post-grid`
   (newest-first): a media thumbnail plus the date/version, title, and one-line
   excerpt. For the thumbnail, use the YouTube thumbnail
   (`https://img.youtube.com/vi/<id>/hqdefault.jpg`) if the release has a video,
   otherwise the `.post-card__ph` placeholder naming `assets/thumb-news-<version>.png`.
4. Rebuild and check it: `cd web && npm run ci` (format check, lint, build).

## Contributing a change

The everyday loop for landing a change:

1. **Branch** off `master`. Work items are tracked as `tasks/` markdown (see
   [Task tracking](#task-tracking) below); check the backlog first.
2. **Build and format**: `cargo check && cargo fmt` before you commit. Do NOT
   run `cargo test` or `cargo clippy` locally unless asked - they are slow and
   CI is the source of truth; when you skip them, say so.
3. **Drive it with an example.** For a substantial feature, add or extend the
   `examples/` example that exercises it, with a harnessed autopilot assertion
   (see [Examples](#examples)) - this repo prefers a runnable example over an
   isolated unit test.
4. **Open a PR.** CI (`.github/workflows/ci.yaml`) runs on every PR and push to
   `master`: `cargo fmt --check`, `cargo clippy --workspace --all-targets
   --features debug`, `cargo test --workspace --features debug`, then the
   windowed `examples_smoke` suite under Xvfb/lavapipe, plus a dependency-license
   gate. All of it must be green to merge.

Commit messages are plain and use ASCII punctuation only. Releases are a
separate, tagged flow (see [Cutting a release](#cutting-a-release)).

## Task tracking

Work items live as markdown under `tasks/` (managed with the `tatr` CLI), so
they are versioned alongside the code. Check the backlog before starting and
close tasks when done. Each task has its own folder holding its `TASK.md` plus
any task-scoped records (`SPIKE.md`, `REVIEW.md`, `RETRO.md`, `NOTES.md`).
Multi-task plans (`docs/plans/`) and the lessons ledger (`docs/LESSONS.md`)
live under `docs/`.
