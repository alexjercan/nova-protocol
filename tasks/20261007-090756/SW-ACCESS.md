# SW-ACCESS

Scope: TRANSIENT-GATE.md section 12, "Transient integration (owner,
2026-10-09)" items 1-4. Files touched: `crates/nova_scenario/src/objects/
spaceship.rs`, `crates/nova_ship/src/sections/cargo_intake_section.rs`,
`crates/nova_scenario/src/objects/asteroid_carve.rs`. No other file was
edited. No test was added.

## Item 1: FrozenShip::has_section

Claim: added a `has_section` accessor so `check_saved_ids` can test a
section id against the sections a frozen ship kept.

Evidence: `crates/nova_scenario/src/objects/spaceship.rs:579-594` for
`FrozenShipState.sections: BTreeMap<String, FrozenSection>`.

Change, as written (`spaceship.rs:596-603`):

```rust
impl FrozenShip {
    /// Whether section `section` survived to freeze: whether it still has a
    /// state entry a thaw would spawn.
    #[must_use]
    pub fn has_section(&self, section: &str) -> bool {
        self.state.sections.contains_key(section)
    }
}
```

Blast radius: new public method on an existing type. No existing caller
changed.

## Item 2: FrozenCanister::id

Claim: added an `id` accessor so a dangling-ref check can read a canister
record's durable runtime id without reaching into the struct's private
field.

Evidence: `crates/nova_ship/src/sections/cargo_intake_section.rs:278-283`
for `FrozenCanister.id: CargoCanisterRuntimeId` (the field is `Copy`, see
`crates/nova_gameplay/src/inventory.rs:630-634`).

Change, as written (`cargo_intake_section.rs:285-292`):

```rust
impl FrozenCanister {
    /// The canister's durable runtime id, the key a saved torpedo target
    /// resolves against on thaw.
    #[must_use]
    pub fn id(&self) -> CargoCanisterRuntimeId {
        self.id
    }
}
```

Blast radius: new public method on an existing type. No existing caller
changed.

## Item 3: FrozenRockChunk::validate

Claim: added a `validate` method that rejects the seven named faults
before a thaw ever reads the record, matching the shape
`FrozenTransient::validate` already uses for the `Round`, `Torpedo` and
`ShedFixture` arms, and that `FrozenTransient::validate`'s `RockChunk`
arm (`crates/nova_world_base/src/save/transients.rs:132`, main worker's
file, not touched here) already calls as `chunk.validate()?`.

Evidence: `FrozenRockChunk` at `asteroid_carve.rs:1049-1062` (`surface`,
`mesh: FrozenChunkMesh`, `translation`, `rotation`, `scale`, `linear`,
`angular`, `grace: Option<f32>`); `FrozenChunkMesh` at
`asteroid_carve.rs:1036-1040` (`positions`, `normals`, `indices`).

Change, as written (`asteroid_carve.rs:1063-1124`):

```rust
impl FrozenRockChunk {
    /// # Errors
    ///
    /// A non-finite pose or velocity; a non-finite or negative `grace`; an
    /// empty mesh; a mesh whose positions and normals disagree in length; a
    /// mesh index count that is not a multiple of 3; a mesh index past its
    /// positions; or a non-finite mesh position or normal.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.translation.is_finite()
            && self.rotation.is_finite()
            && self.scale.is_finite()
            && self.linear.is_finite()
            && self.angular.is_finite())
        {
            return Err("a rock chunk's pose or velocity is not finite".to_string());
        }
        if let Some(grace) = self.grace {
            if !(grace.is_finite() && grace >= 0.0) {
                return Err(format!(
                    "a rock chunk's grace of {grace} is not finite and non-negative"
                ));
            }
        }
        if self.mesh.positions.is_empty() {
            return Err("a rock chunk's mesh has no positions".to_string());
        }
        if self.mesh.positions.len() != self.mesh.normals.len() {
            return Err(format!(
                "a rock chunk's mesh has {} positions but {} normals",
                self.mesh.positions.len(),
                self.mesh.normals.len()
            ));
        }
        if self.mesh.indices.len() % 3 != 0 {
            return Err(format!(
                "a rock chunk's mesh has {} indices, which is not a multiple of 3",
                self.mesh.indices.len()
            ));
        }
        let vertex_count = self.mesh.positions.len();
        for &index in &self.mesh.indices {
            if index as usize >= vertex_count {
                return Err(format!(
                    "a rock chunk's mesh index {index} names a vertex past its {vertex_count} positions"
                ));
            }
        }
        if self
            .mesh
            .positions
            .iter()
            .any(|position| !Vec3::from_array(*position).is_finite())
            || self
                .mesh
                .normals
                .iter()
                .any(|normal| !Vec3::from_array(*normal).is_finite())
        {
            return Err("a rock chunk's mesh has a non-finite position or normal".to_string());
        }
        Ok(())
    }
}
```

The index-past-positions message matches the owner's worked example in
form: `"a rock chunk's mesh index 41 names a vertex past its 40
positions"` for `index == 41, vertex_count == 40`.

Order of checks follows the owner's list: pose and velocity, then grace,
then the four mesh shape checks, then the mesh finiteness check.

Blast radius: new public method on an existing type, callable by the main
worker's `FrozenTransient::validate` (already wired to call it) and by
`check_saved_ids` if it wants it. No existing caller changed.

### Named mutation

None of the seven branches is required to fail today: `freeze_rock_chunk`
only ever builds a `FrozenRockChunk` from a live, well-formed chunk, so no
code path in this crate can produce an invalid record to prove the new
`Err` arms against. The branches close the gap for a hand-edited or
corrupted save file read back at world-open.

The open-time test the main worker should add a case to is
`a_world_with_a_duplicate_id_is_refused_on_open`
(`crates/nova_world_base/src/save/tests.rs:997`, main worker's file, not
touched here). It already drives `open_world` over a `cases` table of
`(FrozenSectors, Vec<FrozenTransient>, &str)` and checks
`Err(WorldRefusal::Unreadable(format!("state.{generation}.ron:
{expected}")))`; a `FrozenTransientType::RockChunk` case with, for
example, a zero-length `indices` Vec against one position (indices not a
multiple of 3, or an index at the vertex count) would exercise `validate`
through `WorldRefusal::Unreadable` end to end, the same way the existing
"its owner 'ghost' is not saved" case exercises the dangling-ref path.

## Item 4: comment cleanup on the chunk save code

Removed task/report citations from `asteroid_carve.rs`, kept the reason
in plain words, and updated `rebuild_chunk_mesh`'s doc so its asserts now
read as the thaw's programming-error guard, not the data-validation
layer (that is now `FrozenRockChunk::validate`, called before a thaw ever
reaches it).

- `asteroid_carve.rs:1167`: "the other T3 freezes" -> "the other
  transient freezes".
- `asteroid_carve.rs:1263-1265` (was "see TRANSIENT-GATE.md's 'Rock chunk
  thaw' decision"): citation dropped, sentence rejoined around the plain
  reason that stands on its own (unit-testable against bare `Assets`
  fixtures).
- `asteroid_carve.rs:1730` test section heading: "Rock chunk freeze/thaw
  (P-T7)" -> "Rock chunk freeze/thaw".
- `rebuild_chunk_mesh` doc (`asteroid_carve.rs:1221-1230`): now says
  `FrozenRockChunk::validate` already rejects these shapes before a thaw
  reaches here, so a panic here means the thaw was handed a chunk that
  skipped that check, not a save-time condition. The three `assert!`/
  `assert_eq!` calls in the function body are unchanged.

No remaining line-number references to other files were found in the
touched region; none were added.

## Checks

`cargo test -p nova_scenario --lib asteroid_carve` and `cargo check -p
nova_ship --features serde`: both currently fail to compile, blocked by
an unrelated in-progress edit in `crates/nova_ship/src/sections/
torpedo_section/frozen.rs` (a sibling worker's file, not touched here).
`nova_scenario` depends on `nova_ship`, so both commands fail at the
`nova_ship` compile step before reaching anything this task touched:

```
error[E0308]: mismatched types
  --> crates/nova_ship/src/sections/torpedo_section/frozen.rs:139:9
    (expected `Entity`, found `&_`, in a `let &part = children.iter().find(...)`)

error[E0271]/E0599: `children.iter().copied()` at
  crates/nova_ship/src/sections/torpedo_section/frozen.rs:341:45
  (`Copied` applied twice; does not satisfy `Iterator`)
```

Neither error touches `spaceship.rs`, `cargo_intake_section.rs`, or
`asteroid_carve.rs`. Recommend the owner re-run both checks once
`torpedo_section/frozen.rs` lands.

`cargo check -p nova_world_base --tests`: skipped on the owner's
instruction mid-session (the main worker is editing `nova_world_base` and
it will not compile for a while).

## Unverified

- The three new/edited signatures type-check against the compiler only to
  the extent `cargo fmt` parses them; neither check above reached the
  actual compile of `nova_scenario`'s or `nova_ship`'s own sources past
  the `torpedo_section/frozen.rs` error, so `FrozenShip::has_section`,
  `FrozenCanister::id`, and `FrozenRockChunk::validate` are UNVERIFIED by
  a passing build in this session. `cargo fmt -p nova_scenario -p
  nova_ship` ran clean (no diagnostics) over both crates, which parses
  but does not type-check them.
- No test exercises `FrozenRockChunk::validate`'s `Err` arms; see "Named
  mutation" above.
