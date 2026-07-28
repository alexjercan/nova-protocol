# DECISION - Preload static assets + phosphor boot loading screen

Task 20260729-000956 (epic 20260728-175719). Owner approved building directly
via /flow ("it's already planned so do work directly"), which accepts the three
recommended forks below. Two further load-bearing choices surfaced mid-build and
are recorded here too.

## D1 - Boot-state font gating for the loading screen (ACCEPTED, recommended)

Options: (a) add a `GameAssetsStates::Boot` collection that preloads the UI font
so the screen renders themed text from frame one; (b) bevy's `default_font`
feature; (c) a font-free screen.

Chosen: (a). A second `bevy_asset_loader` loading state on the SAME
`GameAssetsStates` enum (`Boot -> Loading`) loads a tiny `BootAssets` collection
(the font). This stays inside the bevy_asset_loader model and gives the phosphor
screen its Iosevka face immediately. The two-loading-states-on-one-enum chaining
is the only novel mechanism; it is pinned by
`boot_then_loading_collections_gate_in_sequence`.

## D2 - Font slimming: single-face .ttf extraction (ACCEPTED, recommended)

Options: (a) extract the single Regular face to a `.ttf`; (b) keep the 66 MB
`.ttc`; (c) glyph-level subsetting.

Chosen: (a). Extracted face 0 ("Iosevka Term", Regular) from the 6-face `.ttc`
with fonttools -> `assets/fonts/SGr-IosevkaTerm-Regular.ttf`, 10.8 MB (down from
66 MB), under the 15 MB DoD ceiling. Face extraction keeps ALL glyphs of the
face (box-drawing, full block, arrows verified present), so nothing the NOVA OS
output draws is lost. Subsetting is deferred (risked dropping those glyphs; the
web-payload revisit lives in 175734 if still needed). Credits + OFL license file
updated to the `.ttf` and note the single-face extraction.

## D3 - Indeterminate CRT loading animation (ACCEPTED, recommended)

Options: (a) an indeterminate CRT animation (blinking block cursor + marching
dots); (b) an `iyes_progress` percent bar.

Chosen: (a). A percent bar needs a new dependency and per-collection progress
wiring; the indeterminate animation is purely time-driven, needs neither, and
matches the phosphor look. Deferred (b).

## D4 - UI SFX load-gating WITHOUT a bevy_common_systems release (mid-build)

The plan's Step described a `SoundBank::from_handles` constructor to build the
UI bank from a mapped collection's handles. On inspection `SoundBank::from_handles`
already exists in `bevy_common_systems` but is PRIVATE, and bcs is pinned by git
TAG (v0.19.5). Making it public would require cutting a new bcs tag and bumping
nova's Cargo.toml/lock - a release of a SEPARATE repo (outward-facing), which the
owner has not asked for.

Chosen route-around (no bcs change): add a `collection(mapped, typed)` `ui_sfx`
field to `GameAssets` (keyed by `AssetFileStem`) with an explicit `paths(...)`
list of the 14 distinct UI wavs. This makes the SFX part of the main collection
gate - they load before `Processing`, which is the load-gating the step wanted.
`register_sounds` keeps the existing public `SoundBank::load(&assets,
UI_SFX_FILES)`; because the AssetServer dedups by path, the bank's handles point
at the same already-loaded (gated) assets. Parity is pinned by
`ui_sfx_collection_matches_ui_sfx_files` (collection paths == distinct
`UI_SFX_FILES` files, each existing on disk). If the owner later wants the bank
built directly from the collection handles, that is a follow-up gated on a bcs
release exposing a public from-handles constructor.

## D5 - DoD 1 test altitude: chaining mechanism, not full GameAssets (mid-build)

DoD 1 asked for a headless walk `Boot -> Loading -> Processing -> Loaded` with
"both collections resolved". The full `GameAssets` collection (glTF `WorldAsset`s,
audio, the recursive mod catalog) has NO headless loader set anywhere in the test
suite - that full walk is only ever exercised by the render-based
screenshot/probe harness (which reaches `Loaded`) and, for this task, the
native-run verification (DoD 7).

Chosen: `boot_then_loading_collections_gate_in_sequence` pins the ONLY novel
mechanism - two bevy_asset_loader loading states chaining on one enum - on the
REAL `GameAssetsStates` enum with real async image loads (one small PNG per
collection). It uses lightweight probe collections rather than the shipped
`BootAssets`/`GameAssets` because the chaining is independent of which assets the
collections hold. The shipped collections' correctness is covered by the native
run + the existing screenshot harness reaching `Loaded`.
