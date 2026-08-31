# Section animation research

How an authored section declares an animation. Three known consumers: the
torpedo bay's muzzle doors (this task), the railgun charge cue
(20260824-125947), and the PDC stow (20260831-083622). The owner's framing:
these are procedural animations, mostly a state machine plus coded logic,
but "what if we change the door type" - the declaration must stay generic.

## What the runtime does today (facts, from the code)

- A section's art is one glb spawned two children under the section entity:
  `section -> "Torpedo Section Body" -> [SectionRenderOf + WorldAssetRoot]
  -> gltf nodes`. Bevy 0.19 replaced `SceneRoot` with `WorldAssetRoot`;
  `WorldInstanceReady` fires on that entity when the nodes exist
  (`bevy_world_serialization-0.19.1/src/world_asset_spawner.rs:33`).
- The gltf loader names node entities (`bevy_gltf-0.19.1/src/loader/mod.rs:
  1768`), but NOTHING in the workspace reaches into a spawned scene by
  `Name`. Zero hits for `WorldInstanceReady` or scene-child queries.
- No `AnimationPlayer`, `AnimationClip`, `AnimationGraph` anywhere in the
  workspace. Every moving part is procedural on nova-owned entities: turret
  joints are separate glbs per joint, driven by `SmoothLookRotation`
  (`nova_gameplay/src/transform/smooth_look_rotation.rs`), whose sync system
  REPLACES `transform.rotation` - it cannot compose with a node's authored
  base rotation.
- The shipped art has no named nodes and no clips: `bay_tube.glb` is one
  unnamed node with one merged mesh, emitted by `scripts/nova_glb.py`
  (stdlib-only, byte-deterministic). Correction to the research brief: the
  generator does NOT use trimesh; it is our own glb writer, so any clip
  authoring means emitting the animation JSON and sampler accessors
  ourselves.
- The torpedo fire path has no pre-launch event. `shoot_spawn_projectile`
  (`nova_ship/src/sections/torpedo_section/bay.rs`) spawns the torpedo in
  the same system body that reads the trigger; the 0.6 s `ignition_delay`
  coast (`TorpedoColdLaunch`) is the emergence window the task hangs art on.

## Q1. What Bevy 0.19 gives us

The workspace pulls `bevy = "0.19.0"` with default features, so
`bevy_animation` 0.19.1 is compiled in and `load_animations` defaults on.
If a glb carried clips, the loader would load `AnimationClip` assets, tag
animated nodes with `AnimationTargetId` + `AnimatedBy`, and insert a
default `AnimationPlayer` on each animation root
(`bevy_gltf-0.19.1/src/loader/mod.rs:1088-1094`).

Playback still needs runtime wiring per instance: build an
`AnimationGraph::from_clip(handle)`, insert `AnimationGraphHandle` next to
the player, then `player.play(node)`. A door needs open, hold, close,
reopen-while-closing; with clips that is `set_speed` (negative to reverse),
`seek_to`, and repeat handling - a small bespoke state machine ANYWAY, on
top of graph assets, clip labels through the `AssetRef` plumbing, and the
first `AnimationPlayer` usage in the codebase.

## Q2. Can our generator author clips?

Yes, with work in our own writer, not via a library. `nova_glb.write_glb`
emits the glTF JSON directly; clips are an `animations` array plus two
accessors per channel (time input, value output) in the BIN chunk. Cost:
roughly 100 lines of deterministic writer code, plus recipe schema for
keyframes, plus the Q1 runtime wiring. It buys authored easing curves,
which nothing needs: every known consumer is a constant-rate mechanical
motion described by an axis, a range, and a duration.

## Q3. The procedural alternative

A shared state machine plus motion described as authored data, over a
node-naming convention. Sketch:

```ron
// on BaseSectionConfig, default empty, elided when empty
animations: [
    (
        cue: MuzzleDoor,             // WHEN: which gameplay moment drives it
        nodes: "door_",              // WHAT: gltf node name prefix
        motion: RotateX(105.0),      // HOW: about each node's local X hinge
        open_seconds: 0.25,
        close_seconds: 0.7,
    ),
]
```

- The glb authors each moving part as a named node whose local frame IS the
  mechanism: origin on the hinge, X along the hinge axis. The node's base
  transform places it; the runtime composes `base * rotate_x(angle)`.
- A generic driver (nova_ship) resolves the named nodes when
  `WorldInstanceReady` fires, then eases a progress value toward a target
  and writes child transforms. It knows nothing about torpedoes.
- Kind code owns WHEN: the bay's fire path holds the `MuzzleDoor` cue open
  across the ignition window and releases it after. The railgun will hold a
  `Charge` cue; the stow a `Stow` cue. Cues are the closed enum; undriven
  cues rest at 0.

Swapping a door type costs: new nodes in the recipe, new `nodes` prefix or
`motion` values in the builder. No Rust beyond a new `motion` variant when
a genuinely new mechanism appears (slide = translate along local axis, one
arm in one match). That answers "what if we change the door type": the
door type lives in data, the machinery does not change.

## Q4. The hybrid

Clip-by-name when the asset has one, archetype fallback when it does not.
Two authoring paths and two runtime paths, kept consistent forever, gated
by an asset introspection ("does this glb have a clip named X"). Nothing
today produces a clip, so the clip half would ship dead. Not now. The seam
stays cheap to add later: the declaration already names the cue and the
target nodes; a future `motion: Clip("open")` variant slots into the same
enum if hand-authored art ever arrives with baked animation.

## Comparison

| | glTF clips (Q1+Q2) | procedural archetype (Q3) | hybrid (Q4) |
|---|---|---|---|
| new writer code | ~100 lines + keyframe schema | named-node support only | both |
| new runtime code | graph/player wiring + state machine | state machine + transform driver | both |
| repo precedent | none | all animation is procedural | half |
| reverse/hold/interrupt | seek/speed juggling | trivial (progress toward target) | mixed |
| door-type swap | re-author keyframes | edit recipe + two numbers | depends |
| testability | needs animation assets | plain ECS unit tests | mixed |
| authored easing curves | yes | no (constant rate) | yes |

## Recommendation

Option Q3, the procedural archetype. One declaration on the section config
(cue, node prefix, motion, durations), named nodes in the glb whose local
frames encode the hinge, a generic driver that eases progress and composes
transforms, and kind-owned state machines that only write the cue target.
It matches every precedent in the codebase, it is the smallest thing that
serves all three known consumers, reversal and interruption are free, and
it is testable headless without assets. Clips stay a future `motion`
variant, not a parallel system.
