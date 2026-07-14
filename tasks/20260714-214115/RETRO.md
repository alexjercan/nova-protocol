# Retro: restyle nova_menu to nova_ui theme

- TASK: 20260714-214115
- BRANCH: ui/menu-restyle
- REVIEW ROUNDS: 1 (APPROVE)

A smooth cycle - process notes only.

## What went well

- The nova_ui foundation (214111) made this almost mechanical: delete 5 consts,
  `sed` the two panel/text colours, retheme `button()` + `update_button_colors`,
  and re-point ~9 inline colours. The heavy design decisions were already made.
- Keeping the menu's own polling colour system (rather than adopting nova_ui's
  observers) was the right self-containment call: it dodged double-registering the
  global colour observers in the assembled app, for the cost of one extra
  `BorderColor` in the query.
- Treating the HUD-family lesson early: I classified the menu's greens as SEMANTIC
  (on/off state) and mapped them to the shared cyan accent instead of flattening,
  keeping the state legible.

## What went wrong

- Nothing notable. The `12_menu_newgame` autopilot completed this time (lighter
  phase machine than 09_editor + the machine had settled), so the New-Game path
  verified cleanly.

## What to improve next time

- The bulk-panel-border edit needed splitting by indentation (3 nested panels vs
  the menu panel) because `replace_all` matches exact whitespace. Fine, but worth
  remembering: group structural edits by their indent level.

## Action items

- [x] Landed; 11 tests + autopilot green.
- [ ] Next: task 20260714-214118 (HUD palette centralization - the subtle one).
