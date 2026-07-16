# Spike: Diegetic HP - move the health readout onto the ship

- DATE: 20260711-202901
- STATUS: RECOMMENDED
- TAGS: spike, hud, ui, v0.7.0

## Question

The generic screen-space health bar (bevy_common_systems `HealthDisplay`,
spawned in `hud/mod.rs::setup_hud_health`) reads as chrome bolted onto the
screen and throws away the one thing Nova's health model is rich in:
per-section integrity. This spike reduces one uncertainty: in which visual
language does the ship's health readout live once it comes off the screen
edge, and does the ship itself become the display or does a new widget carry
it? A good answer picks one primary channel, is honest about what it costs to
read an exact number, says what happens to the generic bar, and is concrete
enough that `/plan` can expand it without re-litigating the choice.

## Context

What already exists (file:line in `crates/nova_gameplay/src`):

- **The generic bar.** `hud/mod.rs::setup_hud_health` (mod.rs:519-541) spawns
  one `health_display(HealthDisplayConfig { target: ship })` from
  bevy_common_systems, tagged `HudTier::Instrument`. It reads the *root*
  `Health` only - a single aggregate number.
- **Health is per-section.** Every section (`sections/base_section.rs`) carries
  a bcs `Health { current, max }`. `integrity/glue.rs::aggregate_ship_health`
  (glue.rs:130-186) sums living section health into the root every frame; the
  root `Health` the bar reads is that sum. Sections are direct children of the
  `SpaceshipRootMarker` entity, laid out on a 1-unit grid, each with a
  `Transform` (local position) and a `ConnectedTo(neighbors)` graph.
- **Sections already react to death, not to damage.** On zero health a section
  gets `IntegrityDisabledMarker`; non-leaf -> `SectionInactiveMarker`, leaf ->
  destroyed and `explode.rs` slices the mesh into debris + `OnDestroyedEvent`.
  So today the ship shows *dead* sections (they blow off) but a section at 30%
  looks identical to one at 100%. The damaged-but-alive gradient is invisible
  in-world - exactly the information the bar flattens into one number.
- **Section meshes are tintable.** Default sections are a cuboid with
  `MeshMaterial3d(materials.add(Color::srgb(0.8,0.8,0.8)))`
  (`sections/hull_section.rs:87-92`); hull sections may instead load a gltf
  `WorldAssetRoot` scene (hull_section.rs:78-84) whose nested meshes carry their
  own materials. Each render child back-references its section via
  `SectionRenderOf`.
- **The chip family / screen_indicator substrate.** `hud/screen_indicator.rs`
  projects a UI node onto a world anchor (`ScreenIndicatorAnchor::Entity`) with
  a pixel `Offset`, `Hide`/`ClampToEdge` offscreen policy, and Fixed /
  ApparentSize / WorldRadius sizing. `hud/flight_status.rs` already rides the
  ship with a speed chip and an autopilot mode chip in the NAV_CYAN palette
  (flight_status.rs:64-250). Sibling spike `tasks/20260710-234019/SPIKE.md`
  established this as the HUD's diegetic vocabulary: world-space holo geometry +
  screen-projected chips, with all text on the UI pass (no billboard text).
- **HUD tiers.** `HudVisibility` (All/Minimal/None) x `HudTier`
  (Instrument/Chrome), enforced by `apply_hud_visibility` in PostUpdate
  (mod.rs:303-335). The current bar is `Instrument`. Note the tier system hides
  *UI nodes*; it does not touch world-space meshes.
- **Promotion note.** `web/src/wiki/dev/architecture.md` keeps generic helpers
  (including health / status bar) in bevy_common_systems; the task flags
  `hud/health.rs` as a promotion candidate and asks whether a diegetic
  replacement changes that calculus.
- **Live health-bar churn.** Two open sibling tasks touch the current bar's
  behaviour: 20260716-165617 (percent rounds a living sliver to 0%) and the
  enemy-ghost-at-0-HP bug 20260716-162701 - evidence the aggregate-number bar is
  both buggy and low-signal.

## Options considered

### Option 1 - Per-section damage tint/glow on the ship's own meshes (recommended primary)

Drive each section's material toward a damage colour as its `Health` falls -
e.g. lerp the base albedo (and/or an emissive term) from neutral -> amber ->
red as `current/max` drops, so a battered flank visibly reddens and a healthy
one stays clean. The ship *is* the readout.

- **How it works here.** A PostUpdate system queries
  `(&Health, section render children)` per `SectionMarker`, computes the 0..1
  ratio, and writes the material colour. For default-cuboid sections this is a
  one-line `MeshMaterial3d` colour swap (or a handle to a shared damage-graded
  material). For gltf sections it means overriding the scene's materials
  (walk `SectionRenderOf` children, swap/patch their `StandardMaterial`), or
  layering an emissive damage term. `SectionInactiveMarker` sections read as
  fully dark/dead; destroyed sections already blow off.
- **Pros.** Fully diegetic - no screen furniture at all. Surfaces the
  per-section integrity the aggregate bar discards, and fills the
  damaged-but-alive gap that today only death reveals. Reads at a glance and
  spatially (you see *which* side is hurt, which the number never told you).
  Complements, not fights, the existing explode-on-death feedback. Reinforces
  the ship-cluster diegetic language the sibling spike committed to.
- **Cons / unknowns.** No exact number - a colour can't say "42%" for a docking
  or repair decision (same tradeoff the sibling spike hit for speed, which it
  solved by keeping a numeric chip). Legibility depends on camera framing: the
  chase camera is not locked top-down, so at some angles/distances a hull face
  is small or edge-on; the ship can also leave frame. gltf-scene material
  override is more work than a cuboid colour swap and risks stomping authored
  looks (mitigate with an emissive/rim damage overlay rather than replacing
  albedo). Colour-only excludes colour-blind players unless paired with a
  value/brightness ramp. HUD `None` (cinematic) can't hide a world mesh tint the
  way it hides a UI node - if we want damage tint to vanish for clean shots it
  needs its own toggle.

### Option 2 - Ship-anchored hull-integrity chip in the flight-status family

A compact numeric chip (e.g. `HULL 84%` or `672/800`) projected beside the
speed/mode chips via `screen_indicator`, anchored to the ship, NAV_CYAN,
`HudTier::Instrument`.

- **How it works here.** Near-copy of `drive_speed_chip`
  (flight_status.rs:201-224): read the root `Health`, format, update the chip's
  text and anchor. Minimal new tech; proven substrate.
- **Pros.** Cheapest and lowest-risk. Gives the exact value the tint can't.
  Sits in the established chip cluster and inherits tier visibility for free.
- **Cons.** Least diegetic of the three - it relocates the bar to ride the ship
  but it is still a screen-space number, and on its own it keeps flattening
  health to one aggregate figure (the same low-signal readout, just anchored).
  Does not answer "*on/in* the ship" so much as "*next to* the ship."

### Option 3 - Schematic mini-ship (top-down section outline, coloured per section)

A HUD widget drawing the ship's section grid as tiles, each tile coloured by
its section's health (FTL / Star Citizen style).

- **How it works here.** New widget: build a tile layout from the section grid
  positions (`Transform` on each `SectionMarker`, already 1-unit spaced), colour
  each tile from that section's `Health`, tag `HudTier::Instrument`.
- **Pros.** Per-section detail in a compact, always-legible, camera-independent
  panel. Squarely in the flight-computer visual language.
- **Cons.** The most new code (layout synthesis from the section graph, redraw on
  section add/destroy). And it is largely *redundant with Option 1*: it is a
  schematic top-down section map of a ship that, on a top-down-ish camera, the
  real ship already is - so we would be building a small copy of the ship to
  colour it, instead of colouring the ship. It is screen furniture, which is the
  thing the task set out to remove.

### Option 4 - Do nothing (keep the generic bar)

Rejected by the task: the bar is generic, low-signal, and currently the subject
of two bug tasks. Deferring costs nothing to build but leaves the per-section
richness unused and the readout undiegetic.

## Recommendation

**Make the ship its own health readout via per-section mesh damage tint/glow
(Option 1), and retire the generic bar for the player ship.** Pair it with a
small numeric hull chip (Option 2) in a subordinate role so the exact value
survives - the same shape the sibling flight-status spike used for speed
(diegetic channel carries the gestalt, a chip carries the number).

Why Option 1 over the others:

- It is the only option that literally puts health *on/in* the ship, which is
  the task's ask, and it turns Nova's per-section integrity from an internal
  detail into the primary feedback - closing the damaged-but-alive blind spot
  that today only death reveals.
- It beats Option 3 because on this camera the real ship already reads as the
  section layout a mini-ship would redraw; colour the ship, don't build a copy.
  Option 3 stays a strong *fallback* if playtest shows the on-ship tint is
  illegible at combat framing.
- It beats Option 2-alone because a lone anchored number is barely more diegetic
  than the bar and keeps health flattened to one figure. But Option 2 is the
  right *backstop*, so keep it - subordinate, not the headline.

Concretely for `/plan`:

1. **Per-section damage tint** (primary): a system grading each section's
   material by `current/max`. v1 targets default-cuboid sections with a colour
   ramp that also varies brightness (colour-blind safe); gltf-scene sections get
   an emissive/rim damage overlay rather than an albedo replace so authored art
   survives. Dead/inactive sections read dark; destroyed sections already
   detach.
2. **Retire the generic bar for the ship**: stop spawning `HealthDisplay` in
   `setup_hud_health`. Land this in the same change that ships the tint so the
   readout is replaced, not duplicated.
3. **Numeric hull chip** (subordinate backstop): a `screen_indicator` chip in
   the flight-status family showing the aggregate percent/value, `HudTier::
   Instrument`, NAV_CYAN. Can ship in the same task or as a fast follow.

**Promotion calculus.** The diegetic replacement is Nova-specific (it keys on
Nova's section graph and materials), so it does *not* become a bevy_common_
systems promotion candidate. The generic `HealthDisplay` stays in the library,
available for other games and for non-player entities; Nova's player ship simply
stops using it. Update the architecture note to say the health *bar* remains
promoted-and-generic while Nova's *player* readout is diegetic and local.

## Open questions

- **Camera legibility.** How readable is on-ship tint at real combat framing
  (distance, angle, ship partly off-frame)? Resolve by playtest; the mini-ship
  (Option 3) is the ready fallback if it fails.
- **Tint mechanic for gltf sections.** Emissive/rim overlay vs material
  override - a `/plan` decision. Overlay is the honest v1 (does not stomp
  authored art). Whether a shared graded material or a per-frame colour write is
  a perf/simplicity call; per-frame write is fine at ship section counts.
- **Ramp shape.** Where the colour crosses amber->red, and whether it pulses
  under fire. Start linear on `current/max`, tune by feel.
- **Cinematic hide.** Should damage tint disappear at `HudVisibility::None`?
  World meshes ignore the UI tier system, so this needs an explicit toggle if
  wanted - likely not for v1.
- **Non-player ships.** Do enemy/allied ships get the same tint? It would help
  target assessment, but interacts with the 0-HP-ghost bug (20260716-162701);
  scope v1 to the player ship, revisit after that bug lands.

## Next steps

Direction-level tasks (for `/plan` to break into steps when picked up):

- tatr 20260717-003613: Diegetic HP v1 - per-section mesh damage tint/glow +
  retire the generic bar for the player ship (primary).
- tatr 20260717-003620: Numeric hull-integrity chip in the flight-status chip
  family (subordinate exact-value backstop).

## Fix record

- 20260717 tatr 20260717-003613 (Diegetic HP v1) landed: `sections::damage_tint`
  grades each player-ship section's mesh material by its `Health` (redden +
  darken + red glow, burnt when dead), and the generic `HealthDisplay` bar is no
  longer spawned for the player ship. KEY CORRECTION to this spike's framing: the
  shipped ship is all gltf `WorldAssetRoot` meshes, not cuboids, so the mechanism
  is per-section gltf material cloning (captured on `Added<MeshMaterial3d>`), not
  a cuboid colour swap. On-screen legibility at combat framing (this spike's main
  open question) still needs a human playtest. See tasks/20260717-003613/NOTES.md.
