# Spike: how do we make finite ammo a mechanic we actually want to turn on, and add reload cheaply?

- DATE: 20260717-000000
- STATUS: RECOMMENDED
- TAGS: spike, hud, ui, weapons, v0.7.0

## Question

The v0.7.0 plan (strand 3) carried an "ammo/magazine HUD readout" task that
asked for a HUD instrument showing loaded bullet type, rounds remaining, and
reload state for the turret and torpedo bay. Two of those three already ship as
a diegetic readout drawn on the weapon. The third, "reload state," has nothing
to show because **there is no reload mechanic** - and precisely because there is
no reload, almost every scenario runs `infinite_ammo: true`, which strips the
`SectionAmmo` component and hides the readout entirely. So the real uncertainty
is not "how do we draw an ammo chip" (largely done) but:

1. Why is finite ammo "too hard" to turn on today, and how do we make it a
   mechanic authors reach for instead of avoid?
2. What is the cheapest reload mechanic that (a) removes the permanent
   dead-weapon failure state and (b) gives the readout a "reload state" to
   display?
3. Is the existing UI actually enough, or does the task's "instrument-family
   chip" still need building?

A good answer picks one reload direction, says why it beats the alternatives,
and hands `/plan` a task list that is small and bounded - no re-litigating.

## Context

What exists today (all verified in code this spike, master @ a2e22be2):

- **Ammo is one scalar pool per weapon section.**
  `SectionAmmo { rounds, capacity }` lives on the turret / torpedo-bay entity
  (`crates/nova_gameplay/src/sections/ammo.rs`). `try_consume()` spends one
  round per shot and gates fire at zero; `capacity` is kept only as "what a
  reload would refill to." **Absence of the component = unlimited ammo.** There
  is no regen, no reload, no reserve - once `rounds` hits 0 the weapon is dead
  for the rest of the scenario.

- **Infinite ammo = no component.**
  `PlayerControllerConfig.infinite_ammo` (`crates/nova_scenario/src/objects/spaceship.rs:47`)
  forces `ammo_capacity = None` at spawn, so no `SectionAmmo` is attached.
  Player-scoped; enemies keep their magazines. The base scenarios that turn it
  ON: `shakedown_run`, `broadside`, the `example` mod. The only base scenario
  with finite player ammo is `asteroid_field` (`infinite_ammo: false`).

- **The loaded round type is a separate runtime slot.**
  `LoadedBullet { kind: DamageType, damage }` on the turret
  (`turret_section.rs:188`), seeded from config, read by the fire path and the
  readout. Torpedoes are always `Explosive`. Bullet-type *switching* is
  architected (mutate the component) but no input drives it yet.

- **The diegetic readout already shows loaded-type + count** (the user's "UI is
  solid"). `crates/nova_gameplay/src/hud/ammo_readout.rs`: a turret shows an
  8-pip ring that drains as rounds deplete; a torpedo bay shows a `||||` bar,
  one pip per round. Pips are colored by the loaded round's
  `damage_type_color()` (Kinetic amber / AP steel-blue / EMP cyan / Explosive
  red-orange) at lit alpha 0.95 / dim 0.16, with a dark outline for contrast.
  It is `HudTier::Instrument` (survives the Minimal tier, like the speed/mode
  chips) and anchored to the weapon via the `screen_indicator` widget. The
  exact number is a `debug`-feature-only overlay. **Crucially, the reconcile
  filter skips any weapon with no `SectionAmmo`, so on an `infinite_ammo`
  scenario the readout is invisible.**

- **No reload anywhere.** `grep` for reload turns up only doc comments pointing
  at the deferred task (`20260708-162005`, since CLOSED/superseded) and the
  ammo.rs note "a future reload is `rounds = capacity`." The growth seam is
  documented and tiny; nobody has built it.

The constraint this puts on the answer: the HUD half of the original task is
mostly done and the user likes it. The blocker is the *mechanic*. Any answer
that ships a nicer chip without addressing "finite ammo permanently kills your
weapon" leaves the readout hidden in every scenario that matters.

## Why finite ammo is "too hard" today

It is not implementation difficulty - the pool already works. It is a **design
and authoring** problem with two faces:

1. **Terminal failure.** With no replenishment, running dry is permanent. A
   turret magazine sized for a short fight leaves the player defenceless for the
   rest of a long one, with no counterplay. That is un-fun, so the safe authoring
   choice is `infinite_ammo: true`.
2. **Per-scenario tuning burden.** To use finite ammo *well* without a reload,
   an author must hand-size every magazine to every scenario's expected engagement
   length. Get it wrong and the player softlocks or the cap never bites. That is
   fragile and expensive, so nobody does it. The result: finite ammo is a mechanic
   the game technically has but effectively never uses, and the readout the team
   built rides along hidden.

The fix is a **replenishment mechanic**. Once ammo comes back, magazine size
stops being a softlock risk and becomes a *sustained-fire budget* / pacing knob -
forgiving to author, and finally worth turning on broadly.

## Options considered

Two axes: (A) how ammo replenishes, and (B) whether we add UI beyond what
exists. They are largely independent; the recommendation picks one from each.

### Axis A - the replenishment mechanic

- **A0. Do nothing (keep infinite ammo).** The honest baseline. Cost: the whole
  ammo/readout investment stays dormant; "reload state" is unbuildable; the game
  keeps a combat system it never uses. Rejected - it makes the original task moot.

- **A1. Passive trickle regen.** `SectionAmmo` grows a `regen_per_sec` (or
  `regen_interval`); a system ticks `rounds` back toward `capacity`, optionally
  only while not firing. Ammo becomes heat-like: a rate limiter you can't
  permanently exhaust.
  - Pros: zero new input, zero new UI mode, no softlock ever, self-balancing
    across any scenario length, and the existing ring/bar *already* animates the
    refill for free. Smallest possible change (one field + one system, the
    documented `rounds = capacity` seam generalized to `rounds += ...`).
  - Cons: no discrete "reload" *event*, so the task's literal "reload state" is a
    continuous recharge, not a moment. Less tactically punchy than a magazine.

- **A2. Discrete manual reload (press-to-reload).** Add a reload input +
  `ReloadState { timer }`; while reloading the weapon can't fire; on completion
  `rounds = capacity`. Classic FPS. Assumes an effectively infinite reserve
  (so it is never terminal).
  - Pros: matches "reload state" literally; tactical (commit to downtime);
    gives a real HUD state to show.
  - Cons: needs a **new input binding** in a mouse-flight game (design + keybind
    hint churn), a fire-lockout window, and player education. Heaviest option.

- **A3. Auto-reload on empty (timed, no input).** When `rounds` hits 0 (or after
  a cooldown since the last shot), start a `ReloadState` timer; on completion
  refill to `capacity`. A middle path: discrete reload *state* with **no new
  input** and no terminal failure.
  - Pros: gives the task's "reload state" a real, showable moment; never
    softlocks; no new binding; reads naturally as "weapon cycling / cooling."
    Still just a timer + refill on the existing seam.
  - Cons: less player *agency* than A2 (you don't choose when); a purely
    empty-triggered reload can feel like a stutter if magazines are tiny (tune
    with generous capacity + short reload, or trigger reload on a
    time-since-last-shot idle instead of strictly on empty).

- **A4. Pickups / resupply zones.** Ammo crates from kills, or a friendly
  station/dock that refills. Rejected as the *first* step: it is content and
  spawning work, not a cheap mechanic, and it still needs a fallback so a player
  who finds no pickup isn't stuck. A good *later* layer on top of A1/A3, not the
  keystone.

### Axis B - the UI

- **B0. Keep the diegetic readout, add nothing.** It already shows loaded-type +
  count and the user calls it solid. Its one genuine gap is the missing reload
  state (because the mechanic doesn't exist yet).
- **B1. Add a reload-state visual to the *existing* diegetic readout.** When a
  section is reloading/recharging, animate the ring/bar to say so: e.g. a
  clockwise "fill sweep" on the turret ring proportional to reload progress, or a
  pulsing/desaturated track, reusing the pip nodes and `damage_type_color` hue.
  No new widget, no corner panel.
- **B2. Build the separate instrument-family corner chip the original task
  described** (text: type + `rounds/capacity` + RELOADING), in addition to the
  diegetic readout. Rejected: it duplicates information the diegetic readout
  already conveys, the user explicitly likes the current UI, and the debug
  numeric overlay already covers "exact count" for when you need it. Two readouts
  for one fact is clutter, not polish.

## Recommendation

**A3 (auto-reload on empty/idle, timer-driven, no new input) + A1 as the
generalized primitive, paired with B1 (reload-state on the existing readout).
Do not build B2.**

Concretely:

1. **Make replenishment the primitive.** Grow `SectionAmmo` (or a sibling
   `SectionReload` component) with a reload/regen descriptor and a small system
   that refills. Model it so both behaviors fall out of one mechanism:
   - a `reload_time` + auto-trigger (on empty, or after N seconds idle since the
     last shot) gives the **discrete A3** feel and a clean `Reloading` state;
   - degenerate settings (continuous, per-round) give the **A1 trickle** if a
     weapon wants heat-like behavior.
   Refill is the documented seam (`rounds = capacity` on completion, or
   `rounds += k` per tick). Keep "no component = unlimited" intact so infinite
   ammo and every headless test are unchanged.

2. **Author it to be forgiving.** Default catalog weapons to generous capacity +
   short auto-reload so running dry is a brief cadence beat, never a death. This
   is what makes finite ammo *safe to turn on* - magazine size becomes a
   fire-pacing knob, not a softlock. Then flip the base combat scenarios
   (`shakedown_run` first) from `infinite_ammo: true` to finite + auto-reload and
   playtest the feel. Keep `infinite_ammo` as an authoring escape hatch (tutorials,
   sandboxes), but it stops being the default crutch.

3. **Surface the reload state on the readout we already have (B1).** Add a
   `Reloading` visual to `drive_ammo_readouts`: a fill-sweep / pulse over the
   existing ring (turret) and bar (torpedo) in the loaded round's hue, driven by
   the reload timer's progress. This is the single genuinely-missing piece of the
   original task, and it lands as a local change to one system, not a new widget.

Why this beats the runners-up: A3+B1 delivers exactly the original task's three
signals (type, count, reload state) while removing the reason finite ammo is
avoided, for the least new surface area - **no new input binding (vs A2), no
second HUD widget (vs B2), no content pipeline (vs A4), and no dormant system
(vs A0).** It reuses the documented refill seam and the existing readout
wholesale. A2 (manual reload) and A4 (pickups) are real, better-feeling
mechanics we may want later; they are additive layers on top of this primitive,
not prerequisites, so they stay backlog.

## Open questions

- **Auto-reload trigger: on-empty vs idle-timeout?** On-empty is simplest but can
  stutter with small mags; an "idle for N seconds since last shot, top up" trigger
  feels more like a recharge and never interrupts a burst. Resolve by feel during
  the mechanic task's playtest - it is a one-line difference in the trigger
  condition.
- **Per-weapon vs global reload tuning?** Torpedoes (6 rounds, slow) and turrets
  (150-500 rounds, fast) want very different reload cadences. Likely a per-config
  field, defaulted per catalog weapon. Confirm when sizing the defaults.
- **Does the readout need to appear at all for `infinite_ammo` weapons?** Today it
  is hidden, which is intended. Once finite+reload is the default this is moot for
  combat scenarios; leave the "no component = no readout" behavior as-is.
- **Reload SFX / muzzle cue?** Out of scope for the spike; a nice-to-have the audio
  pass can pick up once the state exists.

## Next steps

Direction-level tasks this spike seeds, for `/plan` to break into steps:

- tatr 20260717-085640 (new): **Weapon reload/regen mechanic** - grow
  `SectionAmmo` with an auto-reload/regen descriptor + refill system (A3 primitive,
  A1 as a degenerate setting), keep "no component = unlimited," default catalog
  weapons to generous-capacity + short-auto-reload, and flip `shakedown_run` from
  `infinite_ammo` to finite+auto-reload as the proof. This is the keystone that
  unblocks everything else. Priority just above the readout task (must land first).
- tatr 20260716-123556 (this task, reframed): **Reload-state on the diegetic ammo
  readout** - add the `Reloading` fill-sweep/pulse visual to `drive_ammo_readouts`
  (B1). Do NOT build a separate corner chip (B2 rejected); loaded-type + count are
  already shipped. Depends on the reload mechanic above for a state to show.

Explicitly NOT seeded (stay backlog, layer on later): manual press-to-reload with
its own input binding (A2); ammo pickups / station resupply (A4); bullet-type
switching input (the `LoadedBullet` swap seam, already architected).

## Fix record

(Appended by each implementing task as it lands.)
