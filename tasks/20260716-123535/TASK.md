# Story campaign mod: an alt storyline across multiple scenarios with a real story, published on the portal

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.7.0,scenario,content,story,spike

## Goal (reframed 20260716, user direction)

An ALT STORYLINE shipped as a downloadable portal mod: a big story told
across MULTIPLE scenarios, with a real narrative arc - not a bag of unrelated
one-offs. The base game's New Game progression stays its own storyline (the
vertical slice 20260708-203659 is its next chapter); this campaign is the
alternative one you install from the portal. Chapters chain with
NextScenario, the story carries between them (objective text, variables,
scripted beats), and each chapter should have its own fantasy and look.

Write a cool story first - premise, cast of ships, escalation, ending - then
lower it into chapters. Candidates to weigh when planning: a salvage crew in
over its head, a convoy escort arc that goes wrong, a hunt across the belt.
Authoring is hand-written RON per the proven gauntlet path (the editor
builder stays backlog). Publishing it dogfoods the portal pipeline -
authoring, nova_portal_gen validation, install-over-the-wire, enable/merge,
multi-file bundles - end to end with a real release artifact.

Direction-level; /plan breaks it into steps when picked up.

## Direction (spike 20260716-183104, 2026-07-16)

The story is chosen: "The Ledger" - a salvage crew in over its head,
four chapters (Dead Weight / Claim Jumpers / The Quiet Channel / The
Buyer), shipped as webmods/the-ledger (four content files, ch1 listed,
ch2-4 hidden, linear NextScenario chain, branching two-ending finale
INSIDE ch4 - no cross-chapter state needed, which the spike verified
has no carrier today anyway: world.clear() on every scenario change).
Every beat maps to a proven primitive; the one engine prerequisite is
the comms/story-beat panel (20260716-183220, degradable). Full premise,
cast, per-chapter mechanics and the rejected options live in
tasks/20260716-183104/SPIKE.md - /plan starts there.

## Steps (planned 2026-07-16, from SPIKE 20260716-183104)

- [ ] Scaffold webmods/the-ledger/: the-ledger.bundle.ron (meta name
      "The Ledger", version 1.0.0, four content files) + README.md.
- [ ] Chapter 1 "Dead Weight" (ledger_ch1.content.ron): wreck-strip -
      salvage-crate quota via act-gated OnEnter counting (shakedown
      pattern), Magpie scout on patrol (Neutral, flavor), scripted
      black-box crate twist, StoryMessage voice beats, Victory ->
      NextScenario ch2; player-death Defeat + retry-self (broadside
      pattern). Listed in the picker (hidden: false).
- [ ] Chapter 2 "Claim Jumpers" (ledger_ch2.content.ron): two corvette
      waves (per-id OnDestroyed counting, arena pattern), Neutral hauler
      escort whose death is Defeat (broadside pattern), Victory -> ch3.
      hidden: true.
- [ ] Chapter 3 "The Quiet Channel" (ledger_ch3.content.ron): NAV-gate
      threading (gauntlet expression-guarded sequencing) through a dense
      breakable debris lane, one mid-channel ambush spawn, Victory ->
      ch4. hidden: true.
- [ ] Chapter 4 "The Buyer" (ledger_ch4.content.ron): HANDOFF/BURN
      choice beacons (OnEnter branch sets `choice`), AUDITOR boss fight,
      two Victory endings gated on `choice` with distinct closing text;
      no NextScenario (campaign end). hidden: true.
- [ ] Every handler act-gated (gate-scenario-handlers-to-their-acts);
      every chapter opens with a recap objective + StoryMessage.
- [ ] Validate: webmods_validation (real loaders over webmods/),
      nova_portal_gen tests (generic publish gate), check/fmt; headless
      load-smoke of each chapter if a cheap rig exists, playtest gap
      reported honestly.
- [ ] CHANGELOG Unreleased (Modding & Mod Portal): The Ledger published.
- [ ] SPIKE.md fix record entry.

## Notes

- Spike: tasks/20260716-122954/SPIKE.md (v0.7.0 release scope)
- Plan: docs/plans/20260716-v0.7.0-plan.md, strand 1
- Campaign-own art (skybox, textures) rides the mod-resources support in the
  asset variety pack (20260716-123544); until that lands, chapters can only
  reference base-game assets like gauntlet does today.
- If chapters share an enemy ship definition, that is the consumer that
  activates ship prototypes (20260714-134115).
- A multi-scenario story will stress the scenario vocabulary (persistent
  state between chapters, story text presentation); gaps found are v0.7.0
  modding-surface work, which is part of the point.
