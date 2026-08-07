# Clippy and lint audit

Measured 2026-08-07 on `master` @ `4a8b55aa`, rustc/clippy 0.1.98
(2026-07-02), via `nix develop --command cargo clippy`.

Doc lints were switched OFF for this audit per the owner's instruction
("disable comment warnings"): `-A missing_docs_in_private_items`,
`-A doc_markdown`, `-A missing_errors_doc`, `-A missing_panics_doc`,
`-A missing_safety_doc`. Also `-A module_name_repetitions` (fights the
`nova_os::NovaOsPlugin` naming the repo already chose).

## Headline: the default lint set is already clean

```
cargo clippy --workspace --all-targets --features debug
-> 0 warnings, exit 0
```

**Zero.** Not "a few". The tree passes stock clippy across every crate, test
and example.

This changes the CI gap recorded in `08-tests-ci-risk.md`. That note said
"clippy runs without `-D warnings`, so warnings never fail CI". True, but the
implied cost - a cleanup pass before the flag can be added - **does not
exist**. Adding `-D warnings` to `.github/workflows/ci.yaml` is a one-line
change that passes today.

**Do it before the benchmark baseline.** During a large refactor, unused
imports and dead code are the most common fallout, and both are warnings. Right
now they land green.

The one caveat is a dependency, not this code:

```
warning: the following packages contain code that will be rejected by a
future version of Rust: proc-macro-error2 v2.0.1
```

Transitive, via a proc-macro crate. Not blocking, but `-D warnings` does not
cover it, so it needs its own tracking.

## Pedantic + nursery: 3,998 warnings

The interesting number. Triaged into three buckets by lint, then sampled by
reading sites.

### Bucket 1 - REJECT. Hostile to Bevy or to this repo's chosen idiom. 3,528 hits.

| Lint | Hits | Why it is wrong here |
| --- | --- | --- |
| `needless_pass_by_value` | 1,366 | Fires on **every Bevy system parameter**. `Res<T>`, `On<Add, T>`, `Query<..>` and `Commands` are taken by value because the `SystemParam` API requires it. Sampled 8: `controller_section.rs:379` `add: On<Add, ControllerSectionMarker>`, `merge.rs:47` `catalogs: Res<Assets<InstalledCatalog>>`, `combat.rs:99` `damage: On<HealthApplyDamage>`. All 8 mandatory. Uncodeable-around |
| `redundant_pub_crate` | 1,270 | Fires on `pub(crate)` inside a private module, telling you to write `pub`. This repo deliberately writes `pub(crate)` to state crate-internal intent at the item. Sampled 6: `nova_os/components.rs:16`, `mods.rs:63`, `style.rs:61`. The suggestion is strictly worse - it loses the intent and would leak through any future `pub mod` |
| `too_long_first_doc_paragraph` | 822 | A doc lint. Out of scope by the owner's instruction; listed only so the count is accounted for |
| `must_use_candidate` | 690 | Would add `#[must_use]` to ~690 signatures. Pure annotation churn |
| `use_self` | 489 | Cosmetic |
| `missing_const_for_fn` | 311 | Nursery, high false-positive rate, and `const fn` is not a goal here |
| `return_self_not_must_use` | 86 | Same class as `must_use_candidate` |

The two leaders alone are 2,636 of 3,998 - **66% of the pedantic output is
noise from two lints.** Any future "turn on pedantic" proposal must allow
these two, or the signal is unreadable.

### Bucket 2 - REAL SIGNAL. Worth reading each site. 51 hits in `src/`.

Test and example hits stripped out; these are all production paths.

| Lint | src hits | Assessment |
| --- | --- | --- |
| `float_cmp` | 9 | See below. One genuine modding footgun, the rest defensible |
| `cast_possible_truncation` | 21 | Sampled 4, all guarded. See below |
| `cast_sign_loss` | 16 | Overlaps the above - same lines flagged twice |
| `cast_possible_wrap` | 1 | `component_lock.rs:204,209` |
| `redundant_clone` | 14 | Free wins, several in per-frame HUD systems |
| `needless_pass_by_ref_mut` | 5 | **Bevy-relevant, see below** |
| `suspicious_operation_groupings` | 1 | Verified false positive, see below |

### Bucket 3 - STYLE, feeds CONVENTIONS.md rather than a fix pass

`map_unwrap_or` 190, `suboptimal_flops` 205, `redundant_closure_for_method_calls`
87, `option_if_let_else` 65, `uninlined_format_args` 52, `wildcard_imports` 47,
`single_match_else` 47, `items_after_statements` 32,
`semicolon_if_nothing_returned` 33, `explicit_iter_loop` 29.

These are exactly the "is this our house style?" questions
`conventions-prompt.md` exists to answer. **Do not fix them in this epic** -
route the list to the conventions workstream and let the owner rule. Any rule
adopted there arrives with a free violation count from this run.

`too_many_lines` (47 src hits) is a size signal, not a style one - it
independently corroborates the size outliers in `03`/`05`/`06`.

`format_push_string` (90 hits) is almost entirely `nova_probe`'s HTML and
report writers (`run_report/html.rs`, `report.rs`, `aggregate.rs`). It is the
correct shape for a string builder; not a defect.

## The findings worth acting on

### `needless_pass_by_ref_mut` - 5 src sites, and this one matters in Bevy

```
crates/nova_gameplay/src/hud/chip_layout_rig.rs:278
crates/nova_gameplay/src/input/ai/behavior.rs:909
crates/nova_gameplay/src/input/targeting/component_lock.rs:403
crates/nova_gameplay/src/input/targeting/radar.rs:387
crates/nova_gameplay/src/sections/turret_section/aim.rs:510
```

Plus `examples/sections/controller_section.rs:463` and
`thruster_section.rs:314`.

In ordinary Rust this is a nit. In Bevy it is not: a parameter declared `&mut`
that is never used mutably, if it reaches a system signature, declares a write
that the scheduler must serialize against every other access to that data.
The cost is lost parallelism and **spurious ambiguity** between systems that
do not actually conflict. Worth confirming whether these five are helper
functions or system params before dismissing.

### `float_cmp` - one genuine footgun

`crates/nova_scenario/src/variables.rs:270`:

```rust
(VariableLiteral::Number(l), VariableLiteral::Number(r)) => Ok(l == r),
```

This is the `Equal` node of the **scenario condition language** - the
vocabulary mod authors write in RON. Exact float equality means a mod author
writing `Equal(hull_fraction, 0.5)` against any computed value will see the
condition essentially never fire, with no error and no warning. It is
untyped-language behavior surfacing as a silent no-op.

Needs an owner decision, not a mechanical fix: epsilon compare, an explicit
`ApproxEqual` node, or documented as-is. It is also a **benchmark question
candidate** for the modder persona.

The other 8 are defensible - dirty-check guards comparing a value against the
value it was just assigned (`slider.rs:217`, `key_glyphs.rs:163-168`), or
comparisons against exact preset constants (`settings.rs:457,461`, in a test).

### The casts are guarded

Sampled the four highest-risk. All correct:

- `settings.rs:233-234,502-503,517-518` `render_target_size` - the fraction is
  `.clamp(MIN_RENDER_SCALE, 1.0)` first and each axis takes `.max(1)`, so the
  cast is on a bounded positive f32 and cannot produce the zero-area target
  that would be a fatal wgpu error. The comment at `:227-229` states exactly
  this. **Model site.**
- `nova_ui/src/widget/slider.rs:26` - `fraction.clamp(0.0, 1.0) * SEGMENTS`.
- `nova_ui/src/status_bar.rs:334` - FPS from `DiagnosticsStore`, non-negative.

**Verdict: `cast_*` is not a cleanup target.** 37 of the 51 "real signal"
hits are these, and the sample says they are fine. Do not spend the epic on
them. `nova_probe/src/stats.rs:167,379` (percentile math in the CI gate) is
the one place still worth a read - assigned to the probe reviewer.

### `suspicious_operation_groupings` - false positive

`crates/nova_gameplay/src/hud/key_glyphs.rs:166`. The lint sees
`image.image == self.image && image.rect == self.cap && node.width == .. &&
node.height == ..` and expects field symmetry across the chain. The chain
deliberately spans two different objects. Not a bug.

The other hit is in a test file.

### Lower-priority confirmed items

- `while_float` x2, both in tests (`nova_os_map/tests.rs:842`,
  `nova_os_ship/tests.rs:1316`). A float loop condition in a test can spin
  forever on a NaN. Cheap to make integer-counted.
- `iter_with_drain` at `mesh/explode.rs:200` - `drain(..)` where `into_iter()`
  was meant; the `Vec` is kept and reused. Verify that is intentional.
- `case_sensitive_file_extension_comparisons` at
  `nova_probe/src/run_report/artifacts.rs:81` - artifact discovery by
  extension. Irrelevant on Linux CI, real on a case-insensitive filesystem.
- `redundant_clone` in per-frame HUD systems: `flight_status.rs:204`,
  `torpedo_target.rs:180`, `turret_lead.rs:222`, `damage_tint.rs:473,638`,
  `nova_os_map/scene.rs:104`, `nova_os_ship/scene.rs:213`. Free allocations
  every frame. 14 sites, mechanical.

## Recommended actions, ranked

| # | Action | Cost | Why |
| --- | --- | --- | --- |
| 1 | Add `-D warnings` to the CI clippy step | one line, passes today | Stops refactor fallout landing green. Must precede the benchmark baseline |
| 2 | Owner ruling on `variables.rs:270` float equality | a decision, then 1 file | The only user-visible correctness defect clippy found |
| 3 | Read the 5 `needless_pass_by_ref_mut` sites | 30 min | Cheap, and in Bevy it is a scheduling fact, not a nit |
| 4 | Route bucket 3 to the conventions workstream | zero - hand over the list | Each candidate rule arrives with a violation count |
| 5 | Fix the 14 `redundant_clone` sites | mechanical | Per-frame allocations |
| 6 | Add a `--target wasm32-unknown-unknown` check job | one job, passes today after item 7 | Currently zero coverage of a target that ships |
| 7 | Gate `nova_probe/src/report.rs` behind `cfg(not(wasm32))` | 1 line | Clears all 7 wasm warnings; the gate roster in `lib.rs:82-109` already does this for its siblings |
| 8 | `cfg(feature = "debug")` the 11 dead example items, then add a default-features job | ~20 min | Unblocks the second CI gap |
| 9 | Track `proc-macro-error2 v2.0.1` future-incompat | separate task | Not this code, will break on a rustc bump |

**Explicitly NOT recommended:** turning on `clippy::pedantic` in CI. 66% of its
output here is `needless_pass_by_value` and `redundant_pub_crate`, both of
which are wrong for a Bevy codebase.

## Reproducing

```sh
nix develop --command cargo clippy --workspace --all-targets --features debug \
  --message-format json -- \
  -W clippy::pedantic -W clippy::nursery \
  -A clippy::missing_docs_in_private_items -A clippy::doc_markdown \
  -A clippy::missing_errors_doc -A clippy::missing_panics_doc \
  -A clippy::missing_safety_doc -A clippy::module_name_repetitions
```

Then `jq -r '.message | select(.code != null) | .code.code'` to rebuild the
histogram. Respect the `jobs = 4` cap in `.cargo/config.toml`.

## The two CI blind spots, now measured

`08-tests-ci-risk.md` predicted both would be non-zero. Tested rather than
assumed.

### Default features - CONFIRMED, 11 warnings

```
cargo clippy --workspace --all-targets     (no --features debug)
-> 11 warnings, exit 0
```

Every one is dead code in `examples/`, unreachable once `debug` is off:

| Site | Warning |
| --- | --- |
| `examples/sections/hull_section.rs:535,547,563` | `com_snapshot`, `attached_section_moment`, `camera_anchor` never used |
| `examples/sections/torpedo_section.rs:69,349` | `CROSSING_ID`, `crossing_range` never used |
| `examples/sections/controller_section.rs:64` | variant `B` never constructed |
| `examples/screenshots/screenshot_combat.rs:128,134` | `RAIDER_BLOWN_SECTION`, `RAIDER_BLAST_SECTION` never used |
| `examples/screenshots/screenshot_sections.rs:199` | `CAMERA_BEARING` never used |
| `examples/systems/player_path.rs:55` | `ROUNDS` never used |
| `examples/stress/many_sections.rs:37` | unused imports `ComputedCenterOfMass`, `ComputedMass` |

**Consequence for sequencing.** `-D warnings` is free on the *debug* job (0
warnings) but would fail a default-features job on these 11. Two options:
gate the new job on `--workspace --lib` only, or `#[cfg(feature = "debug")]`
the 11 items first. The second is ~20 minutes and is the honest fix - these
items exist *only* to serve debug-feature code, and the cfg says so.

Note this is real signal, not noise: it is exactly the class of fallout a
refactor produces (an item orphaned behind a cfg), and it is invisible today.

### wasm32 - NOT CONFIRMED. It compiles clean.

```
cargo check -p nova_assets -p nova_probe --target wasm32-unknown-unknown
-> exit 0, 7 warnings, no errors
```

All 14 workspace crates were pulled in and checked: nova_os, nova_ui,
nova-protocol, nova_autopilot, nova_events, nova_gameplay, nova_scenario,
nova_modding, nova_assets, nova_menu, nova_editor, nova_core, nova_probe.

**`08-tests-ci-risk.md` and `05-assets-scenario.md` both implied the wasm
paths were probably rotten. They are not.** The uncompiled-code worry was
reasonable but is not what the evidence says. `persist.rs`, `mod_cache.rs`
and the whole `portal/` stack type-check on wasm today.

What that does and does not buy:

- It DOES retire the "wasm code has silently bit-rotted" risk. The
  `Storage`-trait refactor in `05` is now justified by testability and gate
  removal alone, not by a latent-breakage argument.
- It does NOT mean the wasm paths are correct. Type-checking is not behavior.
  `persist.rs:149-153` still admits the backend is guarded by static review
  only, and no test runs it. The reviewer covering nova_assets was asked to
  read these paths as unreviewed code.

The 7 warnings are one cluster - **the entirety of
`crates/nova_probe/src/report.rs` is dead on wasm**: `run_renderer:19`,
`split_label:31`, `escape:41`, `render_chart:52`, `render_table:138`,
`delta_cell:200`, `STYLE:215`. The module is compiled into the wasm target
and nothing calls it, so it wants a `cfg(not(target_arch = "wasm32"))` gate
like its siblings in `lib.rs:82-109`. An oversight in the existing gate
roster, not a design flaw.

The `wasm32-unknown-unknown` std ships in the pinned toolchain, so this job
costs CI build time and no new tooling. Caveat on the measurement: this was
`cargo check` on libs, not `clippy --all-targets`. A clippy job may surface
more.
