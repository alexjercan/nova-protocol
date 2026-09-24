# Before: `PlanetConfig { invulnerable: false }` was refused

Baseline commit: `8a85df12a` (`8a85df12adcf9d75625de1b29f992546108d1a0f`).
Raw logs were captured to `/tmp/nova-vulnerability-before/`. Compile output is
omitted here.

## Commands and results

```bash
nix develop --command cargo test -p nova_scenario --lib lint::scenario::tests::a_planet_authored_with_impossible_figures_is_an_error -- --exact --nocapture
```

```text
running 1 test
test lint::scenario::tests::a_planet_authored_with_impossible_figures_is_an_error ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 438 filtered out; finished in 0.00s
```

That test authored a planet `destructible` with `invulnerable: false` and
asserted that lint reports an error naming `planet 'destructible'`.

```bash
nix develop --command cargo test -p nova_scenario --lib objects::planet::tests::before_artifact_false_planet_is_refused_at_load -- --exact --nocapture
```

```text
running 1 test
BEFORE: invulnerable=false planet spawned no PlanetMarker
test objects::planet::tests::before_artifact_false_planet_is_refused_at_load ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 438 filtered out; finished in 0.07s
```

`before_artifact_false_planet_is_refused_at_load` was a disposable test added
to the baseline tree only for this capture. It is not in any commit.

## Old lint refusal

Source: `check_planet` in `crates/nova_scenario/src/lint/scenario.rs` at
`8a85df12a`, `"invulnerable"` branch (line 1309).

```rust
"invulnerable" => format!(
    "planet '{id}' authors `invulnerable: false`, and there is no destructible \
     planet: the body would take no damage marks, emit no collision events and \
     never fire OnDestroyed. Author `invulnerable: true`"
),
```

## Old runtime refusal

Source: `planet_scenario_object_prepared` in
`crates/nova_scenario/src/objects/planet.rs` at `8a85df12a` (lines 138-143).

```rust
if !config.invulnerable {
    error!(
        "planet: `invulnerable: false` is not a destructible planet, it is a planet \
         that quietly cannot be destroyed; nothing spawned"
    );
    return;
}
```

Behavior: loading logged the error and returned before inserting any planet
component. The spawned entity had no `PlanetMarker`.
