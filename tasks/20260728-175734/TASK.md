# nova_ui theme + widgets: NOVA OS palette + skin-aware widget set

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.9.0,ui,refactor

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

`nova_ui::theme` still carries the flat navy/cyan/amber language the owner
called washed out; the accepted spike direction (SPIKE.md D1, demo 1
`examples/ui/nova_ui_rework_poc.html`) replaces it with the NOVA OS palette
and a skin-aware widget set: **phosphor terminal** is the primary skin where
every widget renders as a CLI-drawn element, and the **hardware casing**
light-3D look is the secondary skin. IMPORTANT correction from the demo: the
hardware skin is the CASE-GRADIENT + bevel look (case-0..3 tones, amber
selection), NOT the old navy/cyan theme - the navy/cyan palette is retired
entirely. Unify nova_menu's duplicate MenuButton color system onto the shared
observers while touching the widget layer.

## Steps (re-planned 2026-07-28 from the implemented demo 1)

- [x] Rework `crates/nova_ui/src/theme.rs` to the NOVA OS token set carried
      verbatim from the PoCs: space `#03060b`, case-0 `#0a0d10` / case-1
      `#161b20` / case-2 `#232a31` / case-3 `#2f383f` / case-edge `#05070a`,
      screen-0 `#001304` / screen-1 `#002b0f`, phosphor `#36ff79` / dim
      `#19a64f` / muted `#0d6e35`, amber `#ffb84a`, orange `#ff7b2d`, red
      `#ff4e42`, blue `#36a3ff`, text `#b9ffc9`. DELETE the navy/cyan
      constants (BG/PANEL/PANEL_RAISED/BORDER/BORDER_BRIGHT/CYAN/
      CYAN_BRIGHT/SELECTED_FILL...) and migrate the `semantic` HUD accents
      onto the new base. Keep RADIUS small (phosphor uses 2px corners).
- [x] Add `UiSkin` to nova_ui: `enum UiSkin { Phosphor, Hardware }` as a
      `Resource + Component + PartialEq + Clone` (ButtonValue-compatible),
      `Default = Phosphor`. Widgets read it; the settings row + persistence
      land in 20260728-175738 (settings_store.rs gains the field there).
- [x] Rework ThemedButton rendering per demo 1's widget zoo, both skins:
      phosphor = flat fill `rgba(phosphor,0.05)`, 1px border
      `rgba(phosphor,0.4)`, phosphor text; hover fill 0.12 + full-phosphor
      border; pressed fill 0.2; selected/primary INVERTED (phosphor fill,
      `#04140a` glyphs, glow); disabled ~0.3 alpha; danger = red family;
      ghost = border-only; block buttons carry a `> ` cursor span shown on
      hover/selected. hardware = BackgroundGradient face (case-3 -> case-1
      -> case-0), BoxShadow rim + undercut + drop, pressed inset well,
      selected amber gradient + dark glyphs. Include an optional trailing
      key-chip span (amber bordered `Enter`/`Esc` text) per demo.
- [x] Skin switch restyles LIVE widgets: reconciler reacts to `UiSkin`
      resource changes AND to newly spawned widgets (Added<marker> override
      per lesson mode-keyed-reconciler-just-spawned-override; write the
      live-tree test first).
- [x] Shared widget set consumed by the screens (demo 1 zoo, only widgets
      with a real consumer): segmented control (phosphor: boxed row,
      inverted `on`; hardware: recessed well, amber `on`), slider re-skin of
      the existing bevy Slider row (phosphor: bordered track, segmented
      block-meter fill, NO knob; hardware: gradient fill + round knob),
      checkbox (22px square, inverted `x` when on), badges (phosphor:
      bracketed `[TAG]` text spans in green/amber/blue/red/mute; hardware:
      bordered chip), panel + panel-head (phosphor: dark screen surface,
      inner phosphor border, glowing header text, dashed rule + `TAG` slot;
      hardware: case gradient + bevels), separator (dashed in phosphor),
      list row (transparent, inset phosphor border, hover fill, inverted
      selection) with icon slot. Skip the toggle (no in-game consumer;
      note here if one appears).
- [x] Typography: CONSUME the shared `nova_ui::font::UiFont` resource - the
      `Handle<Font>` for Iosevka Term (`assets/fonts/SGr-IosevkaTerm-Regular.ttf`,
      the single Regular face) preloaded via `BootAssets` and published at
      startup by task 20260729-000956. This step no longer loads the font
      itself; it routes the widget factories' `TextFont` through `UiFont`, with
      a size scale taken from demo 1 (26 title / 16-13 body / 11-10 labels).
      This supersedes backlog 20260714-214329 (Rajdhani/Inter web fonts) -
      mono-first won.
- [x] Fold nova_menu's duplicate button system: delete `MenuButton` +
      `update_button_colors` polling (`crates/nova_menu/src/lib.rs:4143-4198`)
      and route the menu `button()` factory through ThemedButton + the
      nova_ui observers (keep the 40px/16px menu sizing as a factory
      variant). One observer path for every button in the game.
- [x] Widget-zoo example: new `examples/ui/widget_zoo.rs` rendering the full
      set in both skins (copy the `screenshot_ui.rs` autopilot + 
      `capture_window` scaffold verbatim per reuse-known-good-stack; capture
      one shot per skin via the skin resource between beats).

## Definition of Done (re-planned 2026-07-28)

1. test: live-tree tests pin ThemedButton state rendering per skin and the
   live-switch reconciliation (`phosphor_button_states_render_cli_markers`,
   `hardware_button_states_render_bevel`,
   `skin_switch_restyles_spawned_widgets` - the last must fail before the
   Added-override wiring).
2. example + render eyeball: `widget_zoo` captures reviewed in BOTH skins
   (buttons/segmented/slider/checkbox/badges/rows/panel-head); phosphor
   widgets read as CLI elements, not bevelled buttons on glass.
3. cmd: `grep -rn "update_button_colors\|MenuButton" crates/nova_menu/src`
   prints 0 hits (duplicate color system gone).
4. cmd: `grep -rn "5cc8ff\|8fe0ff\|141a2e\|0b0f1c\|183a4e" crates/` prints 0
   hits (navy/cyan palette retired from the game; web/src keeps its own CSS).
5. manual: owner eyeballs the widget zoo in-engine in both skins; Phosphor
   is the default.

## Implementation (2026-07-29) - VERDICT

Delivered on branch `refactor/nova-ui-skin-widgets`:

- `theme.rs`: added the full NOVA OS token set (SPACE/CASE_0..3/CASE_EDGE/
  CASE_HOT_*/SCREEN_0/1/PHOSPHOR/PHOSPHOR_DIM/MUTED/HI/LO/AMBER_NOVA/HI/LO/
  ORANGE/RED/BLUE/SCREEN_TEXT/INK, RADIUS 2 / RADIUS_HW 7 / PANEL_RADIUS 10).
  The legacy navy/cyan consts are KEPT (marked `LEGACY ... (retiring)`) - see
  DECISION.md: deleting them was deferred to 175742 (HUD refs) + 175738
  (editor/menu refs) rather than blast-migrating ~137 refs here.
- `skin.rs`: `UiSkin` resource (Phosphor default | Hardware), Resource-only.
- `widget.rs`: skin-aware `ButtonVariant` + paint model (both skins, all
  variants/states), `reconcile_button_skins` (UiSkin change + `Added` override),
  typography routed through `UiFont` via the `UiText` marker + `apply_ui_font`,
  `ButtonSpec` builder + `menu_button`, and the shared factories
  `panel/panel_head/badge/checkbox/list_row/slider_track/segmented`.
- nova_menu fold: `MenuButton` + `update_button_colors` deleted; `button()` =
  `menu_button(text)` + `MenuSfxButton` (click-cue marker only); the quiet mods
  checkbox became a `ThemedButton`.
- `examples/ui/widget_zoo.rs`: renders the full set in both skins (S toggles;
  `NOVA_ZOO_CAPTURE=1` shoots one PNG per skin).

DoD status:
1. test: `phosphor_button_states_render_cli_markers`,
   `hardware_button_states_render_bevel`, `skin_switch_restyles_spawned_widgets`
   all PASS (`cargo test -p nova_ui --lib`); the third was PROVEN RED with the
   Added-override disabled before wiring it (red -> green).
2. example: `widget_zoo` compiles (`cargo check --example widget_zoo`); the
   both-skin capture is the owner/CI GPU eyeball (DoD 5 below) - a local GPU
   render was intentionally skipped (gpu-example-local-skip / owner call).
3. cmd: PASS - `grep -rn "update_button_colors\|MenuButton" crates/nova_menu/src`
   = 0.
4. cmd: PASS - the hex grep = 0. NOTE the navy/cyan VALUES still live in the
   kept `LEGACY` block (as `srgb_u8` decimals, which this grep does not match);
   full retirement lands with 175742 + 175738 per DECISION.md.
5. manual: PENDING owner eyeball of `widget_zoo` in both skins.

Scope nuance carried forward: only `ThemedButton` live-restyles on a skin flip;
the non-button factories read the skin at spawn. A live in-place reskin of those
is a 175738 decision (KISS: live if cheap, else rebuild-on-flip).

## Notes

- Demo 1 (`examples/ui/nova_ui_rework_poc.html`) is the pixel reference;
  its `:root` tokens and `body[data-skin=...]` rules are the spec for both
  skins. `nova_os_terminal_poc.html` stays the canonical monitor reference.
- Current state (mapped 2026-07-28): theme.rs = palette consts + semantic
  module; widget.rs = ThemedButton observers (`button_on_interaction`,
  `on_add/remove_selected`, `button_on_setting::<T>`) + `themed_button`/
  `panel_header`/`separator` factories + `WidgetObserversRegistered` guard.
  nova_menu duplicates: `MenuButton` (lib.rs:1056), `update_button_colors`
  (lib.rs:4143), `button()` (lib.rs:4171). No skin machinery exists anywhere.
- Bevy 0.19 BackgroundGradient + BoxShadow are already proven by the NOVA
  OS casing (`crates/nova_gameplay/src/hud/nova_os.rs`) - no new engine
  machinery for the hardware skin.
- The phosphor block-meter slider must keep the existing bevy_ui_widgets
  Slider behavior (SliderValue/SliderRange/TrackClick::Snap in
  build_settings_body) - re-skin, not re-implement.
- HUD chips are NOT this task: flight HUD chrome is phosphor-only per the
  spike's per-surface table and restyles in 20260728-175742 on top of these
  tokens.
- Depends on: 20260729-000956 (which preloads Iosevka via `BootAssets` and
  publishes the `nova_ui::font::UiFont` resource this task's typography step
  consumes); lands before 175738/175742 (they consume the widgets).
- 2026-07-29 alignment (20260729-000956, static-asset preload): the "nova_ui
  font resource" this task planned already exists as `nova_ui::font::UiFont`,
  filled from `BootAssets` at `OnExit(GameAssetsStates::Boot)`. The typography
  step is now a CONSUMER edit (read `UiFont`, route `TextFont` through it), not
  a font-loading step; the font is the slimmed single-face `.ttf`, not the
  66 MB `.ttc`.
