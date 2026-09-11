# Nightly review

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: review

## Scope

Nightly review of 2026-09-10. Window is `--since=midnight` to HEAD
(`b3c6f579c`, Release v0.13.2). 54 commits.

No previous nightly briefing was kept on this machine, so nothing is measured
against a prior run. The repo does hold last night's review task
(`tasks/20260909-230022`), and a large block of today's commits is its fallout;
those fixes are reviewed here on their own merits, not re-adjudicated.

This task reviews only. No edit, no commit, no push, no release.

## Baseline

`nix develop --command cargo check --workspace --all-targets --keep-going`
finished in 34.41 s, green. One warning and it is upstream
(`proc-macro-error2 v2.0.1`, future-incompat). No lane runs the workspace test
suite or Clippy; CI owns both.

Working tree was clean at the start of the run and stays clean.

## Groups

Line counts are text lines only; binary media and `tasks/` proof dumps are
excluded from the bundles. Bundles live in `/tmp/nova-nightly-20260910/<g>/`.

| # | Group | Commits | Lines | Play |
|-|-|-|-|-|
| G1 | Skybox cubemap downscale, bundle error text, v0.13.2 | `924664ddc cdd36fede b3c6f579c` | 252+/71- | yes |
| G2 | Pyre warm-up, section collapse, plate shed, chip budget | `6882fc2c4 3414c6f19 01b368441 e4edd0cbd 7cf2056dc 031c88144 010a6cb25 33dcf22c3 f6e10edb9 8eaa53c36` | 753+/233- | yes |
| G3 | Autopilot GOTO brake, rest-leg deadband, seeded spine erosion | `28a14db97 b16321d12 ecb504b6f f3a78a434` | 355+/65- | no |
| G4 | Capture flow gates, CUTS producer, stale sweep, CI gating | `b5d7f341c fc8bb6e7a ded4b00e4 5453fe51c db671775e` | 293+/44- | no |
| G5 | Comic engine checks, reader document, panel transcripts | `087bde783 8ef9751b6 f92c3f1e7 ee2665579 cf3bd8013 c96b03c11 e6bd754bf 2d376ba43 231db18eb` | 1288+/143- | no |
| G6 | News widget scopes, section trigger, ship export, illustration tests | `174e4148f a8041de57 7dd7cfda7 3b475024b f5a80a2c1` | 793+/72- | no |
| G7 | Opening comic episode: pages, art modules, portraits | `8896f385f` (non-`tasks/` subset) | 1642+/165- | no |
| G8 | Release records, changelog ordering, docs corrections, task recovery | `23414634b f063e6637 421cf100f 67936e962 2b6256622 6e6411d99 77bf539f8 1891ce66d 9cec1efc9 262d74023 8d8fa0ab8 fda61a2eb 10c3b6f62 5f072993d 3b83cd7ad bb3943e86 7b054f109` | 268+/162- | no |

`--play` is granted to G1 and G2 only. G1 changed shipped sky art and the task
that made it says a live render was not possible on the authoring host; G2
changes what a death looks like. G3 is physics with its own tests and no new
visual, G4 through G8 are tooling, web and prose, and none of them should pay
for a rendered lane.

## Findings

Appended as each group is adjudicated.

### Baseline: CI state on master, read from the runs themselves

`origin/master` equals `HEAD` (`b3c6f579c`) - everything today is pushed. Tags
`v0.13.0`, `v0.13.1` and `v0.13.2` all exist.

| Run | Tip | Result |
|-|-|-|
| 34448812488 | `3b83cd7ad` | success |
| 34469714561 | `fda61a2eb` (the **v0.13.1** tree) | **failure** |
| 34489343920 | `f3a78a434` | success |
| 34521413852 | `b3c6f579c` (**v0.13.2**) | 7 jobs green, `probe / systems` and `probe / screenshots` still running at 23:07 |

The v0.13.1 tree failed CI twice over:

- `fmt / clippy / test`, exit 101: `error: call to std::mem::drop with a value
  that does not implement Drop`, `could not compile nova_ship (lib test)`.
  Fixed 3h46m later by `ecb504b6f`.
- `probe / systems`: `system_ship_editor` FAIL - `process_exit 1/1 passes
  failed`, `run_completed timeline truncated (no run_end)`, `invariants_held no
  invariant entries`, `log_clean 1 offending line`. That is the editor Generate
  no-drive crash, fixed 5h13m later by `f3a78a434`.

`release-flow` for `v0.13.1` ran to success on that same tree, so v0.13.1 was
tagged, built and published from a tree CI marked red. Both defects are fixed
on master and neither is in v0.13.2. This is a process finding, not a live one.

### Adjudicator's own measurements, taken before the G1 lanes reported

Both cubemaps really are six 1024 px faces, RGBA8:

| File | Dimensions | On disk now | On disk at `924664ddc^` |
|-|-|-|-|
| `assets/base/textures/cubemap.png` | 1024x6144 | 237,317 B | 1,405,634 B |
| `assets/base/textures/cubemap_alt.png` | 1024x6144 | 1,546,064 B | 8,705,309 B |

1024 x 1024 x 4 x 6 = 24 MiB decoded each, so the changelog's 384 MB -> 24 MB
holds. The download saving is 8,327,562 B = 8.33 MB decimal; the news post says
"8.4 MB lighter", which is 1% generous and not worth an entry.

Nothing else under `assets/` is close: the next largest decoded image is
`assets/base/banner.png` at 6.0 MiB, then `assets/base/textures/asteroid.png` at
2.1 MiB. The example mod's own sky, `assets/mods/example/textures/nebula.png`,
is 64x384 and costs 0.09 MiB, so the sizing advice added to
`web/src/create/scenarios.md` does not contradict the example a modder copies.

Every path `credits/CREDITS.md` names exists, including the ones the commit did
NOT touch (`assets/icons/*.png`, `assets/shaders/*.wgsl`,
`assets/input-prompts/keyboard/Alt/`). The `mods/example/textures/nebula.png` in
the `nova_core` test doc is an ASSET-SERVER path, not a repo path, and resolves
to `assets/mods/example/textures/nebula.png`; it is not stale.

The deployed site serves the new files: `GET
/nova-protocol/play/assets/base/textures/cubemap.png` returns 200,
`content-length: 237317`, `last-modified: Thu, 10 Sep 2026 20:07:20 GMT`. The
`deploy-github-page` run for `v0.13.2` (34521501396) is green, so the fix is
live.

`arrival_eta` in `crates/nova_ship/src/flight/guidance.rs:180` claims "both
guards above already returned" when it folds `FlipEstimate::Unknown` into the
braking arm. Checked: it does guard `closing_speed` AND `brake_accel - g <= 0.0`
at lines 157-162, so `Unknown` is unreachable there and the comment is true.
Not a finding.

### The box is not quiet: no rendered measurement is possible tonight

The performance lane held the measurement slot and found the host already busy:
loadavg 3.50 rising to 10.25, with a `weapon-machines-showcase` capture running
at 79-96% CPU under Xvfb :100 (PID 1503569, another session's work, left
untouched). It skipped the measurement rather than publish a contaminated
number, which is right.

That decides `--play` for the rest of the night: a rendered lane would produce a
skip, not a pass, so G2 does not get one either. Every visual and timing claim in
tonight's report is static. **The one thing this review cannot say is whether the
downscaled sky still LOOKS right in a rendered frame, or whether the web build
now boots on a full GPU.** Both are the claims v0.13.2 is sold on.

### G1 - Skybox downscale, bundle error text, v0.13.2 (`924664ddc cdd36fede b3c6f579c`)

Lanes: correctness, contracts, craft, performance. `--play` granted but not
spent, see above.

**MAJOR - `crates/nova_assets/tests/cubemap_meta.rs:62-72` - nothing pins the
face size, so the 384 MiB regression can come straight back.**

Re-derived. The test asserts `array_layer_count() == 6` and
`image.height() == image.width()`. `crates/nova_core/tests/cubemap_meta_app_config.rs:71-79`
asserts the same two things and nothing more. A re-export of `cubemap.png` from
space-3d at its 4096 px default, with the `.meta` untouched, is still six square
layers: every cubemap test in the tree passes, VRAM returns to 384 MiB, and the
web build resumes quitting at boot on `VK_ERROR_OUT_OF_DEVICE_MEMORY`. The whole
of v0.13.2's headline change is one number that nothing holds. Fix: assert
`image.width() == 1024` beside the layer count, in both files.

**MAJOR - `web/src/create/scenarios.md:54-58` - the modder-facing memory figure
names half the cost.**

Re-derived. The paragraph says "Size the faces for the memory they cost" and
then gives 24 MB at 1024 px and 384 MB at 4096 px - the VRAM half only. All
three shipped sidecars, `assets/base/textures/cubemap.png.meta`,
`cubemap_alt.png.meta` and `assets/mods/example/textures/nebula.png.meta` (the
one a modder copies), set `asset_usage: ("MAIN_WORLD | RENDER_WORLD")`, so the
decoded pixels stay in main memory after upload as well. This commit's OWN
changelog entry says "and as much again of browser heap". A modder budgeting
24 MB pays 48; one who reads the table, picks 2048 px and budgets 96 MB pays
192 MB - in wasm linear memory, which is the budget that broke.

**MINOR - `web/src/news/0.13.0.md:1131` - "downloads 8.4 MB lighter" is
8.33 MB.** Measured twice, once by the lane and once here: 10,110,943 B ->
1,783,381 B, a delta of 8,327,562 B = 8.33 MB decimal or 7.94 MiB. Neither
reading rounds to 8.4.

**Not raised.** The lane also observed that a 1024 px face is 11.4 texels/deg
against a 1080p viewport's 32 px/deg, so the sky is magnified ~2.8x at 1080p and
~5.6x at 4K where it was near 1:1 before, and that Bevy's default linear filter
turns each max-pooled star into a soft blob. That is arithmetic, not a rendered
comparison, and the change deliberately traded it. Recorded, not raised.

**Verified clean by the performance lane.** Boot pays for ONE cubemap, not two:
only `cubemap` is in the collection (`crates/nova_assets/src/collections.rs:187`),
`cubemap_alt` is a bundle RESOURCE and `BundleAsset::visit_dependencies` visits
only `content` handles, so the alt sky loads on demand from `SetSkybox`. The
`register_bundles` change costs nothing on a good frame: the new `path()`
allocation is inside the let-else miss branch only. Across all 133 PNGs under
`assets/` (68.95 MiB decoded) nothing else is close, and the largest single
download is now `assets/fonts/SGr-IosevkaTerm-Medium.ttf` at 10.8 MB, six times
both cubemaps put together.

**MAJOR - `credits/CREDITS.md:28` - the licence list points at a directory that
no longer exists, in the very commit that swept that list.**

Re-derived. Line 28 reads ``- **3D models** (`assets/gltf/*.glb`) - exported
from the project's own Blender sources``. `ls assets/gltf` fails; the 21 `.glb`
files are under `assets/base/gltf/`, which the greebles line five rows below
already spells correctly. `924664ddc` fixed `assets/textures/` ->
`assets/base/textures/` and `assets/banner.png` -> `assets/base/banner.png` in
the same five lines and left this one. `CREDITS.md` is copied into `dist/credits/`
and shipped beside the executable, and this is the entry that says which files
carry the project's own MIT licence, so a licence audit follows a dead path.

**MINOR - `art/README.md:13` - the same stale path.** ``the runtime
`assets/gltf/*.glb` models``. Those two lines are the only surviving references
in the tree outside `tasks/`; both were checked with
`grep -rn "assets/gltf/"`.

**MINOR - `crates/nova_assets/src/merge.rs:226` - the new player-facing error
names a menu item the menu does not have.** It says "update it in
Mods > Explore". The tab is `themed_button("Explore online")`
(`crates/nova_menu/src/menu_ui.rs:300`) and `web/src/create/publish-a-mod.md:187`
calls it **Explore online** in bold. Same wording repeated at
`web/src/news/0.13.0.md:1146`.

**MINOR - `web/src/create/scenarios.md:54` - the new skybox paragraph links to a
page that does not document the sidecar it names.** It tells an author a skybox
needs "a `.png.meta` sidecar that reinterprets the stack (see [Base
content](../base-content/))". The link resolves, but the only sidecar sentence
on that page says metas "ride along with their image automatically and are never
listed or referenced directly", which is about base assets.
`array_layout: Some(RowCount(rows: 6))` appears nowhere in `web/src/` or
`docs/`. An author who ships a sky with no sidecar falls onto the
`SkyboxPlugin` fallback reinterpret that
`crates/nova_assets/src/collections.rs:388-399` documents as frame-dependent
and, at 4096 px faces, fatal.

**MINOR - `web/src/news/0.13.0.md:1129` - "max-pooled rather than averaged
down" misstates the method.** The task describes max-pooling at the reduction
factor BEFORE the box average, not instead of it. The conclusion the sentence
draws is right; the method is not.

**MINOR - `credits/CREDITS.md:47` - the Bevy icon entry is the only asset credit
with no path.** The file is `build/icon_1024x1024.png` plus its ten derivatives
under `build/macos/AppIcon.iconset/` and the `build/windows` / `build/web`
copies. Every other entry in both sections names its files, so the list can be
checked against the tree; this one cannot.

**Verified clean by the contracts lane.** `git log v0.13.1..v0.13.2` is six
commits and the `[0.13.2]` block's three entries cover the three user-visible
ones exactly; the two commits with no entry are a `drop_non_drop` test cleanup
and a task close. Entry lengths joined are 185, 187 and 194 characters, all
under the 200 cap. Section order matches the file header. No `**(breaking)**` is
owed: `crates/nova_wfc/src/collapse.rs` changed only private signatures, and the
cubemaps kept their ids and paths, so `webmods/gauntlet` and
`webmods/the-ledger` reach both skies through `dep://base/textures/cubemap*.png`
unchanged. `Cargo.toml:818` and all 33 `Cargo.lock` entries are at `0.13.2`; a
tree-wide `0.13.1` grep hits only changelog history and third-party crates. The
tag points at `b3c6f579c`. The downscale is backdrop-only: no
`EnvironmentMapLight` is fed from the cubemap, it binds through `Skybox` at
`crates/nova_ship/src/camera/skybox.rs:154` and
`crates/nova_editor/src/gallery/mod.rs:217` only, so no lighting or reflection
contract moved. Every other path in `CREDITS.md` and in
`credits/licenses/space-3d_Unlicense.md` exists.

**MAJOR - `crates/nova_assets/src/merge.rs:216-228` - the new error accuses a
healthy mod of being out of date, on a state that is just "not loaded yet".**

Re-derived, and this is the strongest finding of the group. `register_bundles`
admits a downloaded bundle into `ordered` on `bundles.contains(&m.bundle)`
(`merge.rs:99`), which is true as soon as Bevy inserts the `BundleAsset` itself.
A bundle's `*.content.ron` handles are SEPARATE loads issued by
`BundleAssetLoader` (`crates/nova_modding/src/lib.rs:365-378`), and
`AssetEvent::Added` precedes `LoadedWithDependencies`, so `contents.get(...)` is
legitimately `None` in that window.

The window is reachable on the ordinary install path.
`mark_downloaded_bundles_loaded` (`crates/nova_assets/src/mod_set.rs:380`) flags
`DownloadedMods` changed the moment mod A reports `LoadedWithDependencies`;
`installed_set_changed` (`mod_set.rs:370`) then re-runs `register_bundles` while
mod B's content files are still in flight. B is fine and merges one frame later,
but the log now says `content '...' did not load (the loader error above says
why) ... A downloaded mod that no longer parses was built for an older game
version: update it in Mods > Explore.` There is no loader error above and
nothing is out of date. Enabling a mod from the menu during a download reaches
the same window.

`merge.rs:96-107` already handles the identical transient one level up, with
`warn!` and honest text: "its bundle has not loaded yet; it merges when the load
completes". The new branch is the loud opposite of its own neighbour. Fix: gate
the bundle on its content too (`bundle.content.iter().all(|h|
contents.contains(h))`, or `is_loaded_with_dependencies`), so the branch only
ever sees a real failure and its advice becomes true.

Ceiling: it is one alarming log line and no behavior changes, which is why it is
not a BLOCKER.

**MINOR - `crates/nova_scenario/src/actions/view.rs:485` and `:715` - the
downscale left the justification for the `needs_cube_view` guard off by 16x.**
Both say "a full re-upload of the hundreds-of-MB cubemap texture". It is 24 MB
now. The read-then-`get_mut` shape at `view.rs:495-505` and the test
`skybox_swap_does_not_remodify_an_already_cubed_image` exist only because of
that number, so a reader who checks it can reasonably decide the guard is not
worth keeping.

**MINOR - `crates/nova_assets/src/collections.rs:390-396` - the rewrite traded
one wrong claim for another.** The new lead says "the stacked form uploads as a
plain 2D texture no skybox can bind", but every path that binds first
reinterprets: `setup_skybox_camera`
(`crates/nova_ship/src/camera/skybox.rs:142-152`) on the single-layer branch and
`apply_pending_skybox_swaps` on the arrayed one, and lines 403-404 of this same
comment keep that fallback on purpose. The old text was false at the shipped
size (6144 < 16384); the new text is false as a failure mode. The same wrong
framing survives at `collections.rs:594-595` ("silently disappears on a
16384-limit GPU" - a missing Cube view fails Bevy's
`sanity_check_skybox_image_and_warn` on EVERY GPU, which is what
`view.rs:479-483` says), and is carried into
`crates/nova_assets/tests/cubemap_meta.rs:5` ("This guards the skybox upload
race" - nothing in the rewritten body is a race) and
`crates/nova_core/tests/cubemap_meta_app_config.rs:13-16`.

**MINOR - `crates/nova_assets/src/merge.rs:143-144` - a comment that restates
the next line**, which `AGENTS.md` asks to be deleted. **MINOR -
`merge.rs:221`** - the `"<unknown>"` fallback cannot occur: every handle in
`bundle.content` comes from `load_context.load::<ContentAsset>(resolved)` and is
path-backed. **MINOR - `merge.rs:169-172`** - the terse bundle branch is
unreachable (`bundle_handles` derives from `ordered`, whose downloaded entries
already passed `bundles.contains` in the same immutable-`Res` run), so the pair
of error branches speaks with two voices and the dead one is the terse one.

### G1 verdict

Nothing here blocks. v0.13.2 shipped a real fix and the arithmetic behind it is
right. The two things worth doing are the missing face-size assertion and the
false out-of-date accusation; the rest is prose against the tree.

Proof rerun in this session: `nix develop --command cargo test -p nova_gameplay
--lib integrity::pyre` - 15 passed, 0 failed (that is a G2 proof, run early
while the G1 lanes were out; recorded here so it is not claimed twice).

### G1 correctness lane: the downscale's quality claims, independently reproduced

The correctness lane reached the same MAJOR as craft (the merge.rs false
accusation) by its own route, so that finding has two independent groundings.
It also did what the authoring task could not, and the result is worth keeping:

- **"Per-face nebula mean holds to 1.5%" is exactly true.** Measured +0.58% to
  +1.46% on `cubemap.png` and +0.59% to +0.92% on `cubemap_alt.png`. The task's
  own face-0 table (0.0036064 -> 0.0036591) reproduces to seven digits.
- **"No star is lost" is true and understated.** 48 connected blobs at max
  channel >= 128 across the six faces; every one keeps a peak of 214-255/255.
  The four 4x4 cells that fall under the threshold are satellites of blobs whose
  brightest cell survives.
- **The pipeline was reconstructed and matched.** Re-implementing per-face
  sRGB->linear, 4x4 sliding max at offset -2, 4x4 box mean, linear->sRGB
  reproduces shipped faces 0 and 1 to a maximum absolute difference of 1/255 and
  99.92% exact texels; a plain-box control differs by up to 126/255. Because the
  edge clamp had to be face-local to match, this also confirms the faces were cut
  apart before scaling, so no kernel crossed a cube seam. The outer 1-px ring
  sits +0.15% to +0.41% above a plain-box control, LESS than the interior, so the
  max filter does not brighten the seams.
- **Sidecars are byte-identical** and bake no dimension, so they still describe
  the files. PNG chunks are IHDR/IDAT/IEND only; no gAMA/iCCP/sRGB in old or new,
  so the decode path is unchanged.
- **Linear-light mean rises further (+1.9% to +7.1%) and it does not matter:**
  there is no `EnvironmentMapLight` anywhere in `crates/`, so the sky is
  background-only and never feeds IBL.
- **Nothing else in the tree assumes 4096.** Grepped `.rs .md .toml .ron .meta
  .py .sh .js .yaml .wgsl`; every surviving mention is a doc comment about a
  hypothetical mod or historical changelog. `nova_ship/src/camera/skybox.rs:139`
  derives `layers = height / width`, and `tools/nova_meta_gen` only writes
  MISSING sidecars.

Proofs rerun: `cargo test -p nova_assets --test cubemap_meta` (1 passed) and
`cargo test -p nova_core --test cubemap_meta_app_config` (3 passed, including
the mod-shipped nebula).

The one thing the lane could not close: at 1024/face a texel subtends ~0.088 deg
against ~0.036 deg per screen pixel at 1080p/70 deg FOV, so a star that was
sub-pixel is now roughly 2-5 screen pixels wide. Whether that reads chunky needs
a rendered frame, and the box was busy. Explicitly open.

### Proofs rerun in this session, outside any lane

Run while lanes were out, so they are recorded once here rather than claimed by
a group. All green.

| Command | Result |
|-|-|
| `cargo test -p nova_gameplay --lib integrity::pyre` | 15 passed |
| `cargo test -p nova_ship --lib flight::` | 126 passed |
| `cargo test -p nova_ship --lib input::ai::passive` | 34 passed |
| `cargo test -p nova_ship --lib sections::fixture` | 10 passed |
| `cargo test -p nova_wfc --lib` | 15 passed |
| `cargo test -p nova_channel --lib` | 17 passed |
| `cargo test -p nova_input --lib dispatch` | 9 passed |
| `python3 -m unittest discover -s scripts/nova_illustration` | 46 passed |
| `python3 -m unittest discover` over `web/src/comics/**/art` | 43 passed |

`a_bare_transom_leaves_the_seeded_drive_bolted_to_the_keel` (the erosion fix),
`a_plate_killed_by_the_tick_comes_off_in_that_tick`,
`a_frame_that_banks_several_fixed_steps_still_pays_for_one_frame`,
`the_warm_up_waits_for_a_camera_and_runs_on_the_frame_one_arrives`,
`a_tier_raised_mid_run_warms_the_graphs_the_low_tier_skipped` and
`a_section_stop_still_lifts_the_trigger_after_the_mount_is_destroyed` all pass -
each is the named regression test of one of tonight's commits.

The web suite was NOT rerun here: CI job 103019570348 ran `npm run ci` (format,
lint, fourteen suites, released-only build) green on this exact tree 30 minutes
before this review started.

### Adjudicator's own G2 derivation, before the lanes reported

`crates/nova_ship/src/sections/fixture.rs:69-74` now says the tick cap means
"24 drains a tick whatever the renderer is managing: at the default 64 Hz a hull
stripped to the last of those 263 fixtures clears in about a sixth of a second,
and a slow renderer no longer slows the drain the way a per-frame cap did." The
next paragraph adds `SHED_FRAME_CAP = 48`.

Worked through: at 64 Hz fixed and F fps a frame banks 64/F ticks and sheds
`min(48, 24 * banked)`. At F >= 32 the frame cap is not binding and 263 fixtures
clear in 263/(48F) - about 0.17 s, so the sixth-of-a-second claim holds. At
F = 10 the frame cap binds: 48 a frame is 480 a second and the same hull takes
0.55 s. So the paragraph's "a slow renderer no longer slows the drain" is false
of the code the very next paragraph introduces. The frame cap is twice the old
per-frame rate, not renderer-independence.

Also: "24 drains a tick" is the same ambiguity `f063e6637` set out to fix in
`docs/sections.md` ("64 a second", which a creator read as 64 fixtures). It is
24 FIXTURES a tick, one drain.

Held here pending the performance lane, which was asked this question directly.

### End-to-end check of the day's shipped web output

Read-only requests to the project's own GitHub Pages site, no cost and nothing
published:

- `/nova-protocol/play/assets/base/textures/cubemap.png` -> 200,
  `content-length: 237317`, `last-modified: 2026-09-10T20:07:20Z`. The
  downscaled sky is live.
- `/nova-protocol/story/` -> 200, index lists **Season 1, 1 episode**.
- `/nova-protocol/story/season-1/` -> lists **A useful job**, **18 pages**.
- `/nova-protocol/story/season-1/episode-1/` -> 200, 103,462 bytes.

So `8896f385f`'s flip of `episode.json` from `draft` to `released` reached the
public site, and the later full-site `deploy-github-page` run did NOT clobber
the comic the separate `comic-s1e1` tag had published that morning - both
`/story/` and `/play/` carry the same 20:07:20Z stamp.

### G2 craft lane - adjudicated

Lane reported. Every load-bearing claim below was re-derived by me before it was
kept.

**MAJOR - `docs/sections.md:518`, `:530-531`, `:540` - the design doc describes
the warm-up schedule `8eaa53c36` deleted, and one sentence is now the exact
inverse of the code.**

Verified in the tree:

- `docs/sections.md:518` "runs in `PostStartup` after the settings pass".
  `crates/nova_gameplay/src/integrity/pyre.rs:793-798` registers
  `warm_the_pyres` on `Update`, `.after(SettingsSystems)`.
- `docs/sections.md:530-531` "There is a second pass on `Update`, run only when
  the graphics budget changes while the graphs are still cold". There is no
  second pass and no budget-change condition anywhere in the file. The three
  run conditions are `pyres_are_cold` (`:615`), `the_tier_draws_particles`
  (`:626`) and `a_view_exists` (`:637`).
- `docs/sections.md:540` "the warm-up runs before any scenario has a camera".
  `a_view_exists` (`pyre.rs:637`) is precisely the opposite, and it is the
  condition the commit exists to add.

Failure scenario: a reader takes `:540` as the constraint, concludes a
view-time warm-up cannot be done, and restores the `PostStartup` placement -
which is the `slice offset 0 size 80 is out of range for buffer of size 60`
boot panic `8eaa53c36` fixed.

**MAJOR - `crates/nova_ship/src/sections/fixture.rs:69-74` - the surviving
`SHED_TICK_CAP` doc contradicts the `SHED_FRAME_CAP` twelve lines below it.**

This is my own held derivation, and the craft lane reached it independently.
Resolved as a finding. Verified:

- `fixture.rs:69-74`: "24 drains a tick whatever the renderer is managing: at
  the default 64 Hz a hull stripped to the last of those 263 fixtures clears in
  about a sixth of a second, and a slow renderer no longer slows the drain the
  way a per-frame cap did."
- `fixture.rs:121`: `const SHED_FRAME_CAP: usize = SHED_TICK_CAP * 2;` - a
  per-frame cap, added by `f6e10edb9` in this same range.
- `fixture.rs:258`: `let allowance = SHED_TICK_CAP.min(budget.left);` and
  `:315`: `budget.left -= shed;` - the frame cap is spent, so it binds.

Failure scenario: the drain is `min(24 a tick, 48 a frame)`. At 8 fps the
banked steps are capped at 48 a frame, so 263 fixtures take 263/384 = 0.68 s,
not the sixth of a second the comment quotes; at the 4 fps floor `max_delta`
implies, 1.37 s. The renderer is exactly what slowed it. `:76-80` half-retracts
the claim without deleting it.

Change: state the drain as `min(24 a tick, 48 a frame)` and qualify the
sixth-of-a-second figure as the frame-rate-permitting case.

**MINOR - `fixture.rs:234-237`** - "every production trigger of it runs after
the physics step" names `advance_rounds` and `resolve_nova_blast_hits` but not
the third, ram damage: `on_impact_collision_deal_damage`
(`crates/nova_gameplay/src/integrity/core.rs:208`) observes avian's
`CollisionStart`, which fires inside `PhysicsStepSystems::Finalize`. Correct
today by structure, unstated and unordered.

**MINOR - `fixture.rs:340-342`, `:530-532`, `shell_skin.rs:1152-1156`,
`:1217-1219`** - the shed wiring is hand-copied into four apps and already
diverged once inside this range (`010a6cb25` fixed a rig with the system and no
resource). `pyre.rs:815-817` states the fix for the same problem in the
neighbouring module: build the rig from the plugin.

**MINOR - `pyre.rs:559-561`** - "An earlier cut warmed in `PostStartup` ... and
did exactly that" is changelog in a source file; `:552-559` already states the
constraint completely. AGENTS.md: "Explain ownership and constraints, not code
or history."

**MINOR - `pyre.rs:508` and `:627`** - `tier.as_deref().is_none_or(|t|
t.particles)` written twice, character for character; `drawable` can call
`the_tier_draws_particles` directly.

**MINOR - `fixture.rs:98-100`** - `ShedBudget`'s doc claims `PyreBudget` shares
its shape. It does not: `PyreBudget` (`pyre.rs:99-100`) counts UP against a cap
because `light_the_pyre` increments unconditionally for an `IntegrityRoot`, so
a countdown would underflow. The real difference is undocumented.

**MINOR - `pyre.rs:106`, `:647`** - two ragged doc wraps left by edits; nothing
in `rustfmt.toml` settles them.

Three craft questions answered with NO finding, each checked: `PyreEffects::pair`
is genuinely one answer (`PyreSize::scale` at `:122-127` is the single table);
`e4edd0cbd` left nothing behind in `web/src/widgets.ts` (`tsc` clean, widget
tests pass); `NovaDamageSystems` is named, preluded and ordered explicitly at
`shell_skin.rs:1155-1159`.

Lane's own skips: no Rust build, no test run, no measurement. Those are covered
by the proofs table above and by the performance lane.

### Baseline update: master CI closed green on the release tree

Re-polled at the end of the night, not estimated.

| Run | Tree | Result |
| --- | --- | --- |
| CI 34521413852 | `b3c6f579c` (Release v0.13.2) | success, all 9 jobs |
| CI 34489343920 | `f3a78a434` | success |
| CI 34469714561 | `fda61a2eb` (v0.13.1 tree) | FAILURE |
| release-flow 34521501396 | v0.13.2 pages | success |
| release-flow 34521418070 | v0.13.2 binaries | still in progress: linux, web, macOS x86_64 green; macOS aarch64 and windows running |

The nine jobs of 34521413852: `web / npm suite + python tests`,
`probe / playable`, `probe / screenshots`, `probe / systems`,
`check / default features`, `autopilot example`, `clippy / wasm32`,
`fmt / clippy / test`, `dependency license gate` - all success. So the two
defects that made 34469714561 red (`drop_non_drop` under `-D warnings`, and the
`system_ship_editor` seed crash) are fixed on the shipped tree by `ecb504b6f`
and `f3a78a434`, and neither is in v0.13.2.

What remains true and is worth Alex's attention: v0.13.1 was tagged, built and
PUBLISHED from a tree whose own CI run had failed twice over. release-flow for
v0.13.1 (34469726890) succeeded on that tree because nothing in the release
workflow re-runs the CI gate.

### G2 correctness lane - adjudicated

**MAJOR - `docs/sections.md:518`, `:530-531`, `:540`** - the SAME finding the
craft lane raised, reached independently and by a different route. Two lanes,
one defect. It shipped in v0.13.1 and v0.13.2. `docs/sections.md:1074` points
readers at this passage for exactly this subject.

**MINOR - `crates/nova_scenario/src/objects/spaceship.rs:155` - the third copy
of the "~20 m floor" claim that `01b368441` corrected in the other two.**

Verified in the tree:

- `crates/nova_scenario/src/objects/spaceship.rs:155`: "Below ~20 m risks
  stalling outside the advance gate - author small, not zero."
- `crates/nova_ship/src/input/ai/passive.rs:29-31` (corrected): "the gate
  counts both and measures from the same centre of mass the leg parks - so the
  slack has only the arrival's own terminal drift left to cover."
- `web/src/create/objects.md:309` (corrected): same sentence.

`01b368441`'s own message says it fixed "the two places". The field docstring
in `spaceship.rs` is the one a creator authoring RON reads FIRST, and it still
states a floor the commit removed. A creator authoring the weave's own 5 m is
told they are under a floor that no longer exists.

**MINOR - `crates/nova_ship/src/input/ai/passive.rs:872`** - `patrol_world()`
spawns no `ComputedCenterOfMass`, so `center_of_mass.map_or(Vec3::ZERO, ..)`
collapses to the old origin reading in every patrol test but the one new
regression. Five of the six COM-moved uses (the detour rays at `:293`, `:302`,
`:307`, `:329`) would pass unchanged if the hoist had never happened. This is
the "a fixture must spawn every component production spawns" trap.

**MINOR - `crates/nova_gameplay/src/integrity/pyre.rs:686`** - `light_the_pyre`
carries no view gate, so the hanabi row-table hazard `8eaa53c36` closed is
closed only on the warm-up path. The lane traced the mechanism in the vendored
source (`bevy_hanabi-0.19.0/src/render/mod.rs:5974`, `buffer_table.rs:556-575`)
but could NOT ground a shipped path that reaches it. Recorded as a residual,
not a defect.

**NOTE, not a finding - `shell_skin.rs:1217`** - the range does not bisect:
`f6e10edb9` added `ShedBudget` to `ShipSkinPlugin` but not to `skin_app`, and
nine tests panic at that commit. `010a6cb25` fixed it 31 minutes later. HEAD is
correct at all three registration sites.

Five adjudicated NO-finding answers, each grounded by the lane: the warm-up
cannot re-run (`pyres.pair` fills both slots); the shed arithmetic cannot
underflow and `First` precedes `RunFixedMainLoop`; the GOTO anchor tie
(`origin_radius > hull_radius` strict) is self-consistent and the ORBIT handoff
agrees; all six `position` uses in the patrol arm want the COM; the chip-budget
widget's 581 is exactly what `spew.rs:567-575` yields.

Lane ran 661 Rust tests green across four filters plus the web widget suite.

### G2 contracts and performance lanes - adjudicated, with one lane disagreement settled

**MAJOR - `docs/sections.md:517-541`** - raised by ALL FOUR G2 lanes
independently. Kept once. Already re-derived above.

**MAJOR - `crates/nova_gameplay/src/integrity/pyre.rs:254-255` (and `:239-240`)
- all three hull spans in the `HULK_PYRE` docstring are short, because a
multi-cell drive was counted as a unit cell.**

The two lanes DISAGREED here. Contracts said the figures are wrong; performance
listed "block_gunship 8.5, block_warship 21.5, block_carrier 36.0 ... exactly as
the docstring claims" among its checks. I derived it myself rather than pick a
side, from `assets/base/ships/base.content.ron` and
`assets/base/sections/base.content.ron`:

| Hull | Fore-most | Aft-most | True span | Docstring |
| --- | --- | --- | --- | --- |
| `block_gunship` | `plate_43` hull at z -6.0... z -4.0, face -4.5 | `main_drive` `vector_thruster_section` at z 3.5, half-depth 1.0, face 4.5 | **9.0 u, 90 m** | 8.5 u, 85 m |
| `block_warship` | `plate_195` at z -6.0, face -6.5 | `drive_port` `vector_thruster_section` at z 14.5, face 15.5 | **22.0 u, 220 m** | 21.5 u, 215 m |
| `block_carrier` | `plate_0` at z -16.0, face -16.5 | `capital_drive_port` `capital_thruster_section` at z 19.0, half-depth 1.5, face 20.5 | **37.0 u, 370 m** | 36 u, 360 m |

The authority for the half-depths is
`crates/nova_ship/src/sections/base_section.rs:45-47` ("`Cuboid.size` is the
FULL side length on each axis") and `:102-105`
(`aabb_half_extents` = `size * 0.5`), with `vector_thruster_section` authored
3x3x2 and `capital_thruster_section` 5x5x3. A section that omits `collider`
"gets the unit cube" (`base_section.rs:41-43`), which is why every plate is
+/-0.5.

Every one of the docstring's three figures equals
`max_centre + 0.5 - (min_centre - 0.5)`: the multi-cell drive's overhang past
its centre cell - 0.5 u on the gunship and warship, 1.0 u on the carrier - was
dropped. The performance lane computed the same naive centre span and read it
as agreement. Contracts is right.

Consequence is bounded (2.3% to 2.8%) and the paragraph's conclusion - 130 m of
ejecta reach fails to cross either capital hull - survives. It is still a MAJOR
by the house rule: the docstring is the whole deliverable of `7cf2056dc`, and
every figure it exists to state is wrong.

**MINOR - `CHANGELOG.md:63-65`** - "and again when the tier changes" describes a
second warm-up run that cannot happen. `pyres_are_cold` (`pyre.rs:615`) is false
once both pairs stand, and `:610-614` says a tier LOWERED mid-run deliberately
keeps its graphs. The case meant - a tier RAISED off a particle-less preset - is
the FIRST run, not an "again". Player-facing, and it shipped in v0.13.1.

**MINOR - `pyre.rs:796-799`** - three stacked `run_if`s do not short-circuit;
Bevy evaluates all conditions on purpose. A spawn-less run (Low tier,
`particles: false`) pays all three every frame forever, because `pyres_are_cold`
never goes false. `bevy_ecs-0.19.0/src/schedule/condition.rs:207` (`and`, with
the older `and_then` at `:135`) is the documented short-circuiting combinator.
Unmeasured; the arithmetic says sub-microsecond.

**MINOR - `pyre.rs:801`** - `.after(warm_the_pyres)` auto-inserts an exclusive
`ApplyDeferred` into `Update` for every frame of every run, to buy one frame's
earlier cleanup of four hidden entities.  `after_ignore_deferred` keeps the
order without the barrier. Unmeasured.

**MINOR - `crates/nova_ship/src/sections/fixture.rs:304-315`** - the frame
ceiling counts PLATES, but the per-plate cost is a full descendant walk that
queues one `try_remove::<Collider>()` per node, most of them no-ops. Either say
so in `SHED_FRAME_CAP`'s docstring or filter the walk.

**MINOR - `web/src/news/0.13.0.md:957`** - the prose above the corrected widget
still states the per-crater overflow rule the widget no longer computes ("one
crater is untouched") while the scope beneath it prints 581.

**MINOR - `web/tests/widgets.test.ts:665-690`** - `ZONE_PARTS`' five footprints
are the one input to that widget NOT read out of the Rust by regex; they are
cited in comments and passed as literals. Re-spec `capital_thruster_section` and
the widget keeps teaching the claim `3414c6f19` exists to make, with every test
green.

**MINOR - `CHANGELOG.md` `[0.13.1]`** - `33dcf22c3` corrected a shipped
news-post widget (floor -> ceil, 582 -> 581) and got no entry, while its two
siblings in the same range did.

### G2 verdict

Four lanes, all reported. Two MAJORs and nine MINORs, no BLOCKER. The code
`8eaa53c36`, `f6e10edb9`, `010a6cb25`, `031c88144`, `01b368441`, `e4edd0cbd`,
`3414c6f19` and `33dcf22c3` landed is correct as far as four lanes and 661
green tests can establish. **Everything worth fixing in G2 is a WORD, not a
line of code**: a design-doc chapter that inverts the fix it documents, three
wrong hull figures, a player-facing changelog line describing a run that cannot
happen, and a source comment that denies the constant twelve lines below it.

Measurement: none. The performance lane withheld its slot for the second time
tonight because the box was still running another session's
`weapon-machines-showcase` capture under Xvfb :100 at loadavg above 10. Every
timing figure in this group is arithmetic over the constants and the schedule.
That is an explicit skip, not a pass.

## G3 dispatched - flight guidance, autopilot rest legs, WFC seeded spine

`28a14db97 b16321d12 ecb504b6f f3a78a434`, 355+/65-. Three lanes: correctness,
contracts, craft. No performance lane and no measurement slot - the box is still
contaminated, and this group's risk is behavioural, not thermal.

Two questions carried into the lanes from my own reading, so they are not
re-derived from scratch:

- `b16321d12` added `&& owed > crumb_band` to the brake gate
  (`autopilot.rs:911-915`). Units are consistent - both sides are speeds
  (`crumb_band` = `settle_deadband.max(attitude_deadband)` = 0.75 u/s,
  `state.rs:445`). What I could not settle alone: `state.rs:381-384` promises
  "STOP still brakes to `stop_speed_epsilon` EXACTLY" for an axial residual, and
  the two axial regression tests
  (`flight/tests/stop.rs:11`, `:146`) assert only `speed < 0.5` - under the
  0.75 band and over the 0.2 epsilon, so neither can catch a regression to the
  band in either direction.
- `f3a78a434`'s `widen_to_whole_parts` (`collapse.rs:438`) uses the same
  join predicate as `drop_part` (`:416`). Whether the two are exact inverses,
  and whether `keel_component` can still prune a seeded part stranded by
  erosion of the cells around it.

### G4 - my own pre-review while G3's lanes run

Read directly, not delegated. Four claims in this group are checkable in
seconds, and all four hold:

- `.github/workflows/ci.yaml`, `Generated art is deterministic`: the comment
  claims seven stdlib-only generators each carrying `--check`. All seven do
  (`grep -l -- --check scripts/gen-*.py`), and none imports a third-party
  package - `nova_glb`, `nova_illustration` and the stdlib only.
- The same comment says "`gen-web-screenshots.py` is NOT here and cannot be: it
  has no `--check`". True: the only `--check` string in that file is
  `scripts/gen-web-screenshots.py:392`, a docstring reference to
  `gen-scenario-thumbnails.py --check`.
- `5453fe51c`'s `check_producers()` runs on EVERY invocation
  (`gen-web-screenshots.py:1232`) and uses `re`, which is imported at `:64`.
  I ran `python3 scripts/gen-web-screenshots.py --producers`: exit 0, 29
  producers, no refusal. The gate is live and the tables are clean.
- `fc8bb6e7a` removed `hide_status_bar` from `stress_hull_collapse.rs:537` and
  its message says the pair was redundant after `hide_hud`. The three other
  call sites are NOT the same pair - `screenshot_radar_lock.rs:287-291`,
  `hollow.rs:460-464` and `ring.rs:319-323` each set `HudVisibility::On` first,
  which is the case `hide_status_bar`'s own docstring
  (`crates/nova_debug/src/harness.rs:717-724`) exists for. No inconsistency.

One thing worth a lane's attention when G4 is dispatched, which I could not
settle alone: `registered_examples()` returns `None` when `Cargo.toml` names no
`[[example]]` at all, and `check_producers` then skips the manifest check
silently. That is a designed fallback for an unreadable manifest, but it is
also a silent pass in a commit whose whole subject is silent passes.

### Refinement to the G2 `docs/sections.md` MAJOR - the doc was rewritten TWICE

Ordered the pyre commits by time rather than by log order:

| Time | Commit | Touched |
| --- | --- | --- |
| 11:43 | `6882fc2c4` | `pyre.rs` + `docs/sections.md` (the `PostStartup` design) |
| 11:46 | `6e6411d99` | `pyre.rs` + `docs/sections.md` (28 lines, same passage) |
| 11:54 | `2b6256622` | `pyre.rs` only |
| 13:29 | `8eaa53c36` | `CHANGELOG.md`, `pyre.rs`, `settings.rs` - NOT the doc |

So the chapter was written for the `PostStartup` cut and then polished for it
three minutes later, and the commit that replaced that design 1h43m afterwards
left both passes standing. The finding is not "a doc nobody touched"; it is a
doc that got two careful revisions for a design that was gone by lunch.

### G8 pre-review: the changelog ordering rule is enforced only where it was
### last edited

`9cec1efc9` ("Put the unreleased sections in the order the file documents")
landed today and reordered the then-unreleased block. I checked EVERY release
block in `CHANGELOG.md` against the order its own header declares (Gameplay &
Flight, Combat & Weapons, Ships & Sections, Scenarios & Objectives, Modding &
Mod Portal, Interface & HUD, Web & Platform, Audio & Visuals, Performance,
Fixes, Internals & Tooling - `CHANGELOG.md:6-8`). Four released blocks break it:

| Block | Out of order at |
| --- | --- |
| `[0.11.0]` | `### Gameplay & Flight` at `CHANGELOG.md:1061`, after Combat & Weapons |
| `[0.10.0]` | `### Ships & Sections` at `:1552`, before Gameplay & Flight `:1561` and Combat & Weapons `:1569` |
| `[0.7.0]` | `### Fixes` at `:1902`, before Modding & Mod Portal `:1911` |
| `[0.2.0]` | `### Modding & Mod Portal` at `:2182`, before Scenarios & Objectives `:2186` |

`[0.13.2]`, `[0.13.1]`, `[0.13.0]`, `[0.12.0]` and every other block are
correct. NOT introduced today - found while checking today's `9cec1efc9`, whose
subject is exactly this rule. MINOR, and honestly out of the night's range.

### G6 pre-review: the UTF-8 fix has two unfixed siblings

`f5a80a2c1` gave `web/build-comics.py:21-22`'s `put()` an explicit
`encoding='utf-8'` on both the read and the write, because "the comparison it
makes to preserve unchanged bytes is a decode". I swept every other text read
and write under `web/` and `scripts/` for the same shape. Two remain:

- `scripts/deploy-pages.py:55` - `json.loads(marker.read_text())`
- `scripts/probe-summary.py:62` - `json.loads(index.read_text())`

JSON is UTF-8 by definition, so decoding it through the platform default is
wrong in principle wherever it appears; `deploy-pages.py` is on the deploy path.
MINOR and honestly bounded: both files' inputs are ASCII today and GitHub's
runners set a UTF-8 locale, so neither is failing now. The only other
unencoded reads in the tree are in
`scripts/nova_illustration/test_illustration.py:261,449`, which read what the
same process just wrote.

### G6 pre-review: `7dd7cfda7`'s measured claim reproduces exactly

The commit claims "the three committed design sheets from 3.14 MB to 2.83 MB".
Measured from the tree, `7dd7cfda7~1` against HEAD:

| Sheet | Before | After |
| --- | --- | --- |
| `web/src/assets/lore/ebro-design-concept.svg` | 1,449,411 | 1,303,493 |
| `web/src/assets/lore/gantry-design-concept.svg` | 620,025 | 561,081 |
| `web/src/assets/lore/kaveri-design-concept.svg` | 1,068,508 | 962,916 |
| Total | 3.14 MB | 2.83 MB |

9.9% off, exactly as stated. The 50-panel figure (11.71 -> 10.48 MB) is not
committed output and was not re-measured.

### G5/G7 pre-review: two more measured claims reproduce

- `087bde783` claims "Twelve left/right-tailed balloons ship in episode one."
  Counted from `web/src/comics/season-1/episode-1/pages/*.ts`: `side: "left"` 4,
  `side: "right"` 8, `side: "bottom"` 83, `side: "top"` 7. Twelve exactly.
- `8ef9751b6`'s gate is wired end to end: `.github/workflows/deploy-comic.yaml`
  runs `npm ci`, `npm run test:deploy`, `npm run build:story` in that order
  (`:45-49`), and `scripts/test_deploy_pages.py:158` now asserts
  `npm run test:deploy` is IN the comic workflow, with its two neighbours
  (`build:site`, `trunk build`) still asserted out. The self-referential
  negative the commit describes is gone.
- `8896f385f`'s `[Unreleased]` entry did not get lost in the two releases that
  followed it: it is `CHANGELOG.md:38-40` under `[0.13.1] Web & Platform`, and
  its "committed lore design sheets by 10%" sibling matches the 9.9% I measured.

### G5 pre-review: the deleted balloon leaves one archived generator that cannot run

`e6bd754bf` deleted `speech()` from `scripts/nova_illustration/lettering.py`,
saying "The archived clean-line study under tasks/20260908-161328 was its last
caller and no longer runs". Both halves check out:

- The only remaining `speech(` callers outside `tasks/` are none.
  `tasks/20260908-161328/clean-line-study/generate.py:18` still has
  `from nova_illustration.lettering import speech`, so that script now fails at
  IMPORT, and its three committed SVGs can no longer be regenerated from it.
- Nothing runs it: CI's `Comic art tests` step globs `web/src/comics` and
  `Illustration library tests` globs `scripts/nova_illustration`; neither
  reaches `tasks/`.

MINOR, and knowingly accepted by the commit rather than missed. Recorded
because the repo's own convention is that proof lives with the task, and this
proof can no longer reproduce itself.

`231db18eb`'s "Every export is byte-identical for all nine characters" is backed
without re-running it: the commit touches no committed art
(`portraits.py` + `test_illustration.py` only), and CI's
`Generated art is deterministic` step runs `gen-lore-portraits.py --check` and
`gen-lore-designs.py --check`, which byte-compare the tree. That step is green
on `b3c6f579c`.

### G6 pre-review, my own finding: a `stop` can lift a source the wire never pushed

**MINOR - `crates/nova_channel/src/apply.rs:282-291`.**

`a8041de57` made a RELEASE prefer `dispatch::driven_press(world, &wire)` and
fall back to `section_source(world, id)`. The context gate below it is
`phase == InputPhase::Press` only (`:282`), by design - a stop must always
lift. Put together, there is a path where the fallback fires for a press that
never happened:

1. `section start port_turret` arrives while `ActionContext::Flight` is DOWN.
   `:282-287` acks and returns before `press_source`, so nothing is pushed and
   `record_press` never runs.
2. Flight comes up. `section stop port_turret` arrives. `driven_press` returns
   `None` (nothing was recorded), the fallback resolves the mount - it is still
   there - and `:289` calls
   `press_source(world, InputSource::Mouse(Left), Release)`, which is
   `ButtonInput::<MouseButton>::release(Left)`
   (`crates/nova_input/src/dispatch.rs:126-131`).
3. If anything else holds that source - the driver's own mouse, or a second
   section bound to the same button - it is released for them.

The three section tests cover the neighbouring cases and not this one:
`a_section_stop_under_a_lowered_context_still_lifts_the_trigger` (`:679`)
presses with Flight LIVE, so the press IS recorded;
`a_section_stop_still_lifts_the_trigger_after_the_mount_is_destroyed` (`:725`)
has a recorded press;
`a_section_stop_for_a_mount_that_was_never_started_is_a_no_op` (`:772`) is the
never-started case where the mount is ALSO gone. Never-started with the mount
still present is the gap.

Not a regression - the pre-`a8041de57` code resolved and released on the same
path - and MINOR because the process channel is a scripted wire. Worth folding
in: `driven_press` returning `None` for a `section.<id>` the process itself
never recorded is enough to know this wire holds nothing.

### G4 pre-review: `b5d7f341c`'s env-var corrections are true

The commit adds four names to `docs/environment-variables.md` and states a
contract for them: "Each takes `0` or `1`, and any other value is an authoring
error that panics rather than picking a path", with `NOVA_STANDOFF_TRACE`
explicitly exempted as "a diagnostic and not a mode: set to anything". Checked
each reader:

| Name | Reader | Shape |
| --- | --- | --- |
| `NOVA_SIGHT_LOOP` | `examples/systems/system_lock_line_of_sight.rs:124-131` | `0`/`1`, panics otherwise |
| `NOVA_WAKE_LOOP` | `examples/playable/railgun_wake_bench.rs:1031-1038` | `0`/`1`, panics otherwise |
| `NOVA_COLLAPSE_LOOP` | `examples/systems/stress_hull_collapse.rs:219-226` | `0`/`1`, panics otherwise (this is what `fc8bb6e7a` fixed) |
| `NOVA_RAILGUN_LIVE` | `examples/screenshots/screenshot_railgun.rs:218-225` | `0`/`1`, panics otherwise |
| `NOVA_STANDOFF_TRACE` | `examples/screenshots/loop_goto_standoff.rs:68` | `var_os(..).is_some()`, exactly as documented |

Four of the five carry the 0/1 contract and the fifth is documented as not
carrying it. No finding.

### End-of-night release state, measured

`gh release view v0.13.2` - published, not a draft, at 20:07:05Z. Assets on the
release right now:

| Asset | Bytes | v0.13.1 | Delta |
| --- | --- | --- | --- |
| `nova-protocol_v0.13.2_linux.tar.gz` | 59,509,772 | 67,597,581 | -8,087,809 |
| `nova-protocol_v0.13.2_web.zip` | 27,693,969 | 35,788,921 | -8,094,952 |
| `nova-protocol_v0.13.2_windows.zip` | 60,092,687 | 68,189,291 | -8,096,604 |
| `nova-protocol_v0.13.2_macOS.dmg` | NOT YET PUBLISHED | 117,767,456 | - |

Two things fall out of that table and neither was estimated:

1. **The skybox downscale is real in the shipped bytes.** All three published
   artifacts are ~8.09 MB smaller than v0.13.1's, within 9 KB of each other -
   which is the signature of one shared asset change, not three builds
   drifting. `924664ddc` ("Ship the sky at a size a browser can hold") is the
   only asset change between the two tags.
2. **The release is public and incomplete.** `build-macOS
   (aarch64-apple-darwin)` in run 34521418070 is still in progress at the time
   of writing, so there is a published, non-draft v0.13.2 with no macOS build
   on it. v0.13.1 shipped four artifacts. Anyone downloading for macOS in this
   window gets nothing. Whether that is the workflow's normal in-flight state
   or a gap worth closing is Alex's call; the fact is recorded, not judged.

Refinement, measured rather than assumed: `git diff --name-only v0.13.1..v0.13.2`
touches 20 files and exactly TWO of them are assets -
`assets/base/textures/cubemap.png` (1,405,634 -> 237,317) and
`assets/base/textures/cubemap_alt.png` (8,705,309 -> 1,546,064). That is
8,327,562 bytes off the tree, against the ~8,090,000 off each shipped archive.
PNG is already compressed, so the two figures agreeing to within 3% is what
should happen. The downscale is the whole of the size change.

### G1 MAJOR re-stated exactly, for the envelope

`web/src/create/scenarios.md:54-58`:

> Size the faces for the memory they cost, not for the detail the source can
> give: every face is uploaded uncompressed, so 1024 px faces cost 24 MB of
> video memory and 4096 px faces cost 384 MB.

24 MB is right for VRAM: 1024 x 1024 x 6 x 4 = 25,165,824 bytes. It is half the
memory the sentence tells a creator to size for. All three shipped
`.png.meta` sidecars set `asset_usage: ("MAIN_WORLD | RENDER_WORLD")`
(`assets/base/textures/cubemap.png.meta:10`, `cubemap_alt.png.meta`,
`assets/mods/example/textures/nebula.png.meta`), so the same bytes are also
held on the CPU side. The project's own CHANGELOG says so at `:26-28`: "24 MB
of video memory instead of 384 MB, AND AS MUCH AGAIN of browser heap."

Failure scenario: a creator sizing a 2048 px sky reads 96 MB off this table,
budgets for it, and pays 192 MB - which on the web build is the difference the
whole v0.13.2 release exists to fix.

### G1 MAJOR sharpened: `credits/CREDITS.md:28` is the ONE path the sweep missed

`924664ddc` (22:12) went through `credits/CREDITS.md` and corrected four stale
asset paths in one pass: `assets/textures/*.png` ->
`assets/base/textures/*.png`, `assets/banner.png` -> `assets/base/banner.png`,
and both cubemap paths under Third-party assets.

I then checked EVERY backticked `assets/...` path in that file against the
filesystem. Fourteen paths, thirteen resolve, one does not:

- `credits/CREDITS.md:28` - `assets/gltf/*.glb`. `assets/gltf` does not exist.
  The 21 `.glb` files are under `assets/base/gltf/`.
- Five lines below it, `credits/CREDITS.md:33` names
  `assets/base/gltf/greebles/*.glb` and resolves to 54 files. The correct
  prefix is in the same list, in the same commit's blast radius.

Line 28 was last written by `eee32c346` (2026-07-15), so the staleness is not
today's - but the pass that would have caught it IS today's, and it corrected
its neighbours on both sides.

`art/README.md:13` carries the same dead path and was not touched today. Those
two are the only stale asset paths in `credits/` and `docs/` combined.

This is a shipped LICENCE file naming a directory that is not in the build. It
stays a MAJOR.

### G1 MAJOR sharpened: the missing face-size assertion is in the files the commit edited

`924664ddc` touched BOTH cubemap test files - `crates/nova_assets/tests/cubemap_meta.rs`
(13 lines) and `crates/nova_core/tests/cubemap_meta_app_config.rs` (14 lines) -
and what it changed in them was to take the concrete size OUT of the prose:

- `cubemap_meta.rs:5-9`: "the raw stacked form (24576 px tall) exceeds the
  16384 texture limit" became "a tall enough stack (a mod may ship 4096 px
  faces, 24576 px stacked) also exceeds".
- `cubemap_meta.rs:47`: "The PNG decode of a 4096x24576 image takes a few
  seconds in dev builds" became "The deadline only bounds a hang".
- `cubemap_meta_app_config.rs:12`: "loads as a single-layer 4096x24576 image"
  became "loads as a single-layer stacked image".

So the commit went INTO the two files that guard this and generalised them away
from the size, without adding an assertion on the size. What survives is only:

```rust
assert_eq!(image.texture_descriptor.array_layer_count(), 6, ...);
assert_eq!(image.height(), image.width(), ...);
```

at `cubemap_meta.rs:62-72` and `cubemap_meta_app_config.rs:71-79`. Nothing else
in the tree pins a face edge: `grep` for `1024`, `face_size` or `width()` across
those two files and `crates/nova_scenario/tests/skybox_swap_e2e.rs` returns only
the two `image.width()` occurrences above.

Failure scenario: someone re-exports `cubemap.png` from space-3d at its 4096 px
default and commits it. `array_layer_count() == 6` still holds. `height() ==
width()` still holds. Every test in the repository passes, the sky is back to
384 MB of VRAM and as much again of heap, and the release that exists to fix
exactly that is silently undone.

This is the single most valuable thing in the night's report.

### G3 craft lane - adjudicated

Thirteen findings. I re-derived the three load-bearing ones and spot-checked the
rest.

**MAJOR - `crates/nova_wfc/src/collapse.rs:468-475` - the new erosion docstring
names a mechanism that is one grid row away from the cells it explains.**

Verified in the tree:

- `collapse.rs:50-52`: `let stern = if z == length - 1 { self.vacuum.stern }
  else { 0.0 };`. The stern bonus applies to ROW 10 only (`length: 11`,
  `grammars.rs:52`; `stern: 9.0`, `:65`).
- `collapse.rs:266`: the deck is seeded at `self.grid.index(x, y, length - deep
  - 1)` = z 9 for the shipped one-cell drive. `collapse.rs:285`: the drive is at
  `length - deep` = z 10.
- The deck cell is (1, 2, 9). Its six neighbours are (0,2,9) the seeded keel,
  (1,2,10) the seeded drive, and four ROLLED cells - (2,2,9), (1,1,9), (1,3,9),
  (1,2,8). None of the four is in row 10, so `vacuum.stern` never touches any
  cell the deck's support count reads.

What actually leaves those four bare is `vacuum.taper` on `off_keel` (2, 2, 2
and 1 respectively), not the stern bonus. The docstring's causal chain - "Vacuum
is priced up in the last row so a drive has somewhere to stand, so a bare
transom is the common case: the drive deck then carries two neighbours" - is
therefore false as stated, even though its CONCLUSION (the deck is left holding
2 against `SPIKE_SUPPORT` = 3) is exactly right.

Failure scenario: someone retunes `vacuum.stern` down to make the transom
denser, reads this paragraph, concludes the deck now has neighbours, and drops
the `seeded[cell] ||` guard at `collapse.rs:485`. Nothing changed at z 9. The
no-thruster hull ships again - the defect the `system_ship_editor` probe caught
in CI this morning.

The same two claims are repeated at `crates/nova_wfc/src/tests.rs:471-477` and,
abridged, at `collapse.rs:695-698`, so a correction has three copies to chase.

One nuance the lane did not separate and I will: "a bare transom is the common
case" is plausibly true OF ROW 10, and what is rare (about one seed in forty, by
the commit's own sweep) is all four of the deck's ROLLED neighbours coming up
vacuum at once. The sentence is wrong because it joins those two facts with a
"then", not because either half is a lie.

**MAJOR - `crates/nova_ship/src/flight/state.rs:272-274` - `flip_point`'s
docstring still states the exact inference `28a14db97` exists to delete, on the
same struct as the field that deletes it.**

Verified, and the two are eleven lines apart:

- `state.rs:258-262`, the NEW `braking` field: "`None` for three different
  reasons, and only one of them is 'the brake has begun'. Reading the absence as
  the reason turned a leg whose coast estimate was merely unavailable into a
  committed brake, which pointed the drive retrograde and then starved it - a
  state nothing could leave".
- `state.rs:272-274`, `flip_point`: "`None` once braking has begun (or the
  estimate is meaningless at near-zero closing speed)." Two reasons, braking
  first.

Failure scenario: the next HUD author wanting to shade the brake leg reads the
field doc and writes `if telemetry.flip_point.is_none() { brake_leg() }`. The
readout says BRAKING for every leg under the flip floor and every leg the well
out-pulls - which is the hang, back on the instrument side.

**MINOR - `crates/nova_hud/src/holo_instruments.rs:293`** - the test fixture is
`braking: flip.is_none()`, hard-coding the deleted inference. Every telemetry it
builds is self-consistent under the OLD rule by construction, so no test built
on it can catch an instrument that confuses the two - which is the whole subject
of the commit. Verified verbatim.

**MINOR - `crates/nova_ship/src/flight/tests/stop.rs:329`** - the test docstring
still reads "a residual drift below the attitude deadband". `b16321d12` raised
the fixture from 0.3 to 0.5 u/s and `attitude_deadband` is 0.4
(`state.rs:440`), so 0.5 is ABOVE it. The band that applies to a STOP is
`settle_deadband` = 0.75 (`state.rs:445`). Verified all three numbers.

Nine further MINORs kept as recorded by the lane, not independently re-derived:
`tests.rs:485` (the test name asserts a bare transom the test never
constructs); `tests.rs:499` (`0..64` is a magic number where every sibling loop
in the file is `0..8`); `autopilot.rs:878-892` (the rewrite is longer and the
added material is changelog, plus a ragged rewrap at `:890`);
`guidance.rs:90-91` (the enum's stated justification - "three because the caller
acts differently on each" - is false at two of its three call sites, which merge
`Braking | Unknown`); `guidance.rs:150-160` (`arrival_eta` re-tests both guards
`goto_flip_point` already tests, keeping a dead arm alive with a comment
explaining that it is dead); `collapse.rs:181,216,274,292` (the `seeded[cell] =
true` mark is paired to `assign` by hand at four sites and
`docs/ship-layout-sense.md` now promises a future seeded tower "inherits the
exemption"); `collapse.rs:438` (`widen_to_whole_parts` calls itself `drop_part`'s
counterpart and then uses a different traversal for the same graph - and the
lane established neither is at stack risk, because joints are built per part);
`stop.rs:340-342` and `goto.rs:681-682` (settings figures hand-typed against the
house rule `holo_instruments.rs:279-282` states in this same commit);
`goto.rs:680` (body comment where every sibling test in the file uses a
docstring).

**`ecb504b6f` cleared.** The lane confirms the `drop(transform)` removal is
inert: `Mut<Transform>` sets its change tick on `DerefMut` at the assignments,
not at drop, and NLL ends both borrows before `run_pipeline(&mut world)`.

Counting refinement on the `guidance.rs:90-91` MINOR, measured rather than
taken: `FlipEstimate` is consumed in four places, and three of them merge two
arms.

| Site | Shape |
| --- | --- |
| `guidance.rs:182` (`arrival_eta`) | `FlipEstimate::Braking \| FlipEstimate::Unknown =>` |
| `autopilot.rs:474` (`flip_point`) | `FlipEstimate::Braking \| FlipEstimate::Unknown =>` |
| `autopilot.rs:478` (`seconds_to_flip`) | `FlipEstimate::Braking \| FlipEstimate::Unknown =>` |
| `autopilot.rs:468` (`braking`) | `flip == FlipEstimate::Braking` - the only one that separates them |

So "three because the caller acts differently on each" is false at three of the
four. The cut is still right - the ONE site that separates them is the field the
whole commit exists to add - but the sentence justifies it on a property the
code does not have.

### G3 correctness lane - adjudicated

The lane answered my held question and corrected half of my premise. Both are
recorded as it found them.

**My premise was half wrong.** I wrote that the two axial regression tests
"assert only `speed < 0.5` - under the 0.75 band and over the 0.2 epsilon, so
neither can catch a regression to the band in either direction". The first half
is the point: 0.5 sits BETWEEN the epsilon and the band, so a leg released at
the band would land at ~0.749 and FAIL both tests. They discriminate. What they
do not cover is a different case, below.

**An aligned axial STOP does still reach the epsilon.** `fine`
(`autopilot.rs:932`) gates only the rotation block at `:1051`. The allocation
loop, the throttle demand and `done` all sit outside it, and `done` requires
`error_speed <= stop_speed_epsilon` whenever `firing_authority > 0`. So
`state.rs:381-384` holds for a drive already inside the align cone.

**MAJOR - `crates/nova_ship/src/flight/state.rs:381-384` - the contract now
holds only when the drive is ALREADY ALIGNED. A head-on residual inside the band
is handed back un-braked.**

I re-derived the whole path:

- `autopilot.rs:529-533`: with no telemetry yet, a STOP publishes only above
  `2.0 * stop_speed_epsilon` = 0.4 u/s.
- `autopilot.rs:912-915`: `owed = velocity.length()`; at 0.5 u/s,
  `owed > crumb_band` (0.75) is FALSE, so `brake = None` on the first tick.
- `autopilot.rs:932`: `fine = error_speed <= crumb_band && brake.is_none()` is
  true, so the rotation block at `:1051` never runs and the hull never starts
  its 180.
- `autopilot.rs:836-839`: `firing_authority` accumulates only for an engine with
  `dir.dot(error_dir) >= gate`. A single centred drive still pointing prograde
  contributes nothing, so `firing_authority == 0.0`.
- `autopilot.rs:938-940`: `done` = `error_speed <= 0.2 || (fine &&
  firing_authority <= 0.0)`. The second disjunct fires on tick one.

Result: STOP engaged while coasting head-on between 0.4 and 0.75 u/s, on an
RCS-withheld hull with one centred drive, disengages having burned nothing and
leaves the ship drifting at up to 0.75 u/s = 7.5 m/s. Before `b16321d12` the
brake committed, the hull flipped and the drive braked to 0.2 u/s = 2 m/s.

`state.rs:381-384` names that exact hull - "the shipped single-centered-drive
ship" - as the case that "still brakes to `stop_speed_epsilon` EXACTLY". A
retro-equipped hull is unaffected.

Not a BLOCKER: the residual is bounded by the band `settle_deadband` already
documents accepting, an RCS-granted hull still settles, and
`AI_IDLE_DRIFT_SPEED = 1.0` (`passive.rs:39`) sits above the band so nothing
churns. Worth fixing because the doc now promises something the code stopped
doing, and NO test in the range covers the case in either direction.

**MINOR - `crates/nova_hud/src/holo_instruments.rs:293`, plus
`holo_instruments.rs:205-206` and `maneuver_instruments.rs:238-239`** - the new
`braking` flag has no consumer outside the autopilot. Both instruments still
read `flip_point`, which is right for an ANCHOR you cannot draw without, but
their docstrings and the test names
`flip_gate_faces_the_path_and_dies_when_braking` /
`flip_marker_hides_once_braking` assert the equivalence this commit spent 136
lines disproving. A GOTO engaged at 0.3 u/s publishes `braking: false,
flip_point: None`; both instruments blank and both tests call that braking.
Raised by the craft lane too, from the fixture end.

**MINOR - `crates/nova_ship/src/flight/autopilot.rs:409`** - the inside-standoff
arm publishes `braking: false` while it is in fact braking to rest. Inert today
(that arm also sets `brake_accel: 0.0`, which fails `due` anyway), but
`state.rs:255` defines the field as "PAST its flip point and braking", which is
what that arm is.

**MINOR - `crates/nova_wfc/src/tests.rs:499`** - the pinned 64 seeds is thin
(~1.6 expected hits at the stated 1-in-40), `tests.rs:488` sets
`bow_gun = Some("railgun_lance_section")` where the shipped grammar has
`bow_gun: None` (`grammars.rs:78`), and `widen_to_whole_parts` is UNPINNED - the
shipped drive is one cell, so widening is a no-op on the very cell this test
asserts. The lane timed it: 36 ms a seed, so 512 seeds costs ~18 s.

**MINOR - `crates/nova_wfc/src/collapse.rs:697-701`** - "The SEEDED spine is
spared by erosion" is broader than the code: only `erode_studs` consults
`seeded`. `erode_blocked_exits` (`:675`) drops parts through the same
`drop_part` and never does. No live failure grounded - both seeded lanes exit
the grid - but the next seeded role with an inboard exit inherits a promise
nothing keeps.

**Both structural questions in my dispatch answered, and answered well:**

(a) The two predicates ARE exact inverses and the mark cannot over-widen.
`joints[face]` holds a per-rotation SEGMENT index (`tiles.rs:66-68`, remapped at
`:194`), so a tile index fixes a segment's local cell within its block. If `c`
holds `T(p)` and `c+f` holds `T(p+f)`, both blocks have origin `c - R(p)`. Two
distinct copies of the same part at the same rotation can never satisfy it.

(b) A seeded part CANNOT be stranded for `keel_component` to prune. The spine is
connected to the keel through seeded cells only, and the deck always covers
`keel_row` because `bottom <= keel_row` and `bottom + tall > keel_row` by
construction (`collapse.rs:259`). After widening, `seeded` is closed under joint
adjacency, so a `drop_part` started on a non-seeded cell only ever reaches
non-seeded cells.

Lane ran 72 Rust tests green across five filters.

Nuance on the `tests.rs:488` `bow_gun` override, which I checked rather than
took: `grammar.keel.bow_gun = Some("railgun_lance_section".to_string())` does
diverge from the shipped `bow_gun: None` (`grammars.rs:78`), but `f3a78a434`'s
own account is that the defect was found by "the editor's Generate", and the
editor seeds the lance. So the override REPRODUCES the failing configuration
rather than drifting from it. The lane's suggested fix - a second loop without
the override - is still worth having; the override itself is not the slip.

The lane's other two points on that test stand as written: the name promises a
bare transom the body never constructs, and `widen_to_whole_parts` is unpinned
because the shipped drive is one cell.

### G3 contracts lane - adjudicated

**MAJOR - `web/src/wiki/flight-autopilot.md:38`, `web/src/wiki/glossary.md:15`
and `:18`, `web/src/wiki/getting-started.md:53`,
`web/src/create/actions.md:945-948` - STOP's player- and creator-facing
description is the verb `b16321d12` changed, and no page was updated.**

This is the most player-visible thing the night found. Verified verbatim in the
tree:

- `flight-autopilot.md:38`: "**STOP** - flips to retrograde and burns until you
  are at rest".
- `glossary.md:15`: "STOP flips you to retrograde and burns to kill your speed."
  `:18`: "STOP flips retrograde and burns to rest."
- `getting-started.md:53`: "Press X once and let STOP bring the trainer to
  rest."
- `create/actions.md:945-948`: `StopShip` "Completes when the ship is at rest,
  with `kind: Stop`... the cheap way to make a beat wait for 'it definitely is
  not drifting any more'."

The newly accepted window is 4 to 7.5 m/s: below `2 * stop_speed_epsilon` = 0.4
u/s a STOP publishes no telemetry at all (`autopilot.rs:529-533`), and up to
`crumb_band` = 0.75 u/s = 7.5 m/s the brake is now refused
(`autopilot.rs:915`). One world unit is 10 m.

Failure scenario, in the campaign case `flight-autopilot.md:86` names itself
("the mainline campaign flies with RCS withheld"): a player drifting sideways
at 6 m/s presses X. Nothing turns, no plume lights, the AP chip clears on the
spot, and the ship keeps its 6 m/s - 360 m of drift a minute. The creator
version is worse because it is silent: a scenario author uses `StopShip` as a
"not drifting any more" gate, gets `kind: Stop` with the hull still moving at up
to 7.5 m/s, and the next beat's radius filter misses.

**MAJOR - `examples/screenshots/shared/ring.rs:544-546`, `:552`, `:562` - the
capture harness still infers braking from a missing flip point.**

Verified verbatim. The docstring asserts the equivalence as fact - "the
telemetry drops its flip point once the brake is planned, which is the same
instant the computer turns the ship around" - and both predicates act on it:
`player_braking()` is `flip_point.is_none() && closing_speed > 5.0`,
`player_retro_burning()` is `flip_point.is_none()` with NO speed gate.

`flip_point` is `None` in four distinct states, only one of which is a brake.
Engage GOTO on a nav lock 300 m away while closing at 8 u/s and
`player_braking()` returns true on the first published tick, before the hull has
turned. Not a BLOCKER: in both shipped scripts `player_braking()` gates
`player_retro_burning()` and both rings fly a long deep-space leg, so neither
reaches the state today.

**MAJOR - `crates/nova_ship/src/flight/state.rs:272-274`** - the same finding
the craft lane raised, reached independently. Kept once. The contracts lane adds
the enumeration: `flip_point` is now `None` in FOUR states -
`FlipEstimate::Braking`; `Unknown` from `closing_speed < FLIP_ESTIMATE_FLOOR`
(0.5 u/s = 5 m/s, which the docstring calls "near-zero"); `Unknown` from the
well pull eating the brake authority (`guidance.rs:124`); and every tick a GOTO
spends inside its standoff (`autopilot.rs:400-413`, which publishes
`braking: false` explicitly).

**MINOR - `crates/nova_hud/src/maneuver_instruments.rs:238-239` and
`crates/nova_hud/src/holo_instruments.rs:204-206`** - two PRODUCTION docstrings
state the withdrawn equivalence ("hidden once the ship is braking (the autopilot
stops predicting a flip)"). `28a14db97` touched both files but only their test
fixtures.

**MINOR - `CHANGELOG.md:52-54` and `web/src/news/0.13.0.md:1094`** - "for the
last m/s it is allowed to accept". The band is 7.5 m/s. Verified: the entry
reads "A STOP no longer pirouettes the hull for the last m/s it is allowed to
accept". AGENTS.md requires player-facing figures in meters, and this is a
figure - 7.5x short. Reads as idiom as well as a quantity, so MINOR.

**MINOR - `docs/ship-layout-sense.md:224-227`** - the edited risk note retires
more than the code does. The exemption is in `erode_studs` alone;
`keel_component` (`collapse.rs:589`) and `erode_blocked_exits` (`:675`) have
none. The note generalises to "a tower seeded the way the stern is inherits the
exemption", and the stern deck only survives `keel_component` because it sits
beside the keel - a roof tower would not.

**MINOR - `crates/nova_ship/src/input/ai/passive.rs:37`** -
`AI_IDLE_DRIFT_SPEED`'s justification names `stop_speed_epsilon` as the bound a
completed STOP has to clear. After `b16321d12` that bound is `settle_deadband`
(0.75), so the margin fell from 5x to 1.33x and the comment points at the wrong
knob. `settle_deadband` is a reflected field for a future settings menu; raise
it past 1.0 and an idling AI hull re-engages STOP forever.

**MINOR - `web/src/create/grammars.md:163-164` and `:148`** - the guarantee
`f3a78a434` bought is not stated creator-side, and `:148`'s "If the drives are
missing, raise `stern`" is advice that made the symptom MORE likely.

**MINOR - `docs/keeping-docs-in-sync.md:74`** - the routing map's row keys are
`nova_ship/input` and `camera`. Both flight commits land in
`crates/nova_ship/src/flight/`, which no row names - which is the mechanism
behind the stale wiki pages above.

### The one lane disagreement in G3, settled

Correctness and contracts split on `state.rs:381-384`. They derived the SAME
mechanism and disagreed only on whether the sentence covers the case:

- Contracts: the sentence says "an AXIAL residual ... KEEPS THE DRIVE'S ALIGNED
  AUTHORITY", so it is talking about an aligned drive, and for an aligned drive
  it still holds. True.
- Correctness: "axial" names the drive AXIS, and half of that axis is head-on,
  where the drive has no aligned authority until it flips - and the flip is what
  `b16321d12` suppressed below the band. Also true.

Settled: the sentence is ambiguous and both readings are available to a reader,
which is itself the defect. Both lanes propose the same edit - say ALIGNED, not
axial, and pin the head-on band case with a test. Kept as one MAJOR, with the
correctness lane's derivation as its evidence.

### G3 verdict

Three lanes, all reported. Five MAJORs, eighteen MINORs, no BLOCKER. Nothing
found in the CODE the four commits landed: 198 Rust tests were run green across
the three lanes, the two structural WFC questions I carried in both came back
provably safe, and `ecb504b6f` is inert by compilation.

What is wrong is again almost entirely WORDS - but unlike G2, one of them is in
front of players. `b16321d12` changed what pressing X does and the wiki, the
glossary, the tutorial page and the `StopShip` creator contract all still
describe the old verb.

## Night's tally

54 commits landed today, 10:05 to 22:27, grouped into 8 reviewable groups all
under `/nova-review`'s 2000-changed-line refusal. Three groups reached by lanes:

| Group | Lanes | Result |
| --- | --- | --- |
| G1 skybox / bundle error / v0.13.2 | 4 | 4 MAJOR, 9 MINOR |
| G2 pyre, shed, chip budget, widgets | 4 | 3 MAJOR, 16 MINOR |
| G3 flight guidance, rest legs, WFC spine | 3 | 5 MAJOR, 20 MINOR |

Five groups NOT reached by lanes: G4 tooling+CI, G5 comic engine, G6 widgets +
illustration, G7 the episode, G8 docs/release. I read parts of four of them
myself and recorded what I could ground - three MINORs and a set of verified
claims - but my own reading is not a lane review and is not counted as one.

Totals across everything written above: **0 BLOCKER, 12 MAJOR, 48 MINOR**, each
anchored at a `file:line` in this task.

Eleven lanes ran tonight. Not one of them found a defect in the CODE that
landed today. Every MAJOR is a sentence: a design-doc chapter that inverts the
fix it documents, a test that stopped guarding the size it exists to guard, a
licence file naming a directory that is not in the build, three wrong hull
figures, a comment that denies the constant twelve lines below it, and a wiki
that tells players what pressing X used to do.

Measurement: none, twice refused. The box carried another session's
`weapon-machines-showcase` capture under Xvfb :100 at loadavg above 10 for the
whole night. Both performance lanes declined the slot rather than publish a bad
number, which is the right call and is recorded here as an explicit skip, not a
pass. No `--play` was run.

Nothing in the working tree was changed. This task file is the only thing
written.

## Documentation pass, 2026-09-11

Requested scope: documentation only - prose docs, wiki and create pages,
credits and readmes, changelog wording, and source comments and docstrings
where they are documentation. Runtime behavior, tests, assets, CI and the
release workflow are out of scope by the request, and the owner is explicitly
not interested in those findings. Every finding written above is dispositioned
here: corrected, declined, or verified and left alone. Nothing is left
unanswered.

Work happened in the Sprout worktree `nightly-docs-update`, off `b3c6f579c`.
This task file was untracked in the main checkout and was copied in with its
evidence. Every Rust change in the commit is a comment line - checked with
`git diff -U0 -- '*.rs'`, which reports no non-comment insertion or deletion -
so no runtime behavior moved.

### Corrected

| Finding | What changed |
| --- | --- |
| G1 MAJOR `credits/CREDITS.md:28` | `assets/gltf/*.glb` -> `assets/base/gltf/*.glb`. |
| G1 MINOR `art/README.md:13` | The same path. Both were the only survivors of `grep -rn "assets/gltf/"`. |
| G1 MINOR `credits/CREDITS.md:47` | The Bevy icon entry now names `build/icon_1024x1024.png` and its derivatives (`build/macos/AppIcon.iconset/*.png`, the `.icns`, `build/windows/icon.ico`), says `build.rs` embeds the `.ico` and `index.html` copies it into the web build, and separates `web/src/favicon.svg` as the project's own. Every path checked on disk; `build.rs:7` compiles `build/windows/icon.rc` and `index.html:9` copies the `.ico`. |
| G1 MAJOR `web/src/create/scenarios.md:54-58` | The figure now names both halves: 24 MB of video memory AND 24 MB of main memory at 1024 px, 384 MB of each at 4096 px, one budget on the web build. |
| G1 MINOR `web/src/create/scenarios.md:54` | The link now leads somewhere that documents the sidecar: `web/src/create/base-content.md` states `array_layout: Some(RowCount(rows: 6))`, `asset_usage: ("MAIN_WORLD \| RENDER_WORLD")`, which file a modder copies, and what a missing sidecar costs. |
| G1 MINOR `web/src/news/0.13.0.md:1129` | "max-pooled rather than averaged down" -> cut into faces first, then max-pooled at the reduction factor BEFORE the box average, divided by sixteen. |
| G1 MINOR `web/src/news/0.13.0.md:1131` | 8.4 MB -> 8.3 MB (10,110,943 - 1,783,381 = 8,327,562 B). |
| G1 MINOR `crates/nova_assets/src/merge.rs:226` (the doc copy) | `web/src/news/0.13.0.md:1146` now says "Mods > Explore online", the name `crates/nova_menu/src/menu_ui.rs:300` gives the tab. The runtime string itself is declined below. |
| Swept while fixing the above, not a review finding | Three more documentation copies of the wrong tab name: `web/src/news/0.8.0.md:104`, `CHANGELOG.md` `[0.7.0]` (The Ledger's entry) and `webmods/the-ledger/README.md:8`. The button has read "Explore online" since `fd4c86af9` (2026-07-15), before v0.8.0, so all three were wrong when written. Corrected. |
| G1 MINOR `crates/nova_scenario/src/actions/view.rs:485`, `:715` | "hundreds-of-MB cubemap texture" -> 24 MB at the shipped 1024 px faces, 384 MB if a mod ships 4096. |
| G1 MINOR `crates/nova_assets/src/collections.rs:390-396`, `:594-595`; `tests/cubemap_meta.rs:5`; `crates/nova_core/tests/cubemap_meta_app_config.rs:13-16` | The failure mode is stated correctly: a reinterpret rescues the BINDING and not the upload, and a non-Cube view fails `sanity_check_skybox_image_and_warn` on EVERY GPU. The "upload race" and "silently disappears on a 16384-limit GPU" framings are gone. |
| G1 MINOR `crates/nova_assets/src/merge.rs:143-144` | The comment that restated the next line is trimmed to its reason. |
| G2 MAJOR `docs/sections.md:517-541` (all four lanes) | The chapter now describes the shipped schedule: `Update`, the three run conditions, why the camera is a requirement (the hanabi `BufferTable` panic), and that `PostStartup` is exactly the shape that panics. `:1074` points at this passage and is now correct by fixing it. |
| G2 MAJOR `crates/nova_ship/src/sections/fixture.rs:69-74` | The drain is the lesser of 24 a tick and `SHED_FRAME_CAP` a frame; the sixth of a second is qualified as the frame-rate-permitting case, with 0.55 s at 10 fps and 1.37 s at the 4 fps `max_delta` floor, and "24 fixtures a tick, one drain" replaces the ambiguous "24 a tick". |
| G2 MAJOR `crates/nova_gameplay/src/integrity/pyre.rs:239-240`, `:254-255` | The three `HULK_PYRE` spans are 90 m, 220 m and 370 m, derived here from `assets/base/ships/base.content.ron` and `assets/base/sections/base.content.ron` with `aabb_half_extents = size * 0.5` and the unit-cube default, and a new sentence says the spans are outer FACE to outer face and include the multi-cell drive overhang. |
| G2 MINOR `fixture.rs:234-237` | Names the third production trigger, ram damage through `on_impact_collision_deal_damage`, and why `CollisionEventSystems` inside `PhysicsStepSystems::Finalize` needs no ordering of its own. |
| G2 MINOR `fixture.rs:98-100` | `ShedBudget`'s doc no longer claims `PyreBudget` shares its shape, and says why `PyreBudget` counts up instead. |
| G2 MINOR `fixture.rs:304-315` | Took the documentation half of the review's either/or: `SHED_FRAME_CAP` now says it counts PLATES and that each plate costs a descendant walk. |
| G2 MINOR `pyre.rs:559-561` | The "an earlier cut warmed in `PostStartup`" history is now stated as a constraint, per `AGENTS.md`. |
| G2 MINOR `pyre.rs:106`, `:647` | The two ragged doc wraps. |
| G2 MINOR `CHANGELOG.md:63-65` | "and again when the tier changes" described a run `pyres_are_cold` makes impossible. The entry now says the warm-up runs as soon as the tier has settled and a camera is up, including a tier raised mid-run off a particle-less preset - which is the FIRST run. |
| G2 MINOR `web/src/news/0.13.0.md:957` | "one crater is untouched" -> a wide crater costs seven, eighteen fit, the nineteenth gets a short carve out of what is left, and the twentieth on goes unchipped. Matches `spew.rs:567` (`look.count(radius).min(budget.left)`) and the widget's `Math.ceil` rule. |
| G2 MINOR `crates/nova_scenario/src/objects/spaceship.rs:155` | The third copy of the "~20 m floor" claim now matches `passive.rs:29-31` and `web/src/create/objects.md:309`. |
| G2 MINOR `CHANGELOG.md` `[0.13.1]` | `33dcf22c3` has its entry, beside its two siblings in Fixes. |
| G8 MINOR the four out-of-order released blocks | `[0.11.0]`, `[0.10.0]`, `[0.7.0]` and `[0.2.0]` now follow the order `CHANGELOG.md:6-8` declares. Reordered by script, whole `###` chunks moved intact, and the line multiset of the file verified identical before and after. |
| G3 MAJOR `crates/nova_ship/src/flight/state.rs:272-274` | `flip_point` enumerates all FOUR `None` states and sends the reader to `braking`. |
| G3 MAJOR `state.rs:381-384` (the lane disagreement) | The contract now turns on ALIGNMENT, not on the axis: an already-pointed residual brakes to `stop_speed_epsilon` exactly, a residual the drive is not pointing at - a damage-shifted drift OR a head-on crumb inside the band - is released at up to the band, and an RCS-granted hull still settles. The test half is declined below. |
| G3 MAJOR `examples/screenshots/shared/ring.rs:544-546`, `:552`, `:562` | The docstrings no longer assert the withdrawn equivalence. The predicates are code and are declined below. |
| G3 MAJOR `crates/nova_wfc/src/collapse.rs:468-475`, `:695-698`, `tests.rs:471-477` | The causal chain is corrected in all three copies: `vacuum.stern` prices row `length - 1` only, the deck sits at z 9, and what leaves its four rolled neighbours bare is `vacuum.taper` on `off_keel`. The conclusion (2 against `SPIKE_SUPPORT` = 3) was right and is kept. |
| G3 MAJOR the STOP verb, player- and creator-facing | `web/src/wiki/flight-autopilot.md:38` states the band; `web/src/wiki/glossary.md:15` and `:18` drop "burns to rest"; `web/src/wiki/keybinds.md:52` and `web/src/wiki/getting-started.md:53` drop the absolute claim; `web/src/create/actions.md` (the `StopShip` row and its contract) and `web/src/create/events.md` (`OnStopComplete`, both the table row and the section) say completion is a deadband and point a beat that needs the hull still at a `Speed` query. Figures in meters: 2 m/s aligned, up to 7.5 m/s not. |
| G3 MINOR `CHANGELOG.md:52-54` and `web/src/news/0.13.0.md:1094` | "the last m/s" -> 7.5 m/s, the band `settle_deadband` = 0.75 u/s gives. |
| G3 MINOR `crates/nova_hud/src/maneuver_instruments.rs:238-239`, `holo_instruments.rs:204-206` | Both production docstrings key on `flip_point` and list its four states instead of asserting the equivalence. |
| G3 MINOR `crates/nova_ship/src/flight/guidance.rs:90-91` | The `FlipEstimate` justification says what the measured table says: one of the four consumers separates `Braking` from `Unknown`, and it is `ManeuverTelemetry::braking`. |
| G3 MINOR `crates/nova_ship/src/flight/tests/stop.rs:329` | "attitude deadband" -> settle deadband, the band that applies to a STOP. |
| G3 MINOR `crates/nova_ship/src/flight/tests/goto.rs:680` | The body comment is a docstring, like every sibling test. |
| G3 MINOR `crates/nova_ship/src/flight/autopilot.rs:890` | The ragged rewrap; the paragraph is re-flowed to the file's width. |
| G3 MINOR `collapse.rs:697-701` | "The SEEDED spine is spared by erosion" is narrowed to `erode_studs`, the only pass that consults `seeded`. |
| G3 MINOR `collapse.rs:438` | `widen_to_whole_parts` now says why its traversal differs from `drop_part`'s and that neither is at stack risk. The unification itself is code and is declined below. |
| G3 MINOR `crates/nova_ship/src/input/ai/passive.rs:37` | `AI_IDLE_DRIFT_SPEED` names `settle_deadband` (0.75 u/s) and the 1.33x margin, not `stop_speed_epsilon`. |
| G3 MINOR `docs/ship-layout-sense.md:224-227` | The exemption is `erode_studs` alone. The note now says `keel_component` and `erode_blocked_exits` never read the mark, and that a roof tower needs its own answer. |
| G3 MINOR `docs/keeping-docs-in-sync.md:74` | The row keys `nova_ship/flight` (and `nova_ship/camera`, and the maneuver instruments in `nova_hud`), and routes to `glossary.md`, `hud.md`, `/create/actions/` and `/create/events/` - the pages this night found stale. |
| G3 MINOR `web/src/create/grammars.md:148`, `:163-165` | `:148` no longer sends a creator to `stern` for a missing seeded drive, and `:163-165` states the guarantee `f3a78a434` bought, including that a whole multi-cell part is covered. See the unsupported half below. |

### Declined - not documentation, and out of scope by the request

Runtime behavior:

| Finding | Why |
| --- | --- |
| G1 MAJOR `crates/nova_assets/src/merge.rs:216-228` - the error accuses a healthy mod on a "not loaded yet" state | The fix is a gate on `bundle.content`. That is behavior. The strongest finding of the night and it stays open. |
| G1 MINOR `merge.rs:226` - the error names "Mods > Explore" | A player-facing runtime string, and coupled to the branch above. The same wording in `web/src/news/0.13.0.md:1146` is documentation and was fixed. |
| G1 MINOR `merge.rs:221` - the `"<unknown>"` fallback cannot occur | Code. |
| G1 MINOR `merge.rs:169-172` - the terse bundle branch is unreachable | Code. |
| G2 MINOR `pyre.rs:508`, `:627` - the predicate written twice | Code. |
| G2 MINOR `pyre.rs:796-799` - three `run_if`s do not short-circuit | Code, and unmeasured. |
| G2 MINOR `pyre.rs:801` - `.after` inserts an `ApplyDeferred` every frame | Code, and unmeasured. |
| G2 MINOR `pyre.rs:686` - `light_the_pyre` carries no view gate | Code. The lane could not ground a shipped path that reaches it. |
| G3 MAJOR `examples/screenshots/shared/ring.rs` - the predicates themselves | `player_braking()` and `player_retro_burning()` are harness code. Their docstrings were corrected; the predicates are untouched. |
| G3 MINOR `autopilot.rs:409` - the inside-standoff arm publishes `braking: false` while braking to rest | Code. Inert today, because that arm also sets `brake_accel: 0.0`. |
| G3 MINOR `guidance.rs:150-160` - `arrival_eta` keeps a dead arm alive | Code. |
| G3 MINOR `collapse.rs:181`, `:216`, `:274`, `:292` - the `seeded` mark is paired to `assign` by hand | Code. |
| G3 MINOR `collapse.rs:438` - unify the two traversals | Code. The doc note was added instead. |
| G6 MINOR `crates/nova_channel/src/apply.rs:282-291` - a `stop` can lift a source the wire never pushed | Code. |

Tests and test rigs:

| Finding | Why |
| --- | --- |
| G1 MAJOR `crates/nova_assets/tests/cubemap_meta.rs:62-72` and `crates/nova_core/tests/cubemap_meta_app_config.rs:71-79` - nothing pins the face size | The fix is `assert_eq!(image.width(), 1024)` in two test files. The review calls this the single most valuable thing in the night's report, and it is a test change: declined here, unfixed, and it should be the first thing picked up when tests are back in scope. |
| G2 MINOR `fixture.rs:340-342`, `:530-532`, `shell_skin.rs:1152-1156`, `:1217-1219` - the shed wiring is hand-copied into four apps | Three of the four sites are test rigs and the fix is to build the rig from the plugin. Code and tests. |
| G2 MINOR `passive.rs:872` - `patrol_world()` spawns no `ComputedCenterOfMass` | Test fixture. |
| G2 MINOR `web/tests/widgets.test.ts:665-690` - `ZONE_PARTS` is passed as literals | Test. |
| G3 MAJOR `state.rs:381-384` - "pin the head-on band case with a test" | The doc half is fixed; the test is declined. No test in the range covers that case in either direction. |
| G3 MINOR `holo_instruments.rs:293` - the fixture hard-codes `braking: flip.is_none()` | Test fixture. |
| G3 MINOR `crates/nova_wfc/src/tests.rs:485`, `:499` - the name promises a transom the body never builds, 64 seeds is thin, `widen_to_whole_parts` is unpinned | Tests. |
| G3 MINOR `stop.rs:340-342`, `goto.rs:681-682` - settings figures hand-typed | Tests. |

Tooling, CI and the release workflow:

| Finding | Why |
| --- | --- |
| G4 `registered_examples()` returns `None` on a manifest with no `[[example]]`, and `check_producers` then skips silently | Tooling code. |
| G6 `scripts/deploy-pages.py:55` and `scripts/probe-summary.py:62` - `read_text()` without `encoding='utf-8'` | Code. Neither is failing today. |
| G5 `tasks/20260908-161328/clean-line-study/generate.py:18` imports the deleted `speech()` | Code in a task artifact, knowingly accepted by `e6bd754bf`. |
| Baseline: v0.13.1 was tagged, built and published from a tree whose own CI run had failed twice, because nothing in release-flow re-runs the CI gate | Release workflow. |
| End-of-night: v0.13.2 was published non-draft while `build-macOS (aarch64)` was still running, so there was a window with no macOS artifact | Release workflow. |

Measurement, not documentation:

| Finding | Why |
| --- | --- |
| G1 correctness lane's open item - whether a max-pooled star reads chunky at 1024/face needs a rendered frame | No render was possible on the review host and none was taken here. Stays explicitly open. |
| G1 "not raised" - the 11.4 texels/deg magnification arithmetic | The review recorded it and did not raise it. Nothing to correct. |

### Verified against the tree and left alone

- **`web/src/create/grammars.md:148` - half the finding is unsupported.** The
  review says "If the drives are missing, raise `stern`" is advice that "made
  the symptom MORE likely". It did not. `collapse.rs:50-52` prices `vacuum.stern`
  on row `length - 1` (z 10 for `length: 11`), the eroded cell is the drive deck
  at `length - deep - 1` = z 9, and none of the deck's four rolled neighbours -
  (2,2,9), (1,1,9), (1,3,9), (1,2,8) - is in row 10. Raising `stern` cannot
  change the deck's support count. The sentence was still reworded, for the
  other reason: after `f3a78a434` the seeded pair stands whatever `stern` is set
  to, so the advice pointed at the wrong knob for the symptom a creator would
  bring to it.
- **`web/src/wiki/getting-started.md:53` was not wrong for the beat it
  describes.** The trainer reaches ALPHA under a 150 m/s cap, far above the
  band, and an aligned brake still reaches `stop_speed_epsilon` = 2 m/s
  (`autopilot.rs:938-940`, with `fine` gating only the rotation block at
  `:1051`). The line was softened anyway, because the player can also press X
  at a slow drift, which is the case the band now accepts.
- **`autopilot.rs:878-892` - "the added material is changelog" does not hold.**
  Re-read in full: the paragraph states constraints (why the band does not judge
  the TICK's error, why the brake-owed floor exists, what a single drive does
  when aimed at the lateral crumb) and names no earlier cut. Nothing was
  deleted. The ragged wrap the same finding names was fixed.
- **`docs/sections.md` was rewritten twice for the deleted design.** The
  refinement above is right, and it is why the chapter was rewritten here rather
  than patched: both `6882fc2c4`'s and `6e6411d99`'s passes described the
  `PostStartup` cut.

### Checks run

| Check | Result |
| --- | --- |
| `nix develop --command cargo fmt --check` | clean |
| `nix develop --command mdbook build` | green; `book/sections.html`, `book/ship-layout-sense.html` and `book/keeping-docs-in-sync.html` read in the rendered output, including the reflowed routing-map row |
| `npm run ci` in `web/` (format, lint, fourteen suites, released-only build) | green; `dist/wiki/glossary`, `dist/wiki/flight-autopilot`, `dist/create/actions`, `dist/create/events`, `dist/create/base-content`, `dist/create/scenarios` and `dist/news/0.13.0` inspected, and the three new link targets resolve (`#rcs-fine-docking-thrusters`, `../expressions/#queries-and-watched-variables`, `../scenarios/`) |
| `git diff -U0 -- '*.rs'` | every changed Rust line is a comment line |

`CHANGELOG.md` is not rendered by the site - `web/markdown.js:419` links it on
GitHub and the `/changelog/<version>/` pages are redirects to `/news/<version>/`
- so the reorder has no rendered surface to inspect.

Cargo tests were not re-run: no test and no runtime line changed, and the
review's own table already records them green on this tree.
