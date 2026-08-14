# NOTES

## Diagnosis (confirmed)

As filed. The velocity sphere is a real world-space `Mesh3d`, Bevy casts from
every such mesh by default, and the spawn carried no opt-out. The sphere is
centred a radius AHEAD of the ship and scaled to that radius, so it sits between
the key light and the hull and its round shadow lands square on the ship.

`NotShadowCaster` appeared nowhere in the workspace, so this was a missing
convention rather than one bad spawn. Every world-space HUD mesh had it.

## Fix

`NotShadowCaster` on all seven world-space HUD meshes:

- `velocity.rs` - the direction sphere and the orbiting cone
- `holo_instruments.rs` - the trajectory ribbon segments and the flip gate
- `maneuver_instruments.rs` - the orbit ring torus and the radius spoke
- `target_inset.rs` - the section highlight shell

The convention now lives in the `nova_hud` crate doc, next to the reason: an
instrument is a projection the flight computer draws for the pilot, not a thing
in the world.

### Not `NotShadowReceiver`

Deliberate, and checked rather than assumed:

- The holo instruments and the target-inset highlight are already `unlit: true`
  materials, which never sample lighting, so the component would be dead weight.
- The velocity sphere and cone are lit `ExtendedMaterial<StandardMaterial, _>`
  with `perceptual_roughness: 1.0`. Their shading is deliberate and the sphere is
  alpha-blended at 0.2, so nothing in the captured frames argued for changing it.

### Enforcement

Four regressions, one per module, each asserting the spawned mesh entities carry
`NotShadowCaster`: `the_widget_meshes_never_cast_a_shadow`,
`the_holo_geometry_never_casts_a_shadow`,
`the_orbit_holo_geometry_never_casts_a_shadow` and
`the_highlight_shell_never_casts_a_shadow`. Two of them query every `Mesh3d` in
their test world rather than named entities, so a new mesh added to those systems
is covered without touching the test.

Not the "every entity in the HUD's render layer" lint the task floated: there is
no HUD render layer - `RenderLayers` appears nowhere in `nova_hud` - and the one
marker that spans the crate, `HudTier`, sits on widget ROOTS while the opt-out is
per-entity on the mesh. The target-inset highlight is not even under a HUD root;
it is a child of the ship section it marks.

## Verification

From RENDERED FRAMES, per the DoD. `screenshot_flight` captured twice under Xvfb,
once with the fix stashed:

```text
NOVA_SHOT_DIR=... NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
  cargo run --example screenshot_flight --features debug
```

`shots/hull-before.png` and `shots/hull-after.png` are the same crop of
`feature-autopilot.png` from the two runs. Before, the hull is uniformly dulled -
the sphere's shadow across it. After, it is lit warm with its own highlight and
shade. The same change reads on `variant-flight-chase` and on the rest of the
set. Also 216/216 `nova_hud` lib tests and a clean workspace `cargo check`.

## Follow-up, not done here

The fix brightens three SHIPPED manifest images (`tutorial-orbit`,
`feature-autopilot`, `wiki-flight`). They want a re-capture, but on a real GPU -
these frames came off llvmpipe under Xvfb, which is fine for a before/after
comparison and not what the manifest images should ship from.
