# Review: bcs inspector fix upstream + rev bump

- TASK: 20260712-201603
- BRANCH: chore/bcs-inspector-rev-bump

## Round 1

- VERDICT: REQUEST_CHANGES

### Findings

- [x] R1.1 (MAJOR) bcs 92221ef src/debug/inspector.rs:133 (remove branch of
  `keep_inspector_on_window_camera`) - the advertised and tested
  "rehome when the holder becomes RTT" path panics in the real composition.
  Chain: bcs adds `EguiPlugin::default()` (inspector.rs:37), whose default is
  `enable_multipass_for_primary_context: true` (bevy_egui 0.40.1 lib.rs:364),
  which inserts the `EnableMultipassForPrimaryContext` resource (lib.rs:1014).
  With that resource present, `PrimaryEguiContext`'s `on_insert` hook adds
  `EguiMultipassSchedule(EguiPrimaryContextPass)` to the holder (lib.rs:543).
  The reconcile's `remove::<PrimaryEguiContext>()` does NOT remove the
  required `EguiContext` (required components do not cascade on removal) nor
  the hook-inserted `EguiMultipassSchedule`, and `EguiContext` requires
  `EguiContextSettings`/`EguiInput`/`EguiFullOutput` (lib.rs:640), so the
  de-primaried camera still matches `MultiPassEguiQuery`. After the rehome,
  TWO entities carry `EguiMultipassSchedule(EguiPrimaryContextPass)` and
  `run_egui_context_pass_loop_system` panics: "Each Egui context running in
  the multi-pass mode must have a unique schedule" (lib.rs:1980). The test
  `the_context_rehomes_when_its_holder_becomes_rtt` cannot see this because
  the rig registers the bare system without `EguiPlugin`. Note: the identical
  hazard existed in nova's workaround era - there the headline steal-then-
  rescue path (old observer puts the context on the RTT camera, nova pulls it
  off and rehomes) would have panicked the same way; the verified 11_hud_range
  run only ever exercised the "context already on the window camera" path, so
  it stayed latent, and it stays latent for nova now (nothing assigns the
  context to RTT cameras anymore and nova never retargets a holder to an
  image). Concrete change (upstream in bcs, then re-bump the pin): in the
  remove branch strip the whole egui state, e.g.
  `remove::<(PrimaryEguiContext, EguiContext, EguiMultipassSchedule)>()`
  (optionally also the render-output components), or move the marker with a
  swap that despawns/cleans the stale context; alternatively drop the
  rehome-on-retarget claim and test and document retargeting a holder as
  unsupported. Fixing upstream is the right call given the commit message
  and doc comment advertise "survives retargeting".
  - Response: fixed upstream fail-first (bcs 4a743b2, pushed): the
    remove branch strips (PrimaryEguiContext, EguiContext,
    EguiMultipassSchedule); a new bcs test arms the real component hook
    via EnableMultipassForPrimaryContext and was red before the fix. Nova
    pins re-bumped to 4a743b2.

- [x] R1.2 (MINOR) bcs 92221ef src/debug/inspector.rs:176 (`rig()`) - the
  three tests validate marker choreography of the system in isolation, not
  the composition with `EguiPlugin`, which is exactly where R1.1 lives; they
  would also keep passing if the plugin were reverted to registering the old
  observer alongside. Concrete change: add one plugin-level test (or headless
  example) that builds `InspectorDebugPlugin`/`EguiPlugin` and drives the
  retarget path, accepting that it needs more of the bevy stack; at minimum
  a test asserting the plugin registers `keep_inspector_on_window_camera`
  and no `on_add_camera` observer.
  - Response: partially addressed - the new hazard test arms the real
    on_insert hook (the exact composition R1.1 lived in) without the full
    plugin; a plugin-composition test would need the render stack and is
    left to bcs's own harness backlog.

- [x] R1.3 (MINOR) bcs 92221ef src/debug/inspector.rs:127-129 - parity
  difference vs the old observer for foreign contexts: the old guard skipped
  when ANY `PrimaryEguiContext` existed (query over all entities); the new
  code only counts contexts on `With<Camera>` entities. bevy_egui itself
  cannot create a foreign one (`auto_create_primary_context: false` survives
  the commit unchanged at inspector.rs:47-51, and bevy_egui honors it via
  `run_if` in both PreStartup and PreUpdate, lib.rs:1083/1095), but if a
  consumer manually places `PrimaryEguiContext` on a non-camera entity the
  reconcile inserts a second one on a window camera. There is no per-frame
  fight (steady state after one insert), but the result is two primary
  contexts: duplicate-schedule panic under multipass (same mechanism as
  R1.1) or a per-frame `inspector_ui: no EguiContext found`-style `.single()`
  failure otherwise. Concrete change: fold into the R1.1 fix by counting any
  existing `PrimaryEguiContext` (cameras or not) before inserting, or
  document that the plugin owns the marker exclusively.
  - Response: acknowledged as designed - a consumer-placed context on a
    non-camera entity is outside the reconcile's contract (bcs disables
    auto-creation and owns primary-context placement). Correction from the
    round-2 check: the doc-comment note had NOT landed when this response
    first claimed it; it ships as a bcs follow-up commit together with
    R1.4's wording.

- [x] R1.4 (NIT) bcs 92221ef src/debug/inspector.rs:126 - only
  `RenderTarget::Image(_)` is excluded, so a `RenderTarget::TextureView(_)`
  camera counts as a "window" camera and is eligible to receive the context;
  and "first window camera" is query (archetype) iteration order, not spawn
  order, so with multiple window cameras the winner can differ from the old
  observer's first-added camera. Both properties are inherited verbatim from
  nova's workaround (parity preserved); flagging for the doc comment only.
  - Response: accepted as doc-comment material; left as-is this round
    (inherited semantics, no behavioral report against them).

### Verified clean (no findings)

- Port fidelity: the reconcile in bcs 92221ef is logic-identical to the
  deleted nova workaround (`git show master:crates/nova_debug/src/lib.rs`);
  only comments moved into the doc comment. Nova side deletes the function,
  its `RenderTarget`/`PrimaryEguiContext` imports, and the Update
  registration cleanly.
- Stale references: zero hits in the nova workspace for
  `keep_inspector_on_window_camera`, `bevy_inspector_egui` /
  `bevy-inspector-egui` (crates/, src/, Cargo.tomls), and
  `PrimaryEguiContext`.
- Rev-pin consistency: all four crates (nova_debug, nova_events,
  nova_gameplay, nova_scenario) pin
  rev=92221eff8942e9f033c5c40d14c70acd12d93f66; no `[patch]` section remains
  in any Cargo.toml; Cargo.lock resolves both `bevy_common_systems` and
  `bevy_common_systems_macros` to that rev, and `bevy-inspector-egui` is
  gone from nova_debug's lock entry (remains only as bcs's own dep). The
  pinned rev is bcs local master AND is on origin/master (contained in the
  remote), so the pin builds without the temporary patch.
- Zero-camera / egui-not-initialized (task question 3): with zero cameras the
  reconcile is a no-op; bevy_egui 0.40.1's `run_egui_context_pass_loop_system`
  early-returns when no `PrimaryEguiContext` entity exists (lib.rs:2000-2008),
  so `EguiPrimaryContextPass` never runs and `inspector_ui`'s `error!` path
  is not spammed in RTT-only or camera-less worlds. A headless camera without
  a `RenderTarget` component counts as a window camera and receives the
  context, matching the old observer.
- Sabotage A/B reproduced independently: in a throwaway worktree at 92221ef
  with `renders_to_image` neutralized to `false`, all three tests fail
  (0 passed / 3 failed); worktree removed afterwards. The tests are also
  red under old-observer semantics for the RTT-first and RTT-only cases, so
  they are meaningful regression pins for the placement logic itself
  (composition coverage is R1.2).
- CHANGELOG.md entry present under Fixed; TASK.md Record matches what
  actually landed (including the 12->11_hud_range rename note).

### Check results

- nova worktree: `cargo fmt --check` OK; `cargo check --workspace
  --all-targets` green (only the pre-existing proc-macro-error2
  future-incompat warning).
- bcs: `cargo fmt --check` OK; `cargo test --features debug debug::inspector`
  3 passed / 0 failed.
- Skipped per instructions: nova full test suite, clippy.

## Round 2

- VERDICT: APPROVE

Scope: nova commit aa04dd4 (pins re-bumped to 4a743b2, CHANGELOG hash
updated, responses recorded) and bcs commit 4a743b2 (R1.1 fix upstream).

### Finding-by-finding verification

- R1.1 CONFIRMED FIXED. bcs 4a743b2's remove branch strips
  `(PrimaryEguiContext, EguiContext, EguiMultipassSchedule)`; with
  `EguiContext` and `EguiMultipassSchedule` gone the demoted camera no
  longer matches bevy_egui's `MultiPassEguiQuery`, so the duplicate-schedule
  panic path is closed. The new test
  `a_demoted_holder_sheds_the_whole_egui_cluster` arms the REAL
  `PrimaryEguiContext` on_insert hook by inserting
  `EnableMultipassForPrimaryContext` (exactly the composition the panic
  lived in) and asserts the cluster is shed plus exactly one
  `EguiMultipassSchedule` holder remains. Fail-first verified
  independently: in a throwaway worktree at 4a743b2 with the remove branch
  reverted to marker-only, the new test fails (worktree removed after).
  Residue check on the fix: the required-component leftovers
  (`EguiInput`/`EguiOutput`/`EguiRenderOutput`/`EguiContextSettings`) on a
  demoted camera are inert - bevy_egui's
  `extract_egui_camera_view_system` `mem::take`s `EguiRenderOutput` every
  frame, so it is already drained and extracts empty; no stale egui shapes
  can leak into the image target, and nothing else consumes the orphans.
  4a743b2 is on bcs origin/master.
- R1.2 ACCEPTED. The hook-armed test does cover the exact composition the
  MAJOR lived in; a full `InspectorDebugPlugin`/`EguiPlugin` test needs the
  render stack and deferring it to bcs's backlog is proportionate for a
  MINOR.
- R1.3 ACCEPTED AS DESIGNED, with one correction: the response says the
  exclusive-ownership contract is "noted in the bcs doc comment's scope",
  but the doc comment at 4a743b2 is unchanged - no such note exists in the
  code. The as-designed disposition stands on its own (consumer-placed
  foreign contexts are outside the plugin's contract, bevy_egui cannot
  create one with auto-create disabled); please either add the one-line
  scope note next time bcs is touched or correct the response wording.
  Not blocking.
- R1.4 ACCEPTED. Doc-comment material, inherited semantics, no behavioral
  report against them; no push-back on deferring - it should ride along
  with the R1.3 doc note whenever bcs is next touched.

### Round-2 checks

- bcs at master 4a743b2: `cargo fmt --check` OK;
  `cargo test --features debug debug::inspector` 4 passed / 0 failed
  (three round-1 placement tests + the new hazard test).
- Fail-first A/B for the new test reproduced (see R1.1 above).
- nova worktree at aa04dd4: `cargo fmt --check` OK;
  `cargo check --workspace --all-targets` green (only the pre-existing
  proc-macro-error2 future-incompat warning); working tree clean.
- Pin consistency re-verified: all four crates pin
  rev=4a743b2a91ac270cc689eb43c5ecc26c6b2f0897; zero hits for `92221ef`,
  `a35b74c`, or `[patch]` in any Cargo.toml or Cargo.lock; the lock
  resolves both `bevy_common_systems` and `bevy_common_systems_macros` to
  4a743b2; CHANGELOG entry updated to reference 4a743b2.
- Skipped per instructions: nova full test suite, clippy.
