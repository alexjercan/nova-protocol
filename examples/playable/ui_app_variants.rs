//! ui_app_variants: a clickable, themed sketch of map, ship, inventory and
//! docked screens in normal themed UI instead of the NOVA OS CRT.
//!
//! It is a SANDBOX. Every value on screen is example-local sample data: no
//! live ship, scanner, cargo or station state exists behind it, and no
//! production crate knows it is here. The Undocked/Docked control flips a local
//! flag, not a dock joint. Station services and cargo actions are drawn and
//! disabled in both contexts, because no service runtime exists.
//!
//! The app is the real one: `AppBuilder`, an empty scenario, the mod themes,
//! the shared widget factories, and a live Phosphor/Hardware repaint. NOVA OS
//! and its TAB binding are left as they are. TAB opens NOVA OS only when a
//! player ship exists, and this scenario has none, so TAB does nothing here.
//!
//! ```text
//! cargo run --example ui_app_variants --features debug
//! ```
//!
//! Harnessed modes:
//! - `NOVA_AUTOPILOT=1`: click through every tab, the dock context and the
//!   theme, assert the visible pane and the disabled actions, check that the
//!   theme flip repaints without a rebuild, repeat at a narrow window size,
//!   and exit.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>`: the same walk,
//!   plus a frame of every view in both themes at desktop size and in Hardware
//!   at the narrow size.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use nova_ui::{
    prelude::*,
    theme::{UiColor, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
    widget::{ButtonSpec, ThemedBorder, ThemedFill, ThemedText, UiText},
};

#[derive(Parser)]
#[command(name = "ui_app_variants")]
#[command(version = "1.0.0")]
#[command(
    about = "Click through themed map, ship, inventory and docked UI sketches",
    long_about = None
)]
struct Cli;

/// The empty scenario the sketch sits over.
const SCENARIO_ID: &str = "ui_app_variants_sandbox";

// The `Name`s the harness clicks and asserts by.
const TAB_MAP: &str = "Sketch Tab Map";
const TAB_SHIP: &str = "Sketch Tab Ship";
const TAB_INVENTORY: &str = "Sketch Tab Inventory";
const TAB_DOCKED: &str = "Sketch Tab Docked";
const CONTEXT_UNDOCKED: &str = "Sketch Context Undocked";
const CONTEXT_DOCKED: &str = "Sketch Context Docked";
const THEME_PHOSPHOR: &str = "Sketch Theme Phosphor";
const THEME_HARDWARE: &str = "Sketch Theme Hardware";
const PANE_CHART: &str = "Sketch Pane Chart";
const PANE_CONTACTS: &str = "Sketch Pane Contacts";
const PANE_HULL: &str = "Sketch Pane Hull";
const PANE_PART: &str = "Sketch Pane Part";
const PANE_HOLD: &str = "Sketch Pane Hold";
const PANE_MANIFEST: &str = "Sketch Pane Manifest";
const PANE_STATION: &str = "Sketch Pane Station";
const PART_TITLE: &str = "Sketch Part Title";
const DOCK_STATUS: &str = "Sketch Dock Status";

/// Which screen the sketch shows.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SketchView {
    #[default]
    Map,
    Ship,
    Inventory,
    Docked,
}

impl SketchView {
    const ALL: [(Self, &'static str, &'static str); 4] = [
        (Self::Map, "Map", TAB_MAP),
        (Self::Ship, "Ship", TAB_SHIP),
        (Self::Inventory, "Inventory", TAB_INVENTORY),
        (Self::Docked, "Docked", TAB_DOCKED),
    ];

    /// The panes this view must show, and every other view must not.
    #[cfg(feature = "debug")]
    fn panes(self) -> &'static [&'static str] {
        match self {
            Self::Map => &[PANE_CHART, PANE_CONTACTS],
            Self::Ship => &[PANE_HULL, PANE_PART],
            Self::Inventory => &[PANE_HOLD, PANE_MANIFEST],
            Self::Docked => &[PANE_STATION],
        }
    }

    #[cfg(feature = "debug")]
    fn slug(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Ship => "ship",
            Self::Inventory => "inventory",
            Self::Docked => "docked",
        }
    }
}

/// Whether the sketch draws the ship as docked. A local flag only: nothing
/// docks.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SketchContext {
    #[default]
    Undocked,
    Docked,
}

/// The ship component the hull schematic has selected.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SketchPart {
    Bridge,
    #[default]
    Reactor,
    Railgun,
    CargoBay,
    DockPort,
    Thrusters,
}

/// Sample detail for one ship component.
struct PartSheet {
    title: &'static str,
    name: &'static str,
    role: &'static str,
    integrity: f32,
    status: (BadgeKind, &'static str),
    stats: [(&'static str, &'static str); 3],
}

impl SketchPart {
    fn sheet(self) -> PartSheet {
        match self {
            Self::Bridge => PartSheet {
                title: "Bridge",
                name: "Sketch Part Bridge",
                role: "Helm and sensors",
                integrity: 1.0,
                status: (BadgeKind::Green, "online"),
                stats: [("Mass", "18 t"), ("Sensor range", "40 km"), ("Crew", "2")],
            },
            Self::Reactor => PartSheet {
                title: "Reactor",
                name: "Sketch Part Reactor",
                role: "Main power",
                integrity: 0.92,
                status: (BadgeKind::Green, "nominal"),
                stats: [("Mass", "34 t"), ("Output", "48 MW"), ("Heat", "61 %")],
            },
            Self::Railgun => PartSheet {
                title: "Railgun",
                name: "Sketch Part Railgun",
                role: "Spinal kinetic weapon",
                integrity: 0.88,
                status: (BadgeKind::Amber, "low ammo"),
                stats: [("Mass", "22 t"), ("Slugs", "12 / 40"), ("Charge", "3.2 s")],
            },
            Self::CargoBay => PartSheet {
                title: "Cargo bay",
                name: "Sketch Part Cargo Bay",
                role: "32-slot hold",
                integrity: 0.97,
                status: (BadgeKind::Blue, "14 / 32"),
                stats: [("Mass", "40 t"), ("Load", "11.6 t"), ("Slots", "14 / 32")],
            },
            Self::DockPort => PartSheet {
                title: "Dock port",
                name: "Sketch Part Dock Port",
                role: "Bow clamp",
                integrity: 1.0,
                status: (BadgeKind::Mute, "free"),
                stats: [("Mass", "6 t"), ("Clamp", "open"), ("Seal", "ok")],
            },
            Self::Thrusters => PartSheet {
                title: "Thrusters",
                name: "Sketch Part Thrusters",
                role: "Main drive",
                integrity: 0.62,
                status: (BadgeKind::Red, "damaged"),
                stats: [("Mass", "28 t"), ("Thrust", "310 kN"), ("Loss", "38 %")],
            },
        }
    }
}

/// The full-screen sketch root; the top bar under it is never rebuilt.
#[derive(Component)]
struct SketchRoot;

/// The view body, rebuilt whenever the view, the context or the part changes.
#[derive(Component)]
struct SketchBody;

fn main() -> AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(sketch_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(sketch_script());
        if capturing() {
            app.add_systems(Startup, hide_dev_overlays);
        }
    }

    app.run()
}

fn sketch_plugin(app: &mut App) {
    app.init_resource::<SketchView>()
        .init_resource::<SketchContext>()
        .init_resource::<SketchPart>();
    // Every control drives its resource through the path the game's Settings
    // use. With its own game plugins this app has no menu, and the menu is
    // what registers the theme observer in the game.
    app.add_observer(button_on_setting::<SelectedUiTheme>)
        .add_observer(button_on_setting::<SketchView>)
        .add_observer(button_on_setting::<SketchContext>)
        .add_observer(button_on_setting::<SketchPart>);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_sandbox);
    app.add_systems(OnEnter(GameStates::Playing), spawn_root);
    app.add_systems(Update, rebuild_body.run_if(in_state(GameStates::Playing)));
}

fn load_sandbox(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "An empty session under the UI sketch".to_string(),
        ..ScenarioConfig::new(
            SCENARIO_ID,
            "UI App Variants",
            game_assets.cubemap.clone().into(),
        )
    }));
}

fn spawn_root(
    mut commands: Commands,
    view: Res<SketchView>,
    context: Res<SketchContext>,
    theme: Res<SelectedUiTheme>,
) {
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
                padding: UiRect::all(px(20)),
                row_gap: px(16),
                ..default()
            },
            // Above the flight HUD, below every NOVA OS layer.
            GlobalZIndex(MENU_PANEL_Z),
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColor::Void),
        ))
        .with_children(|root| top_bar(root, view, context, &theme));
}

/// Title, view tabs, dock context and theme. Built once; `button_on_setting`
/// moves each group's mark.
fn top_bar(root: &mut ChildSpawnerCommands, view: SketchView, context: SketchContext, theme: &str) {
    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: px(16),
        row_gap: px(10),
        width: percent(100),
        ..default()
    })
    .with_children(|bar| {
        text(bar, "NOVA", 18.0, UiColor::Primary);
        bar.spawn(segmented_container()).with_children(|seg| {
            for (value, label, name) in SketchView::ALL {
                let mut option =
                    seg.spawn((segmented_option(label), ButtonValue(value), Name::new(name)));
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
            for (value, label, name) in [
                (SketchContext::Undocked, "Undocked", CONTEXT_UNDOCKED),
                (SketchContext::Docked, "Docked", CONTEXT_DOCKED),
            ] {
                let mut option =
                    seg.spawn((segmented_option(label), ButtonValue(value), Name::new(name)));
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
                ));
                if id == theme {
                    option.insert(Selected);
                }
            }
        });
    });
}

/// Respawn the body for the current view, context and part. A theme change
/// never comes through here: every widget repaints itself.
fn rebuild_body(
    mut commands: Commands,
    view: Res<SketchView>,
    context: Res<SketchContext>,
    part: Res<SketchPart>,
    roots: Query<Entity, With<SketchRoot>>,
    new_roots: Query<(), Added<SketchRoot>>,
    bodies: Query<Entity, With<SketchBody>>,
) {
    let changed = view.is_changed() || context.is_changed() || part.is_changed();
    if !changed && new_roots.is_empty() {
        return;
    }
    let Ok(root) = roots.single() else {
        return;
    };
    for body in &bodies {
        commands.entity(body).despawn();
    }
    let (view, context, part) = (*view, *context, *part);
    commands.entity(root).with_children(|root| {
        root.spawn((
            SketchBody,
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                column_gap: px(16),
                row_gap: px(16),
                width: percent(100),
                ..default()
            },
        ))
        .with_children(|body| match view {
            SketchView::Map => map_view(body),
            SketchView::Ship => ship_view(body, part, context),
            SketchView::Inventory => inventory_view(body, context),
            SketchView::Docked => docked_view(body, context),
        });
    });
}

// Map.

/// Chart edge in logical px: three sectors across.
const CHART_PX: f32 = 432.0;
/// One sector edge in logical px.
const SECTOR_PX: f32 = CHART_PX / 3.0;
/// One sector edge in meters, as `world_sectors` streams them.
const SECTOR_M: f32 = 32_000.0;
/// Ship heading in degrees clockwise from chart up.
const HEADING_DEG: f32 = 40.0;

/// A sample contact: label, kind, range in meters, bearing in degrees.
struct Contact {
    label: &'static str,
    kind: &'static str,
    badge: BadgeKind,
    color: UiColor,
    range_m: f32,
    bearing_deg: f32,
}

const CONTACTS: [Contact; 5] = [
    Contact {
        label: "Tollgate Relay",
        kind: "station",
        badge: BadgeKind::Blue,
        color: UiColor::Info,
        range_m: 18_000.0,
        bearing_deg: 300.0,
    },
    Contact {
        label: "K-7 Field",
        kind: "rocks",
        badge: BadgeKind::Mute,
        color: UiColor::Secondary,
        range_m: 9_000.0,
        bearing_deg: 250.0,
    },
    Contact {
        label: "Heron",
        kind: "raider",
        badge: BadgeKind::Red,
        color: UiColor::Danger,
        range_m: 19_000.0,
        bearing_deg: 115.0,
    },
    Contact {
        label: "Marrow",
        kind: "derelict",
        badge: BadgeKind::Amber,
        color: UiColor::Accent,
        range_m: 27_000.0,
        bearing_deg: 320.0,
    },
    Contact {
        label: "Beacon B-12",
        kind: "beacon",
        badge: BadgeKind::Green,
        color: UiColor::Nominal,
        range_m: 36_000.0,
        bearing_deg: 190.0,
    },
];

/// Chart position of a range and bearing around the chart centre.
fn chart_point(range_m: f32, bearing_deg: f32) -> Vec2 {
    let radius = range_m / SECTOR_M * SECTOR_PX;
    let (sin, cos) = bearing_deg.to_radians().sin_cos();
    Vec2::new(CHART_PX / 2.0 + radius * sin, CHART_PX / 2.0 - radius * cos)
}

fn map_view(body: &mut ChildSpawnerCommands) {
    card(body, PANE_CHART, "Chart", Some("S 4,2"), |c| {
        c.spawn((
            Node {
                width: px(CHART_PX),
                height: px(CHART_PX),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Surface, 0.4),
        ))
        .with_children(chart);
        text(
            c,
            "Rings at 10 and 20 km. Up is north.",
            11.0,
            UiColor::Secondary,
        );
    });
    card(body, PANE_CONTACTS, "Contacts", Some("5"), |c| {
        for contact in &CONTACTS {
            c.spawn(list_row()).with_children(|row| {
                row.spawn(badge(contact.badge, contact.kind));
                row.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|name| text(name, contact.label, 13.0, UiColor::Body));
                text(
                    row,
                    &format!("{:.0} km", contact.range_m / 1000.0),
                    12.0,
                    UiColor::Secondary,
                );
            });
        }
    });
}

fn chart(chart: &mut ChildSpawnerCommands) {
    // Sector boundaries, and each sector's grid label.
    for i in 1..3 {
        let at = SECTOR_PX * i as f32;
        line(chart, px(at), px(0), px(1), percent(100));
        line(chart, px(0), px(at), percent(100), px(1));
    }
    for row in 0..3 {
        for col in 0..3 {
            chart
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(SECTOR_PX * col as f32 + 6.0),
                    top: px(SECTOR_PX * row as f32 + 4.0),
                    ..default()
                })
                .with_children(|at| {
                    text(
                        at,
                        &format!("{},{}", 3 + col, 1 + row),
                        10.0,
                        UiColor::Label,
                    )
                });
        }
    }
    // Range rings.
    for range_m in [10_000.0, 20_000.0] {
        let radius = range_m / SECTOR_M * SECTOR_PX;
        chart.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(CHART_PX / 2.0 - radius),
                top: px(CHART_PX / 2.0 - radius),
                width: px(radius * 2.0),
                height: px(radius * 2.0),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Secondary, 0.5),
        ));
    }
    // The forward mark: a line out of the ship along its heading. Rotated
    // about its own centre, so the centre sits half a length out.
    let length = 56.0;
    let (sin, cos) = HEADING_DEG.to_radians().sin_cos();
    let mid = Vec2::new(
        CHART_PX / 2.0 + sin * length / 2.0,
        CHART_PX / 2.0 - cos * length / 2.0,
    );
    chart.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(mid.x - 1.0),
            top: px(mid.y - length / 2.0),
            width: px(2),
            height: px(length),
            ..default()
        },
        UiTransform::from_rotation(Rot2::degrees(HEADING_DEG)),
        BackgroundColor(Color::NONE),
        ThemedFill::new(UiColor::Primary),
    ));
    let tip = Vec2::new(
        CHART_PX / 2.0 + sin * (length + 10.0),
        CHART_PX / 2.0 - cos * (length + 10.0),
    );
    chart
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(tip.x - 4.0),
            top: px(tip.y - 8.0),
            ..default()
        })
        .with_children(|at| text(at, "FWD", 10.0, UiColor::Primary));
    marker(chart, Vec2::splat(CHART_PX / 2.0), 10.0, UiColor::Primary);
    for contact in &CONTACTS {
        let at = chart_point(contact.range_m, contact.bearing_deg);
        marker(chart, at, 8.0, contact.color);
        chart
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(at.x + 8.0),
                top: px(at.y - 7.0),
                ..default()
            })
            .with_children(|label| text(label, contact.label, 11.0, contact.color));
    }
}

/// A thin themed rule at an absolute place in the chart.
fn line(chart: &mut ChildSpawnerCommands, left: Val, top: Val, width: Val, height: Val) {
    chart.spawn((
        Node {
            position_type: PositionType::Absolute,
            left,
            top,
            width,
            height,
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.45),
    ));
}

/// A square contact mark centred on `at`.
fn marker(chart: &mut ChildSpawnerCommands, at: Vec2, size: f32, color: UiColor) {
    chart.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x - size / 2.0),
            top: px(at.y - size / 2.0),
            width: px(size),
            height: px(size),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::new(color),
    ));
}

// Ship.

fn ship_view(body: &mut ChildSpawnerCommands, part: SketchPart, context: SketchContext) {
    card(body, PANE_HULL, "Hull", Some("Cargoa"), |c| {
        c.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(8),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(24)),
                width: px(440),
                ..default()
            },
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Secondary, 0.5),
        ))
        .with_children(|hull| {
            hull_row(hull, &[(SketchPart::Bridge, 130.0)], part);
            hull_row(
                hull,
                &[
                    (SketchPart::Railgun, 124.0),
                    (SketchPart::Reactor, 124.0),
                    (SketchPart::DockPort, 124.0),
                ],
                part,
            );
            hull_row(hull, &[(SketchPart::CargoBay, 240.0)], part);
            hull_row(hull, &[(SketchPart::Thrusters, 180.0)], part);
        });
        text(c, "Click a component.", 11.0, UiColor::Secondary);
    });
    let sheet = part.sheet();
    card(body, PANE_PART, "Component", None, |c| {
        c.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            ..default()
        })
        .with_children(|head| {
            head.spawn((
                Name::new(PART_TITLE),
                UiText,
                Text::new(sheet.title),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Primary),
            ));
            head.spawn(badge(sheet.status.0, sheet.status.1));
        });
        text(c, sheet.role, 12.0, UiColor::Secondary);
        text(
            c,
            &format!("Integrity {:.0} %", sheet.integrity * 100.0),
            11.0,
            UiColor::Label,
        );
        c.spawn(slider_track(sheet.integrity));
        for (key, value) in sheet.stats {
            stat_row(c, key, value);
        }
        disabled_action(c, "Repair", "Sketch Action Repair");
        text(c, dock_note(context), 11.0, UiColor::Secondary);
    });
}

fn hull_row(hull: &mut ChildSpawnerCommands, parts: &[(SketchPart, f32)], selected: SketchPart) {
    hull.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(8),
        ..default()
    })
    .with_children(|row| {
        for &(part, width) in parts {
            let sheet = part.sheet();
            row.spawn(Node {
                width: px(width),
                ..default()
            })
            .with_children(|slot| {
                let mut button = slot.spawn((
                    button(ButtonSpec::new(sheet.title).block()),
                    ButtonValue(part),
                    Name::new(sheet.name),
                ));
                if part == selected {
                    button.insert(Selected);
                }
            });
        }
    });
}

// Inventory.

/// Hold size in slots.
const HOLD_COLS: u32 = 8;
const HOLD_ROWS: u32 = 4;
/// One slot edge and the gap between slots, in logical px.
const SLOT_PX: f32 = 44.0;
const SLOT_GAP: f32 = 4.0;

/// A sample cargo stack: label, amount, colour, and its slot box.
struct Stack {
    label: &'static str,
    amount: &'static str,
    color: UiColor,
    at: (u32, u32),
    size: (u32, u32),
}

const STACKS: [Stack; 7] = [
    Stack {
        label: "Nickel ore",
        amount: "4.0 t",
        color: UiColor::Accent,
        at: (0, 0),
        size: (2, 2),
    },
    Stack {
        label: "Ice",
        amount: "2.2 t",
        color: UiColor::Info,
        at: (2, 0),
        size: (2, 1),
    },
    Stack {
        label: "Hull plate",
        amount: "x3",
        color: UiColor::Nominal,
        at: (2, 1),
        size: (2, 1),
    },
    Stack {
        label: "Slugs",
        amount: "x12",
        color: UiColor::Danger,
        at: (4, 0),
        size: (1, 2),
    },
    Stack {
        label: "Core",
        amount: "x1",
        color: UiColor::AccentHigh,
        at: (5, 0),
        size: (1, 1),
    },
    Stack {
        label: "Fuel",
        amount: "x2",
        color: UiColor::Primary,
        at: (5, 1),
        size: (1, 1),
    },
    Stack {
        label: "Scrap",
        amount: "1.8 t",
        color: UiColor::Secondary,
        at: (0, 2),
        size: (2, 1),
    },
];

impl Stack {
    fn covers(&self, col: u32, row: u32) -> bool {
        (self.at.0..self.at.0 + self.size.0).contains(&col)
            && (self.at.1..self.at.1 + self.size.1).contains(&row)
    }
}

fn slot_span(cells: u32) -> f32 {
    cells as f32 * SLOT_PX + cells.saturating_sub(1) as f32 * SLOT_GAP
}

fn slot_origin(cell: u32) -> f32 {
    cell as f32 * (SLOT_PX + SLOT_GAP)
}

fn inventory_view(body: &mut ChildSpawnerCommands, context: SketchContext) {
    let used: u32 = STACKS.iter().map(|s| s.size.0 * s.size.1).sum();
    let slots = HOLD_COLS * HOLD_ROWS;
    card(
        body,
        PANE_HOLD,
        "Hold",
        Some(&format!("{used} / {slots}")),
        |c| {
            c.spawn(Node {
                width: px(slot_span(HOLD_COLS)),
                height: px(slot_span(HOLD_ROWS)),
                ..default()
            })
            .with_children(|grid| {
                for row in 0..HOLD_ROWS {
                    for col in 0..HOLD_COLS {
                        if STACKS.iter().any(|stack| stack.covers(col, row)) {
                            continue;
                        }
                        grid.spawn((
                            slot_node(col, row, (1, 1)),
                            BorderColor::all(Color::NONE),
                            ThemedBorder::alpha(UiColor::Secondary, 0.3),
                        ));
                    }
                }
                for stack in &STACKS {
                    grid.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::all(px(5)),
                            ..slot_node(stack.at.0, stack.at.1, stack.size)
                        },
                        BackgroundColor(Color::NONE),
                        ThemedFill::alpha(stack.color, 0.22),
                        BorderColor::all(Color::NONE),
                        ThemedBorder::new(stack.color),
                    ))
                    .with_children(|cell| {
                        text(cell, stack.label, 10.0, UiColor::Body);
                        text(cell, stack.amount, 10.0, stack.color);
                    });
                }
            });
        },
    );
    card(body, PANE_MANIFEST, "Manifest", None, |c| {
        text(
            c,
            &format!("Hold {used} / {slots} slots"),
            11.0,
            UiColor::Label,
        );
        c.spawn(slider_track(used as f32 / slots as f32));
        stat_row(c, "Load", "11.6 t");
        for stack in &STACKS {
            stat_row(c, stack.label, stack.amount);
        }
        disabled_action(c, "Transfer", "Sketch Action Transfer");
        disabled_action(c, "Sell", "Sketch Action Sell");
        text(c, dock_note(context), 11.0, UiColor::Secondary);
    });
}

fn slot_node(col: u32, row: u32, size: (u32, u32)) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(slot_origin(col)),
        top: px(slot_origin(row)),
        width: px(slot_span(size.0)),
        height: px(slot_span(size.1)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(3)),
        ..default()
    }
}

// Docked.

/// A station service card: name, action label, one line.
const SERVICES: [(&str, &str, &str); 5] = [
    ("Repair", "Sketch Service Repair", "Restore hull integrity."),
    ("Rearm", "Sketch Service Rearm", "Refill rail slugs."),
    ("Trade", "Sketch Service Trade", "Sell cargo, buy parts."),
    ("Refit", "Sketch Service Refit", "Swap ship sections."),
    ("Storage", "Sketch Service Storage", "Leave cargo here."),
];

fn docked_view(body: &mut ChildSpawnerCommands, context: SketchContext) {
    let tag = match context {
        SketchContext::Docked => "visual only",
        SketchContext::Undocked => "offline",
    };
    card(body, PANE_STATION, "Station", Some(tag), |c| {
        c.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            ..default()
        })
        .with_children(|status| {
            let (kind, label, line) = match context {
                SketchContext::Docked => (BadgeKind::Green, "docked", "Tollgate Relay, berth 2"),
                SketchContext::Undocked => (
                    BadgeKind::Mute,
                    "no dock link",
                    "Dock at a station to use services.",
                ),
            };
            status.spawn(badge(kind, label));
            status.spawn((
                Name::new(DOCK_STATUS),
                UiText,
                Text::new(line),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Body),
            ));
        });
        c.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(10),
            row_gap: px(10),
            max_width: px(660),
            ..default()
        })
        .with_children(|grid| {
            for (label, name, line) in SERVICES {
                grid.spawn((
                    Node {
                        width: px(200),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8),
                        padding: UiRect::all(px(12)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BorderColor::all(Color::NONE),
                    ThemedBorder::alpha(UiColor::Secondary, 0.5),
                ))
                .with_children(|service| {
                    text(service, label, 14.0, UiColor::Primary);
                    text(service, line, 11.0, UiColor::Secondary);
                    disabled_action(service, label, name);
                });
            }
        });
    });
}

fn dock_note(context: SketchContext) -> &'static str {
    match context {
        SketchContext::Undocked => "Dock to use this.",
        SketchContext::Docked => "Visual only: no service runtime.",
    }
}

// Shared pieces.

/// A titled panel with a padded column body, named for the harness.
fn card(
    body: &mut ChildSpawnerCommands,
    name: &str,
    title: &str,
    tag: Option<&str>,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    body.spawn((Name::new(name.to_string()), panel_node(), panel()))
        .with_children(|card| {
            card.spawn(panel_head(title, tag));
            card.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(14)),
                ..default()
            })
            .with_children(build);
        });
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: UiColor) {
    parent.spawn((
        UiText,
        Text::new(value.to_string()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
    ));
}

fn stat_row(parent: &mut ChildSpawnerCommands, key: &str, value: &str) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(24),
            min_width: px(240),
            ..default()
        })
        .with_children(|row| {
            text(row, key, 12.0, UiColor::Label);
            text(row, value, 12.0, UiColor::Body);
        });
}

/// A drawn action that cannot be used: no service runtime exists.
fn disabled_action(parent: &mut ChildSpawnerCommands, label: &str, name: &str) {
    parent.spawn((
        button(ButtonSpec::new(label)),
        bevy::ui::InteractionDisabled,
        Name::new(name.to_string()),
    ));
}

// Harness.

/// Desktop and narrow window sizes, in logical px.
#[cfg(feature = "debug")]
const DESKTOP: Vec2 = Vec2::new(1600.0, 900.0);
#[cfg(feature = "debug")]
const NARROW: Vec2 = Vec2::new(720.0, 1000.0);

/// Seconds the harness gives the assets and the empty scenario to come up.
#[cfg(feature = "debug")]
const LOAD_DEADLINE: f32 = 60.0;

/// Every disabled action per view, by name.
#[cfg(feature = "debug")]
fn disabled_actions(view: SketchView) -> Vec<&'static str> {
    match view {
        SketchView::Map => Vec::new(),
        SketchView::Ship => vec!["Sketch Action Repair"],
        SketchView::Inventory => vec!["Sketch Action Transfer", "Sketch Action Sell"],
        SketchView::Docked => SERVICES.iter().map(|(_, name, _)| *name).collect(),
    }
}

/// The driven walk. Every view, context and theme change is a real click on
/// the named control; every verdict reads the live tree.
#[cfg(feature = "debug")]
fn sketch_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("sketch: the tabs are up")
        .until(and(state_is(GameStates::Playing), ui_node_present(TAB_MAP)))
        .diagnose(ui_node_diagnosis(TAB_MAP))
        .deadline(LOAD_DEADLINE)
        .add();
    script = resize(script, DESKTOP, "desktop");
    script = verdict(script, SketchView::Map, "desktop");
    script = shot(script, "phosphor-map");

    script = open(script, SketchView::Ship);
    script = verdict(script, SketchView::Ship, "desktop");
    script = script
        .click_named(
            "sketch: select the thrusters",
            SketchPart::Thrusters.sheet().name,
            resource_where::<SketchPart>(|part| *part == SketchPart::Thrusters),
            BEAT_DEADLINE_SECS,
        )
        .step("sketch: the thrusters sheet is up")
        .until(and(ui_node_present(PART_TITLE), frames(2)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sketch: the detail follows the selection")
        .on_enter(|world: &mut World| {
            let title = named_text(world, PART_TITLE);
            assert_eq!(
                title, "Thrusters",
                "clicking the thrusters must show their sheet, not {title}"
            );
            assert_selected_part(world, SketchPart::Thrusters);
            info!("sketch: part detail is {title}");
        })
        .add();
    script = shot(script, "phosphor-ship");

    script = open(script, SketchView::Inventory);
    script = verdict(script, SketchView::Inventory, "desktop");
    script = shot(script, "phosphor-inventory");

    script = open(script, SketchView::Docked);
    script = verdict(script, SketchView::Docked, "desktop undocked");
    script = script
        .step("sketch: the status line says there is no dock")
        .on_enter(|world: &mut World| {
            let status = named_text(world, DOCK_STATUS);
            assert!(
                status.starts_with("Dock at a station"),
                "undocked, the station pane must say there is no dock, not `{status}`"
            );
        })
        .add();
    script = shot(script, "phosphor-docked-undocked");

    script = script.click_named(
        "sketch: switch to docked",
        CONTEXT_DOCKED,
        resource_where::<SketchContext>(|context| *context == SketchContext::Docked),
        BEAT_DEADLINE_SECS,
    );
    script = verdict(script, SketchView::Docked, "desktop docked");
    script = script
        .step("sketch: the status line names the station")
        .on_enter(|world: &mut World| {
            let status = named_text(world, DOCK_STATUS);
            assert!(
                status.starts_with("Tollgate Relay"),
                "docked, the station pane must name the station, not `{status}`"
            );
        })
        .add();
    script = shot(script, "phosphor-docked");

    script = script
        .step("sketch: record the body before the theme flip")
        .on_enter(|world: &mut World| {
            let probe = FlipProbe {
                body: body_entity(world),
                station_fill: station_fill(world),
            };
            world.insert_resource(probe);
        })
        .add()
        .click_named(
            "sketch: switch to hardware",
            THEME_HARDWARE,
            resource_where::<SelectedUiTheme>(|theme| theme.0 == HARDWARE_THEME_ID),
            BEAT_DEADLINE_SECS,
        );
    script = verdict(script, SketchView::Docked, "after the theme flip");
    script = script
        .step("sketch: the theme repainted in place")
        .on_enter(|world: &mut World| {
            let (body, fill) = {
                let probe = world.resource::<FlipProbe>();
                (probe.body, probe.station_fill)
            };
            assert_eq!(
                body_entity(world),
                body,
                "a theme flip must repaint the widgets, not rebuild the body"
            );
            let now = station_fill(world);
            assert_ne!(
                now, fill,
                "the station panel must repaint for the new theme (still {now:?})"
            );
            info!("sketch: theme repainted in place ({fill:?} -> {now:?})");
        })
        .add();
    script = shot(script, "hardware-docked");
    for view in [SketchView::Map, SketchView::Ship, SketchView::Inventory] {
        script = open(script, view);
        script = verdict(script, view, "desktop hardware");
        script = shot(script, &format!("hardware-{}", view.slug()));
    }

    script = resize(script, NARROW, "narrow");
    for view in [
        SketchView::Inventory,
        SketchView::Ship,
        SketchView::Map,
        SketchView::Docked,
    ] {
        if view != SketchView::Inventory {
            script = open(script, view);
        }
        script = verdict(script, view, "narrow");
        script = shot(script, &format!("narrow-{}", view.slug()));
    }
    script
}

/// Reshape the window and wait for the layout to take it.
#[cfg(feature = "debug")]
fn resize(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    size: Vec2,
    label: &str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("sketch: {label} window"))
        .on_enter(move |world: &mut World| {
            let mut windows =
                world.query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>();
            let mut window = windows.single_mut(world).expect("one primary window");
            window.resolution.set(size.x, size.y);
        })
        .until(and(window_size_is(size.x, size.y), frames(4)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

/// Click the tab for `view` and wait for the resource to take it.
#[cfg(feature = "debug")]
fn open(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    view: SketchView,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let (_, label, name) = SketchView::ALL
        .into_iter()
        .find(|(value, _, _)| *value == view)
        .expect("every view has a tab");
    script.click_named(
        &format!("sketch: open {label}"),
        name,
        resource_where::<SketchView>(move |current| *current == view),
        BEAT_DEADLINE_SECS,
    )
}

/// Wait for the view's panes, then assert what is on screen.
#[cfg(feature = "debug")]
fn verdict(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    view: SketchView,
    when: &'static str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let pane = view.panes()[0];
    script
        .step(format!(
            "sketch: {} panes are laid out ({when})",
            view.slug()
        ))
        .until(and(ui_node_present(pane), frames(3)))
        .diagnose(ui_node_diagnosis(pane))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("sketch: {} is the only view ({when})", view.slug()))
        .on_enter(move |world: &mut World| assert_view(world, view, when))
        .add()
}

/// Shoot the window on the capture path; nothing otherwise.
#[cfg(feature = "debug")]
fn shot(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    slug: &str,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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

/// The view's panes are laid out and inside the window, no other view's pane
/// exists, its actions are disabled, and no node or text runs off screen.
#[cfg(feature = "debug")]
fn assert_view(world: &mut World, view: SketchView, when: &str) {
    assert_eq!(
        *world.resource::<SketchView>(),
        view,
        "the view resource disagrees with the clicked tab ({when})"
    );
    let window = {
        let mut windows = world.query_filtered::<&Window, With<bevy::window::PrimaryWindow>>();
        let window = windows.single(world).expect("one primary window");
        Rect::new(0.0, 0.0, window.width(), window.height())
    };
    for (other, _, _) in SketchView::ALL {
        for pane in other.panes() {
            let count = world
                .query::<&Name>()
                .iter(world)
                .filter(|name| name.as_str() == *pane)
                .count();
            let expected = usize::from(other == view);
            assert_eq!(
                count, expected,
                "{pane} must exist {expected} time(s) on the {view:?} view ({when})"
            );
        }
    }
    for pane in view.panes() {
        let rect = ui_node_rect(world, pane)
            .unwrap_or_else(|| panic!("{pane} is not laid out and visible ({when})"));
        assert!(
            inside(rect, window),
            "{pane} runs out of a {}x{} window: {rect:?} ({when})",
            window.max.x,
            window.max.y
        );
    }
    for action in disabled_actions(view) {
        let mut query = world.query::<(&Name, Has<bevy::ui::InteractionDisabled>)>();
        let disabled: Vec<bool> = query
            .iter(world)
            .filter(|(name, _)| name.as_str() == action)
            .map(|(_, disabled)| disabled)
            .collect();
        assert_eq!(
            disabled,
            vec![true],
            "{action} must be drawn once and disabled ({when})"
        );
    }
    assert_nothing_overflows(world, window, when);
    info!("sketch: {view:?} verified ({when})");
}

/// No laid-out node under the root leaves the window or its parent's box, and
/// no text is wider than its own box.
///
/// The parent check is what catches a flex row whose measured height is
/// shorter than what it draws: the siblings below it are then placed outside
/// the panel they belong to.
#[cfg(feature = "debug")]
fn assert_nothing_overflows(world: &mut World, window: Rect, when: &str) {
    let root = world
        .query_filtered::<Entity, With<SketchRoot>>()
        .single(world)
        .expect("the sketch root is up");
    let mut stack = vec![(root, window)];
    let mut offenders = Vec::new();
    while let Some((entity, parent)) = stack.pop() {
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
    body: Entity,
    station_fill: Color,
}

/// The one live view body.
#[cfg(feature = "debug")]
fn body_entity(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<SketchBody>>()
        .single(world)
        .expect("exactly one view body")
}

/// The station pane's panel fill, which the panel reconciler repaints for the
/// live theme.
#[cfg(feature = "debug")]
fn station_fill(world: &mut World) -> Color {
    let fills: Vec<Color> = world
        .query::<(&Name, &BackgroundColor)>()
        .iter(world)
        .filter(|(name, _)| name.as_str() == PANE_STATION)
        .map(|(_, fill)| fill.0)
        .collect();
    match fills.as_slice() {
        [fill] => *fill,
        _ => panic!("{} station panes; want one", fills.len()),
    }
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

/// Exactly one part button carries the mark, and it is `part`'s.
#[cfg(feature = "debug")]
fn assert_selected_part(world: &mut World, part: SketchPart) {
    let marked: Vec<SketchPart> = world
        .query_filtered::<&ButtonValue<SketchPart>, With<Selected>>()
        .iter(world)
        .map(|value| value.0)
        .collect();
    assert_eq!(
        marked,
        vec![part],
        "the hull must mark only the selected component"
    );
}
