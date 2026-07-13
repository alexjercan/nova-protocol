# Create a main menu

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.5.0,ui,spike

Things I would like the main menu to have:
- New Game : brings us into a ready to play scenario - something more fun than
  03_scenario example, but still simple to play, sure there can be enemies but
  not that aggresive and instantly destroying us: TODO: creating a task follow
  up for the scenario with spike and stuff; not that relevant for this task,
  but brainstorming opinions
- Sandbox : brings us into the editor mode where you can load/save ships and
  build new ones, useful for the task follow up that already exists for
  load/save; then allow you to play into a sandbox scenario that is really
  simple: e.g 03_scenario is a good fit for this one but maybe with some tweaks
  and without objectives (idealy no enemies, or passive enemies that get
  activated on contact)
- Settings : for now it doesn't do anything, but it will help with visual
  quality (there is a task about this) , maybe keybinds, sounds etc but for now
  it can be empty.
- Exit : quits the game

I think the main menu should be small-ish in the bottom right corner somehow,
with the title Nova Protocol at the top and then in the middle of the screen
maybe we can have game scenes playing out, we can think of some simple ideas of
how to achieve that (factorio style I am thinking)

Also another follow up task would be enabling/disabling of the extra UI
elements (e.g by pressing `~` it would disable the HUD). I think it would be
nice for cinematic shots to not have the HUD enabled. We can even have multiple
levels of HUD: ALL, MINIMAL and NONE, and for example `~` can cycle them, or
holding `~` goes to NONE and pressing goes from NONE -> MINIMAL -> ALL or
something. Whatever would make most sense for the user to be fair.

I would start by creating the needed tasks to implement the main menu and
adding them to v0.5.0; stuff that can be defered for later can go into v0.6.0

Resolution (spike, 20260711): researched and closed. Direction and reasoning
in tasks/20260711-180500/SPIKE.md. Seeded tasks:

- 20260711-180426: main menu state, panel UI, mode wiring (v0.5.0, p45)
- 20260711-180455: ambient menu background scenario (v0.5.0, p44)
- 20260711-180501: HUD visibility levels via tilde (v0.5.0, p42)
- 20260711-180506: starter New Game scenario (v0.5.0, p40, needs own spike)
- 20260711-180511: settings menu content (v0.6.0, p0)
