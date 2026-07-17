# Spike: why are the second scenarios brutally hard, and how do we rework scenarios to be fair without dumbing down AI or damage

- DATE: 20260717-111808
- STATUS: RECOMMENDED
- TAGS: spike, scenario, balance, ai, content

## Question

The developer (a far-above-average player) reports: the ledger's second
scenario (`ledger_ch2_claim_jumpers`) "shreds you instantly"; the base
campaign's second scenario (`broadside`) is hard but playable. If the
developer struggles, the average player's experience is likely "impossible".
Why exactly are these scenarios so hard, and what scenario-level reworks make
them meaningfully easier while still requiring skill?

Hard constraints from the user:

- Do NOT make the AI dumber (aim, lead prediction, decision-making stay).
- Do NOT reduce damage dealt to the player (a 10-20 bullet burst shredding
  you is intended lethality).
- The hard controls are intended and stay.
- Cover, checkpoints, pacing and encounter design are the intended levers.

A good answer names the concrete mechanical causes with numbers, and seeds
tasks whose fixes stay inside those constraints.

## Context

Campaign order: `shakedown_run` (tutorial) -> `broadside`
(assets/base/scenarios/, chained via NextScenario at
shakedown_run.content.ron:1227). Ledger order: `ledger_ch1_dead_weight`
(no combat) -> `ledger_ch2_claim_jumpers` (webmods/the-ledger/, chained at
ledger_ch1.content.ron:431). Death in every shipped scenario is
`Outcome(Defeat)` + `NextScenario(<same scenario>)`: a full restart, no
checkpoints anywhere.

All numbers below were verified against source, not just subagent reports
(files cited inline). The two key mechanical facts that frame everything:

1. **Bullets DO stop on tangible cover.** `despawn_bullet_on_hit`
   (crates/nova_gameplay/src/sections/turret_section.rs:431) expends a round
   on its first contact with any non-Sensor collider, including no-Health
   invulnerable bodies (the bullet carries `CollisionEventsEnabled`
   specifically so planetoid cover works; see the spawn-bundle comment at
   turret_section.rs:1152, review R1.2 MAJOR). Physical cover is real.
2. **The AI has no line-of-sight concept.** Target acquisition, aim and the
   fire gate never raycast (crates/nova_gameplay/src/input/ai.rs: acquisition
   at 2000u, engage at 800u, standoff orbit at 250u, fires when barrel
   alignment > 0.95 and range < 0.9 x bullet range). An occluded enemy keeps
   tracking you and keeps firing into the rock, and its orbit motion
   (8 u/s at ~250u standoff) slowly regains the angle. Cover absorbs damage
   but never relieves targeting pressure.

## Findings: the difficulty, ranked by contribution

### F1. better_turret in AI hands is the shredder

The turret catalog (assets/base/sections/base.content.ron) has two guns:

| Turret | fire_rate (rds/s) | damage | dps | muzzle (u/s) | effective range (0.9 x speed x 5s life) |
|---|---|---|---|---|---|
| light_turret_section | 25 | 3.825 | ~96 | 60 | 270u |
| better_turret_section | 100 | 4.0 | 400 | 100 | 450u |

fire_rate is rounds per SECOND (turret_section.rs:89, interval = 1/fire_rate
at :484). AI turrets get perfect lead prediction (target velocity fed to the
lead-intercept solve, ai.rs:1321) and zero spread (the 0.2 spread in the
muzzle shader is visual only, turret_section.rs:1410). AI fire cadence is a
1.5s burst / 0.8s hold cycle (ai.rs:1032).

The player ship in these scenarios is ~500 HP total across four sections
(reinforced_hull 200, basic_controller 100, better_turret 130,
basic_thruster 70; health is per section, ship dies when all sections die,
crates/nova_gameplay/src/integrity/glue.rs:130). One aligned better_turret
strips the whole ship in ~1.25s; the 70 HP thruster dies to ~0.18s of fire,
and losing thruster or controller is a death spiral well before actual death.

- broadside act 1: two corvettes with LIGHT turrets (~190 dps combined,
  60 u/s bullets = long flight time = dodgeable). Hard but playable.
- ledger_ch2 act 1: two magpies with BETTER turrets (800 dps combined,
  100 u/s bullets). Act 2: two more better turrets. That single loadout
  choice is most of the difference between "playable" and "shredded".

### F2. Spawn geometry: ledger_ch2 starts inside kill range

- broadside: corvettes spawn ~550u out (beyond their own 270u effective
  range), gunship at ~1177u. The approach is the breathing room.
- ledger_ch2: magpies spawn at ~175u on OnStart
  (ledger_ch2.content.ron:135-208), already inside their 450u effective
  range; the act-2 pair spawns at ~130u (ledger_ch2.content.ron:278) the
  frame the kill counter flips. There is never an approach phase, and wave 2
  arrives mid-dogfight on top of the player.

### F3. Crossfire defeats the only dodge that exists

Perfect lead is exact only for constant velocity: the skill move is
constantly changing acceleration. That works against ONE stream. Against 2-4
shooters on different bearings (all AI() with no patrol/leash in ledger_ch2,
so all converge to a 250u orbit around the player), dodging one stream
aligns you with another. Simultaneous shooter count is the real difficulty
dial, and it jumps 0 (ch1, no combat at all) -> 2 -> 4-ish with no stagger.
broadside at least leashes its corvettes (420u, broadside.content.ron:304).

### F4. Cover is absent, or paper

- ledger_ch2 authors ZERO cover objects. Open void.
- broadside's 24 ring rocks are destructible at health 100
  (broadside.content.ron:197-240): one better turret eats a rock in 0.25s.
- Invulnerable cover is already supported and shipped:
  `invulnerable: true` on AsteroidConfig
  (crates/nova_scenario/src/objects/asteroid.rs:45, unit-tested to collide
  with no Health node; used by shakedown's planetoid, asteroid_field:33 and
  ledger_ch4:100). No second-scenario fight space uses it.
- Even where cover blocks bullets, mechanical fact 2 from Context applies:
  no AI line of sight means no pressure relief, and the AI wastes no time
  re-angling (orbit is built in).

### F5. Wave chaining with zero breathing room

`OnUpdate` triggers fire the frame their variable gate flips; both second
scenarios spawn the next wave the instant the previous one dies. The engine
has NO timer primitive (no delay action, no elapsed-time event; pacing can
only come from OnEnter proximity gates or player actions -
crates/nova_scenario/src/events.rs:13-30, actions.rs:28-55). Authors
could not write "30s breather, then reinforcements" even if they wanted to.

### F6. The escort dies to the player's own dodging

AI only targets Hostile relations, and only Player<->Enemy is Hostile
(crates/nova_gameplay/src/relations.rs:50): enemies NEVER deliberately shoot
the neutral hauler. The Dray Mule dies to stray rounds aimed at the player
that miss and expend on it (bullets stop on the first tangible thing). In
ledger_ch2 the hauler sits at (0,-5,-60) between the player spawn (0,0,40)
and both magpie bearings (+/-145, ~-155): successful dodges funnel fire into
the loss condition. The objective text ("keep them off the Dray Mule")
suggests staying close, which is exactly wrong; kiting fire AWAY is the
correct play and nothing teaches that.

### F7. Death restarts the whole multi-act scenario

Sub-second TTK plus full-scenario retry means re-earning act 1 on every act 2
death. Scenario variables and state are wiped on reload
(crates/nova_scenario/src/world.rs:173), there is no checkpoint, respawn,
teleport or cross-restart persistence anywhere in the engine. The retry loop,
not the TTK, is what makes the lethality feel unfair.

## Options considered

- **A. Content-only rework of the second scenarios** (loadout discipline,
  spawn ranges, invulnerable cover, leash/patrol stagger, escort geometry,
  act-splitting for checkpoints). Pros: every lever verified available in
  shipped RON today; fixes the two reported scenarios immediately; zero
  engine risk. Cons: cover still cannot relieve targeting pressure (F-two);
  pacing is limited to proximity/action gates (no timers); the same traps
  stay open for future content and modders.
- **B. Engine: AI line-of-sight fire gate + reposition.** Hold fire while a
  tangible blocker occludes the aim point; keep tracking and use the existing
  approach/orbit machinery to regain the angle. Pros: makes cover a real
  pressure-relief mechanic; NOT a dumber AI (it stops wasting ammo into
  rocks and visibly maneuvers for the shot, which reads smarter); benefits
  every current and future scenario. Cons: touches the hottest AI code path;
  needs care so PD (torpedo defense) is exempted or handled; needs a
  perf-conscious raycast (SpatialQuery per firing turret).
- **C. Engine: scenario timer primitive** (an elapsed-time event or a
  delayed action, e.g. OnTimer/`after_seconds` gates). Pros: unlocks authored
  breathers, staged reinforcements, timed setpieces for base game and mods;
  small, well-bounded engine surface. Cons: alone it fixes nothing; it is an
  enabler for content passes.
- **D. Engine: true mid-scenario checkpoints** (snapshot variables + player
  state, respawn at marker). Pros: the "real" checkpoint feature. Cons: the
  engine has no persistence, no teleport, no state snapshot; this is a large
  feature. Act-splitting (in A) gets ~90% of the value with shipped
  mechanics: each act becomes its own hidden scenario chained via
  NextScenario, so defeat retries only the current act (hidden scenarios and
  same-id retry chains are both shipped patterns already).
- **E. Rejected: AI/damage nerfs.** Attacker-slot caps, accuracy/spread
  knobs, damage-to-player multipliers, enemy weapon stat nerfs. All violate
  the spike's constraints (dumber AI or reduced lethality). Note the
  distinction we DO allow: swapping which catalog turret an enemy carries is
  encounter design, canon since broadside's corvettes carry light turrets;
  the weapons themselves keep their stats.
- **F. Do nothing / tutorialize harder.** More story text does not fix an
  800 dps crossfire at 175u with no cover. Rejected.

## Recommendation

Two tracks, engine and content, converging on a second content pass:

1. **Engine, highest priority: B (AI LOS fire gate + reposition).** This is
   the single change that turns "more cover" from damage-absorption into an
   actual play pattern (duck behind rock -> pressure stops -> reposition or
   peek). Without it, cover-based scenario design has a low ceiling.
2. **Content: rework ledger_ch2 against today's engine** (do not block on
   B): light turrets on act-1 magpies, better turret reserved for at most one
   act-2 ship; spawns pushed to 500-800u so an approach phase exists; an
   invulnerable rock field placed between the spawn bearings and the Dray
   Mule; patrols + leashes to stagger aggro instead of instant 4-ship
   convergence; hauler moved off the crossfire axis; act-split so defeat
   retries the current act only.
3. **Content: rework broadside pacing**: act-split (corvette act / gunship
   act) so gunship deaths do not re-earn the corvette fight; harden part of
   the cover ring to invulnerable; keep the gunship spawn distance (its
   1177u approach is good design worth keeping as the pattern).
4. **Engine: C (timer primitive)**, then a second pacing pass over both
   scenarios (delayed reinforcements, an authored breather between waves)
   plus adoption in future content.
5. **Tooling: a scenario balance audit rig** that computes, from shipped
   RON, per-scenario exposure metrics: combined enemy dps at spawn, spawn
   distance vs effective weapon range, simultaneous-shooter count over the
   script, cover count/hardness between spawn bearings and the fight space,
   and TTK-vs-player-ship. This encodes the fairness invariants as computed
   assertions (repo lesson `authored-vs-derived-values`), makes balance
   reviewable in CI, and calibrates "developer skill >> average player" with
   numbers instead of feel. Run it over ch3/ch4 and the rest of the campaign,
   which nobody has balance-checked yet.

Success criteria for the content tasks: the developer clears each reworked
scenario within a few attempts, the dominant failure reads as "my mistake"
not "spawned dead"; every fight offers at least one reachable hard-cover
position per attack bearing; wave 2 never spawns inside 400u of the player.

## Open questions

- LOS gate semantics: fire-gate only (recommended), or also drop the TARGET
  after N seconds fully occluded? Target-drop risks reading as dumber AI;
  the fire-gate alone satisfies the constraints. Decide in the task.
- Should point-defense fire (anti-torpedo) ignore the LOS gate? Recommend
  yes (torpedoes are the occluder-hugging case and PD is defensive).
- Act-split retries refill health/ammo by construction (fresh scenario).
  Accepted as deliberate easing, or does attrition across acts matter to the
  design? Recommend accept; revisit only if acts become trivially short.
- ledger ch3/ch4 and asteroid_field difficulty are unmeasured (user never got
  past ch2). The audit rig answers this without playtests.
- Whether enemy ships should ALSO benefit from cover against the player is
  free with B (symmetric mechanic) but changes player-side TTK on entrenched
  enemies; watch it in the content pass.

## Next steps

Direction-level tasks this spike seeded, for /plan to break into steps:

- tatr 20260717-112622: AI line-of-sight fire gate: hold fire and reposition
  when cover occludes the target (engine, addresses F4 via the LOS half)
- tatr 20260717-112630: Rework ledger_ch2 encounter design: loadouts, spawn
  ranges, real cover, aggro stagger, act-split retry (content, F1/F2/F3/F4/F6/F7)
- tatr 20260717-112639: Rework broadside pacing: act-split retry and hardened
  cover ring (content, F4/F7)
- tatr 20260717-112647: Scenario timer primitive: elapsed-time events /
  delayed actions for authored pacing (engine, F5)
- tatr 20260717-112656: Scenario balance audit rig: computed exposure/TTK
  metrics over shipped scenario RON (tooling, guards all of the above)

## Fix record

(Each implementing task appends a few lines here as it lands: what shipped,
the headline number, a pointer to its TASK.md.)
