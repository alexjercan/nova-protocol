//! The world-state snapshot: WHAT THE WORLD LOOKS LIKE right now, as one JSON
//! object - the counterpart to [`timeline`](super::timeline), which records
//! what HAPPENED.
//!
//! [`capture_snapshot`] is the whole serializer and it does no IO: give it a
//! `&mut World` and it returns the object. Everything else in this module is
//! plumbing around that one function - an env-gated sink, and the triggers that
//! decide when to call it. A future headless "JSON mode" that reads actions on
//! stdin and emits state on stdout is the same call with a different sink.
//!
//! ## What is in a snapshot
//!
//! A header (`schema`, `scenario`, `frame`, `elapsed`, `t_real`, `game_state`,
//! `reason`), then:
//!
//! - `ships` - every [`SpaceshipRootMarker`]: identity, transform, velocity,
//!   aggregate health, mass, the collapse/defeat/neutralize flags, weapon
//!   locks, its `skin`, and its `sections`.
//! - `ships[].skin` - the DERIVED SKIN as a whole, for a clad ship: the relief
//!   histogram, how many plates have a flat top rather than a cone, how many
//!   are the diagonal saddle, the mean flat area, the per-rule decoration
//!   tally, and every cell the derivation refused to clad with the reason. Null
//!   for a ship wearing none.
//! - `ships[].sections` - every [`SectionMarker`] child: id, prototype, class,
//!   local pose, health, alive/disabled, the `modifications` applied to it, its
//!   `weapon` state when it carries one, and its `fixtures`.
//! - `ships[].sections[].fixtures` - every [`SectionFixture`] hanging off that
//!   section (skin plates, decor), with health and alive. A skin plate also
//!   carries `plate`: the eight boundary samples, the relief they read as, and
//!   every zone fact the decoration scatter filtered on. A decoration carries
//!   `stands_on`, which is the plate under it.
//! - `ordnance` - every torpedo and turret round in flight: owner, position,
//!   velocity, damage, remaining lifetime, target.
//!
//! ## Why the skin, and why in this much detail
//!
//! Because it is the one subsystem a dump makes REPRODUCIBLE. The skin is a
//! pure function of structure - no RNG, the decoration hashed off the cell - so
//! the eight samples on a plate record are a complete repro of that tile, and a
//! defect can be pinned in a test instead of photographed. The derivation is
//! re-run here over the ship's live sections rather than read back off the
//! plates, which is also the only way to report a cell that carries NO plate.
//!
//! ## Why JSON, and why it diffs clean
//!
//! JSON, not RON: a snapshot is machine output nobody authors, the rest of the
//! probe wire format is already JSON, and the eventual stdout mode is a JSON
//! object per frame. RON stays the authoring format for content people write.
//!
//! Two snapshots of one world must be BYTE-IDENTICAL or the artifact is
//! useless for diffing, so:
//!
//! - **Order is value-derived.** Every list is sorted by a natural key with the
//!   serialized record as the tie-break ([`ordered`]). Never entity id, never
//!   query or hash order - a respawn renumbers entities and must not churn the
//!   diff.
//! - **Floats are rounded** to [`SNAPSHOT_DECIMALS`] decimals, and `-0.0` is
//!   normalized to `0.0`. Four decimals is 0.1 mm at world scale and about
//!   0.01 degrees on a quaternion, which is finer than anything worth reading
//!   and coarse enough to swallow the last bit of an `f32`.
//! - **Entities are named, never numbered.** A referenced entity (a lock, a
//!   projectile owner, a torpedo's target) resolves to its scenario
//!   [`EntityId`] or its [`Name`], and to `null` when it has neither.
//!
//! ## Arming and triggering
//!
//! Inert unless `NOVA_PERF_SNAPSHOT` names a `.jsonl` sink (native only - the
//! browser has no filesystem). One snapshot is one line, appended and flushed,
//! so a run that panics keeps everything it took. Three triggers:
//!
//! | Trigger | Set | Fires |
//! |---------|-----|-------|
//! | `NOVA_PERF_SNAPSHOT_FRAMES` | comma-separated [`FrameCount`] values | once per LISTED value, so `600,600` takes two snapshots of one frozen frame |
//! | [`SNAPSHOT_KEY`] | (always, when a keyboard exists) | on key press, for a hand-run game |
//! | [`probe_snapshot`] | (always) | wherever a script, an autopilot beat, or a driver calls it |
//!
//! Per-frame emission needs no rewrite: call [`probe_snapshot`] every frame and
//! the sink already appends one line each.

/// Glob-import surface for the world-state snapshot capability.
pub mod prelude {
    pub use super::{
        capture_snapshot, nova_snapshot, parse_snapshots, probe_snapshot, ProbeSnapshots,
        SnapshotPlugin, SnapshotSchedule, SNAPSHOT_DECIMALS, SNAPSHOT_FRAMES_PARAM, SNAPSHOT_KEY,
        SNAPSHOT_PARAM, SNAPSHOT_SCHEMA,
    };
}

use std::{
    fs::{File, OpenOptions, TryLockError},
    io::{BufWriter, Write},
    path::PathBuf,
};

use avian3d::prelude::{AngularVelocity, ComputedMass, LinearVelocity};
use bevy::{diagnostic::FrameCount, prelude::*};
use nova_events::prelude::{EntityId, EntityTypeName};
use nova_gameplay::{
    prelude::{
        Allegiance, DefeatedMarker, Health, HealthZeroMarker, IntegrityDisabledMarker,
        NeutralizedMarker, ProjectileDamage, ProjectileOwner, SectionClass, SectionMarker,
        SpaceshipRootMarker, TempEntity, TempEntityState, TorpedoProjectileMarker,
        TurretBulletProjectileMarker,
    },
    GameStates,
};
use nova_scenario::{
    prelude::{
        CurrentScenario, SectionAmmoOverride, SectionHealthOverride, SectionRename,
        SpaceshipController,
    },
    world::NovaEventWorld,
};
use nova_ship::prelude::{
    derive_skin, muzzle_aim_error, read_plates, read_structure, section_cell, skin_report,
    skin_summary, AITarget, CombatLock, GameStyles, PlacedPart, PlateReport, PointDefenseMount,
    SectionAmmo, SectionExit, SectionFixture, SectionLinkPoints, SectionReload, ShipDecorMarker,
    ShipSkin, ShipSkinMarker, ShipStyle, SkinReport, StructuralCollapseMarker, TorpedoArming,
    TorpedoBlast, TorpedoSectionInput, TorpedoTargetEntity, TorpedoTargetPosition, TorpedoType,
    TravelLock, TurretDefenseTarget, TurretSectionAimPoint, TurretSectionInput,
    TurretSectionMuzzleEntity, TurretSectionTargetInput, WeaponsHot, WithheldVerbs,
    TURRET_ON_TARGET_RAD,
};

use crate::capabilities::{frametime::prelude::*, timeline::stamp};

/// Wire-format version of a snapshot object. Bump it when a field changes
/// meaning or disappears; adding a field does not need a bump, because a reader
/// that does not know a key ignores it.
pub const SNAPSHOT_SCHEMA: u32 = 1;

/// Decimals every float in a snapshot is rounded to. See the module docs for
/// why the number is fixed rather than "whatever the float prints as".
pub const SNAPSHOT_DECIMALS: i32 = 4;

/// Param (via [`perf_param`], so `NOVA_PERF_SNAPSHOT` on native) naming the
/// JSONL sink that arms [`nova_snapshot`].
pub const SNAPSHOT_PARAM: &str = "snapshot";

/// Param (`NOVA_PERF_SNAPSHOT_FRAMES` on native) listing the [`FrameCount`]
/// values to snapshot at, comma-separated. A repeated value snapshots that
/// frame that many times, which is how a run proves its own determinism.
pub const SNAPSHOT_FRAMES_PARAM: &str = "snapshot_frames";

/// Key that takes a snapshot in a hand-run game (armed runs only).
pub const SNAPSHOT_KEY: KeyCode = KeyCode::F9;

/// Env-gated world-snapshot preset. Inert unless `NOVA_PERF_SNAPSHOT` names a
/// sink (or an explicit [`out`](SnapshotPlugin::out) override is given, which
/// tests use to avoid process-global env races). See the module docs.
pub fn nova_snapshot() -> SnapshotPlugin {
    SnapshotPlugin {
        out: None,
        frames: None,
    }
}

/// Plugin returned by [`nova_snapshot`]. Construct it through that preset.
pub struct SnapshotPlugin {
    out: Option<PathBuf>,
    frames: Option<Vec<u32>>,
}

impl SnapshotPlugin {
    /// Force the sink path, bypassing the env lookup. For tests (env vars are
    /// process-global and race across parallel tests) and for callers that
    /// already resolved a run directory.
    pub fn out(mut self, path: impl Into<PathBuf>) -> Self {
        self.out = Some(path.into());
        self
    }

    /// Force the frame schedule, bypassing the env lookup.
    pub fn at_frames(mut self, frames: impl IntoIterator<Item = u32>) -> Self {
        self.frames = Some(frames.into_iter().collect());
        self
    }
}

impl Plugin for SnapshotPlugin {
    fn build(&self, app: &mut App) {
        crate::contract::declare(app, crate::contract::Capability::Snapshot);
        let Some(path) = self
            .out
            .clone()
            .or_else(|| perf_param(SNAPSHOT_PARAM).map(PathBuf::from))
        else {
            return;
        };
        let sink = match ProbeSnapshots::create(path) {
            Ok(sink) => sink,
            Err(error) => {
                // ERROR, not WARN, for the same reason the timeline's failure
                // is: the artifact was explicitly asked for, and `log_clean`
                // greps whole-word ERROR, so a lost sink fails the run instead
                // of thinning it.
                error!("nova probe: snapshot disabled: {error}");
                return;
            }
        };
        let frames = self.frames.clone().unwrap_or_else(parse_frame_schedule);
        info!(
            "nova probe: snapshot armed -> {:?} (frames {:?}, key {:?})",
            sink.path, frames, SNAPSHOT_KEY
        );
        app.insert_resource(sink);
        app.insert_resource(SnapshotSchedule(frames));
        // Last: after physics, transform propagation and every gameplay
        // system, so the snapshot is the state the frame ENDED in rather than
        // a half-stepped one.
        app.add_systems(Last, capture_triggered_snapshots);
    }
}

/// The [`FrameCount`] values an armed run snapshots at, in the order given.
///
/// A repeated value is honored rather than deduplicated: two snapshots of one
/// frozen frame is the determinism check, and it is the only way to take it.
#[derive(Resource, Debug, Clone, Default)]
pub struct SnapshotSchedule(pub Vec<u32>);

/// Parse `NOVA_PERF_SNAPSHOT_FRAMES`. A malformed entry is dropped with a
/// warning rather than failing the run: the sink is still armed and the key
/// still works.
fn parse_frame_schedule() -> Vec<u32> {
    let Some(raw) = perf_param(SNAPSHOT_FRAMES_PARAM) else {
        return Vec::new();
    };
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| match entry.parse::<u32>() {
            Ok(frame) => Some(frame),
            Err(error) => {
                warn!("nova probe: snapshot frame {entry:?} ignored: {error}");
                None
            }
        })
        .collect()
}

/// The live JSONL sink. Present only while the capability is armed, so
/// [`probe_snapshot`] can treat its absence as "not armed".
#[derive(Resource)]
pub struct ProbeSnapshots {
    sink: BufWriter<File>,
    path: PathBuf,
    written: u64,
}

impl ProbeSnapshots {
    fn create(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        // SINGLETON GUARD, then truncate - the ordering the timeline sink
        // learned the hard way: a plain `File::create` truncates to offset 0
        // while an earlier writer's `BufWriter` keeps writing at its own
        // offset, splicing two streams into one line.
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("could not create {}: {e}", path.display()))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(format!(
                    "another run is already writing {} - refusing to share the path",
                    path.display()
                ))
            }
            Err(TryLockError::Error(e)) => {
                return Err(format!("could not lock {}: {e}", path.display()))
            }
        }
        file.set_len(0)
            .map_err(|e| format!("could not truncate {}: {e}", path.display()))?;
        Ok(Self {
            sink: BufWriter::new(file),
            path,
            written: 0,
        })
    }

    /// Append one snapshot as a line and FLUSH it, so a panic a frame later
    /// cannot lose it.
    fn record(&mut self, snapshot: &serde_json::Value) {
        let line = snapshot.to_string();
        if let Err(error) = writeln!(self.sink, "{line}").and_then(|()| self.sink.flush()) {
            warn!("nova probe: snapshot write failed: {error}");
        }
        self.written += 1;
    }
}

/// Capture a snapshot and append it to the armed sink. A no-op when the
/// capability is not armed, so scripts call it unconditionally:
///
/// ```ignore
/// nova_probe::probe_snapshot(world, "beat: after the first salvo");
/// ```
pub fn probe_snapshot(world: &mut World, reason: &str) {
    if world.get_resource::<ProbeSnapshots>().is_none() {
        return;
    }
    let snapshot = capture_snapshot(world, reason);
    world.resource_mut::<ProbeSnapshots>().record(&snapshot);
}

/// Take the snapshots this frame's triggers ask for.
///
/// Exclusive because [`capture_snapshot`] reads the whole world; it costs
/// nothing on a frame that snapshots nothing, and the capability is not even
/// wired unless the run is armed.
fn capture_triggered_snapshots(world: &mut World) {
    let frame = world.resource::<FrameCount>().0;
    let scheduled = world
        .resource::<SnapshotSchedule>()
        .0
        .iter()
        .filter(|listed| **listed == frame)
        .count();
    for _ in 0..scheduled {
        probe_snapshot(world, "frame");
    }
    if world
        .get_resource::<ButtonInput<KeyCode>>()
        .is_some_and(|keys| keys.just_pressed(SNAPSHOT_KEY))
    {
        probe_snapshot(world, "key");
    }
}

/// Parse a whole JSONL snapshot file back into objects, preserving order.
/// Blank lines are skipped; a line that is not a snapshot is an error naming
/// its number, so a corrupt file is caught instead of silently dropping one.
pub fn parse_snapshots(contents: &str) -> Result<Vec<serde_json::Value>, String> {
    contents
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .filter(|value| {
                    value
                        .get("schema")
                        .and_then(serde_json::Value::as_u64)
                        .is_some()
                })
                .ok_or_else(|| format!("malformed snapshot line {}: {line:?}", i + 1))
        })
        .collect()
}

/// Serialize the whole world as one snapshot object. Pure: no IO, no world
/// mutation beyond the query state bevy builds to read it.
///
/// `reason` is recorded verbatim in the header so a multi-line sink says why
/// each line is there.
pub fn capture_snapshot(world: &mut World, reason: &str) -> serde_json::Value {
    let (t_real, frame, elapsed) = stamp(
        &world.resource::<Time<Real>>().clone(),
        world.resource::<FrameCount>(),
        world.get_resource::<NovaEventWorld>(),
    );
    let scenario = world
        .get_resource::<CurrentScenario>()
        .and_then(|current| current.0.as_ref().map(|config| config.id.clone()));
    let game_state = world
        .get_resource::<State<GameStates>>()
        .map(|state| format!("{:?}", state.get()));

    let mut q_ships = world.query_filtered::<Entity, With<SpaceshipRootMarker>>();
    let ship_entities: Vec<Entity> = q_ships.iter(world).collect();
    let mut q_ordnance = world.query_filtered::<Entity, Or<(
        With<TorpedoProjectileMarker>,
        With<TurretBulletProjectileMarker>,
    )>>();
    let ordnance_entities: Vec<Entity> = q_ordnance.iter(world).collect();

    let ships = ordered(
        ship_entities
            .into_iter()
            .map(|entity| ship_record(world, entity))
            .collect(),
    );
    let ordnance = ordered(
        ordnance_entities
            .into_iter()
            .map(|entity| ordnance_record(world, entity))
            .collect(),
    );

    serde_json::json!({
        "schema": SNAPSHOT_SCHEMA,
        "reason": reason,
        "scenario": scenario,
        "game_state": game_state,
        "frame": frame,
        "elapsed": elapsed,
        "t_real": t_real,
        "ships": ships,
        "ordnance": ordnance,
    })
}

/// Sort `records` into a total, value-derived order and drop the keys.
///
/// The key is the record's natural id; its serialized form breaks a tie. So the
/// order never falls back to entity id or query order, and two snapshots of one
/// world diff clean even after a respawn renumbered every entity.
fn ordered(mut records: Vec<(String, serde_json::Value)>) -> Vec<serde_json::Value> {
    records.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.to_string().cmp(&b.1.to_string()))
    });
    records.into_iter().map(|(_, record)| record).collect()
}

/// Round to [`SNAPSHOT_DECIMALS`] and normalize `-0.0`, so one state always
/// prints the same bytes. NaN and infinity become `null`: JSON cannot spell
/// them, and a snapshot must not panic on a physics blow-up.
fn num(value: f32) -> serde_json::Value {
    let scale = 10f64.powi(SNAPSHOT_DECIMALS);
    let rounded = (f64::from(value) * scale).round() / scale;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    serde_json::Number::from_f64(rounded).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

fn vec3(value: Vec3) -> serde_json::Value {
    serde_json::json!([num(value.x), num(value.y), num(value.z)])
}

/// A cell or a cardinal direction. Whole numbers, so nothing is rounded and
/// [`num`] does not apply.
fn ivec3(value: IVec3) -> serde_json::Value {
    serde_json::json!([value.x, value.y, value.z])
}

fn quat(value: Quat) -> serde_json::Value {
    serde_json::json!([num(value.x), num(value.y), num(value.z), num(value.w)])
}

fn health(value: Option<&Health>) -> serde_json::Value {
    value.map_or(
        serde_json::Value::Null,
        |health| serde_json::json!({ "current": num(health.current), "max": num(health.max) }),
    )
}

/// How a referenced entity is written: its scenario [`EntityId`], else its
/// [`Name`], else `null`. Never the entity index - that is not stable across
/// runs and would churn every diff.
fn label(world: &World, entity: Entity) -> serde_json::Value {
    world
        .get::<EntityId>(entity)
        .map(|id| id.0.clone())
        .or_else(|| world.get::<Name>(entity).map(ToString::to_string))
        .map_or(serde_json::Value::Null, serde_json::Value::String)
}

/// `label` for an optional reference, flattening "no target" and "target is
/// gone" into the same `null` - both mean nothing is being pointed at.
fn label_of(world: &World, entity: Option<Entity>) -> serde_json::Value {
    entity.map_or(serde_json::Value::Null, |entity| label(world, entity))
}

/// A string that identifies a record for sorting. Falls back to the empty
/// string, which sorts first and is then broken by the record itself.
fn key(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// One ship, keyed by its scenario object id.
fn ship_record(world: &World, entity: Entity) -> (String, serde_json::Value) {
    let id = label(world, entity);
    let transform = world.get::<Transform>(entity).copied().unwrap_or_default();
    let skin = skin_index(world, entity);
    let sections = ordered(
        world
            .get::<Children>(entity)
            .map(|children| {
                children
                    .iter()
                    .filter(|child| world.get::<SectionMarker>(*child).is_some())
                    .map(|child| section_record(world, child, skin.as_ref()))
                    .collect()
            })
            .unwrap_or_default(),
    );

    let record = serde_json::json!({
        "id": id,
        "name": world.get::<Name>(entity).map(ToString::to_string),
        "allegiance": world.get::<Allegiance>(entity).map(|a| format!("{a:?}")),
        "controller": world.get::<SpaceshipController>(entity).map(controller_kind),
        "position": vec3(transform.translation),
        "rotation": quat(transform.rotation),
        "linear_velocity": world.get::<LinearVelocity>(entity).map(|v| vec3(v.0)),
        "angular_velocity": world.get::<AngularVelocity>(entity).map(|v| vec3(v.0)),
        // The ship root's own Health IS the aggregate: `aggregate_ship_health`
        // sums the living sections into it every frame, and pins `max` at the
        // running maximum so a destroyed section keeps its share of the
        // denominator.
        "health": health(world.get::<Health>(entity)),
        "mass": world.get::<ComputedMass>(entity).map(|m| num(m.value())),
        "collapsing": world.get::<StructuralCollapseMarker>(entity).is_some(),
        "defeated": world.get::<DefeatedMarker>(entity).is_some(),
        "neutralized": world.get::<NeutralizedMarker>(entity).is_some(),
        "weapons_hot": world.get::<WeaponsHot>(entity).map(|hot| hot.0),
        "travel_lock": label_of(world, world.get::<TravelLock>(entity).and_then(|lock| lock.0)),
        "combat_lock": label_of(world, world.get::<CombatLock>(entity).and_then(|lock| lock.0)),
        "ai_target": label_of(world, world.get::<AITarget>(entity).and_then(|target| target.0)),
        // The derived skin as a whole: the histogram, the measurements and the
        // cells it refused. Per-plate detail hangs off the plate's own fixture
        // record; this is the half no single plate can answer.
        "skin": skin.map(|skin| skin.summary),
        "sections": sections,
    });
    (key(&record["id"]), record)
}

/// One ship's derived skin, indexed the way a fixture record asks for it.
///
/// The dump RE-DERIVES rather than reading the plates back off the ship, and it
/// can: the skin is a pure function of the structure, so the derivation over a
/// live ship's sections is the same one that clad it. That is also the only way
/// to answer the other half - a cell that carries NO plate has no entity to
/// hang a record on, and "why is this hull face bare" is the question the dump
/// was asked for.
struct SkinIndex {
    /// Where the cell lattice this ship's sections stand on has its origin.
    phase: Vec3,
    /// One record per clad cell.
    rows: std::collections::HashMap<IVec3, serde_json::Value>,
    /// The ship-level summary.
    summary: serde_json::Value,
}

impl SkinIndex {
    /// The cell a live plate stands in, off its pose in its section's frame.
    ///
    /// Never off the plate's own translation alone: the derivation works in the
    /// SHIP's cells and a plate is parented to the section it clads.
    fn cell_of(&self, world: &World, plate: Entity, section: Entity) -> Option<IVec3> {
        let local = world.get::<Transform>(plate)?;
        let pose = world.get::<Transform>(section)?;
        Some(section_cell(
            pose.rotation * local.translation + pose.translation,
            self.phase,
        ))
    }
}

/// Derive `root`'s skin and index it, or `None` for a ship that wears none.
fn skin_index(world: &World, root: Entity) -> Option<SkinIndex> {
    if !world.get::<ShipSkin>(root).is_some_and(|skin| skin.0) {
        return None;
    }
    let sections: Vec<(&Transform, &SectionLinkPoints, Option<&SectionExit>)> = world
        .get::<Children>(root)?
        .iter()
        .filter(|child| world.get::<SectionMarker>(*child).is_some())
        .filter_map(|child| {
            Some((
                world.get::<Transform>(child)?,
                world.get::<SectionLinkPoints>(child)?,
                world.get::<SectionExit>(child),
            ))
        })
        .collect();
    let placed: Vec<PlacedPart> = sections
        .iter()
        .map(|(pose, sockets, exit)| PlacedPart {
            position: pose.translation,
            rotation: pose.rotation,
            link_points: sockets.as_slice(),
            exit: exit.map(|exit| exit.0),
        })
        .collect();

    let (structure, phase, _) = read_structure(&placed);
    if structure.is_empty() {
        return None;
    }
    let plates = derive_skin(&structure);
    let readings = read_plates(&structure, &plates);
    let style = world
        .get::<ShipStyle>(root)
        .zip(world.get_resource::<GameStyles>())
        .and_then(|(worn, styles)| worn.resolve(styles));
    let report = skin_report(&structure, &plates, &readings, style);

    Some(SkinIndex {
        phase,
        rows: report
            .plates
            .iter()
            .map(|row| (row.cell, plate_record(row)))
            .collect(),
        summary: skin_record(&report),
    })
}

/// The ship-level half of a skin dump: what the hull came out as, and what it
/// refused.
fn skin_record(report: &SkinReport) -> serde_json::Value {
    let histogram = |counted: &[nova_ship::prelude::ReliefCount]| {
        serde_json::Value::Object(
            counted
                .iter()
                .map(|count| {
                    (
                        count.relief.name().to_string(),
                        serde_json::json!(count.count),
                    )
                })
                .collect(),
        )
    };
    serde_json::json!({
        // The one line a human reads; every number in it is also a field.
        "summary": skin_summary(report),
        "plates": report.plates.len(),
        "shapes": report.shapes,
        "relief": histogram(&report.relief),
        // The measurements the shell-shape work is sized against: how many
        // plates have a whole flat top rather than a cone, how many are the
        // diagonal saddle, and how much room the average plate offers a piece.
        "coplanar": report.coplanar,
        "saddles": report.saddles,
        "leaning": report.leaning,
        "flat_area": num(report.flat_area),
        // STRUCTURE FACING VACUUM. The coverage rule's claim is that this is
        // zero, so anything else is a defect and the cells below say which.
        "bare_faces": report.bare_faces,
        "bare": report.bare.iter().map(|bare| serde_json::json!({
            "cell": ivec3(bare.cell),
            "reason": bare.reason.name(),
            "faces": bare.faces,
        })).collect::<Vec<_>>(),
        "decor": {
            "placed": report.decor,
            "off_flat": report.decor_off_flat,
            "on_creased": report.decor_on_creased,
            // How raked the ground under the decoration is, in radians. A piece
            // is bedded onto its plate's own top normal, so this reads the HULL
            // the pieces landed on rather than how far any of them leans off
            // its own footing.
            "tilt": num(report.decor_tilt),
            "relief": histogram(&report.decor_relief),
            "rules": report.rules.iter().map(|rule| serde_json::json!({
                "id": rule.id,
                "taken": rule.taken,
                "reach": rule.reach,
            })).collect::<Vec<_>>(),
        },
    })
}

/// One clad cell: the eight samples, what they spell, and every fact the
/// scatter read on its way to dressing it.
fn plate_record(row: &PlateReport) -> serde_json::Value {
    let reading = &row.reading;
    serde_json::json!({
        "cell": ivec3(row.cell),
        "anchor": ivec3(row.anchor),
        "out": ivec3(row.out),
        // The eight boundary samples, in the order the shape's id spells them.
        // These ARE the shape: given them, the tile is reproducible offline.
        "corners": row.shape.corners,
        "midpoints": row.shape.midpoints,
        "relief": reading.relief.name(),
        // Which corner slots die to the cell floor. The derivation puts one
        // there for exactly one reason - open space stands at it - so this is
        // the number of directions the plate has vacuum in, and the two
        // diagonal masks are the saddle.
        "fallen": row.fallen,
        "saddle": row.saddle,
        // Whether the fanned top is ONE surface or a cone, and how much of the
        // cell the widest flat piece of it covers.
        "coplanar": row.coplanar,
        "flat_area": num(row.flat_area),
        "tilt": num(row.tilt),
        // The zone facts a scatter rule filters on.
        "along": ivec3(reading.along),
        "fall": ivec3(reading.fall),
        "run": reading.run,
        "border": reading.border,
        "enclosure": reading.enclosure,
        "height": reading.height,
        "depth": reading.depth,
        "fitting": reading.fitting,
        // What landed here, and by which of the two paths.
        "decor": row.decor.iter().map(|piece| serde_json::json!({
            "id": piece.id,
            "reason": piece.reason.name(),
            "turns": piece.turns,
        })).collect::<Vec<_>>(),
    })
}

/// The controller kind, without its config: `None`, `Player` or `AI`.
fn controller_kind(controller: &SpaceshipController) -> &'static str {
    match controller {
        SpaceshipController::None => "None",
        SpaceshipController::Player(_) => "Player",
        SpaceshipController::AI(_) => "AI",
    }
}

/// One section, keyed by its ship-local id.
fn section_record(
    world: &World,
    entity: Entity,
    skin: Option<&SkinIndex>,
) -> (String, serde_json::Value) {
    let transform = world.get::<Transform>(entity).copied().unwrap_or_default();
    let class = world.get::<SectionClass>(entity);
    let record = serde_json::json!({
        // The ship-local slot id; `prototype` is the catalog key it resolved.
        "id": label(world, entity),
        "prototype": world.get::<EntityTypeName>(entity).map(|name| name.0.clone()),
        "name": world.get::<Name>(entity).map(ToString::to_string),
        "class": class.map(|class| format!("{class:?}")),
        "position": vec3(transform.translation),
        "rotation": quat(transform.rotation),
        "health": health(world.get::<Health>(entity)),
        "alive": world.get::<HealthZeroMarker>(entity).is_none(),
        // Dead but still holding the ship together: it stopped working without
        // cutting the integrity graph.
        "disabled": world.get::<IntegrityDisabledMarker>(entity).is_some(),
        "modifications": modifications(world, entity),
        "weapon": weapon(world, entity, class),
        "fixtures": fixtures(world, entity, skin),
    });
    (key(&record["id"]), record)
}

/// The live components a `SectionModification` leaves behind, back in the
/// authored spelling. Reading the components rather than the ship's authored
/// list is deliberate: a snapshot reports the WORLD, and the two disagree the
/// moment anything else writes one of them.
fn modifications(world: &World, entity: Entity) -> Vec<serde_json::Value> {
    let mut applied = Vec::new();
    if let Some(health) = world.get::<SectionHealthOverride>(entity) {
        applied.push(serde_json::json!({ "SetHealth": num(health.0) }));
    }
    if let Some(name) = world.get::<SectionRename>(entity) {
        applied.push(serde_json::json!({ "Rename": name.0.clone() }));
    }
    if let Some(ammo) = world.get::<SectionAmmoOverride>(entity) {
        applied.push(serde_json::json!({ "SetAmmo": ammo.0 }));
    }
    if let Some(withheld) = world.get::<WithheldVerbs>(entity) {
        // A HashSet: sort the spellings, never the iteration order.
        let mut verbs: Vec<String> = withheld.0.iter().map(|verb| format!("{verb:?}")).collect();
        verbs.sort();
        for verb in verbs {
            applied.push(serde_json::json!({ "DisableVerb": verb }));
        }
    }
    applied
}

/// How far a turret's PRIMARY muzzle points off its own aim point, in degrees,
/// or `None` for a section that has no muzzle or nothing to aim at.
///
/// The trigger says what the pilot (or the AI) WANTS; this says whether the
/// barrel can deliver it, and the section fire path spends a round only when
/// this is inside [`TURRET_ON_TARGET_RAD`]. Without it a dump cannot tell a
/// mount that is shooting from one that is holding fire mid-slew - the muzzle's
/// bearing appears nowhere else in a snapshot.
fn muzzle_aim_error_deg(world: &World, entity: Entity) -> Option<f32> {
    let aim = world
        .get::<TurretSectionAimPoint>(entity)
        .and_then(|a| a.0)?;
    let muzzle = world.get::<TurretSectionMuzzleEntity>(entity)?.0;
    let pose = world.get::<GlobalTransform>(muzzle)?;
    Some(muzzle_aim_error(pose.forward().into(), pose.translation(), aim).to_degrees())
}

/// A weapon section's live state, or `null` for a section that is not one.
///
/// Magazine and reload live on the SECTION entity; a section with no
/// [`SectionAmmo`] fires without limit, which is `null` rather than zero.
fn weapon(
    world: &World,
    entity: Entity,
    class: Option<&SectionClass>,
) -> Option<serde_json::Value> {
    let kind = match class? {
        SectionClass::Turret => "turret",
        SectionClass::Torpedo => "torpedo_bay",
        _ => return None,
    };
    let firing = world
        .get::<TurretSectionInput>(entity)
        .map(|input| input.0)
        .or_else(|| {
            world
                .get::<TorpedoSectionInput>(entity)
                .map(|input| input.0)
        });
    let aim_error = muzzle_aim_error_deg(world, entity);
    Some(serde_json::json!({
        "kind": kind,
        // `firing` is the TRIGGER; `on_target` is whether the barrel bears.
        // Rounds leave only when both hold, so the pair is what answers "did
        // this mount fire while off target".
        "firing": firing,
        "aim_error_deg": aim_error.map(num),
        "on_target": aim_error.map(|error| error <= TURRET_ON_TARGET_RAD.to_degrees()),
        "ammo": world.get::<SectionAmmo>(entity).map(|ammo| serde_json::json!({
            "rounds": ammo.rounds,
            "capacity": ammo.capacity,
        })),
        "reload": world.get::<SectionReload>(entity).map(|reload| serde_json::json!({
            "elapsed": num(reload.elapsed),
            "delay": num(reload.delay),
            "amount": reload.amount,
        })),
        // A turret aims at a POINT, not an entity: `target` is the world-space
        // point it was told to hit, `aim_point` the lead solution it steers
        // for. `defense_target` is the one entity a mount holds - the inbound
        // torpedo its point-defense is assigned to - and `authority` says WHO
        // is working the mount, which on a player hull is the whole question
        // (a fired round with `authority: "FlightComputer"` is the ship
        // defending itself, not the player shooting).
        "target": world
            .get::<TurretSectionTargetInput>(entity)
            .and_then(|target| target.0)
            .map(vec3),
        "aim_point": world
            .get::<TurretSectionAimPoint>(entity)
            .and_then(|aim| aim.0)
            .map(vec3),
        "defense_target": label_of(
            world,
            world.get::<TurretDefenseTarget>(entity).and_then(|target| target.0),
        ),
        "authority": world
            .get::<PointDefenseMount>(entity)
            .map(|mount| format!("{:?}", mount.authority)),
    }))
}

/// Every fixture hanging off `section`, at any depth (a greeble bolted to a
/// plate is a fixture on a fixture), keyed by kind + name.
///
/// Recursion does not stop at a fixture and does not cross into another
/// section: sections are children of the ship ROOT, never of each other, so a
/// walk from one section can only reach its own dressing and render children.
fn fixtures(world: &World, section: Entity, skin: Option<&SkinIndex>) -> Vec<serde_json::Value> {
    let mut found = Vec::new();
    collect_fixtures(world, section, skin, None, &mut found);
    ordered(found)
}

/// `cell` is the clad cell the walk is currently INSIDE: `None` at the section,
/// and the plate's own cell everywhere under a plate. That is what lets a
/// greeble say which plate it stands on - its own pose is on top of the plate's
/// surface rather than at the middle of the cell, so nothing about its
/// transform alone would answer.
fn collect_fixtures(
    world: &World,
    parent: Entity,
    skin: Option<&SkinIndex>,
    cell: Option<IVec3>,
    found: &mut Vec<(String, serde_json::Value)>,
) {
    let Some(children) = world.get::<Children>(parent) else {
        return;
    };
    for child in children.iter() {
        let cell = match world.get::<ShipSkinMarker>(child).is_some() {
            true => skin.and_then(|skin| skin.cell_of(world, child, parent)),
            false => cell,
        };
        if world.get::<SectionFixture>(child).is_some() {
            found.push(fixture_record(world, child, parent, skin, cell));
        }
        collect_fixtures(world, child, skin, cell, found);
    }
}

/// One fixture: what it is, what it hangs off, where it sits in its parent's
/// frame, and whether it is still there.
///
/// `shape` is a skin plate's [`ShellShape`](nova_ship::prelude::ShellShape) id,
/// and it is the ANCHOR the skin dump extends from: `plate` carries the eight
/// samples that spell that shape, the relief they read as, and every zone fact
/// the scatter used. One serializer, not two.
///
/// `plate` is null on a plate whose cell the CURRENT structure no longer derives
/// one for - a section died and took the neighbourhood with it, while the plate
/// beside it survived. That is a real state and the null says so rather than
/// inventing a reading.
///
/// A decoration answers `stands_on` instead: the plate under it, and whether
/// that plate is a surface or a crease. It is the placement complaint in one
/// field.
fn fixture_record(
    world: &World,
    entity: Entity,
    parent: Entity,
    skin: Option<&SkinIndex>,
    cell: Option<IVec3>,
) -> (String, serde_json::Value) {
    let plate = world.get::<ShipSkinMarker>(entity);
    let decor = world.get::<ShipDecorMarker>(entity).is_some();
    let kind = if plate.is_some() {
        "skin_plate"
    } else if decor {
        "skin_decor"
    } else {
        "fixture"
    };
    let row = cell.zip(skin).and_then(|(cell, skin)| skin.rows.get(&cell));
    let transform = world.get::<Transform>(entity).copied().unwrap_or_default();
    let record = serde_json::json!({
        "kind": kind,
        "name": world.get::<Name>(entity).map(ToString::to_string),
        "attached_to": label(world, parent),
        "shape": plate.map(|plate| plate.0.id()),
        "position": vec3(transform.translation),
        "rotation": quat(transform.rotation),
        "health": health(world.get::<Health>(entity)),
        "alive": world.get::<HealthZeroMarker>(entity).is_none(),
        "plate": plate.map(|_| row.cloned().unwrap_or(serde_json::Value::Null)),
        "stands_on": decor.then(|| row.map_or(serde_json::Value::Null, |row| serde_json::json!({
            "cell": row["cell"],
            "relief": row["relief"],
            "coplanar": row["coplanar"],
            "flat_area": row["flat_area"],
            "tilt": row["tilt"],
        }))),
    });
    (format!("{kind}/{}", key(&record["name"])), record)
}

/// One round or torpedo in flight, keyed by kind + owner.
///
/// The two carry damage differently and the record says so rather than
/// flattening it: a round has an authored [`ProjectileDamage`], a torpedo has a
/// [`TorpedoBlast`] it only spends when it fuzes, always as `Explosive`.
fn ordnance_record(world: &World, entity: Entity) -> (String, serde_json::Value) {
    let torpedo = world.get::<TorpedoProjectileMarker>(entity).is_some();
    let kind = if torpedo { "torpedo" } else { "bullet" };
    let transform = world.get::<Transform>(entity).copied().unwrap_or_default();
    let owner = label_of(
        world,
        world.get::<ProjectileOwner>(entity).map(|owner| owner.0),
    );
    let damage = if let Some(blast) = world.get::<TorpedoBlast>(entity) {
        serde_json::json!({
            "kind": "Explosive",
            "amount": num(blast.damage),
            "radius": num(blast.radius),
        })
    } else if let Some(damage) = world.get::<ProjectileDamage>(entity) {
        serde_json::json!({
            "kind": format!("{:?}", damage.kind),
            "amount": num(damage.amount),
            "power": num(damage.power),
            "layers": damage.layers,
        })
    } else {
        serde_json::Value::Null
    };

    let record = serde_json::json!({
        "kind": kind,
        // Torpedo only: WHICH ordnance type this is. Two bays on one hull can
        // load different torpedoes, and every other field here - owner, damage,
        // lifetime - is identical between them, so without this a snapshot
        // cannot tell a Lance from a Serpent.
        "type": world
            .get::<TorpedoType>(entity)
            .map(|torpedo_type| torpedo_type.name.clone()),
        "owner": owner,
        "allegiance": world.get::<Allegiance>(entity).map(|a| format!("{a:?}")),
        "position": vec3(transform.translation),
        "velocity": world.get::<LinearVelocity>(entity).map(|v| vec3(v.0)),
        "damage": damage,
        "lifetime": world.get::<TempEntity>(entity).map(|total| serde_json::json!({
            "total": num(total.0),
            "remaining": world
                .get::<TempEntityState>(entity)
                .map(|state| num(state.remaining_secs())),
        })),
        // Torpedo only: what it is chasing, where it last saw it, and whether
        // its safety has let go yet.
        "target": label_of(
            world,
            world.get::<TorpedoTargetEntity>(entity).map(|target| target.0),
        ),
        "target_position": world.get::<TorpedoTargetPosition>(entity).map(|at| vec3(at.0)),
        "armed": world.get::<TorpedoArming>(entity).map(TorpedoArming::is_armed),
    });
    (format!("{kind}/{}", key(&record["owner"])), record)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use nova_gameplay::prelude::DamageType;
    use nova_ship::prelude::{unit_cube_link_points, SectionReloadConfig};

    use super::*;

    /// A unique temp path per test (no clock: process id + counter).
    fn temp_sink() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "nova_probe_snapshot_{}_{n}.jsonl",
            std::process::id()
        ))
    }

    fn rig() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<FrameCount>();
        app
    }

    /// A ship with one turret section carrying a magazine and one skin plate,
    /// plus a round of its own in flight. Every field group the schema claims,
    /// spawned the way production spawns it (markers, not bundles - the
    /// bundles need the whole asset stack).
    fn ship(app: &mut App, id: &str, at: Vec3) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new(id),
                Name::new(id.to_string()),
                Allegiance::Player,
                SpaceshipController::None,
                Transform::from_translation(at),
                Health {
                    current: 120.0,
                    max: 200.0,
                },
                WeaponsHot(true),
            ))
            .id();
        let mut reload = SectionReload::from_config(SectionReloadConfig {
            delay: 4.0,
            amount: 5,
        });
        reload.elapsed = 1.5;
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                SectionClass::Turret,
                EntityId::new(format!("{id}_turret")),
                EntityTypeName::new("pdc_turret_section"),
                Name::new("Turret"),
                Transform::from_xyz(0.0, 1.0, -2.0),
                Health {
                    current: 130.0,
                    max: 130.0,
                },
                SectionAmmo {
                    rounds: 7,
                    capacity: 20,
                },
                reload,
                TurretSectionInput(true),
                ChildOf(root),
            ))
            .id();
        app.world_mut().spawn((
            SectionFixture,
            Name::new("Skin Plate a"),
            Transform::from_xyz(0.5, 0.0, 0.0),
            Health {
                current: 8.0,
                max: 12.0,
            },
            ChildOf(section),
        ));
        app.world_mut().spawn((
            TurretBulletProjectileMarker,
            ProjectileOwner(root),
            Transform::from_translation(at + Vec3::Z),
            ProjectileDamage {
                amount: 4.0,
                power: 1.0,
                layers: 3,
                kind: DamageType::Kinetic,
            },
            TempEntity(3.0),
        ));
        root
    }

    /// A clad ship: one hull cube, and the six plates the derivation lays on
    /// it, spawned the way `spawn_ship_skin` spawns them (children of the
    /// section, posed in ITS frame).
    fn clad_ship(app: &mut App, at: Vec3) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("clad"),
                ShipSkin(true),
                Transform::from_translation(at),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                EntityId::new("clad_hull"),
                SectionLinkPoints(unit_cube_link_points()),
                Transform::IDENTITY,
                ChildOf(root),
            ))
            .id();
        for cell in [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ] {
            // Every face of a lone cube comes out a STUD, whatever way it
            // faces, so one shape covers the six.
            let stud = nova_ship::prelude::ShellShape::new([0; 4], [0; 4]).expect("a legal shape");
            app.world_mut().spawn((
                SectionFixture,
                ShipSkinMarker(stud),
                Name::new(format!("Skin Plate {}", stud.id())),
                Transform::from_translation(cell.as_vec3()),
                ChildOf(section),
            ));
        }
        root
    }

    /// The skin half of the dump: a plate carries the eight samples that spell
    /// its shape, the relief they read as, and the zone facts the scatter used,
    /// and the ship carries the histogram and the cells the derivation refused.
    ///
    /// The samples are the load-bearing part. The skin is a pure function of
    /// the structure, so a dumped shape is a COMPLETE repro - the tile can be
    /// rebuilt offline from these eight digits and pinned in a test.
    #[test]
    fn a_clad_ship_carries_its_derivation_per_plate_and_a_summary_per_ship() {
        let mut app = rig();
        clad_ship(&mut app, Vec3::ZERO);
        app.update();

        let snapshot = capture_snapshot(app.world_mut(), "test");
        let ship = &snapshot["ships"][0];
        let skin = &ship["skin"];
        assert_eq!(skin["plates"], 6, "a lone cube is clad on all six faces");
        assert_eq!(skin["shapes"], 1, "and every face of it is the same stud");
        assert_eq!(skin["relief"]["peak"], 6);
        assert_eq!(skin["coplanar"], 0, "a stud is a pyramid, not a surface");
        assert_eq!(skin["bare_faces"], 0, "nothing on a lone cube is bare");
        assert_eq!(skin["decor"]["placed"], 0, "it wears no style");
        assert!(skin["summary"]
            .as_str()
            .is_some_and(|line| line.contains("6 plate(s)")));

        let plate = ship["sections"][0]["fixtures"]
            .as_array()
            .expect("the plates are fixtures")
            .iter()
            .find(|fixture| fixture["plate"]["cell"] == serde_json::json!([0, 1, 0]))
            .expect("a plate stands on the roof of the cube");
        assert_eq!(plate["kind"], "skin_plate");
        assert_eq!(plate["shape"], "shell_0000_0000");
        assert_eq!(plate["plate"]["corners"], serde_json::json!([0, 0, 0, 0]));
        assert_eq!(plate["plate"]["midpoints"], serde_json::json!([0, 0, 0, 0]));
        assert_eq!(plate["plate"]["relief"], "peak");
        assert_eq!(plate["plate"]["out"], serde_json::json!([0, 1, 0]));
        assert_eq!(plate["plate"]["anchor"], serde_json::json!([0, 0, 0]));
        assert_eq!(
            plate["plate"]["fallen"], 15,
            "every corner dies to the floor"
        );
        assert_eq!(plate["plate"]["saddle"], false);
        assert_eq!(plate["plate"]["coplanar"], false);
        assert_eq!(plate["plate"]["enclosure"], 0, "nothing stands beside it");
        assert_eq!(plate["plate"]["depth"], 1, "one cell of hull under it");
        assert_eq!(plate["plate"]["decor"], serde_json::json!([]));

        // A ship that wears no skin says so, rather than carrying an empty one.
        let mut bare = rig();
        self::ship(&mut bare, "player", Vec3::ZERO);
        bare.update();
        assert!(capture_snapshot(bare.world_mut(), "test")["ships"][0]["skin"].is_null());
    }

    /// The skin half diffs clean too: it is derived from the structure rather
    /// than read off entity order, so two dumps of one clad ship are the same
    /// bytes and two ships built back to front are the same bytes.
    #[test]
    fn a_clad_ship_dumps_the_same_bytes_twice_and_in_either_spawn_order() {
        let mut app = rig();
        clad_ship(&mut app, Vec3::ZERO);
        app.update();
        let first = capture_snapshot(app.world_mut(), "test").to_string();
        assert_eq!(first, capture_snapshot(app.world_mut(), "test").to_string());

        let mut second = rig();
        ship(&mut second, "player", Vec3::new(10.0, 0.0, 0.0));
        clad_ship(&mut second, Vec3::ZERO);
        second.update();
        let mut third = rig();
        clad_ship(&mut third, Vec3::ZERO);
        ship(&mut third, "player", Vec3::new(10.0, 0.0, 0.0));
        third.update();
        assert_eq!(
            capture_snapshot(second.world_mut(), "test").to_string(),
            capture_snapshot(third.world_mut(), "test").to_string(),
        );
    }

    #[test]
    fn a_snapshot_carries_the_ship_its_sections_its_fixtures_and_its_ordnance() {
        let mut app = rig();
        ship(&mut app, "player", Vec3::new(1.0, 2.0, 3.0));
        app.update();

        let snapshot = capture_snapshot(app.world_mut(), "test");

        assert_eq!(snapshot["schema"], SNAPSHOT_SCHEMA);
        assert_eq!(snapshot["reason"], "test");
        let ship = &snapshot["ships"][0];
        assert_eq!(ship["id"], "player");
        assert_eq!(ship["allegiance"], "Player");
        assert_eq!(ship["controller"], "None");
        assert_eq!(ship["position"], serde_json::json!([1.0, 2.0, 3.0]));
        assert_eq!(ship["health"]["current"], 120.0);
        assert_eq!(ship["health"]["max"], 200.0);
        assert_eq!(ship["weapons_hot"], true);
        assert_eq!(ship["collapsing"], false);

        let section = &ship["sections"][0];
        assert_eq!(section["id"], "player_turret");
        assert_eq!(section["prototype"], "pdc_turret_section");
        assert_eq!(section["class"], "Turret");
        assert_eq!(section["alive"], true);
        assert_eq!(section["weapon"]["kind"], "turret");
        assert_eq!(section["weapon"]["firing"], true);
        assert_eq!(section["weapon"]["ammo"]["rounds"], 7);
        assert_eq!(section["weapon"]["reload"]["elapsed"], 1.5);

        let fixture = &section["fixtures"][0];
        assert_eq!(fixture["kind"], "fixture");
        assert_eq!(fixture["attached_to"], "player_turret");
        assert_eq!(fixture["health"]["current"], 8.0);
        assert_eq!(fixture["alive"], true);

        let round = &snapshot["ordnance"][0];
        assert_eq!(round["kind"], "bullet");
        assert_eq!(round["owner"], "player");
        assert_eq!(round["damage"]["kind"], "Kinetic");
        assert_eq!(round["damage"]["amount"], 4.0);
        assert_eq!(round["lifetime"]["total"], 3.0);
    }

    /// The whole point of the artifact: one state, one set of bytes. Two
    /// captures of the SAME frame must not differ, or a diff of two runs is
    /// noise.
    #[test]
    fn two_captures_of_one_frame_are_byte_identical() {
        let mut app = rig();
        ship(&mut app, "player", Vec3::ZERO);
        ship(&mut app, "raider", Vec3::new(10.0, 0.0, 0.0));
        app.update();

        let first = capture_snapshot(app.world_mut(), "test").to_string();
        let second = capture_snapshot(app.world_mut(), "test").to_string();

        assert_eq!(first, second);
    }

    /// ...and the bytes must not depend on the order the world happened to
    /// spawn things in. This is what rules out entity id and query order as
    /// the sort key: the two worlds hold the same ships, built back to front.
    #[test]
    fn spawn_order_does_not_change_the_bytes() {
        let mut forward = rig();
        ship(&mut forward, "alpha", Vec3::ZERO);
        ship(&mut forward, "beta", Vec3::new(10.0, 0.0, 0.0));
        forward.update();

        let mut backward = rig();
        ship(&mut backward, "beta", Vec3::new(10.0, 0.0, 0.0));
        ship(&mut backward, "alpha", Vec3::ZERO);
        backward.update();

        assert_eq!(
            capture_snapshot(forward.world_mut(), "test").to_string(),
            capture_snapshot(backward.world_mut(), "test").to_string()
        );
    }

    /// A rounded float and a normalized negative zero: `-0.0` and `0.0` are
    /// the same state and used to print differently, which is a dirty diff for
    /// nothing.
    #[test]
    fn floats_are_rounded_and_negative_zero_is_normalized() {
        assert_eq!(num(1.234_567_9), serde_json::json!(1.2346));
        assert_eq!(num(-0.0), serde_json::json!(0.0));
        assert_eq!(num(0.000_01), serde_json::json!(0.0));
        assert_eq!(num(f32::NAN), serde_json::Value::Null);
        assert_eq!(num(f32::INFINITY), serde_json::Value::Null);
    }

    #[test]
    fn the_sink_appends_one_line_per_snapshot() {
        let path = temp_sink();
        let mut app = rig();
        app.add_plugins(nova_snapshot().out(&path).at_frames([1, 1, 2]));
        ship(&mut app, "player", Vec3::ZERO);

        app.update();
        app.update();
        app.update();

        // FLUSH-PER-LINE PIN: read the file NOW, before any teardown.
        let lines = parse_snapshots(&std::fs::read_to_string(&path).expect("sink exists"))
            .expect("snapshots parse");
        assert_eq!(lines.len(), 3, "two at frame 1, one at frame 2");
        assert_eq!(lines[0]["frame"], 1);
        assert_eq!(lines[1]["frame"], 1);
        assert_eq!(lines[2]["frame"], 2);
        // The determinism proof, taken the way a real run takes it: a repeated
        // frame in the schedule captures the same frozen state twice.
        assert_eq!(lines[0].to_string(), lines[1].to_string());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_unarmed_run_writes_nothing_and_probe_snapshot_is_safe() {
        let mut app = rig();
        app.add_plugins(SnapshotPlugin {
            out: None,
            frames: None,
        });
        app.update();
        assert!(app.world().get_resource::<ProbeSnapshots>().is_none());
        probe_snapshot(app.world_mut(), "beat");
    }

    #[test]
    fn parse_snapshots_rejects_a_line_that_is_not_a_snapshot() {
        let error = parse_snapshots("{\"ships\": []}\n").expect_err("a headerless line is refused");
        assert!(error.contains("malformed snapshot line 1"), "{error}");
        assert!(parse_snapshots("")
            .expect("an empty file is an empty sink")
            .is_empty());
    }
}
