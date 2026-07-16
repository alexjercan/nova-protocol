# Spike: What story does the campaign mod tell, in how many chapters, and which vocabulary gaps block it?

- DATE: 20260716-183104
- STATUS: RECOMMENDED
- TAGS: spike, story, scenario, modding, v0.7.0

## Question

Task 20260716-123535 wants an alt storyline shipped as a portal mod:
multiple chained scenarios with a real narrative arc. Undefined: the
STORY itself (premise, cast, escalation, ending), the chapter count and
per-chapter mechanics, and whether the scenario vocabulary can carry a
story at all - specifically cross-chapter state and story-text
presentation. A good answer names a story a planner can lower into
chapters mechanically, maps every beat to a PROVEN primitive (or names
the gap), and decides fill-vs-work-around for each gap.

## Context (verified in code, 2026-07-16)

- Variables do NOT survive chapter changes: `teardown_scenario_entities`
  calls `world.clear()` on every unload/load
  (crates/nova_scenario/src/loader.rs:489-507), and NextScenario goes
  through it. Cross-chapter carried state has NO carrier today.
- Branching WITHIN a chapter is fully supported today: expression
  filters read variables during the scenario, `NextScenario` is an
  action like any other, and the Outcome overlay rides a queued
  lingering NextScenario (the broadside/outcome-frame work). So "which
  chapter comes next" and "which ending fires" can branch on in-chapter
  state; only carrying NUMBERS across the boundary cannot.
- Story text today = `Objective`/`ObjectiveComplete` (HUD objective
  lines), `HintEmphasis*`, ship/entity NAMES, and beacon labels.
  `DebugMessage` is log-only. There is no comms/dialog surface (no
  speaker-attributed text). The full action list is
  EventActionConfig (crates/nova_scenario/src/actions.rs:28-52), incl.
  SetSkybox, SetSpeedCap, SetControllerVerb, Despawn, CreateScenarioArea,
  areas with OnEnter/OnExit, OnOrbit, OnDestroyed, OnUpdate.
- Proven story-bearing patterns to steal: gauntlet (gate sequencing via
  expression-guarded OnEnter), demo arena (per-id OnDestroyed kill
  counting + one-shot OnUpdate win), broadside (acts, allegiances,
  neutral ships, defeat-if-escort-dies, outcome chaining), salvage
  crates (on-rails pickups with OnEnter collection).
- Mod constraints: portal mod ids are lowercase/digits/dash (the
  "fixture-slalom" lesson); base assets only until the asset variety
  pack (20260716-123544) lands mod-resource support - three cubemaps
  exist (cubemap, cubemap_alt, cubemap_alt2) for per-chapter skies;
  ships build from base section prototypes (ship-prototype content kind
  20260714-134115 is not needed for v1 - chapters can inline ship
  configs, sharing via authoring discipline).
- The publish pipeline is proven end to end and generically gated
  (webmods_validation, portal generator, install-over-the-wire tests).

## Options considered

STORY (the task named three candidates):

- **Salvage crew in over its head** - fits the verb set exactly
  (salvage crates ARE a core primitive; wrecks, quotas, ambushes all
  proven), and its working-class fantasy contrasts the base storyline's
  military arc - a real ALT identity. Chosen.
- **Convoy escort gone wrong** - broadside already owns escort-defense
  as base content; the alt campaign would feel like more of the same.
  Rejected for identity overlap.
- **Hunt across the belt** - wants detection/stealth vocabulary
  (pursuit, evasion, sensor play) that does not exist; every beat would
  fight the engine. Rejected on vocabulary cost.

CROSS-CHAPTER STATE:

- **Add a carry mechanism** (e.g. `NextScenario { carry: [keys] }` or a
  CampaignState resource surviving teardown) - real modding surface,
  but nothing in the chosen story NEEDS it if endings branch inside the
  finale. Deferred (backlog note, not seeded); adding state before a
  story needs it is speculative engine work.
- **Design the story to not need it** - linear chapter chain, arc
  carried by each chapter's opening objective recap, endings decided by
  an in-finale choice. Chosen: zero engine risk, authorable today.

STORY TEXT:

- **Objectives-only (do nothing)** - workable (broadside shipped this
  way) but a story campaign lives on voice; one-line objectives cannot
  carry cast or tone. Rejected as the shipped form.
- **A comms/story-beat action** (`StoryMessage { speaker, text }` -> a
  small HUD comms panel with a short queue; scenario-scoped, cleared on
  teardown like emphasis) - modest engine work, exactly the "gaps found
  are v0.7.0 modding-surface work" the umbrella task predicted. Chosen
  as a SEEDED PREREQUISITE the campaign consumes (chapters degrade to
  objectives if it slips).

## Recommendation: "The Ledger" - four chapters

Premise: you fly a debt-ridden salvage tug for Mesa Verde Reclamation,
working the belt's wreck lines. A routine strip job turns up a sealed
military black-box crate that everyone in the belt suddenly wants; the
campaign escalates from scavenging to being hunted to deciding who gets
it. Cast: KESTREL (your tug); FOREMAN OKONO (Mesa Verde dispatcher, the
voice on comms); the MAGPIES (rival scav gang - light corvettes with
painted names: "Sixpence", "Tinfoil King"); BROKER VESH (the fence);
the "AUDITOR" (an unmarked military gunship, the finale boss). Tone:
working-class, dry humor, cozy-grim - the Factorio-cute of it.

- **Chapter 1 - Dead Weight** (the fantasy tutorial). Strip the wreck
  of the freighter "Ceres Queen"'s sister ship: collect a crate quota
  (salvage OnEnter counting - arena pattern on crates), a Magpie
  scout loiters non-hostile (orbit directive, flavor). The LAST crate
  is scripted: collecting it fires the twist (the sealed black box),
  flips the objective, Victory -> NextScenario ch2. Sky: cubemap.
- **Chapter 2 - Claim Jumpers** (first fight). The Magpies come to
  take it: two corvette waves (per-id OnDestroyed counting), your
  hauler escort must survive (broadside's defeat-if-neutral-dies
  pattern). Victory -> ch3. Sky: cubemap_alt.
- **Chapter 3 - The Quiet Channel** (tension). Run the crate through a
  mined debris channel to Broker Vesh: thread numbered NAV gates
  (gauntlet's expression-guarded gate sequencing) through a dense
  hazard scatter, with one mid-channel ambush beat (act-gated spawns,
  broadside pattern - and gate every handler to its act, per the
  act-gating domain lesson). Victory -> ch4. Sky: cubemap_alt2.
- **Chapter 4 - The Buyer** (finale, branching). The handoff sours:
  the AUDITOR arrives. TWO beacons open a choice (OnEnter branch sets
  a variable, gauntlet-style): fly the crate to Vesh's HANDOFF berth
  (sell it) or to the BURN buoy (destroy it). The choice gates which
  final wave + which epilogue objective text fires; both paths end in
  a boss fight and a Victory outcome with different closing text. Two
  endings, zero cross-chapter state. Sky: cubemap, relit by layout.

Mod shape: `webmods/the-ledger/` (id "the-ledger" - dash rule), one
`the-ledger.bundle.ron` listing FOUR content files (one per chapter -
deliberately multi-file to dogfood the multi-file bundle path), ch1
listed (`hidden: false`, the campaign's entry in the Scenarios picker),
ch2-4 `hidden: true` (reached only via NextScenario - the documented
pattern). Chapters recap the arc in their opening objective text.

Why this beats the runners-up: every mechanic above names the shipped
content that already proves it; the one engine addition (comms panel)
is optional-degradable; the identity is distinct from the base
storyline; and publishing a four-file bundle exercises exactly the
portal surface the umbrella task wanted stressed.

## Open questions

- Comms panel design details (queue depth, dwell time, skip input) -
  /plan's job on the seeded engine task; keep it scenario-scoped state
  cleared on teardown (the state-diff-aliases-reset family).
- Whether ch3's "mined channel" needs a new hazard primitive or reads
  fine as dense breakable scatter + ambush. Default: no new primitive;
  revisit only if the playtest reads flat.
- Campaign-own art (skybox/texture per chapter) upgrades when
  20260716-123544 lands mod resources; v1 ships on base assets.

## Next steps

- tatr 20260716-183220: comms/story-beat action + HUD panel (engine
  prerequisite, degradable).
- tatr 20260716-123535 (umbrella, updated): author The Ledger's four
  chapters as webmods/the-ledger and release it on the portal - /plan
  breaks it into per-chapter steps when picked up.

## Fix record

(Implementing tasks append here as they land.)
