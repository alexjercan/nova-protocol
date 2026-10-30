# Evidence run: implementation-20261030

Fresh, reproducible evidence for the uncommitted mixed wreck canister implementation
(task 20261026-003724), collected as read-only verification. No source file, test file,
REPORT.html, or TASK.md was edited, staged, or committed by this run.

## Provenance

- Base commit (HEAD at run time): `112b79a0d82193e74e8bdb02e0fcbed846ec0c66` (`HEAD.txt`)
- Working tree: dirty, per `git-status-at-run.txt` (same file list as the session's
  starting `git status`)
- Uncommitted change set: `working-tree.diff` (`git diff --binary`), SHA-256
  `dbf75f45d7dd583a9f08a21e3b9f0a400c3e22cdc1b6bb4fffc5810f4a661706`
  (also recorded independently in `working-tree.diff.sha256`; both match)
- Run timestamp (UTC): see `run-timestamp-utc.txt`
- Toolchain: `rust-toolchain.toml.copy` pins `nightly-2026-07-03`; the devshell
  reported `rustc 1.98.0-nightly (c397dae80 2026-07-02)`,
  `cargo 1.98.0-nightly (a335d47ff 2026-06-26)` (not separately logged to a file,
  recorded here from the live `nix develop --command rustc/cargo --version` output)
- `nix --version`: see `nix-version.txt`
- Host: see `uname.txt`
- Build isolation: `CARGO_TARGET_DIR=/tmp/nova-verify-20261030-target` (unique,
  not the shared workspace target dir), `cargo test -j2` / `cargo run -j2`
  throughout. No workspace-wide test or Clippy run. No GPU/player capture was
  run; DISPLAY=:0 was never touched. An isolated Xvfb runner (`:97`, PID
  group under `/tmp/nova-18`) was already busy with an unrelated lane's
  `screenshot_goto_moving_target` capture for the whole window of this run, so
  no playable/GPU flow was attempted here. Screenshots are omitted, not
  skipped by choice.

## Commands and results

All commands run from the repository root unless noted; full stdout/stderr in
`logs/`.

### 1. Wreck single-drop 1,300 kg -> 1,000 + 300 kg mixed, FIFO, own ids

```
cargo test -j2 -p nova_scenario --lib \
  wreck_cargo::tests::a_ship_destroyed_whole_drops_its_hold_and_its_sections_parts_in_one_drop \
  -- --exact --nocapture
```
Log: `logs/01-wreck-cargo-1300kg-mixed-fifo.log`
Result: `test ... ok` (1 passed, 0 failed).

Asserted content (from the test source at `crates/nova_scenario/src/wreck_cargo/tests.rs`,
read at run time against the dirty tree, not re-quoted from REPORT.html): a ship
destroyed whole drops 6 drive bells, 12 frame spars, 3 hull plates and 25 iron ore
(960 kg of section-part share plus the hold) in one drop of 1,300 kg. The drop
yields exactly two canisters, taken off the drop's FIFO queue in birth order, each
with its own `CargoCanisterRuntimeId`:
- canister 1: `DriveBell x6, FrameSpar x12, HullPlate x3, IronOre x1` = 1,000,000 g
  exactly (packed in item-id order up to the 1,000 kg cap)
- canister 2: `IronOre x24, PdcRound x300` = 300,000 g
This is the ship-destroyed-whole fixture, reused from the existing wreck_cargo test
suite (not a new test); only its expected-value assertions were edited for the new
cap, per the diff this run also captured.

### 2. 20 kg free partial take; remainder keeps its canister entity and id across freeze/thaw

The task's own reference to "150 kg free" is a modeled number from REPORT.html's
analysis of an earlier whole-fit-only design, not a parameter of any test fixture
found in the dirty tree. The closest, and only, existing partial-pickup fixture
uses a 400 kg hold stocked with 38 x 10 kg hull plates, leaving 20 kg free,
against a 40 kg (4-plate) canister:

```
cargo test -j2 -p nova_ship --lib \
  sections::cargo_intake_section::tests::a_canister_heavier_than_the_free_room_gives_up_the_whole_items_that_fit \
  -- --exact --nocapture
```
Log: `logs/02-cargo-intake-partial-full-dup.log` (section 1)
Result: `test ... ok` (1 passed, 0 failed).

Asserted: with 20 kg free, the intake takes 2 of the 4 plates (20 kg) through the
real trigger-contact path, leaving the canister at `HullPlate x2` under the SAME
entity and the SAME `CargoCanisterRuntimeId`; the door then shuts (hold full). The
remainder is explicitly frozen (`freeze_canister`) and thawed (`thaw_canister`) as
a new entity carrying the same runtime id, and once 2 plates of hold room are
freed, the remainder is taken on a second pass, reporting two separate
`ShipInventoryReport::PickedUp { canister: <same id>, item: HullPlate, count: 2 }`
events. No reproduction of 150 kg free was run; none exists in the fixture set, and
inventing one would be a new test, which this verification does not add.

### 3. Full-hold refusal

```
cargo test -j2 -p nova_ship --lib \
  sections::cargo_intake_section::tests::a_full_hold_takes_nothing_and_keeps_the_door_shut \
  -- --exact --nocapture
```
Log: `logs/02-cargo-intake-partial-full-dup.log` (section 2)
Result: `test ... ok` (1 passed, 0 failed).

Asserted: a ship stocked to its 400 kg cap (40 x 10 kg plates, 0 kg free) beside a
drifting 40 kg canister takes nothing (`takes(&app) == 0`), reports no `PickedUp`,
the canister entity keeps its full 4-plate contents untouched, and the door stays
shut (`door == 0.0`) over 120 frames of real trigger contact.

### 4. Two intakes / two ships, one canister, no duplicate pickup

```
cargo test -j2 -p nova_ship --lib \
  sections::cargo_intake_section::tests::two_intakes_on_one_ship_share_its_holds_room \
  -- --exact --nocapture
cargo test -j2 -p nova_ship --lib \
  sections::cargo_intake_section::tests::two_ships_touching_one_canister_split_it_without_taking_an_item_twice \
  -- --exact --nocapture
```
Log: `logs/02-cargo-intake-partial-full-dup.log` (sections 3-4)
Result: both `test ... ok` (1 passed, 0 failed each).

Asserted: (a) one ship with two intakes both in contact with a mixed
HullPlate x4 + RailSlug x1 canister takes exactly one `PickedUp` of 2 plates
(the room available), leaving `HullPlate x2, RailSlug x1` in the same canister;
the second intake's pass sees a now-full hold and takes nothing. (b) two separate
ships, each with 2 plates of free room, touching one 4-plate canister in the same
frame: each ship takes exactly 2 plates via its own intake, the canister is fully
consumed and despawned, each ship logs one `PickedUp { count: 2 }` under the same
canister id, and `takes(&app) == 2` total, i.e. no plate is taken twice and none
is lost.

### 5. 1,100 kg saved canister over the 1,000 kg cap is refused on load

```
cargo test -j2 -p nova_world_base --lib \
  save::tests::a_world_whose_saved_stock_is_over_its_hold_canister_or_storage_cap_is_refused_on_open \
  -- --exact --nocapture
```
Log: `logs/03-save-1100kg-overcap.log`
Result: `test ... ok` (1 passed, 0 failed).

Asserted: a saved `CargoCanister` RON holding 110 x 10 kg iron ore (1,100,000 g)
is refused on load with the message "the canister 5 holds 1100 kg over the
canister cap of 1000 kg"; the matched within-cap case (100 x 10 kg = 1,000,000 g,
exactly at the new `CARGO_CANISTER_MAX_MASS_G`) loads without that refusal. This
is the existing over-cap refusal fixture, re-parameterized in the dirty tree from
torpedoes/200 kg to iron ore/1,000 kg (see diff), not a new test.

### 6. Supporting unit coverage for the new pack/take_fitting primitives and jettison

```
cargo test -j2 -p nova_gameplay --lib \
  inventory::transfer_tests::packing_fills_each_canister_in_id_order_past_an_item_too_heavy_for_the_room_left \
  -- --exact --nocapture
cargo test -j2 -p nova_gameplay --lib \
  inventory::transfer_tests::taking_from_a_canister_moves_only_whole_items_that_fit_and_keeps_the_rest \
  -- --exact --nocapture
cargo test -j2 -p nova_gameplay --lib \
  inventory::transfer_tests::item_jettison_fills_the_waiting_tail_then_splits_the_rest_by_whole_items \
  -- --exact --nocapture
cargo test -j2 -p nova_gameplay --lib \
  inventory::transfer_tests::item_jettison_plans_refuse_in_order \
  -- --exact --nocapture
```
Log: `logs/04-inventory-pack-take-jettison.log`
Result: all four `test ... ok` (1 passed, 0 failed each).

These are the pure-logic unit tests backing `CargoCanister::pack` and
`take_fitting` (id-order fill, skip-too-heavy, whole-item-only take, remainder
conservation) and the jettison planner re-pointed at the new cap (6 torpedoes
filling 900 kg, 7th alone; 50+50+25 rail slugs; 5000+5000+2500 PDC rounds at the
1,000 kg cap). Reused existing fixtures, re-parameterized for the new cap, per the
diff.

### 7. Content lint against the new 1,000,000 g canister cap

```
cargo run -j2 -- content lint
```
Log: `logs/05-content-lint.log`
Result: `content lint: 0 error(s), 0 warning(s), 0 finding(s), 17 scenario(s)
balance-audited, 0 acked, 23 creative map(s)` (exit 0). Confirms no authored
item (including PDC rounds at 200 g and section panels) exceeds the new
1,000,000 g single-canister limit, and the item-mass-vs-canister-cap lint
(`crates/nova_assets/src/items.rs`) still fires correctly in its own unit test
for an item one gram over the cap (not re-run here as a game check; it is a pure
unit test already covered as part of `nova_assets`'s own lib tests, not run in
this pass because it was not in the five requested claims).

### 8. `rustfmt --check` on every touched source file

```
rustfmt --edition 2021 --check <file>
```
run once per file in:
`crates/nova_gameplay/src/inventory.rs`,
`crates/nova_scenario/src/wreck_cargo.rs`,
`crates/nova_scenario/src/wreck_cargo/tests.rs`,
`crates/nova_ship/src/sections/cargo_intake_section.rs`,
`crates/nova_ship/src/sections/cargo_intake_section/tests.rs`,
`crates/nova_world_base/src/save/tests.rs`,
`crates/nova_assets/src/items.rs`,
`crates/nova_hud/src/pickup_sight.rs`,
`crates/nova_interface/src/inventory/tests.rs`,
`crates/nova_scenario/src/mining.rs`,
`crates/nova_events/src/lib.rs`,
`crates/nova_authoring/src/base_content/lessons.rs`,
`crates/nova_authoring/src/base_content/sections/cargo_intake.rs`

Log: `logs/06-fmt-check.log`
Result: exit 0 for every file (no diff printed). No full `cargo fmt --all` was
run, per AGENTS.md guidance that a crate-root `rustfmt` run reformats children;
each touched leaf file was checked individually with `--edition 2021` instead.

## What was NOT run, and why

- No workspace-wide `cargo test`/`cargo check --workspace` or Clippy: excluded by
  the task's own scope limit and by standing project guidance.
- No `nova-probe` range or `nova-bench` flow: none of the five requested claims
  maps to an existing probe/bench flow closer than the unit/ECS tests used above;
  the task instructed reuse over invention, and these five behaviors are already
  covered as unit/asserted-ECS tests in the dirty tree.
- No player/GPU capture on `DISPLAY=:0`: forbidden by the task.
- No isolated-X-runner capture: the one running isolated Xvfb instance (`:97`)
  was occupied by another lane's unrelated capture for this entire run; forcing a
  second GPU user on the same box would contend with it and risk corrupting both
  results (see memory: measurement lanes must be serialized). Screenshots are
  therefore absent from this evidence set, not silently skipped.
- No 150 kg-free partial-take case: no such fixture exists; see section 2 above
  for the actual, closest existing fixture (20 kg free) and why no new test was
  added.

## Honesty notes

- Every PASS above was read from the literal `test ... ok` / `test result: ok`
  lines in the logs, not inferred from a zero process exit code; exit codes were
  additionally captured where the harness used them, but are secondary to the
  per-test line.
- Test names, fixture numbers (masses, counts, hold capacities) and assertion
  text quoted above were read from the current working-tree source at the paths
  cited, at run time, not copied from REPORT.html or TASK.md prose.
- No secrets, tokens, or paths outside the repository and the isolated
  `/tmp/nova-verify-20261030-target` build directory appear in any captured log.
