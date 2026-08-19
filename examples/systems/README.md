# examples/systems

Correctness ranges, for the probe. A range stages a behavior and PANICS when
the behavior is wrong. `Cargo.toml` owns the category contract - who each of
the three example directories is for. This file owns what happens inside this
one.

## The prefix names the KIND of check

| prefix | the check |
| - | - |
| `system_` | FUNCTIONALITY. Does this subsystem do what it is supposed to do. |
| `bug_` | REGRESSION. One defect that was actually found, held down. |
| `stress_` | LOAD. The same correctness shape at a scale no content reaches. |

The prefix is the kind. The rest is the SUBJECT, and the subject is what the
file calls itself inside - log prefix, scenario id, step labels. Renaming a
range for its kind does not rename what it says about itself.

## Choosing between `system_` and `bug_`

`bug_` is earned. The test is what the ROSTER claims, not why somebody sat
down to write the file:

- the invariants pin ONE defect staying fixed -> `bug_`
- the invariants pin a subsystem behaving -> `system_`

A defect that PROMPTED a range does not make it a `bug_` range. Several
`system_` ranges were written in anger: `system_blast_penetration` reproduced
the old blast path before the fix and is still `system_`, because its
invariants are the rules of the mechanism rather than the one bad measurement
that started it.

Undecidable is `system_`. The stricter category is the one you have to earn.

`stress_` needs no test - you know. Its scale constants are named and carry the
comment that they must NEVER reflect real content.

There is no fourth prefix. The obvious candidate was end-to-end player
JOURNEYS, and the code refuses it: `system_player_path` and `system_outcomes`
stand up the same `AppBuilder` rig on a code-built `ScenarioConfig` that
`system_hull_damage` and every `stress_` range does, while the ranges that walk
the SHIPPED app through `editor_app` - `system_menu_boot`, `bug_menu_picker`,
`system_ship_editor`, `bug_sandbox_soak` - already straddle two kinds. Fixture
choice is not a kind of check.

## Every claim is named

`CONVENTIONS.md` Nova 1. An assert with no name can be deleted into a green run
that proves less. So each one carries a `nova_probe::probe_marker` reading
`outcome: <slug>` beside it, and the slug goes on that range's roster in
`crates/nova_probe_cli/tests/catalog_drift.rs`. That test matches both ways: a
slug with no marker is a deleted assertion, a marker with no slug is a claim
nobody declared, and a new range with no roster fails outright.

A range that only measures is not a range. It may also RECORD - a slug can name
a recorded observation rather than a claim, and `bug_sandbox_soak` has two - but
a range with no assert at all does not belong here.

## Never assert a millisecond

A timing assert is a machine asserting a judgement about the machine. Host load
moves these numbers several times over, CI runs the correctness pass on a small
shared runner under lavapipe, and the result is a false-alarm generator that
someone eventually silences by deleting the assert.

So MEASURE and RECORD. Keep the numbers - they are how the 2 FPS sandbox
collapse was found - as `probe_marker` payloads on the roster, read against a
named reference, and `warn!` past it. Never `assert!`. The probe's own
`fps_within_baseline` is the shape to copy: soft gate, WARN, reviewer judges.
`bug_sandbox_soak` is the worked example.

What a range asserts instead is the hardware-independent fact underneath the
symptom - a collider's SHAPE, a count, a component that is or is not there. If
no such fact survives, say so in the module doc and let the range record. Do not
reach for the stopwatch to have something to fail on.

## These are AUTOPILOT-ONLY

Nobody drives one by hand. Say so in the clap `about` - it is the only
description a run has before it loads - and if a hand-held key survives, say
there too that it is a debug aid and not the subject.

```sh
NOVA_AUTOPILOT=1 cargo run --features debug --example <name>
cargo run --features debug probe run <name>     # one range, graded
cargo run --features debug probe run systems    # the category
```

## The code stays in the file

No shared module, no `systems/common/`. Owner's ruling: duplication is cheaper
than coupling here, for now. Ranges repeat their own `custom_plugin`,
`setup_range` and rig builders, and that is accepted - a range that reads top to
bottom is worth more than one that has to be read against a helper module every
other range also pulls on. Copy it.

A module ONE range owns is fine, as a sibling directory named for that range
and reached with `#[path]` (`system_turret_gunnery/slider.rs`). It moves when
the range is renamed.

## Adding one

1. Name it `<kind>_<subject>`.
2. Write the `[[example]]` block in `Cargo.toml`. Auto-discovery is OFF, so
   without one the file does not build and nothing tells you.
3. Put its slugs on the `catalog_drift.rs` roster and fix `SYSTEMS_INVARIANTS`.
4. Emit each slug as an `outcome:` marker beside its assert.
5. RUN it, and `probe run <name>` green.

Renaming one is Nova 5: nothing type-checks a name. Grep the catalog, the
roster, `docs/`, `web/src/`, `.github/` and every sibling range, then run what
you touched.
