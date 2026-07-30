# Relocate input-prompt key glyphs to assets/ (Alt only) and credit them in credits/

- STATUS: CLOSED
- PRIORITY: 41
- TAGS: v0.9.0,ui,assets,web
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

The FREE Input Prompts key glyphs (JulioCacko, CC0) sit under
`examples/ui/assets/input-prompts/` where the real game cannot load them
(Bevy reads `./assets/`), and all three styles (Alt/Dark/White, 99 PNGs
each, 2.4M) are committed although only Alt is used anywhere. Owner
direction (2026-07-28): keep ONLY the Alt style, move it into the real
`./assets/` tree, and record the license under `./credits/` like every
other third-party asset. The web easter egg's webpack copy and the HUD
PoC's img paths must follow the move. This unblocks the HUD icon dock
(20260728-175742), whose glyphs must load from the game asset tree.

## Steps

- [x] `git mv examples/ui/assets/input-prompts/keyboard/Alt
      assets/input-prompts/keyboard/Alt` (99 files, pack filenames verbatim,
      including the pack's `T_Crtl_Key_Alt.png` upstream typo); `git rm -r`
      the unused `Dark/` and `White/` styles; remove the now-empty
      `examples/ui/assets/` tree.
- [x] Credits: add a "FREE Input Prompts" entry to `credits/CREDITS.md`
      Third-party assets (pack name, author JulioCacko, itch.io + fab.com
      source links, version 1.4, obtained 2026-07-25 / imported 2026-07-28,
      CC0 1.0, note that only the Keyboard_Mouse Alt style ships), absorbing
      the provenance from `examples/ui/assets/input-prompts/NOTICE.md`
      (including the trademark note about the not-imported gamepad sets);
      then delete NOTICE.md. Add
      `credits/licenses/FREE-Input-Prompts_CC0-1.0.md` with the CC0 1.0 text
      (mirror the shape of `space-3d_Unlicense.md`).
- [x] While in CREDITS.md: fix the broken Kenney license link
      (`licenses/kenney-space-kit_CC0_1.0.md` -> the actual
      `licenses/Kenney_Space_Kit_License.txt`).
- [x] `web/webpack.config.js:324`: replace
      `{ from: "../examples/ui/assets", to: "nova-hud/assets" }` with
      `{ from: "../assets/input-prompts", to: "nova-hud/assets/input-prompts" }`.
- [x] `examples/ui/hud_rework_poc.html`: keep the deployed-shape
      `assets/input-prompts/...` img srcs, and in the existing onRoute
      script block rewrite the `img.ki`/`.navbtn img` srcs to
      `../../assets/input-prompts/...` when NOT on the `/nova-hud/` route,
      so the file:// review copy still renders glyphs.
- [x] Repo-wide path sweep (sweep-content-repo-wide-not-just-assets):
      `grep -rn "examples/ui/assets"` over code/docs/scripts/web (tasks/
      excluded, dated history stays) and fix every live hit. Add a
      CHANGELOG [Unreleased] credits line.
- [x] Verify per DoD: file counts, web build output, file:// eyeball.

## Definition of Done

1. cmd: `ls assets/input-prompts/keyboard/Alt | wc -l` prints 99 and
   `git ls-files examples/ui/assets` prints nothing.
2. cmd: `git ls-files | grep -cE "_Key_(Dark|White)\.png"` prints 0 (the
   unused styles are gone from git).
3. cmd: `grep -rn "examples/ui/assets" --exclude-dir=tasks --exclude-dir=node_modules --exclude-dir=dist .`
   prints 0 hits.
4. cmd: `grep -n "Input Prompts" credits/CREDITS.md` hits,
   `test -f credits/licenses/FREE-Input-Prompts_CC0-1.0.md` passes, and
   `grep -n "Kenney_Space_Kit_License.txt" credits/CREDITS.md` hits (broken
   link fixed).
5. cmd: web build (`npm run build` in web/), then
   `ls web/dist/nova-hud/assets/input-prompts/keyboard/Alt | wc -l` prints
   99 and no Dark/White dir exists under dist.
6. manual: `hud_rework_poc.html` opened from file:// shows the key glyphs
   on the dock, cues and NOVA OS button (chromium screenshot eyeball,
   render-output-eyeball).

## Notes

- Bevy loads these by plain path
  (`asset_server.load("input-prompts/keyboard/Alt/T_G_Key_Alt.png")`) - no
  manifest entry needed (`mods.catalog.ron` lists mod bundles only). The
  native release workflows and the Trunk web build bundle `assets/`
  wholesale, so the game gains the glyphs with zero extra wiring; the Trunk
  post-build hook auto-generates the web `.meta` sidecars.
- Pack's Ctrl glyph filename is `T_Crtl_Key_Alt.png` (upstream typo) and
  brackets are `T_Brackets_L/R_Key_Alt.png`; keep filenames verbatim, the
  mapping table in 20260728-175742 owns the translation.
- Answers backlog 20260728-214929's "canonical asset home" open question;
  recorded in this task's DECISION.md. 214929 keeps the remaining adoption
  surfaces (web key-UI, NOVA OS help lines, editor key chips, gamepad).
- No git-lfs in this repo; PNGs stay plain git objects (~800K after the
  cut, down from 2.4M).
- Depends on: nothing. Slots before 20260728-175742 (HUD dock is the first
  real consumer).
