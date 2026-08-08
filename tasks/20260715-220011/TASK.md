# Generated placeholder thumbnails for the Scenarios picker

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.10.0, menu, scenario, art, tooling

## Story

Every picker-visible scenario shows the SAME image: the six base scenarios use
`self://banner.png` (each carrying a `TODO(20260715-220011)` in its Rust
builder) and the seven webmod scenarios use
`dep://base/textures/asteroid.png`. The details pane therefore tells the player
nothing about what they are about to fly.

Real scenario art is owner work and is NOT generated from the game: a
scenario image may want a drawn or composed look, not a gameplay still, and
in-game capture would mean building an example per scenario (including
mod-owned ones). Owner call 2026-08-04.

So this task ships GOOD PLACEHOLDERS instead: a deterministic python generator
that renders a distinct, on-theme 320x180 image per scenario - phosphor-styled
glitched title text on a dark field, in the NOVA OS look - the same way
`scripts/gen-placeholder-sounds.py` fills the audio gap. The picker stops
looking broken, each scenario is visually distinct, and real art later
overwrites the same paths with no code change.

## Steps

- [x] Inventory every picker-visible scenario and its current thumbnail source
      (6 base builders + 7 webmod `.content.ron`), incl. hidden/campaign
      members and how the picker indents them.
- [x] Write `scripts/gen-scenario-thumbnails.py`: stdlib-only, deterministic,
      one 320x180 PNG per scenario from its title/id, in the phosphor look
      (glitched/offset title text, scanlines, dark field). Reuse the PNG
      encoder already in `scripts/gen-web-screenshots.py` rather than
      writing a second one.
- [x] Write the generated PNGs into the owning asset trees - `self://` for
      base, the mod's own dir for webmods - and point each builder / RON at
      its own file.
- [x] Regenerate base RON (`cargo run -p nova_assets --bin content -- gen`)
      and keep `content_ron_parity` green.
- [x] Report outstanding real art: the coverage report in `20260724-082856`
      lists every scenario still on a generated placeholder, classed `manual`.
      Advisory, never a failure.
- [x] Inspect the picker at its shipped size and confirm each image is legible
      and distinct.

## Definition of Done

- The generator is deterministic: a second run reproduces every file byte for
  byte. (cmd: `nix develop --command python3 scripts/gen-scenario-thumbnails.py --check`)
- Generated base scenario RON matches its Rust builders.
  (test: `content_ron_parity`)
- The coverage report lists the scenarios still awaiting real art as `manual`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner accepts the generated placeholders in the rendered picker.
  (manual: inspect the Scenarios picker at its shipped layout size)

## Notes

- Real per-scenario art stays owner-owned. If a specific scenario later wants
  an in-game still, that gets its own task with an explicit brief ("this view,
  these objects, this vibe"), not a blanket "capture them all".
- Website post-card thumbnails (`thumb-news-*`) are also owner art and are
  NOT this task.
- Overwriting a generated file with real art at the same path must need no
  code change - the same contract `gen-placeholder-sounds.py` has.
- Picker rendering: `crates/nova_menu/src/scenarios.rs`
  (`refresh_scenario_details`, `poll_scenario_thumbnail`); a thumbnail must be
  a plain 2D image (a cubemap is skipped with a warning).
- Schema: `ScenarioConfig::thumbnail: Option<AssetRef<Image>>` in
  `nova_scenario`.
- No longer depends on the example fleet: nothing here is captured in-game.

## Close-out

**What/why.** `scripts/gen-scenario-thumbnails.py` renders one deterministic
320x180 phosphor plate per picker-visible scenario (13: 6 base builders,
gauntlet, 6 Ledger chapters) from a built-in 5x7 bitmap font - title centred and
wrapped, chromatic offset, torn scanlines, CRT scanlines, starfield, dim id
label. Each PNG lands in the OWNING mod's tree and is referenced
`self://thumbnails/<id>.png`; no scenario borrows another mod's art any more.
The PNG encoder is `gen-web-screenshots.py`'s, split into a new `encode_png`
(bytes) with `write_png` as its one-line file wrapper, so `--check` can compare
in memory without a second codec.

**Alternatives.** Rendering a gameplay still per scenario was already ruled out
by the owner (2026-08-04). Marking placeholders with a sidecar or a filename
suffix was rejected: re-rendering and comparing bytes is exact, needs no marker,
and drops a file off the worklist the moment real art overwrites it - which is
also the "no code change" contract this task requires.

**Difficulties.** Two things bit. (1) The base `.content.ron` files are
GENERATED from the Rust builders; hand-editing them looks fine until `content --
gen` runs. Reverted and regenerated properly - only the builders are edited.
(2) `self://` refs must be declared in the owning bundle's `resources`, so both
webmod bundles gained a `resources` list (they had none) plus a version bump and
a CHANGELOG entry; base's list gained the six thumbnails.

**Evidence.**
- `content_ron_parity` (2), `webmods_validation` (2), `gen_portal_gate` (4),
  `content_lint_gate` (2), `boot_loading_gate`, `content_report_gate` (2) PASS.
- `gen-scenario-thumbnails.py --check` PASS (13 match byte for byte); it fails
  MISSING/STALE, verified before the first generate.
- `gen-web-screenshots.py --report` lists all 13 as `manual`, exit 0;
  `--self-test` PASS after the encoder split.
- `cargo fmt --check` and `cargo check --workspace --all-targets` clean.
- Rendered: `screenshot_ui` on Xvfb :99 - the picker shows the Broadside plate
  at its shipped size, legible and on-theme (`target/shots/news-090-scenario-campaigns.png`).

**Review round 1.** The owner confirmed the rendered picker directly, closing
the manual inspection (R1.1), and cut the picker assertion the plan asked for
(R1.4): a test forbidding `banner.png` and requiring one file per scenario
encodes a POLICY, not a correctness property - reusing `banner.png` as a
thumbnail is a legitimate authoring choice, and the test would fail it. The
Step and its DoD line came out with the file. What actually guards the gap is
the advisory coverage report, which lists a scenario with no art of its own and
never gates. The mod-author guide (R1.2), which still taught
`thumbnail: Some("dep://base/banner.png")`, now shows the per-scenario
`self://` form and points at the generator. The remaining MINOR/NITs were
declined as not worth the churn.

**Reflection.** The generator's manifest (`SCENARIOS`) is a second list of
picker-visible scenarios beside the game's own. Nothing enforces that they
agree; the coverage report only reports what `SCENARIOS` names, so a scenario
added to the game and not to the script is invisible to both. Acceptable while
scenario count is small and webmod content is hand-authored; worth revisiting
if it grows.
