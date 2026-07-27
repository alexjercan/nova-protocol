//! The player's heads-up display: the diegetic instruments and overlays drawn
//! for the player ship (velocity/flight status, lock crosshairs and dwell rings,
//! turret lead and torpedo target reticles, ammo readouts, edge/threat
//! indicators, objective markers, the comms panel and keybind hints). Each
//! widget lives in its own submodule and is a [`HudTier`] layer spawned and
//! despawned with the player ship.
//!
//! Touch this module (or add a submodule) to change what the player sees.
//! [`NovaHudPlugin`] adds every widget; the HUD reads gameplay state (locks,
//! flight, sections) but does not drive it. This is the code-level view; the HUD
//! design lives in the wiki.

use bevy::prelude::*;

use crate::prelude::*;

pub mod allegiance_markers;
pub mod ammo_readout;
pub mod beacon_chips;
pub mod comms_panel;
pub mod component_lock;
pub mod drawer;
pub mod edge_indicators;
pub mod flight_status;
pub mod holo_instruments;
pub mod item_highlights;
pub mod keybind_hints;
pub mod lock_crosshairs;
pub mod lock_dwell_ring;
pub mod maneuver_instruments;
pub mod objective_feedback;
pub mod objective_hint;
pub mod objective_markers;
pub mod objective_reveal;
pub mod readout;
pub mod screen_indicator;
pub mod target_inset;
pub mod torpedo_target;
pub mod turret_lead;
pub mod velocity;

/// Glob-import surface: `use nova_gameplay::hud::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        allegiance_markers::prelude::*, ammo_readout::prelude::*, beacon_chips::prelude::*,
        comms_panel::prelude::*, component_lock::prelude::*, drawer::NovaOsMonitorSettings,
        edge_indicators::prelude::*, flight_status::prelude::*, holo_instruments::prelude::*,
        item_highlights::prelude::*, keybind_hints::prelude::*, lock_crosshairs::prelude::*,
        lock_dwell_ring::prelude::*, maneuver_instruments::prelude::*,
        objective_feedback::prelude::*, objective_markers::prelude::*, readout::prelude::*,
        screen_indicator::prelude::*, target_inset::prelude::*, torpedo_target::prelude::*,
        turret_lead::prelude::*, velocity::prelude::*, HudDrawerExempt, HudSelfDrivenVisibility,
        HudTier, HudVisibility, NovaHudAssets, NovaHudPlugin, NovaHudSystems,
    };
}

/// System set that all HUD update systems belong to, for ordering against the rest of the app.
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
    /// Everything: instruments plus chrome.
    All,
    /// Flight and combat instruments only; chrome drops away.
    Minimal,
    /// A clean screen for cinematic captures.
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
        matches!(
            (self, tier),
            (HudVisibility::All, _)
                | (HudVisibility::Minimal, HudTier::Instrument)
                | (HudVisibility::Minimal, HudTier::Status)
        )
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
    /// Persistent status/reference chrome (the fps/version status bar and the
    /// objective count in it): visible at `All` and `Minimal` like an
    /// instrument, but hidden ONLY at the cinematic `None` level so screenshots
    /// stay clean (task 20260724-171509). Unlike flight chrome it is meant to
    /// persist through the Tab drawer too - tag such a widget `HudDrawerExempt`
    /// as well, which keeps it visible while the drawer is open and z-lifts it
    /// above the backdrop. "It is like the FPS overlay": it rides the whole
    /// session, not the moment-to-moment flight HUD.
    Status,
}

/// Opt-out for widgets that drive their own `Visibility` every frame (the
/// gravity sphere hides itself in flat space): the level-change restore skips
/// them so it cannot stomp their state for a frame; the Hidden enforcement
/// still applies while their tier is off (review R1.2).
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct HudSelfDrivenVisibility;

/// Diagnostic/status chrome that stays visible while the Tab drawer is open.
/// NOVA OS hides ordinary flight HUD and key hints so the cockpit monitor owns
/// the screen; widgets tagged with this marker are exempt from that
/// drawer-scoped hide and z-lift above the backdrop. They are still subject to
/// the grave/tilde [`HudVisibility`] cycle. Tag the widget's tiered root.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct HudDrawerExempt;

/// Nav cyan, the family color of every flight-computer projection (the
/// destination marker tint, the orbit cue, the maneuver chips, the holo
/// ring).
pub(crate) const NAV_CYAN: Color = nova_ui::theme::semantic::NAV;

/// Objective gold, the "do this now" accent (task 20260712-093831): the
/// objective marker chip and the hint-emphasis pulse draw from it. One hue
/// per meaning - cyan is nav infrastructure, red is threat, green is
/// own/done, gold is the current objective.
pub(crate) const OBJECTIVE_GOLD: Color = nova_ui::theme::semantic::OBJECTIVE;

/// Shared HUD art handles ([`NovaHudPlugin`] inits it): the target-reticle
/// sprite reused by the lock crosshairs, torpedo reticle and destination
/// marker. Populated by asset loading; read by the per-widget setup observers.
#[derive(Resource, Clone, Default, Debug)]
pub struct NovaHudAssets {
    /// The shared target-reticle sprite (lock crosshairs, torpedo reticle, destination marker).
    pub target_sprite: Handle<Image>,
}

/// The player HUD umbrella: adds every widget sub-plugin and the observers
/// that spawn/despawn each [`HudTier`] layer with the player ship.
/// Inits [`NovaHudAssets`] and [`HudVisibility`], adds all the per-widget
/// plugins (velocity, flight status, maneuver instruments, crosshairs, insets,
/// readouts, indicators, objectives, comms, ...), runs `cycle_hud_visibility`
/// in Update within [`NovaHudSystems`], and runs `apply_hud_visibility` in
/// PostUpdate after `ScreenIndicatorSystems` and before UI layout.
#[derive(Default)]
pub struct NovaHudPlugin;

impl Plugin for NovaHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("HudPlugin: build");

        app.init_resource::<NovaHudAssets>();

        app.init_resource::<HudVisibility>();
        app.register_type::<HudVisibility>();
        app.register_type::<HudTier>();
        app.register_type::<HudDrawerExempt>();
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
        // Lift the drawer-exempt chrome above the drawer backdrop only while the
        // drawer is open (task 20260724-134335).
        app.add_systems(
            Update,
            lift_exempt_chrome_over_drawer.in_set(NovaHudSystems),
        );

        app.add_plugins(velocity::VelocityHudPlugin);
        app.add_plugins(flight_status::FlightStatusHudPlugin);
        app.add_plugins(maneuver_instruments::ManeuverInstrumentsPlugin);
        app.add_plugins(keybind_hints::KeybindHintsPlugin);
        app.add_plugins(holo_instruments::HoloInstrumentsPlugin);
        // The bcs ObjectivesPlugin owns the `GameObjectives` resource (the
        // drawer + the diegetic reveal read it) and its `rebuild_lines`
        // no-ops when no objectives panel exists. The always-on compact
        // objectives panel was REMOVED from flight (task 20260724-134312):
        // objectives now surface via the diegetic reveal, the objective hint
        // in the status bar (`objective_hint`) and the NOVA OS monitor.
        app.add_plugins(ObjectivesPlugin);
        // bcs tween advancement for HUD fades (first Nova adoption, task
        // 20260717-163033); registered here once for every HUD widget.
        app.add_plugins(bevy_common_systems::prelude::TweenPlugin);
        app.add_plugins(comms_panel::CommsPanelPlugin);
        // The Tab ship-computer drawer shell (task 20260724-102304).
        app.add_plugins(drawer::NovaDrawerPlugin);
        app.add_plugins(readout::HudReadoutPlugin);
        app.add_plugins(screen_indicator::ScreenIndicatorPlugin);
        app.add_plugins(torpedo_target::TorpedoTargetHudPlugin);
        app.add_plugins(turret_lead::TurretLeadPlugin);
        app.add_plugins(ammo_readout::AmmoReadoutPlugin);
        app.add_plugins(component_lock::ComponentLockHudPlugin);
        app.add_plugins(lock_dwell_ring::LockDwellRingHudPlugin);
        app.add_plugins(lock_crosshairs::LockCrosshairsHudPlugin);
        app.add_plugins(target_inset::TargetInsetHudPlugin);
        app.add_plugins(edge_indicators::EdgeIndicatorsHudPlugin);
        app.add_plugins(beacon_chips::BeaconChipsHudPlugin);
        app.add_plugins(allegiance_markers::AllegianceMarkerHudPlugin);
        app.add_plugins(objective_markers::ObjectiveMarkersHudPlugin);
        app.add_plugins(item_highlights::ItemHighlightsHudPlugin);
        app.add_plugins(objective_feedback::ObjectiveFeedbackPlugin);
        // The big diegetic objective reveal that tucks into the drawer tab
        // (task 20260721-211520); fed by objective_feedback's postings.
        app.add_plugins(objective_reveal::ObjectiveRevealPlugin);
        // The minimalist top-right flight objective hint (count + glyph + Tab),
        // and the source of the diegetic reveal's tuck anchor (task
        // 20260724-134312).
        app.add_plugins(objective_hint::ObjectiveHintPlugin);

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
        app.add_observer(setup_hud_torpedo_target);
        app.add_observer(remove_hud_torpedo_target);
        app.add_observer(setup_hud_turret_lead);
        app.add_observer(remove_hud_turret_lead);
        app.add_observer(setup_hud_ammo_readout);
        app.add_observer(remove_hud_ammo_readout);
        app.add_observer(setup_hud_component_lock);
        app.add_observer(remove_hud_component_lock);
        app.add_observer(setup_hud_lock_dwell_ring);
        app.add_observer(remove_hud_lock_dwell_ring);
        app.add_observer(setup_hud_lock_crosshairs);
        app.add_observer(remove_hud_lock_crosshairs);
        app.add_observer(setup_hud_target_inset);
        app.add_observer(remove_hud_target_inset);
        app.add_observer(setup_hud_edge_indicators);
        app.add_observer(remove_hud_edge_indicators);
    }
}

// The always-on compact objectives panel (spawn_objectives_panel /
// style_objective_lines / setup_hud_objectives / remove_hud_objectives) was
// REMOVED in task 20260724-134312; objectives now surface via the diegetic
// reveal, the objective hint in the status bar (`objective_hint`) and the Tab
// drawer's NOVA OS monitor.

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
    pause: Res<State<crate::PauseStates>>,
    mut q_roots: Query<
        (
            &HudTier,
            &mut Visibility,
            Has<HudSelfDrivenVisibility>,
            Has<HudDrawerExempt>,
        ),
        Without<ScreenIndicatorMarker>,
    >,
    mut q_indicators: Query<
        (
            Entity,
            &mut Visibility,
            Option<&HudTier>,
            Has<HudDrawerExempt>,
        ),
        With<ScreenIndicatorMarker>,
    >,
    q_parents: Query<&ChildOf>,
    q_tiers: Query<&HudTier>,
) {
    // While the Tab drawer is open the flight HUD hides so it does not fight the
    // NOVA OS monitor; only diagnostic/status widgets carrying `HudDrawerExempt`
    // stay. The restore branch fires on a pause change too, so CLOSING the
    // drawer un-hides in the same frame - not just on a grave/tilde level
    // change.
    let drawer_open = *pause.get() == crate::PauseStates::Drawer;
    let restore = level.is_changed() || pause.is_changed();
    for (tier, mut visibility, self_driven, exempt) in &mut q_roots {
        let shown = level.shows(*tier) && (!drawer_open || exempt);
        if !shown {
            visibility.set_if_neq(Visibility::Hidden);
        } else if restore && !self_driven {
            visibility.set_if_neq(Visibility::Inherited);
        }
    }
    for (entity, mut visibility, own_tier, exempt) in &mut q_indicators {
        let tier = own_tier
            .copied()
            .or_else(|| ancestor_tier(entity, &q_parents, &q_tiers));
        let Some(tier) = tier else {
            continue;
        };
        let shown = level.shows(tier) && (!drawer_open || exempt);
        if !shown {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

/// Lift drawer-exempt diagnostic/status chrome above the drawer backdrop only
/// while the drawer is open. Its base z is 0, so when the drawer is closed -
/// including while the pause overlay owns the freeze, which sits at the same z
/// as the drawer backdrop - the exempt chrome stays at the base HUD z and the
/// pause overlay covers it normally.
fn lift_exempt_chrome_over_drawer(
    pause: Res<State<crate::PauseStates>>,
    mut q_exempt: Query<&mut GlobalZIndex, With<HudDrawerExempt>>,
) {
    let z = if *pause.get() == crate::PauseStates::Drawer {
        drawer::DRAWER_EXEMPT_Z
    } else {
        0
    };
    for mut zindex in &mut q_exempt {
        zindex.set_if_neq(GlobalZIndex(z));
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
        // Keybind hints are ordinary flight chrome. NOVA OS owns the monitor
        // surface while the drawer is open, so only diagnostic/status chrome
        // carries `HudDrawerExempt`.
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

fn setup_hud_lock_dwell_ring(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut materials: ResMut<Assets<LockDwellRingMaterial>>,
) {
    let entity = add.entity;
    debug!("setup_hud_lock_dwell_ring: entity {:?}", entity);

    if q_spaceship.get(entity).is_err() {
        error!(
            "setup_hud_lock_dwell_ring: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    }

    let material = materials.add(LockDwellRingMaterial::default());
    commands.spawn((HudTier::Chrome, lock_dwell_ring_hud(material)));
}

fn remove_hud_lock_dwell_ring(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<Entity, With<LockDwellRingHudMarker>>,
) {
    let entity = remove.entity;
    debug!("remove_hud_lock_dwell_ring: entity {:?}", entity);

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
        // apply_hud_visibility reads the drawer axis (task 20260724-134335).
        app.init_state::<crate::PauseStates>();
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

    /// The `Status` tier (task 20260724-171509) is persistent reference chrome:
    /// it survives `Minimal` like an instrument (unlike `Chrome`), but the
    /// cinematic `None` level still clears it for a clean screenshot.
    #[test]
    fn status_tier_shows_through_minimal_and_hides_at_none() {
        let mut app = app();
        let status = app
            .world_mut()
            .spawn((HudTier::Status, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        app.update();
        assert_eq!(vis(&app, status), Visibility::Inherited, "All shows Status");

        app.insert_resource(HudVisibility::Minimal);
        app.update();
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "Minimal keeps the status bar (unlike Chrome, which drops here)"
        );

        app.insert_resource(HudVisibility::None);
        app.update();
        assert_eq!(
            vis(&app, status),
            Visibility::Hidden,
            "None clears the status bar for a cinematic capture"
        );
    }

    /// A `Status` widget tagged `HudDrawerExempt` (the real status bar's config)
    /// stays visible while the Tab drawer is open, but the cinematic `None` level
    /// still clears it even mid-drawer (task 20260724-171509).
    #[test]
    fn status_bar_persists_through_the_drawer_but_none_still_clears_it() {
        let mut app = app();
        let status = app
            .world_mut()
            .spawn((HudTier::Status, HudDrawerExempt, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        set_pause(&mut app, crate::PauseStates::Drawer);
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "the status bar stays while the drawer is open"
        );

        app.insert_resource(HudVisibility::None);
        app.update();
        assert_eq!(
            vis(&app, status),
            Visibility::Hidden,
            "None clears the status bar even during the drawer"
        );
    }

    /// The real flight status bar is `HudTier::Status` WITHOUT `HudDrawerExempt`
    /// (task 20260727-014806): opening the NOVA OS computer hides the whole flight
    /// status bar (its FPS item is rehomed onto the terminal topbar), and closing
    /// the drawer restores it in the same frame via the pause-change restore branch.
    #[test]
    fn flight_status_bar_hides_while_the_drawer_is_open_and_returns_on_close() {
        let mut app = app();
        let status = app
            .world_mut()
            .spawn((HudTier::Status, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        // Visible in normal flight (Minimal shows the Status tier).
        app.update();
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "the flight status bar is visible in normal flight"
        );

        // Opening the drawer hides it - it is no longer drawer-exempt.
        set_pause(&mut app, crate::PauseStates::Drawer);
        assert_eq!(
            vis(&app, status),
            Visibility::Hidden,
            "the flight status bar hides while the NOVA OS computer is open"
        );

        // Closing the drawer restores it (pause change fires the restore branch).
        set_pause(&mut app, crate::PauseStates::Unpaused);
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "closing the drawer brings the flight status bar back"
        );
    }

    /// The objective count is a CHILD of the status bar root with no `HudTier` of
    /// its own, so it must INHERIT the bar's visibility (task 20260724-171509):
    /// `apply_hud_visibility` manages only the tiered PARENT and must leave the
    /// child's `Visibility::Inherited` untouched, so Bevy propagation carries the
    /// bar's state (persist through the drawer, clear at None) to the count. This
    /// pins that we do NOT give the child its own tier/visibility management.
    #[test]
    fn childless_node_is_left_to_inherit_the_status_bar() {
        let mut app = app();
        let bar = app
            .world_mut()
            .spawn((HudTier::Status, HudDrawerExempt, Visibility::Inherited))
            .id();
        let child = app
            .world_mut()
            .spawn((Visibility::Inherited, ChildOf(bar)))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        // Drawer open: the bar persists; the child is never touched, so it is
        // left Inherited to follow the (visible) bar.
        set_pause(&mut app, crate::PauseStates::Drawer);
        assert_eq!(vis(&app, bar), Visibility::Inherited);
        assert_eq!(
            vis(&app, child),
            Visibility::Inherited,
            "the child is not tier-managed - it inherits the bar"
        );

        // Cinematic None: the bar hides; the child stays Inherited so propagation
        // hides it with the parent (rather than the child being independently set).
        app.insert_resource(HudVisibility::None);
        app.update();
        assert_eq!(
            vis(&app, bar),
            Visibility::Hidden,
            "None hides the bar root"
        );
        assert_eq!(
            vis(&app, child),
            Visibility::Inherited,
            "the child stays Inherited so it follows the hidden bar"
        );
    }

    fn set_pause(app: &mut App, state: crate::PauseStates) {
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(state);
        app.update();
    }

    /// Opening NOVA OS hides ordinary flight HUD and key hints so they do not
    /// float over the cockpit monitor. Diagnostic/status chrome tagged
    /// `HudDrawerExempt` remains visible above the computer.
    #[test]
    fn nova_os_hides_flight_hud_but_keeps_diagnostics() {
        let mut app = app();
        let instrument = app
            .world_mut()
            .spawn((HudTier::Instrument, Visibility::Inherited))
            .id();
        let key_hints = app
            .world_mut()
            .spawn((HudTier::Chrome, Visibility::Inherited))
            .id();
        let diagnostics = app
            .world_mut()
            .spawn((HudTier::Status, HudDrawerExempt, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        app.update();
        assert_eq!(vis(&app, instrument), Visibility::Inherited);
        assert_eq!(vis(&app, key_hints), Visibility::Inherited);
        assert_eq!(vis(&app, diagnostics), Visibility::Inherited);

        set_pause(&mut app, crate::PauseStates::Drawer);
        assert_eq!(
            vis(&app, instrument),
            Visibility::Hidden,
            "the flight HUD hides while the drawer is open"
        );
        assert_eq!(
            vis(&app, key_hints),
            Visibility::Hidden,
            "lower-left key hints are ordinary flight chrome, not diagnostics"
        );
        assert_eq!(
            vis(&app, diagnostics),
            Visibility::Inherited,
            "diagnostic/status chrome remains visible while the drawer is open"
        );

        set_pause(&mut app, crate::PauseStates::Unpaused);
        assert_eq!(
            vis(&app, instrument),
            Visibility::Inherited,
            "closing the drawer restores the flight HUD"
        );
        assert_eq!(vis(&app, key_hints), Visibility::Inherited);
        assert_eq!(vis(&app, diagnostics), Visibility::Inherited);
    }

    /// The exempt diagnostic/status chrome is lifted above the drawer
    /// backdrop ONLY while the drawer is open. When the PAUSE menu owns the
    /// freeze it drops back to the base HUD z, so the pause overlay (which sits
    /// at the same z as the drawer backdrop) still covers it - not the other way
    /// round (task 20260724-134335). The bug this pins: a static high z made the
    /// status strip poke over the pause menu.
    #[test]
    fn exempt_chrome_lifts_only_while_drawer_open() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_systems(Update, lift_exempt_chrome_over_drawer);
        let widget = app
            .world_mut()
            .spawn((HudDrawerExempt, GlobalZIndex::default()))
            .id();
        let z = |app: &App| app.world().get::<GlobalZIndex>(widget).unwrap().0;

        app.update();
        assert_eq!(z(&app), 0, "base z while unpaused");

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Drawer);
        app.update();
        assert_eq!(
            z(&app),
            drawer::DRAWER_EXEMPT_Z,
            "lifted above the backdrop while the drawer is open"
        );

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Paused);
        app.update();
        assert_eq!(
            z(&app),
            0,
            "dropped to base z when the pause menu - not the drawer - owns the freeze"
        );
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
}
