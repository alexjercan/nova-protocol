# RETRO - task 20260727-172205 (web NOVA OS text invisible / .ttc meta)

## What happened

Web build rendered NO NOVA OS text (native fine). Root cause: the terminal
font is a `.ttc` claimed only by nova_gameplay's custom `NovaOsTtcFontLoader`;
the build-time `.meta` generator `nova_meta_gen` did not register that loader,
so the `.ttc` shipped with no `.meta` sidecar. Under `AssetMetaCheck::Always`,
the web's missing-meta fetch returns 200-OK HTML (SPA fallback), Bevy fails to
parse it as RON, the font never loads, and every glyph is invisible. Fix:
register the same loader type in meta_gen so it emits the default sidecar.
Also added the missing SIL OFL 1.1 attribution for the font.

## What went well

- Diagnosis was evidence-first, not theory-first: counting metas in `dist/`
  (184 assets, 180 metas; the `.ttc` the only real asset without one)
  pinpointed the mechanism before touching code.
- The symptom SHAPE discriminated between two candidate causes. A parallel
  explore agent proposed a RenderLayers/RTT-propagation theory; but "buttons
  and logo draw, only their glyphs are missing" is a font-load failure, not a
  layer failure (a dead layer drops the whole screen). Matching failure-mode to
  symptom killed the wrong lead cheaply.
- RED-first at the right altitude: extended meta_gen's existing end-to-end
  generate test with a `.ttc`, watched it fail for the right reason
  (`NoLoader`), then went green. The web "text visible" proof stays a manual
  browser check, correctly flagged.
- Out-of-context review verified every load-bearing claim against the Bevy 0.19
  source (type_path match, no `init_asset` needed, no GPU/feature regression).

## Difficulties / bugs hit

- FIRST RED run reported `exit 0` and I nearly misread it as a pass - it was
  `cargo test ... | tail -30` letting `tail`'s exit code mask cargo's. This is
  the EXACT footgun AGENTS.md warns about ("never end a build/test with a pipe
  that eats its exit code"). Caught it, re-ran writing to a file + bare exit.
  Reinforced, not new.
- The reviewer's one "minor" (add reserved-font-name "Iosevka" to the copyright
  line) was itself based on an outdated upstream assumption. Verifying the
  ACTUAL current `LICENSE.md` twice showed the notice has no RFN clause;
  reproducing what upstream really ships beat accepting a plausible claim.

## Lessons (candidates for the ledger at Finish)

- `custom-asset-loader-needs-meta-gen-registration` (NEW, domain): every custom
  `AssetLoader` the game registers must ALSO be registered in `nova_meta_gen`,
  or its assets ship with no web `.meta` and fail silently under
  `AssetMetaCheck::Always` (200-OK-HTML SPA fallback). Sibling of
  `asset-meta-always-web-cost`. A future guard: assert meta_gen's loader set
  covers every extension the game's registered loaders claim.
- `match-failure-mode-to-symptom-shape` (reinforce): "only glyphs missing, rest
  of the screen fine" is font-load, not render-layer - the symptom's shape
  ruled out a whole class before any code change.
- `pipe-eats-exit-code` (reinforce, already in AGENTS.md): a test piped to
  tail/grep reports the filter's exit, not the compiler/test's. Bare-run or
  redirect-to-file.
- `verify-engine-guarantees-in-source` (reinforce): both the fix's correctness
  (type_path/loader resolution, init_asset) and the license fidelity were
  settled by reading the actual source/upstream, not by trusting a claim.
