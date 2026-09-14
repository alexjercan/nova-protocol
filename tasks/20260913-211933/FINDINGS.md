# Duplicated algorithms and logic

Identification only. No code was changed.

Severity is DRIFT RISK, not brokenness:

- `BLOCKER` - the copies already disagree, and the difference is reachable.
- `MAJOR` - a plausible near-term change must touch every copy.
- `MINOR` - real duplication, but the behavior is unlikely to move.

`Agreed by` counts how many reviewers independently found the family. Every
`path:line` was re-read at the cited lines during adjudication; claims that did
not survive that read are recorded under "Rejected and corrected".

Two families the reviewers rated highly were **corrected downward after the
owner challenged them** - findings 4 and 5. One of those corrections withdraws
a defect claim the orchestrator had previously called verified.

## Method

Eight reviewer-passes, four chunks, two reviewers per chunk. Each pair read the
same code by two deliberately different routes - bottom-up from code shape, and
top-down from a list of behaviors named before searching - so that agreement
between them is corroboration rather than a shared blind spot.

| Chunk | Crates |
| --- | --- |
| UI and presentation | `nova_ui` `nova_os_ui` `nova_hud` `nova_menu` `nova_os` `nova_console` `nova_input` |
| Simulation | `nova_ship` `nova_gameplay` `nova_autopilot` `nova_events` `nova_core` `nova_channel` |
| World and content | `nova_scenario` `nova_assets` `nova_authoring` `nova_editor` `nova_wfc` `nova_mod*` |
| Cross-crate seams | all crates, shared-utility families only, plus the tooling crates |

Scope per the owner: `crates/*/src/**` only. Examples, benches, fixtures and
test modules are excluded as finding subjects.

---

## BLOCKER

### 1. `rebind-policy-four-surfaces`

Agreed by 3 of 8, across two independent chunks. The largest single family in
the audit.

Four surfaces let a player rebind an input. Each implements the arm-capture-commit
ladder itself, and they now enforce four different policies.

| Surface | Conflict rule | Left Mouse | Gamepad column |
| --- | --- | --- | --- |
| `crates/nova_menu/src/settings.rs:939-1025` | `conflict_for` + `section_conflict` | refused | preserved |
| `crates/nova_os_ui/src/ship/rebind.rs:21-112` | `reserved_conflict`, different wording | refused | **destroyed** |
| `crates/nova_editor/src/keybind.rs:345-416` | none, deliberately | allowed | preserved |
| `crates/nova_console/src/settings.rs:120-163` | **none** | allowed | preserved |

The shared sink enforces nothing either. `InputBindings::rebind`
(`crates/nova_input/src/registry.rs:339-362`) delegates only to `refuse_spec`
(`:373-395`), which checks device-column placement and "ships with no keyboard
button". Those 23 lines contain no cross-action collision rule.

Two live consequences, both traced to their end and verified by reading:

**A console bind is accepted, saved, then silently reverted.**
`crates/nova_console/src/settings.rs:6` states the console's own contract:
"`nova_menu` persists on change, so a value written here reaches the store". At
next launch `crates/nova_menu/src/settings_store.rs:428` calls `apply_overrides`
(`registry.rs:423`), which calls `drop_stored_conflicts` (`:443-468`), which
finds the collision and calls `self.reset(stored)` behind a `warn!`. The player
sets a binding, it works, and it is gone after a restart with no message.
`nova_console` is wired through `nova_core` with no `cfg(feature = "dev")`
gate, so this is reachable from the in-game terminal.

**The in-ship panel drops a section's gamepad binding.**
`crates/nova_os_ui/src/ship/rebind.rs:82` is `let bindings = vec![source];` -
it replaces the whole binding vector rather than the keyboard column. Rebinding
a section from the NOVA OS therefore discards its gamepad trigger. The editor
asserts the opposite behavior for its own path, at
`crates/nova_editor/src/keybind.rs:922-924`: "the gamepad bind is preserved".
Two surfaces, one repository, opposite contracts, each pinned by its own test.

`crates/nova_editor/src/keybind.rs:323-331` openly repudiates the veto policy
that `nova_os_ui/src/ship/rebind.rs` still enforces. The disagreement is
documented; it is simply not resolved anywhere.

**Proposed home.** `nova_input::registry::commit_rebind(action, spec, policy)`
owning the collision rule and the column semantics; all four surfaces already
call into `nova_input` for `captured_desk` / `all_released`, so only the ladder
is copied. The differing policies become an argument, not four implementations.

**Not higher than BLOCKER by category:** no save corruption, no build failure.

### 2. `scroll-driver-fork`

Agreed by 3 of 8.

`nova_ui::screen::scroll` exists for exactly this, and its module doc names the
failure mode in advance - `crates/nova_ui/src/screen/mod.rs:4-9`: "two crates
each carrying their own copy is how they drifted apart".

`nova_os_ui` imports that module and then re-implements it:
`crates/nova_os_ui/src/terminal/input.rs:509-539` and
`crates/nova_os_ui/src/terminal/shell.rs:511-523` duplicate
`crates/nova_ui/src/screen/scroll.rs:67-98` and `:103-112` - the same
hover-gated wheel sum and the same clamp, line for line.

**Already drifted.** The copy uses `DRAWER_SCROLL_LINE_HEIGHT_PX = 20.0`
(`crates/nova_os_ui/src/terminal/style.rs:32`, undocumented) against the shared
`SCROLL_LINE_HEIGHT = 60.0` (documented). One wheel notch moves a NOVA OS
drawer a third as far as every other scrollable surface in the game.

### 3. `terminal-command-error-vocabulary`

Agreed by 1 of 8 (the UI behavioral pass; the structural pass reached this
cluster by the scroll route instead).

The `ResolvedCommand` error arms are written three times:
`crates/nova_os/src/terminal/edit.rs:309-365` (NOVA OS shell),
`crates/nova_os/src/commands.rs:648-713` (Commands shell),
`crates/nova_os/src/terminal/edit.rs:583-591` (prompt hint).

**Already drifted.** `Incomplete` prints a "Subcommands:" list in one shell and
a full help block in the other. `UnexpectedArguments` selects
`command_help_rows` in one and `usage_rows` in the other. The same typo in the
same CRT gets two different answers depending on which shell is open.

**Proposed home.** One `fn render_command_error(&ResolvedCommand) -> Vec<Row>`
in `nova_os`; the shells differ only in where they print it.

---

## MAJOR

### 4. `weapon-fire-sequence-three-paths` - CORRECTED, now MINOR

Agreed by 2 of 2 in its chunk. **Both reviewers overstated this, and the
orchestrator's first verification confirmed it on incomplete evidence. The
owner challenged it and was right. Corrected in full here.**

What the three weapon classes actually share is two statements, repeated at
`crates/nova_ship/src/sections/turret_section/firing.rs:245-247`,
`railgun_section/firing.rs:180-186` and `torpedo_section/bay.rs:317-319`:

```rust
let center_of_mass = position.0 + rotation.mul_vec3(**center);
let inertia_vel = rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, muzzle_position);
```

Both helpers are already shared (`sections/mod.rs:84`,
`physics/rigid_body.rs:27`). Two statements and a copied comment is the "line
or two" case that is explicitly out of scope for this audit. Everything a
weapon class actually does differently - the torpedo's recess and quarter turn,
the railgun's bore and recoil-at-the-muzzle, the turret's per-muzzle bearing
gate - is correctly separate, and should stay separate.

**The "already drifted" claim is withdrawn.** Both reviewers reported that the
railgun is missing the `TranslationEasingState` / `RotationEasingState` seed
that turret and torpedo both set, and called that drift. It is not. The railgun
slug is not an interpolated entity at all: it carries no `TransformInterpolation`,
and its visual is built by a dedicated observer,
`crates/nova_ship/src/sections/railgun_section/render.rs:134`, which draws a
`RoundTracer` streak. That file states the reason at `:130-133` - "A slug
crosses roughly 25 units between two drawn frames at 60 fps against an 0.8 unit
body, so without it the one shot a lance gets every thirteen seconds is drawn
as a handful of disconnected darts". Sub-tick easing is meaningless for an
entity rendered as a streak. The omission is correct and documented; it is
simply documented in `render.rs` rather than `firing.rs`.

**What survives, and it is not duplication.**
`crates/nova_ship/src/sections/torpedo_section/bay.rs:545-552` calls
`try_consume` and discards the failure *after* the round has already spawned,
reachable only because a separate gate catches it first. That is a latent
ordering bug in one file, worth fixing on its own terms.

**Recommendation: do not consolidate the weapon fire paths.**

### 5. `menu-panel-close` - CORRECTED, now MINOR plus a separate UX note

Agreed by 2 of 2 in its chunk. **Filed as MAJOR; the owner challenged it and
was right. Two separate things were conflated.**

*The duplication half is small.* Four panels carry a toggle-and-close pair:
`crates/nova_menu/src/settings.rs:75-90`, `mods.rs:204-226`,
`scenarios.rs:565-580`, `pause.rs:561-577`. Each pair is about thirteen lines
of `Visibility` assignment - a three-arm match and a one-line setter. That is
below this audit's threshold. The only non-trivial detail is that
`on_mods_back` (`mods.rs:213-226`) alone also fires `ReloadContent`, and its
comment explains why; nothing shared records that one of the four is special.

*The UX half is not duplication at all and does not belong in this task.*
Those panels have no Escape close. Every production `KeyCode::Escape` reader in
the seven UI crates was enumerated - `pause.rs:91` closes the pause overlay,
`nova_os_ui/src/terminal/input.rs:121` backs out of the NOVA OS, and
`ship/rebind.rs:40` and `settings.rs:965` each cancel a key capture. None
closes `SettingsPanel`, `ModsPanel` or `ScenariosPanel`.

No player can be trapped - every panel has a Back button - so this is a
consistency gap against the pause overlay and the NOVA OS, which both close on
Escape. It is worth a separate small task if the owner wants it; it is not a
duplication finding and it is not a defect.

### 6. `section-aabb-overlap-rule`

Agreed by 1 of 8. Verified independently during adjudication.

The section interpenetration test is written twice, in two crates, tuning
constant included:

- `crates/nova_editor/src/snap.rs:298-306`, constant at `snap.rs:15`
- `crates/nova_scenario/src/lint/ship.rs:760-782`, constant at `lint/ship.rs:710`

Both are `const OVERLAP_EPSILON: f32 = 1e-3;` followed by the same three-axis
comparison of summed `rotated_aabb_half_extents`.

**Why this one has a clear victim.** `snap.rs` decides whether the editor
refuses a builder's placement (`Refusal::Overlap`). `lint/ship.rs` decides
whether that same hull passes the content lint. If the two disagree, the editor
builds hulls the lint rejects. They already differ in the mate exemption:
`snap.rs:229` uses `candidate_link_point_mates` (all candidates) while
`lint/ship.rs:750` uses `derive_link_point_graph(..).unwrap_or_default()`,
which per `crates/nova_ship/src/sections/link_points.rs:149-167` yields an
**empty** exemption set for an ambiguous or disconnected hull.

**Proposed home.** `nova_ship` - the lowest shared crate - beside
`SectionCollider::rotated_aabb_half_extents`
(`crates/nova_ship/src/sections/base_section.rs:112`), epsilon included.

### 7. `enabled-bundle-set-three-ways`

Agreed by 1 of 8. Verified independently during adjudication.

"Walk the enabled bundles in catalog order, then downloaded, skipping a
downloaded id that shadows a shipped one" is written at
`crates/nova_assets/src/merge.rs:75-125`,
`crates/nova_assets/src/mod_set.rs:133-181`, and
`crates/nova_editor/src/asset_index.rs:179-212`, with a fourth statement of the
no-shadowing rule at `crates/nova_assets/src/portal/install.rs:457-465`.

**Already inconsistent.** `merge.rs:98-107` skips a shadowing downloaded id
with a `warn!`, and its comment names this as the portal generator's rule
"enforced again at the merge because the index is downloaded input". The editor
copy at `asset_index.rs:199-212` has no shadow skip at all and open-codes the
handle resolution instead of calling `catalog_bundle`. The editor therefore
offers content ids that the merge ignores: a creator can select content that
will not load.

**Not BLOCKER:** reaching it needs a downloaded bundle whose id shadows a
shipped one, and the portal generator refuses to produce that, so it takes a
sideloaded mod.

### 8. `scenario-id-to-entity`

Agreed by 1 of 8.

"Which live entities may an authored scenario id address" is answered four
different ways across nine sites, two of which are helpers the other sites
ignore: `crates/nova_scenario/src/actions/view.rs:166-172`,
`actions/spawn.rs:42-48`, `actions/mission.rs:241-247` and `:293-299`,
`actions/ship.rs:37-44`, `:89-96`, `:134-141` and `:1918-1927`, and
`loader/clock.rs:34-39`.

`actions/spawn.rs:33-39` records why the marker gate is load-bearing:
"spaceship SECTIONS also carry EntityId ... an unscoped match on such an id
would rip that section out of every ship in the scene."

**Already drifted.** `loader/clock.rs` resolves authored ids with no
`ScenarioScopedMarker` gate, leaning on `&LinearVelocity` to exclude sections,
while its own doc at `clock.rs:22-24` concedes "severing turns each hull
section into another free body". Latent only because, per `clock.rs:24`, no
shipped scenario reads an entity query.

### 9. `render-target-image-recipe`

Agreed by 1 of 8. Verified independently during adjudication. This one is a
trap for a future contributor.

Four byte-identical `Image::new_target_texture(w, h, Rgba8UnormSrgb, None)`
constructors: `crates/nova_hud/src/target_inset.rs:291-299`,
`crates/nova_os_ui/src/terminal/crt.rs:175`,
`crates/nova_os_ui/src/ship/scene.rs:990`,
`crates/nova_os_ui/src/map/scene.rs:706`.

The `None` view format is **not** a default; it is a WebGL2 requirement.
`target_inset.rs:288-291` explains that a `Some` view needs
`DownlevelFlags::VIEW_FORMATS` and produces a render validation error the
moment the player HUD spawns. The three `nova_os_ui` copies carry no rationale
at all, so they read as an oversight that someone would "correct" by following
Bevy's own example - and break the web build in three places.

**Proposed home.** `nova_hud`, a direct dependency of all three consumers.

### 10. `novaos-viewer-twin`

Agreed by 2 of 2 in its chunk, and flagged independently by the orchestrator's
duplicate-block scan in three separate regions before either report arrived.

The NOVA OS map viewer and ship viewer are the same viewer written twice:
`crates/nova_os_ui/src/map/scene.rs:171-205` against
`crates/nova_os_ui/src/ship/scene.rs:323-355`, `:313-337` against `:402-423`,
`:707-722` against `:991-1004`.

Duplicated: a byte-identical `orbit_eye`, the orbit input block with all five
magic constants (1.6 / 1.45 / 0.12 / 0.0024), cycle-with-wrap, and the image
factories of finding 9. The render-target resize dance - measure, compare,
resize in place, then `projection.set_changed()` to work around a Bevy bug -
has a third copy at `crates/nova_os_ui/src/terminal/crt.rs:188-248`. Two of the
three name the engine bug; the ship copy does not, so whoever removes the
workaround when Bevy fixes it will not find that one.

### 11. `verb-grant-predicate`

Agreed by 1 of 8. Filed as BLOCKER by the reviewer; **downgraded here after
reachability tracing.**

Seven copies of "does a live controller section on this ship grant this verb",
written four ways. `crates/nova_ship/src/input/player/flight_rig.rs:358-377`,
`input/player/hints.rs:96-103`, `flight/manual.rs:300-307` and
`flight/autopilot.rs:123-129` all require `With<PDController>`.
`input/targeting/contacts.rs:210-223` omits it - while its own doc comment
claims it "mirrors player.rs's `ship_grants_verb`". It does not.

`flight/autopilot.rs:120-122` gives the missing filter its meaning: "preview
controllers have none".

**Why not BLOCKER.** Latent, not live: `grep` for `.remove::<PDController>`
returns no production hit; damaged controllers are taken offline with
`SectionInactiveMarker`, which *both* predicates already filter on
(`sections/controller_section.rs:967,1146,1281,1318`); and the only
`ControllerSectionMarker` without a `PDController` is
`preview_controller_section` (`:530`), documented at `:520` as the editor's
non-physics view ship. It stays MAJOR because the false "mirrors" comment is a
live trap and the next verb rule must be found in seven places.

`input/point_defense/ownership.rs:128-133` shows the house standard the others
miss: it documents its own deliberate divergence. `contacts.rs` documents none.

### 12. `hud-anchored-chip-module-cloned`

Agreed by 2 of 2 in its chunk, and by the orchestrator's block scan.

`crates/nova_hud/src/beacon_chips.rs` and
`crates/nova_hud/src/objective_markers.rs` are the same feature module for
module - chip layer bundle, label leaf, spawn observer, despawn observer, and a
label-plus-distance updater identical apart from component names. The comments
were copied verbatim too (`beacon_chips.rs:151-171` against
`objective_markers.rs:215-235`). `nova_hud` already has a generic helper for
this lifecycle whose doc comment names this exact gap.

### 13. `section-source-resolution`

Agreed by 2 of 2 in its chunk.

`SectionSource` (`crates/nova_scenario/src/objects/spaceship.rs:296-303`) has
no inherent resolver, so six production sites hand-roll the same two-arm
lookup: `objects/spaceship.rs:512-525`, `loader/lifecycle.rs:317-320`,
`loader/preload.rs:108-114`, `crates/nova_editor/src/node.rs:249-257`,
`crates/nova_editor/src/preview.rs:345-353`,
`crates/nova_wfc/src/check.rs:54-62`. Two are byte-identical inside
`nova_editor`.

The sibling type already has the idiom -
`crates/nova_scenario/src/objects/ship.rs:152-157`. A mod-scoped id or a
prototype fallback must land on all six; a miss means the editor preview draws
a different hull than the game spawns.

### 14. `authored-object-walk`

Agreed by 1 of 8.

`EventActionConfig::walk` (`crates/nova_scenario/src/actions/mod.rs:428-438`)
handles nested actions and is used by the loader count, lint pass 1 and the
wake profile. Four sites hand-roll a flat `events[].actions[]` loop and miss
nested spawns: `loader/preload.rs:89-118`, `loader/lifecycle.rs:286-301`,
`crates/nova_authoring/src/balance.rs:544-561`,
`crates/nova_authoring/src/lint_walk.rs:495-516`.

Generated content already nests spawns inside `Sequence` steps
(`assets/base/scenarios/tutorial.content.ron:1309,1467,1764`). They are Beacons
today, so the preload gap is latent - armed for the first nested ship spawn.

### 15. `window-capture-flow`

Agreed by 2 of 2 in its chunk.

Three crates capture the primary window.
`crates/nova_autopilot/src/capture.rs:91-142` and
`crates/nova_scenario/src/actions/view.rs:315-330,356-377` each re-implement
the `NOVA_CAPTURE_DIR` resolution rule and the mkdir step;
`crates/nova_debug/src/screenshot.rs:68-86` repeats the mkdir-spawn-`save_to_disk`
tail. `nova_debug` already depends on `nova_autopilot`, so part of the fix is a
deletion.

Note: one reviewer reported these as "already forked" because the pure cores
differ - `capture.rs:135` tests `!dir.is_empty()`, `view.rs:324` does not. That
is corrected below; the composed behavior is identical.

### 16. `mod-ref-scope-build`

Agreed by 1 of 8.

`crates/nova_assets/src/merge.rs:200-234` and
`crates/nova_authoring/src/lint_walk.rs:317-351` build the same dependency
scope, differing only in `base: Some(..)` vs `base: None`. AGENTS.md requires
errors "at lint, then load"; that holds only while both stages build the same
scope. The implicit-`base` rule lives in four places - `RefScope::rewrite_leaf`
(`crates/nova_assets/src/mod_refs.rs:98-102`), `RefScope::violation`
(`:123-132`), and these two builders that must insert `base` for the first two
to find it.

### 17. `player-weapon-input-rig`

Agreed by 2 of 2 in its chunk.

`crates/nova_ship/src/input/player/weapons.rs` is a four-way copy - bind, press
and release for thruster, turret, torpedo and railgun. The weapons-safety gate
is verbatim three times (`:192-198`, `:289-295`, `:388-394`). The same weapon
family is then hand-enumerated in six more places across five crates, including
`crates/nova_channel/src/apply.rs:320-332` and four identical
`match controller_config` blocks in
`crates/nova_scenario/src/objects/spaceship.rs`. A fifth weapon class means
editing ten hand-written lists.

### 18. `deterministic-hash-hand-rolled`

Agreed by 4 of 8 - the most widely corroborated family in the audit.

FNV-1a is hand-written at roughly ten sites across `nova_gameplay`,
`nova_ship` and `nova_scenario`. Two copies of "unit-sphere point from a hash"
use different bit slices while claiming the same rule. A change to the mixing
constants desynchronises anything that assumed two of these agreed.

### 19. `projected-radius-px`

Agreed by 2 of 8, across two chunks.

"How big is this sphere on screen" is written three times in two crates:
`crates/nova_hud/src/screen_indicator.rs:427-439`,
`crates/nova_os_ui/src/map/scene.rs:552-562`, and inlined at
`crates/nova_hud/src/flight_status.rs:348-356`. They already disagree on
robustness (project the centre once vs project both) and on logical/physical
pixel handling - the exact failure `crates/nova_ui/src/screen/float.rs:17-24`
documents for its own family.

### 20. `probe-check-capability-gate`

Agreed by 2 of 2 in its chunk.

The "did this run earn a grade" decision table is written once per check file:
`crates/nova_probe_cli/src/evaluation/checks/reached_playing.rs:24-66`,
`run_completed.rs:25-65`, `invariants_held.rs:45-96`,
`fps_within_baseline.rs:31-71`, `capture_simulated.rs:68-96`. Each maps the same
four `Input` variants onto the same `CheckStatus` values with the same literals
`"not armed"` / `"armed and silent"`. `overall_verdict`
(`checks/mod.rs:210`) folds on `graded()`, so one un-migrated check flips a
run's verdict.

### 21. `editor-tooltip-hover-sync`

Agreed by 1 of 8.

Two hover-hint systems spawned side by side at
`crates/nova_editor/src/ui/mod.rs:1117-1118` and built apart:
`ui/rail.rs:551-598` and `:607-652` against `ui/inspector.rs:694-738` and
`:751-803`. The gap constant is typed twice (`rail.rs:543`,
`inspector.rs:660`, both `8.0`). They already differ in one respect - the
inspector flips sides near the screen edge, the rail does not - which is
exactly what makes a further, unintentional divergence hard to notice.

### 22. `gravity-well-qualification`

Agreed by 2 of 2 in its chunk, **with a severity disagreement between them.**

`insert_asteroid_gravity_well`
(`crates/nova_scenario/src/objects/asteroid.rs:385-408`) and
`insert_planet_gravity_well` (`crates/nova_scenario/src/objects/planet.rs:165-186`)
are identical apart from the query tuple: the same authored-mass-wins /
`min_well_radius` / `default_mass` rule, then the same `RigidBody::Static`
override. `objects/anchor.rs:96-112` deliberately differs, which shows the rule
is a live decision.

One reviewer rated this MINOR (five lines; the expensive half, `from_mass`, is
already shared). Adjudicated as MAJOR: the thing that moves is "when does a
body qualify for a well", and a maximum well radius or a per-kind default mass
is exactly the change that lands in one file.

### 23. `sfx-juice-throttle`

Agreed by 2 of 8, across two chunks.

`JuiceThrottle` (`crates/nova_gameplay/src/juice.rs:233-255`) and `SfxThrottle`
(`crates/nova_gameplay/src/audio/mixing.rs:142-171`) have byte-identical
`allow` and `prune` over the same key type. `juice.rs:221-224` states the
invariant the two must jointly keep - "a frame that is one bang has to be one
kick" - and they already share `CueGroup` for it. The timestamp machinery was
never lifted with it.

### 24. `vfx-base-velocity-property`

Agreed by 1 of 8.

The hanabi property `"base_velocity"` is retyped as a bare string at ten sites
across `nova_gameplay` and `nova_ship`; declare and set are paired by string
only, so drift yields no compile error and no log. The sibling property on the
next line already has a constant (`PYRE_SCALE_PROPERTY`,
`crates/nova_gameplay/src/integrity/pyre.rs:112`).

### 25. `rebind-capture-protocol`

Agreed by 1 of 8. The UI half of finding 1, recorded separately because the fix
is separable.

Three capture flows re-implement Escape-cancel, awaiting-release,
`captured_desk` and left-mouse refusal. The "a capture owns Escape this frame"
rule has four wirings, and `nova_ui`'s purpose-built `InputMode::Bind` is used
by the editor only; `crates/nova_menu/src/pause.rs` needed an ad-hoc
`PendingRebind` peek twice (`:68-70`, `:146-148`).

### 26. `torpedo-launch-commit-three-paths`

Agreed by 1 of 8.

The identical query filter and two-component commit at
`crates/nova_ship/src/input/player/intent.rs:214-247`,
`input/ai/torpedo.rs:226-259`, `sections/torpedo_section/scripted.rs:45-77`. AI
and scripted filter the target with `q_ship_root.contains(target)`; the player
path does not.

### 27. `debris-piece-spawn-recipe`

Agreed by 1 of 8.

`crates/nova_gameplay/src/integrity/explode.rs:342-477` and
`crates/nova_ship/src/sections/fixture.rs:261-357` repeat the same wreck
bundle, `away` fallback chain, descendant-collider strip walk and missing-RNG
degrade.

### 28. `sandbox-scenario-build`

Agreed by 1 of 8.

`crates/nova_editor/src/scenario.rs:229-261` and `:280-300` build the sandbox
scenario identically, declaring the same seven system parameters. These are the
two ways it enters `GameScenarios` - at boot and on the editor's Play hand-off.
An eighth input applied only to Play means the defeat overlay's Retry, which
per `scenario.rs:253-255` resolves against this registry, flies a different
world than Play did.

---

## MINOR

Recorded without detail; each is real duplication whose behavior is unlikely to
move.

- `editor-script-node-set` - three system params and two predicates enumerate
  the same six script node kinds (`nova_editor/src/event.rs:1234-1239,1262-1269,1346-1357`
  against `ui/inspector.rs:315-320,2319-2326,332-343`).
- `staged-node-filter` - `Or<(With<ShipNode>, With<ObjectNode>)>` spelled seven
  times across `nova_editor` gizmo/placement/node, despite the crate already
  using the type-alias idiom at `placement.rs:295`.
- `disabled-row-reconciler` - nine copies; eight change-guarded, and
  `nova_editor/src/generate.rs:121-127` is not.
- `surface-noise-primitives` - `sphere_spread` and `octave_safe_seed`
  byte-identical in `planet_surface.rs` and `asteroid_surface.rs`; a third
  fractal-seed site (`objects/asteroid.rs:719,754`) lacks the overflow guard,
  unreachable today.
- `content-file-read` - `nova_modding::parse_content` / `parse_manifest`
  (`nova_modding/src/lib.rs:293-300`) bypassed by three readers that hand-roll
  the decode (`nova_assets/src/loose.rs:40,105`,
  `nova_authoring/src/lint_walk.rs:91-101`), though both crates already depend
  on `nova_modding`.
- `visible-catalog-overlay` - the same "base + declared deps + own" block
  written three times inside one function
  (`nova_authoring/src/lint_walk.rs:202-215,220-230,234-248`).
- `breath-wave-formula` - seven copies around an already-shared period
  constant, already out of phase with the shared `HudEmphasis` pulse.
- `hud-chevron-arrow` - three builders with three drifted constant triples.
- `single-line-editor` - implemented in both `nova_ui` and `nova_os`.
- `stable-code-minting` - the `PREFIX-n` algorithm written twice.
- `report-modal-shell` - written twice.
- `menu-row-select-and-selection-repair` - the `nova_menu` selection family.
- `kind-label-info-table` - `section_kind_label` in `nova_console` and
  `nova_os_ui`.
- `los-range-rate` - two more copies in `nova_bench/src/observation.rs:466-476`
  and `:604-612`; `nova_bench` refuses `nova_*` edges by design, so the fix is
  constrained.
- `nearest-ancestor-walk` - five copies.
- `repo-root` - three spellings.
- `stepdiag-singleton-lock` - `nova_probe`'s `stepdiag.rs` lacks the singleton
  lock its two sibling sinks have.
- `sphere-orbit-rigs` - the five `nova_gameplay/src/transform/*` movers are
  genuinely distinct; only their rig setup repeats. Both reviewers reached this
  independently.

---

## Confirmed NOT duplicated

Recorded so nobody re-opens them.

- **The owner's PDC question.** A double-barrel turret is not a second
  implementation. `crates/nova_ship/src/sections/turret_section/setup.rs:106-174`
  collects every muzzle joint into one `TurretSectionMuzzles`, and the single
  `shoot_spawn_projectile` loops them against one shared magazine, pinned by
  the test at `firing.rs:780`. Weapon *variants* share the path. Weapon
  *classes* do not - that is finding 5, one level above where the question
  guessed.
- **Lead intercept solve** - one copy,
  `crates/nova_ship/src/sections/turret_section/aim.rs:125-157`, five consumers.
- **Damage pipeline** - one `apply_damage`,
  `crates/nova_gameplay/src/damage.rs:315-490`.
- **Flight control law** - one, `crates/nova_ship/src/flight/autopilot.rs:65`
  onto `flight/guidance.rs`. Player manual and AI share it.
- **Meters / engine seam** - one, `crates/nova_events/src/units.rs:32`. A sweep
  of the world chunk for hand-rolled `* 0.1` / `/ 10.0` found one hit, in a
  test; 24 files call `to_engine` / `from_engine`.
- **UI z-order** - one table, `crates/nova_ui/src/layer.rs:26-74`, pinned by
  the ordering test at `:92-115`.
- **Player-facing unit formatting** - one policy,
  `crates/nova_ui/src/units.rs`; no `km` formatting outside it in production.
- **Key labels** - one authority, `crates/nova_input/src/source.rs`.
- **Keyboard arbitration** - one, `crates/nova_ui/src/input_mode.rs`, no
  production bypass.
- **Button activation** - one entry, `bevy::ui_widgets::Activate`.
- **Prompt editing core** - one, `crates/nova_os/src/terminal/edit.rs` owns
  insert, kill, history and completion; `nova_os_ui` only routes keys into it.
- **`AppBuilder`** - no binary or tooling entry point bypasses it.
- **Asteroid kind validation** - the intended two-stage lint-then-load check,
  sharing one table (`objects/asteroid_kind.rs:253`). One asymmetry recorded
  below.
- **`merge_content_item`'s eight per-kind arms**, the `mod_cache` native/wasm
  split, `teardown_scenario_entities`, and the three `check_*_ship` lints were
  each examined and found correctly factored.

---

## Rejected and corrected

Claims that did not survive adjudication. Recorded so they are not re-filed.

- **`closing_speed` in four crates** - the orchestrator's own highest-priority
  mechanical lead, refuted independently by both cross-crate reviewers and
  confirmed here. `crates/nova_ship/src/sections/torpedo_section/bay.rs:1672`
  is inside the `#[cfg(test)]` module opening at `bay.rs:671`, and
  `crates/nova_ui/src/units.rs:77` is a `String` formatter. The two real
  implementations use different frames for documented reasons and agree on
  sign. The genuine duplicate underneath is the LOS range rate, filed as MINOR.
- **`window-capture-flow` is "already forked"** - struck. The pure cores do
  differ (`capture.rs:135` tests `!dir.is_empty()`, `view.rs:324` does not), but
  `view.rs:316-318` filters the empty case in its wrapper, so composed behavior
  is identical. The duplication is real; the live inconsistency is not. Left in
  as finding 15 without the false consequence.
- **`verb-grant-predicate` as BLOCKER** - downgraded to MAJOR; see finding 11.
- **"Player-input suspend guard at 17 sites" as MAJOR** - dropped. The
  predicate is already shared
  (`crates/nova_ship/src/input/player/control.rs:29`); what repeats is the
  one-line composition
  `pause.get().is_frozen() || player_control_is_suspended(control)`. Below this
  audit's threshold.
- **Four duplicate-block flags in `nova_scenario`** - all test code, refuted by
  the reviewer and confirmed here: `actions/spawn.rs` opens `mod tests` at line
  630, `objects/area.rs` at 237, `lint/scenario.rs` at 1701, and every flagged
  block sits below its gate.
- **`world_to_state_system` / `is_settling`** (trait plus impl),
  **`mouse_sensitivity`** (Bundle vs getter), **`serialize_content`** (a correct
  delegating wrapper, and the model for fixing the others), **`workspace_root`
  in `nova_gameplay`** (test-only), **`nova_bench/game.rs` vs
  `nova_probe_cli/native/cli.rs`** (assemble no plugins at all) - all refuted.

**Tooling note.** The orchestrator's duplicate-block index filters `/tests/`
directories and `test_support.rs` but does **not** see inline `#[cfg(test)]`
modules. Two reviewers caught this independently. Any future use of that script
must gate on the enclosing `#[cfg(test)]` span, not the path.

---

## Noted outside the duplication remit

- Two lines bypass `bevy_rand`, against AGENTS.md:
  `crates/nova_gameplay/src/transform/random_sphere_orbit.rs:107` and
  `crates/nova_gameplay/src/shake.rs:283`.
- `crates/nova_ship/src/flight/mod.rs:32-33` claims the AI is "a cruder version
  of the same idea" that "can adopt it later". Stale: it already shares
  `arrival_speed_limit` and `flip_lead`.
- A lint/load asymmetry, lint-stricter and therefore the documented direction:
  `crates/nova_scenario/src/lint/scenario.rs:536-542` errors on a kind weighted
  `0`, while `actions/spawn.rs:483-487` filters zero-weight entries before
  checking, so `[("rock",1),("bogus",0)]` fails lint and passes load.
- The `area` / `salvage` test rig is repeated five times
  (`objects/area.rs:260-274,340-354,404-418,509-523`,
  `objects/salvage.rs:355-369`) and deserves one shared `fn area_app() -> App`.
  Test hygiene, out of scope as a finding.

---

## Wave 3 - coverage-gap pass

A fifth pair read the surfaces the first eight passes listed as unread: the
editor inspector and `ui/mod.rs`, the portal, `wfc/collapse.rs`, theme and
colour resolution, `nova_authoring/{balance,generation}`, the unreached
`nova_hud` widgets, and the `asteroid_carve` question left open.

### 29. `scenario-name-vocabulary-two-tables` - BLOCKER

Agreed by 1 of 2. Verified independently during adjudication.

"What does this authored string name" is maintained in two tables that must be
edited together, and they have already fallen out of step:

- the lint's hand-written match, `crates/nova_scenario/src/lint/scenario.rs:660`,
  15 `check_target` arms and a `_ => {}` catch-all at `:1139`
- the `Names` reflect attributes the editor reads,
  `crates/nova_editor/src/event.rs:1726-1734`

`SetInfiniteAmmoActionConfig.id` (`crates/nova_scenario/src/actions/ship.rs:1837`)
and `RefillAmmoActionConfig.id` (`:1872`) both carry `#[reflect(@Names::Object)]`.
A grep of `crates/nova_scenario/src/lint/` for either action returns **nothing**,
so both fall through the catch-all.

**Consequence.** One dangling id gets three answers: the lint passes it, the
editor sandbox drops the handler, and the game emits
`warn!("SetInfiniteAmmo: no scoped ship with id '{}'")` (`ship.rs:1852`) and
does nothing. A scenario author or modder gets a green lint and a silently dead
action.

**Proposed home.** In-crate: move `walk_names` to
`crates/nova_scenario/src/names.rs` and fold the lint arms onto it, so the
reflect attribute is the single table.

### 30. `object-reference-prefix-guard` - MAJOR

Agreed by 1 of 2.

The same "does this target resolve" predicate three times; the lint copy
(`crates/nova_scenario/src/lint/scenario.rs:238-244`) lacks the empty-prefix
guard both editor copies carry (`crates/nova_editor/src/inspect.rs:2566`,
`crates/nova_editor/src/scenario.rs:1205`). One empty `id_prefix` makes
`satisfiable` true for every string and silently disables the whole reference
pass - and the editor's stock Scatter action is born empty
(`crates/nova_editor/src/event.rs:642`).

### 31. `editor-window-open-policy` - MAJOR

Agreed by 1 of 2.

The "one window at a time" invariant stated at
`crates/nova_editor/src/ui/window.rs:9-10` is enforced five times in two
dialects: toggle (`window.rs:520-528`, `:579-587`, `:625-632`, byte-identical)
and refuse (`window.rs:302-304`, `crates/nova_editor/src/ui/files.rs:102-104`).
No arbiter queries the window layer as a whole, and all three toggle windows
place at `fresh_window_left(size.x)`, so two different window types open
stacked on each other.

### 32. `editor-escape-ladder-thrice` - MAJOR

Agreed by 1 of 2. The editor's counterpart to finding 25.

One Escape ladder decided in three places: `crates/nova_editor/src/lib.rs:764-784`
(six rungs), `lib.rs:830-849` (three, though its own doc at `:825-829` names
six), `crates/nova_editor/src/ui/mod.rs:3045-3053` (five). The gallery and
rebind differences are explained by `InputMode` ownership; the typing rung is
not, so the key legend advertises "leave the ship" while a text field is
focused.

### 33. `subtree-collider-aabb-walk` - MAJOR

Agreed by 1 of 2.

`crates/nova_hud/src/screen_indicator.rs:400-419` and
`crates/nova_editor/src/frame.rs:334-353` are the same walk line for line, with
the same signature, and both doc comments argue the sensor-exclusion rule from
the same past beacon bug. The half-diagonal is then derived twice
(`target_inset.rs:583-596`, `frame.rs:250-260`). Proposed home `nova_gameplay`;
the editor does not depend on the HUD.

### 34. `nova-os-palette-second-copy` - MAJOR, with a recorded pair disagreement

Agreed by 1 of 2, and **the two reviewers contradicted each other.** Resolved
here by reading every constant.

`crates/nova_os_ui/src/terminal/style.rs:94-139` restates
`crates/nova_ui/src/theme.rs:29-80`. Eight role-matched tokens are byte
identical:

| Role | `nova_ui::theme` | `nova_os_ui::terminal::style` |
| --- | --- | --- |
| case 0 | `CASE_0` 10,13,16 | `NOVA_OS_CASE` 10,13,16 |
| case edge | `CASE_EDGE` 5,7,10 | `NOVA_OS_CASE_EDGE` 5,7,10 |
| case lit | `CASE_3` 47,56,63 | `NOVA_OS_CASE_LIT` 47,56,63 |
| case mid | `CASE_1` 22,27,32 | `NOVA_OS_CASE_MID` 22,27,32 |
| phosphor | `PHOSPHOR` 54,255,121 | `NOVA_OS_PHOSPHOR` 54,255,121 |
| body text | `SCREEN_TEXT` 185,255,201 | `NOVA_OS_TEXT` 185,255,201 |
| info blue | `BLUE` 54,163,255 | `NOVA_OS_INFO` 54,163,255 |
| amber | `AMBER_NOVA` 255,184,74 | `NOVA_OS_AMBER` 255,184,74 |

Four diverge: `PHOSPHOR_DIM` 25,166,79 against 95,238,137; `PHOSPHOR_MUTED`
13,110,53 against 70,207,118; `SCREEN_0` 0,19,4 against 0,4,1; `ORANGE`
255,123,45 against 255,120,40. `nova_os_ui/src/terminal/style.rs:10` already
imports `nova_ui`, so no dependency edge is needed.

**The disagreement.** One reviewer reported the theme family as already
factored and not duplicated; it read `nova_ui/{theme,skin}.rs` in isolation,
which was its assigned scope, and never compared them against the NOVA OS
style file, which sat in an earlier chunk. That is a scope gap, not a
contradiction of evidence. The eight-identical / four-different split above is
fact, verified constant by constant.

**Open question for the owner, not resolved here.** Whether the four
divergences are deliberate CRT art direction or drift. Both files cite the same
PoC HTML as their origin, which argues for drift, but that claim was not
verified against the PoC itself.

### 35. `portal-store-bypasses-storage` - MAJOR

Agreed by 1 of 2.

`crates/nova_assets/src/storage.rs:3-6` claims every platform gate lives there,
but `crates/nova_assets/src/portal/catalog.rs:218-223` and
`crates/nova_assets/src/mod_cache.rs:780-790` each retype the
`nova_protocol.` prefix from `WebStorage::key` plus a copy of the
`web_sys::window()?.local_storage().ok()?` handle. The namespace-pinning test
at `storage.rs:262-280` covers only `enabled_mods` and `settings`, so the two
hand-typed keys would be orphaned by a namespace change.

### 36. `confirm-window-hand-rolls-the-frame` - MAJOR

Agreed by 1 of 2.

`crates/nova_editor/src/ui/window.rs:313-479` duplicates `window_frame`
(`:843-949`); the title bars differ by exactly one line (the `Name`), and the
copy lacks the close button (`:910-928`) and the `max_height: percent(80)`
scroll bound (`:864`). It is the only editor window not using the shared frame.

### 37. `turret-reach-formula` - MAJOR

Agreed by 2 of 2, with a severity disagreement (one filed MAJOR, one MINOR).
Adjudicated as MAJOR on consequence.

`crates/nova_authoring/src/balance.rs:215-216` and `:234-236` retype the reach
derivation that `nova_ship` already names in `turret_section/mod.rs:264-272`
and `railgun_section/mod.rs:250-257`. `nova_authoring` already depends on
`nova_ship`, so this is not a dependency workaround. A stale formula makes the
balance audit pass ships whose guns cannot reach their own standoff band -
the audit silently certifies what it is meant to catch.

### Wave 3 MINORs

- `inspector-asset-kind-table` - the `(Image, AudioSource, WorldAsset)` set
  enumerated four times in `crates/nova_editor/src/inspect.rs`, none
  compiler-checked.
- `selected-row-reconciler` - seven copies; sibling of the already-recorded
  `disabled-row-reconciler`.
- `editor-window-teardown-clones`, `inspector-block-row-shell`,
  `hud-one-widget-per-target`, `item-highlight-crate-colour`.

### Wave 3 additions to existing families

- Finding 24 (`vfx-base-velocity-property`) has a seventh site at
  `crates/nova_authoring/src/balance.rs:187-191`.
- Finding 14 (`authored-object-walk`) has a second flat walk in the same file,
  `balance.rs:558-605`.

### Wave 3 refutations

- **`asteroid_carve` against `integrity/carve` - NOT duplicated. Agreed by 2 of
  2, the cleanest refutation in the audit**, and it closes a question an
  earlier pass left explicitly open. `crates/nova_gameplay/src/integrity/carve.rs`
  owns every mark decision - where a hit landed, how big a sphere it took, the
  budget and pricing, one writer at `carve_body:314`.
  `crates/nova_scenario/src/objects/asteroid_carve.rs` is a pure consumer
  (`carve_asteroid_fields:648`) that reads marks and rebuilds field plus mesh.
  The sphere subtraction exists exactly once, at
  `crates/nova_gameplay/src/mesh/field.rs:134-162`, and the shared pieces
  (`CHUNK_MIN_VOLUME`, `ChunkSpawn`, `spawn_carved_chunk`, `chunk_collider`)
  already come from `integrity/chunk.rs`. The apparently shared radius formula
  is not shared and must not be: `carve.rs:271-277` uses a hemisphere
  `(volume * 3.0 / (2.0 * PI)).cbrt()` for a crater, `asteroid_carve.rs:430` a
  whole sphere `(volume * 3.0 / (4.0 * PI)).cbrt()` for a severed lump. Correct
  in both.
- **The portal states the no-shadowing rule once**, at
  `crates/nova_assets/src/portal/install.rs:442-465`. Finding 7's count of four
  statements stands; there is no fifth.

---

## Coverage gaps

The wave-3 pair closed the list below; it is kept for the record, struck
where covered.

Closed by wave 3: the editor inspector and `ui/mod.rs`, `nova_assets/portal/**`,
`nova_wfc/src/collapse.rs`, `asteroid_carve.rs`, `nova_authoring/{balance,generation}`,
the unreached `nova_hud` widgets, and theme/colour resolution.

Originally named, before wave 3:

- `crates/nova_editor/src/ui/mod.rs` beyond flagged systems (5514 lines), and
  the editor inspector reconciler (~10.9k lines).
- `crates/nova_wfc/src/collapse.rs`, `crates/nova_assets/src/portal/**`,
  `crates/nova_scenario/src/objects/asteroid_carve.rs` - so an
  `asteroid_carve` against `nova_gameplay/src/integrity/carve.rs` overlap is
  neither confirmed nor refuted.
- `crates/nova_authoring/src/{balance,generation}` beyond the similarity pass.
- `nova_hud` widgets not reached by the breath and units families
  (`edge_indicators`, `lock_crosshairs`, `target_inset`, `maneuver_instruments`).
- Theme and colour resolution (`crates/nova_ui/src/theme*`) as a family.
