//! ui_app_variants: map, ship and inventory screens in themed normal UI
//! instead of the NOVA OS CRT.
//!
//! Map and Ship are real 3D views. Each is its own camera drawing into an
//! image that fills its pane. Drag a pane to orbit it and turn the wheel to
//! zoom. A selection eases the view onto the contact or section. On the map,
//! W/A/S/D move the camera across the plane, Space moves it up and Shift
//! down, and Reframe brings it back to its opening framing. Each contact
//! wears the icon of what it is (ship, asteroid, planet, objective) in its
//! stance colour, and a legend names the kinds the map plots. On the ship, a
//! side panel details the selected section, Prev/Next step through the
//! sections, Fit frames the whole hull and Reset also restores the opening
//! angles, and a legend under the view names the section kinds. The data,
//! geometry and orbit feel come from NOVA OS: the `MapContacts` and
//! `ShipSections` models, the map ring, framing and zoom helpers, the ship
//! block and framing helpers, and the shared orbit gesture, zoom and center
//! ease. The palette, icons, blips, selection and pane lifecycle belong to
//! this example. NOVA OS and its TAB binding are unchanged.
//!
//! The scenario puts a catalog picket under the player, with a raider, a
//! hauler and two rocks in range. The simulation is paused while the screens
//! are up, which in this example is always, and player control stays
//! suspended, so no key flies the ship. The flight HUD is hidden.
//!
//! The Undocked/Station/Boarded control is a local mock context: nothing
//! docks or boards. Station names a mock station that is not in the scenario;
//! Boarded names the raider. A context line above every view shows the
//! context, the fixture credits and the last transaction's result, and holds
//! no controls. At the station, the ship panel prices a repair of the
//! selected section while the fixture repair bay is on. The inventory shows
//! the picket's hold under a weight bar in the left half and the station
//! market or the raider's hold in the right half, which stays empty
//! undocked. Category buttons filter the rows. Click a row to select it: the
//! inspector shows it and opens the one deal the context allows on it at one
//! unit, Buy or Sell at the station and Loot from the raider's hold. The
//! wheel over the quantity row, the slider, the typed quantity and All set
//! one quantity. The slider runs from zero, an empty track that Confirm
//! refuses, to the stock, and hides over a stock of one. Nothing moves until
//! Confirm, which then closes the deal. A buy costs and a sale pays credits;
//! loot is free and moves one way. Undocked, the inspector only informs.
//! Every refusal names its reason and changes nothing: an empty or zero
//! quantity, more than the store holds, a full hold or too few credits. The
//! screens click, tick and refuse with the game's interface cues. All of it
//! is example-local fixture state: cargo, prices, credits and section
//! condition are mock data, gameplay health and cargo are never written, and
//! nothing persists.
//!
//! ```text
//! cargo run --example ui_app_variants --features debug
//! ```
//!
//! Harnessed modes:
//! - `NOVA_AUTOPILOT=1`: click every tab, a map contact, a ship section, each
//!   context and the theme; drag and wheel both panes, recenter and reframe
//!   the map, fly the map camera, step, fit and reset the ship, inspect and
//!   filter cargo, click rows to buy, sell and loot, set the quantity by
//!   wheel, slider, typing and All, type and slide quantities Confirm refuses,
//!   double-click a row and Confirm, repair sections, switch the repair bay,
//!   open and close NOVA OS, and press Escape. Assert the panes and scenes and
//!   their constant size, the one live 3D scene that a context change keeps,
//!   the selection details and ease, the camera moves, the map legend, the
//!   section icons and bow arrow, every transaction's exact effect and every
//!   refusal's, the deal a row opens per context, that a double click trades
//!   once, the interface cue of each control, the equal store columns, the
//!   rows, filters and weight bars, that a selection, a quantity change or a
//!   refused Confirm respawns no inventory or context-line node, that a
//!   closed deal gives the keyboard back, the context line, the 3D repaint,
//!   that the simulation never advances and that no input reaches the ship.
//!   Repeat at a mid and a narrow window size, scroll a short window, then
//!   exit.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>`: the same walk,
//!   plus a frame of every view.

use avian3d::prelude::{Physics, PhysicsTime};
#[cfg(feature = "debug")]
use bevy::ecs::system::RunSystemOnce;
use bevy::{
    asset::RenderAssetUsages,
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    input::mouse::MouseWheel,
    picking::hover::Hovered,
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui_widgets::{
        Activate, Button, Slider, SliderPrecision, SliderRange, SliderStep, SliderValue,
        TrackClick, ValueChange,
    },
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_enhanced_input::prelude::ActionSources;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_input::prelude::{ActionContext, ActiveContexts};
use nova_protocol::{
    nova_os_ui::prelude::{
        cuboid_edges, ease_orbit_center, map_radius_default, map_radius_max, map_ring_radii,
        map_spread, orbit_eye, ship_framing, zoom_radius, MapContactCode, MapContactKind,
        MapContacts, OrbitGesture, ShipSections, MAP_RADIUS_MIN, SHIP_BLOCK_FILL_SCALE,
        SHIP_RADIUS_MAX, SHIP_RADIUS_MIN,
    },
    prelude::*,
};
use nova_ui::{
    prelude::*,
    screen::{clamp_stored_scroll, drive_wheel_scroll},
    theme::{ActiveUiTheme, UiColor, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
    widget::{ButtonSpec, ThemedBorder, ThemedFill, ThemedImageTint, ThemedText, UiText},
};

#[derive(Parser)]
#[command(name = "ui_app_variants")]
#[command(version = "1.0.0")]
#[command(
    about = "Click through themed 3D map, 3D ship and inventory screens",
    long_about = None
)]
struct Cli;

const SCENARIO_ID: &str = "ui_app_variants";

// The `Name`s the harness clicks and asserts by.
const TAB_MAP: &str = "Sketch Tab Map";
const TAB_SHIP: &str = "Sketch Tab Ship";
const TAB_INVENTORY: &str = "Sketch Tab Inventory";
const CONTEXT_UNDOCKED: &str = "Sketch Context Undocked";
const CONTEXT_STATION: &str = "Sketch Context Station";
const CONTEXT_BOARDED: &str = "Sketch Context Boarded";
const THEME_PHOSPHOR: &str = "Sketch Theme Phosphor";
const THEME_HARDWARE: &str = "Sketch Theme Hardware";
const PANE_MAP: &str = "Sketch Pane Map";
const PANE_SHIP: &str = "Sketch Pane Ship";
const PANE_HOLD: &str = "Sketch Pane Hold";
const MAP_SCENE: &str = "Sketch Map Scene";
const SHIP_SCENE: &str = "Sketch Ship Scene";
const MAP_READOUT: &str = "Sketch Map Readout";
const MAP_REFRAME: &str = "Sketch Map Reframe";
const MAP_LEGEND: &str = "Sketch Map Legend";
const SHIP_PREV: &str = "Sketch Ship Prev";
const SHIP_NEXT: &str = "Sketch Ship Next";
const SHIP_FIT: &str = "Sketch Ship Fit";
const SHIP_RESET: &str = "Sketch Ship Reset";
const SHIP_PANEL: &str = "Sketch Ship Panel";
const SHIP_PREVIEW: &str = "Sketch Ship Preview";
const SHIP_DETAIL: &str = "Sketch Ship Detail";
const SHIP_STATUS: &str = "Sketch Ship Status";
const SHIP_CONDITION: &str = "Sketch Ship Condition";
const SHIP_SERVICE: &str = "Sketch Ship Service";
const SHIP_REPAIR: &str = "Sketch Ship Repair";
const SHIP_NO_REPAIR: &str = "Sketch Ship No Repair";
const SHIP_BAY: &str = "Sketch Ship Bay";
const DOCK_BANNER: &str = "Sketch Dock Banner";
const DOCK_HEAD: &str = "Sketch Dock Head";
const DOCK_PARTNER: &str = "Sketch Dock Partner";
const DOCK_CREDITS: &str = "Sketch Dock Credits";
const DOCK_NOTICE: &str = "Sketch Dock Notice";
const INSPECTOR: &str = "Sketch Inspector";
const INSPECT_HINT: &str = "Sketch Inspect Hint";
const INSPECT_ICON: &str = "Sketch Inspect Icon";
const INSPECT_NAME: &str = "Sketch Inspect Name";
const INSPECT_CATEGORY: &str = "Sketch Inspect Category";
const INSPECT_ABOUT: &str = "Sketch Inspect About";
const INSPECT_MASS: &str = "Sketch Inspect Mass";
const INSPECT_STOCK: &str = "Sketch Inspect Stock";
const INSPECT_PRICE: &str = "Sketch Inspect Price";
const DEAL_TITLE: &str = "Sketch Deal Title";
const DEAL_ROW: &str = "Sketch Deal Row";
const DEAL_FIELD: &str = "Sketch Deal Field";
const DEAL_QTY: &str = "Sketch Deal Qty";
const DEAL_SLIDER: &str = "Sketch Deal Slider";
const DEAL_TOTAL: &str = "Sketch Deal Total";
const DEAL_HOLD: &str = "Sketch Deal Hold";
const DEAL_ALL: &str = "Sketch Deal All";
const DEAL_CONFIRM: &str = "Sketch Deal Confirm";
const FILTER_ALL: &str = "Sketch Filter All";
const STORE_COLUMN_OWN: &str = "Sketch Store Column Own";
const STORE_COLUMN_PARTNER: &str = "Sketch Store Column Partner";

/// A scenario ship the screens name.
#[derive(PartialEq, Debug)]
struct FixtureShip {
    /// Scenario object id.
    id: &'static str,
    name: &'static str,
    /// Catalog design id: what the spawn builds.
    design: &'static str,
}

/// The player's ship.
static OWN_SHIP: FixtureShip = FixtureShip {
    id: "player",
    name: "Picket",
    design: "block_picket",
};

/// The ship the Boarded context names. A fixture: it is only parked in
/// range, nothing boards it, and its hold is mock loot.
static BOARDED: FixtureShip = FixtureShip {
    id: "raider",
    name: "Raider",
    design: "block_gunship",
};

/// The station the Station context names. A mock: the scenario has no
/// station object, and nothing docks.
const STATION_NAME: &str = "Mock station";

/// Fixture credits the player starts the run with. No wallet runtime exists.
const START_CREDITS: u32 = 1200;
/// Fixture repair price per missing condition point.
const REPAIR_CR_PER_POINT: u32 = 12;
/// Fixture wear: the condition in percent each listed section starts at. A
/// section not listed starts intact. The spawned ship takes no damage, so
/// without this there is nothing to repair; gameplay health is never written.
static WEAR: [(&str, u32); 2] = [("PDC-1", 45), ("THR-1", 20)];

/// Render layers and camera orders of the two example scenes. Clear of the
/// world (0), the NOVA OS RTT (20), its map (21) and ship (22), and the menu
/// (23), so NOVA OS can open over this example and draw its own.
const MAP_LAYER: usize = 24;
const SHIP_LAYER: usize = 25;
const MAP_CAMERA_ORDER: isize = -40;
const SHIP_CAMERA_ORDER: isize = -41;

/// The NOVA OS map's opening angles, so both viewers open on one framing.
const MAP_THETA: f32 = 0.8;
const MAP_PHI: f32 = 0.62;
/// Smallest the hub is drawn, whatever the player hull measures.
const MAP_HUB_MIN: Meters = Meters(16.0);
const SHIP_THETA: f32 = 0.7;
const SHIP_PHI: f32 = 0.5;
/// Share of the NOVA OS ship framing distance this pane uses. The pane is far
/// larger than the CRT panel, so the hull can sit closer and fill it.
const SHIP_PANE_ZOOM: f32 = 0.72;

/// Which screen is up.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SketchView {
    #[default]
    Map,
    Ship,
    Inventory,
}

impl SketchView {
    const ALL: [(Self, &'static str, &'static str); 3] = [
        (Self::Map, "Map", TAB_MAP),
        (Self::Ship, "Ship", TAB_SHIP),
        (Self::Inventory, "Inventory", TAB_INVENTORY),
    ];

    /// The pane this view shows, and every other view must not.
    #[cfg(feature = "debug")]
    fn pane(self) -> &'static str {
        match self {
            Self::Map => PANE_MAP,
            Self::Ship => PANE_SHIP,
            Self::Inventory => PANE_HOLD,
        }
    }

    #[cfg(feature = "debug")]
    fn slug(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Ship => "ship",
            Self::Inventory => "inventory",
        }
    }
}

/// Where the screens say the ship is: in open space, at the mock station, or
/// alongside the raider it has boarded. A local flag: nothing docks or
/// boards.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SketchContext {
    #[default]
    Undocked,
    Station,
    Boarded,
}

impl SketchContext {
    const ALL: [(Self, &'static str, &'static str); 3] = [
        (Self::Undocked, "Undocked", CONTEXT_UNDOCKED),
        (Self::Station, "Station", CONTEXT_STATION),
        (Self::Boarded, "Boarded", CONTEXT_BOARDED),
    ];

    /// The store shown beside the picket's hold, if the context has one.
    fn partner(self) -> Option<Store> {
        match self {
            Self::Undocked => None,
            Self::Station => Some(Store::Market),
            Self::Boarded => Some(Store::Boarded),
        }
    }
}

/// The full-screen root; the top bar under it is never rebuilt.
#[derive(Component)]
struct SketchRoot;

/// The view body, rebuilt whenever the view or the width class changes. A
/// context change keeps it, and with it the live 3D scene. It scrolls when a
/// short window cannot fit its view.
#[derive(Component)]
struct SketchBody;

/// The node the map scene draws into. Exactly one may be mounted.
#[derive(Component)]
struct MapPane;

/// The node the ship scene draws into. Exactly one may be mounted.
#[derive(Component)]
struct ShipPane;

/// One live 3D scene and the pane it draws into.
struct PaneScene {
    host: Entity,
    root: Entity,
    image: Handle<Image>,
    /// Blip node per contact or section entity.
    blips: HashMap<Entity, Entity>,
}

/// The map scene, while a [`MapPane`] is mounted.
#[derive(Resource, Default)]
struct MapScene(Option<PaneScene>);

/// The ship scene, while a [`ShipPane`] is mounted.
#[derive(Resource, Default)]
struct ShipScene(Option<PaneScene>);

/// The contact the map has selected. It outlives the pane, so a return to the
/// map shows the same contact.
#[derive(Resource, Default)]
struct MapSelection(Option<Entity>);

/// The section the ship view has selected.
#[derive(Resource, Default)]
struct ShipSelection(Option<Entity>);

/// Theme-coloured 3D materials, repainted on a theme change.
#[derive(Resource)]
struct SceneMaterials {
    ring: Handle<StandardMaterial>,
    hub: Handle<StandardMaterial>,
    /// Block fill per [`SectionIcon`], in [`SectionIcon::ALL`] order.
    blocks: [Handle<StandardMaterial>; SectionIcon::ALL.len()],
    outline: Handle<StandardMaterial>,
    outline_selected: Handle<StandardMaterial>,
    bow: Handle<StandardMaterial>,
}

#[derive(Component)]
struct MapCamera;
#[derive(Component)]
struct ShipCamera;

/// A pane camera's orbit, in the NOVA OS viewer's terms.
#[derive(Component)]
struct SketchOrbit {
    theta: f32,
    phi: f32,
    radius: f32,
    center: Vec3,
    /// Where `center` eases to.
    center_target: Vec3,
    /// The selection `center_target` was last set from, so a reframe sticks
    /// until the selection changes.
    centered_on: Option<Entity>,
}

/// The map's floor rings, which stand on the orbit center.
#[derive(Component)]
struct MapRings;

/// The bow arrow's head. It points along ship-local -Z, ahead of the hull.
#[derive(Component)]
struct BowArrow;

/// How the screens draw a section kind: one icon, tint and legend word per
/// family, so a weapon reads as a weapon whatever it fires.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SectionIcon {
    Weapon,
    Thruster,
    Controller,
    Hull,
    Docking,
}

/// What a fixture good is for: one icon, tint and filter per category.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Category {
    Food,
    Ammo,
    Repair,
    Raw,
    Fuel,
    Parts,
}

/// What a map contact is, as the map draws it: a ship, a rock, a planet or a
/// nav point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BodyIcon {
    Ship,
    Asteroid,
    Planet,
    Objective,
}

/// A contact's icon and stance: what its blip wears and one legend entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MapMark {
    body: BodyIcon,
    kind: MapContactKind,
}

/// Every icon mask the screens draw, in each family's `ALL` order: white
/// shapes whose alpha is the coverage, tinted by the theme where they are
/// drawn.
#[derive(Resource)]
struct SketchIcons {
    sections: [Handle<Image>; SectionIcon::ALL.len()],
    cargo: [Handle<Image>; Category::ALL.len()],
    bodies: [Handle<Image>; BodyIcon::ALL.len()],
}

/// The status dot on a section badge.
#[derive(Component)]
struct StatusPip;

/// A section's box outline; its parent block names the section.
#[derive(Component)]
struct BlockOutline {
    section: Entity,
}

/// A clickable contact over the map image.
#[derive(Component)]
struct MapBlip {
    contact: Entity,
    mark: MapMark,
}

/// A clickable section over the ship image.
#[derive(Component)]
struct ShipBlip {
    section: Entity,
}

/// A blip's code label.
#[derive(Component)]
struct BlipLabel;

/// The map legend, refilled when the set of plotted marks changes.
#[derive(Component)]
struct MapLegend;

/// The selected section's large icon in the ship panel.
#[derive(Component)]
struct ShipPreview;

/// The selected section's condition bar in the ship panel.
#[derive(Component)]
struct ConditionFill;

fn main() -> AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(sketch_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(sketch_script());
        app.init_resource::<SimAudit>()
            .init_resource::<EaseLog>()
            .init_resource::<SteadyRects>()
            .init_resource::<HeardCues>()
            .add_observer(hear_cue)
            .add_systems(Update, count_easing_frames.after(drive_sketch_cameras))
            .add_systems(PreUpdate, watch_sim_clocks)
            .add_systems(PostUpdate, watch_player_input)
            .add_systems(FixedUpdate, count_fixed_steps)
            .add_systems(
                Last,
                arm_sim_audit
                    .after(hold_sketch_clocks)
                    .run_if(any_with_component::<SketchRoot>),
            );
        if capturing() {
            app.add_systems(Startup, hide_dev_overlays);
        }
    }

    app.run()
}

fn sketch_plugin(app: &mut App) {
    app.init_resource::<SketchView>()
        .init_resource::<SketchContext>()
        .init_resource::<MapScene>()
        .init_resource::<ShipScene>()
        .init_resource::<MapSelection>()
        .init_resource::<ShipSelection>()
        .init_resource::<SketchFixture>()
        .init_resource::<Inspected>()
        .init_resource::<CargoFilter>()
        .init_resource::<Draft>();
    // Every control drives its resource through the path the game's Settings
    // use. With its own game plugins this app has no menu, and the menu is
    // what registers the theme observer in the game.
    app.add_observer(button_on_setting::<SelectedUiTheme>)
        .add_observer(button_on_setting::<SketchView>)
        .add_observer(button_on_setting::<SketchContext>)
        .add_observer(click_cue);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scenario);
    app.add_systems(OnEnter(GameStates::Playing), spawn_root);
    app.add_systems(Startup, (init_scene_materials, init_icons));
    app.add_systems(
        Update,
        (
            rebuild_body.run_if(any_with_component::<PlayerSpaceshipMarker>),
            (
                type_draft,
                settle_inventory,
                refresh_repair_slot,
                refresh_store_columns,
                mark_cargo_rows,
                mark_filter_chips,
                update_inspector,
                sync_draft_controls,
                update_dock_line,
            )
                .chain()
                .after(TextFieldSystems),
            manage_map_scene,
            manage_ship_scene,
            fit_scene_images,
            follow_map_selection,
            follow_ship_selection,
            pan_map.run_if(in_state(PauseStates::Unpaused)),
            drive_sketch_cameras,
            project_map_blips,
            refresh_map_legend,
            project_ship_blips,
            outline_selected_section,
            update_map_readout,
            update_ship_detail,
            repaint_scenes,
        )
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        Update,
        (scroll_body, clamp_stored_scroll::<SketchBody>)
            .chain()
            .run_if(any_with_component::<SketchRoot>),
    );
    app.add_systems(
        PostUpdate,
        free_sketch_pointer.run_if(any_with_component::<SketchRoot>),
    );
    app.add_systems(
        Last,
        (hold_sketch_clocks, own_player_control).run_if(any_with_component::<SketchRoot>),
    );
}

/// Hold the simulation still while the screens are up, which in this example
/// is the whole run.
///
/// Not a `FreezeOwner` hold: those name the production surfaces, and these
/// screens are an example-only sketch with no pause menu over them. The cost
/// is that `ClockFreeze` names no owner while the sketch is idle, and
/// `Clocks::apply` resumes both clocks whenever the last named owner lets go:
/// the scenario load, or NOVA OS closing. So this runs in `Last`, after every
/// release a frame can make, and stops the clocks again before the next
/// frame's `First` advances them. A production screen takes a named hold
/// instead.
fn hold_sketch_clocks(
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    if !virtual_time.is_paused() {
        virtual_time.pause();
    }
    if !physics_time.is_paused() {
        physics_time.pause();
    }
}

/// Take the player's gameplay input while the screens are up, so a key flies
/// the map and never the ship. Suspends only when control is live: the
/// scenario load resumes it as it builds, and this takes it back in the same
/// frame's `Last`, before the next `PreUpdate` could raise the flight context.
/// The screens never close in this example, so nothing resumes it.
fn own_player_control(world: &mut World) {
    if !world.resource::<PlayerControlSuspended>().is_suspended() {
        suspend_player_control(world);
    }
}

/// Give the pointer to the screens while they are up. The player ship's
/// spawn locks and hides the cursor for flight, and in debug builds
/// nova_debug's inspector sync locks it again every frame, so this runs every
/// frame after `Update` and writes only on a difference. The mouse is cut off
/// from every input action, so a click on a pane never fires a weapon and a
/// drag never steers. Picking reads the pointer apart from the actions.
fn free_sketch_pointer(
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut sources: ResMut<ActionSources>,
) {
    if cursor.grab_mode != CursorGrabMode::None || !cursor.visible {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    if sources.mouse_buttons || sources.mouse_motion || sources.mouse_wheel {
        sources.mouse_buttons = false;
        sources.mouse_motion = false;
        sources.mouse_wheel = false;
    }
}

// Scenario.

/// A catalog ship at `position` meters.
fn ship(
    id: &str,
    name: &str,
    design: &str,
    position: Meters3,
    controller: SpaceshipController,
    allegiance: Allegiance,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Prototype {
                id: design.into(),
                section_patches: default(),
            },
            controller,
            allegiance: Some(allegiance),
            ..default()
        }),
    })
}

fn rock(
    id: &str,
    position: Meters3,
    radius: Meters,
    game_assets: &GameAssets,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Rock".to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_ROCK.into(),
            destroy_sound: None,
            mass: None,
            seed: None,
            lock_signature: None,
        }),
    })
}

fn load_scenario(mut commands: Commands, game_assets: Res<GameAssets>) {
    let actions = [
        vec![
            ship(
                OWN_SHIP.id,
                OWN_SHIP.name,
                OWN_SHIP.design,
                Meters3::ZERO,
                SpaceshipController::Player(PlayerControllerConfig::default()),
                Allegiance::Player,
            ),
            ship(
                BOARDED.id,
                BOARDED.name,
                BOARDED.design,
                Meters3::new(900.0, 60.0, -1400.0),
                SpaceshipController::None,
                Allegiance::Enemy,
            ),
            ship(
                "hauler",
                "Hauler",
                "block_hauler",
                Meters3::new(-1300.0, -40.0, -600.0),
                SpaceshipController::None,
                Allegiance::Player,
            ),
            // Both rocks stay under `GravitySettings::min_well_radius`, so
            // neither would pull the parked ships off the map framing if the
            // simulation ran.
            rock(
                "rock_near",
                Meters3::new(500.0, -80.0, 800.0),
                Meters(40.0),
                &game_assets,
            ),
            rock(
                "rock_far",
                Meters3::new(-700.0, 120.0, 1500.0),
                Meters(45.0),
                &game_assets,
            ),
        ],
        ThreePointRig::around(SCENARIO_ID, Meters3::ZERO, 8.0).actions(),
    ]
    .concat();
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A picket with contacts in range, under the UI screens".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID,
            "UI App Variants",
            game_assets.cubemap.clone().into(),
        )
    }));
}

// Chrome.

fn spawn_root(
    mut commands: Commands,
    view: Res<SketchView>,
    context: Res<SketchContext>,
    theme: Res<SelectedUiTheme>,
    mut hud: ResMut<HudVisibility>,
) {
    // The screens cover the flight view; the HUD would only draw over them.
    *hud = HudVisibility::Cinematic;
    let (view, context, theme) = (*view, *context, theme.0.clone());
    commands
        .spawn((
            SketchRoot,
            Name::new("Sketch Root"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(16)),
                row_gap: px(12),
                ..default()
            },
            // Above the flight HUD, below every NOVA OS layer.
            GlobalZIndex(MENU_PANEL_Z),
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColor::Void),
        ))
        .with_children(|root| {
            top_bar(root, view, context, &theme);
        });
}

/// Title, view tabs, mock context and theme. Built once; `button_on_setting`
/// moves each group's mark.
fn top_bar(root: &mut ChildSpawnerCommands, view: SketchView, context: SketchContext, theme: &str) {
    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: px(16),
        row_gap: px(10),
        width: percent(100),
        max_width: px(BODY_MAX_PX),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|bar| {
        text(bar, "NOVA", 18.0, UiColor::Primary);
        bar.spawn(segmented_container()).with_children(|seg| {
            for (value, label, name) in SketchView::ALL {
                let mut option = seg.spawn((
                    segmented_option(label),
                    ButtonValue(value),
                    Name::new(name),
                    SketchClick,
                ));
                if value == view {
                    option.insert(Selected);
                }
            }
        });
        bar.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
        bar.spawn(segmented_container()).with_children(|seg| {
            for (value, label, name) in SketchContext::ALL {
                let mut option = seg.spawn((
                    segmented_option(label),
                    ButtonValue(value),
                    Name::new(name),
                    SketchClick,
                ));
                if value == context {
                    option.insert(Selected);
                }
            }
        });
        bar.spawn(segmented_container()).with_children(|seg| {
            for (id, label, name) in [
                (PHOSPHOR_THEME_ID, "Phosphor", THEME_PHOSPHOR),
                (HARDWARE_THEME_ID, "Hardware", THEME_HARDWARE),
            ] {
                let mut option = seg.spawn((
                    segmented_option(label),
                    ButtonValue(SelectedUiTheme(id.to_string())),
                    Name::new(name),
                    SketchClick,
                ));
                if id == theme {
                    option.insert(Selected);
                }
            }
        });
    });
}

/// Widest the body grows, in logical px.
const BODY_MAX_PX: f32 = 1520.0;

/// Narrower than this, the context line takes two rows and the ship panel and
/// the inspector stack under their views.
const NARROW_BELOW_PX: f32 = 1100.0;
/// Height of the context line, in logical px, one row wide and two rows
/// narrow. Fixed per width class, so the pane under it keeps its size across
/// views and contexts.
const BANNER_PX: f32 = 52.0;
const BANNER_NARROW_PX: f32 = 72.0;

fn banner_px(narrow: bool) -> f32 {
    if narrow {
        BANNER_NARROW_PX
    } else {
        BANNER_PX
    }
}

/// Shortest a view's card may be, in logical px. A window too short for it
/// scrolls the body instead of squeezing the scene or clipping the panels.
fn card_min_px(view: SketchView, narrow: bool) -> f32 {
    match (view, narrow) {
        (SketchView::Map, _) => 360.0,
        (SketchView::Ship | SketchView::Inventory, false) => 440.0,
        (SketchView::Ship, true) => 680.0,
        (SketchView::Inventory, true) => 800.0,
    }
}

/// Respawn the body for the current view and width class. A theme change
/// never comes through here: every widget and material repaints itself. A
/// context, fixture, selection or confirmation change never comes through
/// here either: the context line and the inspector update their nodes in
/// place, and [`refresh_repair_slot`] and [`refresh_store_columns`] rebuild
/// only the slot or store column that changed, so the live 3D scene and its
/// camera survive all of them.
fn rebuild_body(
    mut commands: Commands,
    view: Res<SketchView>,
    icons: Res<SketchIcons>,
    windows: Query<&Window, With<PrimaryWindow>>,
    roots: Query<Entity, With<SketchRoot>>,
    bodies: Query<Entity, With<SketchBody>>,
    mut built_narrow: Local<Option<bool>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    let narrow = windows
        .single()
        .is_ok_and(|window| window.width() < NARROW_BELOW_PX);
    if !bodies.is_empty() && !view.is_changed() && *built_narrow == Some(narrow) {
        return;
    }
    *built_narrow = Some(narrow);
    for body in &bodies {
        commands.entity(body).despawn();
    }
    let view = *view;
    commands.entity(root).with_children(|root| {
        root.spawn((
            SketchBody,
            Name::new("Sketch Body"),
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                max_width: px(BODY_MAX_PX),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
        ))
        .with_children(|body| {
            dock_banner(body, narrow);
            let min_height = card_min_px(view, narrow);
            match view {
                SketchView::Map => map_view(body, narrow, min_height),
                SketchView::Ship => ship_view(body, &icons, narrow, min_height),
                SketchView::Inventory => inventory_view(body, &icons, narrow, min_height),
            }
        });
    });
}

/// Scroll the body with the wheel, except over a 3D scene, where the wheel
/// zooms instead, and over the confirmation's quantity row, where it steps
/// the quantity.
fn scroll_body(
    mut wheel: MessageReader<MouseWheel>,
    owners: Query<&Hovered, Or<(With<MapPane>, With<ShipPane>, With<DraftWheel>)>>,
    bodies: Query<(&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>), With<SketchBody>>,
) {
    if owners.iter().any(Hovered::get) {
        wheel.clear();
        return;
    }
    drive_wheel_scroll::<SketchBody>(wheel, bodies);
}

/// A titled panel that takes the room its parent leaves, but never less than
/// `min_height`, named for the harness.
fn card(
    body: &mut ChildSpawnerCommands,
    name: &str,
    title: &str,
    min_height: f32,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    body.spawn((
        Name::new(name.to_string()),
        Node {
            flex_grow: 1.0,
            flex_shrink: 0.0,
            flex_basis: px(0),
            min_height: px(min_height),
            min_width: px(0),
            ..panel_node()
        },
        panel(),
    ))
    .with_children(|card| {
        card.spawn(panel_head(title, None));
        card.spawn(Node {
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(12)),
            ..default()
        })
        .with_children(build);
    });
}

/// The context line: one fixed height in every view and context, and no
/// controls. Its texts are built once and filled by [`update_dock_line`].
/// Narrow, the result takes its own row.
fn dock_banner(body: &mut ChildSpawnerCommands, narrow: bool) {
    let row = || Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(12),
        ..default()
    };
    body.spawn((
        Name::new(DOCK_BANNER),
        Node {
            height: px(banner_px(narrow)),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip(),
            row_gap: px(6),
            padding: UiRect::axes(px(16), px(6)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.12),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Secondary),
    ))
    .with_children(|banner| {
        if narrow {
            banner.spawn(row()).with_children(|row| {
                dock_head(row);
                spacer(row);
                dock_credits(row);
            });
            banner.spawn(row()).with_children(dock_notice);
        } else {
            banner.spawn(row()).with_children(|row| {
                dock_head(row);
                spacer(row);
                dock_notice(row);
                dock_credits(row);
            });
        }
    });
}

fn dock_head(row: &mut ChildSpawnerCommands) {
    row.spawn((
        Name::new(DOCK_HEAD),
        themed_text("", 20.0, UiColor::Secondary),
    ));
    row.spawn((
        Name::new(DOCK_PARTNER),
        themed_text("", 16.0, UiColor::Primary),
    ));
}

fn dock_notice(row: &mut ChildSpawnerCommands) {
    row.spawn((
        Name::new(DOCK_NOTICE),
        themed_text("", 12.0, UiColor::Body),
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
    ));
}

fn dock_credits(row: &mut ChildSpawnerCommands) {
    row.spawn((
        Name::new(DOCK_CREDITS),
        themed_text("", 16.0, UiColor::Primary),
        TextLayout::new(Justify::Right, LineBreak::NoWrap),
    ));
}

/// Fill the context line in place: the context and what the ship is at, the
/// last transaction's result and the credits.
fn update_dock_line(
    fixture: Res<SketchFixture>,
    context: Res<SketchContext>,
    mut texts: Query<(&Name, &mut Text, &mut ThemedText)>,
) {
    let (head, tone, partner, partner_tone) = match *context {
        SketchContext::Undocked => (
            "UNDOCKED",
            UiColor::Secondary,
            "No station or boarded ship".to_string(),
            UiColor::Label,
        ),
        SketchContext::Station => (
            "STATION",
            UiColor::Accent,
            format!("{STATION_NAME}  (fixture)"),
            UiColor::Primary,
        ),
        SketchContext::Boarded => (
            "BOARDED",
            UiColor::Danger,
            format!("{}  {}  (mock)", BOARDED.name, BOARDED.design),
            UiColor::Primary,
        ),
    };
    set_themed_text(&mut texts, DOCK_HEAD, head, tone);
    set_themed_text(&mut texts, DOCK_PARTNER, &partner, partner_tone);
    set_themed_text(&mut texts, DOCK_NOTICE, &fixture.notice, UiColor::Body);
    set_themed_text(
        &mut texts,
        DOCK_CREDITS,
        &fixture.credits_text(),
        UiColor::Primary,
    );
}

/// Set the text called `name` and its theme colour, writing only what
/// differs so an unchanged node is not marked changed.
fn set_themed_text(
    texts: &mut Query<(&Name, &mut Text, &mut ThemedText)>,
    name: &str,
    value: &str,
    color: UiColor,
) {
    for (node, mut text, mut themed) in texts.iter_mut() {
        if node.as_str() != name {
            continue;
        }
        if text.0 != value {
            text.0 = value.to_string();
        }
        if themed.color != color {
            themed.color = color;
        }
    }
}

fn spacer(row: &mut ChildSpawnerCommands) {
    row.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
}

/// A themed button small enough for the repair slot, a filter or the deal
/// form.
fn compact_button<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    spec: ButtonSpec,
    name: &str,
) -> EntityCommands<'a> {
    let mut entity = parent.spawn((
        button(ButtonSpec {
            min_height: 22.0,
            font_size: 12.0,
            ..spec
        }),
        Name::new(name.to_string()),
    ));
    entity.entry::<Node>().and_modify(|mut node| {
        node.margin = UiRect::ZERO;
        node.padding = UiRect::axes(px(8), px(1));
    });
    entity
}

/// The node a 3D scene draws into: it takes all the room its column leaves.
/// `Hovered` tells [`scroll_body`] the wheel belongs to the scene.
fn scene_node() -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            position_type: PositionType::Relative,
            overflow: Overflow::clip(),
            ..default()
        },
        ImageNode::default(),
        Hovered::default(),
    )
}

/// A row of controls.
fn control_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(12),
        row_gap: px(8),
        flex_shrink: 0.0,
        ..default()
    }
}

/// The map: the scene, then a fixed-height line with the readout, the legend
/// and Reframe. Narrow, the legend takes its own row. Nothing on the map
/// trades, repairs or docks.
fn map_view(body: &mut ChildSpawnerCommands, narrow: bool, min_height: f32) {
    card(body, PANE_MAP, "Map", min_height, |c| {
        c.spawn((MapPane, Name::new(MAP_SCENE), scene_node()))
            .observe(orbit_drag::<MapCamera>)
            .observe(zoom_map);
        c.spawn(Node {
            height: px(if narrow { 84.0 } else { 40.0 }),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: px(8),
            ..default()
        })
        .with_children(|foot| {
            foot.spawn(control_row(JustifyContent::FlexStart))
                .with_children(|line| {
                    line.spawn((
                        Name::new(MAP_READOUT),
                        themed_text("", 14.0, UiColor::Body),
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                        Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                    ));
                    if !narrow {
                        map_legend(line);
                    }
                    line.spawn((
                        button(ButtonSpec::new("Reframe").fit()),
                        Name::new(MAP_REFRAME),
                        SketchClick,
                    ))
                    .observe(reframe_map);
                });
            if narrow {
                foot.spawn(control_row(JustifyContent::FlexStart))
                    .with_children(map_legend);
            }
        });
    });
}

/// The slot [`refresh_map_legend`] fills with the plotted marks.
fn map_legend(line: &mut ChildSpawnerCommands) {
    line.spawn((
        MapLegend,
        Name::new(MAP_LEGEND),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            flex_shrink: 0.0,
            ..default()
        },
    ));
}

/// Width of the ship panel and the inventory inspector beside their views, in
/// logical px.
const SIDE_PX: f32 = 300.0;
/// Height of the ship panel stacked under the ship view, in logical px: the
/// preview head and the repair slot.
const SHIP_PANEL_NARROW_PX: f32 = 200.0;
/// Height of the repair slot, in logical px. Fixed, so the scene beside or
/// above it keeps its size in every context and repair state.
const REPAIR_PX: f32 = 92.0;

/// The ship: the scene with the section legend and the centred step, fit and
/// reset controls under it, and the section panel beside it, or under both
/// when narrow. Wide, the legend sits left of the controls and an equal empty
/// side keeps them centred; narrow, the legend takes its own row.
fn ship_view(body: &mut ChildSpawnerCommands, icons: &SketchIcons, narrow: bool, min_height: f32) {
    card(body, PANE_SHIP, "Ship", min_height, |c| {
        c.spawn(split(narrow)).with_children(|split| {
            split
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                })
                .with_children(|viewer| {
                    viewer
                        .spawn((ShipPane, Name::new(SHIP_SCENE), scene_node()))
                        .observe(orbit_drag::<ShipCamera>)
                        .observe(zoom_ship);
                    if narrow {
                        ship_controls(viewer);
                        section_legend(viewer, icons);
                    } else {
                        viewer
                            .spawn(control_row(JustifyContent::Center))
                            .with_children(|foot| {
                                foot.spawn(footer_side())
                                    .with_children(|side| section_legend(side, icons));
                                ship_controls(foot);
                                foot.spawn(footer_side());
                            });
                    }
                });
            ship_panel(split, icons, narrow);
        });
    });
}

/// Prev, Next, Fit and Reset, centred in their row.
fn ship_controls(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(control_row(JustifyContent::Center))
        .with_children(|controls| {
            for (label, name, step) in [("Prev", SHIP_PREV, -1), ("Next", SHIP_NEXT, 1)] {
                controls
                    .spawn((
                        button(ButtonSpec::new(label).fit()),
                        Name::new(name),
                        SketchClick,
                    ))
                    .observe(step_section(step));
            }
            controls
                .spawn((
                    button(ButtonSpec::new("Fit").fit()),
                    Name::new(SHIP_FIT),
                    SketchClick,
                ))
                .observe(fit_ship);
            controls
                .spawn((
                    button(ButtonSpec::new("Reset").fit()),
                    Name::new(SHIP_RESET),
                    SketchClick,
                ))
                .observe(reset_ship);
        });
}

/// One of the two equal sides of the wide ship footer.
fn footer_side() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// The section kinds: each icon in its tint and its word, wrapping when the
/// row is short.
fn section_legend(parent: &mut ChildSpawnerCommands, icons: &SketchIcons) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: px(12),
            row_gap: px(6),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|legend| {
            for icon in SectionIcon::ALL {
                legend
                    .spawn((
                        Name::new(format!("Sketch Legend {}", icon.label())),
                        legend_entry(),
                    ))
                    .with_children(|entry| {
                        entry.spawn(icon_node(
                            icons.sections[icon.index()].clone(),
                            icon.color(),
                            18.0,
                        ));
                        text(entry, icon.label(), 12.0, UiColor::Body);
                    });
            }
        });
}

/// A card's content split: a row when wide, a column when narrow.
fn split(narrow: bool) -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: if narrow {
            FlexDirection::Column
        } else {
            FlexDirection::Row
        },
        column_gap: px(12),
        row_gap: px(12),
        ..default()
    }
}

/// A themed side panel: [`SIDE_PX`] wide beside its view, or full width and
/// `narrow_px` high under it.
fn side_panel(narrow: bool, narrow_px: f32) -> impl Bundle {
    (
        Node {
            width: if narrow { percent(100) } else { px(SIDE_PX) },
            height: if narrow { px(narrow_px) } else { auto() },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.08),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Secondary),
    )
}

/// A frame around a large icon, `size` px square.
fn icon_frame(size: f32) -> impl Bundle {
    (
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Surface, 0.6),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Secondary),
    )
}

/// The selected section: its icon, code, kind and condition, and the repair
/// slot. [`update_ship_detail`] fills the texts, the icon and the bar;
/// [`refresh_repair_slot`] fills the repair slot.
fn ship_panel(split: &mut ChildSpawnerCommands, icons: &SketchIcons, narrow: bool) {
    split
        .spawn((
            Name::new(SHIP_PANEL),
            side_panel(narrow, SHIP_PANEL_NARROW_PX),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|head| {
                    head.spawn(icon_frame(64.0)).with_children(|frame| {
                        frame.spawn((
                            ShipPreview,
                            Name::new(SHIP_PREVIEW),
                            icon_node(
                                icons.sections[SectionIcon::Hull.index()].clone(),
                                SectionIcon::Hull.color(),
                                48.0,
                            ),
                        ));
                    });
                    head.spawn(Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    })
                    .with_children(|detail| {
                        detail.spawn((
                            Name::new(SHIP_DETAIL),
                            themed_text("", 16.0, UiColor::Primary),
                        ));
                        detail
                            .spawn((Name::new(SHIP_STATUS), themed_text("", 12.0, UiColor::Body)));
                        detail.spawn(bar_track()).with_children(|track| {
                            track.spawn((
                                ConditionFill,
                                Name::new(SHIP_CONDITION),
                                Node {
                                    width: percent(100),
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                ThemedFill::new(UiColor::Nominal),
                            ));
                        });
                    });
                });
            panel.spawn((
                RepairSlot,
                Name::new(SHIP_SERVICE),
                Node {
                    height: px(REPAIR_PX),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: px(6),
                    ..default()
                },
            ));
        });
}

fn legend_entry() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(4),
        ..default()
    }
}

/// The track of a 6 px bar; its child fills it to a share.
fn bar_track() -> impl Bundle {
    (
        Node {
            height: px(6),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(px(3)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.25),
    )
}

fn themed_text(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(value.to_string()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
    )
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: UiColor) {
    parent.spawn(themed_text(value, size, color));
}

// 3D scenes.

fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

fn init_scene_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    // Painted by `repaint_scenes` once the theme resolves.
    let mut add = || materials.add(unlit(Color::NONE));
    commands.insert_resource(SceneMaterials {
        ring: add(),
        hub: add(),
        blocks: std::array::from_fn(|_| add()),
        outline: add(),
        outline_selected: add(),
        bow: add(),
    });
}

// Icons. Each mask's coverage takes `p`, a point in `[-1, 1]` squared with y
// down, and `px`, the width of one texel in those units.

/// Coverage of a shape whose signed distance at a texel is `d`.
fn solid(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

/// Signed distance to a disc.
fn circle(p: Vec2, c: Vec2, r: f32) -> f32 {
    (p - c).length() - r
}

/// Signed distance to a ring of radius `r` and width `w` around `c`.
fn ring(p: Vec2, c: Vec2, r: f32, w: f32) -> f32 {
    ((p - c).length() - r).abs() - w * 0.5
}

/// Signed distance to a stroke from `a` to `b`, `w` wide.
fn segment(p: Vec2, a: Vec2, b: Vec2, w: f32) -> f32 {
    let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
    (p - (a + (b - a) * t)).length() - w * 0.5
}

/// Signed distance to a convex polygon, wound clockwise on screen.
fn polygon(p: Vec2, points: &[Vec2]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| {
            let edge = *b - *a;
            let normal = Vec2::new(edge.y, -edge.x).normalize();
            (p - *a).dot(normal)
        })
        .fold(f32::NEG_INFINITY, f32::max)
}

impl SectionIcon {
    const ALL: [Self; 5] = [
        Self::Weapon,
        Self::Thruster,
        Self::Controller,
        Self::Hull,
        Self::Docking,
    ];

    fn of(kind: SectionClass) -> Self {
        match kind {
            SectionClass::Turret | SectionClass::Torpedo | SectionClass::Railgun => Self::Weapon,
            SectionClass::Thruster => Self::Thruster,
            SectionClass::Controller => Self::Controller,
            SectionClass::Hull => Self::Hull,
            SectionClass::Docking => Self::Docking,
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    fn label(self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Thruster => "Thruster",
            Self::Controller => "Controller",
            Self::Hull => "Hull",
            Self::Docking => "Docking",
        }
    }

    /// The tint of the family's icon and 3D block. Hull keeps the ink colour,
    /// so the bulk of a ship reads as the hull and the rest stands out.
    fn color(self) -> UiColor {
        match self {
            Self::Weapon => UiColor::Danger,
            Self::Thruster => UiColor::Info,
            Self::Controller => UiColor::AccentHigh,
            Self::Hull => UiColor::Primary,
            Self::Docking => UiColor::Nominal,
        }
    }

    /// Alpha of the 3D block fill. Hull stays faint so the tinted parts inside
    /// the outline read first.
    fn block_alpha(self) -> f32 {
        match self {
            Self::Hull => 0.22,
            _ => 0.5,
        }
    }

    /// Each family has its own silhouette: a reticle for a weapon, a nozzle
    /// and plume for a thruster, a diamond core for the controller, a riveted
    /// plate for hull, and a clamped collar for docking.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Weapon => {
                let ticks = [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                    .into_iter()
                    .map(|axis| segment(p, axis * 0.3, axis * 0.95, 0.14))
                    .fold(f32::INFINITY, f32::min);
                let reticle =
                    ring(p, Vec2::ZERO, 0.62, 0.14)
                        .min(ticks)
                        .min(circle(p, Vec2::ZERO, 0.11));
                solid(reticle, px)
            }
            Self::Thruster => {
                let bell = polygon(
                    p,
                    &[
                        Vec2::new(-0.22, -0.85),
                        Vec2::new(0.22, -0.85),
                        Vec2::new(0.62, 0.15),
                        Vec2::new(-0.62, 0.15),
                    ],
                );
                let plume = polygon(
                    p,
                    &[
                        Vec2::new(-0.42, 0.3),
                        Vec2::new(0.42, 0.3),
                        Vec2::new(0.0, 0.95),
                    ],
                );
                solid(bell, px).max(solid(plume, px) * 0.6)
            }
            Self::Controller => {
                let diamond = (p.x.abs() + p.y.abs() - 0.85) * std::f32::consts::FRAC_1_SQRT_2;
                solid(diamond.abs() - 0.07, px).max(solid(circle(p, Vec2::ZERO, 0.27), px))
            }
            Self::Hull => {
                let hexagon: Vec<Vec2> = (0..6)
                    .map(|corner| {
                        let angle = std::f32::consts::FRAC_PI_3 * corner as f32;
                        Vec2::new(angle.cos(), angle.sin()) * 0.88
                    })
                    .collect();
                let plate = polygon(p, &hexagon);
                let rivets = circle(p, Vec2::new(-0.38, 0.0), 0.11).min(circle(
                    p,
                    Vec2::new(0.38, 0.0),
                    0.11,
                ));
                (solid(plate, px) * 0.35)
                    .max(solid(plate.abs() - 0.07, px))
                    .max(solid(rivets, px))
            }
            Self::Docking => {
                let clamps = [45.0f32, 135.0, 225.0, 315.0]
                    .into_iter()
                    .map(|degrees| {
                        let axis = Vec2::from_angle(degrees.to_radians());
                        segment(p, axis * 0.66, axis * 0.95, 0.24)
                    })
                    .fold(f32::INFINITY, f32::min);
                solid(ring(p, Vec2::ZERO, 0.5, 0.16).min(clamps), px)
                    .max(solid(circle(p, Vec2::ZERO, 0.16), px) * 0.6)
            }
        }
    }
}

impl Category {
    const ALL: [Self; 6] = [
        Self::Food,
        Self::Ammo,
        Self::Repair,
        Self::Raw,
        Self::Fuel,
        Self::Parts,
    ];

    fn index(self) -> usize {
        self as usize
    }

    fn label(self) -> &'static str {
        match self {
            Self::Food => "Food",
            Self::Ammo => "Ammo",
            Self::Repair => "Repair",
            Self::Raw => "Raw",
            Self::Fuel => "Fuel",
            Self::Parts => "Parts",
        }
    }

    fn color(self) -> UiColor {
        match self {
            Self::Food => UiColor::Accent,
            Self::Ammo => UiColor::Danger,
            Self::Repair => UiColor::Nominal,
            Self::Raw => UiColor::Secondary,
            Self::Fuel => UiColor::Info,
            Self::Parts => UiColor::AccentHigh,
        }
    }

    /// The `Name` of the filter button that shows only this category.
    fn filter_name(self) -> String {
        format!("Sketch Filter {}", self.label())
    }

    /// A tin for food, three rounds for ammo, a wrench for repair stock, a
    /// pile of lumps for raw goods, a drop for fuel and a gear for parts.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Food => {
                let tin = polygon(
                    p,
                    &[
                        Vec2::new(-0.6, -0.55),
                        Vec2::new(0.6, -0.55),
                        Vec2::new(0.6, 0.75),
                        Vec2::new(-0.6, 0.75),
                    ],
                );
                let lid = segment(p, Vec2::new(-0.72, -0.72), Vec2::new(0.72, -0.72), 0.14);
                let band = segment(p, Vec2::new(-0.6, 0.1), Vec2::new(0.6, 0.1), 0.1);
                (solid(tin, px) * 0.55)
                    .max(solid(tin.abs() - 0.06, px))
                    .max(solid(lid, px))
                    .max(solid(band, px))
            }
            Self::Ammo => [-0.5f32, 0.0, 0.5]
                .into_iter()
                .map(|x| {
                    let case = polygon(
                        p,
                        &[
                            Vec2::new(x - 0.16, -0.2),
                            Vec2::new(x + 0.16, -0.2),
                            Vec2::new(x + 0.16, 0.85),
                            Vec2::new(x - 0.16, 0.85),
                        ],
                    );
                    let tip = circle(p, Vec2::new(x, -0.3), 0.16).max(p.y - (-0.2));
                    let nose = polygon(
                        p,
                        &[
                            Vec2::new(x, -0.85),
                            Vec2::new(x + 0.16, -0.3),
                            Vec2::new(x - 0.16, -0.3),
                        ],
                    );
                    solid(case.min(tip).min(nose), px)
                })
                .fold(0.0, f32::max),
            Self::Repair => {
                let head = ring(p, Vec2::new(-0.35, -0.35), 0.3, 0.2);
                // The jaw: the head's ring opens toward the top left.
                let jaw = circle(p, Vec2::new(-0.62, -0.62), 0.22);
                let handle = segment(p, Vec2::new(-0.15, -0.15), Vec2::new(0.7, 0.7), 0.24);
                solid(head.max(-jaw).min(handle), px)
            }
            Self::Raw => {
                let lumps = circle(p, Vec2::new(-0.4, 0.4), 0.36)
                    .min(circle(p, Vec2::new(0.4, 0.42), 0.34))
                    .min(circle(p, Vec2::new(0.0, -0.2), 0.4));
                solid(lumps, px)
            }
            Self::Fuel => {
                let bulb = circle(p, Vec2::new(0.0, 0.3), 0.52);
                let tip = polygon(
                    p,
                    &[
                        Vec2::new(0.0, -0.9),
                        Vec2::new(0.45, 0.04),
                        Vec2::new(-0.45, 0.04),
                    ],
                );
                solid(bulb.min(tip), px)
            }
            Self::Parts => {
                let teeth = (0..8)
                    .map(|tooth| {
                        let axis = Vec2::from_angle(std::f32::consts::FRAC_PI_4 * tooth as f32);
                        segment(p, axis * 0.5, axis * 0.88, 0.22)
                    })
                    .fold(f32::INFINITY, f32::min);
                let wheel = ring(p, Vec2::ZERO, 0.42, 0.26);
                solid(wheel.min(teeth), px)
            }
        }
    }
}

impl BodyIcon {
    const ALL: [Self; 4] = [Self::Ship, Self::Asteroid, Self::Planet, Self::Objective];

    fn index(self) -> usize {
        self as usize
    }

    /// A chevron for a ship, a rough lump for an asteroid, a ringed disc for
    /// a planet and a diamond for a nav point.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Ship => {
                let starboard = polygon(
                    p,
                    &[
                        Vec2::new(0.0, -0.9),
                        Vec2::new(0.7, 0.75),
                        Vec2::new(0.0, 0.4),
                    ],
                );
                let port = polygon(
                    p,
                    &[
                        Vec2::new(0.0, -0.9),
                        Vec2::new(0.0, 0.4),
                        Vec2::new(-0.7, 0.75),
                    ],
                );
                solid(starboard.min(port), px)
            }
            Self::Asteroid => {
                let lump = polygon(
                    p,
                    &[
                        Vec2::new(-0.2, -0.8),
                        Vec2::new(0.45, -0.7),
                        Vec2::new(0.82, -0.12),
                        Vec2::new(0.62, 0.6),
                        Vec2::new(-0.05, 0.82),
                        Vec2::new(-0.68, 0.45),
                        Vec2::new(-0.78, -0.3),
                    ],
                );
                let crater = circle(p, Vec2::new(0.2, 0.1), 0.18);
                solid(lump, px).min(1.0 - solid(crater, px) * 0.6)
            }
            Self::Planet => {
                let disc = circle(p, Vec2::ZERO, 0.5);
                let band = ring(p, Vec2::ZERO, 0.8, 0.1).max(p.y.abs() - 0.18);
                solid(disc, px).max(solid(band, px))
            }
            Self::Objective => {
                let diamond = (p.x.abs() + p.y.abs() - 0.8) * std::f32::consts::FRAC_1_SQRT_2;
                solid(diamond.abs() - 0.09, px).max(solid(circle(p, Vec2::ZERO, 0.16), px))
            }
        }
    }
}

/// Icon mask edge, in texels.
const ICON_TEXELS: u32 = 64;

/// Draw one mask from its coverage.
fn icon_mask(images: &mut Assets<Image>, coverage: impl Fn(Vec2, f32) -> f32) -> Handle<Image> {
    let texel = 2.0 / ICON_TEXELS as f32;
    let mut data = Vec::with_capacity((ICON_TEXELS * ICON_TEXELS * 4) as usize);
    for y in 0..ICON_TEXELS {
        for x in 0..ICON_TEXELS {
            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * texel - Vec2::ONE;
            let alpha = coverage(at, texel);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: ICON_TEXELS,
            height: ICON_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Draw every icon mask once.
fn init_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(SketchIcons {
        sections: SectionIcon::ALL.map(|icon| icon_mask(&mut images, |p, px| icon.coverage(p, px))),
        cargo: Category::ALL
            .map(|category| icon_mask(&mut images, |p, px| category.coverage(p, px))),
        bodies: BodyIcon::ALL.map(|body| icon_mask(&mut images, |p, px| body.coverage(p, px))),
    });
}

/// An icon mask tinted by the theme, `size` px square.
fn icon_node(image: Handle<Image>, tint: UiColor, size: f32) -> impl Bundle {
    (
        ImageNode::new(image),
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            ..default()
        },
        ThemedImageTint::new(tint),
        Pickable::IGNORE,
    )
}

/// The one mounted pane with marker `T`. Two is a layout bug and panics.
fn single_host<T: Component>(hosts: &Query<Entity, With<T>>, what: &str) -> Option<Entity> {
    let mut hosts = hosts.iter();
    let host = hosts.next()?;
    assert!(
        hosts.next().is_none(),
        "two {what} panes are mounted; a scene draws into exactly one"
    );
    Some(host)
}

/// A scene camera drawing into a fresh image.
fn scene_camera(
    images: &mut Assets<Image>,
    order: isize,
    layer: usize,
    theme: &ActiveUiTheme,
) -> (Handle<Image>, impl Bundle) {
    let image = images.add(new_render_target_image(UVec2::splat(64)));
    let camera = (
        Camera3d::default(),
        Camera {
            order,
            clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle: image.clone(),
            scale_factor: 1.0,
        }),
        RenderLayers::layer(layer),
    );
    (image, camera)
}

/// Tear down `scene` unless it belongs to `host`, with its blips.
fn drop_stale_scene(commands: &mut Commands, scene: &mut Option<PaneScene>, host: Option<Entity>) {
    if scene.as_ref().is_some_and(|scene| Some(scene.host) != host) {
        let scene = scene.take().expect("checked above");
        commands.entity(scene.root).despawn();
        for blip in scene.blips.into_values() {
            commands.entity(blip).try_despawn();
        }
    }
}

/// Build the map scene when a [`MapPane`] mounts and drop it when it goes:
/// the NOVA OS map's rings and hub, framed by its helpers, on this example's
/// camera and palette.
#[expect(
    clippy::too_many_arguments,
    reason = "one scene, its assets and its host"
)]
fn manage_map_scene(
    mut commands: Commands,
    mut scene: ResMut<MapScene>,
    selection: Res<MapSelection>,
    hosts: Query<Entity, With<MapPane>>,
    joined: Query<(), Added<MapContactCode>>,
    player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
    contacts: MapContacts,
    theme: Res<ActiveUiTheme>,
    palette: Res<SceneMaterials>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let host = single_host(&hosts, "map");
    // The scenario's contacts join over several frames, and the framing
    // must see all of them: a contact that joins drops the scene to rebuild.
    drop_stale_scene(
        &mut commands,
        &mut scene.0,
        host.filter(|_| joined.is_empty()),
    );
    let Some(host) = host else {
        return;
    };
    if scene.0.is_some() {
        return;
    }
    let focus = player
        .single()
        .expect("the map pane mounted without exactly one player ship")
        .translation();
    let framing = map_radius_default(map_spread(&contacts, focus));

    let (image, camera) = scene_camera(&mut images, MAP_CAMERA_ORDER, MAP_LAYER, &theme);
    let root = commands
        .spawn((
            Name::new("Sketch Map Scene Root"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    commands.spawn((
        MapCamera,
        camera,
        SketchOrbit {
            theta: MAP_THETA,
            phi: MAP_PHI,
            radius: framing,
            center: focus,
            center_target: focus,
            // A rebuilt map opens framed on the player, as after Reframe;
            // only a new pick recenters it.
            centered_on: selection.0,
        },
        Transform::from_translation(focus + orbit_eye(framing, MAP_THETA, MAP_PHI))
            .looking_at(focus, Vec3::Y),
        ChildOf(root),
    ));
    // The rings stand on the orbit center and follow it to a selected
    // contact, as on the NOVA OS map.
    let rings = commands
        .spawn((
            MapRings,
            Transform::from_translation(focus),
            Visibility::Visible,
            ChildOf(root),
        ))
        .id();
    // Rings a fixed share of the framing thick, so they read at any spread.
    let half_width = framing * 0.0025;
    for radius in map_ring_radii(framing) {
        commands.spawn((
            Mesh3d(meshes.add(Torus::new(radius - half_width, radius + half_width))),
            MeshMaterial3d(palette.ring.clone()),
            Transform::default(),
            RenderLayers::layer(MAP_LAYER),
            ChildOf(rings),
        ));
    }
    // The hub is the player ship and stays on it.
    let hub = contacts
        .collect()
        .into_iter()
        .find(|contact| contact.kind == MapContactKind::OwnShip)
        .and_then(|contact| contact.radius)
        .unwrap_or(0.0)
        .max(MAP_HUB_MIN.to_engine());
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(hub))),
        MeshMaterial3d(palette.hub.clone()),
        Transform::from_translation(focus),
        RenderLayers::layer(MAP_LAYER),
        ChildOf(root),
    ));
    commands.entity(host).insert(ImageNode::new(image.clone()));
    scene.0 = Some(PaneScene {
        host,
        root,
        image,
        blips: HashMap::default(),
    });
}

/// Build the ship scene when a [`ShipPane`] mounts and drop it when it goes:
/// one kind-tinted block and outline per live section, as the NOVA OS ship
/// draws them, and a bow arrow along ship-local -Z.
#[expect(
    clippy::too_many_arguments,
    reason = "one scene, its assets and its host"
)]
fn manage_ship_scene(
    mut commands: Commands,
    mut scene: ResMut<ShipScene>,
    mut selection: ResMut<ShipSelection>,
    hosts: Query<Entity, With<ShipPane>>,
    player: Query<(), With<PlayerSpaceshipMarker>>,
    sections: ShipSections,
    theme: Res<ActiveUiTheme>,
    palette: Res<SceneMaterials>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let host = single_host(&hosts, "ship");
    drop_stale_scene(&mut commands, &mut scene.0, host);
    let Some(host) = host else {
        return;
    };
    if scene.0.is_some() {
        return;
    }
    assert_eq!(
        player.iter().count(),
        1,
        "the ship pane mounted without exactly one player ship"
    );
    // Sections join the model once NOVA OS has coded them, a frame after
    // the ship spawns. Nothing is drawn until then.
    let views = sections.collect();
    if views.is_empty() {
        return;
    }
    let (centre, radius) = ship_framing(&views);
    let radius = radius * SHIP_PANE_ZOOM;
    if selection
        .0
        .is_none_or(|selected| views.iter().all(|view| view.entity != selected))
    {
        selection.0 = views.first().map(|view| view.entity);
    }

    let (image, camera) = scene_camera(&mut images, SHIP_CAMERA_ORDER, SHIP_LAYER, &theme);
    let root = commands
        .spawn((
            Name::new("Sketch Ship Scene Root"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    commands.spawn((
        ShipCamera,
        camera,
        SketchOrbit {
            theta: SHIP_THETA,
            phi: SHIP_PHI,
            radius,
            center: centre,
            center_target: centre,
            centered_on: selection.0,
        },
        Transform::from_translation(centre + orbit_eye(radius, SHIP_THETA, SHIP_PHI))
            .looking_at(centre, Vec3::Y),
        ChildOf(root),
    ));
    let edges = meshes.add(cuboid_edges());
    for view in &views {
        let full = (view.half_extents * 2.0).max(Vec3::splat(0.2));
        let fill = full * SHIP_BLOCK_FILL_SCALE;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(fill.x, fill.y, fill.z))),
            MeshMaterial3d(palette.blocks[SectionIcon::of(view.kind).index()].clone()),
            Transform {
                translation: view.local.translation,
                rotation: view.local.rotation,
                scale: Vec3::ONE,
            },
            RenderLayers::layer(SHIP_LAYER),
            ChildOf(root),
            children![(
                BlockOutline {
                    section: view.entity,
                },
                Mesh3d(edges.clone()),
                MeshMaterial3d(palette.outline.clone()),
                Transform::from_scale(full),
                RenderLayers::layer(SHIP_LAYER),
            )],
        ));
    }
    // Bow is ship-local -Z. The arrow starts clear of the foremost block and
    // points away from the hull.
    let bow = views
        .iter()
        .map(|view| view.local.translation.z - view.half_extents.max_element())
        .fold(f32::INFINITY, f32::min);
    let length = radius * 0.2;
    let gap = length * 0.15;
    let shaft = length * 0.03;
    let point_back = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(shaft, length * 0.7))),
        MeshMaterial3d(palette.bow.clone()),
        Transform::from_translation(Vec3::new(centre.x, centre.y, bow - gap - length * 0.35))
            .with_rotation(point_back),
        RenderLayers::layer(SHIP_LAYER),
        ChildOf(root),
    ));
    commands.spawn((
        BowArrow,
        Mesh3d(meshes.add(Cone {
            radius: shaft * 3.5,
            height: length * 0.3,
        })),
        MeshMaterial3d(palette.bow.clone()),
        Transform::from_translation(Vec3::new(centre.x, centre.y, bow - gap - length * 0.85))
            .with_rotation(point_back),
        RenderLayers::layer(SHIP_LAYER),
        ChildOf(root),
    ));
    commands.entity(host).insert(ImageNode::new(image.clone()));
    scene.0 = Some(PaneScene {
        host,
        root,
        image,
        blips: HashMap::default(),
    });
}

/// The narrowest pane, width over height, that shows a whole framing at the
/// default field of view.
const FRAMED_ASPECT: f32 = 1.5;

/// Keep each scene image the pane's physical size.
fn fit_scene_images(
    map: Res<MapScene>,
    ship: Res<ShipScene>,
    mut images: ResMut<Assets<Image>>,
    nodes: Query<&ComputedNode>,
    mut map_camera: Query<&mut Projection, (With<MapCamera>, Without<ShipCamera>)>,
    mut ship_camera: Query<&mut Projection, (With<ShipCamera>, Without<MapCamera>)>,
) {
    for (scene, projection) in [
        (map.0.as_ref(), map_camera.single_mut().ok()),
        (ship.0.as_ref(), ship_camera.single_mut().ok()),
    ] {
        let Some(scene) = scene else {
            continue;
        };
        let Ok(node) = nodes.get(scene.host) else {
            continue;
        };
        let desired = node.size().round().as_uvec2().max(UVec2::ONE);
        let mut projection = projection;
        if let Some(Projection::Perspective(perspective)) = projection.as_deref_mut() {
            // Both framings are sized for a landscape pane. A narrower pane
            // widens the vertical field so the sides still fit.
            let aspect = desired.x as f32 / desired.y as f32;
            let base = PerspectiveProjection::default().fov;
            let fov = if aspect < FRAMED_ASPECT {
                2.0 * ((base * 0.5).tan() * FRAMED_ASPECT / aspect).atan()
            } else {
                base
            };
            if perspective.fov != fov {
                perspective.fov = fov;
            }
        }
        resize_render_target(&mut images, &scene.image, desired, projection);
    }
}

/// Aim the map orbit at a newly selected contact, and keep the rings on the
/// orbit center.
fn follow_map_selection(
    selection: Res<MapSelection>,
    contacts: MapContacts,
    mut orbit: Query<&mut SketchOrbit, With<MapCamera>>,
    mut rings: Query<&mut Transform, With<MapRings>>,
) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    if selection.0 != orbit.centered_on {
        if let Some(contact) = selection.0.and_then(|selected| {
            contacts
                .collect()
                .into_iter()
                .find(|c| c.entity == selected)
        }) {
            orbit.center_target = contact.world_pos;
        }
        orbit.centered_on = selection.0;
    }
    if let Ok(mut rings) = rings.single_mut() {
        rings.translation = orbit.center;
    }
}

/// Aim the ship orbit at a newly selected section. The scene is ship-local,
/// so the section's local offset is the target.
fn follow_ship_selection(
    selection: Res<ShipSelection>,
    sections: ShipSections,
    mut orbit: Query<&mut SketchOrbit, With<ShipCamera>>,
) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    if selection.0 == orbit.centered_on {
        return;
    }
    if let Some(view) = selection.0.and_then(|selected| {
        sections
            .collect()
            .into_iter()
            .find(|v| v.entity == selected)
    }) {
        orbit.center_target = view.local.translation;
    }
    orbit.centered_on = selection.0;
}

/// Ease each pane camera's center and place its eye. On real time: the
/// simulation clock is paused under these screens, and the views must still
/// move.
fn drive_sketch_cameras(
    time: Res<Time<Real>>,
    mut cameras: Query<(&mut Transform, &mut SketchOrbit)>,
) {
    for (mut transform, mut orbit) in &mut cameras {
        let center = ease_orbit_center(orbit.center, orbit.center_target, time.delta_secs());
        if center != orbit.center {
            orbit.center = center;
        }
        let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
        let wanted = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
        if *transform != wanted {
            *transform = wanted;
        }
    }
}

/// Share of the map orbit radius the free camera crosses per real second.
const MAP_PAN_RATE: f32 = 0.5;

/// Fly the map's orbit center freely: W/A/S/D along and across the camera's
/// heading, Space up and Shift down, on real time. Not while NOVA OS is open:
/// its own viewer reads the same keys then.
fn pan_map(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut orbit: Query<&mut SketchOrbit, With<MapCamera>>,
) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    let held = |any: &[KeyCode]| {
        if keys.any_pressed(any.iter().copied()) {
            1.0
        } else {
            0.0
        }
    };
    let along = held(&[KeyCode::KeyW]) - held(&[KeyCode::KeyS]);
    let across = held(&[KeyCode::KeyD]) - held(&[KeyCode::KeyA]);
    let rise = held(&[KeyCode::Space]) - held(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if along == 0.0 && across == 0.0 && rise == 0.0 {
        return;
    }
    let heading = -Vec3::new(orbit.theta.sin(), 0.0, orbit.theta.cos());
    let side = heading.cross(Vec3::Y);
    let travel = (heading * along + side * across + Vec3::Y * rise)
        * orbit.radius
        * MAP_PAN_RATE
        * time.delta_secs();
    orbit.center += travel;
    orbit.center_target += travel;
}

/// Orbit a pane's camera by a drag on its background, at the NOVA OS rate per
/// pixel. Any button drags: NOVA OS keeps the left button for selection, but
/// here selection is a click on a blip, and a drag that starts on a blip
/// belongs to the blip. Picking reports logical pixels, NOVA OS physical
/// ones, so the rates match at a scale factor of 1.
fn orbit_drag<C: Component>(drag: On<Pointer<Drag>>, mut orbit: Query<&mut SketchOrbit, With<C>>) {
    if drag.original_event_target() != drag.entity {
        return;
    }
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    let gesture = OrbitGesture {
        drag: Some(drag.delta),
        ..default()
    };
    let (theta, phi) = gesture.apply(0.0, orbit.theta, orbit.phi);
    orbit.theta = theta;
    orbit.phi = phi;
}

/// Zoom the map by the wheel, out to what the live scene needs, as NOVA OS
/// does.
fn zoom_map(
    scroll: On<Pointer<Scroll>>,
    contacts: MapContacts,
    mut orbit: Query<&mut SketchOrbit, With<MapCamera>>,
) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    let reach = map_radius_max(map_spread(&contacts, orbit.center));
    orbit.radius = zoom_radius(orbit.radius, scroll.y, MAP_RADIUS_MIN, reach);
}

/// Zoom the ship by the wheel, within the NOVA OS reach of a hull.
fn zoom_ship(scroll: On<Pointer<Scroll>>, mut orbit: Query<&mut SketchOrbit, With<ShipCamera>>) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    orbit.radius = zoom_radius(orbit.radius, scroll.y, SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);
}

/// Put the map back on its opening framing around the player ship. The
/// selection stays, so the readout still names it.
fn reframe_map(
    _: On<Activate>,
    contacts: MapContacts,
    player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
    mut orbit: Query<&mut SketchOrbit, With<MapCamera>>,
) {
    let (Ok(player), Ok(mut orbit)) = (player.single(), orbit.single_mut()) else {
        return;
    };
    let focus = player.translation();
    orbit.theta = MAP_THETA;
    orbit.phi = MAP_PHI;
    orbit.radius = map_radius_default(map_spread(&contacts, focus));
    orbit.center_target = focus;
}

/// The theme colour a contact kind reads in.
fn kind_color(kind: MapContactKind) -> UiColor {
    match kind {
        MapContactKind::OwnShip => UiColor::Primary,
        MapContactKind::Ally => UiColor::Info,
        MapContactKind::Hostile => UiColor::Danger,
        MapContactKind::Objective => UiColor::Accent,
        MapContactKind::Terrain => UiColor::Secondary,
    }
}

impl MapMark {
    /// The legend word. A ship with no side is terrain to the contact model,
    /// and a neutral ship to the legend.
    fn label(self) -> &'static str {
        match (self.body, self.kind) {
            (BodyIcon::Ship, MapContactKind::OwnShip) => "Own ship",
            (BodyIcon::Ship, MapContactKind::Ally) => "Ally ship",
            (BodyIcon::Ship, MapContactKind::Hostile) => "Hostile ship",
            (BodyIcon::Ship, _) => "Neutral ship",
            (BodyIcon::Asteroid, _) => "Asteroid",
            (BodyIcon::Planet, _) => "Planet",
            (BodyIcon::Objective, _) => "Objective",
        }
    }

    /// Legend order: ships by stance, then rocks, planets and nav points.
    fn rank(self) -> (usize, usize) {
        let stance = match self.kind {
            MapContactKind::OwnShip => 0,
            MapContactKind::Ally => 1,
            MapContactKind::Hostile => 2,
            MapContactKind::Terrain => 3,
            MapContactKind::Objective => 4,
        };
        (self.body.index(), stance)
    }

    fn key_name(self) -> String {
        format!("Sketch Map Key {}", self.label())
    }
}

/// The body markers a contact carries.
type BodyMarkers<'w, 's> = Query<
    'w,
    's,
    (
        Has<SpaceshipRootMarker>,
        Has<AsteroidMarker>,
        Has<PlanetMarker>,
    ),
>;

/// What contact `entity` of `kind` is drawn as. The contact model plots
/// ships, asteroids, planets and objectives only, so a contact with none of
/// their markers is a bug in this example and panics.
fn map_mark(entity: Entity, kind: MapContactKind, bodies: &BodyMarkers) -> MapMark {
    let body = match (kind, bodies.get(entity)) {
        (MapContactKind::Objective, _) => BodyIcon::Objective,
        (_, Ok((true, _, _))) => BodyIcon::Ship,
        (_, Ok((_, true, _))) => BodyIcon::Asteroid,
        (_, Ok((_, _, true))) => BodyIcon::Planet,
        _ => panic!("map contact {entity} is no ship, asteroid, planet or objective"),
    };
    MapMark { body, kind }
}

/// Map blip tile and ship badge edge, in logical px.
const BLIP_PX: f32 = 22.0;
const BADGE_PX: f32 = 24.0;

/// A clickable contact tile over `host`: the body icon in its stance colour,
/// and a code label to its right.
fn spawn_blip(
    commands: &mut Commands,
    host: Entity,
    name: String,
    icons: &SketchIcons,
    mark: MapMark,
) -> Entity {
    let blip = commands
        .spawn((
            Name::new(name),
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: px(BLIP_PX),
                height: px(BLIP_PX),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(5)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Void, 0.55),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Accent, 0.0),
            ChildOf(host),
            children![icon_node(
                icons.bodies[mark.body.index()].clone(),
                kind_color(mark.kind),
                BLIP_PX - 6.0,
            )],
        ))
        .id();
    spawn_blip_label(commands, blip, BLIP_PX);
    blip
}

/// A clickable section badge over `host`: the family icon on a dark tile, a
/// status dot in its corner, and a code label to its right.
fn spawn_badge(
    commands: &mut Commands,
    host: Entity,
    name: String,
    icons: &SketchIcons,
    icon: SectionIcon,
    status: UiColor,
) -> Entity {
    let badge = commands
        .spawn((
            Name::new(name),
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: px(BADGE_PX),
                height: px(BADGE_PX),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(5)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Void, 0.8),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Accent, 0.0),
            ChildOf(host),
            children![
                icon_node(
                    icons.sections[icon.index()].clone(),
                    icon.color(),
                    BADGE_PX - 6.0
                ),
                (
                    StatusPip,
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(-4),
                        top: px(-4),
                        width: px(7),
                        height: px(7),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::new(status),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    spawn_blip_label(commands, badge, BADGE_PX);
    badge
}

/// The code label to the right of a blip `size` px wide.
fn spawn_blip_label(commands: &mut Commands, blip: Entity, size: f32) {
    commands.spawn((
        BlipLabel,
        Node {
            position_type: PositionType::Absolute,
            left: px(size + 2.0),
            top: px(size * 0.5 - 10.0),
            padding: UiRect::axes(px(4), px(1)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Void, 0.8),
        ChildOf(blip),
        children![themed_text("", 12.0, UiColor::Body)],
    ));
}

/// Place a blip `size` px wide over its projected point, or hide it when the
/// point is off the image. Writes only on a change, so still blips stay
/// unchanged for the UI layout.
fn place_blip(node: &mut Mut<Node>, visibility: &mut Mut<Visibility>, at: Option<Vec2>, size: f32) {
    match at {
        Some(at) => {
            let (left, top) = (px(at.x - size * 0.5), px(at.y - size * 0.5));
            if node.left != left || node.top != top {
                node.left = left;
                node.top = top;
            }
            visibility.set_if_neq(Visibility::Inherited);
        }
        None => {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

/// Where `world` lands on the pane, in logical px, or `None` off the image.
fn project(camera: (&Camera, &GlobalTransform), node: &ComputedNode, world: Vec3) -> Option<Vec2> {
    let to_logical = node.inverse_scale_factor();
    // A pane that is not laid out yet would put every point at the origin.
    if to_logical <= 0.0 {
        return None;
    }
    let size = node.size() * to_logical;
    camera
        .0
        .world_to_viewport(camera.1, world)
        .ok()
        .map(|at| at * to_logical)
        .filter(|at| at.x >= 0.0 && at.y >= 0.0 && at.x <= size.x && at.y <= size.y)
}

/// Keep one clickable blip per contact over the map image.
#[expect(
    clippy::too_many_arguments,
    reason = "blips, their labels and the scene"
)]
fn project_map_blips(
    mut commands: Commands,
    mut scene: ResMut<MapScene>,
    selection: Res<MapSelection>,
    contacts: MapContacts,
    markers: BodyMarkers,
    icons: Res<SketchIcons>,
    camera: Query<(&Camera, &GlobalTransform), With<MapCamera>>,
    nodes: Query<&ComputedNode>,
    mut blips: Query<(&mut Node, &mut Visibility, &Name, &Children), With<MapBlip>>,
    mut borders: Query<&mut ThemedBorder>,
    mut labels: Query<&Children, With<BlipLabel>>,
    mut texts: Query<&mut Text>,
) {
    let Some(scene) = scene.0.as_mut() else {
        return;
    };
    let (Ok(camera), Ok(pane)) = (camera.single(), nodes.get(scene.host)) else {
        return;
    };
    let list = contacts.collect();
    for contact in &list {
        let Some(&blip) = scene.blips.get(&contact.entity) else {
            let mark = map_mark(contact.entity, contact.kind, &markers);
            let blip = spawn_blip(
                &mut commands,
                scene.host,
                format!("Map Blip {}", contact.code),
                &icons,
                mark,
            );
            commands
                .entity(blip)
                .insert((
                    MapBlip {
                        contact: contact.entity,
                        mark,
                    },
                    SketchClick,
                ))
                .observe(select_map_contact);
            scene.blips.insert(contact.entity, blip);
            continue;
        };
        let Ok((mut node, mut visibility, name, children)) = blips.get_mut(blip) else {
            continue;
        };
        place_blip(
            &mut node,
            &mut visibility,
            project(camera, pane, contact.world_pos),
            BLIP_PX,
        );
        // Codes are minted a frame after a contact appears. The contact
        // model reads `Name`, so the rename goes through commands.
        let wanted = format!("Map Blip {}", contact.code);
        if name.as_str() != wanted {
            commands.entity(blip).insert(Name::new(wanted));
        }
        set_label(children, &mut labels, &mut texts, &contact.code);
        let alpha = if selection.0 == Some(contact.entity) {
            1.0
        } else {
            0.0
        };
        set_border(&mut borders, blip, alpha);
    }
    scene.blips.retain(|contact, blip| {
        let live = list.iter().any(|c| c.entity == *contact);
        if !live {
            commands.entity(*blip).try_despawn();
        }
        live
    });
}

/// Refill the map legend with one entry per mark the map plots, in
/// [`MapMark::rank`] order, when that set changes or the legend is new.
fn refresh_map_legend(
    mut commands: Commands,
    icons: Res<SketchIcons>,
    blips: Query<&MapBlip>,
    legends: Query<(Entity, Ref<MapLegend>)>,
    mut shown: Local<Vec<MapMark>>,
) {
    let Ok((legend, slot)) = legends.single() else {
        return;
    };
    let mut marks: Vec<MapMark> = blips.iter().map(|blip| blip.mark).collect();
    marks.sort_by_key(|mark| mark.rank());
    marks.dedup();
    if !slot.is_added() && *shown == marks {
        return;
    }
    commands
        .entity(legend)
        .despawn_related::<Children>()
        .with_children(|legend| {
            for mark in &marks {
                legend
                    .spawn((Name::new(mark.key_name()), legend_entry()))
                    .with_children(|entry| {
                        entry.spawn(icon_node(
                            icons.bodies[mark.body.index()].clone(),
                            kind_color(mark.kind),
                            16.0,
                        ));
                        text(entry, mark.label(), 12.0, UiColor::Body);
                    });
            }
        });
    *shown = marks;
}

/// Keep one clickable badge per section over the ship image.
#[expect(
    clippy::too_many_arguments,
    reason = "badges, their parts and the scene"
)]
fn project_ship_blips(
    mut commands: Commands,
    mut scene: ResMut<ShipScene>,
    selection: Res<ShipSelection>,
    sections: ShipSections,
    icons: Res<SketchIcons>,
    camera: Query<(&Camera, &GlobalTransform), With<ShipCamera>>,
    nodes: Query<&ComputedNode>,
    mut blips: Query<(&mut Node, &mut Visibility, &Children), With<ShipBlip>>,
    mut borders: Query<&mut ThemedBorder>,
    mut pips: Query<&mut ThemedFill, With<StatusPip>>,
    fixture: Res<SketchFixture>,
    mut labels: Query<(&Children, &mut Visibility), (With<BlipLabel>, Without<ShipBlip>)>,
    mut texts: Query<&mut Text>,
) {
    let Some(scene) = scene.0.as_mut() else {
        return;
    };
    let (Ok(camera), Ok(pane)) = (camera.single(), nodes.get(scene.host)) else {
        return;
    };
    let list = sections.collect();
    for view in &list {
        let Some(&blip) = scene.blips.get(&view.entity) else {
            let blip = spawn_badge(
                &mut commands,
                scene.host,
                format!("Ship Blip {}", view.code),
                &icons,
                SectionIcon::of(view.kind),
                status_color(condition_status(fixture.condition(&view.code))),
            );
            commands
                .entity(blip)
                .insert((
                    ShipBlip {
                        section: view.entity,
                    },
                    SketchClick,
                ))
                .observe(select_ship_section);
            scene.blips.insert(view.entity, blip);
            continue;
        };
        let Ok((mut node, mut visibility, children)) = blips.get_mut(blip) else {
            continue;
        };
        // The scene sits at the origin in ship-local space, like the NOVA OS
        // ship, so the local offset is the point to project.
        place_blip(
            &mut node,
            &mut visibility,
            project(camera, pane, view.local.translation),
            BADGE_PX,
        );
        let selected = selection.0 == Some(view.entity);
        set_border(&mut borders, blip, if selected { 1.0 } else { 0.0 });
        for child in children.iter() {
            if let Ok(mut pip) = pips.get_mut(child) {
                let wanted = status_color(condition_status(fixture.condition(&view.code)));
                if pip.color != wanted {
                    pip.color = wanted;
                }
            }
            // A picket has thirty sections; only the selected one is
            // labelled.
            if let Ok((label, mut visibility)) = labels.get_mut(child) {
                visibility.set_if_neq(if selected {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                for span in label.iter() {
                    if let Ok(mut text) = texts.get_mut(span) {
                        if text.0 != view.code {
                            text.0.clone_from(&view.code);
                        }
                    }
                }
            }
        }
    }
}

fn set_label(
    children: &Children,
    labels: &mut Query<&Children, With<BlipLabel>>,
    texts: &mut Query<&mut Text>,
    value: &str,
) {
    for child in children.iter() {
        let Ok(label) = labels.get_mut(child) else {
            continue;
        };
        for span in label.iter() {
            if let Ok(mut text) = texts.get_mut(span) {
                if text.0 != value {
                    text.0 = value.to_string();
                }
            }
        }
    }
}

/// Show the selection ring at `alpha`; written only on a change, since a
/// write repaints the node.
fn set_border(borders: &mut Query<&mut ThemedBorder>, blip: Entity, alpha: f32) {
    if let Ok(mut border) = borders.get_mut(blip) {
        if border.alpha != alpha {
            border.alpha = alpha;
        }
    }
}

/// The theme colour a section status reads in.
fn status_color(status: &str) -> UiColor {
    match status {
        "nominal" => UiColor::Nominal,
        "degraded" => UiColor::Accent,
        "critical" => UiColor::Danger,
        _ => UiColor::Label,
    }
}

fn select_map_contact(
    activate: On<Activate>,
    blips: Query<&MapBlip>,
    mut selection: ResMut<MapSelection>,
) {
    if let Ok(blip) = blips.get(activate.entity) {
        selection.0 = Some(blip.contact);
    }
}

/// Move the ship selection `step` sections along the code order, wrapping.
/// With nothing selected, Next picks the first and Prev the last.
fn step_section(step: isize) -> impl Fn(On<Activate>, ShipSections, ResMut<ShipSelection>) {
    move |_, sections, mut selection| {
        let views = sections.collect();
        if views.is_empty() {
            return;
        }
        let count = views.len() as isize;
        let at = selection
            .0
            .and_then(|selected| views.iter().position(|view| view.entity == selected));
        let next = match at {
            Some(at) => (at as isize + step).rem_euclid(count),
            None if step > 0 => 0,
            None => count - 1,
        };
        selection.0 = Some(views[next as usize].entity);
    }
}

/// Frame the whole ship at the current angles, as the pane opens.
fn fit_ship(
    _: On<Activate>,
    sections: ShipSections,
    mut orbit: Query<&mut SketchOrbit, With<ShipCamera>>,
) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    let (centre, radius) = ship_framing(&sections.collect());
    orbit.radius = radius * SHIP_PANE_ZOOM;
    orbit.center_target = centre;
}

/// Put the ship view back on its opening framing and angles.
fn reset_ship(
    _: On<Activate>,
    sections: ShipSections,
    mut orbit: Query<&mut SketchOrbit, With<ShipCamera>>,
) {
    let Ok(mut orbit) = orbit.single_mut() else {
        return;
    };
    let (centre, radius) = ship_framing(&sections.collect());
    orbit.theta = SHIP_THETA;
    orbit.phi = SHIP_PHI;
    orbit.radius = radius * SHIP_PANE_ZOOM;
    orbit.center_target = centre;
}

fn select_ship_section(
    activate: On<Activate>,
    blips: Query<&ShipBlip>,
    mut selection: ResMut<ShipSelection>,
) {
    if let Ok(blip) = blips.get(activate.entity) {
        selection.0 = Some(blip.section);
    }
}

/// Swap the selected section's outline to the selection material.
fn outline_selected_section(
    selection: Res<ShipSelection>,
    palette: Res<SceneMaterials>,
    mut outlines: Query<(&BlockOutline, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (outline, mut material) in &mut outlines {
        let wanted = if selection.0 == Some(outline.section) {
            &palette.outline_selected
        } else {
            &palette.outline
        };
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}

fn update_map_readout(
    selection: Res<MapSelection>,
    contacts: MapContacts,
    mut texts: Query<(&Name, &mut Text)>,
) {
    let line = selection
        .0
        .and_then(|selected| {
            contacts
                .collect()
                .into_iter()
                .find(|c| c.entity == selected)
        })
        .map(|contact| {
            format!(
                "{}  {}  {}  {}",
                contact.code,
                contact.name,
                contact.kind.label(),
                nova_ui::units::distance(Meters::from_engine(contact.range)),
            )
        })
        .unwrap_or_else(|| "Select a contact.".to_string());
    set_named_text(&mut texts, MAP_READOUT, &line);
}

/// The selected section's code, name, family and fixture condition in the
/// ship panel, with its family icon and a condition bar.
fn update_ship_detail(
    selection: Res<ShipSelection>,
    sections: ShipSections,
    fixture: Res<SketchFixture>,
    icons: Res<SketchIcons>,
    mut texts: Query<(&Name, &mut Text)>,
    mut previews: Query<(&mut ImageNode, &mut ThemedImageTint), With<ShipPreview>>,
    mut bars: Query<(&mut Node, &mut ThemedFill), With<ConditionFill>>,
) {
    let Some(view) = selection.0.and_then(|selected| {
        sections
            .collect()
            .into_iter()
            .find(|v| v.entity == selected)
    }) else {
        return;
    };
    let pct = fixture.condition(&view.code);
    let icon = SectionIcon::of(view.kind);
    set_named_text(
        &mut texts,
        SHIP_DETAIL,
        &format!("{}  {}", view.code, view.name),
    );
    set_named_text(
        &mut texts,
        SHIP_STATUS,
        &format!(
            "{}  Condition {}%  {}",
            icon.label(),
            pct,
            condition_status(pct)
        ),
    );
    for (mut image, mut tint) in &mut previews {
        let wanted = &icons.sections[icon.index()];
        if image.image != *wanted {
            image.image = wanted.clone();
        }
        if tint.color != icon.color() {
            tint.color = icon.color();
        }
    }
    let status = status_color(condition_status(pct));
    for (mut node, mut fill) in &mut bars {
        let width = percent(pct as f32);
        if node.width != width {
            node.width = width;
        }
        if fill.color != status {
            fill.color = status;
        }
    }
}

fn set_named_text(texts: &mut Query<(&Name, &mut Text)>, name: &str, value: &str) {
    for (node, mut text) in texts.iter_mut() {
        if node.as_str() == name && text.0 != value {
            text.0 = value.to_string();
        }
    }
}

/// Paint the 3D materials and scene backgrounds for the live theme.
fn repaint_scenes(
    theme: Res<ActiveUiTheme>,
    palette: Res<SceneMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cameras: Query<&mut Camera, Or<(With<MapCamera>, With<ShipCamera>)>>,
) {
    if !theme.is_changed() && !palette.is_added() {
        return;
    }
    let blocks = SectionIcon::ALL.map(|icon| {
        (
            &palette.blocks[icon.index()],
            icon.color(),
            icon.block_alpha(),
        )
    });
    for (handle, color, alpha) in [
        (&palette.ring, UiColor::Secondary, 0.6),
        (&palette.hub, UiColor::Primary, 1.0),
        (&palette.outline, UiColor::Primary, 0.95),
        (&palette.outline_selected, UiColor::Accent, 1.0),
        (&palette.bow, UiColor::Accent, 0.9),
    ]
    .into_iter()
    .chain(blocks)
    {
        if let Some(mut material) = materials.get_mut(handle) {
            material.base_color = theme.color_alpha(color, alpha);
        }
    }
    for mut camera in &mut cameras {
        camera.clear_color = ClearColorConfig::Custom(theme.color(UiColor::Surface));
    }
}

// Fixture market, holds and repair.

/// How a good counts: bulk in whole tonnes, or single items.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unit {
    Tonne,
    Item,
}

/// A fixture good: what it is for, how it counts, what one unit weighs and
/// what the mock station pays or asks for it.
#[derive(PartialEq, Debug)]
struct Good {
    label: &'static str,
    category: Category,
    unit: Unit,
    unit_kg: u32,
    /// Fixture market price of one unit, bought or sold.
    price_cr: u32,
    /// The inspector's one-line description.
    about: &'static str,
}

static GOODS: [Good; 10] = [
    Good {
        label: "Nickel ore",
        category: Category::Raw,
        unit: Unit::Tonne,
        unit_kg: 1000,
        price_cr: 30,
        about: "Refinable metal ore from belt rock.",
    },
    Good {
        label: "Ice",
        category: Category::Raw,
        unit: Unit::Tonne,
        unit_kg: 1000,
        price_cr: 12,
        about: "Water ice for life support and reaction mass.",
    },
    Good {
        label: "Hull plate",
        category: Category::Repair,
        unit: Unit::Item,
        unit_kg: 400,
        price_cr: 25,
        about: "Patch plate for hull sections.",
    },
    Good {
        label: "Slugs",
        category: Category::Ammo,
        unit: Unit::Item,
        unit_kg: 20,
        price_cr: 2,
        about: "Kinetic rounds for turrets.",
    },
    Good {
        label: "Core",
        category: Category::Parts,
        unit: Unit::Item,
        unit_kg: 500,
        price_cr: 300,
        about: "A spare controller core.",
    },
    Good {
        label: "Fuel",
        category: Category::Fuel,
        unit: Unit::Item,
        unit_kg: 300,
        price_cr: 40,
        about: "A sealed fuel cell for thrusters.",
    },
    Good {
        label: "Scrap",
        category: Category::Raw,
        unit: Unit::Tonne,
        unit_kg: 1000,
        price_cr: 8,
        about: "Mixed salvage metal, sold by weight.",
    },
    Good {
        label: "Water",
        category: Category::Food,
        unit: Unit::Tonne,
        unit_kg: 1000,
        price_cr: 10,
        about: "Potable water for the crew.",
    },
    Good {
        label: "Rations",
        category: Category::Food,
        unit: Unit::Item,
        unit_kg: 50,
        price_cr: 5,
        about: "Packed crew meals.",
    },
    Good {
        label: "Pump",
        category: Category::Parts,
        unit: Unit::Item,
        unit_kg: 800,
        price_cr: 700,
        about: "A coolant pump for a reactor loop.",
    },
];

/// The fixture good called `label`. An unlisted label is an authoring bug.
fn good(label: &str) -> &'static Good {
    GOODS
        .iter()
        .find(|good| good.label == label)
        .unwrap_or_else(|| panic!("no fixture good `{label}`"))
}

impl Good {
    /// `qty` units as the quantity column shows them.
    fn amount(&self, qty: u32) -> String {
        match self.unit {
            Unit::Tonne => format!("{qty} t"),
            Unit::Item => format!("x{qty}"),
        }
    }
}

/// Where cargo sits: the picket's hold, the mock station market, or the
/// boarded raider's hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Store {
    Own,
    Market,
    Boarded,
}

impl Store {
    fn title(self) -> String {
        match self {
            Self::Own => format!("{} hold", OWN_SHIP.name),
            Self::Market => format!("{STATION_NAME} market"),
            Self::Boarded => format!("{} hold", BOARDED.name),
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Own => "Own",
            Self::Market => "Market",
            Self::Boarded => "Boarded",
        }
    }

    fn panel_name(self) -> String {
        format!("Sketch Store {}", self.tag())
    }

    fn row_name(self, good: &Good) -> String {
        format!("Sketch Row {} {}", self.tag(), good.label)
    }

    fn load_name(self) -> String {
        format!("Sketch Load {}", self.tag())
    }

    fn fill_name(self) -> String {
        format!("Sketch Fill {}", self.tag())
    }
}

/// One row of a store.
#[derive(Clone, PartialEq, Debug)]
struct Line {
    good: &'static Good,
    qty: u32,
}

impl Line {
    fn mass_kg(&self) -> u32 {
        self.qty * self.good.unit_kg
    }
}

/// A store's lines, in the order they joined it.
#[derive(Clone, PartialEq, Debug)]
struct Stock(Vec<Line>);

impl Stock {
    fn new(lines: &[(&str, u32)]) -> Self {
        Self(
            lines
                .iter()
                .map(|(label, qty)| Line {
                    good: good(label),
                    qty: *qty,
                })
                .collect(),
        )
    }

    fn qty(&self, good: &Good) -> u32 {
        self.0
            .iter()
            .find(|line| line.good == good)
            .map_or(0, |line| line.qty)
    }

    fn mass_kg(&self) -> u32 {
        self.0.iter().map(Line::mass_kg).sum()
    }

    /// Take `qty` units off a line; a line at zero leaves the store. The
    /// caller checked the line holds them.
    fn take(&mut self, good: &Good, qty: u32) {
        let at = self
            .0
            .iter()
            .position(|line| line.good == good)
            .expect("a checked line");
        self.0[at].qty -= qty;
        if self.0[at].qty == 0 {
            self.0.remove(at);
        }
    }

    /// Add `qty` units to a line, which joins the store's end if it is new.
    fn put(&mut self, good: &'static Good, qty: u32) {
        match self.0.iter_mut().find(|line| line.good == good) {
            Some(line) => line.qty += qty,
            None => self.0.push(Line { good, qty }),
        }
    }
}

/// A ship's hold under an example-only weight limit.
#[derive(Clone, PartialEq, Debug)]
struct FixtureHold {
    capacity_kg: u32,
    stock: Stock,
}

impl FixtureHold {
    /// The load line: carried over the limit.
    fn load(&self) -> String {
        format!(
            "{} / {}",
            tonnes(self.stock.mass_kg()),
            tonnes(self.capacity_kg)
        )
    }

    fn fits(&self, kg: u32) -> bool {
        self.stock.mass_kg() + kg <= self.capacity_kg
    }
}

/// A cargo move between two stores. The station prices a buy and a sale;
/// loot from the boarded ship is free and moves only into the picket's hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Deal {
    Buy,
    Sell,
    Loot,
}

impl Deal {
    #[cfg(feature = "debug")]
    const ALL: [Self; 3] = [Self::Buy, Self::Sell, Self::Loot];

    /// The deal a click on an item in `store` opens, if the context allows
    /// one: Buy from the market and Sell from the picket's
    /// hold at the station, Loot from the raider's hold when boarded. The
    /// picket's own cargo has no deal when boarded: there is no stow.
    fn offered(context: SketchContext, store: Store) -> Option<Self> {
        match (context, store) {
            (SketchContext::Station, Store::Market) => Some(Self::Buy),
            (SketchContext::Station, Store::Own) => Some(Self::Sell),
            (SketchContext::Boarded, Store::Boarded) => Some(Self::Loot),
            _ => None,
        }
    }

    /// Where the cargo comes from and goes to.
    fn stores(self) -> (Store, Store) {
        match self {
            Self::Buy => (Store::Market, Store::Own),
            Self::Sell => (Store::Own, Store::Market),
            Self::Loot => (Store::Boarded, Store::Own),
        }
    }

    /// The context the deal needs.
    fn context(self) -> SketchContext {
        match self {
            Self::Buy | Self::Sell => SketchContext::Station,
            Self::Loot => SketchContext::Boarded,
        }
    }

    fn unit_price(self, good: &Good) -> u32 {
        match self {
            Self::Buy | Self::Sell => good.price_cr,
            Self::Loot => 0,
        }
    }

    fn title(self) -> String {
        match self {
            Self::Buy => format!("Buy from {}", Store::Market.title()),
            Self::Sell => format!("Sell to {}", Store::Market.title()),
            Self::Loot => format!("Loot from {}", Store::Boarded.title()),
        }
    }

    /// The price line of the confirmation, while the typed quantity is a
    /// number.
    fn summary(self, good: &Good, qty: u32) -> String {
        let (price, total) = (self.unit_price(good), self.unit_price(good) * qty);
        match self {
            Self::Buy => format!("{price} cr each  Total -{total} cr"),
            Self::Sell => format!("{price} cr each  Total +{total} cr"),
            Self::Loot => "Free  no credits change".to_string(),
        }
    }
}

/// `kg` as the screens show a hold's mass.
fn tonnes(kg: u32) -> String {
    format!("{:.1} t", kg as f32 / 1000.0)
}

/// `kg` as the inspector shows one unit: kilograms under a tonne.
fn unit_mass(kg: u32) -> String {
    if kg < 1000 {
        format!("{kg} kg")
    } else {
        tonnes(kg)
    }
}

/// The example's working mock of a station visit and a boarding: the stores,
/// the credits, each section's condition, the station repair bay and the
/// last result. Example state only: nothing reads it outside the screens,
/// the gameplay ship is never changed, and none of it survives the run.
#[derive(Resource, Clone, PartialEq, Debug)]
struct SketchFixture {
    own: FixtureHold,
    market: Stock,
    boarded: FixtureHold,
    credits: u32,
    /// Condition in percent by section code; a code not listed is intact.
    condition: HashMap<String, u32>,
    /// The station's repair capability. Off, the station refuses repairs.
    repair_bay: bool,
    /// The last transaction's result or refusal.
    notice: String,
}

impl Default for SketchFixture {
    fn default() -> Self {
        Self {
            own: FixtureHold {
                capacity_kg: 12_000,
                stock: Stock::new(&[
                    ("Nickel ore", 4),
                    ("Ice", 2),
                    ("Hull plate", 3),
                    ("Slugs", 12),
                    ("Core", 1),
                    ("Fuel", 2),
                    ("Scrap", 2),
                ]),
            },
            market: Stock::new(&[
                ("Water", 6),
                ("Hull plate", 8),
                ("Slugs", 40),
                ("Rations", 20),
                ("Fuel", 4),
                ("Pump", 2),
            ]),
            boarded: FixtureHold {
                capacity_kg: 8_000,
                stock: Stock::new(&[
                    ("Slugs", 60),
                    ("Hull plate", 2),
                    ("Rations", 6),
                    ("Core", 1),
                ]),
            },
            credits: START_CREDITS,
            condition: WEAR
                .iter()
                .map(|(code, pct)| (code.to_string(), *pct))
                .collect(),
            repair_bay: true,
            notice: "Fixture state: no transaction yet".to_string(),
        }
    }
}

impl SketchFixture {
    fn stock(&self, store: Store) -> &Stock {
        match store {
            Store::Own => &self.own.stock,
            Store::Market => &self.market,
            Store::Boarded => &self.boarded.stock,
        }
    }

    fn stock_mut(&mut self, store: Store) -> &mut Stock {
        match store {
            Store::Own => &mut self.own.stock,
            Store::Market => &mut self.market,
            Store::Boarded => &mut self.boarded.stock,
        }
    }

    /// The store's hold, if it has a weight limit. The market has none.
    fn hold(&self, store: Store) -> Option<&FixtureHold> {
        match store {
            Store::Own => Some(&self.own),
            Store::Market => None,
            Store::Boarded => Some(&self.boarded),
        }
    }

    fn credits_text(&self) -> String {
        format!("{} cr  fixture", self.credits)
    }

    fn condition(&self, code: &str) -> u32 {
        self.condition.get(code).copied().unwrap_or(100)
    }

    fn repair_price(&self, code: &str) -> u32 {
        (100 - self.condition(code)) * REPAIR_CR_PER_POINT
    }

    /// The quantity `draft` would move in `context`, or the one refusal line
    /// that names why it cannot: the context, a missing or zero quantity,
    /// the source stock, the destination hold or the credits.
    fn check(&self, context: SketchContext, draft: DealDraft) -> Result<u32, String> {
        let DealDraft { deal, good, qty } = draft;
        if context != deal.context() {
            return Err(match deal {
                Deal::Buy | Deal::Sell => "Refused: not at a station",
                Deal::Loot => "Refused: not boarded",
            }
            .to_string());
        }
        let Some(qty) = qty else {
            return Err("Refused: enter a quantity".to_string());
        };
        if qty == 0 {
            return Err("Refused: quantity is zero".to_string());
        }
        let (from, to) = deal.stores();
        let have = self.stock(from).qty(good);
        if have < qty {
            return Err(format!(
                "Refused: only {} {} left",
                good.amount(have),
                good.label
            ));
        }
        if self
            .hold(to)
            .is_some_and(|hold| !hold.fits(qty * good.unit_kg))
        {
            return Err(format!("Refused: {} full", to.title()));
        }
        let total = deal.unit_price(good) * qty;
        if deal == Deal::Buy && total > self.credits {
            return Err(format!(
                "Refused: need {total} cr, have {} cr",
                self.credits
            ));
        }
        Ok(qty)
    }

    /// Run a confirmed deal, or refuse it with [`Self::check`]'s reason and
    /// change nothing.
    fn transfer(&mut self, context: SketchContext, draft: DealDraft) -> Result<String, String> {
        let qty = self.check(context, draft)?;
        let DealDraft { deal, good, .. } = draft;
        let (from, to) = deal.stores();
        let total = deal.unit_price(good) * qty;
        self.stock_mut(from).take(good, qty);
        self.stock_mut(to).put(good, qty);
        let moved = format!("{} {}", good.amount(qty), good.label);
        Ok(match deal {
            Deal::Buy => {
                self.credits -= total;
                format!("Bought {moved}  -{total} cr")
            }
            Deal::Sell => {
                self.credits += total;
                format!("Sold {moved}  +{total} cr")
            }
            Deal::Loot => format!("Looted {moved}  free"),
        })
    }

    /// Repair section `code` to full at the station, or refuse with the
    /// reason and change nothing.
    fn repair(&mut self, context: SketchContext, code: &str) -> Result<String, String> {
        if context != SketchContext::Station {
            return Err("Refused: repair needs a station".to_string());
        }
        if !self.repair_bay {
            return Err("Refused: repair bay off".to_string());
        }
        let price = self.repair_price(code);
        if price == 0 {
            return Err(format!("Refused: {code} intact"));
        }
        if price > self.credits {
            return Err(format!(
                "Refused: need {price} cr, have {} cr",
                self.credits
            ));
        }
        self.credits -= price;
        self.condition.remove(code);
        Ok(format!("Repaired {code}  -{price} cr"))
    }
}

/// A condition in percent read as a status, on the NOVA OS thresholds.
fn condition_status(pct: u32) -> &'static str {
    match pct {
        0..=25 => "critical",
        26..=70 => "degraded",
        _ => "nominal",
    }
}

/// The item the inspector shows, and the store it was picked in.
#[derive(Resource, Default, PartialEq)]
struct Inspected(Option<(Store, &'static Good)>);

/// The category the store rows show; `None` shows all.
#[derive(Resource, Default, PartialEq)]
struct CargoFilter(Option<Category>);

/// A deal waiting for its quantity and price to be confirmed.
#[derive(Clone, Copy, PartialEq, Debug)]
struct DealDraft {
    deal: Deal,
    good: &'static Good,
    /// The one quantity the wheel, the slider, the field and All set. `None`
    /// while the field holds text that is not a number, which Confirm
    /// refuses.
    qty: Option<u32>,
}

/// The open confirmation, if a row click opened one.
#[derive(Resource, Default)]
struct Draft(Option<DealDraft>);

/// The quantity row: the wheel over it steps the draft by one.
/// `Hovered` tells [`scroll_body`] the wheel belongs to the row.
#[derive(Component)]
struct DraftWheel;

/// The confirmation's quantity slider, from zero to the source stock.
#[derive(Component)]
struct DraftSlider;

/// The confirmation's typed quantity.
#[derive(Component)]
struct DraftField;

/// A store row: a click selects it for the inspector.
#[derive(Component)]
struct CargoRow {
    store: Store,
    good: &'static Good,
}

/// A category filter button; `None` shows all.
#[derive(Component)]
struct FilterChip(Option<Category>);

/// The node [`refresh_repair_slot`] fills with the selected section's repair.
#[derive(Component)]
struct RepairSlot;

/// One of the two store columns: the picket's hold, or the store the context
/// adds. `drawn` is the store, stock and filter its rows show, so
/// [`refresh_store_columns`] rebuilds them only when one of those changes.
#[derive(Component)]
struct StoreColumn {
    partner: bool,
    drawn: Option<(Option<Store>, Stock, Option<Category>)>,
}

/// A part of the inspector that shows or hides with the selection and the
/// open confirmation. Built once; [`update_inspector`] sets its display.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum InspectorPart {
    /// The hint with nothing selected.
    Hint,
    /// The selected item's icon, name and category, over the parts below.
    Item,
    /// The item's description, unit mass, stock and price.
    Facts,
    /// The quantity and price confirmation.
    Form,
}

/// Rebuild the repair slot when it is new, or when the fixture, the context
/// or the selected section changes. Only the slot's children are respawned:
/// the live 3D scene beside it is never touched.
fn refresh_repair_slot(
    mut commands: Commands,
    fixture: Res<SketchFixture>,
    context: Res<SketchContext>,
    selection: Res<ShipSelection>,
    sections: ShipSections,
    slots: Query<(Entity, Ref<RepairSlot>)>,
) {
    let changed = fixture.is_changed() || context.is_changed() || selection.is_changed();
    for (entity, slot) in &slots {
        if !slot.is_added() && !changed {
            continue;
        }
        let code = selection.0.and_then(|selected| {
            sections
                .collect()
                .into_iter()
                .find(|view| view.entity == selected)
                .map(|view| view.code)
        });
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .with_children(|slot| {
                if let Some(code) = code {
                    fill_repair(slot, &fixture, *context, &code);
                }
            });
    }
}

/// Rebuild a store column's panel when its store, that store's stock or the
/// filter differs from what it drew. A selection, a quantity change, a refused
/// Confirm or a repair changes none of them, so the rows the pointer clicks
/// stay the same entities.
fn refresh_store_columns(
    mut commands: Commands,
    fixture: Res<SketchFixture>,
    context: Res<SketchContext>,
    filter: Res<CargoFilter>,
    icons: Res<SketchIcons>,
    mut columns: Query<(Entity, &mut StoreColumn)>,
) {
    let changed = fixture.is_changed() || context.is_changed() || filter.is_changed();
    for (entity, mut column) in &mut columns {
        if column.drawn.is_some() && !changed {
            continue;
        }
        let store = if column.partner {
            context.partner()
        } else {
            Some(Store::Own)
        };
        let stock = store.map_or_else(|| Stock(Vec::new()), |store| fixture.stock(store).clone());
        let wanted = Some((store, stock, filter.0));
        if column.drawn == wanted {
            continue;
        }
        column.drawn = wanted;
        let mut column_commands = commands.entity(entity);
        column_commands.despawn_related::<Children>();
        if let Some(store) = store {
            column_commands.with_children(|column| {
                store_panel(column, &fixture, store, filter.0, &icons);
            });
        }
    }
}

/// Close the confirmation, and drop an inspection of a store the new context
/// hides, when the context changes.
fn settle_inventory(
    context: Res<SketchContext>,
    mut inspected: ResMut<Inspected>,
    mut draft: ResMut<Draft>,
) {
    if !context.is_changed() {
        return;
    }
    if draft.0.is_some() {
        draft.0 = None;
    }
    if inspected
        .0
        .is_some_and(|(store, _)| store != Store::Own && Some(store) != context.partner())
    {
        inspected.0 = None;
    }
}

/// The selected section's repair: at the station, the repair bay switch and a
/// priced repair while the bay is on; elsewhere, a line saying repair needs a
/// station.
fn fill_repair(
    slot: &mut ChildSpawnerCommands,
    fixture: &SketchFixture,
    context: SketchContext,
    code: &str,
) {
    if context != SketchContext::Station {
        slot.spawn((
            Name::new(SHIP_NO_REPAIR),
            themed_text("Repair at a station only", 12.0, UiColor::Label),
        ));
        return;
    }
    let bay = ButtonSpec::new(if fixture.repair_bay {
        "Repair bay on"
    } else {
        "Repair bay off"
    })
    .fit();
    let bay = if fixture.repair_bay { bay } else { bay.ghost() };
    slot.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            compact_button(row, bay, SHIP_BAY)
                .insert(SketchClick)
                .observe(toggle_repair_bay);
            text(row, "fixture", 11.0, UiColor::Label);
        });
    if !fixture.repair_bay {
        slot.spawn((
            Name::new(SHIP_NO_REPAIR),
            themed_text("The bay is off: no repair here", 12.0, UiColor::Label),
        ));
        return;
    }
    slot.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            let price = fixture.repair_price(code);
            if price == 0 {
                compact_button(row, ButtonSpec::new("Intact").fit(), SHIP_REPAIR)
                    .insert(bevy::ui::InteractionDisabled);
            } else {
                compact_button(
                    row,
                    ButtonSpec::new(format!("Repair {code}  {price} cr"))
                        .fit()
                        .primary(),
                    SHIP_REPAIR,
                )
                .observe(repair_selected);
            }
        });
    text(
        slot,
        &format!("{REPAIR_CR_PER_POINT} cr per point"),
        11.0,
        UiColor::Label,
    );
}

/// Height of the inspector stacked under the stores, in logical px.
const INSPECTOR_NARROW_PX: f32 = 350.0;

/// The filter bar and the store columns, and the inspector beside them, or
/// under them when narrow. Built once per view: [`refresh_store_columns`]
/// fills the columns and [`update_inspector`] the inspector.
fn inventory_view(
    body: &mut ChildSpawnerCommands,
    icons: &SketchIcons,
    narrow: bool,
    min_height: f32,
) {
    card(body, PANE_HOLD, "Inventory", min_height, |c| {
        c.spawn(split(narrow)).with_children(|split| {
            split
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                })
                .with_children(|holds| {
                    holds
                        .spawn(control_row(JustifyContent::FlexStart))
                        .with_children(|bar| {
                            filter_chip(bar, None, icons);
                            for category in Category::ALL {
                                filter_chip(bar, Some(category), icons);
                            }
                        });
                    holds
                        .spawn(Node {
                            flex_grow: 1.0,
                            min_height: px(0),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(12),
                            ..default()
                        })
                        .with_children(|stores| {
                            // Equal halves whether or not the context fills
                            // the second: the picket's hold keeps its width
                            // when a store joins it, and the store matches it.
                            for (partner, name) in
                                [(false, STORE_COLUMN_OWN), (true, STORE_COLUMN_PARTNER)]
                            {
                                stores.spawn((
                                    Name::new(name),
                                    StoreColumn {
                                        partner,
                                        drawn: None,
                                    },
                                    Node {
                                        flex_grow: 1.0,
                                        flex_basis: px(0),
                                        min_width: px(0),
                                        min_height: px(0),
                                        flex_direction: FlexDirection::Column,
                                        ..default()
                                    },
                                ));
                            }
                        });
                });
            inspector(split, icons, narrow);
        });
    });
}

/// One filter button: the category icon and word, or `All`.
/// [`mark_filter_chips`] marks the current filter.
fn filter_chip(bar: &mut ChildSpawnerCommands, chip: Option<Category>, icons: &SketchIcons) {
    let (name, label, tone) = match chip {
        Some(category) => (category.filter_name(), category.label(), category.color()),
        None => (FILTER_ALL.to_string(), "All", UiColor::Primary),
    };
    bar.spawn((
        Name::new(name),
        Button,
        FilterChip(chip),
        SketchClick,
        Node {
            height: px(26),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(4),
            padding: UiRect::axes(px(8), px(0)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.08),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.4),
    ))
    .observe(pick_filter)
    .with_children(|chip_node| {
        if let Some(category) = chip {
            chip_node.spawn(icon_node(icons.cargo[category.index()].clone(), tone, 16.0));
        }
        chip_node.spawn((themed_text(label, 12.0, UiColor::Body), Pickable::IGNORE));
    });
}

fn pick_filter(activate: On<Activate>, chips: Query<&FilterChip>, mut filter: ResMut<CargoFilter>) {
    if let Ok(chip) = chips.get(activate.entity) {
        if filter.0 != chip.0 {
            filter.0 = chip.0;
        }
    }
}

/// Mark the current filter's chip in place.
fn mark_filter_chips(
    filter: Res<CargoFilter>,
    mut chips: Query<(&FilterChip, &mut ThemedFill, &mut ThemedBorder)>,
) {
    for (chip, mut fill, mut border) in &mut chips {
        let (color, fill_alpha, border_alpha) = if chip.0 == filter.0 {
            (UiColor::Accent, 0.25, 1.0)
        } else {
            (UiColor::Secondary, 0.08, 0.4)
        };
        if fill.color != color || fill.alpha != fill_alpha {
            fill.color = color;
            fill.alpha = fill_alpha;
        }
        if border.color != color || border.alpha != border_alpha {
            border.color = color;
            border.alpha = border_alpha;
        }
    }
}

/// Height of one store row, in logical px. Every row is the same height, so a
/// line reads the same in every store.
const ROW_PX: f32 = 28.0;
/// Width of the quantity and mass or price columns, in logical px.
const COLUMN_PX: f32 = 64.0;

/// A titled store: a hold shows its load against the limit, the market its
/// prices. Then one fixed-height row per line the filter shows.
fn store_panel(
    column: &mut ChildSpawnerCommands,
    fixture: &SketchFixture,
    store: Store,
    filter: Option<Category>,
    icons: &SketchIcons,
) {
    let hold = fixture.hold(store);
    column
        .spawn((
            Name::new(store.panel_name()),
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Secondary, 0.35),
        ))
        .with_children(|panel| {
            panel
                .spawn(control_row(JustifyContent::FlexStart))
                .with_children(|head| {
                    head.spawn((
                        themed_text(&store.title(), 15.0, UiColor::Primary),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                    head.spawn((
                        Name::new(store.load_name()),
                        themed_text(
                            &hold.map_or_else(|| "Fixture prices".to_string(), FixtureHold::load),
                            13.0,
                            UiColor::Body,
                        ),
                    ));
                });
            if let Some(hold) = hold {
                panel.spawn(bar_track()).with_children(|track| {
                    track.spawn((
                        Name::new(store.fill_name()),
                        Node {
                            width: percent(
                                hold.stock.mass_kg() as f32 / hold.capacity_kg as f32 * 100.0,
                            ),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        ThemedFill::new(UiColor::Primary),
                    ));
                });
            }
            let headings = if hold.is_some() {
                ["Qty", "Mass"]
            } else {
                ["Stock", "Price"]
            };
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::axes(px(8), px(0)),
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|columns| {
                    columns.spawn((
                        themed_text("Item", 11.0, UiColor::Label),
                        Node {
                            flex_grow: 1.0,
                            margin: UiRect::left(px(27)),
                            ..default()
                        },
                    ));
                    for heading in headings {
                        columns.spawn((
                            themed_text(heading, 11.0, UiColor::Label),
                            Node {
                                width: px(COLUMN_PX),
                                justify_content: JustifyContent::FlexEnd,
                                ..default()
                            },
                            TextLayout::new(Justify::Right, LineBreak::NoWrap),
                        ));
                    }
                });
            let lines: Vec<&Line> = fixture
                .stock(store)
                .0
                .iter()
                .filter(|line| filter.is_none_or(|category| line.good.category == category))
                .collect();
            if lines.is_empty() {
                text(
                    panel,
                    if filter.is_some() {
                        "Nothing in this category"
                    } else {
                        "Empty"
                    },
                    12.0,
                    UiColor::Label,
                );
            }
            for line in lines {
                store_row(panel, store, line, hold.is_some(), icons);
            }
        });
}

/// Fill and border alpha of a row, selected or not.
fn row_alphas(selected: bool) -> (f32, f32) {
    if selected {
        (0.32, 1.0)
    } else {
        (0.1, 0.6)
    }
}

fn store_row(
    panel: &mut ChildSpawnerCommands,
    store: Store,
    line: &Line,
    hold: bool,
    icons: &SketchIcons,
) {
    let tone = line.good.category.color();
    let value = if hold {
        tonnes(line.mass_kg())
    } else {
        format!("{} cr", line.good.price_cr)
    };
    let (fill, border) = row_alphas(false);
    panel
        .spawn((
            Name::new(store.row_name(line.good)),
            CargoRow {
                store,
                good: line.good,
            },
            Node {
                height: px(ROW_PX),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::axes(px(8), px(0)),
                border: UiRect::left(px(3)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(tone, fill),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(tone, border),
        ))
        .observe(inspect_row)
        .with_children(|row| {
            row.spawn(icon_node(
                icons.cargo[line.good.category.index()].clone(),
                tone,
                18.0,
            ));
            row.spawn((
                themed_text(line.good.label, 13.0, UiColor::Body),
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                },
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                Pickable::IGNORE,
            ));
            for (value, color) in [(line.good.amount(line.qty), tone), (value, UiColor::Body)] {
                row.spawn((
                    themed_text(&value, 13.0, color),
                    Node {
                        width: px(COLUMN_PX),
                        ..default()
                    },
                    TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    Pickable::IGNORE,
                ));
            }
        });
}

/// Mark the selected row in place. A new row is marked the frame it spawns.
fn mark_cargo_rows(
    inspected: Res<Inspected>,
    mut rows: Query<(&CargoRow, &mut ThemedFill, &mut ThemedBorder)>,
) {
    for (row, mut fill, mut border) in &mut rows {
        let (fill_alpha, border_alpha) = row_alphas(inspected.0 == Some((row.store, row.good)));
        if fill.alpha != fill_alpha {
            fill.alpha = fill_alpha;
        }
        if border.alpha != border_alpha {
            border.alpha = border_alpha;
        }
    }
}

/// Select a clicked row for the inspector and open the confirmation of the
/// deal the context allows on it at one unit, closing any other, with one
/// click cue. A click on the selected row whose confirmation is open changes
/// nothing and stays silent, so a double click opens it once. After a done
/// deal, a click on the row opens it again.
#[expect(
    clippy::too_many_arguments,
    reason = "the row, the state the deal reads and the cue it plays"
)]
fn inspect_row(
    click: On<Pointer<Click>>,
    rows: Query<&CargoRow>,
    fixture: Res<SketchFixture>,
    context: Res<SketchContext>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut inspected: ResMut<Inspected>,
    mut draft: ResMut<Draft>,
) {
    let Ok(row) = rows.get(click.entity) else {
        return;
    };
    let picked = Some((row.store, row.good));
    let opened = open_draft(&fixture, *context, row.store, row.good);
    let deal_of = |draft: Option<DealDraft>| draft.map(|open| (open.deal, open.good));
    let reselected = inspected.0 == picked;
    let same_deal = deal_of(draft.0) == deal_of(opened);
    if !reselected {
        inspected.0 = picked;
    }
    if !same_deal {
        draft.0 = opened;
    }
    if !reselected || !same_deal {
        play_cue(
            &mut commands,
            bank.as_deref(),
            UiSfx::MenuSelect,
            MENU_SELECT_VOLUME,
        );
    }
}

/// A column of inspector content.
fn column_node(row_gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(row_gap),
        flex_shrink: 0.0,
        ..default()
    }
}

/// The inspector, built once: a hint with nothing selected; the item's icon,
/// name and category; its facts; and the confirmation in their place while
/// one is open. [`update_inspector`] fills it.
fn inspector(split: &mut ChildSpawnerCommands, icons: &SketchIcons, narrow: bool) {
    split
        .spawn((
            Name::new(INSPECTOR),
            side_panel(narrow, INSPECTOR_NARROW_PX),
        ))
        .with_children(|panel| {
            panel
                .spawn((InspectorPart::Hint, column_node(10.0)))
                .with_children(|hint| {
                    text(hint, "Click an item to inspect it.", 13.0, UiColor::Body);
                    hint.spawn((
                        Name::new(INSPECT_HINT),
                        themed_text("", 12.0, UiColor::Label),
                    ));
                });
            panel
                .spawn((InspectorPart::Item, column_node(10.0)))
                .with_children(|item| {
                    item.spawn(control_row(JustifyContent::FlexStart))
                        .with_children(|head| {
                            head.spawn(icon_frame(56.0)).with_children(|frame| {
                                frame.spawn((
                                    Name::new(INSPECT_ICON),
                                    icon_node(
                                        icons.cargo[Category::Raw.index()].clone(),
                                        Category::Raw.color(),
                                        40.0,
                                    ),
                                ));
                            });
                            head.spawn(column_node(4.0)).with_children(|names| {
                                names.spawn((
                                    Name::new(INSPECT_NAME),
                                    themed_text("", 16.0, UiColor::Primary),
                                ));
                                names.spawn((
                                    Name::new(INSPECT_CATEGORY),
                                    themed_text("", 12.0, UiColor::Body),
                                ));
                            });
                        });
                    item.spawn((InspectorPart::Facts, column_node(6.0)))
                        .with_children(|facts| {
                            facts.spawn((
                                Name::new(INSPECT_ABOUT),
                                themed_text("", 12.0, UiColor::Body),
                            ));
                            for (label, name) in [
                                ("Unit mass", INSPECT_MASS),
                                ("Stock", INSPECT_STOCK),
                                ("Price", INSPECT_PRICE),
                            ] {
                                fact(facts, label, name);
                            }
                        });
                    item.spawn((InspectorPart::Form, column_node(10.0)))
                        .with_children(deal_form);
                });
        });
}

/// A label on the left and a named value on the right.
fn fact(panel: &mut ChildSpawnerCommands, label: &str, name: &str) {
    panel
        .spawn(control_row(JustifyContent::SpaceBetween))
        .with_children(|row| {
            text(row, label, 12.0, UiColor::Label);
            row.spawn((
                Name::new(name.to_string()),
                themed_text("", 13.0, UiColor::Body),
                TextLayout::new(Justify::Right, LineBreak::NoWrap),
            ));
        });
}

/// The confirmation: the deal; the typed quantity, its amount and All on a
/// row the wheel steps; the quantity slider; the price; the hold after the
/// deal; and Confirm.
fn deal_form(form: &mut ChildSpawnerCommands) {
    form.spawn((
        Name::new(DEAL_TITLE),
        themed_text("", 13.0, UiColor::Accent),
    ));
    form.spawn((
        Name::new(DEAL_ROW),
        DraftWheel,
        Hovered::default(),
        control_row(JustifyContent::FlexStart),
    ))
    .observe(wheel_draft)
    .with_children(|row| {
        row.spawn(Node {
            width: px(72),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|cell| {
            cell.spawn((
                Name::new(DEAL_FIELD),
                DraftField,
                text_field(TextFieldSpec::new("1").max_chars(5).dense()),
            ));
        });
        row.spawn((
            Name::new(DEAL_QTY),
            themed_text("", 15.0, UiColor::Primary),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ));
        compact_button(row, ButtonSpec::new("All").fit().ghost(), DEAL_ALL)
            .insert(SketchClick)
            .observe(fill_draft);
    });
    form.spawn((
        Name::new(DEAL_SLIDER),
        DraftSlider,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(1.0),
        SliderRange::new(0.0, 1.0),
        SliderStep(1.0),
        SliderPrecision(0),
        slider_track(0.0),
    ))
    .observe(slide_draft);
    form.spawn((Name::new(DEAL_TOTAL), themed_text("", 13.0, UiColor::Body)));
    form.spawn((Name::new(DEAL_HOLD), themed_text("", 12.0, UiColor::Label)));
    form.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            compact_button(
                row,
                ButtonSpec::new("Confirm").fit().primary(),
                DEAL_CONFIRM,
            )
            .observe(confirm_draft);
        });
}

/// The confirmation a click on `good` in `store` opens at one unit: the
/// context's deal on that store, while the store still holds the item.
fn open_draft(
    fixture: &SketchFixture,
    context: SketchContext,
    store: Store,
    good: &'static Good,
) -> Option<DealDraft> {
    Deal::offered(context, store)
        .filter(|_| fixture.stock(store).qty(good) > 0)
        .map(|deal| DealDraft {
            deal,
            good,
            qty: Some(1),
        })
}

/// Fill the inspector in place: which parts show, the selected item and the
/// open confirmation. Writes only what differs, so a click or a quantity
/// change respawns nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "the inspector's nodes and every state they show"
)]
fn update_inspector(
    fixture: Res<SketchFixture>,
    context: Res<SketchContext>,
    inspected: Res<Inspected>,
    draft: Res<Draft>,
    icons: Res<SketchIcons>,
    mut parts: Query<(&InspectorPart, &mut Node)>,
    mut texts: Query<(&Name, &mut Text, &mut ThemedText)>,
    mut images: Query<(&Name, &mut ImageNode, &mut ThemedImageTint)>,
) {
    for (part, mut node) in &mut parts {
        let shown = match part {
            InspectorPart::Hint => inspected.0.is_none(),
            InspectorPart::Item => inspected.0.is_some(),
            InspectorPart::Facts => inspected.0.is_some(),
            InspectorPart::Form => draft.0.is_some(),
        };
        show(&mut node, shown);
    }
    let hint = match *context {
        SketchContext::Undocked => "Dock at a station or board a ship to move cargo.",
        SketchContext::Station => "Buy from the market or sell from your hold.",
        SketchContext::Boarded => "Loot from the raider's hold into yours.",
    };
    set_themed_text(&mut texts, INSPECT_HINT, hint, UiColor::Label);
    let Some((store, good)) = inspected.0 else {
        return;
    };
    let tone = good.category.color();
    for (name, mut image, mut tint) in &mut images {
        if name.as_str() != INSPECT_ICON {
            continue;
        }
        let wanted = &icons.cargo[good.category.index()];
        if image.image != *wanted {
            image.image = wanted.clone();
        }
        if tint.color != tone {
            tint.color = tone;
        }
    }
    set_themed_text(&mut texts, INSPECT_NAME, good.label, UiColor::Primary);
    set_themed_text(&mut texts, INSPECT_CATEGORY, good.category.label(), tone);
    set_themed_text(&mut texts, INSPECT_ABOUT, good.about, UiColor::Body);
    set_themed_text(
        &mut texts,
        INSPECT_MASS,
        &unit_mass(good.unit_kg),
        UiColor::Body,
    );
    set_themed_text(
        &mut texts,
        INSPECT_STOCK,
        &format!(
            "{} in {}",
            good.amount(fixture.stock(store).qty(good)),
            store.title()
        ),
        UiColor::Body,
    );
    let price = match (*context, store) {
        (SketchContext::Station, _) => format!("{} cr  station", good.price_cr),
        (SketchContext::Boarded, Store::Boarded) => "Free  loot".to_string(),
        _ => "No price".to_string(),
    };
    set_themed_text(&mut texts, INSPECT_PRICE, &price, UiColor::Body);
    let Some(DealDraft { deal, good, qty }) = draft.0 else {
        return;
    };
    set_themed_text(&mut texts, DEAL_TITLE, &deal.title(), UiColor::Accent);
    let (amount, summary, tone) = match qty {
        Some(qty) => (good.amount(qty), deal.summary(good, qty), UiColor::Body),
        None => (
            String::new(),
            "Type a whole number".to_string(),
            UiColor::Danger,
        ),
    };
    set_themed_text(&mut texts, DEAL_QTY, &amount, UiColor::Primary);
    set_themed_text(&mut texts, DEAL_TOTAL, &summary, tone);
    let moved = qty.unwrap_or(0) * good.unit_kg;
    let carried = fixture.own.stock.mass_kg();
    let after = if deal.stores().1 == Store::Own {
        carried + moved
    } else {
        carried.saturating_sub(moved)
    };
    set_themed_text(
        &mut texts,
        DEAL_HOLD,
        &format!(
            "{} after: {} / {}",
            Store::Own.title(),
            tonnes(after),
            tonnes(fixture.own.capacity_kg)
        ),
        if after > fixture.own.capacity_kg {
            UiColor::Danger
        } else {
            UiColor::Label
        },
    );
}

/// Show or hide a node, writing only a change.
fn show(node: &mut Mut<Node>, shown: bool) {
    let display = if shown { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
}

/// Marks a control whose every activation clicks. Controls with an outcome,
/// the rows, Confirm and Repair, play their own cue instead, so a refusal
/// sounds only its buzz.
#[derive(Component)]
struct SketchClick;

/// Click once for each activation of a [`SketchClick`] control.
fn click_cue(
    activate: On<Activate>,
    clicks: Query<(), With<SketchClick>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if clicks.contains(activate.entity) {
        play_cue(
            &mut commands,
            bank.as_deref(),
            UiSfx::MenuSelect,
            MENU_SELECT_VOLUME,
        );
    }
}

/// Play one UI cue, if the sound bank has loaded.
fn play_cue(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>, cue: UiSfx, volume: f32) {
    if let Some(bank) = bank {
        commands.play_sfx(bank.get(cue), AudioRoute::Interface, volume);
    }
}

/// Set the open confirmation's quantity to `qty`, or do nothing if it
/// already holds it. A new number ticks; text that is not a number is silent
/// until Confirm refuses it. Every quantity control but All goes through
/// here, so one change makes at most one tick.
fn set_draft_qty(
    draft: &mut Draft,
    qty: Option<u32>,
    commands: &mut Commands,
    bank: Option<&SoundBank<UiSfx>>,
) {
    let Some(open) = draft.0 else {
        return;
    };
    if open.qty == qty {
        return;
    }
    draft.0 = Some(DealDraft { qty, ..open });
    if qty.is_some() {
        play_cue(commands, bank, UiSfx::UiTick, UI_TICK_VOLUME);
    }
}

/// What the source store of the open confirmation holds, and never less
/// than one, so the wheel's range is never empty.
fn draft_stock(fixture: &SketchFixture, open: DealDraft) -> u32 {
    fixture.stock(open.deal.stores().0).qty(open.good).max(1)
}

/// Step the open confirmation's quantity by one per wheel event over the
/// quantity row, between one and what the source store holds.
fn wheel_draft(
    scroll: On<Pointer<Scroll>>,
    fixture: Res<SketchFixture>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut draft: ResMut<Draft>,
) {
    let Some(open) = draft.0 else {
        return;
    };
    if scroll.y == 0.0 {
        return;
    }
    let step = if scroll.y > 0.0 { 1 } else { -1 };
    let have = i64::from(draft_stock(&fixture, open));
    let qty = (i64::from(open.qty.unwrap_or(0)) + step).clamp(1, have) as u32;
    set_draft_qty(&mut draft, Some(qty), &mut commands, bank.as_deref());
}

/// Take the slider's value as the open confirmation's quantity. An empty
/// track is zero, which Confirm refuses. A drag reports every frame it
/// moves; only a new whole quantity ticks.
fn slide_draft(
    change: On<ValueChange<f32>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut draft: ResMut<Draft>,
) {
    let qty = change.value.round().max(0.0) as u32;
    set_draft_qty(&mut draft, Some(qty), &mut commands, bank.as_deref());
}

/// Take the typed text as the open confirmation's quantity: a whole number,
/// or `None`, which Confirm refuses.
fn type_draft(
    fields: Query<&TextFieldValue, (With<DraftField>, Changed<TextFieldValue>)>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut draft: ResMut<Draft>,
) {
    for value in &fields {
        let typed = value.trim().parse::<u32>().ok();
        set_draft_qty(&mut draft, typed, &mut commands, bank.as_deref());
    }
}

/// Show the open confirmation's quantity on the slider and the field, writing
/// only a difference, so the nodes stay and a drag does not fight itself.
/// The slider runs from zero, an empty track, to the whole stock, and hides
/// while the source holds one unit or none, where it could not move; the
/// field and All still set the quantity. While the field holds text that is
/// not a number, the slider keeps the last number rather than read as zero,
/// and the field is marked as an error. The field is rewritten only when the
/// draft holds a number its text does not read as: text that is not a number
/// stays as typed. A rewrite of the focused field puts the caret at the end,
/// since the old caret may sit past the new text. With no confirmation open
/// the field lets go of the keyboard, so a hidden field never holds it.
fn sync_draft_controls(
    mut commands: Commands,
    fixture: Res<SketchFixture>,
    draft: Res<Draft>,
    mut sliders: Query<(Entity, &SliderValue, &SliderRange, &mut Node), With<DraftSlider>>,
    mut fields: Query<
        (
            Entity,
            &mut TextFieldValue,
            Has<TextFieldFocused>,
            Has<TextFieldError>,
        ),
        With<DraftField>,
    >,
) {
    let Some(open) = draft.0 else {
        for (field, _, focused, _) in &fields {
            if focused {
                commands.entity(field).remove::<TextFieldFocused>();
            }
        }
        return;
    };
    let have = draft_stock(&fixture, open);
    for (slider, value, range, mut node) in &mut sliders {
        show(&mut node, have > 1);
        let wanted = SliderRange::new(0.0, have as f32);
        if *range != wanted {
            commands.entity(slider).insert(wanted);
        }
        if let Some(qty) = open.qty {
            let shown = qty.min(have) as f32;
            if value.0 != shown {
                commands.entity(slider).insert(SliderValue(shown));
            }
        }
    }
    for (field, mut value, focused, marked) in &mut fields {
        let Some(qty) = open.qty else {
            if !marked {
                commands.entity(field).insert(TextFieldError(String::new()));
            }
            continue;
        };
        if marked {
            commands.entity(field).remove::<TextFieldError>();
        }
        if value.trim().parse::<u32>().ok() == Some(qty) {
            continue;
        }
        value.0 = qty.to_string();
        if focused {
            commands
                .entity(field)
                .insert(TextFieldFocused::at_end(&value.0));
        }
    }
}

/// Set the open confirmation to everything the source store holds.
fn fill_draft(_: On<Activate>, fixture: Res<SketchFixture>, mut draft: ResMut<Draft>) {
    let Some(open) = draft.0 else {
        return;
    };
    draft.0 = Some(DealDraft {
        qty: Some(draft_stock(&fixture, open)),
        ..open
    });
}

/// Run the confirmed deal and close the confirmation. A refusal keeps it
/// open so the quantity can change. With none open, as on the second click
/// of a double click, nothing happens.
fn confirm_draft(
    _: On<Activate>,
    context: Res<SketchContext>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut fixture: ResMut<SketchFixture>,
    mut draft: ResMut<Draft>,
) {
    let Some(open) = draft.0 else {
        return;
    };
    let (notice, cue, volume) = match fixture.transfer(*context, open) {
        Ok(result) => {
            draft.0 = None;
            (result, UiSfx::MenuSelect, MENU_SELECT_VOLUME)
        }
        Err(refusal) => (refusal, UiSfx::EditorDeny, EDITOR_DENY_VOLUME),
    };
    fixture.notice = notice;
    play_cue(&mut commands, bank.as_deref(), cue, volume);
}

fn repair_selected(
    _: On<Activate>,
    context: Res<SketchContext>,
    selection: Res<ShipSelection>,
    sections: ShipSections,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut fixture: ResMut<SketchFixture>,
) {
    let Some(code) = selection.0.and_then(|selected| {
        sections
            .collect()
            .into_iter()
            .find(|view| view.entity == selected)
            .map(|view| view.code)
    }) else {
        return;
    };
    let (notice, cue, volume) = match fixture.repair(*context, &code) {
        Ok(result) => (result, UiSfx::MenuSelect, MENU_SELECT_VOLUME),
        Err(refusal) => (refusal, UiSfx::EditorDeny, EDITOR_DENY_VOLUME),
    };
    fixture.notice = notice;
    play_cue(&mut commands, bank.as_deref(), cue, volume);
}

fn toggle_repair_bay(_: On<Activate>, mut fixture: ResMut<SketchFixture>) {
    fixture.repair_bay = !fixture.repair_bay;
    let state = if fixture.repair_bay { "on" } else { "off" };
    fixture.notice = format!("Repair bay {state} (fixture)");
}

// Harness.

/// Window sizes the walk checks, in logical px: desktop, a mid width just
/// over the narrow break, narrow, and a short narrow window that scrolls.
#[cfg(feature = "debug")]
const DESKTOP: Vec2 = Vec2::new(1600.0, 900.0);
#[cfg(feature = "debug")]
const MID: Vec2 = Vec2::new(1120.0, 820.0);
#[cfg(feature = "debug")]
const NARROW: Vec2 = Vec2::new(720.0, 1000.0);
#[cfg(feature = "debug")]
const SHORT: Vec2 = Vec2::new(720.0, 760.0);

/// Seconds the harness gives the assets and the scenario to come up.
#[cfg(feature = "debug")]
const LOAD_DEADLINE: f32 = 90.0;

/// The raider's and the far rock's map codes, and a section the walk selects
/// on the picket.
#[cfg(feature = "debug")]
const RAIDER_CODE: &str = "HOST-1";
#[cfg(feature = "debug")]
const ROCK_CODE: &str = "AST-2";
#[cfg(feature = "debug")]
const PICKED_SECTION: &str = "PDC-1";

/// How close an eased orbit center must come to its target, in world units.
#[cfg(feature = "debug")]
const CENTERED: f32 = 0.05;

/// The first rect seen for each key: the pane at a window size, and each
/// scene at a window size. Every later view, context and repair state must
/// lay it out the same.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct SteadyRects(Vec<(String, Rect)>);

/// What the walk has seen of the simulation clocks since the screens first
/// held them.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct SimAudit {
    /// Set on the first frame [`hold_sketch_clocks`] has run.
    armed: bool,
    /// Fixed steps run since then. The pause allows none.
    fixed_steps: u32,
    /// Frames that started with a clock running or a virtual delta.
    live_frames: Vec<String>,
    /// Frames in which the player ship could be flown.
    flown_frames: Vec<String>,
}

#[cfg(feature = "debug")]
fn arm_sim_audit(mut audit: ResMut<SimAudit>) {
    if !audit.armed {
        audit.armed = true;
    }
}

#[cfg(feature = "debug")]
fn count_fixed_steps(mut audit: ResMut<SimAudit>) {
    if audit.armed {
        audit.fixed_steps += 1;
    }
}

/// Record a frame that begins with either clock running. `PreUpdate` reads
/// the clocks after `First` has advanced them, so a release that slipped
/// past [`hold_sketch_clocks`] shows here as a delta.
#[cfg(feature = "debug")]
fn watch_sim_clocks(
    mut audit: ResMut<SimAudit>,
    frame: Res<bevy::diagnostic::FrameCount>,
    virtual_time: Res<Time<Virtual>>,
    physics_time: Res<Time<Physics>>,
) {
    if !audit.armed {
        return;
    }
    if !virtual_time.is_paused() || virtual_time.delta_secs() > 0.0 || !physics_time.is_paused() {
        audit.live_frames.push(format!(
            "frame {}: virtual paused {} dt {}, physics paused {}",
            frame.0,
            virtual_time.is_paused(),
            virtual_time.delta_secs(),
            physics_time.is_paused()
        ));
    }
}

/// Record a frame in which the player ship could be flown: the flight
/// context up, or a burn, RCS, thruster or turret intent held. The screens
/// own the input, so none may occur. `PostUpdate` sees this frame's
/// `PreUpdate` context sync and every intent written up to `Update`.
#[cfg(feature = "debug")]
fn watch_player_input(
    mut audit: ResMut<SimAudit>,
    frame: Res<bevy::diagnostic::FrameCount>,
    contexts: Res<ActiveContexts>,
    ships: Query<(Entity, &FlightIntent, Option<&RcsIntent>), With<PlayerSpaceshipMarker>>,
    thrusters: Query<(&ChildOf, &ThrusterSectionInput)>,
    turrets: Query<(&ChildOf, &TurretSectionInput)>,
) {
    if !audit.armed {
        return;
    }
    let Ok((player, intent, rcs)) = ships.single() else {
        return;
    };
    let thrust = thrusters
        .iter()
        .any(|(parent, input)| parent.parent() == player && **input != 0.0);
    let fire = turrets
        .iter()
        .any(|(parent, input)| parent.parent() == player && **input);
    let flight = contexts.is_live(ActionContext::Flight);
    let rcs = rcs.is_some_and(|rcs| rcs.0 != Vec3::ZERO);
    if flight || intent.burn != 0.0 || rcs || thrust || fire {
        audit.flown_frames.push(format!(
            "frame {}: flight context {flight}, burn {}, rcs {rcs}, thrust {thrust}, fire {fire}",
            frame.0, intent.burn
        ));
    }
}

/// The screens hold the player's control, and no frame since they first held
/// the clocks could have flown the ship or fired its weapons.
#[cfg(feature = "debug")]
fn assert_ship_unflown(world: &mut World, when: &str) {
    assert!(
        world.resource::<PlayerControlSuspended>().is_suspended(),
        "the screens must hold the player's control ({when})"
    );
    let audit = world.resource::<SimAudit>();
    assert!(
        audit.flown_frames.is_empty(),
        "input reached the player ship under the screens ({when}): {:#?}",
        audit.flown_frames
    );
}

/// The clocks and every contact's pose when the walk first saw the scene.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct SimSnapshot {
    virtual_elapsed: std::time::Duration,
    physics_elapsed: std::time::Duration,
    bodies: Vec<(String, Vec3, Quat)>,
}

#[cfg(feature = "debug")]
fn sim_snapshot(world: &mut World) -> SimSnapshot {
    let mut bodies: Vec<(String, Vec3, Quat)> = world
        .query::<(&MapContactCode, &GlobalTransform)>()
        .iter(world)
        .map(|(code, transform)| {
            let (_, rotation, translation) = transform.to_scale_rotation_translation();
            (code.0.clone(), translation, rotation)
        })
        .collect();
    bodies.sort_by(|a, b| a.0.cmp(&b.0));
    SimSnapshot {
        virtual_elapsed: world.resource::<Time<Virtual>>().elapsed(),
        physics_elapsed: world.resource::<Time<Physics>>().elapsed(),
        bodies,
    }
}

/// Both clocks are stopped, no frame has advanced them since the screens
/// took them, and nothing in the scene has moved since the snapshot.
#[cfg(feature = "debug")]
fn assert_sim_still(world: &mut World, when: &str) {
    assert!(
        world.resource::<Time<Virtual>>().is_paused()
            && world.resource::<Time<Physics>>().is_paused(),
        "the simulation clocks must stay paused under the screens ({when})"
    );
    let audit = world.resource::<SimAudit>();
    assert!(audit.armed, "the screens never held the clocks ({when})");
    assert!(
        audit.fixed_steps == 0 && audit.live_frames.is_empty(),
        "the simulation advanced under the screens ({when}): {} fixed step(s), live frames {:#?}",
        audit.fixed_steps,
        audit.live_frames
    );
    let Some(before) = world.remove_resource::<SimSnapshot>() else {
        return;
    };
    let now = sim_snapshot(world);
    assert_eq!(
        (now.virtual_elapsed, now.physics_elapsed),
        (before.virtual_elapsed, before.physics_elapsed),
        "the game clocks moved ({when})"
    );
    assert_eq!(
        now.bodies, before.bodies,
        "a ship or rock moved under the paused screens ({when})"
    );
    world.insert_resource(before);
}

#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// The driven walk. Every view, selection, context and theme change is a real
/// click on the named control, every orbit a real drag or wheel on a pane,
/// every cargo move a real row click, quantity gesture and Confirm, and every
/// verdict reads the live world.
#[cfg(feature = "debug")]
fn sketch_script() -> Script {
    use SketchContext::{Boarded, Station, Undocked};
    use SketchView::{Inventory, Map, Ship};

    let mut script = Script::new()
        .step("sketch: the map is up")
        .until(and(
            state_is(GameStates::Playing),
            ui_node_present(PANE_MAP),
        ))
        .diagnose(ui_node_diagnosis(PANE_MAP))
        .deadline(LOAD_DEADLINE)
        .add();
    script = resize(script, DESKTOP, "desktop");
    script = script
        .step("sketch: the scenario is built and every contact is on the map")
        .until(and(
            and(
                resource_where::<ScenarioLoadGate>(|gate| !gate.is_held()),
                and(
                    ui_node_present("Map Blip AST-1"),
                    ui_node_present(format!("Map Blip {ROCK_CODE}")),
                ),
            ),
            and(
                ui_node_present("Map Blip ALLY-1"),
                ui_node_present(format!("Map Blip {RAIDER_CODE}")),
            ),
        ))
        .diagnose(|world: &World| {
            let Some(mut blips) =
                world.try_query_filtered::<(&Name, &Node, &InheritedVisibility), With<MapBlip>>()
            else {
                return "no map blip has spawned".to_string();
            };
            let seen: Vec<String> = blips
                .iter(world)
                .map(|(name, node, shown)| {
                    format!(
                        "{name} at {:?},{:?} shown {}",
                        node.left,
                        node.top,
                        shown.get()
                    )
                })
                .collect();
            format!("map blips: {seen:?}")
        })
        .deadline(LOAD_DEADLINE)
        .add()
        .step("sketch: record the paused scene")
        .on_enter(|world: &mut World| {
            let snapshot = sim_snapshot(world);
            info!(
                "sketch: {} bodies frozen at virtual {:?}",
                snapshot.bodies.len(),
                snapshot.virtual_elapsed
            );
            world.insert_resource(snapshot);
        })
        .add();
    script = verdict(script, Map, Undocked, "desktop");
    let raider = format!("Map Blip {RAIDER_CODE}");
    script = watch_ease(script, Map)
        .step("sketch: forget the cues before the map")
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add();
    script = script
        .click_named(
            "sketch: select the raider",
            &raider,
            ui_node_present(raider.clone()),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: the readout names the raider")
        .until(frames(3))
        .add()
        .step("sketch: the map selected the raider")
        .on_enter(|world: &mut World| {
            let line = named_text(world, MAP_READOUT);
            assert!(
                line.starts_with(RAIDER_CODE) && line.contains("HOSTILE"),
                "clicking the raider must show it in the readout, not `{line}`"
            );
            info!("sketch: map readout is `{line}`");
        })
        .add();
    script = expect_cues(script, "a map contact", &[UiSfx::MenuSelect]);
    script = ease_onto(script, Map, RAIDER_CODE);
    script = shot(script, "phosphor-map");
    script = drag_pane(script, Map);
    script = wheel_pane(script, Map);
    script = script
        .click_named(
            "sketch: reframe the map",
            MAP_REFRAME,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: the map is back on its opening framing")
        .until(std::sync::Arc::new(|world: &World| {
            let (orbit, player) = (map_orbit(world), player_position(world));
            orbit.center.distance(player) < CENTERED
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sketch: the reframe restored the angles and zoom")
        .on_enter(|world: &mut World| {
            let orbit = map_orbit(world);
            let expected = expected_map_framing(world);
            assert_eq!(
                (orbit.theta, orbit.phi, orbit.radius),
                (MAP_THETA, MAP_PHI, expected),
                "Reframe must restore the opening orbit"
            );
            info!("sketch: reframed to radius {expected}");
        })
        .add();
    script = expect_cues(script, "Reframe", &[UiSfx::MenuSelect]);
    let rock = format!("Map Blip {ROCK_CODE}");
    script = watch_ease(script, Map);
    script = script.click_named(
        "sketch: select the far rock",
        &rock,
        resource_where::<MapSelection>(|selection| selection.0.is_some()),
        BEAT_DEADLINE_SECS,
    );
    script = ease_onto(script, Map, ROCK_CODE);
    script = expect_cues(script, "the far rock", &[UiSfx::MenuSelect]);
    script = shot(script, "phosphor-map-recentered");
    script = verdict(script, Map, Undocked, "recentered");
    script = fly_map(script);
    script = verdict(script, Map, Undocked, "flown");
    script = nova_os_round_trip(script);
    script = verdict(script, Map, Undocked, "after NOVA OS and Escape");
    // Every context over the live map: the scene and its camera stay, and
    // the map gains no dock, trade or repair control.
    for context in [Station, Boarded, Undocked] {
        script = switch_context(script, context);
        script = verdict(script, Map, context, "context on the map");
    }

    script = open(script, Ship);
    script = verdict(script, Ship, Undocked, "desktop");
    let section = format!("Ship Blip {PICKED_SECTION}");
    script = watch_ease(script, Ship);
    script = script
        .click_named(
            "sketch: select the point defence",
            &section,
            ui_node_present(section.clone()),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: the detail follows the section")
        .until(frames(3))
        .add()
        .step("sketch: the ship selected the point defence")
        .on_enter(|world: &mut World| {
            let detail = named_text(world, SHIP_DETAIL);
            assert!(
                detail.starts_with(PICKED_SECTION),
                "clicking {PICKED_SECTION} must show its detail, not `{detail}`"
            );
            let status = named_text(world, SHIP_STATUS);
            assert!(
                status.contains("Condition 45%  degraded"),
                "{PICKED_SECTION} must show its fixture wear, not `{status}`"
            );
            assert_selected_outline(world);
            info!("sketch: ship detail is `{detail}`");
        })
        .add();
    script = expect_cues(script, "a ship section", &[UiSfx::MenuSelect]);
    script = ease_onto(script, Ship, PICKED_SECTION);
    // Shot before the wheel beats, which leave the hull at the zoom ceiling.
    script = shot(script, "phosphor-ship");
    script = step_sections(script);
    script = drag_pane(script, Ship);
    script = wheel_pane(script, Ship);
    script = fit_and_reset_ship(script);
    script = verdict(script, Ship, Undocked, "orbited");

    script = open(script, Inventory);
    script = verdict(script, Inventory, Undocked, "desktop");
    script = inspect_and_filter(script);
    script = verdict(script, Inventory, Undocked, "inspected");
    script = shot(script, "phosphor-inventory");
    script = context_refusals(script, Undocked);

    script = switch_context(script, Station);
    script = verdict(script, Inventory, Station, "desktop");
    script = station_deals(script);
    script = verdict(script, Inventory, Station, "after deals");

    script = switch_context(script, Boarded);
    script = verdict(script, Inventory, Boarded, "desktop");
    script = boarded_loot(script);
    script = verdict(script, Inventory, Boarded, "after loot");
    script = shot(script, "phosphor-inventory-boarded");
    script = context_refusals(script, Boarded);

    script = open(script, Ship);
    script = verdict(script, Ship, Boarded, "desktop");
    script = switch_context(script, Station);
    script = verdict(script, Ship, Station, "desktop");
    script = ship_repairs(script);
    script = verdict(script, Ship, Station, "repaired");
    script = shot(script, "phosphor-ship-station");

    script = script
        .step("sketch: record the scene before the theme flip")
        .on_enter(|world: &mut World| {
            let probe = FlipProbe {
                root: scene_root(world, Ship),
                block: block_color(world),
            };
            world.insert_resource(probe);
        })
        .add();
    script = pick_theme(script, HARDWARE_THEME_ID, THEME_HARDWARE);
    script = script
        .step("sketch: the 3D scene repainted in place")
        .on_enter(|world: &mut World| {
            let (root, before) = {
                let probe = world.resource::<FlipProbe>();
                (probe.root, probe.block)
            };
            assert_eq!(
                scene_root(world, Ship),
                root,
                "a theme flip must repaint the scene, not rebuild it"
            );
            let after = block_color(world);
            assert_ne!(
                after, before,
                "the ship blocks must take the new theme (still {after:?})"
            );
            info!("sketch: 3D repainted in place ({before:?} -> {after:?})");
        })
        .add();
    script = verdict(script, Ship, Station, "hardware");
    script = shot(script, "hardware-ship-station");
    script = open(script, Map);
    script = verdict(script, Map, Station, "hardware");
    script = shot(script, "hardware-map-station");
    script = open(script, Inventory);
    script = verdict(script, Inventory, Station, "hardware");
    script = shot(script, "hardware-inventory-station");
    script = switch_context(script, Boarded);
    script = verdict(script, Inventory, Boarded, "hardware");
    script = shot(script, "hardware-inventory-boarded");
    script = open(script, Ship);
    script = verdict(script, Ship, Boarded, "hardware");
    script = shot(script, "hardware-ship-boarded");

    // Just over the narrow break: the side panels still sit beside their
    // views.
    script = resize(script, MID, "mid");
    script = verdict(script, Ship, Boarded, "mid");
    script = switch_context(script, Station);
    script = verdict(script, Ship, Station, "mid");
    script = shot(script, "mid-hardware-ship-station");
    for view in [Inventory, Map] {
        script = open(script, view);
        script = verdict(script, view, Station, "mid");
    }

    script = resize(script, NARROW, "narrow");
    for view in [Map, Ship, Inventory] {
        if view != Map {
            script = open(script, view);
        }
        script = verdict(script, view, Station, "narrow");
        script = shot(script, &format!("narrow-hardware-{}-station", view.slug()));
    }
    // Narrow, the deal still sits under the item's facts, and a short window
    // scrolls the body until its Confirm is on screen.
    script = select_row(
        script,
        "open the water narrow",
        Store::Market,
        "Water",
        Some(Deal::Buy),
    );
    script = verdict(script, Inventory, Station, "narrow deal");
    script = script
        .step("sketch: the narrow deal shows Confirm under the facts")
        .on_enter(|world: &mut World| {
            let window = window_rect(world);
            let inspector = ui_node_rect(world, INSPECTOR).expect("the inspector is laid out");
            let stock = ui_node_rect(world, INSPECT_STOCK).expect("the facts are laid out");
            let confirm = ui_node_rect(world, DEAL_CONFIRM).expect("the deal shows Confirm");
            assert!(
                stock.max.y <= confirm.min.y
                    && inside(confirm, inspector)
                    && inside(confirm, window),
                "narrow, Confirm {confirm:?} must sit under the facts {stock:?} inside the \
                 inspector {inspector:?} and the window {window:?}"
            );
            info!("sketch: narrow inspector {inspector:?}, Confirm {confirm:?}");
        })
        .add();
    script = shot(script, "narrow-hardware-inventory-deal");
    script = resize(script, SHORT, "short deal");
    script = script
        .step("sketch: aim the wheel at the context line (short deal)")
        .on_enter(|world: &mut World| {
            let banner = ui_node_centre(world, DOCK_BANNER).expect("the context line is laid out");
            move_cursor(banner)(world);
        })
        .until(frames(2))
        .add()
        .step("sketch: wheel down to the short deal")
        .on_enter(scroll_lines(-10.0))
        .until(frames(4))
        .add()
        .step("sketch: the short window scrolled Confirm into view")
        .on_enter(|world: &mut World| {
            let window = window_rect(world);
            let inspector = ui_node_rect(world, INSPECTOR).expect("the inspector is laid out");
            let confirm = ui_node_rect(world, DEAL_CONFIRM).expect("the deal shows Confirm");
            assert!(
                inside(confirm, inspector) && inside(confirm, window),
                "the wheel must scroll Confirm {confirm:?} into the {window:?} window, inside \
                 the inspector {inspector:?}"
            );
            info!("sketch: short inspector {inspector:?}, Confirm {confirm:?}");
        })
        .add();
    script = shot(script, "short-hardware-inventory-deal");
    script = resize(script, NARROW, "narrow after the short deal");
    script = switch_context(script, Boarded);
    script = verdict(script, Inventory, Boarded, "narrow");
    script = shot(script, "narrow-hardware-inventory-boarded");
    script = pick_theme(script, PHOSPHOR_THEME_ID, THEME_PHOSPHOR);
    script = verdict(script, Inventory, Boarded, "narrow phosphor");
    script = shot(script, "narrow-phosphor-inventory-boarded");
    script = open(script, Ship);
    script = verdict(script, Ship, Boarded, "narrow phosphor");
    script = shot(script, "narrow-phosphor-ship-boarded");
    script = switch_context(script, Undocked);
    script = verdict(script, Ship, Undocked, "narrow phosphor");
    script = shot(script, "narrow-phosphor-ship-undocked");
    script = open(script, Map);
    script = verdict(script, Map, Undocked, "narrow phosphor");
    script = shot(script, "narrow-phosphor-map-undocked");

    script = open(script, Ship);
    script = scroll_short(script);
    script = resize(script, NARROW, "narrow again");
    verdict(script, Ship, Undocked, "narrow after the short window")
}

/// Click the theme called `name` and let it settle.
#[cfg(feature = "debug")]
fn pick_theme(script: Script, id: &'static str, name: &str) -> Script {
    let script = script
        .step("sketch: forget the cues before the theme")
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add()
        .click_named(
            &format!("sketch: switch to {name}"),
            name,
            resource_where::<SelectedUiTheme>(move |theme| theme.0 == id),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: the theme settles")
        .until(frames(3))
        .add();
    expect_cues(script, "a theme switch", &[UiSfx::MenuSelect])
}

/// The live view as a context change must leave it: the body, the scene and
/// its camera, and the orbit.
#[cfg(feature = "debug")]
#[derive(Resource, PartialEq, Debug)]
struct ViewKept {
    body: Entity,
    scene: Option<(Entity, Entity)>,
    orbit: Option<(f32, f32, f32, Vec3)>,
}

#[cfg(feature = "debug")]
fn view_kept(world: &mut World) -> ViewKept {
    let body = world
        .query_filtered::<Entity, With<SketchBody>>()
        .single(world)
        .expect("one sketch body");
    let root = match *world.resource::<SketchView>() {
        SketchView::Map => world.resource::<MapScene>().0.as_ref().map(|s| s.root),
        SketchView::Ship => world.resource::<ShipScene>().0.as_ref().map(|s| s.root),
        SketchView::Inventory => None,
    };
    let camera = world
        .query_filtered::<(Entity, &SketchOrbit), Or<(With<MapCamera>, With<ShipCamera>)>>()
        .iter(world)
        .map(|(entity, orbit)| {
            (
                entity,
                (orbit.theta, orbit.phi, orbit.radius, orbit.center_target),
            )
        })
        .next();
    ViewKept {
        body,
        scene: root.zip(camera.map(|(entity, _)| entity)),
        orbit: camera.map(|(_, orbit)| orbit),
    }
}

/// Click the context called for and check it kept the live view: the same
/// body, scene, camera and orbit.
#[cfg(feature = "debug")]
fn switch_context(script: Script, context: SketchContext) -> Script {
    let (_, label, name) = SketchContext::ALL
        .into_iter()
        .find(|(value, _, _)| *value == context)
        .expect("every context has a control");
    let script = script
        .step(format!("sketch: note the view before {label}"))
        .on_enter(|world: &mut World| {
            let kept = view_kept(world);
            world.insert_resource(kept);
            world.resource_mut::<HeardCues>().0.clear();
        })
        .add()
        .click_named(
            &format!("sketch: switch to {label}"),
            name,
            resource_where::<SketchContext>(move |current| *current == context),
            BEAT_DEADLINE_SECS,
        )
        .step(format!("sketch: {label} settles"))
        .until(frames(3))
        .add()
        .step(format!("sketch: {label} kept the live view"))
        .on_enter(move |world: &mut World| {
            let before = world
                .remove_resource::<ViewKept>()
                .expect("the view was noted");
            let after = view_kept(world);
            assert_eq!(
                after, before,
                "switching to {label} must keep the body, the 3D scene, its camera and the orbit"
            );
        })
        .add();
    expect_cues(script, "a context switch", &[UiSfx::MenuSelect])
}

/// The fixture as a step noted it, for the step after an action to compare.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct FixtureBefore(SketchFixture);

/// Note the fixture, run `act`, and hand `check` the fixture from before and
/// after it.
#[cfg(feature = "debug")]
fn around_fixture(
    script: Script,
    what: &'static str,
    act: impl FnOnce(Script) -> Script,
    check: impl Fn(&SketchFixture, &SketchFixture) + Send + Sync + 'static,
) -> Script {
    let script = script
        .step(format!("sketch: note the fixture before {what}"))
        .on_enter(|world: &mut World| {
            let before = world.resource::<SketchFixture>().clone();
            world.insert_resource(FixtureBefore(before));
        })
        .add();
    act(script)
        .step(format!("sketch: the screens show {what}"))
        .until(frames(3))
        .add()
        .step(format!("sketch: {what} changed the fixture as expected"))
        .on_enter(move |world: &mut World| {
            let before = world
                .remove_resource::<FixtureBefore>()
                .expect("the fixture was noted")
                .0;
            let after = world.resource::<SketchFixture>().clone();
            check(&before, &after);
            assert_sim_still(world, what);
            assert_ship_unflown(world, what);
            info!("sketch: {what}: `{}`", after.notice);
        })
        .add()
}

/// Click the control called `button` between the fixture checks.
#[cfg(feature = "debug")]
fn transact(
    script: Script,
    what: &'static str,
    button: &str,
    check: impl Fn(&SketchFixture, &SketchFixture) + Send + Sync + 'static,
) -> Script {
    let button = button.to_string();
    around_fixture(
        script,
        what,
        move |script| {
            script.click_named(
                &format!("sketch: {what}"),
                &button,
                pointer_released(),
                BEAT_DEADLINE_SECS,
            )
        },
        check,
    )
}

/// Everything but the notice is as it was, and the notice is `reason`.
#[cfg(feature = "debug")]
fn assert_refused(before: &SketchFixture, after: &SketchFixture, reason: &str) {
    assert_eq!(after.notice, reason, "the refusal must name its reason");
    assert_unchanged(before, after, reason);
}

/// Everything but the notice is as it was.
#[cfg(feature = "debug")]
fn assert_unchanged(before: &SketchFixture, after: &SketchFixture, what: &str) {
    assert_eq!(
        SketchFixture {
            notice: String::new(),
            ..after.clone()
        },
        SketchFixture {
            notice: String::new(),
            ..before.clone()
        },
        "{what} must change no cargo, credits or condition"
    );
}

/// Units of `label` in `store`; none is zero.
#[cfg(feature = "debug")]
fn qty(fixture: &SketchFixture, store: Store, label: &str) -> u32 {
    fixture.stock(store).qty(good(label))
}

/// Outside its context, the fixture itself refuses every deal and a repair
/// with the reason, and changes nothing.
#[cfg(feature = "debug")]
fn context_refusals(script: Script, context: SketchContext) -> Script {
    script
        .step(format!(
            "sketch: {context:?}, the fixture refuses what needs another context"
        ))
        .on_enter(move |world: &mut World| {
            let live = world.resource::<SketchFixture>().clone();
            let mut probe = live.clone();
            for deal in Deal::ALL {
                if deal.context() == context {
                    continue;
                }
                let expected = match deal {
                    Deal::Buy | Deal::Sell => "Refused: not at a station",
                    Deal::Loot => "Refused: not boarded",
                };
                assert_eq!(
                    probe.transfer(
                        context,
                        DealDraft {
                            deal,
                            good: good("Slugs"),
                            qty: Some(1),
                        }
                    ),
                    Err(expected.to_string()),
                    "{deal:?} must be refused {context:?}"
                );
            }
            if context != SketchContext::Station {
                assert_eq!(
                    probe.repair(context, "THR-1"),
                    Err("Refused: repair needs a station".to_string())
                );
            }
            assert_eq!(probe, live, "a refusal must change nothing");
        })
        .add()
}

/// Click the `label` row in `store`: it selects the row and opens the
/// confirmation of `deal`, the deal the context allows on it, and moves
/// nothing.
#[cfg(feature = "debug")]
fn select_row(
    script: Script,
    what: &'static str,
    store: Store,
    label: &'static str,
    deal: Option<Deal>,
) -> Script {
    let row = store.row_name(good(label));
    around_fixture(
        script,
        what,
        move |script| {
            script.click_named(
                &format!("sketch: {what}"),
                &row,
                resource_where::<Inspected>(move |inspected| {
                    inspected.0 == Some((store, good(label)))
                }),
                BEAT_DEADLINE_SECS,
            )
        },
        move |before, after| assert_unchanged(before, after, what),
    )
    .step(format!("sketch: {what} opened {deal:?}"))
    .on_enter(move |world: &mut World| {
        assert_eq!(
            world
                .resource::<Draft>()
                .0
                .map(|open| (open.deal, open.good)),
            deal.map(|deal| (deal, good(label))),
            "{label} in {store:?} must open exactly {deal:?}"
        );
    })
    .add()
}

/// Every UI node under the inventory card and the context line, as a step
/// noted it.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct NodesBefore(Vec<Entity>);

/// Every UI node under the inventory card and the context line, sorted.
#[cfg(feature = "debug")]
fn inventory_nodes(world: &mut World) -> Vec<Entity> {
    let roots = [named(world, PANE_HOLD), named(world, DOCK_BANNER)];
    let mut nodes: Vec<Entity> = roots
        .into_iter()
        .flat_map(|root| descendants_with::<Node>(world, root))
        .collect();
    nodes.sort();
    nodes
}

/// Note the inventory and context-line nodes for [`assert_nodes_kept`].
#[cfg(feature = "debug")]
fn note_nodes(script: Script) -> Script {
    script
        .step("sketch: note the inventory nodes")
        .on_enter(|world: &mut World| {
            let nodes = inventory_nodes(world);
            world.insert_resource(NodesBefore(nodes));
        })
        .add()
}

/// The inventory and context-line nodes are the ones [`note_nodes`] saw:
/// `what` respawned no row, inspector part or context-line text.
#[cfg(feature = "debug")]
fn assert_nodes_kept(script: Script, what: &'static str) -> Script {
    script
        .step(format!("sketch: {what} kept every inventory node"))
        .on_enter(move |world: &mut World| {
            let before = world
                .remove_resource::<NodesBefore>()
                .expect("the nodes were noted")
                .0;
            let after = inventory_nodes(world);
            let gone = before.iter().filter(|node| !after.contains(node)).count();
            let new = after.iter().filter(|node| !before.contains(node)).count();
            assert!(
                gone == 0 && new == 0,
                "{what} must update the inventory and context line in place, \
                 not despawn {gone} and spawn {new} node(s)"
            );
            info!("sketch: {what} kept all {} inventory nodes", after.len());
        })
        .add()
}

/// The open confirmation is `deal` of `qty` `label` priced as `summary`, and
/// the inspector shows it.
#[cfg(feature = "debug")]
fn expect_draft(
    script: Script,
    deal: Deal,
    label: &'static str,
    qty: u32,
    summary: &'static str,
) -> Script {
    script
        .step(format!("sketch: the confirmation settles on {qty} {label}"))
        .until(frames(3))
        .add()
        .step(format!(
            "sketch: the confirmation reads {deal:?} {qty} {label}"
        ))
        .on_enter(move |world: &mut World| {
            let good = good(label);
            assert_eq!(
                world.resource::<Draft>().0,
                Some(DealDraft {
                    deal,
                    good,
                    qty: Some(qty)
                }),
                "the confirmation must hold {deal:?} of {qty} {label}"
            );
            assert_eq!(
                (
                    named_text(world, DEAL_TITLE),
                    named_text(world, DEAL_QTY),
                    named_text(world, DEAL_TOTAL),
                ),
                (deal.title(), good.amount(qty), summary.to_string()),
                "the confirmation must show the deal, quantity and price"
            );
            let field = world
                .query_filtered::<&TextFieldValue, With<DraftField>>()
                .single(world)
                .expect("one quantity field")
                .0
                .clone();
            let slider = world
                .query_filtered::<&SliderValue, With<DraftSlider>>()
                .single(world)
                .expect("one quantity slider")
                .0;
            assert_eq!(
                (field, slider),
                (qty.to_string(), qty as f32),
                "the quantity field and slider must show {qty}"
            );
        })
        .add()
}

/// Click All on the confirmation and wait for the quantity.
#[cfg(feature = "debug")]
fn set_draft(script: Script, what: &str, button: &str, qty: u32) -> Script {
    script.click_named(
        &format!("sketch: {what}"),
        button,
        resource_where::<Draft>(move |draft| draft.0.is_some_and(|open| open.qty == Some(qty))),
        BEAT_DEADLINE_SECS,
    )
}

/// Click the quantity field, clear it and type `text`, and wait for the
/// draft to read it: its number, or none for text that is not one. The
/// keys land in one frame, so the field changes once.
#[cfg(feature = "debug")]
fn type_qty(script: Script, what: &'static str, text: &'static str) -> Script {
    let typed = text.parse::<u32>().ok();
    script
        .click_named(
            &format!("sketch: {what}: focus the field"),
            DEAL_FIELD,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step(format!("sketch: {what}: the field takes the keyboard"))
        .until(frames(2))
        .add()
        .step(format!("sketch: {what}: type `{text}`"))
        .on_enter(move |world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..5 {
                press_edit_key(Key::Backspace)(world);
            }
            type_text(text)(world);
        })
        .until(resource_where::<Draft>(move |draft| {
            draft.0.is_some_and(|open| open.qty == typed)
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("sketch: {what}: the field shows `{text}`"))
        .until(frames(3))
        .add()
        .step(format!("sketch: {what}: the draft reads `{text}`"))
        .on_enter(move |world: &mut World| {
            let field = world
                .query_filtered::<&TextFieldValue, With<DraftField>>()
                .single(world)
                .expect("one quantity field")
                .0
                .clone();
            assert_eq!(
                (
                    field.as_str(),
                    world.resource::<Draft>().0.map(|open| open.qty)
                ),
                (text, Some(typed)),
                "typing `{text}` must stay in the field and set the draft to {typed:?}"
            );
        })
        .add()
}

/// Every cue played since [`expect_cues`] last drained it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeardCues(Vec<Handle<AudioSource>>);

#[cfg(feature = "debug")]
fn hear_cue(play: On<PlaySfx>, mut heard: ResMut<HeardCues>) {
    heard.0.push(play.handle.clone());
}

/// Drain the cues heard since the last drain: the interface cues among them
/// must be exactly `cues`, in order. A sound outside the UI bank is not the
/// screens' and is dropped. Each measured beat is followed by one of these,
/// so a drain holds that beat's cues alone; a walk clears what earlier views
/// played before its first beat.
#[cfg(feature = "debug")]
fn expect_cues(script: Script, what: &'static str, cues: &'static [UiSfx]) -> Script {
    script
        .step(format!("sketch: {what} sounds {cues:?}"))
        .on_enter(move |world: &mut World| {
            let heard = std::mem::take(&mut world.resource_mut::<HeardCues>().0);
            let bank = world
                .get_resource::<SoundBank<UiSfx>>()
                .expect("the UI sound bank is loaded");
            let ui: Vec<UiSfx> = heard
                .iter()
                .filter_map(|handle| {
                    UI_SFX_FILES
                        .iter()
                        .map(|(key, _)| *key)
                        .find(|key| bank.try_get(*key).as_ref() == Some(handle))
                })
                .collect();
            assert_eq!(ui, cues, "{what} must sound exactly {cues:?}");
        })
        .add()
}

/// Click the ore row and read the inspector, which offers no deal undocked
/// and respawns no node, then filter to ammo and back.
#[cfg(feature = "debug")]
fn inspect_and_filter(script: Script) -> Script {
    let script = note_nodes(script)
        .step("sketch: forget the cues before the ore")
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add();
    let script = select_row(script, "select own ore", Store::Own, "Nickel ore", None);
    let script = expect_cues(script, "selecting the ore undocked", &[UiSfx::MenuSelect]);
    let script = assert_nodes_kept(script, "selecting the ore");
    let script = script
        .step("sketch: the inspector shows the ore")
        .on_enter(|world: &mut World| {
            assert_eq!(
                [
                    INSPECT_NAME,
                    INSPECT_CATEGORY,
                    INSPECT_MASS,
                    INSPECT_STOCK,
                    INSPECT_PRICE
                ]
                .map(|name| named_text(world, name)),
                [
                    "Nickel ore",
                    "Raw",
                    "1.0 t",
                    "4 t in Picket hold",
                    "No price"
                ],
                "undocked, the inspector must show the ore without a price"
            );
        })
        .add()
        .click_named(
            "sketch: filter to ammo",
            &Category::Ammo.filter_name(),
            resource_where::<CargoFilter>(|filter| filter.0 == Some(Category::Ammo)),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: the filter settles")
        .until(frames(3))
        .add();
    let script = expect_cues(script, "the ammo filter", &[UiSfx::MenuSelect]);
    let script = script
        .step("sketch: only the slugs show")
        .on_enter(|world: &mut World| {
            assert_eq!(
                cargo_rows(world),
                vec![(Store::Own, "Slugs")],
                "the ammo filter must show only the slugs"
            );
            assert_view(
                world,
                SketchView::Inventory,
                SketchContext::Undocked,
                "ammo filter",
            );
        })
        .add()
        .click_named(
            "sketch: show all cargo",
            FILTER_ALL,
            resource_where::<CargoFilter>(|filter| filter.0.is_none()),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: all cargo settles")
        .until(frames(3))
        .add();
    expect_cues(script, "the All filter", &[UiSfx::MenuSelect])
}

/// The rows on screen, by store and good, sorted.
#[cfg(feature = "debug")]
fn cargo_rows(world: &mut World) -> Vec<(Store, &'static str)> {
    let mut rows: Vec<(Store, &'static str)> = world
        .query::<&CargoRow>()
        .iter(world)
        .map(|row| (row.store, row.good.label))
        .collect();
    rows.sort_by_key(|(store, label)| (store.tag(), *label));
    rows
}

/// At the station: double-click the market water, which opens its
/// confirmation once. Wheel it past the hold limit and Confirm, then slide
/// it, type an empty, a zero and an overstock quantity and Confirm each,
/// then type one tonne and double-click Confirm, which buys it once. Sell
/// all the ore, and try to buy both pumps short of credits. Only Confirm
/// moves cargo, every refusal names its reason and changes nothing, every
/// control sounds its cue once, and none of it respawns a node.
#[cfg(feature = "debug")]
fn station_deals(mut script: Script) -> Script {
    let water = Store::Market.row_name(good("Water"));
    script = script
        .step("sketch: forget the cues before the deals")
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add();
    script = note_nodes(script);
    script = around_fixture(
        script,
        "double-click market water",
        move |script| {
            script.double_click_named(
                "sketch: double-click market water",
                &water,
                BEAT_DEADLINE_SECS,
            )
        },
        |before, after| assert_unchanged(before, after, "double-clicking water"),
    );
    script = expect_draft(script, Deal::Buy, "Water", 1, "10 cr each  Total -10 cr");
    script = expect_cues(script, "double-clicking a row", &[UiSfx::MenuSelect]);
    script = script
        .step("sketch: one tonne of six fills a sixth of the slider")
        .on_enter(|world: &mut World| {
            let (value, range) = world
                .query_filtered::<(&SliderValue, &SliderRange), With<DraftSlider>>()
                .single(world)
                .map(|(value, range)| (value.0, *range))
                .expect("one quantity slider");
            assert!(
                range == SliderRange::new(0.0, 6.0)
                    && (range.thumb_position(value) - 1.0 / 6.0).abs() < 1e-4,
                "one tonne must fill 1/6 of a 0..=6 track, not {value} on {range:?}"
            );
        })
        .add();
    script = script
        .step("sketch: point at the quantity row")
        .on_enter(hover_named(DEAL_ROW))
        .until(pointer_over_node(DEAL_ROW))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sketch: wheel the quantity up one")
        .on_enter(scroll_lines(1.0))
        .until(resource_where::<Draft>(|draft| {
            draft.0.is_some_and(|open| open.qty == Some(2))
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add();
    script = expect_draft(script, Deal::Buy, "Water", 2, "10 cr each  Total -20 cr");
    script = expect_cues(script, "the wheel", &[UiSfx::UiTick]);
    script = script
        .step("sketch: the wheel left the body where it was")
        .on_enter(|world: &mut World| {
            let scrolled = world
                .query_filtered::<&ScrollPosition, With<SketchBody>>()
                .single(world)
                .expect("one sketch body")
                .0
                .y;
            assert_eq!(scrolled, 0.0, "the wheel over the quantity must not scroll");
        })
        .add()
        .step("sketch: the confirmation shows the hold overfilled")
        .on_enter(|world: &mut World| {
            assert_eq!(
                named_text(world, DEAL_HOLD),
                "Picket hold after: 12.5 t / 12.0 t"
            );
        })
        .add();
    script = transact(
        script,
        "confirm two tonnes past the hold limit",
        DEAL_CONFIRM,
        |before, after| {
            assert_refused(before, after, "Refused: Picket hold full");
        },
    );
    script = expect_cues(script, "a refused Confirm", &[UiSfx::EditorDeny]);
    script = expect_draft(script, Deal::Buy, "Water", 2, "10 cr each  Total -20 cr");
    // The track runs 0..=6 t: its left edge is zero, and 60% of it rounds
    // to four.
    for (share, slid, summary) in [
        (0.0, 0, "10 cr each  Total -0 cr"),
        (0.6, 4, "10 cr each  Total -40 cr"),
    ] {
        script = script
            .step(format!("sketch: aim at the quantity slider at {share}"))
            .on_enter(move |world: &mut World| {
                let track = ui_node_rect(world, DEAL_SLIDER).expect("the slider is laid out");
                let x = track.min.x + (track.width() * share).max(1.0);
                move_cursor(Vec2::new(x, track.center().y))(world);
            })
            .until(frames(2))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("sketch: press the quantity slider at {share}"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("sketch: release the quantity slider at {share}"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(resource_where::<Draft>(move |draft| {
                draft.0.is_some_and(|open| open.qty == Some(slid))
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add();
        script = expect_draft(script, Deal::Buy, "Water", slid, summary);
        script = expect_cues(script, "the slider", &[UiSfx::UiTick]);
        if slid == 0 {
            script = transact(
                script,
                "confirm an empty slider",
                DEAL_CONFIRM,
                |before, after| assert_refused(before, after, "Refused: quantity is zero"),
            );
            script = expect_cues(script, "a refused zero", &[UiSfx::EditorDeny]);
        }
    }
    for (typed, refusal, cues) in [
        ("", "Refused: enter a quantity", &[][..]),
        ("0", "Refused: quantity is zero", &[UiSfx::UiTick][..]),
        ("999", "Refused: only 6 t Water left", &[UiSfx::UiTick][..]),
    ] {
        script = type_qty(script, "type a quantity Confirm refuses", typed);
        script = expect_cues(script, "typing a quantity", cues);
        script = transact(
            script,
            "confirm a typed quantity",
            DEAL_CONFIRM,
            move |before, after| assert_refused(before, after, refusal),
        );
        script = expect_cues(script, "a refused typed quantity", &[UiSfx::EditorDeny]);
        script = script
            .step(format!("sketch: `{typed}` stays in the field"))
            .on_enter(move |world: &mut World| {
                let field = world
                    .query_filtered::<&TextFieldValue, With<DraftField>>()
                    .single(world)
                    .expect("one quantity field")
                    .0
                    .clone();
                let slider = world
                    .query_filtered::<&SliderValue, With<DraftSlider>>()
                    .single(world)
                    .expect("one quantity slider")
                    .0;
                let typed_qty = typed.parse::<u32>().ok();
                assert_eq!(
                    (
                        field.as_str(),
                        world.resource::<Draft>().0.map(|open| open.qty)
                    ),
                    (typed, Some(typed_qty)),
                    "a refused `{typed}` must stay as typed until it is corrected"
                );
                assert_eq!(
                    slider,
                    typed_qty.map_or(4.0, |qty| qty.min(6) as f32),
                    "the slider shows the number held to the stock, or keeps the last one"
                );
                let marked = world
                    .query_filtered::<(), (With<DraftField>, With<TextFieldError>)>()
                    .iter(world)
                    .count();
                assert_eq!(
                    marked,
                    usize::from(typed_qty.is_none()),
                    "the field is marked exactly while it holds no number"
                );
                if typed.is_empty() {
                    assert_eq!(named_text(world, DEAL_TOTAL), "Type a whole number");
                }
            })
            .add();
    }
    script = type_qty(script, "type one tonne", "1");
    script = expect_cues(script, "typing one tonne", &[UiSfx::UiTick]);
    script = expect_draft(script, Deal::Buy, "Water", 1, "10 cr each  Total -10 cr");
    script = assert_nodes_kept(
        script,
        "opening water, wheeling, sliding, typing and refused Confirms",
    );
    // The second click lands where Confirm was, after the first closed it.
    let confirm_at = std::sync::Arc::new(std::sync::Mutex::new(Vec2::ZERO));
    let aim = confirm_at.clone();
    script = around_fixture(
        script,
        "double-click Confirm on one tonne of water",
        move |script| {
            script
                .step("sketch: double-click Confirm")
                .on_enter(move |world: &mut World| {
                    let centre = ui_node_centre(world, DEAL_CONFIRM).expect("Confirm is laid out");
                    *aim.lock().expect("the aim is not poisoned") = centre;
                    click_at(centre, MouseButton::Left)(world);
                })
                .each(move |world: &mut World, _, frame| match frame {
                    1 | 3 => release_mouse(MouseButton::Left)(world),
                    2 => click_at(
                        *confirm_at.lock().expect("the aim is not poisoned"),
                        MouseButton::Left,
                    )(world),
                    _ => {}
                })
                .until(and(frames(4), pointer_released()))
                .deadline(BEAT_DEADLINE_SECS)
                .add()
        },
        |before, after| {
            assert_eq!(
                (
                    qty(after, Store::Own, "Water"),
                    qty(after, Store::Market, "Water"),
                    after.credits,
                    after.own.stock.mass_kg(),
                ),
                (
                    qty(before, Store::Own, "Water") + 1,
                    qty(before, Store::Market, "Water") - 1,
                    before.credits - 10,
                    before.own.stock.mass_kg() + 1000,
                ),
                "a double-clicked Confirm must buy one tonne of water once, for 10 cr"
            );
            assert_eq!(after.notice, "Bought 1 t Water  -10 cr");
        },
    );
    script = expect_cues(script, "a double-clicked Confirm", &[UiSfx::MenuSelect]);
    script = drafts_closed(script, "the purchase");
    script = script.click_named(
        "sketch: click the water again",
        &Store::Market.row_name(good("Water")),
        resource_where::<Draft>(|draft| draft.0.is_some()),
        BEAT_DEADLINE_SECS,
    );
    script = expect_draft(script, Deal::Buy, "Water", 1, "10 cr each  Total -10 cr");
    script = expect_cues(script, "clicking the water again", &[UiSfx::MenuSelect]);
    script = select_row(
        script,
        "open own ore",
        Store::Own,
        "Nickel ore",
        Some(Deal::Sell),
    );
    script = expect_draft(
        script,
        Deal::Sell,
        "Nickel ore",
        1,
        "30 cr each  Total +30 cr",
    );
    script = expect_cues(script, "opening the ore", &[UiSfx::MenuSelect]);
    script = set_draft(script, "sell all the ore", DEAL_ALL, 4);
    script = expect_draft(
        script,
        Deal::Sell,
        "Nickel ore",
        4,
        "30 cr each  Total +120 cr",
    );
    script = expect_cues(script, "All", &[UiSfx::MenuSelect]);
    script = script
        .step("sketch: All fills the slider")
        .on_enter(|world: &mut World| {
            let (value, range) = world
                .query_filtered::<(&SliderValue, &SliderRange), With<DraftSlider>>()
                .single(world)
                .map(|(value, range)| (value.0, *range))
                .expect("one quantity slider");
            assert!(
                range == SliderRange::new(0.0, 4.0) && range.thumb_position(value) == 1.0,
                "All must fill the 0..=4 track, not {value} on {range:?}"
            );
        })
        .add();
    script = transact(
        script,
        "sell four tonnes of ore",
        DEAL_CONFIRM,
        |before, after| {
            assert_eq!(
                (
                    qty(after, Store::Own, "Nickel ore"),
                    qty(after, Store::Market, "Nickel ore"),
                    after.credits,
                ),
                (0, qty(before, Store::Market, "Nickel ore") + 4, 1310),
                "Sell must move all four tonnes of ore into the market for 120 cr"
            );
            assert_eq!(after.notice, "Sold 4 t Nickel ore  +120 cr");
        },
    );
    script = expect_cues(script, "the sale", &[UiSfx::MenuSelect]);
    script = drafts_closed(script, "the sale");
    script = select_row(
        script,
        "open a market pump",
        Store::Market,
        "Pump",
        Some(Deal::Buy),
    );
    script = set_draft(script, "ask for both pumps", DEAL_ALL, 2);
    script = expect_draft(script, Deal::Buy, "Pump", 2, "700 cr each  Total -1400 cr");
    script = expect_cues(
        script,
        "opening the pump and All",
        &[UiSfx::MenuSelect, UiSfx::MenuSelect],
    );
    script = transact(
        script,
        "buy both pumps short of credits",
        DEAL_CONFIRM,
        |before, after| {
            assert_refused(before, after, "Refused: need 1400 cr, have 1310 cr");
        },
    );
    script = expect_cues(script, "a refusal for credits", &[UiSfx::EditorDeny]);
    script = expect_draft(script, Deal::Buy, "Pump", 2, "700 cr each  Total -1400 cr");
    shot(script, "phosphor-inventory-station")
}

/// No confirmation is open, the quantity field has let go of the keyboard,
/// and the selection stays on the item.
#[cfg(feature = "debug")]
fn drafts_closed(script: Script, after: &'static str) -> Script {
    script
        .step(format!("sketch: {after} closed the confirmation"))
        .on_enter(move |world: &mut World| {
            assert_eq!(
                world.resource::<Draft>().0,
                None,
                "{after} must close the confirmation"
            );
            assert!(
                ui_node_rect(world, DEAL_CONFIRM).is_none()
                    && ui_node_rect(world, INSPECT_STOCK).is_some(),
                "{after} must hide Confirm and keep the item's facts"
            );
            assert!(
                world.resource::<Inspected>().0.is_some(),
                "{after} must keep the item selected"
            );
            let focused = world
                .query_filtered::<(), (With<DraftField>, With<TextFieldFocused>)>()
                .iter(world)
                .count();
            assert_eq!(
                (focused, *world.resource::<InputMode>()),
                (0, InputMode::Normal),
                "{after} must take the keyboard back from the quantity field"
            );
        })
        .add()
}

/// Boarded: the picket's own slugs open no deal, since there is no stow, and
/// their click still selects them.
/// The raider's one core hides the quantity slider, and its sixty slugs show
/// it again. All the slugs are looted into the picket's hold for free.
#[cfg(feature = "debug")]
fn boarded_loot(mut script: Script) -> Script {
    script = script
        .step("sketch: forget the cues before the loot")
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add();
    script = select_row(script, "select own slugs", Store::Own, "Slugs", None);
    script = expect_cues(script, "a row with no deal", &[UiSfx::MenuSelect]);
    script = select_row(
        script,
        "open the raider's one core",
        Store::Boarded,
        "Core",
        Some(Deal::Loot),
    );
    script = expect_cues(script, "opening the core", &[UiSfx::MenuSelect]);
    script = script
        .step("sketch: one core hides the slider and leaves no gap")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, DEAL_SLIDER).is_none(),
                "a slider over one unit cannot move and must hide"
            );
            assert!(
                ui_node_rect(world, DEAL_FIELD).is_some()
                    && ui_node_rect(world, DEAL_ALL).is_some(),
                "the field and All stay over one unit"
            );
            let row = ui_node_rect(world, DEAL_ROW).expect("the quantity row is laid out");
            let total = ui_node_rect(world, DEAL_TOTAL).expect("the price line is laid out");
            let gap = total.min.y - row.max.y;
            assert!(
                (gap - 10.0).abs() < 0.5,
                "the price line must follow the quantity row by the form's 10 px gap, \
                 not {gap} px: the hidden slider leaves no band"
            );
        })
        .add();
    script = select_row(
        script,
        "open the raider's slugs",
        Store::Boarded,
        "Slugs",
        Some(Deal::Loot),
    );
    script = expect_draft(script, Deal::Loot, "Slugs", 1, "Free  no credits change");
    script = script
        .step("sketch: sixty slugs show the slider again")
        .on_enter(|world: &mut World| {
            let range = *world
                .query_filtered::<&SliderRange, With<DraftSlider>>()
                .single(world)
                .expect("one quantity slider");
            assert!(
                ui_node_rect(world, DEAL_SLIDER).is_some() && range == SliderRange::new(0.0, 60.0),
                "over sixty slugs the slider must show and run 0..=60, not {range:?}"
            );
        })
        .add();
    script = set_draft(script, "loot every slug", DEAL_ALL, 60);
    script = expect_cues(
        script,
        "opening the slugs and All",
        &[UiSfx::MenuSelect, UiSfx::MenuSelect],
    );
    script = transact(script, "loot sixty slugs", DEAL_CONFIRM, |before, after| {
        assert_eq!(
            (
                qty(after, Store::Own, "Slugs"),
                qty(after, Store::Boarded, "Slugs"),
                after.credits,
            ),
            (qty(before, Store::Own, "Slugs") + 60, 0, before.credits),
            "Loot must move every slug into the picket's hold for free"
        );
        assert_eq!(after.notice, "Looted x60 Slugs  free");
    });
    script = expect_cues(script, "the loot", &[UiSfx::MenuSelect]);
    drafts_closed(script, "the loot")
}

/// Select a ship section by its badge and check the selection took.
#[cfg(feature = "debug")]
fn select_section(script: Script, code: &'static str) -> Script {
    let badge = format!("Ship Blip {code}");
    script
        .click_named(
            &format!("sketch: select {code}"),
            &badge,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step(format!("sketch: {code} is selected"))
        .until(frames(3))
        .add()
        .step(format!("sketch: the detail shows {code}"))
        .on_enter(move |world: &mut World| {
            assert_eq!(
                section_order(world).0.as_deref(),
                Some(code),
                "clicking the {code} badge must select it"
            );
        })
        .add()
}

/// At the station: repair a worn section for its price, refuse one the
/// credits cannot cover, and show that with the repair bay off the station
/// prices nothing and the fixture refuses.
#[cfg(feature = "debug")]
fn ship_repairs(mut script: Script) -> Script {
    script = script
        .step("sketch: forget the cues before the repairs")
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add();
    script = select_section(script, PICKED_SECTION);
    script = expect_cues(script, "a ship section", &[UiSfx::MenuSelect]);
    script = transact(
        script,
        "repair the point defence",
        SHIP_REPAIR,
        |before, after| {
            assert_eq!(
                (
                    before.condition(PICKED_SECTION),
                    after.condition(PICKED_SECTION),
                    before.credits,
                    after.credits
                ),
                (45, 100, 1310, 650),
                "Repair must restore {PICKED_SECTION} for 660 cr"
            );
            assert_eq!(after.notice, format!("Repaired {PICKED_SECTION}  -660 cr"));
        },
    );
    script = expect_cues(script, "a repair", &[UiSfx::MenuSelect]);
    script = script
        .step("sketch: the repaired section reads intact")
        .on_enter(|world: &mut World| {
            let status = named_text(world, SHIP_STATUS);
            assert!(
                status.contains("Condition 100%  nominal"),
                "the detail must show the repair, not `{status}`"
            );
            assert_eq!(button_label(world, SHIP_REPAIR), "Intact");
        })
        .add();
    script = verdict(script, SketchView::Ship, SketchContext::Station, "intact");
    script = select_section(script, "THR-1");
    script = expect_cues(script, "a ship section", &[UiSfx::MenuSelect]);
    script = transact(
        script,
        "repair the thruster short of credits",
        SHIP_REPAIR,
        |before, after| {
            assert_refused(before, after, "Refused: need 960 cr, have 650 cr");
        },
    );
    script = expect_cues(script, "a refused repair", &[UiSfx::EditorDeny]);
    script = transact(
        script,
        "switch the repair bay off",
        SHIP_BAY,
        |before, after| {
            assert!(!after.repair_bay, "the switch must turn the bay off");
            assert_eq!(after.notice, "Repair bay off (fixture)");
            assert_unchanged(
                before,
                &SketchFixture {
                    repair_bay: true,
                    ..after.clone()
                },
                "the bay switch",
            );
        },
    );
    script = expect_cues(script, "the bay switch", &[UiSfx::MenuSelect]);
    script = script
        .step("sketch: with the bay off, the fixture refuses a repair")
        .on_enter(|world: &mut World| {
            let live = world.resource::<SketchFixture>().clone();
            let mut probe = live.clone();
            assert_eq!(
                probe.repair(SketchContext::Station, "THR-1"),
                Err("Refused: repair bay off".to_string())
            );
            assert_eq!(probe, live, "a refused repair must change nothing");
        })
        .add();
    script = verdict(script, SketchView::Ship, SketchContext::Station, "bay off");
    script = transact(
        script,
        "switch the repair bay back on",
        SHIP_BAY,
        |_, after| {
            assert!(after.repair_bay, "the switch must turn the bay on");
        },
    );
    expect_cues(script, "the bay switch", &[UiSfx::MenuSelect])
}

/// In a window too short for the narrow ship view, the wheel over the
/// context line scrolls the body until the ship panel is on screen.
#[cfg(feature = "debug")]
fn scroll_short(script: Script) -> Script {
    let script = resize(script, SHORT, "short");
    script
        .step("sketch: the short window cuts the ship panel off")
        .on_enter(|world: &mut World| {
            let window = window_rect(world);
            let panel = ui_node_rect(world, SHIP_PANEL).expect("the ship panel is laid out");
            assert!(
                panel.max.y > window.max.y,
                "a {}x{} window must be too short for the narrow ship view, panel {panel:?}",
                window.max.x,
                window.max.y
            );
            let banner = ui_node_centre(world, DOCK_BANNER).expect("the context line is laid out");
            move_cursor(banner)(world);
        })
        .until(frames(2))
        .add()
        .step("sketch: wheel down over the context line")
        .on_enter(scroll_lines(-10.0))
        .until(frames(4))
        .add()
        .step("sketch: the body scrolled the ship panel into view")
        .on_enter(|world: &mut World| {
            let scrolled = world
                .query_filtered::<&ScrollPosition, With<SketchBody>>()
                .single(world)
                .expect("one sketch body")
                .0
                .y;
            let window = window_rect(world);
            let panel = ui_node_rect(world, SHIP_PANEL).expect("the ship panel is laid out");
            assert!(
                scrolled > 0.0 && inside(panel, window),
                "the wheel must scroll the body ({scrolled} px) until the ship panel {panel:?} \
                 is inside the window"
            );
            assert_sim_still(world, "short window");
            info!("sketch: the short window scrolled {scrolled} px");
        })
        .add()
}

/// Reshape the window and wait for the layout to take it.
#[cfg(feature = "debug")]
fn resize(script: Script, size: Vec2, label: &str) -> Script {
    script
        .step(format!("sketch: {label} window"))
        .on_enter(move |world: &mut World| {
            let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
            let mut window = windows.single_mut(world).expect("one primary window");
            window.resolution.set(size.x, size.y);
        })
        .until(and(window_size_is(size.x, size.y), frames(4)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

/// Click the tab for `view` and wait for the resource to take it.
#[cfg(feature = "debug")]
fn open(script: Script, view: SketchView) -> Script {
    let (_, label, name) = SketchView::ALL
        .into_iter()
        .find(|(value, _, _)| *value == view)
        .expect("every view has a tab");
    let script = script
        .step(format!("sketch: forget the cues before {label}"))
        .on_enter(|world: &mut World| world.resource_mut::<HeardCues>().0.clear())
        .add()
        .click_named(
            &format!("sketch: open {label}"),
            name,
            resource_where::<SketchView>(move |current| *current == view),
            BEAT_DEADLINE_SECS,
        )
        .step(format!("sketch: {label} settles"))
        .until(frames(3))
        .add();
    expect_cues(script, "a view tab", &[UiSfx::MenuSelect])
}

/// Open NOVA OS with its own key over the screens and close it with Escape,
/// then press Escape with nothing open. The screens stay up, nothing pauses
/// or opens, and the clocks stay held throughout.
#[cfg(feature = "debug")]
fn nova_os_round_trip(script: Script) -> Script {
    let pause_is =
        |wanted: PauseStates| resource_where::<State<PauseStates>>(move |s| *s.get() == wanted);
    script
        .step("sketch: Tab opens NOVA OS over the screens")
        .on_enter(press_key(KeyCode::Tab))
        .until(pause_is(PauseStates::NovaOs))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sketch: release Tab")
        .on_enter(release_key(KeyCode::Tab))
        .until(frames(10))
        .add()
        .step("sketch: NOVA OS kept the simulation still")
        .on_enter(|world: &mut World| {
            assert_sim_still(world, "NOVA OS open");
            let orbit = map_orbit(world);
            world.insert_resource(GestureProbe {
                at: Vec2::ZERO,
                orbit,
                blips: Vec::new(),
            });
            press_key(KeyCode::KeyW)(world);
        })
        .until(frames(5))
        .add()
        .step("sketch: W under NOVA OS left the screens' map alone")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::KeyW)(world);
            // The target, not the center: the center may still be settling
            // its ease, and only a pan moves the target.
            let before = world
                .remove_resource::<GestureProbe>()
                .expect("the map was noted")
                .orbit
                .center_target;
            let after = map_orbit(world).center_target;
            assert_eq!(
                after, before,
                "W while NOVA OS is open belongs to NOVA OS, not the screens' map"
            );
            assert_ship_unflown(world, "W under NOVA OS");
        })
        .add()
        .step("sketch: Escape closes NOVA OS")
        .on_enter(press_key(KeyCode::Escape))
        .until(pause_is(PauseStates::Unpaused))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sketch: release Escape after NOVA OS")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(10))
        .add()
        .step("sketch: closing NOVA OS left the simulation still")
        .on_enter(|world: &mut World| assert_sim_still(world, "NOVA OS closed"))
        .add()
        .step("sketch: Escape with nothing open")
        .on_enter(press_key(KeyCode::Escape))
        .until(frames(5))
        .add()
        .step("sketch: release the lone Escape")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(5))
        .add()
        .step("sketch: Escape opened nothing")
        .on_enter(|world: &mut World| {
            let pause = *world.resource::<State<PauseStates>>().get();
            assert_eq!(
                pause,
                PauseStates::Unpaused,
                "Escape over the screens must neither pause nor open NOVA OS"
            );
            assert!(
                world.resource::<SketchView>() == &SketchView::Map,
                "Escape must leave the screens on their view"
            );
            info!("sketch: Escape opened nothing");
        })
        .add()
}

/// Hold W, D and Space on the map, then Shift: the orbit center flies along,
/// across and up the camera's heading, then down, on real time, while the
/// ship takes none of it and the simulation stays still.
#[cfg(feature = "debug")]
fn fly_map(script: Script) -> Script {
    let flying = [KeyCode::KeyW, KeyCode::KeyD, KeyCode::Space];
    script
        .step("sketch: hold W, D and Space on the map")
        .on_enter(move |world: &mut World| {
            let orbit = map_orbit(world);
            world.insert_resource(GestureProbe {
                at: Vec2::ZERO,
                orbit,
                blips: Vec::new(),
            });
            for key in flying {
                press_key(key)(world);
            }
        })
        .until(frames(5))
        .add()
        .step("sketch: the map flew forward, right and up")
        .on_enter(move |world: &mut World| {
            for key in flying {
                release_key(key)(world);
            }
            let before = copy_orbit(&world.resource::<GestureProbe>().orbit);
            let after = map_orbit(world);
            let heading = -Vec3::new(before.theta.sin(), 0.0, before.theta.cos());
            let side = heading.cross(Vec3::Y);
            let moved = after.center - before.center;
            assert!(
                moved.dot(heading) > 0.0 && moved.dot(side) > 0.0 && moved.y > 0.0,
                "W, D and Space must fly the map forward, right and up, not by {moved:?}"
            );
            assert!(
                (after.center_target - before.center_target - moved).length() < 1e-3,
                "the free camera must carry the ease target with the center"
            );
            assert_eq!(
                (after.theta, after.phi, after.radius),
                (before.theta, before.phi, before.radius),
                "flying must not turn or zoom the map"
            );
            assert_ship_unflown(world, "W, D and Space on the map");
            assert_sim_still(world, "W, D and Space on the map");
            info!("sketch: the map flew {:.1} u", moved.length());
            world.resource_mut::<GestureProbe>().orbit = after;
            press_key(KeyCode::ShiftLeft)(world);
        })
        .until(frames(5))
        .add()
        .step("sketch: Shift flew the map down")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::ShiftLeft)(world);
            let before = world
                .remove_resource::<GestureProbe>()
                .expect("the map was noted")
                .orbit;
            let after = map_orbit(world);
            assert!(
                after.center.y < before.center.y,
                "Shift must fly the map down, not from {:?} to {:?}",
                before.center,
                after.center
            );
            assert_ship_unflown(world, "Shift on the map");
            assert_sim_still(world, "Shift on the map");
        })
        .until(frames(2))
        .add()
}

/// The code of the selected ship section, and the codes in selection order.
#[cfg(feature = "debug")]
fn section_order(world: &mut World) -> (Option<String>, Vec<String>) {
    let selected = world.resource::<ShipSelection>().0;
    let views = world
        .run_system_once(|sections: ShipSections| sections.collect())
        .expect("the section model runs");
    let current = selected.and_then(|selected| {
        views
            .iter()
            .find(|view| view.entity == selected)
            .map(|view| view.code.clone())
    });
    (current, views.into_iter().map(|view| view.code).collect())
}

/// Next moves the ship selection one section along the code order, and Prev
/// brings it back.
#[cfg(feature = "debug")]
fn step_sections(script: Script) -> Script {
    let script = script
        .click_named(
            "sketch: next section",
            SHIP_NEXT,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: Next selected the following section")
        .until(frames(3))
        .add()
        .step("sketch: Next moved one section along")
        .on_enter(|world: &mut World| {
            let (current, codes) = section_order(world);
            let at = codes
                .iter()
                .position(|code| code == PICKED_SECTION)
                .expect("the picked section is listed");
            let expected = &codes[(at + 1) % codes.len()];
            assert_eq!(
                current.as_deref(),
                Some(expected.as_str()),
                "Next after {PICKED_SECTION} must select {expected}"
            );
            assert!(
                named_text(world, SHIP_DETAIL).starts_with(expected.as_str()),
                "the detail must follow Next to {expected}"
            );
            info!("sketch: Next selected {expected}");
        })
        .add();
    let script = expect_cues(script, "Next", &[UiSfx::MenuSelect]);
    let script = script
        .click_named(
            "sketch: previous section",
            SHIP_PREV,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: Prev selected the section before")
        .until(frames(3))
        .add()
        .step("sketch: Prev came back")
        .on_enter(|world: &mut World| {
            let (current, _) = section_order(world);
            assert_eq!(
                current.as_deref(),
                Some(PICKED_SECTION),
                "Prev must come back to {PICKED_SECTION}"
            );
        })
        .add();
    expect_cues(script, "Prev", &[UiSfx::MenuSelect])
}

/// Fit frames the whole ship at the dragged angles; Reset also restores the
/// opening angles, which the drag before it turned away from.
#[cfg(feature = "debug")]
fn fit_and_reset_ship(script: Script) -> Script {
    let framing = |world: &mut World| {
        let (centre, radius) = world
            .run_system_once(|sections: ShipSections| ship_framing(&sections.collect()))
            .expect("the section model runs");
        (centre, radius * SHIP_PANE_ZOOM)
    };
    let script = script
        .step("sketch: note the ship orbit before Fit")
        .on_enter(|world: &mut World| {
            let orbit = pane_orbit(world, SketchView::Ship);
            world.insert_resource(GestureProbe {
                at: Vec2::ZERO,
                orbit,
                blips: Vec::new(),
            });
        })
        .add()
        .click_named(
            "sketch: fit the ship",
            SHIP_FIT,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: Fit framed the whole ship")
        .on_enter(move |world: &mut World| {
            let before = world
                .remove_resource::<GestureProbe>()
                .expect("the orbit was noted")
                .orbit;
            let (centre, radius) = framing(world);
            let orbit = pane_orbit(world, SketchView::Ship);
            assert_eq!(
                (orbit.theta, orbit.phi, orbit.radius, orbit.center_target),
                (before.theta, before.phi, radius, centre),
                "Fit must frame the whole ship and keep the angles"
            );
            assert_ne!(
                (before.radius, before.center_target),
                (radius, centre),
                "the ship was already fitted: Fit proves nothing"
            );
        })
        .add();
    let script = expect_cues(script, "Fit", &[UiSfx::MenuSelect]);
    let script = script
        .click_named(
            "sketch: reset the ship",
            SHIP_RESET,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: Reset restored the opening view")
        .on_enter(move |world: &mut World| {
            let (centre, radius) = framing(world);
            let orbit = pane_orbit(world, SketchView::Ship);
            assert_eq!(
                (orbit.theta, orbit.phi, orbit.radius, orbit.center_target),
                (SHIP_THETA, SHIP_PHI, radius, centre),
                "Reset must restore the opening orbit"
            );
        })
        .add();
    expect_cues(script, "Reset", &[UiSfx::MenuSelect])
}

/// The camera orbit of a view's pane.
#[cfg(feature = "debug")]
fn pane_orbit(world: &World, view: SketchView) -> SketchOrbit {
    let orbit = match view {
        SketchView::Map => world
            .try_query_filtered::<&SketchOrbit, With<MapCamera>>()
            .and_then(|mut query| query.single(world).ok().map(copy_orbit)),
        SketchView::Ship => world
            .try_query_filtered::<&SketchOrbit, With<ShipCamera>>()
            .and_then(|mut query| query.single(world).ok().map(copy_orbit)),
        SketchView::Inventory => None,
    };
    orbit.unwrap_or_else(|| panic!("the {view:?} pane has no orbit camera"))
}

#[cfg(feature = "debug")]
fn copy_orbit(orbit: &SketchOrbit) -> SketchOrbit {
    SketchOrbit {
        theta: orbit.theta,
        phi: orbit.phi,
        radius: orbit.radius,
        center: orbit.center,
        center_target: orbit.center_target,
        centered_on: orbit.centered_on,
    }
}

#[cfg(feature = "debug")]
fn map_orbit(world: &World) -> SketchOrbit {
    pane_orbit(world, SketchView::Map)
}

#[cfg(feature = "debug")]
fn player_position(world: &World) -> Vec3 {
    world
        .try_query_filtered::<&GlobalTransform, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.single(world).ok().map(GlobalTransform::translation))
        .expect("one player ship")
}

/// The opening map radius for the live scene, by the NOVA OS helpers.
#[cfg(feature = "debug")]
fn expected_map_framing(world: &mut World) -> f32 {
    let player = player_position(world);
    world
        .run_system_once(move |contacts: MapContacts| {
            map_radius_default(map_spread(&contacts, player))
        })
        .expect("the contact model runs")
}

/// Where the selected thing named `code` sits in its pane's scene space.
#[cfg(feature = "debug")]
fn selection_point(world: &mut World, view: SketchView, code: &'static str) -> Vec3 {
    match view {
        SketchView::Map => world
            .run_system_once(move |contacts: MapContacts| {
                contacts
                    .collect()
                    .into_iter()
                    .find(|contact| contact.code == code)
                    .map(|contact| contact.world_pos)
            })
            .expect("the contact model runs")
            .unwrap_or_else(|| panic!("no contact {code}")),
        SketchView::Ship => world
            .run_system_once(move |sections: ShipSections| {
                sections
                    .collect()
                    .into_iter()
                    .find(|view| view.code == code)
                    .map(|view| view.local.translation)
            })
            .expect("the section model runs")
            .unwrap_or_else(|| panic!("no section {code}")),
        SketchView::Inventory => unreachable!("the hold has no orbit"),
    }
}

/// Frames in which a pane camera's center moved by one real-time ease step.
/// A step counts at any frame length, and a step on the paused virtual clock
/// would not match it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct EaseLog(u32);

#[cfg(feature = "debug")]
fn count_easing_frames(
    time: Res<Time<Real>>,
    orbits: Query<(Entity, &SketchOrbit)>,
    mut last: Local<HashMap<Entity, Vec3>>,
    mut log: ResMut<EaseLog>,
) {
    for (entity, orbit) in &orbits {
        let Some(before) = last.insert(entity, orbit.center) else {
            continue;
        };
        let eased = ease_orbit_center(before, orbit.center_target, time.delta_secs());
        if before != orbit.center && eased.distance(orbit.center) < 1e-4 {
            log.0 += 1;
        }
    }
}

/// Where an ease started, and the [`EaseLog`] count then.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct EaseWatch {
    start: Vec3,
    moving: u32,
}

/// Note the view's orbit center before a selection click.
#[cfg(feature = "debug")]
fn watch_ease(script: Script, view: SketchView) -> Script {
    script
        .step(format!(
            "sketch: note the {} orbit before selecting",
            view.slug()
        ))
        .on_enter(move |world: &mut World| {
            let watch = EaseWatch {
                start: pane_orbit(world, view).center,
                moving: world.resource::<EaseLog>().0,
            };
            world.insert_resource(watch);
        })
        .add()
}

/// Wait for the view's orbit center to ease onto `code`, and check it moved by
/// real-time ease steps while the simulation stayed paused.
#[cfg(feature = "debug")]
fn ease_onto(script: Script, view: SketchView, code: &'static str) -> Script {
    script
        .step(format!("sketch: {} eases onto {code}", view.slug()))
        .until(std::sync::Arc::new(move |world: &World| {
            let orbit = pane_orbit(world, view);
            orbit.center.distance(orbit.center_target) < CENTERED
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("sketch: {} eased onto {code}", view.slug()))
        .on_enter(move |world: &mut World| {
            let watch = world
                .remove_resource::<EaseWatch>()
                .expect("the ease was noted");
            let target = selection_point(world, view, code);
            let orbit = pane_orbit(world, view);
            assert!(
                orbit.center.distance(target) < CENTERED,
                "selecting {code} must center the {view:?} orbit on it, not {:?}",
                orbit.center
            );
            assert!(
                watch.start.distance(target) >= CENTERED,
                "the {view:?} orbit already sat on {code}: the ease proves nothing"
            );
            let moving = world.resource::<EaseLog>().0 - watch.moving;
            assert!(
                moving >= 1,
                "the {view:?} center must ease onto {code} on real time, not jump"
            );
            assert_sim_still(world, "after an ease");
            info!(
                "sketch: {} eased {:.2} u onto {code} in {moving} real-time step(s)",
                view.slug(),
                watch.start.distance(target),
            );
        })
        .add()
}

/// A point on the view's pane that is far from every blip and label, so a
/// press there lands on the pane itself.
#[cfg(feature = "debug")]
fn pane_background(world: &World, view: SketchView) -> Vec2 {
    let pane = match view {
        SketchView::Map => MAP_SCENE,
        SketchView::Ship => SHIP_SCENE,
        SketchView::Inventory => unreachable!("the hold has no orbit"),
    };
    let rect = ui_node_rect(world, pane).unwrap_or_else(|| panic!("{pane} is not laid out"));
    let mut taken: Vec<Vec2> = Vec::new();
    if let Some(mut nodes) = world.try_query_filtered::<(
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    ), Or<(With<MapBlip>, With<ShipBlip>, With<BlipLabel>)>>()
    {
        for (node, transform, shown) in nodes.iter(world) {
            if shown.get() {
                taken.push(transform.translation * node.inverse_scale_factor());
            }
        }
    }
    let mut best = (rect.center(), f32::NEG_INFINITY);
    for row in 1..8 {
        for col in 1..10 {
            let at = rect.min + rect.size() * Vec2::new(col as f32 / 10.0, row as f32 / 8.0);
            let clear = taken
                .iter()
                .map(|blip| blip.distance(at))
                .fold(f32::INFINITY, f32::min);
            if clear > best.1 {
                best = (at, clear);
            }
        }
    }
    best.0
}

/// What a drag or wheel beat measured before it moved the pointer.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct GestureProbe {
    at: Vec2,
    orbit: SketchOrbit,
    blips: Vec<(Entity, Vec2)>,
}

#[cfg(feature = "debug")]
fn blip_positions(world: &World) -> Vec<(Entity, Vec2)> {
    world
        .try_query_filtered::<(Entity, &UiGlobalTransform, &InheritedVisibility), Or<(With<MapBlip>, With<ShipBlip>)>>()
        .map(|mut query| {
            query
                .iter(world)
                .filter(|(_, _, shown)| shown.get())
                .map(|(entity, transform, _)| (entity, transform.translation))
                .collect()
        })
        .unwrap_or_default()
}

/// Drag the pane background 120 px right with the left button, and check
/// the orbit turned by the NOVA OS drag rate and the blips moved with it.
#[cfg(feature = "debug")]
fn drag_pane(script: Script, view: SketchView) -> Script {
    const STEP_PX: f32 = 40.0;
    const STEPS: u32 = 3;
    let mut script = script
        .step(format!("sketch: press on the {} pane", view.slug()))
        .on_enter(move |world: &mut World| {
            let at = pane_background(world, view);
            let probe = GestureProbe {
                at,
                orbit: pane_orbit(world, view),
                blips: blip_positions(world),
            };
            world.insert_resource(probe);
            click_at(at, MouseButton::Left)(world);
        })
        .until(frames(2))
        .add();
    for step in 1..=STEPS {
        script = script
            .step(format!("sketch: drag the {} pane ({step})", view.slug()))
            .on_enter(move |world: &mut World| {
                let at = world.resource::<GestureProbe>().at;
                move_cursor(at + Vec2::X * STEP_PX * step as f32)(world);
            })
            .until(frames(2))
            .add();
    }
    script
        .step(format!("sketch: let go of the {} pane", view.slug()))
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(3))
        .add()
        .step(format!("sketch: the drag orbited the {}", view.slug()))
        .on_enter(move |world: &mut World| {
            let probe = world
                .remove_resource::<GestureProbe>()
                .expect("the drag ran");
            let orbit = pane_orbit(world, view);
            let turned = orbit.theta - probe.orbit.theta;
            let expected = OrbitGesture {
                drag: Some(Vec2::X * STEP_PX * STEPS as f32),
                ..default()
            }
            .apply(0.0, probe.orbit.theta, probe.orbit.phi)
            .0 - probe.orbit.theta;
            assert!(
                (turned - expected).abs() < 1e-3,
                "a {} px drag must turn the {view:?} orbit by {expected}, not {turned}",
                STEP_PX * STEPS as f32
            );
            let after = blip_positions(world);
            let moved = probe.blips.iter().any(|(blip, before)| {
                after
                    .iter()
                    .any(|(other, now)| other == blip && now.distance(*before) > 2.0)
            });
            assert!(moved, "the drag must move the {view:?} blips");
            assert_sim_still(world, "after a drag");
            info!(
                "sketch: the drag turned the {} by {turned:.3} rad",
                view.slug()
            );
        })
        .add()
}

/// Wheel in two notches, then far out, and check the zoom and its clamp.
#[cfg(feature = "debug")]
fn wheel_pane(script: Script, view: SketchView) -> Script {
    script
        .step(format!("sketch: point at the {} pane", view.slug()))
        .on_enter(move |world: &mut World| {
            let at = pane_background(world, view);
            let probe = GestureProbe {
                at,
                orbit: pane_orbit(world, view),
                blips: Vec::new(),
            };
            world.insert_resource(probe);
            move_cursor(at)(world);
        })
        .until(frames(2))
        .add()
        .step(format!("sketch: wheel in on the {}", view.slug()))
        .on_enter(scroll_lines(2.0))
        .until(frames(3))
        .add()
        .step(format!("sketch: the wheel zoomed the {} in", view.slug()))
        .on_enter(move |world: &mut World| {
            let (before, limits) = {
                let probe = world.resource::<GestureProbe>();
                (probe.orbit.radius, zoom_limits(world, view))
            };
            let radius = pane_orbit(world, view).radius;
            let expected = zoom_radius(before, 2.0, limits.0, limits.1);
            assert!(
                radius < before && (radius - expected).abs() < 1e-3,
                "two notches must zoom the {view:?} from {before} to {expected}, not {radius}"
            );
        })
        .add()
        // One wheel event scales the radius once, so the pull out to the
        // ceiling takes a few.
        .step(format!("sketch: wheel far out on the {}", view.slug()))
        .on_enter(scroll_lines(-60.0))
        .until(frames(2))
        .add()
        .step(format!("sketch: wheel further out on the {}", view.slug()))
        .on_enter(scroll_lines(-60.0))
        .until(frames(2))
        .add()
        .step(format!("sketch: wheel out to the {} ceiling", view.slug()))
        .on_enter(scroll_lines(-60.0))
        .until(frames(3))
        .add()
        .step(format!(
            "sketch: the {} zoom stops at its reach",
            view.slug()
        ))
        .on_enter(move |world: &mut World| {
            world.remove_resource::<GestureProbe>();
            let limits = zoom_limits(world, view);
            let radius = pane_orbit(world, view).radius;
            assert!(
                (radius - limits.1).abs() < 1e-3,
                "wheeling far out must stop the {view:?} at {}, not {radius}",
                limits.1
            );
            info!(
                "sketch: {} zoom clamps at {radius:.1} (floor {:.1})",
                view.slug(),
                limits.0
            );
        })
        .add()
}

/// The NOVA OS zoom floor and ceiling a pane's wheel is held between.
#[cfg(feature = "debug")]
fn zoom_limits(world: &mut World, view: SketchView) -> (f32, f32) {
    match view {
        SketchView::Map => {
            let center = map_orbit(world).center;
            let reach = world
                .run_system_once(move |contacts: MapContacts| {
                    map_radius_max(map_spread(&contacts, center))
                })
                .expect("the contact model runs");
            (MAP_RADIUS_MIN, reach)
        }
        SketchView::Ship => (SHIP_RADIUS_MIN, SHIP_RADIUS_MAX),
        SketchView::Inventory => unreachable!("the hold has no zoom"),
    }
}

/// Wait for the view's pane, then assert what is on screen.
#[cfg(feature = "debug")]
fn verdict(script: Script, view: SketchView, context: SketchContext, when: &'static str) -> Script {
    let pane = view.pane();
    script
        .step(format!("sketch: {} is laid out ({when})", view.slug()))
        .until(and(
            ui_node_present(pane),
            and(scene_ready(view), frames(3)),
        ))
        .diagnose(ui_node_diagnosis(pane))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("sketch: {} is the only view ({when})", view.slug()))
        .on_enter(move |world: &mut World| assert_view(world, view, context, when))
        .add()
}

/// The view's 3D scene has its blips up; a view without one is ready at once.
#[cfg(feature = "debug")]
fn scene_ready(view: SketchView) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| match view {
        SketchView::Map => world
            .resource::<MapScene>()
            .0
            .as_ref()
            .is_some_and(|scene| !scene.blips.is_empty()),
        SketchView::Ship => world
            .resource::<ShipScene>()
            .0
            .as_ref()
            .is_some_and(|scene| !scene.blips.is_empty()),
        SketchView::Inventory => true,
    })
}

/// Shoot the window on the capture path; nothing otherwise.
#[cfg(feature = "debug")]
fn shot(script: Script, slug: &str) -> Script {
    if !capturing() {
        return script;
    }
    let file = format!("ui_app_variants-{slug}.png");
    let path = file.clone();
    script
        .step(format!("sketch: shoot {slug}"))
        .on_enter(move |world: &mut World| {
            hide_status_bar(world);
            shoot(world, &path);
        })
        .until(shot_written(file))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

#[cfg(feature = "debug")]
fn window_rect(world: &mut World) -> Rect {
    let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
    let window = windows.single(world).expect("one primary window");
    Rect::new(0.0, 0.0, window.width(), window.height())
}

/// One pane is up and inside the window, the other views' panes are gone,
/// exactly the view's own 3D scene is live and drawing at the pane's size,
/// the pane and scene keep their rects, the context line and the view's
/// controls match the fixture and context, the HUD is hidden, the cursor is
/// free, the simulation is still, and no node or text runs off screen.
#[cfg(feature = "debug")]
fn assert_view(world: &mut World, view: SketchView, context: SketchContext, when: &str) {
    assert_eq!(
        (
            *world.resource::<SketchView>(),
            *world.resource::<SketchContext>()
        ),
        (view, context),
        "the view and context resources disagree with the clicked controls ({when})"
    );
    assert_eq!(
        *world.resource::<HudVisibility>(),
        HudVisibility::Cinematic,
        "the flight HUD must stay hidden under the screens ({when})"
    );
    let cursor = world
        .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
        .single(world)
        .expect("one primary window");
    assert!(
        cursor.visible && cursor.grab_mode == CursorGrabMode::None,
        "the screens need a free, visible cursor, not {:?} visible {} ({when})",
        cursor.grab_mode,
        cursor.visible
    );
    let sources = world.resource::<ActionSources>();
    assert!(
        !sources.mouse_buttons && !sources.mouse_motion && !sources.mouse_wheel,
        "a pointer on the screens must not reach the flight and weapon actions ({when})"
    );
    assert_ship_unflown(world, when);
    assert_sim_still(world, when);
    let window = window_rect(world);
    for (other, _, _) in SketchView::ALL {
        let pane = other.pane();
        let count = count_named(world, pane);
        let expected = usize::from(other == view);
        assert_eq!(
            count, expected,
            "{pane} must exist {expected} time(s) on the {view:?} view ({when})"
        );
    }
    let rect = ui_node_rect(world, view.pane())
        .unwrap_or_else(|| panic!("{} is not laid out and visible ({when})", view.pane()));
    assert!(
        inside(rect, window),
        "{} runs out of a {}x{} window: {rect:?} ({when})",
        view.pane(),
        window.max.x,
        window.max.y
    );
    // The pane is the largest thing on screen, not a strip in a corner. The
    // context line takes its fixed share in every context.
    let share = rect.width() * rect.height() / (window.width() * window.height());
    assert!(
        share > 0.4,
        "{} covers only {:.0}% of the window ({when})",
        view.pane(),
        share * 100.0
    );
    let size = window.size();
    assert_steady(world, format!("{size} pane"), rect, when);
    let scene = match view {
        SketchView::Map => Some(MAP_SCENE),
        SketchView::Ship => Some(SHIP_SCENE),
        SketchView::Inventory => None,
    };
    if let Some(scene) = scene {
        let rect = ui_node_rect(world, scene)
            .unwrap_or_else(|| panic!("{scene} is not laid out ({when})"));
        assert_steady(world, format!("{size} {scene}"), rect, when);
    }
    assert_scenes(world, view, when);
    assert_dock(world, context, when);
    match view {
        SketchView::Map => {
            assert_map_controls(world, when);
            assert_map_legend(world, when);
        }
        SketchView::Ship => {
            assert_ship_marks(world, when);
            assert_repair(world, context, when);
        }
        SketchView::Inventory => {
            assert_holds(world, context, when);
            assert_inspector(world, context, when);
        }
    }
    assert_nothing_overflows(world, window, when);
    info!("sketch: {view:?} {context:?} verified ({when})");
}

/// `rect` matches the first rect seen under `key`, or becomes it.
#[cfg(feature = "debug")]
fn assert_steady(world: &mut World, key: String, rect: Rect, when: &str) {
    let mut seen = world.resource_mut::<SteadyRects>();
    match seen.0.iter().find(|(at, _)| *at == key) {
        Some((_, first)) => assert!(
            (first.min - rect.min).abs().max_element() < 0.5
                && (first.max - rect.max).abs().max_element() < 0.5,
            "{key} moved from {first:?} to {rect:?}: it must keep its place across views, \
             contexts and repairs ({when})"
        ),
        None => seen.0.push((key, rect)),
    }
}

/// The descendants of `root` that carry `T`.
#[cfg(feature = "debug")]
fn descendants_with<T: Component>(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if world.get::<T>(entity).is_some() {
            found.push(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    found
}

/// The one node called `name`.
#[cfg(feature = "debug")]
fn named(world: &mut World, name: &str) -> Entity {
    let found: Vec<Entity> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter(|(_, named)| named.as_str() == name)
        .map(|(entity, _)| entity)
        .collect();
    match found.as_slice() {
        [entity] => *entity,
        _ => panic!("{} nodes named `{name}`; want one", found.len()),
    }
}

/// The context line keeps its fixed height, holds no control, shows the
/// fixture credits and the last result, and names what the context is at.
#[cfg(feature = "debug")]
fn assert_dock(world: &mut World, context: SketchContext, when: &str) {
    let narrow = window_rect(world).width() < NARROW_BELOW_PX;
    let banner = ui_node_rect(world, DOCK_BANNER)
        .unwrap_or_else(|| panic!("the context line is not shown ({when})"));
    assert!(
        (banner.height() - banner_px(narrow)).abs() < 0.5,
        "the context line must keep its {} px height, not {} ({when})",
        banner_px(narrow),
        banner.height()
    );
    let banner_node = named(world, DOCK_BANNER);
    assert!(
        descendants_with::<Button>(world, banner_node).is_empty(),
        "the context line must hold no control ({when})"
    );
    let fixture = world.resource::<SketchFixture>().clone();
    assert_eq!(
        (
            named_text(world, DOCK_CREDITS),
            named_text(world, DOCK_NOTICE)
        ),
        (fixture.credits_text(), fixture.notice.clone()),
        "the context line must show the fixture credits and last result ({when})"
    );
    let (head, partner) = match context {
        SketchContext::Undocked => ("UNDOCKED", "No station or boarded ship"),
        SketchContext::Station => ("STATION", "Mock station  (fixture)"),
        SketchContext::Boarded => ("BOARDED", "Raider  block_gunship  (mock)"),
    };
    assert_eq!(
        (
            named_text(world, DOCK_HEAD),
            named_text(world, DOCK_PARTNER)
        ),
        (head.to_string(), partner.to_string()),
        "the context line must name the context and what the ship is at ({when})"
    );
}

/// The map holds no dock, trade, repair or transfer control: every button in
/// the body is Reframe or a contact blip.
#[cfg(feature = "debug")]
fn assert_map_controls(world: &mut World, when: &str) {
    let body = world
        .query_filtered::<Entity, With<SketchBody>>()
        .single(world)
        .expect("one sketch body");
    let strays: Vec<String> = descendants_with::<Button>(world, body)
        .into_iter()
        .filter(|&button| {
            world.get::<MapBlip>(button).is_none()
                && world.get::<Name>(button).map(Name::as_str) != Some(MAP_REFRAME)
        })
        .map(|button| {
            world
                .get::<Name>(button)
                .map_or_else(|| format!("{button}"), |name| name.to_string())
        })
        .collect();
    assert!(
        strays.is_empty(),
        "the map may hold only Reframe and contact blips, not {strays:?} ({when})"
    );
}

/// Every blip wears its body's icon in its stance colour, drawn from the
/// contact's own markers, and the legend names exactly the marks the map
/// plots, in rank order. This scenario plots the picket, the hauler, the
/// raider and two rocks.
#[cfg(feature = "debug")]
fn assert_map_legend(world: &mut World, when: &str) {
    let icons = world.resource::<SketchIcons>().bodies.clone();
    let blips: Vec<(Entity, MapMark, Vec<Entity>)> = world
        .query::<(&MapBlip, &Children)>()
        .iter(world)
        .map(|(blip, children)| (blip.contact, blip.mark, children.iter().collect()))
        .collect();
    let mut marks: Vec<MapMark> = Vec::new();
    for (contact, mark, children) in &blips {
        let ship = world.get::<SpaceshipRootMarker>(*contact).is_some();
        let rock = world.get::<AsteroidMarker>(*contact).is_some();
        let planet = world.get::<PlanetMarker>(*contact).is_some();
        let body = match (ship, rock, planet) {
            (true, false, false) => BodyIcon::Ship,
            (false, true, false) => BodyIcon::Asteroid,
            (false, false, true) => BodyIcon::Planet,
            _ => BodyIcon::Objective,
        };
        assert_eq!(
            mark.body, body,
            "a blip must wear the icon of its contact's own marker ({when})"
        );
        let drawn = children.iter().find_map(|child| {
            let image = world.get::<ImageNode>(*child)?;
            let tint = world.get::<ThemedImageTint>(*child)?;
            Some((image.image.clone(), tint.color))
        });
        assert_eq!(
            drawn,
            Some((icons[mark.body.index()].clone(), kind_color(mark.kind))),
            "a {} blip must show its icon in its stance colour ({when})",
            mark.label()
        );
        if !marks.contains(mark) {
            marks.push(*mark);
        }
    }
    marks.sort_by_key(|mark| mark.rank());
    assert_eq!(
        marks.iter().map(|mark| mark.label()).collect::<Vec<_>>(),
        ["Own ship", "Ally ship", "Hostile ship", "Asteroid"],
        "the map must plot the picket, the hauler, the raider and rocks ({when})"
    );
    let legend = named(world, MAP_LEGEND);
    let entries: Vec<String> = world
        .get::<Children>(legend)
        .map(|children| {
            children
                .iter()
                .filter_map(|child| {
                    world
                        .get::<Name>(child)
                        .map(|name| name.as_str().to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        entries,
        marks.iter().map(|mark| mark.key_name()).collect::<Vec<_>>(),
        "the legend must name every plotted mark and nothing else ({when})"
    );
    for mark in &marks {
        let key = mark.key_name();
        assert!(
            ui_node_rect(world, &key).is_some(),
            "{key} must be on screen ({when})"
        );
    }
}

/// The ship panel shows the selected section's icon, fixture condition and
/// bar in a fixed-height repair slot. At the station with the bay on, it
/// prices that section's repair; with the bay off, it says so; anywhere
/// else, it says repair needs a station.
#[cfg(feature = "debug")]
fn assert_repair(world: &mut World, context: SketchContext, when: &str) {
    let fixture = world.resource::<SketchFixture>().clone();
    let code = section_order(world)
        .0
        .unwrap_or_else(|| panic!("the ship view has a selection ({when})"));
    let pct = fixture.condition(&code);
    let status = named_text(world, SHIP_STATUS);
    assert!(
        status.contains(&format!("Condition {pct}%")) && status.ends_with(condition_status(pct)),
        "the panel must show {code}'s fixture condition {pct}%, not `{status}` ({when})"
    );
    let fill = ui_node_rect(world, SHIP_CONDITION)
        .unwrap_or_else(|| panic!("the condition bar is not shown ({when})"));
    let track = parent_rect(world, SHIP_CONDITION);
    assert!(
        (fill.width() - track.width() * pct as f32 / 100.0).abs() < 1.0,
        "the condition bar fills {} of {} px, not {pct}% ({when})",
        fill.width(),
        track.width()
    );
    let slot = ui_node_rect(world, SHIP_SERVICE)
        .unwrap_or_else(|| panic!("the repair slot is not shown ({when})"));
    assert!(
        (slot.height() - REPAIR_PX).abs() < 0.5,
        "the repair slot must keep its {REPAIR_PX} px height, not {} ({when})",
        slot.height()
    );
    let counts = (
        count_named(world, SHIP_REPAIR),
        count_named(world, SHIP_NO_REPAIR),
        count_named(world, SHIP_BAY),
    );
    match (context, fixture.repair_bay) {
        (SketchContext::Station, true) => {
            assert_eq!(counts, (1, 0, 1), "{code} must offer one repair ({when})");
            let price = fixture.repair_price(&code);
            let expected = if price == 0 {
                "Intact".to_string()
            } else {
                format!("Repair {code}  {price} cr")
            };
            assert_eq!(
                (
                    button_label(world, SHIP_REPAIR),
                    button_label(world, SHIP_BAY)
                ),
                (expected, "Repair bay on".to_string()),
                "the repair must be priced for {code} ({when})"
            );
        }
        (SketchContext::Station, false) => {
            assert_eq!(
                counts,
                (0, 1, 1),
                "with the bay off the station prices nothing ({when})"
            );
            assert_eq!(
                (
                    named_text(world, SHIP_NO_REPAIR),
                    button_label(world, SHIP_BAY)
                ),
                (
                    "The bay is off: no repair here".to_string(),
                    "Repair bay off".to_string()
                ),
                "the panel must say the bay is off ({when})"
            );
        }
        _ => {
            assert_eq!(
                counts,
                (0, 1, 0),
                "away from the station the ship offers no repair ({when})"
            );
            assert_eq!(
                named_text(world, SHIP_NO_REPAIR),
                "Repair at a station only",
                "the panel must say repair needs a station ({when})"
            );
        }
    }
}

/// Every section badge wears its family's icon and tint, every block its
/// family's material, the panel preview the selected family's icon, and the
/// bow arrow points past the foremost block along ship-local -Z.
#[cfg(feature = "debug")]
fn assert_ship_marks(world: &mut World, when: &str) {
    let views = world
        .run_system_once(|sections: ShipSections| sections.collect())
        .expect("the section model runs");
    let kinds: HashMap<Entity, SectionIcon> = views
        .iter()
        .map(|view| (view.entity, SectionIcon::of(view.kind)))
        .collect();
    let icons = world.resource::<SketchIcons>().sections.clone();
    let blocks = world.resource::<SceneMaterials>().blocks.clone();
    let mut badges = world.query::<(&ShipBlip, &Children)>();
    let badges: Vec<(Entity, Vec<Entity>)> = badges
        .iter(world)
        .map(|(blip, children)| (blip.section, children.iter().collect()))
        .collect();
    assert_eq!(
        badges.len(),
        views.len(),
        "every section needs a badge ({when})"
    );
    for (section, children) in &badges {
        let icon = kinds[section];
        let drawn = children.iter().find_map(|child| {
            let image = world.get::<ImageNode>(*child)?;
            let tint = world.get::<ThemedImageTint>(*child)?;
            Some((image.image.clone(), tint.color))
        });
        assert_eq!(
            drawn,
            Some((icons[icon.index()].clone(), icon.color())),
            "a {icon:?} badge must show the {icon:?} icon in its tint ({when})"
        );
    }
    let selected = world
        .resource::<ShipSelection>()
        .0
        .unwrap_or_else(|| panic!("the ship view has a selection ({when})"));
    let preview = named(world, SHIP_PREVIEW);
    let shown = (
        world
            .get::<ImageNode>(preview)
            .map(|node| node.image.clone()),
        world.get::<ThemedImageTint>(preview).map(|tint| tint.color),
    );
    let icon = kinds[&selected];
    assert_eq!(
        shown,
        (Some(icons[icon.index()].clone()), Some(icon.color())),
        "the panel preview must show the selected {icon:?} icon ({when})"
    );
    let mut families: Vec<SectionIcon> = Vec::new();
    let mut outlined = world.query::<(&BlockOutline, &ChildOf)>();
    let pairs: Vec<(Entity, Entity)> = outlined
        .iter(world)
        .map(|(outline, parent)| (outline.section, parent.parent()))
        .collect();
    for (section, block) in pairs {
        let icon = kinds[&section];
        let material = world
            .get::<MeshMaterial3d<StandardMaterial>>(block)
            .expect("a block has a material");
        assert_eq!(
            material.0,
            blocks[icon.index()],
            "a {icon:?} block must wear the {icon:?} tint ({when})"
        );
        if !families.contains(&icon) {
            families.push(icon);
        }
    }
    for (code, _) in WEAR {
        assert!(
            views.iter().any(|view| view.code == code),
            "fixture wear names {code}, which the picket does not have ({when})"
        );
    }
    assert!(
        families.len() >= 4,
        "the picket must show at least four section families, not {families:?} ({when})"
    );
    let front = views
        .iter()
        .map(|view| view.local.translation.z - view.half_extents.max_element())
        .fold(f32::INFINITY, f32::min);
    let arrow = world
        .query_filtered::<&Transform, With<BowArrow>>()
        .single(world)
        .unwrap_or_else(|_| panic!("the ship scene must draw one bow arrow ({when})"));
    // The cone's tip is its local +Y.
    let tip = arrow.rotation * Vec3::Y;
    assert!(
        arrow.translation.z < front && tip.z < -0.99,
        "the bow arrow must point along -Z past the hull ({} vs {front}), not {tip:?} ({when})",
        arrow.translation.z
    );
    info!("sketch: ship shows families {families:?} and the bow arrow ({when})");
}

/// The picket's hold, and the store the context adds, show the fixture: a
/// hold's load against its limit and a bar filled to match, the market's
/// prices, and one fixed-height row per line the filter shows, in order,
/// with name, quantity and mass or price. The marked filter chip is the
/// filter.
#[cfg(feature = "debug")]
fn assert_holds(world: &mut World, context: SketchContext, when: &str) {
    let fixture = world.resource::<SketchFixture>().clone();
    let filter = world.resource::<CargoFilter>().0;
    let shown: Vec<Store> = [Some(Store::Own), context.partner()]
        .into_iter()
        .flatten()
        .collect();
    for store in [Store::Own, Store::Market, Store::Boarded] {
        assert_eq!(
            count_named(world, &store.panel_name()),
            usize::from(shown.contains(&store)),
            "{} must show only in its context ({when})",
            store.title()
        );
    }
    let own = ui_node_rect(world, STORE_COLUMN_OWN).expect("the picket's column is laid out");
    let partner =
        ui_node_rect(world, STORE_COLUMN_PARTNER).expect("the partner column is laid out");
    assert!(
        (own.width() - partner.width()).abs() < 1.0,
        "the two store columns must share the row equally, filled or not: {} and {} px ({when})",
        own.width(),
        partner.width()
    );
    let size = window_rect(world).size();
    assert_steady(world, format!("{size} {STORE_COLUMN_OWN}"), own, when);
    let mut expected_rows = Vec::new();
    for &store in &shown {
        let panel = ui_node_rect(world, &store.panel_name())
            .unwrap_or_else(|| panic!("{} is not laid out ({when})", store.title()));
        assert!(
            (panel.width() - own.width()).abs() < 1.0,
            "{} must fill its column: {} of {} px ({when})",
            store.title(),
            panel.width(),
            own.width()
        );
        let hold = fixture.hold(store);
        assert_eq!(
            named_text(world, &store.load_name()),
            hold.map_or_else(|| "Fixture prices".to_string(), FixtureHold::load),
            "{} shows the wrong load ({when})",
            store.title()
        );
        match hold {
            Some(hold) => {
                let fill = ui_node_rect(world, &store.fill_name())
                    .unwrap_or_else(|| panic!("{} has no load bar ({when})", store.title()));
                let track = parent_rect(world, &store.fill_name());
                let share = hold.stock.mass_kg() as f32 / hold.capacity_kg as f32;
                assert!(
                    (fill.width() - track.width() * share).abs() < 1.0,
                    "{} fills {} of {} px, not {:.0}% ({when})",
                    store.title(),
                    fill.width(),
                    track.width(),
                    share * 100.0
                );
            }
            None => assert_eq!(count_named(world, &store.fill_name()), 0),
        }
        let mut above = f32::MIN;
        for line in fixture
            .stock(store)
            .0
            .iter()
            .filter(|line| filter.is_none_or(|category| line.good.category == category))
        {
            let row_name = store.row_name(line.good);
            let row = ui_node_rect(world, &row_name)
                .unwrap_or_else(|| panic!("{row_name} is not shown ({when})"));
            assert!(
                (row.height() - ROW_PX).abs() < 0.5 && row.min.y > above,
                "{row_name} must be a {ROW_PX} px row under the one before, not {row:?} ({when})"
            );
            above = row.min.y;
            let value = if hold.is_some() {
                tonnes(line.mass_kg())
            } else {
                format!("{} cr", line.good.price_cr)
            };
            assert_eq!(
                row_texts(world, &row_name, row, when),
                [
                    line.good.label.to_string(),
                    line.good.amount(line.qty),
                    value
                ],
                "{row_name} must show name, quantity and mass or price ({when})"
            );
            expected_rows.push((store, line.good.label));
        }
    }
    expected_rows.sort_by_key(|(store, label)| (store.tag(), *label));
    assert_eq!(
        cargo_rows(world),
        expected_rows,
        "the stores must draw exactly the filtered lines ({when})"
    );
    let marked: Vec<Option<Category>> = world
        .query::<(&FilterChip, &ThemedBorder)>()
        .iter(world)
        .filter(|(_, border)| border.alpha == 1.0)
        .map(|(chip, _)| chip.0)
        .collect();
    assert_eq!(
        marked,
        vec![filter],
        "exactly the current filter's chip must be marked ({when})"
    );
    assert_eq!(
        world.query::<&FilterChip>().iter(world).count(),
        Category::ALL.len() + 1,
        "the filter bar must offer All and every category ({when})"
    );
}

/// The inspector shows the inspected item's icon, name and category, then
/// either its facts, or the open confirmation of the deal the context allows
/// on it; with nothing inspected, only a hint.
#[cfg(feature = "debug")]
fn assert_inspector(world: &mut World, context: SketchContext, when: &str) {
    assert!(
        ui_node_rect(world, INSPECTOR).is_some(),
        "the inspector must be shown ({when})"
    );
    let inspected = world.resource::<Inspected>().0;
    let draft = world.resource::<Draft>().0;
    let shown = |world: &mut World, name: &str| ui_node_rect(world, name).is_some();
    let Some((store, good)) = inspected else {
        assert_eq!(
            (
                shown(world, INSPECT_HINT),
                shown(world, INSPECT_NAME),
                draft
            ),
            (true, false, None),
            "with nothing inspected the inspector shows only its hint ({when})"
        );
        return;
    };
    assert!(
        store == Store::Own || Some(store) == context.partner(),
        "the inspector must not show an item from a hidden store ({when})"
    );
    let icons = world.resource::<SketchIcons>().cargo.clone();
    let icon = named(world, INSPECT_ICON);
    assert_eq!(
        (
            world.get::<ImageNode>(icon).map(|node| node.image.clone()),
            world.get::<ThemedImageTint>(icon).map(|tint| tint.color),
            named_text(world, INSPECT_NAME),
            named_text(world, INSPECT_CATEGORY),
        ),
        (
            Some(icons[good.category.index()].clone()),
            Some(good.category.color()),
            good.label.to_string(),
            good.category.label().to_string(),
        ),
        "the inspector must show the item's icon, name and category ({when})"
    );
    let fixture = world.resource::<SketchFixture>().clone();
    assert_eq!(
        (
            shown(world, INSPECT_STOCK),
            named_text(world, INSPECT_STOCK)
        ),
        (
            true,
            format!(
                "{} in {}",
                good.amount(fixture.stock(store).qty(good)),
                store.title()
            )
        ),
        "the inspector must show the item's facts and live stock ({when})"
    );
    match draft {
        None => {
            assert!(
                !shown(world, DEAL_CONFIRM),
                "the inspector must show no confirmation ({when})"
            );
        }
        Some(open) => {
            assert_eq!(
                (shown(world, DEAL_CONFIRM), open.good),
                (true, good),
                "the confirmation must show under the facts of the selected item ({when})"
            );
            assert_eq!(
                Some(open.deal),
                Deal::offered(context, store),
                "the confirmation must hold the deal the context allows ({when})"
            );
            assert_eq!(
                named_text(world, DEAL_QTY),
                open.qty.map_or_else(String::new, |qty| good.amount(qty)),
                "the confirmation must show its quantity ({when})"
            );
        }
    }
}

/// The texts of a hold row, each checked to show inside it.
#[cfg(feature = "debug")]
fn row_texts(world: &mut World, row_name: &str, row: Rect, when: &str) -> Vec<String> {
    let children: Vec<Entity> = world
        .query::<(&Name, &Children)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == row_name)
        .map(|(_, children)| children.iter().collect())
        .expect("the row has its cells");
    let mut texts = Vec::new();
    for child in children {
        let Some(text) = world.get::<Text>(child).map(|text| text.0.clone()) else {
            continue;
        };
        let node = world.get::<ComputedNode>(child).expect("a laid-out cell");
        let at = world
            .get::<UiGlobalTransform>(child)
            .expect("a placed cell");
        let scale = node.inverse_scale_factor();
        let rect = Rect::from_center_size(at.translation * scale, node.size() * scale);
        assert!(
            node.size().x > 0.0 && inside(rect, row),
            "`{text}` must show inside {row_name}: {rect:?} in {row:?} ({when})"
        );
        texts.push(text);
    }
    texts
}

/// The laid-out rect of the parent of the node named `name`.
#[cfg(feature = "debug")]
fn parent_rect(world: &mut World, name: &str) -> Rect {
    let parent = world
        .query::<(&Name, &ChildOf)>()
        .iter(world)
        .find(|(named, _)| named.as_str() == name)
        .map(|(_, child_of)| child_of.parent())
        .unwrap_or_else(|| panic!("`{name}` has no parent"));
    let node = world
        .get::<ComputedNode>(parent)
        .expect("a laid-out parent");
    let at = world
        .get::<UiGlobalTransform>(parent)
        .expect("a placed parent");
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at.translation * scale, node.size() * scale)
}

/// Only the view's own scene is live: one root, one camera, and an image the
/// size of its pane. A scene left behind by the last view fails here.
#[cfg(feature = "debug")]
fn assert_scenes(world: &mut World, view: SketchView, when: &str) {
    let map_cameras = world
        .query_filtered::<(), With<MapCamera>>()
        .iter(world)
        .count();
    let ship_cameras = world
        .query_filtered::<(), With<ShipCamera>>()
        .iter(world)
        .count();
    let map = world.resource::<MapScene>().0.is_some();
    let ship = world.resource::<ShipScene>().0.is_some();
    assert_eq!(
        (map, map_cameras, ship, ship_cameras),
        match view {
            SketchView::Map => (true, 1, false, 0),
            SketchView::Ship => (false, 0, true, 1),
            SketchView::Inventory => (false, 0, false, 0),
        },
        "the {view:?} view must hold exactly its own scene ({when})"
    );
    let scene = match view {
        SketchView::Map => world.resource::<MapScene>().0.as_ref(),
        SketchView::Ship => world.resource::<ShipScene>().0.as_ref(),
        SketchView::Inventory => None,
    };
    let Some((host, image)) = scene.map(|scene| (scene.host, scene.image.clone())) else {
        return;
    };
    let pane = world
        .get::<ComputedNode>(host)
        .expect("the scene host is laid out")
        .size()
        .round()
        .as_uvec2();
    let drawn = world
        .resource::<Assets<Image>>()
        .get(&image)
        .expect("the scene image exists")
        .size();
    assert_eq!(
        drawn, pane,
        "the {view:?} scene must draw at its pane's size ({when})"
    );
    let shown = world.get::<ImageNode>(host).map(|node| node.image.clone());
    assert_eq!(
        shown,
        Some(image),
        "the {view:?} pane must show its scene ({when})"
    );
}

/// No laid-out node under the root leaves the window or its parent's box, and
/// no text is wider than its own box. Blips are skipped: a contact at the
/// image edge is allowed to straddle it, as on the NOVA OS map.
#[cfg(feature = "debug")]
fn assert_nothing_overflows(world: &mut World, window: Rect, when: &str) {
    let root = world
        .query_filtered::<Entity, With<SketchRoot>>()
        .single(world)
        .expect("the sketch root is up");
    let mut stack = vec![(root, window)];
    let mut offenders = Vec::new();
    while let Some((entity, parent)) = stack.pop() {
        if world.get::<MapBlip>(entity).is_some() || world.get::<ShipBlip>(entity).is_some() {
            continue;
        }
        let rect = match (
            world.get::<ComputedNode>(entity),
            world.get::<UiGlobalTransform>(entity),
        ) {
            (Some(node), Some(transform)) if node.size().x > 0.0 => {
                let scale = node.inverse_scale_factor();
                let rect =
                    Rect::from_center_size(transform.translation * scale, node.size() * scale);
                let label = world
                    .get::<Name>(entity)
                    .map(|name| name.to_string())
                    .or_else(|| {
                        world
                            .get::<Text>(entity)
                            .map(|text| format!("text `{}`", text.0))
                    })
                    .unwrap_or_else(|| format!("{entity}"));
                if !inside(rect, window) {
                    offenders.push(format!("{label} leaves the window: {rect:?}"));
                }
                if !inside(rect, parent) {
                    offenders.push(format!("{label} leaves its parent: {rect:?} in {parent:?}"));
                }
                if let Some(layout) = world.get::<bevy::text::TextLayoutInfo>(entity) {
                    let width = layout.size.x * scale;
                    if width > rect.width() + 1.0 {
                        offenders.push(format!(
                            "{label} text {width} wider than its box {}",
                            rect.width()
                        ));
                    }
                }
                rect
            }
            _ => parent,
        };
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().map(|child| (child, rect)));
        }
    }
    assert!(
        offenders.is_empty(),
        "{} node(s) overflow in a {}x{} window ({when}): {offenders:#?}",
        offenders.len(),
        window.max.x,
        window.max.y
    );
}

/// Whether `rect` sits inside `outer`, give or take a pixel of rounding.
#[cfg(feature = "debug")]
fn inside(rect: Rect, outer: Rect) -> bool {
    rect.min.x >= outer.min.x - 1.0
        && rect.min.y >= outer.min.y - 1.0
        && rect.max.x <= outer.max.x + 1.0
        && rect.max.y <= outer.max.y + 1.0
}

/// What the walk measured before the theme flip, so the step after it can
/// tell a repaint in place from a rebuild.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct FlipProbe {
    root: Entity,
    block: Color,
}

#[cfg(feature = "debug")]
fn scene_root(world: &mut World, view: SketchView) -> Entity {
    let scene = match view {
        SketchView::Map => world.resource::<MapScene>().0.as_ref(),
        SketchView::Ship => world.resource::<ShipScene>().0.as_ref(),
        SketchView::Inventory => None,
    };
    scene
        .map(|scene| scene.root)
        .unwrap_or_else(|| panic!("the {view:?} view has no scene"))
}

/// The colour the hull blocks are drawn in.
#[cfg(feature = "debug")]
fn block_color(world: &mut World) -> Color {
    let handle = world.resource::<SceneMaterials>().blocks[SectionIcon::Hull.index()].clone();
    world
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .expect("the block material exists")
        .base_color
}

/// Exactly one block outline carries the selection material, and it is the
/// selected section's.
#[cfg(feature = "debug")]
fn assert_selected_outline(world: &mut World) {
    let selected = world
        .resource::<ShipSelection>()
        .0
        .expect("a section is selected");
    let wanted = world.resource::<SceneMaterials>().outline_selected.clone();
    let marked: Vec<Entity> = world
        .query::<(&BlockOutline, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter(|(_, material)| material.0 == wanted)
        .map(|(outline, _)| outline.section)
        .collect();
    assert_eq!(
        marked,
        vec![selected],
        "only the selected section's outline must be marked"
    );
}

/// The one text node called `name`.
#[cfg(feature = "debug")]
fn named_text(world: &mut World, name: &str) -> String {
    let texts: Vec<String> = world
        .query::<(&Name, &Text)>()
        .iter(world)
        .filter(|(n, _)| n.as_str() == name)
        .map(|(_, text)| text.0.clone())
        .collect();
    match texts.as_slice() {
        [text] => text.clone(),
        _ => panic!("{} text nodes named `{name}`; want one", texts.len()),
    }
}

/// How many nodes are called `name`.
#[cfg(feature = "debug")]
fn count_named(world: &mut World, name: &str) -> usize {
    world
        .query::<&Name>()
        .iter(world)
        .filter(|named| named.as_str() == name)
        .count()
}

/// The label of the one button called `name`.
#[cfg(feature = "debug")]
fn button_label(world: &mut World, name: &str) -> String {
    let children: Vec<Entity> = world
        .query::<(&Name, &Children)>()
        .iter(world)
        .find(|(named, _)| named.as_str() == name)
        .map(|(_, children)| children.iter().collect())
        .unwrap_or_else(|| panic!("no button `{name}`"));
    children
        .into_iter()
        .find_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
        .unwrap_or_else(|| panic!("button `{name}` has no label"))
}
