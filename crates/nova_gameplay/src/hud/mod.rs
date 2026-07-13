use bevy::prelude::*;

use crate::prelude::*;

pub mod ammo_readout;
pub mod beacon_chips;
pub mod component_lock;
pub mod edge_indicators;
pub mod flight_status;
pub mod holo_instruments;
pub mod item_highlights;
pub mod keybind_hints;
pub mod lock_crosshairs;
pub mod maneuver_instruments;
pub mod objective_feedback;
pub mod objective_markers;
pub mod screen_indicator;
pub mod target_inset;
pub mod torpedo_target;
pub mod turret_lead;
pub mod velocity;

pub mod prelude {
    pub use super::{
        ammo_readout::prelude::*, beacon_chips::prelude::*, component_lock::prelude::*,
        edge_indicators::prelude::*, flight_status::prelude::*, holo_instruments::prelude::*,
        item_highlights::prelude::*, keybind_hints::prelude::*, lock_crosshairs::prelude::*,
        maneuver_instruments::prelude::*, objective_feedback::prelude::*,
        objective_markers::prelude::*, screen_indicator::prelude::*, target_inset::prelude::*,
        torpedo_target::prelude::*, turret_lead::prelude::*, velocity::prelude::*,
        HudSelfDrivenVisibility, HudTier, HudVisibility, NovaHudAssets, NovaHudPlugin,
        NovaHudSystems,
    };
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaHudSystems;

/// Player-facing HUD visibility level, cycled with grave/tilde (task
/// 20260711-180501): `All` shows everything, `Minimal` keeps the flight and
/// combat instruments but drops the chrome, `None` clears the screen for
/// cinematic shots. The menu (nova_menu) also drives this to `None` while the
/// main menu is up.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Hash, Default, Reflect)]
#[reflect(Resource)]
pub enum HudVisibility {
    #[default]
    All,
    Minimal,
    None,
}

impl HudVisibility {
    /// The cycle order behind the grave/tilde key.
    pub fn next(self) -> Self {
        match self {
            HudVisibility::All => HudVisibility::Minimal,
            HudVisibility::Minimal => HudVisibility::None,
            HudVisibility::None => HudVisibility::All,
        }
    }

    /// Whether a widget of `tier` is visible at this level.
    pub fn shows(self, tier: HudTier) -> bool {
        match (self, tier) {
            (HudVisibility::All, _) => true,
            (HudVisibility::Minimal, HudTier::Instrument) => true,
            _ => false,
        }
    }
}

/// The visibility tier a HUD widget belongs to. Tag the widget's root where it
/// spawns; screen-indicator nodes resolve their tier from the nearest tagged
/// ancestor (or their own tag), so reconciled children (pips, brackets,
/// arrows) inherit their module's tier automatically.
/// Deliberately untagged: the juice gizmo flashes (juice.rs) are combat FX,
/// not HUD, and stay visible at every level.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component)]
pub enum HudTier {
    /// Flight/combat instruments: visible at `All` and `Minimal`.
    Instrument,
    /// Learning aids and secondary overlays: visible only at `All`.
    Chrome,
}

/// Opt-out for widgets that drive their own `Visibility` every frame (the
/// gravity sphere hides itself in flat space): the level-change restore skips
/// them so it cannot stomp their state for a frame; the Hidden enforcement
/// still applies while their tier is off (review R1.2).
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct HudSelfDrivenVisibility;

/// Objectives panel width (px): narrow enough to sit as a side column,
/// wide enough that a beat objective wraps to 3-4 lines.
const OBJECTIVES_PANEL_WIDTH_PX: f32 = 280.0;

/// Objective line font size (px). The bcs ObjectivesPlugin spawns its text
/// lines at the default (much larger) size; nova restyles them as they
/// appear (playtest 2026-07-12 finding 3).
const OBJECTIVES_FONT_PX: f32 = 13.0;

/// Nav cyan, the family color of every flight-computer projection (the
/// destination marker tint, the orbit cue, the maneuver chips, the holo
/// ring).
pub(crate) const NAV_CYAN: Color = Color::srgba(0.3, 0.9, 1.0, 0.9);

/// Objective gold, the "do this now" accent (task 20260712-093831): the
/// objective marker chip and the hint-emphasis pulse draw from it. One hue
/// per meaning - cyan is nav infrastructure, red is threat, green is
/// own/done, gold is the current objective.
pub(crate) const OBJECTIVE_GOLD: Color = Color::srgba(1.0, 0.85, 0.3, 0.95);

#[derive(Resource, Clone, Default, Debug)]
pub struct NovaHudAssets {
    pub target_sprite: Handle<Image>,
}

#[derive(Default)]
pub struct NovaHudPlugin;

impl Plugin for NovaHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("HudPlugin: build");

        app.init_resource::<NovaHudAssets>();

        app.init_resource::<HudVisibility>();
        app.register_type::<HudVisibility>();
        app.register_type::<HudTier>();
        // The cycle key is gameplay-only (the menu drives the resource
        // itself); plain ButtonInput, same pattern as the debug F11 toggle.
        app.add_systems(
            Update,
            cycle_hud_visibility.run_if(in_state(crate::GameStates::Playing)),
        );
        // Visibility enforcement runs AFTER the screen-indicator projection:
        // the widget writes Visibility::Visible on its nodes every frame in
        // PostUpdate (ignoring hidden ancestors), so a tier-hidden node must
        // be overwritten downstream of that producer, not from Update.
        // Bounded on both sides: after the indicator projection (whose
        // Visible writes it must overrule) and before UI layout - which runs
        // upstream of transform + visibility propagation - so the writes land
        // in THIS frame's propagation deterministically instead of by
        // schedule tie-break (review R1.1).
        app.add_systems(
            PostUpdate,
            apply_hud_visibility
                .after(ScreenIndicatorSystems)
                .before(bevy::ui::UiSystems::Layout),
        );

        app.add_plugins(velocity::VelocityHudPlugin);
        app.add_plugins(flight_status::FlightStatusHudPlugin);
        app.add_plugins(maneuver_instruments::ManeuverInstrumentsPlugin);
        app.add_plugins(keybind_hints::KeybindHintsPlugin);
        app.add_plugins(holo_instruments::HoloInstrumentsPlugin);
        // The health and objectives HUDs are now the generic bevy_common_systems widgets.
        app.add_plugins(HealthDisplayPlugin);
        app.add_plugins(ObjectivesPlugin);
        app.add_plugins(screen_indicator::ScreenIndicatorPlugin);
        app.add_plugins(torpedo_target::TorpedoTargetHudPlugin);
        app.add_plugins(turret_lead::TurretLeadPlugin);
        app.add_plugins(ammo_readout::AmmoReadoutPlugin);
        app.add_plugins(component_lock::ComponentLockHudPlugin);
        app.add_plugins(lock_crosshairs::LockCrosshairsHudPlugin);
        app.add_plugins(target_inset::TargetInsetHudPlugin);
        app.add_plugins(edge_indicators::EdgeIndicatorsHudPlugin);
        app.add_plugins(beacon_chips::BeaconChipsHudPlugin);
        app.add_plugins(objective_markers::ObjectiveMarkersHudPlugin);
        app.add_plugins(item_highlights::ItemHighlightsHudPlugin);
        app.add_plugins(objective_feedback::ObjectiveFeedbackPlugin);

        // Restyle freshly rebuilt objective lines. After the Sync set in
        // the same schedule: the rebuild despawns and respawns the text
        // entities, so styling keys on Added<ObjectiveMarker> and must run
        // downstream of the producer within the frame.
        app.add_systems(
            Update,
            style_objective_lines
                .after(ObjectivesPluginSystems::Sync)
                .in_set(NovaHudSystems),
        );

        // Keep the generic HUD widgets inside nova's HUD ordering slot, as the local ones were.
        // ScreenIndicatorSystems is NOT in this Update slot anymore: the
        // projection runs in PostUpdate after the chase camera's final move
        // (task 20260710-231928), and the Update-schedule driver systems
        // precede it by schedule order alone.
        app.configure_sets(
            Update,
            (
                HealthDisplayPluginSystems::Sync,
                ObjectivesPluginSystems::Sync,
            )
                .in_set(NovaHudSystems),
        );

        // Screen indicators project through the spaceship chase camera. The
        // widget is camera-agnostic (its own marker keeps it promotable), so
        // nova tags the camera whenever the controller hands it over.
        app.add_observer(add_screen_indicator_camera);
        app.add_observer(remove_screen_indicator_camera);

        // Setup and remove HUDs when player spaceship is added/removed
        app.add_observer(setup_hud_velocity);
        app.add_observer(remove_hud_velocity);
        app.add_observer(setup_hud_flight_status);
        app.add_observer(remove_hud_flight_status);
        app.add_observer(setup_hud_health);
        app.add_observer(remove_hud_health);
        app.add_observer(setup_hud_objectives);
        app.add_observer(remove_hud_objectives);
        app.add_observer(setup_hud_torpedo_target);
        app.add_observer(remove_hud_torpedo_target);
        app.add_observer(setup_hud_turret_lead);
        app.add_observer(remove_hud_turret_lead);
        app.add_observer(setup_hud_ammo_readout);
        app.add_observer(remove_hud_ammo_readout);
        app.add_observer(setup_hud_component_lock);
        app.add_observer(remove_hud_component_lock);
        app.add_observer(setup_hud_lock_crosshairs);
        app.add_observer(remove_hud_lock_crosshairs);
        app.add_observer(setup_hud_target_inset);
        app.add_observer(remove_hud_target_inset);
        app.add_observer(setup_hud_edge_indicators);
        app.add_observer(remove_hud_edge_indicators);
    }
}

/// Spawn the bcs objectives panel, then REPLACE its Node with nova's
/// layout: fixed width so objective text WRAPS instead of running across
/// the screen, stacked as a column (playtest 2026-07-12 finding 3; the
/// full HUD treatment is the conveyance task 20260712-093831). The
/// override MUST be a second insert, not part of the spawn bundle - a
/// bundle with two Nodes (the panel's and ours) PANICS on duplicate
/// components (playtest crash, review R1.5). insert-on-existing replaces.
/// Factored out so the styling test exercises this exact spawn path.
fn spawn_objectives_panel(commands: &mut Commands) {
    commands
        .spawn((HudTier::Chrome, objectives_panel(ObjectivesPanelConfig {})))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            right: Val::Px(8.0),
            width: Val::Px(OBJECTIVES_PANEL_WIDTH_PX),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        });
}

/// Restyle the bcs objectives panel's text lines the frame they (re)spawn:
/// smaller font, left-justified, wrapping inside the panel's fixed width.
/// The lines are rebuilt from scratch on every objectives change
/// (ObjectivesPlugin), so Added fires for each rebuild.
fn style_objective_lines(mut commands: Commands, q_lines: Query<Entity, Added<ObjectiveMarker>>) {
    for line in &q_lines {
        commands.entity(line).insert((
            TextFont::from_font_size(OBJECTIVES_FONT_PX),
            TextLayout {
                justify: Justify::Left,
                linebreak: LineBreak::WordBoundary,
            },
        ));
    }
}

/// Cycle the HUD level on grave/tilde (or the gamepad Select button).
/// Press-to-cycle, no hold gesture (the spike's call: three states are at most
/// two presses away).
fn cycle_hud_visibility(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    mut level: ResMut<HudVisibility>,
) {
    let pad = gamepad
        .map(|g| g.just_pressed(GamepadButton::Select))
        .unwrap_or(false);
    if keys.just_pressed(KeyCode::Backquote) || pad {
        let next = level.next();
        info!("hud visibility: {:?} -> {:?}", *level, next);
        *level = next;
    }
}

/// Enforce the current [`HudVisibility`] level on every tagged widget.
///
/// Two passes:
/// - Tagged roots (and tagged world-space instruments like ribbon segments):
///   hidden while their tier is off, restored to `Inherited` once when the
///   level changes back. Self-driving widgets (the gravity sphere) re-assert
///   their own state every frame, so the one-shot restore cannot wedge them.
/// - Screen-indicator nodes: their projection re-writes `Visibility::Visible`
///   every frame in this same schedule, so the tier-hidden ones are
///   overwritten here (after the projection) every frame; no restore branch
///   is needed because the widget re-drives them. Tier resolves from the node
///   or its nearest tagged ancestor; untagged trees are not HUD-managed.
fn apply_hud_visibility(
    level: Res<HudVisibility>,
    mut q_roots: Query<
        (&HudTier, &mut Visibility, Has<HudSelfDrivenVisibility>),
        Without<ScreenIndicatorMarker>,
    >,
    mut q_indicators: Query<
        (Entity, &mut Visibility, Option<&HudTier>),
        With<ScreenIndicatorMarker>,
    >,
    q_parents: Query<&ChildOf>,
    q_tiers: Query<&HudTier>,
) {
    let level_changed = level.is_changed();
    for (tier, mut visibility, self_driven) in &mut q_roots {
        if !level.shows(*tier) {
            visibility.set_if_neq(Visibility::Hidden);
        } else if level_changed && !self_driven {
            visibility.set_if_neq(Visibility::Inherited);
        }
    }
    for (entity, mut visibility, own_tier) in &mut q_indicators {
        let tier = own_tier
            .copied()
            .or_else(|| ancestor_tier(entity, &q_parents, &q_tiers));
        let Some(tier) = tier else {
            continue;
        };
        if !level.shows(tier) {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

/// The nearest ancestor's [`HudTier`], if any.
fn ancestor_tier(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    tiers: &Query<&HudTier>,
) -> Option<HudTier> {
    while let Ok(ChildOf(parent)) = parents.get(entity) {
        if let Ok(tier) = tiers.get(*parent) {
            return Some(*tier);
        }
        entity = *parent;
    }
    None
}

/// Tag the spaceship chase camera as the projection camera for screen
/// indicators.
fn add_screen_indicator_camera(
    add: On<Add, crate::camera_controller::SpaceshipCameraController>,
    mut commands: Commands,
) {
    debug!("add_screen_indicator_camera: entity {:?}", add.entity);
    commands.entity(add.entity).insert(ScreenIndicatorCamera);
}

/// Untag the camera when the spaceship controller releases it (e.g. back to
/// the WASD camera after the player ship dies), so indicators hide instead of
/// projecting through a free camera.
fn remove_screen_indicator_camera(
    remove: On<Remove, crate::camera_controller::SpaceshipCameraController>,
    mut commands: Commands,
) {
    debug!("remove_screen_indicator_camera: entity {:?}", remove.entity);
    // try_remove, not remove: get_entity only proves the entity exists at
    // QUEUE time - a scenario teardown despawns the camera in the same
    // command flush, and the plain remove then warns "entity despawned"
    // (playtest 2026-07-13, the asteroid_next transition).
    if let Ok(mut camera) = commands.get_entity(remove.entity) {
        camera.try_remove::<ScreenIndicatorCamera>();
    }
}

fn setup_hud_velocity(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_velocity: entity {:?}", entity);

    let Ok(spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_velocity: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((
        HudTier::Instrument,
        velocity_hud(VelocityHudConfig {
            radius: 5.0,
            sharpness: 20.0,
            target: spaceship,
            ..default()
        }),
    ));
    // The gravity indicator: same widget, yellow, pointing down the
    // dominant well's pull, hidden in flat space. Nested slightly outside
    // the velocity sphere so the two shells never z-fight.
    commands.spawn((
        HudTier::Instrument,
        // Hides itself in flat space; the level-change restore must not
        // overrule that (review R1.2).
        HudSelfDrivenVisibility,
        velocity_hud(VelocityHudConfig {
            radius: 5.6,
            sharpness: 20.0,
            target: spaceship,
            source: VelocityHudSource::Gravity,
            palette: VelocityHudPalette::GRAVITY,
        }),
    ));
}

fn remove_hud_velocity(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &VelocityHudTargetEntity), With<VelocityHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_velocity: entity {:?}", entity);

    for (hud_entity, target) in &q_hud {
        if **target == entity {
            commands.entity(hud_entity).despawn();
        }
    }
}

fn setup_hud_flight_status(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_existing_cluster: Query<(), With<KeybindHintClusterMarker>>,
    assets: Res<NovaHudAssets>,
) {
    let entity = add.entity;
    debug!("setup_hud_flight_status: entity {:?}", entity);

    let Ok(spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_flight_status: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((
        HudTier::Instrument,
        flight_status_hud(FlightStatusHudConfig { target: spaceship }),
    ));
    commands.spawn((
        HudTier::Instrument,
        autopilot_destination_hud(AutopilotDestinationHudConfig::new(
            spaceship,
            assets.target_sprite.clone(),
        )),
    ));
    commands.spawn((
        HudTier::Instrument,
        maneuver_instruments_hud(ManeuverInstrumentsHudConfig { ship: spaceship }),
    ));
    // The cluster and cues are global singletons, not ship-targeted
    // widgets: one player, one set (same guard as the flight input rig).
    if q_existing_cluster.is_empty() {
        commands.spawn((HudTier::Chrome, keybind_hint_cluster_hud()));
        commands.spawn((HudTier::Chrome, verb_cues_hud()));
    }
}

fn remove_hud_flight_status(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &FlightStatusHudTargetEntity), With<FlightStatusHudMarker>>,
    q_destination: Query<Entity, With<AutopilotDestinationHudMarker>>,
    q_cluster: Query<Entity, With<KeybindHintClusterMarker>>,
    q_cues: Query<Entity, With<VerbCuesHudMarker>>,
    q_instruments: Query<Entity, With<ManeuverInstrumentsHudMarker>>,
    q_ring: Query<Entity, With<OrbitRingMarker>>,
    q_spoke: Query<Entity, With<RadiusSpokeMarker>>,
    q_ribbon: Query<Entity, With<TrajectoryRibbonSegment>>,
    q_gate: Query<Entity, With<FlipGateMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_flight_status: entity {:?}", entity);

    for (hud_entity, target) in &q_hud {
        if **target == entity {
            commands.entity(hud_entity).despawn();
        }
    }
    for hud_entity in &q_destination {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_cluster {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_cues {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_instruments {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in &q_ring {
        commands.entity(hud_entity).despawn();
    }
    for hud_entity in q_spoke.iter().chain(q_ribbon.iter()).chain(&q_gate) {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_health(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_health: entity {:?}", entity);

    let Ok(spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_health: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((
        HudTier::Instrument,
        health_display(HealthDisplayConfig {
            target: Some(spaceship),
        }),
    ));
}

fn remove_hud_health(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &HealthDisplayTarget), With<HealthDisplayMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_health: entity {:?}", entity);

    for (hud_entity, target) in &q_hud {
        if let Some(target_entity) = **target {
            if target_entity == entity {
                commands.entity(hud_entity).despawn();
            }
        }
    }
}

fn setup_hud_objectives(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_objectives: entity {:?}", entity);

    let Ok(_) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_objectives: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    spawn_objectives_panel(&mut commands);
}

fn remove_hud_objectives(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<ObjectivesPanelMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_objectives: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_turret_lead(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_turret_lead: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_turret_lead: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((HudTier::Instrument, turret_lead_hud()));
}

fn remove_hud_turret_lead(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<TurretLeadHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_turret_lead: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_ammo_readout(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_ammo_readout: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_ammo_readout: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((HudTier::Instrument, ammo_readout_hud()));
}

fn remove_hud_ammo_readout(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<AmmoReadoutHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_ammo_readout: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_component_lock(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_component_lock: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_component_lock: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((HudTier::Chrome, component_lock_hud()));
}

fn remove_hud_component_lock(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<ComponentLockHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_component_lock: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_lock_crosshairs(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    assets: Res<NovaHudAssets>,
) {
    let entity = add.entity;
    debug!("setup_hud_lock_crosshairs: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_lock_crosshairs: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((
        HudTier::Instrument,
        lock_crosshairs_hud(assets.target_sprite.clone()),
    ));
}

fn remove_hud_lock_crosshairs(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<LockCrosshairsHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_lock_crosshairs: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

/// Build the target-inset render target + highlight assets (Assets exist at
/// runtime, not necessarily at plugin build) and spawn the corner panel Hidden.
/// The inset camera itself spawns/despawns with the focus dwell
/// (`target_inset::drive_inset_camera`), not with the player.
fn setup_hud_target_inset(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut render_target: ResMut<TargetInsetRenderTarget>,
) {
    let entity = add.entity;
    debug!("setup_hud_target_inset: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_target_inset: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    let image = target_inset::create_render_target(&mut images);
    **render_target = Some(image.clone());
    commands.insert_resource(TargetInsetHighlightAssets {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(target_inset::highlight_material()),
    });
    commands.spawn(target_inset_hud(image));
}

fn remove_hud_target_inset(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_panel: Query<Entity, With<TargetInsetHudMarker>>,
    q_camera: Query<Entity, With<TargetInsetCameraMarker>>,
    q_highlights: Query<Entity, With<TargetInsetHighlightMarker>>,
    mut render_target: ResMut<TargetInsetRenderTarget>,
) {
    let entity = remove.entity;
    debug!("remove_hud_target_inset: entity {:?}", entity);

    for panel in &q_panel {
        commands.entity(panel).despawn();
    }
    for camera in &q_camera {
        commands.entity(camera).despawn();
    }
    for highlight in &q_highlights {
        commands.entity(highlight).despawn();
    }
    **render_target = None;
    commands.remove_resource::<TargetInsetHighlightAssets>();
}

fn setup_hud_edge_indicators(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let entity = add.entity;
    debug!("setup_hud_edge_indicators: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_edge_indicators: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((HudTier::Chrome, edge_indicators_hud()));
}

fn remove_hud_edge_indicators(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<EdgeIndicatorsHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_edge_indicators: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

fn setup_hud_torpedo_target(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    assets: Res<NovaHudAssets>,
) {
    let entity = add.entity;
    debug!("setup_hud_torpedo_target: entity {:?}", entity);

    let Ok(_spaceship) = q_spaceship.get(entity) else {
        error!(
            "setup_hud_torpedo_target: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.spawn((
        HudTier::Instrument,
        torpedo_target_hud(TorpedoTargetHudConfig {
            target_sprite: assets.target_sprite.clone(),
        }),
    ));
}

fn remove_hud_torpedo_target(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<TorpedoTargetHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_torpedo_target: entity {:?}", entity);

    for hud_entity in &q_hud {
        commands.entity(hud_entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// A stand-in for the real projection: writes Visible on every indicator
    /// node each frame, in the real ScreenIndicatorSystems set, so the tests
    /// exercise the actual schedule contract (enforcement must win the same
    /// frame, downstream of this producer). Review R1.3.
    fn fake_widget_drive(mut q: Query<&mut Visibility, With<ScreenIndicatorMarker>>) {
        for mut visibility in &mut q {
            visibility.set_if_neq(Visibility::Visible);
        }
    }

    /// Headless app with exactly the HudVisibility wiring the plugin
    /// registers (the full NovaHudPlugin drags in assets/materials), plus the
    /// stand-in widget driver inside ScreenIndicatorSystems.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::GameStates>();
        app.init_resource::<HudVisibility>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(
            Update,
            cycle_hud_visibility.run_if(in_state(crate::GameStates::Playing)),
        );
        app.add_systems(PostUpdate, fake_widget_drive.in_set(ScreenIndicatorSystems));
        // Same double-bounded registration as the plugin (review R1.1).
        app.add_systems(
            PostUpdate,
            apply_hud_visibility
                .after(ScreenIndicatorSystems)
                .before(bevy::ui::UiSystems::Layout),
        );
        app.world_mut()
            .resource_mut::<NextState<crate::GameStates>>()
            .set(crate::GameStates::Playing);
        app.update();
        app
    }

    fn press_backquote(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Backquote);
        app.update();
        // Headless apps have no InputPlugin frame-clear; do it by hand so the
        // next press registers as a fresh just_pressed.
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Backquote);
        keys.clear();
        app.update();
    }

    fn level(app: &App) -> HudVisibility {
        *app.world().resource::<HudVisibility>()
    }

    /// Delivery-guarded per step (LESSONS assert-each-gesture-step): the level
    /// is asserted after every individual press, not just at the end.
    #[test]
    fn backquote_cycles_all_minimal_none_all() {
        let mut app = app();
        assert_eq!(level(&app), HudVisibility::All);
        press_backquote(&mut app);
        assert_eq!(level(&app), HudVisibility::Minimal);
        press_backquote(&mut app);
        assert_eq!(level(&app), HudVisibility::None);
        press_backquote(&mut app);
        assert_eq!(level(&app), HudVisibility::All);
    }

    #[test]
    fn tiers_hide_and_restore_across_levels() {
        let mut app = app();
        let instrument = app
            .world_mut()
            .spawn((HudTier::Instrument, Visibility::Inherited))
            .id();
        let chrome = app
            .world_mut()
            .spawn((HudTier::Chrome, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        app.update();
        assert_eq!(vis(&app, instrument), Visibility::Inherited);
        assert_eq!(vis(&app, chrome), Visibility::Inherited);

        app.insert_resource(HudVisibility::Minimal);
        app.update();
        assert_eq!(vis(&app, instrument), Visibility::Inherited);
        assert_eq!(vis(&app, chrome), Visibility::Hidden);

        app.insert_resource(HudVisibility::None);
        app.update();
        assert_eq!(vis(&app, instrument), Visibility::Hidden);
        assert_eq!(vis(&app, chrome), Visibility::Hidden);

        app.insert_resource(HudVisibility::All);
        app.update();
        assert_eq!(vis(&app, instrument), Visibility::Inherited);
        assert_eq!(vis(&app, chrome), Visibility::Inherited);
    }

    /// The screen-indicator projection writes Visibility::Visible on its nodes
    /// every frame (ignoring hidden ancestors), so enforcement must overwrite
    /// tier-hidden nodes every frame, resolving the tier from the nearest
    /// tagged ancestor. This simulates the widget by re-writing Visible before
    /// each update.
    #[test]
    fn indicator_nodes_are_overwritten_every_frame_via_ancestor_tier() {
        let mut app = app();
        let root = app.world_mut().spawn((HudTier::Chrome,)).id();
        let node = app
            .world_mut()
            .spawn((ScreenIndicatorMarker, Visibility::Visible, ChildOf(root)))
            .id();

        app.insert_resource(HudVisibility::Minimal);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(node).unwrap(),
            Visibility::Hidden
        );

        // The in-schedule stand-in re-drives the node to Visible inside
        // ScreenIndicatorSystems every frame; enforcement must win the SAME
        // frame, every frame, even though the level did not change. This is
        // the executable form of the ordering contract (review R1.3): moving
        // apply_hud_visibility before the set fails here.
        app.update();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(node).unwrap(),
            Visibility::Hidden
        );

        // Back at All the enforcement stands down and the widget owns it.
        app.insert_resource(HudVisibility::All);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(node).unwrap(),
            Visibility::Visible
        );
    }

    /// Review R1.2: self-driving widgets opt out of the level-change restore
    /// (their own Update driver holds the correct state), but the Hidden
    /// enforcement still applies while their tier is off.
    #[test]
    fn self_driven_roots_skip_the_restore_but_not_the_hide() {
        let mut app = app();
        let sphere = app
            .world_mut()
            .spawn((
                HudTier::Instrument,
                HudSelfDrivenVisibility,
                // Self-driven state: hidden (flat space).
                Visibility::Hidden,
            ))
            .id();

        app.insert_resource(HudVisibility::None);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(sphere).unwrap(),
            Visibility::Hidden
        );

        // Restoring to All must NOT stomp the widget's own Hidden.
        app.insert_resource(HudVisibility::All);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(sphere).unwrap(),
            Visibility::Hidden,
            "restore must skip self-driven widgets"
        );
    }

    /// Objective lines are restyled the frame the bcs plugin (re)spawns
    /// them: nova's font size and wrapping layout land on every rebuild,
    /// including replacements (the tally text swaps lines wholesale). The
    /// panel is spawned through the PRODUCTION helper - a bare-panel spawn
    /// here let a duplicate-Node bundle panic ship to a live playtest
    /// (R1.5), and round 2 shipped a no-op edit claiming this was fixed
    /// (R2.1): the helper call below is the actual guard, and the width
    /// assert catches a dropped override.
    #[test]
    fn objective_lines_get_novas_font_and_wrap() {
        let mut app = App::new();
        app.add_plugins(ObjectivesPlugin);
        app.add_systems(
            Update,
            style_objective_lines.after(ObjectivesPluginSystems::Sync),
        );
        app.add_systems(Startup, |mut commands: Commands| {
            spawn_objectives_panel(&mut commands);
        });
        app.update();

        let mut q_panel = app
            .world_mut()
            .query_filtered::<&Node, With<ObjectivesPanelMarker>>();
        let panel_node = q_panel.single(app.world()).expect("the panel spawned");
        assert_eq!(
            panel_node.width,
            Val::Px(OBJECTIVES_PANEL_WIDTH_PX),
            "nova's Node override replaced the bcs panel layout"
        );

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();
        app.update();

        let expected = TextFont::from_font_size(OBJECTIVES_FONT_PX).font_size;
        let mut q_lines = app
            .world_mut()
            .query_filtered::<&TextFont, With<ObjectiveMarker>>();
        let fonts: Vec<_> = q_lines
            .iter(app.world())
            .map(|font| font.font_size)
            .collect();
        assert_eq!(fonts.len(), 1, "one line per objective");
        assert_eq!(fonts[0], expected, "the line carries nova's font size");

        // A rebuild (message swap) restyles the fresh line too.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Supply crates recovered: 1/3.")];
        app.update();
        app.update();
        let fonts: Vec<_> = {
            let mut q = app
                .world_mut()
                .query_filtered::<&TextFont, With<ObjectiveMarker>>();
            q.iter(app.world()).map(|font| font.font_size).collect()
        };
        assert!(
            fonts.iter().all(|size| *size == expected),
            "rebuilt lines are restyled, got {:?}",
            fonts
        );
    }
}
