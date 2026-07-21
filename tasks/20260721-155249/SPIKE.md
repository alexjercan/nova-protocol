# Spike: base campaign arc - what shape does the Shakedown-to-Broadside extension take?

- DATE: 20260721-155249
- STATUS: RECOMMENDED
- TAGS: spike, scenario, content, campaign, v0.8.0

## Question

Task 20260718-152313 wants the base campaign (Shakedown Run -> Broadside ->
Broadside: Rust Tally) longer and more interesting - more beats/acts, better
pacing, more encounter variety - with data/scenario work only (v0.8.0
no-new-features rule). The open design question this spike answers: WHERE do
the new acts go, WHAT encounter shapes fill them, and WHAT story carries them,
given exactly the v0.7.0 authoring toolbox?

A good answer is a beat sheet the owner can nod at (the task's own step 2
requires that nod before content is built), with the alternatives weighed and
the technical risks named.

## Context

Everything below verified in source this session (worktree at master
fa93d43f).

### The current chain (~7-12 min total)

1. **shakedown_run** (tutorial, ~3-5 min, picker-visible, New Game start):
   12 beats, one gesture each (beat-sheet v2, spike 20260713-140742). Ends in
   a 1v1 scavenger duel; victory hook: "it was flying scout. A distress call
   is already crackling from the deep field." Chains (linger) -> broadside.
   Builder: crates/nova_assets/src/scenario/shakedown.rs.
2. **broadside** (~2-3 min, picker-visible): coast to the neutral hauler
   Ceres Queen, two-corvette ambush among 24 destructible rocks + 5
   invulnerable boulders (hard cover). Hauler death is soft-fail flavor
   ("Make it cost them."), not defeat. Victory = checkpoint, chains ->
   broadside_gunship. Builder: crates/nova_assets/src/scenario/broadside.rs:289-440.
3. **broadside_gunship** (~2-4 min, hidden continuation): same arena, the
   cargoB gunship Rust Tally (2 full turrets, 2 torpedo tubes, 70 HP hull
   sections) burns in from 720u; PDC-screen its torpedoes, break it section
   by section. Victory says "The gang is done picking this belt clean." and
   DEAD-ENDS - no NextScenario, only Main Menu
   (broadside.rs:446-540). Defeat retries the gunship only (the checkpoint
   contract).

Side content, not in the chain: asteroid_field/asteroid_next sandbox loop
(NOTE: currently `hidden: true` in the generated RON while the player wiki
scenarios.md still lists it as a picker sandbox - stale doc or unintended
regression, flagged below) and three menu backdrops.

### Toolbox facts that shape the design (all verified, file:line)

- **Ally-side ships are engine-supported but never shipped.** AI target
  acquisition is relation-driven, not player-hardwired: every AI ship picks
  the nearest entity whose relation is Hostile
  (nova_gameplay/src/input/ai.rs:242-289, `update_ai_target`), and
  Player<->Enemy is Hostile both ways (relations.rs:54-62). The authored
  `allegiance:` override is a plain insert that overwrites the controller
  default and is documented ordering-safe (nova_scenario/src/actions.rs:2527-2536)
  and accepts any variant, so `allegiance: Some(Player)` on an AI-controlled
  hauler makes enemies genuinely target it - a defendable convoy, data-only.
  Only `Some(Neutral)` has shipped (broadside hauler), so this needs a
  harness rig FIRST (risk section).
- **The base campaign has ZERO StoryMessage comms.** All narrative rides
  objective text and outcome banners; the only voiced speakers in shipped
  content are the Ledger mod's Foreman Okono and Broker Vesh (grep of
  assets/base/scenarios/*.content.ron). The comms panel, queueing, dwell -
  the whole v0.7.0 voice surface - is unused by the base game. Cheapest
  narrative lever available.
- **Encounter palette**: 3 hulls (racer, cargoA hauler, cargoB capital), two
  grades (player-grade vs scavenger-nerfed via SetHealth + fire-rate,
  craft.rs:145-159), light/full turrets, cargoB torpedo tubes. AI: patrol
  routes, orbit-a-well directive, leash, engage_delay arrival grace
  (spaceship.rs:91-140). No escort/follow, no flee, no formations.
- **Environment**: asteroids with `surface_gravity` = real gravity wells
  (orbitable, OnOrbit event); `invulnerable: true` rocks = hard cover and
  wreck set-dressing (the Ledger's Ceres Matron trick); ScatterObjects with
  Box or RING regions (ring never used in base combat); beacons with trigger
  areas; salvage crates. No nebula/fog.
- **Pacing/structure**: scenario_elapsed clock + one-shot gates, HudReadout
  (Number/Integer/Time formats - a countdown is just an OnUpdate-recomputed
  variable; only setting `scenario_elapsed` itself is linted), Outcome with
  auto_advance, NextScenario linger/delay, checkpoints as chained hidden
  scenarios. Variables are scenario-scoped: NO state crosses a chain
  boundary, each act restates its world.
- **Balance lint floor**: opening hostile inside its own effective range of
  the player spawn = ERROR (never ackable); triggered spawn inside envelope =
  WARN, ackable with reason in crates/nova_assets/balance_acks.ron. Precedent
  ack: the Ledger ch4 Auditor, "shooting the incoming torpedo down IS the
  fight."
- **Beat-sheet convention is enforced**: announce -> breathe -> arrive ->
  fight -> confirm -> breathe -> next; one story line per beat; every fight
  telegraphed (warning line + far spawn + engage_delay); no StoryMessage
  beside an Outcome (web/src/wiki/dev/guide-author-scenario.md, "The beat
  sheet"; lint arms from task 20260717-163058).

## Options considered

Arc-level, where the new content goes:

- **A. Append Chapter 3 after the gunship (recommended).** Two new chained
  scenarios continuing the gang arc; the gunship victory dead-end becomes the
  hook. Pros: existing, playtested content is touched only lightly (one
  victory handler + a polish pass); difficulty stays monotonic; the
  checkpoint pattern (visible chapter head + hidden continuation) is proven
  by Broadside. Cons: none structural; chapter 3 must avoid re-treading
  chapter 2's shapes (solved by encounter choice below).
- **B. Insert a chapter between Shakedown and Broadside.** Smooths the
  1v1-to-2v1 difficulty step and builds the hauler relationship before her
  distress call. Rejected as the primary: it rewrites the existing
  shakedown->broadside transition (the hook line and premise directly
  connect), renumbers "chapter two" fiction, and delays the campaign's best
  content; the difficulty step it smooths was already addressed by the
  difficulty rework + arrival graces.
- **C. Both (A + B).** Longest campaign, most new content. Rejected for this
  cycle: the binding constraint is owner playtest/tuning bandwidth per
  scenario, not authoring speed; ship A, keep B in the pocket if playtest
  says the front half is thin.
- **D. Extend inside the existing files (more acts per scenario).** Rejected:
  violates the checkpoint contract (death must never replay more than one
  fight), which is exactly why Broadside was split into two files in the
  first place.
- **E. Do nothing.** Rejected by the task and the release DoD ("both
  campaigns meaningfully longer").

Chapter-3-part-1 encounter type (the variety slot, the one real choice):

- **(i) Convoy defense (recommended).** Two Player-allegiance AI haulers
  crawl a freight lane; raider waves try to kill them; countdown HudReadout
  to relief. Genuinely NEW shape on every axis (objective: protect, not
  kill-all; comp: light waves; pacing: clock pressure) and it exercises the
  relation model discovery. Risk: ally allegiance unshipped - carries the
  rig-first requirement and a designed fallback.
- **(ii) Salvage-under-fire.** Loot the Rust Tally's wreck field (salvage
  verb from the tutorial) while stragglers hunt the player. Safe (no ally
  dependency), good fantasy, but pressure is all on the player, so it is
  closer to "fight near crates" than a new shape. Kept as the DESIGNED
  FALLBACK for (i): same arena, same wave schedule, objective wiring swaps
  from protect-haulers to recover-pods.
- **(iii) Pursuit (catch the fleeing scout).** No flee AI exists; faking it
  with a patrol route reading away is brittle and untunable. Rejected.

## Recommendation

Option A with encounter (i): append **Chapter 3** as two chained scenarios,
plus a voice/polish pass on the existing chain. Working chain:

    shakedown_run -> broadside -> broadside_gunship -> lifeline -> final_tally

(ids/names are proposals; nod items below.) Extended play time ~13-20 min.

### Story spine

The Rust Tally was the gang's muscle, not its head. Breaking it provokes the
gang: they hit the belt's supply convoy in revenge (ch3a), and the burst
transmission you intercept there points at their claim - a wrecked megahauler
anchorage in a planetoid's gravity well - where their flagship, the Final
Tally, is berthed (ch3b). Proposed cast (base campaign's first voiced
speakers): Captain Halloran of the Ceres Queen (the hauler you saved; her
guild runs the convoy), the Tallyman (gang boss, taunts), Belt Relay
(dispatch connective tissue).

### Polish pass on the existing chain

1. **Voice layer**: give broadside/gunship a StoryMessage comms track (the
   distress call, the ambush spring, first-corvette-down, the capital-burn
   warning, the victory hooks move from objective text to voiced lines;
   objectives shrink to imperative goals). Shakedown stays text-taught - the
   tutorial's objective text IS the teacher; at most its closing hook becomes
   a comms line.
2. **Un-dead-end the gunship**: victory line rewritten to hook chapter 3
   ("...but the claim is still out there") + lingering NextScenario ->
   lifeline.
3. **Conditional flavor**: broadside/gunship victory lines vary on whether
   the Ceres Queen survived (scenario-local variable, two gated handlers) -
   protecting her finally gets acknowledged.
4. **Wiki fix**: resolve the asteroid_field hidden-vs-wiki contradiction
   (owner call: unhide or fix scenarios.md).

### ch3a "Lifeline" (picker-visible chapter head, checkpoint)

Arena: open freight lane between two beacons, light chaff + 3-4 invulnerable
boulders staggered along it (cover exists but does not enclose - a lane, not
the Broadside bowl). Two cargoA haulers, `allegiance: Some(Player)`,
controller AI with patrol waypoints crawling the lane. Player spawns trailing
the convoy.

- Announce: Belt Relay + Halloran lines; objective "Screen the convoy";
  HudReadout countdown "RELIEF mm:ss" (~4 min; OnUpdate recomputes
  relief_remaining = T - scenario_elapsed).
- Waves (scenario_elapsed + wave-cleared gates, each telegraphed: warning
  line, far spawn outside weapon envelope, engage_delay): W1 two light
  racers, one vector. Breathe. W2 three light, split vectors (one flanker
  forces the player to choose a side). Breathe + Tallyman taunt. W3 one
  full-turret corvette + one light (first mixed pair).
- Win: relief timer expires with at least one hauler alive (clearing W3
  early also ends it): Victory checkpoint, intercepted-transmission hook,
  chain (linger) -> final_tally.
- Lose: player death, or both haulers destroyed (distinct defeat messages);
  retry lifeline only.
- Balance: wave spawns authored outside their own envelopes so the lint
  floor stays clean by construction; if playtest wants a closer W3 drop,
  that is an ack with a reason (Auditor precedent).

**The single reversal switch**: if the ally rig (below) fails, Lifeline
becomes (ii) - same lane, same wave schedule and telegraphs, haulers turn
Neutral wrecks-to-be, objective wiring swaps to recovering their jettisoned
cargo pods (salvage crates) between waves; lose = player death only. One
scenario's wiring changes; the chapter, chain, and beat rhythm survive a
playtest reversal untouched.

### ch3b "Final Tally" (hidden continuation, finale)

Arena: the base chain's first combat gravity well - a planetoid with
`surface_gravity`, a Ring-region asteroid belt scattered around it, and two
big invulnerable wreck-rocks (the broken megahauler) as the anchorage.

- Arrive: coast into the SOI (a deliberate callback to the tutorial's
  gravity-coast beat), announce; survey beat - OnTravelLock the anchorage
  wreck to confirm the claim (lock verb reused narratively).
- Fight 1, the picket: two light racers holding ORBIT directives around the
  well (first combat use of the orbit directive - guards on rails, a shape
  no earlier fight has). Breathe + Tallyman line.
- Fight 2, the flagship: the Final Tally (cargoB, full grade, 2 turrets + 2
  tubes) casts off from the anchorage with one corvette screening it - the
  campaign's only simultaneous capital + escort fight, in belt cover, inside
  a well.
- Confirm -> breathe -> end: flagship down, two closing comms lines
  clock-gated AFTER the kill, then the Victory overlay (campaign-complete
  message; no NextScenario - the chain ends here properly instead of by
  omission).
- Lose: player death; retry final_tally only.

### DoD check (task 20260718-152313)

Chain grows by two scenarios (floor was one). Consecutive shapes: 1v1 duel ->
pair ambush in cover -> solo capital torpedo screen -> light-wave convoy
defense on a clock -> orbital picket then capital+escort in a well; no two
consecutive fights share composition or shape. Every scenario keeps win+lose
Outcomes, one-fight checkpoints, beat-sheet comms; thumbnails follow the
shipped `self://banner.png` placeholder pattern (real art stays backlog task
20260715-220011); picker wiring follows the Broadside precedent (chapter head
visible, continuation hidden); scenarios.md + CHANGELOG sync in-task.

## Open questions

- **Names/tone (owner nod)**: gang formalized as "the Tally" (makes Rust
  Tally read as a fleet name; finale flagship "Final Tally" lands the pun),
  Captain Halloran, the Tallyman, Belt Relay, chapter title "Lifeline". All
  placeholders until the nod; they gate only text, not structure.
- **Ally rig outcome**: primary vs fallback for Lifeline is decided by the
  first /work step, not by this doc; the reversal is contained by design.
- **Picker policy**: confirm chapter heads stay picker-visible (Broadside
  precedent) - and whether asteroid_field's `hidden: true` is intended.
- **Tuning defaults**: relief timer length, wave sizes, flagship escort
  count are first-pass numbers for playtest; every fight must stay winnable
  AND losable (release DoD).
- **Out of scope, noted for later**: music/ambience beds (no system; old
  spike note stands), cross-scenario state (engine feature, v0.9.0+
  territory), The Ledger extension (own task 20260718-152320 - it should
  reuse the ally-rig finding if it lands).

## Next steps

No new implementation tasks seeded: the direction task already exists and
carries this work.

- tatr 20260718-152313 (v0.8.0, p50): base campaign polish + extension -
  Notes now link this doc; its step-2 beat sheet IS this doc's
  Recommendation, pending the owner's nod. /plan expands it when picked up;
  whether polish/ch3a/ch3b split into sub-tasks is /plan's call.
- The ally-allegiance harness rig is that task's FIRST implementation step
  (fail-fast on the one unshipped mechanism this design leans on).
