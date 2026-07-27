# REVIEW - task 20260727-172205 (web NOVA OS text / .ttc meta)

## Round 1 - out-of-context reviewer (commit c04ad340)

Verdict: **APPROVE** (one minor, resolved below).

Verified against Bevy 0.19 source:

1. CORRECTNESS (loader type-string match) - OK. `default_meta()` emits
   `L::type_path()`; the runtime resolves a meta-named loader via
   `get_asset_loader_with_type_name`. Both the game (`nova_os.rs`) and the tool
   (`nova_meta_gen/src/lib.rs`) register the SAME single `NovaOsTtcFontLoader`
   type, reached via the public path `nova_gameplay::hud::nova_os`, so both
   emit/resolve `nova_gameplay::hud::nova_os::NovaOsTtcFontLoader`. Not a
   re-declaration.
2. `init_asset::<Font>()` omission - SAFE.
   `write_default_loader_meta_file_for_path` only does a registry lookup +
   `default_meta()` + read/write; it never touches `Assets<Font>`.
   `register_asset_loader` only calls `register_loader`. Mirrors the existing
   `AudioLoader` registration (no `init_asset::<AudioSource>()`).
3. Feature-set / GPU risk - CLEAR. `nova_modding` already depended on
   `nova_gameplay`, so it was already in meta_gen's graph; Cargo.lock adds one
   edge, no new crates, no render feature, no `RenderPlugin`. Headless deploy
   hook unaffected.
4. TEST QUALITY - genuine RED-first. Counts consistent (`written:8`,
   `already_exists:1`, `no_loader:1`; second pass `already_exists:9`); the
   `cases` array asserts `fonts/term.ttc` names `NovaOsTtcFontLoader`. Before
   the fix the `.ttc` classifies `NoLoader` and the meta read panics - fails for
   the right reason.
5. OFL license / credits - license file is canonical SIL OFL 1.1; CREDITS.md
   link path correct.

### Minor raised: Reserved Font Name clause in the copyright line

Reviewer suggested restoring `... with reserved font name "Iosevka".` to the
copyright line.

RESOLUTION: **rejected with evidence, no change.** Two independent reads of the
current upstream `LICENSE.md`
(https://raw.githubusercontent.com/be5invis/Iosevka/refs/heads/main/LICENSE.md)
confirm the copyright line is verbatim
`Copyright (c) 2015-2026, Renzhi Li (aka. Belleve Invis, belleve@typeof.net)`
with NO reserved-font-name clause. Current Iosevka does not declare an RFN in
its notice (older versions did). Our license file must reproduce the notice the
author actually ships; adding an RFN the author did not declare would misstate
it. Kept verbatim-faithful.

### Nits (accepted, no action)

- No `.ttc.meta` is committed for the real font - correct; the repo commits
  only the two hand-authored cubemap metas and generates all default metas at
  build time. DoD item 4 ("web shows text") remains a manual browser check.
- 66 MB `.ttc` download cost is explicitly scoped out with a follow-up note.

No blockers or majors. Ready to land after the retro.
