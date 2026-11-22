# Menus: replace the remaining explanation lines with implied state

- STATUS: OPEN
- PRIORITY: 32
- TAGS: v0.19.0, ui, menu

## User facts
- Owner rule (2026-11-17): UI implies state; no explanation or refusal
  text.

## Agent findings
- Explanation lines left after the UI polish pass (2026-11-20):
  "Resets key and pad bindings in every group." under Reset Defaults, the
  reason line under a disabled Create in New Game, "Select a ..." hints,
  and the Load preview hint.

## Delivery
1. Propose, per line, what shows the state instead; owner picks.
2. Remove the lines and build the replacement.

## Verification
- Menu frames before and after; walks still find their buttons.

## Done when
- None of these lines remain and the state shows without prose.

## Owner check (2026-11-22)
- Keep in v0.19.0 at this priority.

## Proposal (2026-11-22)
Owner picks per row. Every row deletes its line and adds no new sentence.
New state uses kit paint only: `Selected`, `InteractionDisabled`, danger
button, the theme's invalid field outline, a tag badge. Frames: `before/`
(both themes), full size in `~/Pictures/nova-menu-lines/before/`. Explainer:
`proposal.html`.

### A. The four lines the task names
| # | Where | Text (when) | Options | Rec | Walks/tests that change |
|---|---|---|---|---|---|
| A1 | `crates/nova_menu/src/settings.rs:303`, spawn `:1565` | "Resets key and pad bindings in every group." (Reset Defaults armed) | a) delete; Cancel + danger Confirm carry it. b) a) plus idle Reset Defaults disabled at 0 overrides, armed Confirm reads `Reset 3` (`InputBindings::overrides().len()`) | b | `examples/screenshots/screenshot_menu.rs` (`RESET_PROMPT` in `assert_armed_row`) |
| A2 | `crates/nova_menu/src/world_setup.rs:82`, tooltip `:605`, label `:612`, details `:857` "No start role is loaded." | "Create is unavailable: no start role is loaded." (only with no start role loaded: a broken mod set) | a) delete label, tooltip, details line; disabled Create + empty role list carry it. b) a) plus New Game disabled on the main menu with no role | b | none |
| A3 | `crates/nova_menu/src/mods.rs:470`, `scenarios.rs:463`, `training.rs:919`; siblings `scenarios.rs:244`, `training.rs:743` | "Select a mod to see its details." / "Select a scenario to see its details." / "Select a lesson." / "No scenarios available." / "No lessons installed." (only with zero rows: each list selects its first row) | a) empty pane and empty list draw nothing. b) a muted icon in the empty pane | a | `crates/nova_menu/src/tests/mods.rs:260` |
| A4 | `crates/nova_menu/src/load_screen.rs:1392`; `world_setup.rs:1069`, `:1084` | "Right-drag turns, wheel zooms." (every readable save); "The sectors round the start of seed N: K station(s) in reach. Right-drag turns, wheel zooms." / "Seed N has no start: ..." | a) delete both; the chart shows the stations; a seed with no start marks the seed field invalid and greys Next (today only Create refuses it). b) a) plus a grab cursor over a preview (new; no `CursorIcon` in the game yet) | a | `tests/world_setup.rs:360`; `examples/screenshots/news_0160_world_setup.rs:60,266` waits on the caption; mock text in `examples/playable/ui_gallery/new_game.rs:302`, `load.rs:232` |

### B. Same-kind lines the sweep found
| # | Where | Text (when) | Options | Rec | Walks/tests that change |
|---|---|---|---|---|---|
| B1 | `crates/nova_ui/src/widget/text_field.rs:196` draws `TextFieldError`; texts `world_setup.rs:94`, `:123`, `nova_world_base/src/save/mod.rs:2116-2139` | "Enter a whole number from 0 to 4294967295", "name the world", "'!' is not allowed; use letters, digits, space, _ and -", name taken, "This system has no folder for saved worlds" (while typed, or after Create) | a) menu fields keep the invalid outline and draw no message (a `TextFieldSpec` flag; editor unchanged); seed field takes digits only. b) widget-wide: `TextFieldError` becomes a marker, editor messages die too | a; b is an owner call | `tests/world_setup.rs:280-284` (asserts "exists" in the message) |
| B2 | `crates/nova_menu/src/portal.rs:81`, spawn `:565` | "Deletes {name} from this computer." (Uninstall armed) | a) delete; Cancel + danger Confirm carry it | a | `screenshot_menu.rs` (`UNINSTALL_PROMPT` in `assert_armed_row`) |
| B3 | `crates/nova_menu/src/load_screen.rs:1539` | "Delete {title}? This removes its save files and cannot be undone." (Delete armed) | a) delete. b) a) plus the Reset/Uninstall shape: Cancel takes Delete's box, danger Confirm beside it | b | `tests/load_screen.rs:461` |
| B4 | `load_screen.rs:931`, `:1493`, `:1343` | row "Cannot load"; details refusal text; "Cannot delete {slug}: {reason}" | a) row line dies (INVALID badge shows it); details show one tag per `WorldRefusal` kind (LOCKED, FORMAT, CATALOG, UNREADABLE); refused delete plays deny, row stays. b) keep the details text as a fault report | a | `tests/load_screen.rs:377,431` |
| B5 | `load_screen.rs:781`, `:767` | "No saved worlds yet"; "This system has no folder for saved worlds" | a) main-menu Load disabled with no world listed; note dies. b) Load opens on the column head with no rows | a | none |
| B6 | `settings.rs:1413` from `nova_input/src/registry.rs:285,290,558`; armed chip `settings.rs:267` | "Q is already bound to Roll Left", "Left Mouse stays the pointer", "...the ship's {id} section"; "PRESS A KEY" / "PRESS A BUTTON" | a) refusal carries the holder, not a line: holder chip flashes danger (other group: its group tab), deny cue; armed chip shows its `Selected` face blank. b) keep the refusal, drop PRESS A KEY only | a | `tests/settings.rs:731,789,1078`; `nova_input/src/registry.rs:1260` |
| B7 | `settings.rs:1341` | "{theme}: {message}" (selected theme failed to load) | a) failed theme stays in the Theme row as a disabled segment with danger outline; message to the log | a | none |
| B8 | `mods.rs:403`, `:427`, `:449`; `portal.rs:723` | "Fetching the mod portal catalog...", "portal catalog fetch failed: io: Connection refused", "offline - showing the last fetched catalog", "offline - reconnect to install or update" | a) one tag on the Explore tab (FETCHING / OFFLINE, like CHECKING / INVALID), Retry stays, Install/Update drawn disabled | a | `tests/mods.rs:255`; `tests/portal.rs` error cases |
| B9 | `leave.rs:230`, `:244`, `:239` | "Progress since the last save is discarded. The ship resumes exactly as saved."; save wait reason; "{error}. The last save is kept." | a) first two bodies die, the title carries; SAVE FAILED keeps `{error}` only | a | none on the text |
| B10 | `safe_mode.rs:100` | "The game started without these mods, because their content failed to load. ..." | a) delete; banner + rows carry | a | none |
| B11 | `outcome.rs:372` | "Failed to start '{name}':" (repeats FAILED TO START) | a) scenario name only | a | none on the text |
| B12 | `world_setup.rs:403` (wasm) | "Saved worlds need the desktop build." | a) delete | a | none |

### C. Kept: data or authored content
Field note card and New pilot blurb (training content, own setting),
"Full manual: {page}" (`training.rs:1355`), lesson counts (`training.rs:612`),
mod dependency lines, "Enabled (base)", install progress, CHECKING / INVALID
badges, the save notice. Outside the menus: editor inspector "Nothing
selected. Pick a row, or a body on the stage."
(`crates/nova_editor/src/ui/inspector.rs:2532`), owner call.

### Findings beside the lines
- Hardware paints the danger Confirm and Delete world like a plain button
  (`before/reset-defaults-armed-hardware.png`,
  `before/load-delete-prompt-hardware.png`). With the prompts gone, the
  danger paint carries the state, so hardware danger needs a visible paint
  first.
- A field error message overlaps the next field's label
  (`before/field-refusals-*.png`).
- The rebind refusal line pushes every row down one line
  (`before/rebind-refusal-*.png`).
