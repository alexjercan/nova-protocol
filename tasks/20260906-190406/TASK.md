# Retire the First Shift chapter: examples and story mod

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.13.0, content, cleanup

## Goal

The story goes back to the drawing board: the lore book comes before any
authored dialogue. The scripted First Shift chapter and its nine per-beat
scene examples are in the way of that, so retire them.

What survives is the SPATIAL work, as examples of what the engine can build:
`first_shift_map` (the belt layout bench) and `first_shift_ships` (the shipped
hulls posed in a row). The belt geometry those two read moves back beside them
under `examples/playable/shared/`, where it started, and stops being engine
content.

## Scope

- Delete `first_shift_01_departure` .. `first_shift_09_aftermath` and their
  launcher `shared/first_shift_scene.rs`.
- Delete `crates/nova_authoring/src/mod_content/` - the builders that generate
  the shipped `nova_protocol` mod - and the generated
  `assets/mods/nova_protocol/` tree with them.
- Move the belt stage (`stage.rs`) back into
  `examples/playable/shared/first_shift_stage.rs`, replacing the stale copy
  that sat there.
- Follow the removal through generation, the content CLI, the tests that read
  the generated chapter, the picker screenshot walk, the perf-web default
  scenario, and the docs.

## Non-goals

- The campaign FORMAT stays. `CampaignConfig`, the picker's campaign grouping
  and their tests are engine features a mod still uses; only the shipped
  campaign CONTENT goes.

## What changed

**Examples.** The nine per-beat scene fixtures and their `shared/first_shift_scene.rs`
launcher are gone, with their `[[example]]` blocks. `first_shift_map` and
`first_shift_ships` stay, re-documented as benches rather than campaign
fixtures.

**The stage moved back.** `examples/playable/shared/first_shift_stage.rs` is now
the belt itself, carrying what `nova_authoring`'s `stage.rs` had grown that the
example-side copy had not: the `SALVAGE_MIX` / `AMBIENT_MIX` material mixes, a
`rock` builder that names a kind and a destroy sound, and the planetoid ids. The
two copies that had to be kept in step are one copy again.

**The generator is gone.** `crates/nova_authoring/src/mod_content/` (5 279
lines) and the `assets/mods/nova_protocol/` tree it wrote are deleted, along
with the catalog entry that enabled the mod on a fresh install and the six story
portraits in `scripts/generate-campaign-portraits.py`. `generation.rs` lost
`build_story_scenarios`, `build_campaigns`, their `*_contents` wrappers and
`STORY_MOD_DIR`; `content_files()` is base-only.

**Tests kept their subject.** Three read the deleted chapter and were retargeted
rather than dropped, because the contracts are still real:

- `neutralized_ships` -> the tutorial. Basic Training has the same shape the
  chapter did - an `OnNeutralized` on the player gated below the outro beat,
  and drones that go quiet without declaring anything.
- `campaign_membership` -> a campaign declared over the generated base
  scenarios, the way a mod declares one. `merge_bundles`' campaign path has no
  other coverage.
- `example_scenario` -> a two-mod catalog (base + example).

## Proof

- `cargo check --workspace --all-targets`, and again `--features debug`: clean,
  no warnings.
- `content gen` writes no diff; `content lint`: 0 errors, 0 warnings,
  9 scenarios balance-audited.
- `cargo test -p nova_authoring --test content_ron_parity --test campaign_membership`: 4 passed.
- `cargo test -p nova_assets --test neutralized_ships --test example_scenario`: 17 passed.
- Live runs under Xvfb with `NOVA_AUTOPILOT=1`, all three "cycle complete, no
  panic": `first_shift_ships`, `first_shift_map`, and
  `screenshot_scenario_picker` - whose walk now selects the tutorial row and
  asserts the picker marked it `Selected`, then plays into the example arena.
- `mdbook build` and `web && npm run ci`: pass.

## Docs

`CHANGELOG.md` treats the chapter as never-shipped: the entries that announced
An Ordinary Shift, the story mod and its portraits are removed rather than
answered with a removal note. What is left against the v0.12.0 baseline is one
breaking entry - the `nova_protocol` campaign and its five chapters are gone -
plus the base portraits that really do ship.

The wiki, the create docs and the dev book lose their An Ordinary Shift claims;
where the claim was about a mechanic rather than the story (the 150 m/s speed
cap, `SetControllerVerb` withholding a verb, the beat-counter idiom) it now
cites Basic Training, which does the same thing.
