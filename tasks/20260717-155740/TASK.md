# Spike: storytelling and pacing - breathers, spawn telegraphs, readable comms/objectives

- STATUS: OPEN
- PRIORITY: 41
- TAGS: spike,v0.7.0,scenario,gameplay,hud

User feedback (2026-07-17 playtest, verbatim intent): "the gameplay feels
really rushed, each event happens immediately with no breaks and no space
to breathe; the enemies appear out of nowhere and it's hard to read
objectives and story messages: how can we improve storytelling here?"

Candidate mechanisms to weigh in the spike (grounded in the current
engine, see the difficulty-rework family tasks/20260717-111808/SPIKE.md):
- Content pacing pass: the scenario clock (scenario_elapsed, task
  20260717-112647) exists but only the example mod uses it - the campaign
  and ledger still chain every beat zero-delay (wave 2 the frame wave 1
  dies, victory the frame the last kill lands). Beat-sheet the shipped
  scenarios: announce -> pause -> spawn -> pause -> objective.
- Spawn telegraphs (engine): ships materialize instantly today. Options:
  clock-spaced StoryMessage warnings before spawns; an engage-delay /
  spawn-passive field on AIControllerConfig (spawn on patrol, go hot
  after N seconds or proximity); an entrance effect + radar ping action.
- Readable comms (engine/HUD): the story feed appends instantly - a paced
  queue with minimum on-screen time per line, a radio blip cue per line
  (audio infra just landed), shorter authored lines.
- Objective legibility (HUD): a new-objective toast/chime moment instead
  of only the side-panel update mid-dogfight.
