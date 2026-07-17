# Review: Editor skybox may miss its Cube view (FALSIFIED)

- TASK: 20260717-133332
- BRANCH: work/editor-skybox-cube-view

## Round 1

- VERDICT: APPROVE

A falsification cycle: the suspected bug does not exist, and the diff correctly
delivers an evidence rig + non-behavior pin instead of a code fix. Diff is a new
test, a comment at the editor insert site, and the TASK.md resolution - no
behavior change.

Independently re-verified the falsification's load-bearing claim (shared
implementer/reviewer session), re-deriving the state ordering from source rather
than trusting the TASK.md note:

- `prepare_cubemap_view` is chained at `nova_assets/src/lib.rs:949` in
  `OnEnter(GameAssetsStates::Processing)`, and the chain ends by setting
  `GameAssetsStates::Loaded` (`:957`).
- `OnEnter(GameAssetsStates::Loaded)` transitions to `GameStates::Playing`
  (`nova_core/src/lib.rs:151-160`).
- `OnEnter(GameStates::Playing)` + `GameMode::Sandbox` transitions to
  `ExampleStates::Editor` (`nova_editor/src/lib.rs:76-80`), which runs
  `setup_editor_scene`.

So `prepare_cubemap_view` sets the Cube view on `game_assets.cubemap` strictly
before the editor inserts `SkyboxConfig` on that SAME handle. The bcs observer
sees a ready 6-layer + Cube image and just attaches `Skybox`. Falsification
confirmed.

Stronger than the task states: even if the meta had NOT applied (image still
single-layer when `prepare_cubemap_view` runs, so it warns and leaves it), the
bcs observer's fallback branch reinterprets AND sets the view on insert - and the
editor is a stable scene, not a scenario tearing down mid-PNG-decode, so it is
not exposed to the teardown upload race at all. The editor is safe in every
branch.

Test is meaningful (would fail with the fix's premise removed):

- `prepare_cubemap_view_sets_cube_view_on_the_game_assets_cubemap`
  (`nova_assets/src/lib.rs`): the arrayed case asserts the view becomes `Cube` -
  it returns `None` and fails if `prepare_cubemap_view` is deleted or stops
  writing. The single-layer "stays None" case is not vacuous: the arrayed case
  is its paired delivery guard, proving the system ran and can write, so the
  None result means "deliberately left for the fallback", not "system never ran".

Checks: `nova_assets` lib test 1/1 pass; `cargo check -p nova_editor` clean;
`cargo fmt` clean (reverted unrelated pre-existing example fmt drift, as in the
sibling task). Full suite deferred to CI per the local-test policy; the diff is
additive (test + comment + docs), no existing behavior touched.

No findings. The comment at `nova_editor/src/ui/mod.rs` pointing future readers
at `prepare_cubemap_view` is exactly what stops this from being re-filed.
