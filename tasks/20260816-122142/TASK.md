# Stop leaking webmods into the main codebase

- STATUS: IN_PROGRESS
- PRIORITY: 64
- TAGS: v0.11.0, modding, testing, tooling

## The principle

Owner: "we SHOULD NOT HAVE UNIT TESTS THAT TEST LEDGER ... webmods ARE
PRACTICALLY OUTSIDE of the project, they get tested by the authors + linters so
it's all good there". And on the balance ack: "the ack is something really bad in
my opinion, because it leaks the webmods in the codebase".

**Webmods will move OUT of this repository.** So nothing in the main codebase may
know a webmod exists, name one, or depend on one to pass.

## It already cost us

The ship content lane left 458 KB of duplicated cargoa and cargob inlined across
the webmods, and said why: "the ledger's chapter tests assert on the exact
prototype mix of each hull". A webmod test blocked an engine-side cleanup. That
is the concrete cost, not a hypothetical one.

## 1. Six tests assert on webmod content

- `crates/nova_assets/tests/`: `ledger_ch2_encounter.rs`, `ledger_ch3_channel.rs`,
  `ledger_ch4_ending.rs`, `ledger_skybox.rs`, `gauntlet_course.rs`
- `crates/nova_authoring/tests/`: `ledger_ch5_raid.rs`

These are not only content assertions. They are production-faithful rigs that
register real handlers and drive act machines with real engine events - the
scenario event machine, deferred overlays, clock gating, skybox switching. That
is ENGINE coverage wearing ledger clothes, and deleting it wholesale loses it.

**The replacement, per the owner: independent tests built from RUST STRINGS
rather than read from webmod `.ron` files, using generic ids like `spaceship_1`.**
So each rig is rebuilt against a synthetic scenario defined inline in the test,
exercising the same engine path with no webmod in sight.

Audit each rig first: name the engine behaviour it pins, then rebuild that
behaviour on a synthetic fixture. A rig that pins only story content - which
chapter says what line - has no engine claim and simply goes.

## 2. Delete `webmods_validation.rs` outright

`crates/nova_assets/tests/webmods_validation.rs` loads every bundle under
`webmods/` through the real loaders to a recursive `Loaded`. It is the deep
publish gate.

Owner: "remove that thing, webmods WILL MOVE OUT at some point so let's not do
anything with it in the main codebase; afaik we only have a python script to
build the catalog". So the catalog script (`scripts/gen-portal.py`) owns that
check alone.

**Keep the engine-side loader coverage** it happens to provide, rebuilt on a
synthetic bundle under `assets/mods/` or defined in-test. The claim worth keeping
is "the real loaders load a real bundle to Loaded", which needs no webmod.

## 3. Move the balance ack into the mod

`crates/nova_authoring/balance_acks.ron` is a file in the MAIN codebase whose
entire content is one webmod's difficulty justification:

```
bundle: "the-ledger", scenario: "ledger_ch4_the_buyer", hostile: "auditor"
```

That is the leak in its purest form. It is also STALE - it claims "the light mook
turret (downgraded for exactly this spawn)" and no `_light` variant exists, and
it rests on a playtest taken when ammunition was infinite.

A mod declares its own acks, and the linter resolves them from the bundle it is
linting. **A declared list, not a parsed comment**: a comment is invisible to the
loader and brittle to match against a specific finding. Base content keeps its
acks in base's own bundle, symmetrically.

Once moved, the stale ch4 ack is the ledger author's problem, which is the right
place for it. Do not fix it here.

## Keep - these are BASE, not webmods

- `nova_assets/tests/lifeline_convoy.rs` and `final_tally_claim.rs` read
  `assets/base/scenarios/`. They look like campaign tests and are base content.
- `nova_authoring/tests/campaign_membership.rs` runs over generated base content.

## Definition of done

- no test in `crates/` reads anything under `webmods/`
- no file in `crates/` names a webmod bundle, scenario or object
- the engine behaviour each removed rig pinned is still pinned, on a synthetic
  fixture with generic ids
- a mod can declare its own balance acks and the linter reads them there
- `content lint` still reports the ch4 finding, now acked from inside the ledger
