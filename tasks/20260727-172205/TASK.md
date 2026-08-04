# Web build: NOVA OS view renders no text (font .ttc gets no .meta sidecar)

- PRIORITY: 90
- TAGS: v0.9.0, bug, web, assets
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Problem

In the WEB (wasm/trunk) build, the Computer / NOVA OS view renders NO TEXT at
all: no FPS text, no bottom-left logo text, no text on buttons, no terminal
text on screen. Every non-text element and every mechanic works exactly like
native (typing `exit` stops it, wrong input plays the error sound, buttons and
the logo shape draw). Only glyphs are missing. Native is fine.

## Root cause (diagnosed, evidence-backed)

All NOVA OS text uses one custom font, `fonts/SGr-IosevkaTerm-Regular.ttc`, a
66 MB TrueType Collection (`.ttc`). It is loaded at runtime by a bespoke
`NovaOsTtcFontLoader` (crates/nova_gameplay/src/hud/nova_os.rs:649-671),
registered in `NovaOsPlugin::build` at nova_os.rs:1199. That loader claims the
`ttc` extension - Bevy's built-in `FontLoader` only claims `ttf`/`otf`, so this
custom loader is the ONLY thing that can load the font.

The game ships `AssetMetaCheck::Always` (nova_core::assets_plugin), so on the
web every asset load first fetches `<path>.meta`. A missing `.meta` under
`trunk serve` / any SPA-fallback host returns `200 OK` + an HTML body, which
Bevy tries to parse as RON and fails, and the asset never loads (see the
`asset-meta-always-web-cost` lesson, LESSONS.md). To avoid this, the
`nova_meta_gen` build tool writes a default `.meta` sidecar for every asset at
web build time.

But `nova_meta_gen` (tools/nova_meta_gen/src/lib.rs) only registers the
Image / Shader / Audio / Gltf / RON loaders. It does NOT register
`NovaOsTtcFontLoader`. So when it walks the assets and reaches the `.ttc`, no
registered loader claims the `ttc` extension -> `Outcome::NoLoader` -> no
`.meta` is written for the font.

Hard evidence: in the built `dist/assets/`, 184 real assets have 180 `.meta`
sidecars. The 4 without a meta are three `README.md` files (no loader, correct)
and `fonts/SGr-IosevkaTerm-Regular.ttc` - the only REAL asset missing its meta.

Consequence on web: Bevy fetches `...ttc.meta`, gets HTML, fails to parse it,
the font handle never loads, and every glyph drawn with that font is invisible.
Non-text UI (borders, buttons, logo shape) draws because it does not depend on
the font. On native the missing-meta fetch is a real filesystem NotFound, Bevy
falls back to the loader default meta, and the font loads - so native is fine.

This also rules out the RenderLayers-propagation theory: an RTT/layer failure
would drop the WHOLE screen content (buttons and logo too), not just glyphs.

## Fix direction (to confirm at the plan gate)

`nova_meta_gen` must know about the `.ttc` font loader so it emits a
`...ttc.meta` naming that loader. The meta must name the SAME loader type that
the runtime registers, or the runtime (which resolves the loader named in the
meta) will not find it. Candidate approaches recorded in DECISION.md.

## Reproduction plan (bug playbook: reproduce first)

Highest-fidelity harness we can run headlessly is the existing
`tools/nova_meta_gen/tests/generate.rs` end-to-end test. Add a `.ttc` asset to
its temp asset tree and assert the CURRENT behavior classifies it as
`NoLoader` / writes no sidecar (the reproduction of the web failure mechanism).
After the fix, flip it to assert a `...ttc.meta` is written naming the font
loader. Real "web shows text" is a manual browser check (manual: DoD item).

## Chosen fix

Option A (see DECISION.md): register the real `NovaOsTtcFontLoader` in
`nova_meta_gen` so it auto-generates the default `...ttc.meta` naming that
loader, exactly as it does for every other asset. `nova_gameplay` is already in
meta_gen's build graph, so this is a `pub` on the loader type, a direct
`nova_gameplay` dep, and one `register_asset_loader` line.

## Steps

- [x] RED: extend `tools/nova_meta_gen/tests/generate.rs` - add a
      `fonts/x.ttc` asset to the temp tree, expect a `x.ttc.meta` written
      naming `NovaOsTtcFontLoader`, and bump the summary counts. Run it and
      watch it fail for the right reason (font classified `NoLoader`, no meta).
- [x] Make `NovaOsTtcFontLoader` `pub` in
      `crates/nova_gameplay/src/hud/nova_os.rs` (keep it where it is; the
      module path `nova_gameplay::hud::nova_os` is already public).
- [x] Add `nova_gameplay = { path = "../../crates/nova_gameplay" }` to
      `tools/nova_meta_gen/Cargo.toml` and `register_asset_loader`
      `NovaOsTtcFontLoader` in `nova_meta_gen::build_app`, next to the other
      loader registrations, with a comment on why the font's custom loader must
      be here.
- [x] Run the meta_gen test suite; the RED test now passes (meta written,
      names the loader, default `()` settings).
- [x] Regenerate metas for the real asset tree (`cargo run -p nova_meta_gen`
      against `assets/`, or a fresh `trunk build`) and confirm
      `assets/fonts/SGr-IosevkaTerm-Regular.ttc.meta` (or the dist copy) now
      exists and names `NovaOsTtcFontLoader`.
- [x] ATTRIBUTION: the shipped Iosevka Term `.ttc` is SIL OFL 1.1, which
      REQUIRES the copyright notice + license text to ship with the font (it is
      currently uncredited - a pre-existing gap from task 20260726-142635).
      Add `credits/licenses/Iosevka_OFL-1.1.md` containing the exact copyright
      line (`Copyright (c) 2015-2026, Renzhi Li (aka. Belleve Invis,
      belleve@typeof.net)`) and the full OFL 1.1 text, and add a
      **Iosevka Term** bullet under "Third-party assets" in `credits/CREDITS.md`
      linking that license file and naming the font path.
- [x] `cargo fmt` + `cargo check` the touched crates.

## Definition of Done

1. The `.ttc` font gets a default `.meta` from `nova_meta_gen`, proven by the
   generate test. (test: `cargo test -p nova_meta_gen`)
2. `NovaOsTtcFontLoader` is registered in `nova_meta_gen::build_app` so no
   custom-loader asset is silently skipped. (cmd: `grep -n NovaOsTtcFontLoader
   tools/nova_meta_gen/src/lib.rs`)
3. A built/regenerated asset tree contains
   `fonts/SGr-IosevkaTerm-Regular.ttc.meta` naming the font loader. (cmd:
   `test -f assets/fonts/SGr-IosevkaTerm-Regular.ttc.meta` after a gen run)
4. In the WEB build, the NOVA OS view renders text (FPS, logo, buttons,
   terminal). (manual: load the wasm build and confirm text is visible)
5. The Iosevka Term font is attributed per SIL OFL 1.1: a license file exists
   and CREDITS.md lists it. (cmd: `test -f credits/licenses/Iosevka_OFL-1.1.md
   && grep -i iosevka credits/CREDITS.md`)

## Notes / out of scope

- The 66 MB `.ttc` download size is a separate perf concern (subset/convert to
  a single-face `.ttf`); not part of this text-visibility fix. File as a
  follow-up if desired.
