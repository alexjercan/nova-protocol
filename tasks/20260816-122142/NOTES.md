# Notes

## The principle applied

Nothing under `crates/` may read `webmods/`, name a webmod bundle, scenario or
object, or depend on one to pass. Webmods are outside the project; their authors
and `content lint --target <mod>` cover them. `scripts/gen-portal.py` is now the
only thing in the repository that reads `webmods/`.

## 1. The six rigs: audit and where each claim moved

Every rig was read for its ENGINE claim before deletion. The claims were rebuilt
against scenarios authored as Rust string constants inside the test, with
generic ids (`scenario_1`, `spaceship_1`, `beacon_1`, `gate_1`, `area_1`).

| deleted rig | engine behaviour it pinned | now pinned in | dropped as story-only |
|---|---|---|---|
| `nova_assets/tests/ledger_ch2_encounter.rs` | defeat lifecycle -> counters; per-target one-shot so a neutralized-then-destroyed ship counts once; deferred Victory on `win_gate = scenario_elapsed + delay`; Victory queues `NextScenario` with `linger`; Defeat requeues this scenario; post-win death declares nothing | `scenario_act_machine.rs` | spawn ranges, bearing spread, cover-corridor and rock-overlap geometry, `_light` turret counts, bundle version bump - all assertions about one mod's content quality |
| `nova_assets/tests/ledger_ch4_ending.rs` | `OnEnter` routes by area AND entrant; a branch handler spawns a ship mid-scenario with an `engage_delay` telegraph; a branch handler's `SetSkybox` runs to completion against a real `AssetServer`; distinct terminal messages; a synchronous terminal act latch closes the death window; only one branch chains onward; a settled outcome is never overwritten | `scenario_branch_choice.rs` | which chapter says SOLD vs BURNED, the ch5 chain target |
| `nova_assets/tests/gauntlet_course.rs` | ordered `gate == N` sequencing with out-of-order entries inert; the `other_id` filter half; an area re-counts on re-entry; two mutually exclusive `Outcome` branches keyed on a counter; a terminal gate value disarms the loss handler; `HudReadout` slot/variable/format/visibility | `scenario_gate_course.rs` | gate-area non-overlap and racing-line clearance geometry, the racer's seven-part loadout, the gravity well's mass, bundle version + mod CHANGELOG pins |
| `nova_assets/tests/ledger_ch3_channel.rs` | authored `Allegiance` reaches the LIVE component through the real spawn action + command flush; `SetAllegiance` overwrites it for several ships from one handler; `OnEnter` on a `CreateScenarioArea` zone and `OnCombatLockStart` reach the same wake; a shared one-shot composes the triggers; the reserved `player_speed` readout is filterable and combines with `scenario_elapsed` into a warn -> rearm -> countdown -> trip machine with a cancellable window; an objective posts lazily out of a clock-paced cascade | `scenario_provocation.rs` | pinch-gap and watch-zone geometry, the chapter's beat sheet, the ch4 chain target |
| `nova_authoring/tests/ledger_ch5_raid.rs` | multi-condition win gate (station AND every hostile); terminal win with no chain; a late death cannot overwrite the settled Victory | `scenario_act_machine.rs` | the torpedo gunship's loadout, escort/raider allegiances, the thrusterless base, the hidden flag, ch4 -> ch5 reachability, the frozen `version: "1.23.0"` literal |
| `nova_assets/tests/ledger_skybox.rs` | none of its own | already covered: `nova_scenario/tests/skybox_swap_e2e.rs` drives `SetSkybox` end to end on a real cubemap, `actions/view.rs` unit-tests the deferred install, and `scenario_branch_choice.rs` runs a `SetSkybox` inside a real handler | the whole file: which chapter starts on which cubemap and which story beat swaps it |

Two rig claims turned out to be duplicates rather than losses:

- `ledger_ch5_raid.rs`'s ship-shape pins used
  `nova_authoring::generation::{spawned_ship_sections, build_section_catalog}`,
  which `nova_authoring/tests/broadside_assault.rs` (BASE content) and the
  shakedown tests already exercise on the same code path.
- Its torpedo-binding pin round-tripped `BindingInput::try_from(&Binding)`,
  already unit-tested in `nova_scenario/src/objects/binding_input.rs`.

So `nova_authoring` needed no replacement file at all.

New coverage totals 29 tests across four files, all passing:
`scenario_act_machine` 7, `scenario_branch_choice` 9, `scenario_gate_course` 6,
`scenario_provocation` 7.

## 2. `webmods_validation.rs` deleted

Both of its tests went:

- `every_webmods_bundle_loads_recursively` - the engine claim ("the real loaders
  take a real bundle to recursive `Loaded`") is rebuilt as
  `every_installed_bundle_loads_recursively` in
  `nova_assets/tests/example_scenario.rs`, over every bundle the shipped
  `assets/mods.catalog.ron` names (base + `assets/mods/example`). Repo-owned
  content, no webmod.
- `the_ledger_campaign_lists_its_chapters_in_order` - pure webmod content.
  Campaign membership as an ENGINE rule is `nova_scenario`'s `lint_campaign` and
  `nova_authoring/tests/campaign_membership.rs` over generated base content.

Worth recording: this test would not have caught the breakage that prompted the
task. `47cec257` made `hull` required on `SpaceshipConfig` and migrated the
in-repo `webmods/` with it, so the test stayed green while the owner's INSTALLED
copies under `~/.local/share/nova-protocol/mods/` were stale and failed to
parse. It validated the wrong copies.

## 3. Balance acks moved into the bundle

`crates/nova_authoring/balance_acks.ron` is deleted. Its single entry now lives
at `webmods/the-ledger/balance_acks.ron`, stale claims and all - fixing it is
the ledger author's job, which is the point.

### Sibling file, not a manifest field

Decided: a `balance_acks.ron` beside the `*.bundle.ron`, read by the lint walk
only.

- The manifest is a RUNTIME asset. `BundleManifest` lives in `nova_mod_format`
  and is loaded by the real `BundleLoader` for every installed mod, in the wasm
  build too. A manifest field would put a `BalanceAck` type - a paragraph of
  authoring prose per entry - into the shipped game's memory for every mod,
  forever, to serve a check the game never runs.
- It would also drag lint vocabulary (`close-spawn`, `spawned-dead`, task ids)
  into `nova_mod_format`, whose whole job is the wire format the loader and the
  portal agree on.
- A sibling file is still a DECLARED LIST, not a parsed comment: it is RON,
  deserialized into `BalanceAck`, matched field-by-field against a specific
  finding. The brief's objection to a comment does not apply.
- The portal generator copies every file in a mod directory verbatim, so the
  file publishes with the mod and an external `content lint --target <dir>` run
  on a downloaded copy resolves the same acks.
- Base is symmetric: `assets/base/balance_acks.ron` would be read the same way.
  Base declares none today, and an absent file means no acks.

### Shape change

`BalanceAck` lost its `bundle` field: the walk supplies the id of the directory
the file came from, so an ack can only ever name its own bundle's content. The
`(bundle, ack)` pairing is now explicit in the API, mirroring how findings are
already carried as `(bundle, finding)`.

- removed `balance::shipped_acks()`
- added `balance::BALANCE_ACKS_FILE`
- `partition_findings` takes `&[(String, BalanceAck)]` and returns stale acks as
  `(&str, &BalanceAck)`
- `lint_walk::read_bundle` reads the file; `WalkedBundle` and `AuditBundle` both
  carry `acks`
- added `lint_walk::tree_acks()` for the CI balance gate

An unparsable ack file panics like any other malformed bundle file: a silently
dropped ack turns an intended exception back into an open warning.

## 4. Other leaks found and closed

The DoD's grep turned up more than the six rigs.

- `nova_authoring/tests/content_lint_gate.rs` targeted `the-ledger` by id and
  asserted on the Auditor ack. Retargeted at `example`, and the ack claim is
  rebuilt properly: a new test writes three fixture mods (no ack, a matching
  ack, a stale ack) and pins that the linter resolves acks from the bundle it is
  linting, reports an acked finding with the author's reason and task, and
  grades a stale ack as an Error.
- `nova_authoring/tests/content_report_gate.rs` walked
  `["the-ledger", "gauntlet", "example"]`; now `["base", "example"]`.
- `nova_assets/tests/gen_portal_gate.rs` ran the generator over the real
  `webmods/` tree in two tests. Both now use a synthetic two-mod source
  (`multi_mod_source`), keeping the multi-mod publish and byte-for-byte
  determinism claims.
- `nova_authoring/tests/balance_audit_gate.rs` used `shipped_acks()`; now uses
  `tree_acks()`, and gained a test that every declared ack names a real finding
  kind (the check the deleted `shipped_acks_parse_with_valid_kinds` made).
- Fixture strings naming webmod content: `content_report.rs` (`the-ledger`,
  `ledger_ch1/2.content.ron`, `chapter_one/two`), `actions/ship.rs` (`magpie`),
  and the `Vesh` / `Okono` / `icons/okono.png` speaker fixtures across
  `nova_hud`, `nova_os_ui`, `nova_scenario` and `nova_assets`. All renamed to
  generic ids and speakers.
- Doc-comment mentions: `balance.rs`, `cli.rs`, `lint/ship.rs`,
  `actions/mission.rs`, `hud/readout.rs`, `base_content/ships/racer.rs`,
  `base_content/scenarios/nova_protocol/{cast,final_tally}.rs`,
  `portal/install.rs`, `nova_mod_format/src/lib.rs`, `portal_install.rs`.

Deliberately KEPT: `lint_walk.rs` walks `webmods/` and `assets/mods/` as SEARCH
PATHS, and `cli.rs` documents them as `--target` resolution roots. That is the
linter doing its job - the owner's own position is that linters cover webmods -
and it is what keeps `content lint` reporting the ch4 finding. A missing
directory yields an empty walk, so the day webmods move out nothing breaks.

## 5. Docs shipped with the code

- `web/src/wiki/modding/mod-files.md` - new "Balance acknowledgments" section:
  the mod-facing surface did not exist before, so mod authors now have somewhere
  to read what the file is and what each field means.
- `web/src/wiki/dev/development.md` - the ack file is per-bundle, not
  `crates/nova_authoring/balance_acks.ron`.
- `web/src/wiki/dev/scenario-system.md` - the ordered-gate pattern's rig is now
  `scenario_gate_course.rs`; geometry invariants are named as a content concern
  checked per bundle by `content lint`.
- `.github/workflows/deploy-page.yaml` - the note claiming deep validation is
  "the webmods_validation test on regular CI" was false as of this change.

## 6. The ledger bundle

Bumped `1.23.0 -> 1.24.0` with a CHANGELOG entry. The published file set changed
(the ack file rides along), and the portal serves files under
`<id>/<version>/`, so an unbumped republish would never reach an installed copy.

## Verification

- `cargo check --workspace --all-targets`
- `cargo fmt --check`
- `cargo run content -- gen`, then `git status assets/` clean on a second run
- `cargo run content -- lint` - the ch4 close-spawn finding still REPORTS,
  acked from inside the ledger
- the four new rigs (29 tests) plus the reworked
  `content_lint_gate`, `content_report_gate`, `balance_audit_gate`,
  `gen_portal_gate` and `example_scenario`
- `cargo test --lib` for `nova_authoring`, `nova_scenario`, `nova_hud`,
  `nova_os_ui`, `nova_assets` (the crates whose in-source fixtures changed)

Per the standing instruction the full workspace suite and clippy are CI's job.
