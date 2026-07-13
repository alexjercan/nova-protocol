# Spike: How should the per-weapon ammo readout be drawn - world-space diegetic widget or screen-projected HUD node?

- DATE: 20260712-143113
- STATUS: RECOMMENDED
- TAGS: spike, hud, weapons, ammo

## Question

Task 20260712-131348 asks for a readout of remaining rounds per weapon so the
player can see a turret or torpedo bay running dry. The desired feel is
diegetic - the count sits *on the weapon itself*, not in a corner panel - and
it need not be a number: a chunked circle that empties like a loading ring for
the PDC turret (`o` reading as `c` as it drains), a row of small bars `||||`
for the torpedo bay's remaining rockets, nothing (or a static mark) when the
weapon has infinite ammo, and a real number only in a debug mode.

The uncertainty this spike reduces: **what substrate do we draw that on?** A
genuine world-space 3D widget parented to the weapon, or a screen-projected UI
node anchored to the weapon via the existing `hud/screen_indicator.rs`? And
how does the answer fit `SectionAmmo`, the `infinite_ammo` path, and the
"intuitive at a glance, number only in debug" requirement. A good answer picks
one substrate, justifies it against nova's existing HUD convention, and is
concrete enough to plan without re-litigating.

## Context

- **Ammo model.** `SectionAmmo { rounds, capacity }`
  (`crates/nova_gameplay/src/sections/ammo.rs`) lives on the weapon *section*
  entity (the turret or torpedo bay). `rounds/capacity` is the exact fill
  fraction a readout wants, and `capacity` is the chunk count for a bar row.
  A section with **no** `SectionAmmo` fires without limit - that absence *is*
  the infinite-ammo state. The `infinite_ammo` player flag
  (`nova_scenario/.../spaceship.rs`) works by forcing `ammo_capacity = None`,
  so an infinite weapon simply has no component to read. A readout driven off
  "sections that have `SectionAmmo`" therefore shows nothing for infinite
  weapons for free, matching the "don't even show it" option.

- **The HUD is deliberately UI-pass.** Every overlay (`health`, `velocity`,
  `turret_lead`, `torpedo_target`, `edge_indicators`, ...) is a Bevy UI node,
  not a second `Camera2d` (a second window camera on Bevy 0.19 blacks out the
  3D scene - documented in `torpedo_target.rs`) and not gizmos (debug-grade,
  no image/text styling). This convention is stated explicitly at the top of
  `hud/screen_indicator.rs` and in `tasks/20260708-165647/SPIKE.md`.

- **A world-anchored substrate already exists.** `hud/screen_indicator.rs`
  projects a UI node to a world anchor every frame: anchor by `Entity` (follow
  its `GlobalTransform`, auto-hide when it despawns) or by `Point`; size
  `Fixed` or `ApparentSize` (track the entity's on-screen extent, min-clamped);
  offset in px; off-screen policy Hide or ClampToEdge-with-arrow. It projects
  through the `ScreenIndicatorCamera` in PostUpdate against the frame's final
  camera pose (jitter-free).

- **A per-weapon consumer pattern already exists.** `hud/turret_lead.rs` keeps
  exactly one indicator per turret child of the player ship with an idempotent
  reconcile system (`sync_turret_pips`): despawn the indicator of a turret that
  died or left the player, spawn one for any player turret missing it. Turret
  and torpedo sections carry stable markers (`TurretSectionMarker`,
  `TorpedoSectionMarker`) and are children of the player `SpaceshipRootMarker`.
  This is the ammo readout's template almost verbatim, with `Entity` anchors
  (follow the weapon) instead of the lead pip's computed `Point`.

- **No world-space UI precedent.** There is no billboard, look-at-camera, or
  3D-text helper anywhere in `crates/` (grep is empty). Sections render as 3D
  scenes/meshes parented under the section entity; nothing draws text or a
  fill gauge in the 3D pass. A true world-space readout is net-new
  infrastructure.

- **"Chunked", not analog.** The user's own description - `o` that looks like
  `c` when lower, and `||||` bars - is *quantized*: pick a discrete glyph or
  a count of pips from the ammo fraction. Neither substrate needs a smooth
  radial-fill shader; the turret ring can be a small set of bucket frames (or
  a discrete arc) and the torpedo bar is literally `capacity` child pips with
  `rounds` of them lit.

## Options considered

### Substrate

- **A. True world-space diegetic widget.** A billboarded quad/sprite (or small
  mesh set) parented under the weapon section, textured to show the fill;
  debug number as 3D text. Pros: maximally diegetic - part of the scene,
  occludes behind hull geometry, scales naturally with distance. Cons: net-new
  infrastructure with zero precedent (billboarding, a radial-fill material or a
  frame atlas, 3D text for the debug number); breaks the project's explicit
  UI-pass convention; legibility is *worse* where it matters - a weapon section
  is small and often self-occluded by the ship's own hull, so the count you
  most need when a turret is dialed away from camera is exactly the one you
  can't see; distance-shrink is the very problem `ApparentSize` was built to
  solve for the UI overlays. Reversibility: low - a bespoke render path to
  maintain.

- **B. Screen-projected UI node anchored to the weapon (recommended).** One
  `screen_indicator` per player weapon section, `Entity`-anchored to the
  section so it rides on the weapon in screen space, with a px offset so it
  hovers just off the barrel. Content is chosen by ammo fraction: for a turret,
  a small chunked ring (bucketed frames or a discrete arc node); for a torpedo
  bay, a `||||` row of `capacity` pips with `rounds` lit. Reuses projection,
  visibility lifecycle, sizing, offset, and camera-pose correctness wholesale;
  reuses `turret_lead`'s reconcile-per-weapon system almost verbatim; the
  `SectionAmmo`-presence filter gives infinite-ammo hiding for free; wasm-safe;
  matches every sibling overlay. Cons: pseudo-diegetic - it is a UI overlay
  painted at the weapon's screen position, so it does not occlude behind
  geometry and always faces the camera (which, for a readout, is a feature).
  Off-screen policy `Hide` keeps it from cluttering when the weapon is behind
  the camera.

- **C. Corner/panel readout (the obvious non-diegetic default).** A fixed list
  of weapons with counts, like a classic ammo counter. Cheapest, most legible,
  but explicitly *not* what the task wants (diegetic, on the weapon) and adds
  yet another static chrome panel. Rejected on feel, kept here as the baseline.

### Representation (independent of substrate)

- **Turret ring, chunked.** Map `rounds/capacity` to a small number of buckets
  (e.g. full / three-quarter / half / quarter / empty) and show the matching
  ring frame, so it "chunks" from `o` toward `c` as it drains. Bucketing (not a
  continuous fill) is what the user asked for and reads faster at a glance.
- **Torpedo bar, discrete pips.** `capacity` small vertical bars; `rounds` lit,
  the rest dimmed. Exact rocket count is readable when `capacity` is small (it
  is - torpedo bays hold few), which is the case a bar suits.
- **Debug number.** A `Text` child showing `rounds/capacity`, hidden unless a
  debug/dev toggle is on. Nova has no dev-overlay resource yet; the smallest
  version is a dedicated `AmmoReadoutDebug` resource (or reuse a diagnostics
  toggle if one lands), gating the text child's visibility. Minor, and
  separable from the core readout.

## Recommendation

**Option B: draw the ammo readout as a screen-projected UI node anchored per
weapon section, reusing `hud/screen_indicator.rs` and the `hud/turret_lead.rs`
reconcile pattern.** Representation is chunked/quantized per weapon family: a
bucketed ring for turrets, a discrete `||||` pip row for torpedo bays; a debug
number as a visibility-gated `Text` child.

Why B over A: nova has already made this exact call for every other combat
indicator, and for good reasons that apply here unchanged - the UI pass exists
precisely to get styleable, always-legible, camera-correct markers without a
second camera or debug gizmos. A true world-space widget is a net-new render
path (billboard + fill material + 3D text) whose main advantage, occlusion,
is actively *harmful* for a status readout: the moment the weapon tucks behind
the hull is the moment you still want to see it is dry. `ApparentSize` already
solves the "sits on the weapon and scales with it" part that makes B feel
attached rather than floaty. B is a thin consumer; A is infrastructure.

Why B over C: C is the anti-goal. The task explicitly wants the count on the
weapon, and B delivers that placement while keeping C's legibility.

What makes it feel diegetic despite being UI: anchor by `Entity` with
`ApparentSize` (min-clamped) so the widget tracks the weapon's on-screen size
and position, a small offset so it reads as *attached to* rather than *on top
of* the barrel, and `Hide` off-screen so it never floats in dead space. This
is the same recipe that makes the lead pips and lock reticle read as part of
the scene.

Fit with the ammo model and infinite ammo: reconcile spawns a readout only for
player weapon sections that **have** a `SectionAmmo`; infinite-ammo weapons
(no component, via `ammo_capacity = None`) get no readout at all - exactly the
"don't even show it" behavior, with no special-casing. A driver system each
frame reads `rounds/capacity` and updates the chunk state (turret bucket /
torpedo lit-pip count), mirroring `drive_pip_anchors`.

Tiering: register the readout as a combat `HudTier::Instrument` so it survives
the `Minimal` HUD level and clears in `None`, like the other weapon overlays.

## Open questions

- **Turret ring art.** Bucketed sprite frames vs. a few discrete arc nodes vs.
  a thin generated ring mesh-as-UI. Decide at plan/impl time; the bucketed
  approach needs the fewest moving parts and matches the `o`->`c` description.
  A smooth radial fill is explicitly out of scope (the user wants chunked).
- **Debug-number gating.** No dev-overlay resource exists yet. Land a tiny
  `AmmoReadoutDebug` toggle, or fold into a future general diagnostics overlay
  if one is planned. Non-blocking for the core readout.
- **Multi-pool future.** Task 20260708-162005 may split `SectionAmmo` into
  per-bullet-type magazines. The readout should then show the *selected*
  type's pool. Keep the driver reading a single "current pool" so swapping the
  source later is a one-line change, per the note in `ammo.rs`.
- **Promotion.** If a second world-anchored per-child overlay wants the same
  reconcile scaffold, factor the "one indicator per player section" loop out of
  `turret_lead`/this readout - not now, but flag it.

## Next steps

This spike backs the pre-existing task rather than seeding a new family; the
task's direction is now settled to Option B.

- tatr 20260712-131348 (Ammo HUD readout for weapon sections): implement the
  per-weapon screen-projected ammo readout per this spike - reconcile one
  `Entity`-anchored `screen_indicator` per player weapon section with
  `SectionAmmo`, chunked turret ring + discrete torpedo pip row content, a
  driver reading `rounds/capacity`, infinite-ammo weapons drawing nothing, a
  debug-gated number, registered as `HudTier::Instrument`. Spike:
  tasks/20260712-143113/SPIKE.md.
