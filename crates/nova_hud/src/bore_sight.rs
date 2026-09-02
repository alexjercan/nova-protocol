//! The bore sight: a lance's line of fire, drawn in world space, ending where
//! the slug would end - plus a mark on every section that shot would destroy.
//!
//! A spinal gun has no turret to follow a crosshair, so the SHIP is the aim.
//! Nothing else on the HUD tells you where a hull is pointing, and a lance
//! bolted onto a flank does not point where the nose does. This is the only
//! instrument that answers "where does it go".
//!
//! # The depth read
//!
//! The interesting question a lance poses is not "am I on it" - a bracket
//! answers that - but "does this angle GUT it or clip a corner". So the sight
//! traces the bore and walks the crossed sections through
//! [`pierce_remainder`], the same pure function the round itself is resolved
//! with, spending the authored `slug_power` layer by layer. Every section the
//! shot would destroy gets a ring; the line stops where the slug stops. Aiming
//! down a ship's long axis therefore looks visibly different from catching its
//! shoulder, which is the whole skill of the weapon.
//!
//! Sharing `pierce_remainder` is what keeps the promise honest: a sight with
//! its own copy of the pierce math is a sight that eventually lies. The
//! surface skin each cast restarts past is shared for the same reason - it is
//! `rounds::PIERCE_SKIN`, not a second copy of the same constant.
//!
//! # When it is up
//!
//! Gated on [`WeaponsHot`], which is `raised OR combat-locked` (see
//! `nova_ship`'s weapons safety). That gate is the reason this needs no mode
//! of its own: holding the combat stance freezes the hull's heading, but a
//! COMBAT LOCK keeps the weapons hot with the stance released - so the sight
//! is live in normal flight, where the mouse still steers, and the loop is
//! lock the target, align, then commit.
//!
//! A lance with an EMPTY magazine still draws, DIMMED. The reload is twelve
//! seconds and it is exactly when a pilot wants to be lining the next shot up,
//! so taking the only aiming instrument away for it made the sight look
//! broken. The dim is the "not yet"; the ammo gauge is the countdown.
//!
//! # What it is drawn with
//!
//! The world-space holo language of [`holo_instruments`](mod@super::holo_instruments) -
//! thin unlit geometry the flight computer projects into space - in the slug's
//! own Pierce blue rather than NAV_CYAN, because this is a weapon instrument
//! and not a nav one. The line THICKENS with the charge, so the seconds you
//! are committed to holding a heading are readable without a second widget.

use avian3d::prelude::*;
use bevy::{light::NotShadowCaster, prelude::*};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::holo_instruments::segment_transform;

/// `BoreSightPlugin` and the two world-space components it owns.
pub mod prelude {
    pub use super::{BoreSightMark, BoreSightPlugin, BoreSightSegment};
}

/// Sight line radius, world units. Half the trajectory ribbon's: a nav plan is
/// something you read, a sight is something you look PAST.
const LINE_RADIUS: f32 = 0.03;

/// How much fatter the line gets at full charge. The commit is the one moment
/// the line stops being an aid and becomes a countdown, so it earns the weight.
const CHARGE_THICKEN: f32 = 2.4;

/// Kill-ring radius, world units. Sized against the 1-unit section cell it
/// marks, so a ring reads as "this cell" and not as a target bracket.
const MARK_RADIUS: f32 = 0.55;
/// Kill-ring tube thickness, world units.
const MARK_MINOR_RADIUS: f32 = 0.045;

/// Opacity of the sight line and of a kill ring on a LOADED lance.
const LINE_ALPHA: f32 = 0.5;
const MARK_ALPHA: f32 = 0.85;

/// What the sight's opacity is multiplied by while the magazine is empty.
///
/// Faint enough that "cannot fire" is the first thing read off it, solid
/// enough to still aim down through a twelve-second reload.
const EMPTY_ALPHA_SCALE: f32 = 0.3;

/// Ceiling on how many layers the trace will walk in one frame.
///
/// The weapon has no layer cap by design - power is its only bound - but the
/// SIGHT runs every frame on the main schedule, and a bore laid down the long
/// axis of a station would cast until the power ran out. Deep enough that no
/// ship reaches it, cheap enough that nothing has to think about it.
const MAX_TRACE_LAYERS: usize = 24;

/// One lance's sight line. The lance it belongs to, so a destroyed gun takes
/// its sight with it.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct BoreSightSegment {
    /// The railgun section this line comes out of.
    pub lance: Entity,
}

/// One ring on a section the shot would destroy.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct BoreSightMark {
    /// The railgun section whose shot this ring belongs to.
    pub lance: Entity,
    /// The collider this ring is drawn on - the one carrying the health pool
    /// the trace priced, which for a real section is the collider child.
    ///
    /// The ring's IDENTITY, and deliberately not its depth order. The order
    /// changes every frame a nose sweeps across a hull, so reconciling on it
    /// despawned and respawned a `Mesh3d` entity per frame for sections that
    /// never stopped being marked. Keyed this way a ring is spawned when its
    /// section starts being gutted and despawned when it stops, and the marks
    /// in between are only re-posed.
    pub target: Entity,
}

/// Shared meshes and materials for every sight, built on the first frame one
/// is drawn. A `Resource` rather than a `Local` so a headless app that never
/// raises weapons never allocates them, exactly as `HoloAssets` does.
#[derive(Resource, Default)]
struct BoreSightAssets {
    line_mesh: Option<Handle<Mesh>>,
    mark_mesh: Option<Handle<Mesh>>,
    /// Indexed by [`SightState`]: the loaded look, then the reloading one.
    line_material: [Option<Handle<StandardMaterial>>; 2],
    mark_material: [Option<Handle<StandardMaterial>>; 2],
}

/// Whether the lance this sight belongs to could fire right now. The ONLY
/// thing that changes about the sight, and it changes opacity and nothing
/// else: the line still ends where the shot would end and the rings still
/// name what it would take off, because that is what a pilot is reading
/// through the reload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SightState {
    /// A shell is loaded.
    Loaded,
    /// The magazine is empty and the reload is running.
    Reloading,
}

impl SightState {
    fn index(self) -> usize {
        match self {
            Self::Loaded => 0,
            Self::Reloading => 1,
        }
    }

    /// What the base opacity is scaled by in this state.
    fn alpha_scale(self) -> f32 {
        match self {
            Self::Loaded => 1.0,
            Self::Reloading => EMPTY_ALPHA_SCALE,
        }
    }
}

impl BoreSightAssets {
    fn line_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.line_mesh
            .get_or_insert_with(|| meshes.add(Cylinder::new(LINE_RADIUS, 1.0)))
            .clone()
    }

    fn mark_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.mark_mesh
            .get_or_insert_with(|| {
                meshes.add(Torus::new(
                    MARK_RADIUS - MARK_MINOR_RADIUS,
                    MARK_RADIUS + MARK_MINOR_RADIUS,
                ))
            })
            .clone()
    }

    fn line_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        state: SightState,
    ) -> Handle<StandardMaterial> {
        self.line_material[state.index()]
            .get_or_insert_with(|| materials.add(sight_material(LINE_ALPHA * state.alpha_scale())))
            .clone()
    }

    fn mark_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        state: SightState,
    ) -> Handle<StandardMaterial> {
        self.mark_material[state.index()]
            .get_or_insert_with(|| materials.add(sight_material(MARK_ALPHA * state.alpha_scale())))
            .clone()
    }
}

/// The sight's look: the Pierce hue the slug itself is drawn in, unlit and
/// blended so it reads as projected light rather than as a solid in the scene.
fn sight_material(alpha: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: damage_type_color(DamageType::Pierce).with_alpha(alpha),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    }
}

/// What one traced bore would do.
struct BoreTrace {
    /// Where the slug stops: the surface that expends it, or the end of its
    /// reach when nothing does.
    stop: Vec3,
    /// Every crossed section the shot would DESTROY, in depth order: the
    /// collider that carries the health pool, and the centre to ring it at. A
    /// section the slug crosses without killing is deliberately unmarked - the
    /// read is "what comes off", not "what I touch".
    ///
    /// The collider rides along because it is the ring's IDENTITY, not its
    /// position in this list: a nose sweeping across a hull changes the depth
    /// ORDER of the sections it would gut every frame, and a ring keyed on
    /// that order would be despawned and respawned for a section that never
    /// stopped being marked.
    kills: Vec<(Entity, Vec3)>,
}

/// Walk the bore, spending the authored power the way the round will.
///
/// Closing speed is taken as the slug's own muzzle speed - the target-at-rest
/// case. It is the honest prediction to draw: the sight cannot know what the
/// target will be doing when the shell arrives a charge and a flight later, and
/// a Pierce round's DAMAGE does not vary with speed anyway (only what crossing
/// costs it), so the marks move only at the deep end of a long rake.
#[expect(
    clippy::too_many_arguments,
    reason = "the trace takes the whole cast context by reference"
)]
fn trace_bore(
    spatial: &SpatialQuery,
    q_sensor: &Query<(), With<Sensor>>,
    q_collider_of: &Query<&ColliderOf>,
    q_health: &Query<&Health>,
    q_global: &Query<&GlobalTransform>,
    ship: Entity,
    muzzle: Vec3,
    bore: Dir3,
    config: &RailgunSectionConfigHelper,
) -> BoreTrace {
    let reach = config
        .slug_speed
        .over(config.slug_lifetime)
        .to_engine()
        .max(0.0);
    let mut damage = ProjectileDamage {
        amount: config.slug_damage,
        power: config.slug_power,
        // The authored round's own rule: power is the only bound.
        layers: u32::MAX,
        kind: DamageType::Pierce,
    };
    let closing = config.slug_speed.to_engine();
    let bite = hit_bite(damage, closing);

    let mut kills: Vec<(Entity, Vec3)> = Vec::new();
    let mut crossed: Vec<Entity> = Vec::new();
    let mut origin = muzzle;
    let mut travelled = 0.0f32;

    while travelled < reach && crossed.len() < MAX_TRACE_LAYERS {
        let body_of = |collider: Entity| q_collider_of.get(collider).map(|of| of.body);
        let hit = spatial.cast_ray_predicate(
            origin,
            bore,
            reach - travelled,
            true,
            &SpatialQueryFilter::default(),
            // The round's own `passable` rule: triggers are transparent, and a
            // gun never shoots the hull it is bolted to. Already-crossed
            // colliders drop out too - a ray restarting on a surface re-finds
            // it at distance zero and the walk would never advance.
            &|collider| {
                !q_sensor.contains(collider)
                    && body_of(collider) != Ok(ship)
                    && !crossed.contains(&collider)
            },
        );
        let Some(hit) = hit else {
            break;
        };

        let at = origin + bore * hit.distance;
        // A collider with no `Health` is a wall to a slug: an asteroid or a
        // planetoid has no thickness the power rule can price.
        let health = q_health.get(hit.entity).ok();
        if health.is_some_and(|health| bite >= health.current) {
            // The section's own origin, not the surface the ray entered on: a
            // ring on the entry face reads as a hit marker, a ring on the cell
            // reads as the cell coming off.
            let centre = q_global
                .get(hit.entity)
                .map_or(at, |global| global.translation());
            kills.push((hit.entity, centre));
        }

        crossed.push(hit.entity);
        travelled += hit.distance + PIERCE_SKIN;
        let Some(remainder) = pierce_remainder(damage, health, closing) else {
            return BoreTrace { stop: at, kills };
        };
        damage = remainder;
        origin = at + bore * PIERCE_SKIN;
    }

    // Nothing downrange stopped the slug: the loop only falls through here with
    // `travelled` at the reach it was capped to, so the line runs its full
    // length. (An expended slug returns early, above, at its last bite.)
    BoreTrace {
        stop: muzzle + bore * reach,
        kills,
    }
}

/// Own the sight: one line and N kill rings per live player lance while the
/// weapons are hot, updated every frame, gone the moment any of that stops
/// being true.
///
/// An empty magazine DIMS the sight rather than removing it - see the module
/// doc. A destroyed lance and a cold ship still take it away entirely.
///
/// A reconcile pass rather than spawn/despawn observers, for the reasons the
/// rest of the HUD uses one: a lance dies mid-fight, a magazine empties, and
/// the safety drops between frames.
#[expect(
    clippy::too_many_arguments,
    reason = "a reconcile pass over two entity families plus the cast context"
)]
fn sync_bore_sight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<BoreSightAssets>,
    spatial: SpatialQuery,
    q_player: Query<
        (Entity, Option<&WeaponsHot>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_lance: Query<
        (
            Entity,
            &ChildOf,
            &GlobalTransform,
            &RailgunSectionConfigHelper,
            &RailgunCharge,
            Option<&SectionAmmo>,
        ),
        (With<RailgunSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
    q_health: Query<&Health>,
    q_global: Query<&GlobalTransform>,
    mut q_segment: Query<
        (
            Entity,
            &BoreSightSegment,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<BoreSightMark>,
    >,
    mut q_mark: Query<
        (
            Entity,
            &BoreSightMark,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<BoreSightSegment>,
    >,
) {
    // A ship with no `WeaponsHot` is unmanaged (a bare rig); it has no player
    // looking at a HUD either, so it draws nothing rather than reading as hot.
    let hot_player = q_player
        .iter()
        .find(|(_, hot)| hot.is_some_and(|hot| hot.0))
        .map(|(ship, _)| ship);

    /// One lance's sight for this frame.
    struct Drawn {
        lance: Entity,
        state: SightState,
        line: Transform,
        marks: Vec<(Entity, Transform)>,
    }

    let mut drawn: Vec<Drawn> = Vec::new();
    if let Some(ship) = hot_player {
        for (lance, &ChildOf(parent), global, config, charge, ammo) in &q_lance {
            if parent != ship {
                continue;
            }
            let state = if ammo.is_some_and(SectionAmmo::is_empty) {
                SightState::Reloading
            } else {
                SightState::Loaded
            };
            // The RENDERED pose, unlike the shot itself: the sight has to leave
            // the muzzle the player can see, and inside `Update` that is the
            // eased transform rather than the raw physics one.
            let (_, rotation, _) = global.to_scale_rotation_translation();
            let Ok(bore) = Dir3::new(rotation * Vec3::NEG_Z) else {
                continue;
            };
            let muzzle = global.transform_point(config.muzzle_offset);

            let trace = trace_bore(
                &spatial,
                &q_sensor,
                &q_collider_of,
                &q_health,
                &q_global,
                ship,
                muzzle,
                bore,
                config,
            );

            // Thickness, not brightness: the line is already faint enough to
            // look past, and a swelling bore is the same language the charge
            // bolt speaks on the gun itself.
            let charge = charge.progress(config.charge_seconds);
            let girth = 1.0 + charge * (CHARGE_THICKEN - 1.0);
            let mut line = segment_transform(muzzle, trace.stop);
            line.scale.x *= girth;
            line.scale.z *= girth;

            let marks = trace
                .kills
                .iter()
                .map(|&(target, centre)| {
                    (
                        target,
                        Transform {
                            translation: centre,
                            // Face the ring across the bore, so it reads as
                            // something the shot goes THROUGH.
                            rotation: Quat::from_rotation_arc(Vec3::Y, *bore),
                            ..default()
                        },
                    )
                })
                .collect();
            drawn.push(Drawn {
                lance,
                state,
                line,
                marks,
            });
        }
    }

    // Re-pose what is already there, and drop anything whose lance stopped
    // drawing (destroyed, safed, or simply fewer kills this frame). The
    // material is reconciled too, and only WRITTEN when it changes: a swap
    // every frame would flag the whole sight dirty for nothing.
    for (entity, segment, mut transform, mut material) in &mut q_segment {
        match drawn.iter().find(|d| d.lance == segment.lance) {
            Some(d) => {
                *transform = d.line;
                let wanted = assets.line_material(&mut materials, d.state);
                if material.0 != wanted {
                    material.0 = wanted;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (entity, mark, mut transform, mut material) in &mut q_mark {
        match drawn.iter().find(|d| d.lance == mark.lance).and_then(|d| {
            d.marks
                .iter()
                .find(|(target, _)| *target == mark.target)
                .map(|(_, pose)| (d.state, pose))
        }) {
            Some((state, pose)) => {
                *transform = *pose;
                let wanted = assets.mark_material(&mut materials, state);
                if material.0 != wanted {
                    material.0 = wanted;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }

    // Spawn what is missing. Counting what already exists is cheaper than a
    // second pass and keeps this one idempotent.
    for d in &drawn {
        if !q_segment.iter().any(|(_, seg, ..)| seg.lance == d.lance) {
            let mesh = assets.line_mesh(&mut meshes);
            let material = assets.line_material(&mut materials, d.state);
            commands.spawn((
                Name::new("Bore Sight"),
                // HUD-managed like every other world-space instrument, so a
                // cinematic and the NOVA OS monitor take it away with the rest
                // of them instead of leaving a line drawn across the frame.
                crate::HudTier::Instrument,
                BoreSightSegment { lance: d.lance },
                Mesh3d(mesh),
                MeshMaterial3d(material),
                d.line,
                NotShadowCaster,
            ));
        }
        for &(target, pose) in &d.marks {
            if q_mark
                .iter()
                .any(|(_, mark, ..)| mark.lance == d.lance && mark.target == target)
            {
                continue;
            }
            let mesh = assets.mark_mesh(&mut meshes);
            let material = assets.mark_material(&mut materials, d.state);
            commands.spawn((
                Name::new("Bore Sight Kill"),
                crate::HudTier::Instrument,
                BoreSightMark {
                    lance: d.lance,
                    target,
                },
                Mesh3d(mesh),
                MeshMaterial3d(material),
                pose,
                NotShadowCaster,
            ));
        }
    }
}

/// Draws the world-space bore sight of every loaded player lance while the
/// weapons are hot. Inits `BoreSightAssets`, registers
/// [`BoreSightSegment`]/[`BoreSightMark`], and runs `sync_bore_sight` in
/// `Update` within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct BoreSightPlugin;

impl Plugin for BoreSightPlugin {
    fn build(&self, app: &mut App) {
        trace!("BoreSightPlugin: build");

        app.init_resource::<BoreSightAssets>();
        app.register_type::<BoreSightSegment>()
            .register_type::<BoreSightMark>();
        app.add_systems(Update, sync_bore_sight.in_set(super::NovaHudSystems));
    }
}

#[cfg(test)]
mod tests {
    use nova_events::units::prelude::*;
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;

    /// A physics app running only the sight. Real avian, because the sight IS
    /// a `SpatialQuery` consumer - a hand-placed hit list would prove nothing
    /// about what the bore actually crosses.
    fn sight_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        // The shared harness brings meshes but not materials: it stops short
        // of the PBR plugin that would normally register them, and the sight
        // builds one material of its own.
        app.init_asset::<StandardMaterial>();
        app.add_plugins(BoreSightPlugin);
        app.finish();
        app
    }

    /// A player ship with one axial lance, its own hull block sitting ON the
    /// bore so the trace has to look past it.
    fn spawn_lance_ship(
        app: &mut App,
        config: RailgunSectionConfig,
        hot: bool,
    ) -> (Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                Name::new("ship"),
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                WeaponsHot(hot),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_translation(Vec3::NEG_Z),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        let lance = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("lance"),
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 3.0),
                ColliderDensity(1.0),
                railgun_section(config),
            ))
            .id();
        settle(app);
        (ship, lance)
    }

    /// One target section: a body carrying a child collider with the health
    /// pool, exactly as a real ship's sections are built.
    fn spawn_plate(app: &mut App, z: f32, hp: f32) -> Entity {
        let body = app
            .world_mut()
            .spawn((
                Name::new("plate"),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * z),
            ))
            .id();
        app.world_mut()
            .spawn((
                ChildOf(body),
                Name::new("plate section"),
                Transform::default(),
                Collider::cuboid(8.0, 8.0, 1.0),
                ColliderDensity(1.0),
                Health::new(hp),
            ))
            .id()
    }

    fn marks(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut query = world.query_filtered::<(), With<BoreSightMark>>();
        query.iter(world).count()
    }

    fn lines(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut query = world.query_filtered::<(), With<BoreSightSegment>>();
        query.iter(world).count()
    }

    /// A slug at 1500 u/s is far past the power curve's 3x ceiling, so every
    /// plate costs a third of its max health. Named so the arithmetic in the
    /// tests below is checkable rather than magic.
    fn power_cost(max_health: f32) -> f32 {
        max_health / 3.0
    }

    /// The depth read, which is the whole point of the instrument: a ring on
    /// every section the shot DESTROYS, and none on a section it merely
    /// crosses. The middle plate is deliberately too tough to kill and too
    /// cheap to stop, so a sight that marked "what I hit" instead of "what
    /// comes off" would score three.
    #[test]
    fn the_sight_rings_only_the_sections_the_shot_would_destroy() {
        let mut app = sight_app();
        let config = RailgunSectionConfig {
            slug_damage: 300.0,
            slug_power: 1_000.0,
            slug_speed: MetersPerSecond(15_000.0),
            slug_lifetime: 1.0,
            ..default()
        };
        // 266.7 of the 1000 power budget: the slug outlasts the whole stack,
        // so nothing here is testing the cutoff.
        assert!(power_cost(200.0) + power_cost(500.0) + power_cost(100.0) < config.slug_power);

        spawn_lance_ship(&mut app, config, true);
        spawn_plate(&mut app, -10.0, 200.0);
        spawn_plate(&mut app, -20.0, 500.0);
        spawn_plate(&mut app, -30.0, 100.0);
        settle(&mut app);

        assert_eq!(lines(&mut app), 1, "one loaded lance draws one line");
        assert_eq!(
            marks(&mut app),
            2,
            "a 300-point bite kills the 200 and the 100 and leaves the 500 standing"
        );
    }

    /// The ring drawn on `target`, if the sight is drawing one.
    fn mark_of(app: &mut App, target: Entity) -> Option<Entity> {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &BoreSightMark)>();
        query
            .iter(world)
            .find(|(_, mark)| mark.target == target)
            .map(|(entity, _)| entity)
    }

    /// A ring belongs to the SECTION it is drawn on, not to its place in the
    /// depth order. A nose sweeping across a hull changes that order every
    /// frame, and reconciling on it churned real `Mesh3d` entities - an
    /// archetype move and a render-world sync each - for sections that never
    /// stopped being marked.
    #[test]
    fn a_kill_ring_outlives_a_change_in_what_is_in_front_of_it() {
        let mut app = sight_app();
        let config = RailgunSectionConfig {
            slug_damage: 300.0,
            slug_power: 1_000.0,
            slug_speed: MetersPerSecond(15_000.0),
            slug_lifetime: 1.0,
            ..default()
        };

        spawn_lance_ship(&mut app, config, true);
        let near = spawn_plate(&mut app, -10.0, 200.0);
        let far = spawn_plate(&mut app, -20.0, 200.0);
        settle(&mut app);

        assert_eq!(marks(&mut app), 2, "both plates come off");
        let ring = mark_of(&mut app, far).expect("the far plate is ringed");

        // The near plate hardens past one bite. It is still cheap enough to
        // rake through (166.7 of the 1000 power budget), so the far plate
        // stays marked - it simply becomes the FIRST kill instead of the
        // second.
        app.world_mut().entity_mut(near).insert(Health::new(500.0));
        settle(&mut app);

        assert_eq!(marks(&mut app), 1, "only the far plate still comes off");
        assert_eq!(
            mark_of(&mut app, far),
            Some(ring),
            "the far plate keeps the ring it already had"
        );
    }

    /// The power budget ends the rake, and the sight has to end with it. Three
    /// plates the slug can kill, and only enough power to reach two of them.
    #[test]
    fn the_sight_stops_where_the_power_runs_out() {
        let mut app = sight_app();
        let config = RailgunSectionConfig {
            slug_damage: 300.0,
            // Two plates at 200/3 each leaves 133 in hand; the third's 66.7
            // would take it to 66 - so the budget is deliberately cut to stop
            // the rake on the second.
            slug_power: 100.0,
            slug_speed: MetersPerSecond(15_000.0),
            slug_lifetime: 1.0,
            ..default()
        };

        spawn_lance_ship(&mut app, config, true);
        spawn_plate(&mut app, -10.0, 200.0);
        spawn_plate(&mut app, -20.0, 200.0);
        spawn_plate(&mut app, -30.0, 200.0);
        settle(&mut app);

        assert_eq!(
            marks(&mut app),
            2,
            "100 power buys two 200 hp layers at the ceiling and no third"
        );
    }

    /// The safety owns the sight. `WeaponsHot` is `raised OR combat-locked`,
    /// so this is also what keeps a line off the screen of a player who is
    /// simply flying somewhere with a lance bolted on.
    #[test]
    fn a_cold_ship_draws_no_sight() {
        let mut app = sight_app();
        let (ship, _) = spawn_lance_ship(&mut app, RailgunSectionConfig::default(), false);
        spawn_plate(&mut app, -10.0, 100.0);
        settle(&mut app);

        assert_eq!(lines(&mut app), 0, "weapons cold, nothing to aim");

        app.world_mut().get_mut::<WeaponsHot>(ship).unwrap().0 = true;
        settle(&mut app);

        assert_eq!(lines(&mut app), 1, "hot, and the sight comes up");
    }

    /// The line a segment is currently wearing.
    fn line_alpha(app: &mut App) -> f32 {
        let world = app.world_mut();
        let mut q =
            world.query_filtered::<&MeshMaterial3d<StandardMaterial>, With<BoreSightSegment>>();
        let handle = q.iter(world).next().expect("a segment is drawn").0.clone();
        world
            .resource::<Assets<StandardMaterial>>()
            .get(&handle)
            .expect("its material is live")
            .base_color
            .alpha()
    }

    /// A reload is twelve seconds of holding a heading, so the sight stays up
    /// through it - dimmed, because the gun cannot answer yet.
    #[test]
    fn a_reloading_lance_keeps_a_dimmed_sight() {
        let mut app = sight_app();
        let (_, lance) = spawn_lance_ship(&mut app, RailgunSectionConfig::default(), true);
        spawn_plate(&mut app, -10.0, 100.0);
        app.world_mut()
            .entity_mut(lance)
            .insert(SectionAmmo::new(1));
        settle(&mut app);
        assert_eq!(lines(&mut app), 1, "a loaded shell draws its line");
        let loaded = line_alpha(&mut app);

        app.world_mut()
            .get_mut::<SectionAmmo>(lance)
            .unwrap()
            .rounds = 0;
        settle(&mut app);
        assert_eq!(
            lines(&mut app),
            1,
            "spent, and the line is still there to aim"
        );
        let reloading = line_alpha(&mut app);
        assert!(
            reloading < loaded,
            "but faint, so the pilot reads 'not yet': {reloading} against {loaded}"
        );

        app.world_mut()
            .get_mut::<SectionAmmo>(lance)
            .unwrap()
            .rounds = 1;
        settle(&mut app);
        assert_eq!(
            line_alpha(&mut app),
            loaded,
            "and the shell coming back is what brings it up again"
        );
    }

    /// A destroyed lance is a different thing from an empty one: it takes its
    /// sight with it.
    #[test]
    fn a_dead_lance_draws_no_sight() {
        let mut app = sight_app();
        let (_, lance) = spawn_lance_ship(&mut app, RailgunSectionConfig::default(), true);
        spawn_plate(&mut app, -10.0, 100.0);
        settle(&mut app);
        assert_eq!(lines(&mut app), 1, "a live lance draws its line");

        app.world_mut()
            .entity_mut(lance)
            .insert(SectionInactiveMarker);
        settle(&mut app);
        assert_eq!(lines(&mut app), 0, "shot off, and the line goes with it");
    }
}
