# Review: Editor preview controller spams PD 'root not found' errors

- TASK: 20260706-212909
- BRANCH: fix/editor-preview-controller

## Round 1

- VERDICT: APPROVE

Diff adds a render-only `preview_controller_section` (nova_gameplay), rewires the editor's two
preview-controller spawn sites onto it, gates `insert_controller_section_target` on
`With<PDController>`, and adds two co-located tests.

Verified:

- Root cause confirmed in the bcs source: `update_controller_root_torque` iterates
  `(PDController, PDControllerInput, PDControllerTarget, PDControllerOutput)` and errors when the
  target root lacks `(ComputedAngularInertia, Rotation, AngularVelocity)` - i.e. a rigid body.
  The editor gates nova's own controller systems to the Scenario state, but the bcs PD system is
  not state-gated, so it runs in the editor against the preview controller. Removing `PDController`
  from the preview takes it out of that query entirely - a structural fix, not a suppression.
- No regression to real ships. `controller_section` still bundles `PDController` with
  `ControllerSectionMarker`, so when the `With<PDController>`-gated target observer fires on Add,
  the component is already present (same bundle) and the root is still targeted. The
  `only_a_live_controller_gets_a_pd_target` test pins exactly this: live controller gets a target,
  preview controller does not.
- The observer's dropped `error!` is correct: a controller with no `PDController` is now a valid
  (preview) case, so silently returning is right, not a swallowed error. A live controller always
  has both components, so its path is unchanged.
- Both editor preview sites are covered (create-with-controller and the section-placement
  handler); I grepped for other `controller_section(` uses and the only remaining live ones are
  the scenario ship spawn (`nova_scenario/objects/spaceship.rs`) and the torpedo warhead
  (`torpedo_section`) - both real physics bodies that correctly keep a live controller.
- Tests assert behavior, and the "can never match the bcs query" argument is sound: the preview
  controller has neither `PDController` nor `PDControllerTarget`.
- Green: `cargo clippy --workspace --all-targets` clean (only the pre-existing `hull_section.rs`
  warning), `cargo test --workspace` (58 nova_gameplay incl. 2 new, examples_smoke under Xvfb),
  and a headless editor boot with no panic and no `q_root` spam.

Honest scope note in TASK.md: the UI-triggered spam is not reproduced headless (needs a pointer);
it is proven structurally + a clean boot. The sibling preview kinds (turret/thruster/torpedo)
carrying live behavior on a non-physics root is flagged as a possible follow-up, correctly out of
scope here.

No BLOCKER/MAJOR/MINOR findings.
