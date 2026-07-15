# Scenario picker: a Scenarios menu modal in the mods-screen style - list + details pane, play any registered scenario

- STATUS: OPEN
- PRIORITY: 12
- TAGS: feature,menu,scenario

User request (20260715, on seeing the new mods screen): "the mods page looks
really good! we should add a similar style page for playing custom scenarios
(new game just plays the main story) but maybe we should add a 'scenarios'
button that opens a modal that let's you choose a scenario to play and
similarly it shows details and info about it with description name image etc".

Goal: a "Scenarios" main-menu button opening a modal in the mods-screen style
(GlobalZIndex overlay, list pane + details pane): the list shows every
PLAYABLE registered scenario from `GameScenarios` (base story scenarios plus
whatever enabled mods added - demo_mod_arena, gauntlet_run once installed);
selecting one shows details (name, description, source mod?, image); a Play
button loads exactly that scenario (LoadScenario + the New-Game-style state
handoff) instead of the main story chain. New Game stays untouched.

Known design questions for /plan (capture, do not decide here):
- IMAGE: ScenarioConfig has no image metadata today (id/name/description/
  cubemap/events). Needs a small schema addition - e.g. an optional
  `thumbnail: Option<String>` asset path on ScenarioConfig (serde-defaulted,
  same back-compat discipline as ModMeta) - plus authoring for the base
  scenarios, OR phase 1 ships text-only details like the mods screen did.
- FILTERING: some registered scenarios are not player-facing (menu_ambience;
  possibly chained mid-story entries) - likely needs a `listed: bool` or
  similar flag, mirroring the catalog's `hidden` lesson (and its session-only
  semantics discussion).
- PLAY WIRING: confirm how New Game hands off (GameMode + state) and what
  "play one scenario, then what" means at the end (return to menu?).
- Reuse: the two-pane markers/systems from 142911 (nova_menu) are the
  template; consider extracting shared list+details scaffolding rather than
  copy-pasting a second screen.

Related: the 142911 mods screen (style template), spike
tasks/20260714-202515/SPIKE.md (family context), 13_screenshot_reel
(LoadScenario-directly precedent).
