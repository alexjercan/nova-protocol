# L9 - nova_gameplay four-way split

**Baseline: BLOCKS - lands AFTER it.** The bulk of the epic and its highest
risk.

Findings: **F53** (subsumed by rule 10), **F81**.

**Depends on:** L2, **L4** (fix the NOVAOS defects before the lines move),
**L5** (the rule-10 set count is 16 only after `TweenSystems` and
`StatusBarPluginSystems` die), and **L8** (the gate must be trustworthy first).

## The four seams

```
CORE  <-  FLIGHT  <-  HUD  <-  NOVAOS
```

Dependencies point left. **Cut in the order NOVAOS -> HUD -> FLIGHT -> CORE** -
outermost first, so each cut is against a base that has not moved yet.

| Seam | Roughly | Notes |
| --- | --- | --- |
| NOVAOS | `hud/nova_os*`, ~14.3k lines | a terminal runtime that is not a HUD. **Densest defect cluster and the biggest navigability win** |
| HUD | the rest of `hud/` | 43% of the crate today, minus NOVAOS |
| FLIGHT | `flight/`, `sections/`, `input/`, `camera/`, `physics/` | the sound simulation core |
| CORE | `math/`, components, shared markers | everything the other three import |

## Resolve the three back-edges FIRST

Before any file moves. Each is a lower layer reaching up:

```rust
// crates/nova_gameplay/src/camera/framing.rs:200
//   MOVE the helper it needs into `math` (which is CORE and already moving).

// crates/nova_gameplay/src/sections/controller_section.rs:301
//   INVERT the scheduling edge - the dependency is on ordering, not on data.

// crates/nova_gameplay/src/plugin.rs:107,111,115
//   LIFT into the assembly crate. The plugin that wires four crates together
//   belongs above all four, not inside one of them.
```

## CONVENTIONS.md rule 10 - and it is not free

The owner ruled that **every** subsystem plugin declares a `SystemSet` and
orders it. Measured: 98 plugins, 30 sets, 21 `configure_sets` calls, and only
**14 sets ever passed to one**. So **68 plugins need a new set** and **16
existing sets need an ordering they have never had**.

**This lands here, per seam, and nowhere earlier.** Sixty-eight ordering
decisions made across `nova_gameplay` as it stands today are sixty-eight
decisions re-made the moment the crate is cut four ways, because "what runs
before what, across this boundary" is precisely the question the seam forces.
Done *as* the split, each new crate's `configure_sets` block is the artifact
that proves the seam is real and the order is intentional.

```rust
// The shape, once per plugin, per seam:
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum <Subsystem>Systems { /* the phases this plugin actually has */ }

impl Plugin for <Subsystem>Plugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, <Subsystem>Systems::X.after(<Upstream>Systems::Y));
        app.add_systems(Update, (..).in_set(<Subsystem>Systems::X));
    }
}
```

The **16 declared-but-unordered sets** are the natural first slice and the
cheapest evidence the rule is workable:

`DirectionalSphereOrbitSystems`, `HudSituationSensing`, `IntegritySystems`,
`NovaOsMapSystems`, `NovaOsShipSystems`, `ObjectivesPluginSystems`,
`PointRotationSystems`, `SmoothLookRotationSystems`, `SpaceshipTargetingSystems`,
`SphereOrbitSystems`, `SphereRandomOrbitSystems`, `StatusBarPluginSystems`,
`TempEntitySystems`, `TurretSectionAimSystems`, `TweenSystems`,
`WASDCameraControllerSystems`.

**Two retire for free in L5** - `TweenSystems` dies with the `Tween` subsystem
(F45) and `StatusBarPluginSystems` sits in the same file as F46/F51. `Objectives
PluginSystems` goes with F48. **Do the L5 deletions before counting the
remaining set work.**

Rule 9's two renames (`HudSituationSensing`, `CameraAuthority` -> `*Systems`)
also land in L5, so this lane inherits the corrected names.

### F53 is subsumed

```rust
// crates/nova_gameplay/src/hud/nova_os_ship/mod.rs:166
// crates/nova_gameplay/src/hud/nova_os_map/mod.rs:139
//   NovaOsShipSystems / NovaOsMapSystems are declared as SystemSets and NEVER
//   passed to configure_sets - verified, zero references outside their own
//   defining file, not even a prelude re-export. They have no ordering edge to
//   NovaHudSystems, which owns both the producer and the consumer of what they
//   write. Whether a `ship repair` result row appears this frame or next is
//   decided by bevy's arbitrary topological order; the peek_pending_invocation
//   dance at nova_os_ship/app.rs:195 exists BECAUSE OF THIS.
```

The measurement shows F53 is not two sites, it is 16. Fixing it is the NOVAOS
seam's first `configure_sets` block. **Watch `peek_pending_invocation`:** once
the ordering is real, that workaround may be deletable - which is the kind of
deletion success criterion #2 is looking for.

## F81 - one `SystemParam`, two suppressions, one duplication

```rust
// crates/nova_gameplay/src/hud/nova_os_map/scene.rs:259   map_input
// crates/nova_gameplay/src/hud/nova_os_ship/scene.rs:336  ship_input
//   an IDENTICAL 6-param cluster, each behind #[allow(clippy::too_many_arguments)]
```

```rust
// NEW  the shared param - #[derive(SystemParam)] is already the local idiom
//      (nova_os_ship/sections.rs:223 ShipSections)
#[derive(SystemParam)]
pub struct NovaOsAppInput<'w, 's> { /* the 6 */ }
```

Removes two suppressions and a duplication with one struct - **and that struct
has to be placed on one side of the NOVAOS seam regardless**, so doing it
before the split means doing it twice.

Note F34 (`ship_input` bypassing the Control guard) is in the **same function**
and lands in L4, before this. Expected: L4 fixes the behavior, L9 moves the
signature.

## Folded in for free

**The 633 crate-local `pub` items** (`../notes/02-workspace-map.md`;
`nova_gameplay` holds 358, ~55%). Splitting four ways forces each seam to
decide what crosses its boundary, so the visibility audit is work the split
does anyway rather than a separate pass. **Truly dead items: zero** - this is a
"tighten what is public", not a "delete what is unused".

**Rules 3 and 4 - 26 module preludes**, the same edit as the visibility audit:
deciding what goes in a module's prelude *is* deciding what crosses its
boundary. Costs nothing extra in the same pass, costs a second full read of the
crate if deferred. `math` alone accounts for 5 of the deep-import violations
and is already moving here (`camera/framing.rs:200`).

## Verified by

**`probe run --all` per seam. Not once at the end.**

**The benchmark does NOT rerun per seam - owner ruling 2026-08-07.** It runs
twice in the whole epic: the L2 baseline and one final run the owner starts and
runs by hand. Re-keying happens once, immediately before that final run - see
`lane02.md`. Accepted cost: a seam that is not paying for itself is not visible
until the epic is over.

**Re-keying is L2's work, not this lane's**, but this lane is what invalidates
the key. `_coverage` in `keys/tier1.json` maps question ids to areas;
`nova_os_hud_seam` is 5 of the 30 and NOVAOS is the first seam cut. Note as you
go which questions your moves invalidate, so the single re-keying pass is not a
reconstruction from memory.
