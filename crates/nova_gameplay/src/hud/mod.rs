//! The player's heads-up display: the diegetic instruments and overlays drawn
//! for the player ship (velocity/flight status, lock crosshairs and dwell rings,
//! turret lead and torpedo target reticles, ammo readouts, edge/threat
//! indicators, objective markers, the comms panel and the keybind dock). Each
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
pub mod edge_indicators;
pub mod emphasis;
pub mod flight_status;
pub mod holo_instruments;
pub mod item_highlights;
pub mod key_glyphs;
pub mod keybind_dock;
pub mod lock_crosshairs;
pub mod lock_dwell_ring;
pub mod maneuver_instruments;
pub mod nova_os;
pub mod nova_os_map;
pub mod nova_os_ship;
pub mod objective_feedback;
pub mod objective_markers;
pub mod objective_stack;
pub mod readout;
pub mod screen_indicator;
pub mod situation;
pub mod target_inset;
pub mod torpedo_target;
pub mod turret_lead;
pub mod velocity;

/// Live-tree UI layout rig shared by the world-anchored chip tests.
#[cfg(test)]
mod chip_layout_rig;

/// Live-tree rig for the NOVA OS's forwarded pointer, shared by the CRT mapping
/// and the map/ship blip click tests.
#[cfg(test)]
mod nova_os_pointer_rig;

/// Glob-import surface: `use nova_gameplay::hud::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        allegiance_markers::prelude::*, ammo_readout::prelude::*, beacon_chips::prelude::*,
        comms_panel::prelude::*, component_lock::prelude::*, edge_indicators::prelude::*,
        emphasis::prelude::*, flight_status::prelude::*, holo_instruments::prelude::*,
        item_highlights::prelude::*, key_glyphs::prelude::*, keybind_dock::prelude::*,
        lock_crosshairs::prelude::*, lock_dwell_ring::prelude::*, maneuver_instruments::prelude::*,
        nova_os::NovaOsMonitorSettings, nova_os_map::prelude::*, nova_os_ship::prelude::*,
        objective_feedback::prelude::*, objective_markers::prelude::*, objective_stack::prelude::*,
        readout::prelude::*, screen_indicator::prelude::*, situation::prelude::*,
        target_inset::prelude::*, torpedo_target::prelude::*, turret_lead::prelude::*,
        velocity::prelude::*, HudContextGate, HudNovaOsExempt, HudSelfDrivenVisibility,
        HudSituationSensing, HudTier, HudVisibility, NovaHudAssets, NovaHudPlugin, NovaHudSystems,
    };
}

/// System set that all HUD update systems belong to, for ordering against the rest of the app.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaHudSystems;

/// The contextual-situation sense, ordered before [`NovaHudSystems`] so every
/// widget driver in that set reads the same frame's [`HudSituations`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HudSituationSensing;

/// Player-facing HUD level, cycled with grave/tilde: `On` is the full
/// contextual HUD - which is already near-empty in idle cruise and surfaces
/// each element while its situation is live - and `Cinematic` clears the
/// screen entirely.
///
/// The old three-level `All`/`Minimal`/`None` triple existed because the HUD
/// showed everything all the time, so "everything minus the chrome" was a
/// useful middle. Contextual visibility IS that middle now, and a manual
/// detail dial on top of it would just be a second, competing answer to the
/// same question. The menu (nova_menu) also drives this to `Cinematic` while
/// the main menu is up.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Hash, Default, Reflect)]
#[reflect(Resource)]
pub enum HudVisibility {
    #[default]
    /// The full contextual HUD.
    On,
    /// A clean screen for cinematic captures.
    Cinematic,
}

impl HudVisibility {
    /// The cycle order behind the grave/tilde key.
    pub fn next(self) -> Self {
        match self {
            HudVisibility::On => HudVisibility::Cinematic,
            HudVisibility::Cinematic => HudVisibility::On,
        }
    }

    /// Whether HUD widgets are visible at this level. Every tier answers the
    /// same at two levels (see [`HudTier`]), so the tier is not a parameter.
    pub fn shows(self) -> bool {
        matches!(self, HudVisibility::On)
    }
}

/// The kind of HUD widget a layer is, and - the part the enforcement actually
/// uses - the marker that says this subtree is HUD-MANAGED at all. Tag the
/// widget's root where it spawns; screen-indicator nodes resolve their tier
/// from the nearest tagged ancestor (or their own tag), so reconciled children
/// (pips, brackets, arrows) inherit their module's tier automatically, and an
/// untagged tree is left alone.
///
/// Since the level cycle collapsed to On/Cinematic the three variants no
/// longer differ in what the LEVEL does to them - all show at
/// `On`, all clear at `Cinematic`. They stay because they are the vocabulary
/// the HUD is documented and reasoned in (the wiki's tier table, the NOVA OS
/// exemption rules below), and because deciding a widget's kind at its spawn
/// site is what keeps the marker honest. Whether an element is on screen RIGHT
/// NOW is the contextual question, answered per widget by [`HudContextGate`].
/// Deliberately untagged: the juice gizmo flashes (juice.rs) are combat FX,
/// not HUD, and stay visible at every level.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component)]
pub enum HudTier {
    /// Flight/combat instruments (velocity sphere, flight chips, reticles, the
    /// target inset).
    Instrument,
    /// Learning aids and secondary overlays (the keybind dock, verb cues, edge
    /// indicators, objective markers).
    Chrome,
    /// Persistent status/reference chrome (the fps/version status bar): it
    /// rides the whole session rather than the moment-to-moment flight HUD,
    /// and the cinematic level still clears it so screenshots stay clean.
    /// Meant to persist
    /// through the Tab NOVA OS too - tag such a widget `HudNovaOsExempt` as
    /// well, which keeps it visible while the NOVA OS is open and z-lifts it
    /// above the backdrop.
    Status,
}

/// The contextual show rule on a HUD widget: `false` hides the widget (and its
/// screen-indicator descendants) even at [`HudVisibility::On`], because its
/// situation is not live. Absent means "always on",
/// which is what most widgets are - either they are unconditional (the velocity
/// shader, the speed chip) or they already gate themselves by driving their own
/// anchor/visibility (the mode chip, the reticle, the target inset).
///
/// Written by the widget's OWN driver from [`HudSituations`], enforced centrally
/// in [`apply_hud_visibility`] - because a projected indicator re-asserts
/// `Visibility::Visible` every frame downstream of any Update-schedule writer,
/// exactly like the level enforcement it rides along with.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component)]
pub struct HudContextGate(pub bool);

impl Default for HudContextGate {
    /// Open: a gate only ever narrows what the level already allows.
    fn default() -> Self {
        Self(true)
    }
}

/// Opt-out for widgets that drive their own `Visibility` every frame (the
/// gravity sphere hides itself in flat space): the level-change restore skips
/// them so it cannot stomp their state for a frame; the Hidden enforcement
/// still applies while their tier is off.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct HudSelfDrivenVisibility;

/// Diagnostic/status chrome that stays visible while the Tab NOVA OS is open.
/// NOVA OS hides ordinary flight HUD and key hints so the cockpit monitor owns
/// the screen; widgets tagged with this marker are exempt from that
/// NOVA OS-scoped hide and z-lift above the backdrop. They are still subject to
/// the grave/tilde [`HudVisibility`] cycle. Tag the widget's tiered root.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct HudNovaOsExempt;

/// Nav cyan, the family color of every flight-computer projection (the
/// destination marker tint, the orbit cue, the maneuver chips, the holo
/// ring).
pub(crate) const NAV_CYAN: Color = nova_ui::theme::semantic::NAV;

/// Objective gold, the "do this now" accent: the objective marker chip and
/// the hint-emphasis pulse draw from it. One hue
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
    /// The NOVA CRT brand mark (the NOVA OS drawer plate logo).
    pub nova_crt_mark: Handle<Image>,
    /// The preloaded keycap glyphs, keyed by display label - the icon dock, the
    /// anchored verb cues and the objective stack's Tab affordance draw from
    /// here (see [`key_glyphs`]). Empty on bare-app rigs, which fall back to
    /// text chips.
    pub key_glyphs: key_glyphs::KeyGlyphs,
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
        app.register_type::<HudContextGate>();
        app.register_type::<HudNovaOsExempt>();

        // NOTE: the contextual layer is bounded on both sides of the widget
        // drivers: the situations are sensed BEFORE
        // [`NovaHudSystems`], so every driver in that set reads this frame's
        // truth, and the shared emphasis is applied in PostUpdate, so a
        // driver's `set_held`/`pop` lands in the SAME frame it is written
        // rather than by schedule tie-break.
        app.init_resource::<situation::HudSituations>();
        app.register_type::<situation::HudSituations>();
        app.register_type::<emphasis::HudEmphasis>();
        app.add_systems(
            Update,
            situation::sense_hud_situations
                .in_set(HudSituationSensing)
                .before(NovaHudSystems),
        );
        app.add_systems(
            PostUpdate,
            emphasis::drive_hud_emphasis.before(bevy::ui::UiSystems::Layout),
        );
        // The cycle key is gameplay-only (the menu drives the resource
        // itself); plain ButtonInput, same pattern as the debug F11 toggle.
        app.add_systems(
            Update,
            cycle_hud_visibility.run_if(in_state(crate::GameStates::Playing)),
        );
        // NOTE: visibility enforcement runs AFTER the screen-indicator
        // projection: the widget writes Visibility::Visible on its nodes in
        // PostUpdate (ignoring hidden ancestors), so a tier-hidden node must
        // be overwritten downstream of that producer, not from Update.
        // Bounded on both sides: after the indicator projection (whose
        // Visible writes it must overrule) and before UI layout - which runs
        // upstream of transform + visibility propagation - so the writes land
        // in THIS frame's propagation deterministically instead of by
        // schedule tie-break.
        app.add_systems(
            PostUpdate,
            apply_hud_visibility
                .after(ScreenIndicatorSystems)
                .before(bevy::ui::UiSystems::Layout),
        );
        // Lift the NOVA OS-exempt chrome above the NOVA OS backdrop only while
        // the NOVA OS is open.
        app.add_systems(
            Update,
            lift_exempt_chrome_over_nova_os.in_set(NovaHudSystems),
        );

        app.add_plugins(velocity::VelocityHudPlugin);
        app.add_plugins(flight_status::FlightStatusHudPlugin);
        app.add_plugins(maneuver_instruments::ManeuverInstrumentsPlugin);
        app.add_plugins(keybind_dock::KeybindDockPlugin);
        app.add_plugins(holo_instruments::HoloInstrumentsPlugin);
        // The bcs ObjectivesPlugin owns the `GameObjectives` resource (the
        // NOVA OS + the objective stack read it) and its `rebuild_lines`
        // no-ops when no objectives panel exists. There is no always-on compact
        // objectives panel any more: objectives surface via the objective stack
        // (`objective_stack`) and the NOVA OS monitor.
        app.add_plugins(ObjectivesPlugin);
        // bcs tween advancement for HUD fades; registered here once for every
        // HUD widget.
        app.add_plugins(bevy_common_systems::prelude::TweenPlugin);
        app.add_plugins(comms_panel::CommsPanelPlugin);
        // The Tab ship-computer NOVA OS shell.
        app.add_plugins(nova_os::NovaOsPlugin);
        // The `map` NOVA OS app: registers the app + drives its schematic 3D
        // scene, orbit camera, blips and GOTO. NOTE: added AFTER NovaOsPlugin
        // so the app registry it registers into exists.
        app.add_plugins(nova_os_map::NovaOsMapPlugin);
        // The `ship` NOVA OS app: registers the `ship` app +
        // `view`/`section`/`reload`/`repair` verbs and drives the schematic 3D
        // scene, orbit camera, blips and section actions. NOTE: added AFTER
        // NovaOsPlugin so the command registry exists.
        app.add_plugins(nova_os_ship::NovaOsShipPlugin);
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
        // The top-centre objective NOTIFICATION stack: demo 2's objective chip,
        // one per posting, read by its dwell or by opening NOVA OS. The chip IS
        // the posting - it spawns and pops the frame the objective arrives,
        // replacing both the top-right status-bar hint and the diegetic
        // cockpit reveal card.
        app.add_plugins(objective_stack::ObjectiveStackPlugin);

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
        app.add_observer(objective_stack::setup_objective_stack);
        app.add_observer(objective_stack::remove_objective_stack);
    }
}

/// Cycle the HUD level on grave/tilde (or the gamepad Select button).
/// Press-to-cycle, no hold gesture: two states, so one press round-trips.
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

/// Enforce the current [`HudVisibility`] level and every [`HudContextGate`] on
/// the tagged widgets.
///
/// Two passes:
/// - Tagged roots (and tagged world-space instruments like ribbon segments):
///   hidden while the level is cinematic or their own gate is shut, restored to
///   `Inherited` once when either changes back. Self-driving widgets (the
///   gravity sphere) re-assert their own state every frame, so the one-shot
///   restore cannot wedge them.
/// - Screen-indicator nodes: their projection re-writes `Visibility::Visible`
///   every frame in this same schedule, so the hidden ones are overwritten here
///   (after the projection) every frame; no restore branch is needed because
///   the widget re-drives them. Tier and gate both resolve from the node or its
///   nearest tagged ancestor; untagged trees are not HUD-managed.
fn apply_hud_visibility(
    level: Res<HudVisibility>,
    pause: Res<State<crate::PauseStates>>,
    // The tier tag is a FILTER here: at two levels it no longer selects a
    // different answer, it only says "this root is HUD-managed".
    mut q_roots: Query<
        (
            Option<Ref<HudContextGate>>,
            &mut Visibility,
            Has<HudSelfDrivenVisibility>,
            Has<HudNovaOsExempt>,
        ),
        (With<HudTier>, Without<ScreenIndicatorMarker>),
    >,
    mut q_indicators: Query<
        (Entity, &mut Visibility, Has<HudNovaOsExempt>),
        With<ScreenIndicatorMarker>,
    >,
    q_parents: Query<&ChildOf>,
    q_tiers: Query<&HudTier>,
    q_gates: Query<&HudContextGate>,
) {
    // While the Tab NOVA OS is open the flight HUD hides so it does not fight the
    // NOVA OS monitor; only diagnostic/status widgets carrying `HudNovaOsExempt`
    // stay. The restore branch fires on a pause change too, so CLOSING the
    // NOVA OS un-hides in the same frame - not just on a grave/tilde level
    // change.
    let nova_os_open = *pause.get() == crate::PauseStates::NovaOs;
    let level_restore = level.is_changed() || pause.is_changed();
    for (gate, mut visibility, self_driven, exempt) in &mut q_roots {
        let open = gate.as_ref().is_none_or(|gate| gate.0);
        let shown = level.shows() && open && (!nova_os_open || exempt);
        // A gate that just opened restores this widget even on a quiet level -
        // that is the whole point of a contextual show.
        let restore = level_restore || gate.is_some_and(|gate| gate.is_changed());
        if !shown {
            visibility.set_if_neq(Visibility::Hidden);
        } else if restore && !self_driven {
            visibility.set_if_neq(Visibility::Inherited);
        }
    }
    for (entity, mut visibility, exempt) in &mut q_indicators {
        // ONE walk answers both questions: is this subtree HUD-managed (does a
        // tier tag sit on the node or above it - an untagged tree is somebody
        // else's UI and is left alone), and is the nearest gate on that chain
        // open.
        let (managed, open) = resolve_chain(entity, &q_parents, &q_tiers, &q_gates);
        if !managed {
            continue;
        }
        let shown = level.shows() && open && (!nova_os_open || exempt);
        if !shown {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

/// Lift NOVA OS-exempt diagnostic/status chrome above the NOVA OS backdrop only
/// while the NOVA OS is open. Its base z is 0, so when the NOVA OS is closed -
/// including while the pause overlay owns the freeze, which sits at the same z
/// as the NOVA OS backdrop - the exempt chrome stays at the base HUD z and the
/// pause overlay covers it normally.
fn lift_exempt_chrome_over_nova_os(
    pause: Res<State<crate::PauseStates>>,
    mut q_exempt: Query<&mut GlobalZIndex, With<HudNovaOsExempt>>,
) {
    let z = if *pause.get() == crate::PauseStates::NovaOs {
        nova_os::DRAWER_EXEMPT_Z
    } else {
        0
    };
    for mut zindex in &mut q_exempt {
        zindex.set_if_neq(GlobalZIndex(z));
    }
}

/// Walk from `entity` up its ancestors once, answering both questions the
/// indicator pass needs: is a [`HudTier`] tag present anywhere on the chain (so
/// the subtree is HUD-managed at all), and is the NEAREST [`HudContextGate`] on
/// it open. No gate anywhere on the chain means always-on.
fn resolve_chain(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    tiers: &Query<&HudTier>,
    gates: &Query<&HudContextGate>,
) -> (bool, bool) {
    let mut managed = false;
    let mut open = None;
    loop {
        managed |= tiers.contains(entity);
        if open.is_none() {
            if let Ok(gate) = gates.get(entity) {
                open = Some(gate.0);
            }
        }
        // Keep walking even once the gate is known: the tier tag can sit above
        // it (the ammo layer carries both, but a widget may split them).
        let Ok(ChildOf(parent)) = parents.get(entity) else {
            return (managed, open.unwrap_or(true));
        };
        entity = *parent;
    }
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
    // NOTE: try_remove, not remove: get_entity only proves the entity exists
    // at QUEUE time - a scenario teardown despawns the camera in the same
    // command flush, and the plain remove then warns "entity despawned".
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
        // NOTE: hides itself in flat space; the level-change restore must not
        // overrule that.
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
    q_existing_dock: Query<(), With<KeybindDockMarker>>,
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
    // The dock and cues are global singletons, not ship-targeted widgets: one
    // player, one set (same guard as the flight input rig).
    if q_existing_dock.is_empty() {
        // Keybind hints are ordinary flight chrome. NOVA OS owns the monitor
        // surface while the NOVA OS is open, so only diagnostic/status chrome
        // carries `HudNovaOsExempt`.
        commands.spawn((HudTier::Chrome, keybind_dock_hud()));
        commands.spawn((HudTier::Chrome, verb_cues_hud()));
    }
}

fn remove_hud_flight_status(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hud: Query<(Entity, &FlightStatusHudTargetEntity), With<FlightStatusHudMarker>>,
    q_destination: Query<Entity, With<AutopilotDestinationHudMarker>>,
    q_dock: Query<Entity, With<KeybindDockMarker>>,
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
    for hud_entity in &q_dock {
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
        // apply_hud_visibility reads the NOVA OS axis.
        app.init_state::<crate::PauseStates>();
        app.init_resource::<HudVisibility>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(
            Update,
            cycle_hud_visibility.run_if(in_state(crate::GameStates::Playing)),
        );
        app.add_systems(PostUpdate, fake_widget_drive.in_set(ScreenIndicatorSystems));
        // Same double-bounded registration as the plugin.
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
    /// is asserted after every individual press, not just at the end. Two
    /// levels, so one press round-trips.
    #[test]
    fn backquote_cycles_on_cinematic() {
        let mut app = app();
        assert_eq!(level(&app), HudVisibility::On);
        press_backquote(&mut app);
        assert_eq!(level(&app), HudVisibility::Cinematic);
        press_backquote(&mut app);
        assert_eq!(level(&app), HudVisibility::On);
    }

    /// Every tier answers the level the same way now: on at `On`, cleared at
    /// `Cinematic`. What differs per widget is the CONTEXTUAL gate, pinned
    /// separately below.
    #[test]
    fn every_tier_hides_at_cinematic_and_restores_at_on() {
        let mut app = app();
        let spawn = |app: &mut App, tier: HudTier| {
            app.world_mut().spawn((tier, Visibility::Inherited)).id()
        };
        let instrument = spawn(&mut app, HudTier::Instrument);
        let chrome = spawn(&mut app, HudTier::Chrome);
        let status = spawn(&mut app, HudTier::Status);
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        app.update();
        for (entity, name) in [
            (instrument, "instrument"),
            (chrome, "chrome"),
            (status, "status"),
        ] {
            assert_eq!(vis(&app, entity), Visibility::Inherited, "On shows {name}");
        }

        app.insert_resource(HudVisibility::Cinematic);
        app.update();
        for (entity, name) in [
            (instrument, "instrument"),
            (chrome, "chrome"),
            (status, "status"),
        ] {
            assert_eq!(
                vis(&app, entity),
                Visibility::Hidden,
                "Cinematic clears {name}"
            );
        }

        app.insert_resource(HudVisibility::On);
        app.update();
        for (entity, name) in [
            (instrument, "instrument"),
            (chrome, "chrome"),
            (status, "status"),
        ] {
            assert_eq!(
                vis(&app, entity),
                Visibility::Inherited,
                "back On restores {name}"
            );
        }
    }

    /// A shut [`HudContextGate`] hides its widget even at `On`, and opening it
    /// brings the widget back without any level change - the contextual show.
    #[test]
    fn a_shut_context_gate_hides_the_widget_at_on() {
        let mut app = app();
        let gated = app
            .world_mut()
            .spawn((
                HudTier::Instrument,
                HudContextGate(false),
                Visibility::Inherited,
            ))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        app.update();
        assert_eq!(
            vis(&app, gated),
            Visibility::Hidden,
            "the situation is not live, so the widget is off even at On"
        );

        app.world_mut()
            .entity_mut(gated)
            .insert(HudContextGate(true));
        app.update();
        assert_eq!(
            vis(&app, gated),
            Visibility::Inherited,
            "the situation went live, so the widget comes back with no level change"
        );

        // ... and the level still wins over an open gate.
        app.insert_resource(HudVisibility::Cinematic);
        app.update();
        assert_eq!(
            vis(&app, gated),
            Visibility::Hidden,
            "Cinematic clears the screen whatever the gates say"
        );
    }

    /// A projected indicator resolves its gate from its ancestor layer, the same
    /// walk the tier uses - so the ammo readouts (indicators under a gated
    /// layer) are overwritten every frame downstream of their own projection.
    #[test]
    fn indicators_inherit_a_shut_gate_from_their_layer() {
        let mut app = app();
        let layer = app
            .world_mut()
            .spawn((
                HudTier::Instrument,
                HudContextGate(false),
                Visibility::Inherited,
            ))
            .id();
        let indicator = app
            .world_mut()
            .spawn((ScreenIndicatorMarker, Visibility::Inherited, ChildOf(layer)))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        // fake_widget_drive writes Visible on the indicator every frame; the
        // enforcement must still win.
        app.update();
        app.update();
        assert_eq!(
            vis(&app, indicator),
            Visibility::Hidden,
            "the shut gate beats the projection's every-frame Visible"
        );

        app.world_mut()
            .entity_mut(layer)
            .insert(HudContextGate(true));
        app.update();
        assert_eq!(
            vis(&app, indicator),
            Visibility::Visible,
            "an open gate leaves the projection's own write alone"
        );
    }

    /// A `Status` widget tagged `HudNovaOsExempt` (the real status bar's config)
    /// stays visible while the Tab NOVA OS is open, but the cinematic `None`
    /// level still clears it even mid-NOVA OS.
    #[test]
    fn status_bar_persists_through_the_nova_os_but_cinematic_still_clears_it() {
        let mut app = app();
        let status = app
            .world_mut()
            .spawn((HudTier::Status, HudNovaOsExempt, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        set_pause(&mut app, crate::PauseStates::NovaOs);
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "the status bar stays while the NOVA OS is open"
        );

        app.insert_resource(HudVisibility::Cinematic);
        app.update();
        assert_eq!(
            vis(&app, status),
            Visibility::Hidden,
            "Cinematic clears the status bar even during the NOVA OS"
        );
    }

    /// The real flight status bar is `HudTier::Status` WITHOUT
    /// `HudNovaOsExempt`: opening the NOVA OS computer hides the whole flight
    /// status bar (its FPS item is rehomed onto the terminal topbar), and
    /// closing the NOVA OS restores it in the same frame via the pause-change
    /// restore branch.
    #[test]
    fn flight_status_bar_hides_while_the_nova_os_is_open_and_returns_on_close() {
        let mut app = app();
        let status = app
            .world_mut()
            .spawn((HudTier::Status, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        // Visible in normal flight.
        app.update();
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "the flight status bar is visible in normal flight"
        );

        // Opening the NOVA OS hides it - it is no longer NOVA OS-exempt.
        set_pause(&mut app, crate::PauseStates::NovaOs);
        assert_eq!(
            vis(&app, status),
            Visibility::Hidden,
            "the flight status bar hides while the NOVA OS computer is open"
        );

        // Closing the NOVA OS restores it (pause change fires the restore branch).
        set_pause(&mut app, crate::PauseStates::Unpaused);
        assert_eq!(
            vis(&app, status),
            Visibility::Inherited,
            "closing the NOVA OS brings the flight status bar back"
        );
    }

    /// A childless status-bar item is a CHILD of the status bar root with no
    /// `HudTier` of its own, so it must INHERIT the bar's visibility:
    /// `apply_hud_visibility` manages only the tiered PARENT and must leave the
    /// child's `Visibility::Inherited` untouched, so Bevy propagation carries the
    /// bar's state (persist through the NOVA OS, clear at None) to the count. This
    /// pins that we do NOT give the child its own tier/visibility management.
    #[test]
    fn childless_node_is_left_to_inherit_the_status_bar() {
        let mut app = app();
        let bar = app
            .world_mut()
            .spawn((HudTier::Status, HudNovaOsExempt, Visibility::Inherited))
            .id();
        let child = app
            .world_mut()
            .spawn((Visibility::Inherited, ChildOf(bar)))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        // NovaOs open: the bar persists; the child is never touched, so it is
        // left Inherited to follow the (visible) bar.
        set_pause(&mut app, crate::PauseStates::NovaOs);
        assert_eq!(vis(&app, bar), Visibility::Inherited);
        assert_eq!(
            vis(&app, child),
            Visibility::Inherited,
            "the child is not tier-managed - it inherits the bar"
        );

        // Cinematic: the bar hides; the child stays Inherited so propagation
        // hides it with the parent (rather than the child being independently set).
        app.insert_resource(HudVisibility::Cinematic);
        app.update();
        assert_eq!(
            vis(&app, bar),
            Visibility::Hidden,
            "Cinematic hides the bar root"
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
    /// `HudNovaOsExempt` remains visible above the computer.
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
            .spawn((HudTier::Status, HudNovaOsExempt, Visibility::Inherited))
            .id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();

        app.update();
        assert_eq!(vis(&app, instrument), Visibility::Inherited);
        assert_eq!(vis(&app, key_hints), Visibility::Inherited);
        assert_eq!(vis(&app, diagnostics), Visibility::Inherited);

        set_pause(&mut app, crate::PauseStates::NovaOs);
        assert_eq!(
            vis(&app, instrument),
            Visibility::Hidden,
            "the flight HUD hides while the NOVA OS is open"
        );
        assert_eq!(
            vis(&app, key_hints),
            Visibility::Hidden,
            "lower-left key hints are ordinary flight chrome, not diagnostics"
        );
        assert_eq!(
            vis(&app, diagnostics),
            Visibility::Inherited,
            "diagnostic/status chrome remains visible while the NOVA OS is open"
        );

        set_pause(&mut app, crate::PauseStates::Unpaused);
        assert_eq!(
            vis(&app, instrument),
            Visibility::Inherited,
            "closing the NOVA OS restores the flight HUD"
        );
        assert_eq!(vis(&app, key_hints), Visibility::Inherited);
        assert_eq!(vis(&app, diagnostics), Visibility::Inherited);
    }

    /// The exempt diagnostic/status chrome is lifted above the NOVA OS
    /// backdrop ONLY while the NOVA OS is open. When the PAUSE menu owns the
    /// freeze it drops back to the base HUD z, so the pause overlay (which sits
    /// at the same z as the NOVA OS backdrop) still covers it - not the other
    /// way round. The bug this pins: a static high z made the status strip
    /// poke over the pause menu.
    #[test]
    fn exempt_chrome_lifts_only_while_nova_os_open() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_systems(Update, lift_exempt_chrome_over_nova_os);
        let widget = app
            .world_mut()
            .spawn((HudNovaOsExempt, GlobalZIndex::default()))
            .id();
        let z = |app: &App| app.world().get::<GlobalZIndex>(widget).unwrap().0;

        app.update();
        assert_eq!(z(&app), 0, "base z while unpaused");

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::NovaOs);
        app.update();
        assert_eq!(
            z(&app),
            nova_os::DRAWER_EXEMPT_Z,
            "lifted above the backdrop while the NOVA OS is open"
        );

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Paused);
        app.update();
        assert_eq!(
            z(&app),
            0,
            "dropped to base z when the pause menu - not the NOVA OS - owns the freeze"
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

        app.insert_resource(HudVisibility::Cinematic);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(node).unwrap(),
            Visibility::Hidden
        );

        // The in-schedule stand-in re-drives the node to Visible inside
        // ScreenIndicatorSystems every frame; enforcement must win the SAME
        // frame, every frame, even though the level did not change. This is
        // the executable form of the ordering contract: moving
        // apply_hud_visibility before the set fails here.
        app.update();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(node).unwrap(),
            Visibility::Hidden
        );

        // Back On the enforcement stands down and the widget owns it.
        app.insert_resource(HudVisibility::On);
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

        app.insert_resource(HudVisibility::Cinematic);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(sphere).unwrap(),
            Visibility::Hidden
        );

        // Restoring to On must NOT stomp the widget's own Hidden.
        app.insert_resource(HudVisibility::On);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(sphere).unwrap(),
            Visibility::Hidden,
            "restore must skip self-driven widgets"
        );
    }
}
