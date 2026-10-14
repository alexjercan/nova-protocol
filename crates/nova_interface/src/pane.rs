//! The TAB interface: one themed full-screen surface over the frozen world,
//! with a Map pane, a Ship pane and an Inventory pane.
//!
//! TAB opens it from flight (`Unpaused -> Interface`) and closes it again.
//! Keyboard M and gamepad Y step Map -> Ship -> Inventory -> Map, and the
//! Map, Ship and Inventory buttons in the card's title row pick a pane. The
//! first open shows Map; after that TAB reopens the last pane. The `:` command
//! modal opens over the pane without tearing it down, so closing the modal
//! returns to the same pane with its scene, camera and selection.
//!
//! Escape is not read here: the one back-out owner is
//! [`close_surface_from_menu_keys`](crate::terminal), so a single press never
//! closes both the modal and the pane.
//!
//! Touch this module when changing how the interface opens, which pane shows,
//! or the layout around the panes.

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_gameplay::{
    prelude::{
        AudioRoute, PlayerSpaceshipMarker, SfxCommandsExt, SoundBank, UiSfx, MENU_SELECT_VOLUME,
        UI_TOGGLE_VOLUME,
    },
    PauseStates,
};
use nova_input::prelude::{ActionContext, ActiveContexts, InputBindings, InputSources};
use nova_ui::{
    prelude::*,
    theme::UiColor,
    widget::{ThemedBorder, ThemedFill, ThemedText, UiText},
};

use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    inventory::inventory_body,
    map::{
        map_chart_button, on_map_chart_button, on_map_reframe_button, spawn_map_panel,
        MapLegendMarker, MapViewportMarker,
    },
    ship::{
        on_ship_fit_button, on_ship_reset_button, on_ship_viewport_click, on_ship_viewport_enter,
        on_ship_viewport_leave, on_ship_viewport_move, spawn_ship_panel, ShipRuntime,
        ShipViewportMarker,
    },
    recycle::{recycle_body, RecycleDock},
    station::{station_body, StationDock},
    terminal::NovaOsCloseTransition,
};

/// Which pane the TAB interface shows.
///
/// A resource, not a state: the pane is remembered while the interface is
/// closed, and it must survive the `:` modal opening over it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InterfacePaneType {
    /// The local-space map: contacts, range and bearing, and GOTO.
    #[default]
    Map,
    /// The player ship: sections, repair, rebind and mates.
    Ship,
    /// What the player ship carries, and what a docked ship carries.
    Inventory,
    /// What the docked station sells: ships, escorts and the station itself.
    /// Its tab shows only while the player ship is docked at a living station.
    Station,
    /// Strip and loot the docked wreck. Its tab shows only while the player
    /// ship is docked with a hull it may loot that is not a living station,
    /// so it never shows beside Station.
    Recycle,
}

impl InterfacePaneType {
    /// Every pane, in title-row order, with its button label.
    const ALL: [(Self, &'static str); 5] = [
        (Self::Map, "Map"),
        (Self::Ship, "Ship"),
        (Self::Inventory, "Inventory"),
        (Self::Station, "Station"),
        (Self::Recycle, "Recycle"),
    ];

    /// The per-pane action context id its verbs are bound under.
    pub(crate) fn context_id(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Ship => "ship",
            Self::Inventory => "inventory",
            Self::Station => "station",
            Self::Recycle => "recycle",
        }
    }

    /// The pane `interface_next_tab` steps to: Map -> Ship -> Inventory ->
    /// the `docked` tab, if one shows, -> Map.
    fn next(self, docked: Option<Self>) -> Self {
        match self {
            Self::Map => Self::Ship,
            Self::Ship => Self::Inventory,
            Self::Inventory => docked.unwrap_or(Self::Map),
            Self::Station | Self::Recycle => Self::Map,
        }
    }

    /// Whether the pane's tab shows only while docked.
    fn docked_only(self) -> bool {
        matches!(self, Self::Station | Self::Recycle)
    }

    /// The pane's title and button label.
    fn label(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(pane, _)| *pane == self)
            .map(|(_, label)| *label)
            .expect("InterfacePaneType::ALL lists every pane")
    }
}

/// The full-screen interface root. [`rebuild_interface_body`] builds the card
/// under it once and replaces only the pane body on a switch.
#[derive(Component)]
pub(crate) struct InterfaceRootMarker;

/// `interface_toggle` opens the interface from flight and closes it again.
///
/// Opening needs a ship: `Playing` also covers the editor's build mode, and an
/// interface over a scene with no ship would have nothing to show. Closing is
/// ungated by the ship, but an armed section rebind owns the next key, so it
/// refuses the toggle rather than lose the capture. Under the command modal the
/// desk key is the prompt's completion, so `Commands` is left alone.
pub(crate) fn toggle_interface(
    sources: InputSources,
    bindings: Res<InputBindings>,
    player: Query<(), With<PlayerSpaceshipMarker>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    ship: Option<Res<ShipRuntime>>,
) {
    let Some(action) = bindings.get("interface_toggle") else {
        return;
    };
    if !sources.just_pressed(action) {
        return;
    }
    if ship.is_some_and(|ship| ship.rebind_armed()) {
        return;
    }
    match current.get() {
        PauseStates::Unpaused if !player.is_empty() => next.set(PauseStates::Interface),
        PauseStates::Interface => next.set(PauseStates::Unpaused),
        PauseStates::Unpaused | PauseStates::Paused | PauseStates::Commands => {}
    }
}

/// Play the pause overlay's toggle blip as the interface opens from flight or
/// closes back to it.
///
/// Registered on those two transitions, not fired by the toggles: TAB and the
/// Escape back-out can both close the interface in one frame, and one
/// transition plays one cue. The command modal's transitions keep its own
/// power sweeps. The bank is absent on headless rigs, where the cue is a no-op.
pub(crate) fn play_interface_toggle(mut commands: Commands, bank: Option<Res<SoundBank<UiSfx>>>) {
    let Some(bank) = bank else {
        return;
    };
    commands.play_sfx(
        bank.get(UiSfx::UiToggle),
        AudioRoute::Interface,
        UI_TOGGLE_VOLUME,
    );
}

/// `interface_next_tab` steps to the next pane while the interface owns the
/// screen. An armed section rebind takes the key instead: leaving the Ship pane
/// would drop the capture. The plugin runs it in [`InputMode::Normal`] only, so
/// a focused text field, such as the Inventory quantity, types the key instead.
///
/// [`InputMode::Normal`]: nova_ui::prelude::InputMode::Normal
pub(crate) fn next_interface_pane(
    sources: InputSources,
    bindings: Res<InputBindings>,
    current: Res<State<PauseStates>>,
    mut pane: ResMut<InterfacePaneType>,
    ship: Option<Res<ShipRuntime>>,
    dock: DockedTab,
) {
    if *current.get() != PauseStates::Interface {
        return;
    }
    if ship.is_some_and(|ship| ship.rebind_armed()) {
        return;
    }
    let Some(action) = bindings.get("interface_next_tab") else {
        return;
    };
    if !sources.just_pressed(action) {
        return;
    }
    *pane = pane.next(dock.tab());
}

/// The docked-only tab that shows now: Station at a living station, Recycle
/// at a hull the player may loot, or none.
#[derive(SystemParam)]
pub(crate) struct DockedTab<'w, 's> {
    station: StationDock<'w, 's>,
    recycle: RecycleDock<'w, 's>,
}

impl DockedTab<'_, '_> {
    /// The docked-only pane whose tab shows, if any. The two conditions never
    /// hold together; a living station wins if they ever did.
    pub(crate) fn tab(&self) -> Option<InterfacePaneType> {
        if self.station.docked() {
            Some(InterfacePaneType::Station)
        } else if self.recycle.docked() {
            Some(InterfacePaneType::Recycle)
        } else {
            None
        }
    }
}

/// Leave a docked-only pane whose tab no longer shows for the Ship pane.
/// Runs before the body is built, so the card never opens on a pane it
/// cannot fill.
pub(crate) fn leave_docked_pane(dock: DockedTab, mut pane: ResMut<InterfacePaneType>) {
    if pane.docked_only() && dock.tab() != Some(*pane) {
        *pane = InterfacePaneType::Ship;
    }
}

/// Show a docked-only tab only while [`DockedTab`] names it. Runs after the
/// card is built, so a new card's tabs are right on its first frame.
pub(crate) fn show_docked_tabs(
    dock: DockedTab,
    mut q_tab: Query<(&ButtonValue<InterfacePaneType>, &mut Node)>,
) {
    let shown = dock.tab();
    for (value, mut node) in &mut q_tab {
        if !value.0.docked_only() {
            continue;
        }
        let display = if shown == Some(value.0) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

/// Whether the interface is on screen: open, or covered by the command modal
/// that will return to it.
pub(crate) fn interface_shown(pause: &State<PauseStates>, close: &NovaOsCloseTransition) -> bool {
    match pause.get() {
        PauseStates::Interface => true,
        PauseStates::Commands => close.return_to == PauseStates::Interface,
        PauseStates::Unpaused | PauseStates::Paused => false,
    }
}

/// Spawn the interface root when the interface comes on screen, and despawn it
/// when it goes. The root outlives the command modal over it, so the pane's
/// scene is not rebuilt on the way back.
pub(crate) fn spawn_interface_root(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    q_root: Query<Entity, With<InterfaceRootMarker>>,
) {
    let shown = interface_shown(&pause, &close);
    let roots: Vec<Entity> = q_root.iter().collect();
    assert!(
        roots.len() <= 1,
        "the TAB interface has {} roots; spawn_interface_root owns the only one",
        roots.len()
    );
    match (shown, roots.first()) {
        (true, None) => {
            commands.spawn((
                InterfaceRootMarker,
                Name::new("InterfaceRoot"),
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
                // Above the flight HUD, below the command modal. No fill: the
                // card is the surface, and the frozen world shows around it.
                GlobalZIndex(MENU_PANEL_Z),
            ));
        }
        (false, Some(&root)) => commands.entity(root).despawn(),
        _ => {}
    }
}

#[cfg(test)]
mod tests;

/// Widest the interface grows, in logical px.
const INTERFACE_MAX_PX: f32 = 1520.0;

/// The card's title row, which holds the pane title and the pane tabs.
#[derive(Component)]
pub(crate) struct InterfacePaneHead;

/// The node the shown pane's body is built in. Only its children are replaced
/// on a pane switch.
#[derive(Component)]
pub(crate) struct InterfacePaneBody;

/// Build the card on a fresh root, and on a pane change replace only the
/// body's children.
///
/// The card, its title row and the pane tabs are spawned once per root. A
/// switch rewrites the title text and moves [`Selected`] to the shown pane's
/// tab in place, writing only a difference: a tab click has already moved it
/// through `button_on_setting`, a keyboard or pad switch has not.
pub(crate) fn rebuild_interface_body(
    mut commands: Commands,
    pane: Res<InterfacePaneType>,
    icons: Res<InterfaceIcons>,
    q_root: Query<(Entity, Ref<InterfaceRootMarker>)>,
    q_body: Query<Entity, With<InterfacePaneBody>>,
    q_head: Query<&Children, With<InterfacePaneHead>>,
    mut q_title: Query<&mut Text, With<PanelHeadTitle>>,
    q_tab: Query<(Entity, &ButtonValue<InterfacePaneType>, Has<Selected>)>,
) {
    let Ok((root, marker)) = q_root.single() else {
        return;
    };
    if marker.is_added() {
        spawn_interface_card(&mut commands, root, *pane, &icons);
        return;
    }
    if !pane.is_changed() {
        return;
    }
    let pane = *pane;
    let body = q_body
        .single()
        .expect("an interface root that is not new has its card and pane body");
    commands.entity(body).despawn_children();
    commands
        .entity(body)
        .with_children(|body| pane_body(body, pane, &icons));

    let title = pane.label().to_uppercase();
    for child in q_head
        .single()
        .expect("an interface root that is not new has its title row")
    {
        if let Ok(mut text) = q_title.get_mut(*child) {
            if text.0 != title {
                text.0.clone_from(&title);
            }
        }
    }
    for (tab, value, selected) in &q_tab {
        match (value.0 == pane, selected) {
            (true, false) => {
                commands.entity(tab).insert(Selected);
            }
            (false, true) => {
                commands.entity(tab).remove::<Selected>();
            }
            _ => {}
        }
    }
}

/// The card under `root`: the title row with the pane tabs, and the body of
/// `pane`.
fn spawn_interface_card(
    commands: &mut Commands,
    root: Entity,
    pane: InterfacePaneType,
    icons: &InterfaceIcons,
) {
    commands.entity(root).with_children(|root| {
        root.spawn((
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                max_width: px(INTERFACE_MAX_PX),
                ..panel_node()
            },
            panel(),
        ))
        .with_children(|card| {
            // TODO: bevy_ui 0.19.0 `clip_check_recursive` clips a node only by
            // its direct parent, so a scrolled side panel's nested rows stay
            // pickable past the panel's top edge and take tab clicks. The
            // `ZIndex` stacks the head above the body for picking. Remove it
            // when bevy_ui is at 0.19.1 or later, which walks every ancestor.
            card.spawn((InterfacePaneHead, ZIndex(1), panel_head(pane.label(), None)))
                .with_children(|head| {
                    head.spawn(segmented_container()).with_children(|seg| {
                        for (value, label) in InterfacePaneType::ALL {
                            let mut option = seg.spawn((
                                segmented_option(label),
                                ButtonValue(value),
                                Name::new(format!("InterfaceTab{label}")),
                            ));
                            option.observe(on_interface_tab_click);
                            if value == pane {
                                option.insert(Selected);
                            }
                        }
                    });
                });
            card.spawn((
                InterfacePaneBody,
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: UiRect::all(px(12)),
                    ..default()
                },
            ))
            .with_children(|body| pane_body(body, pane, icons));
        });
    });
}

/// The body of `pane`.
fn pane_body(body: &mut ChildSpawnerCommands, pane: InterfacePaneType, icons: &InterfaceIcons) {
    match pane {
        InterfacePaneType::Map => map_body(body, icons),
        InterfacePaneType::Ship => ship_body(body, icons),
        InterfacePaneType::Inventory => inventory_body(body, icons),
        InterfacePaneType::Station => station_body(body),
        InterfacePaneType::Recycle => recycle_body(body),
    }
}

/// Click once when a tab that is not the shown pane is activated. The
/// shown pane's tab carries [`Selected`] until the switch's commands apply, so
/// a click on it changes nothing and stays silent.
fn on_interface_tab_click(
    activate: On<Activate>,
    q_tab: Query<Has<Selected>, With<ButtonValue<InterfacePaneType>>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if q_tab.get(activate.entity) == Ok(false) {
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Play the interface's click, if the sound bank has loaded.
pub(crate) fn play_menu_select(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>) {
    if let Some(bank) = bank {
        commands.play_sfx(
            bank.get(UiSfx::MenuSelect),
            AudioRoute::Interface,
            MENU_SELECT_VOLUME,
        );
    }
}

/// A pane's 3D view: the node its scene image fills and its blips ride on.
/// It takes what the side panel leaves of the row.
fn viewport_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        min_height: px(0),
        position_type: PositionType::Relative,
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The row a viewer pane splits into its view and its side panel.
fn view_split() -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    }
}

/// A row of controls.
pub(crate) fn control_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(12),
        flex_shrink: 0.0,
        ..default()
    }
}

/// Least width of a viewer pane's side panel, in logical px: the widest note,
/// the section rebind prompt, fits on one line at this width. A small window
/// keeps this width rather than the panel's 20% share.
const SIDE_PANEL_MIN_PX: f32 = 300.0;

/// Side of the preview frame at the head of a side panel, in logical px.
pub(crate) const PANEL_PREVIEW_PX: f32 = 96.0;

/// A viewer pane's side panel: 20% of the row beside the view, never
/// narrower than [`SIDE_PANEL_MIN_PX`], whose content past its height takes
/// `overflow`.
pub(crate) fn side_panel(overflow: Overflow) -> impl Bundle {
    (
        Node {
            width: percent(20),
            min_width: px(SIDE_PANEL_MIN_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow,
            ..default()
        },
        panel_paint(),
    )
}

/// The fill and border of a pane's panel.
pub(crate) fn panel_paint() -> impl Bundle {
    (
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.08),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Secondary),
    )
}

/// The framed square at the head of a side panel that holds the selection's
/// icon.
pub(crate) fn panel_preview_frame() -> impl Bundle {
    (
        Node {
            width: px(PANEL_PREVIEW_PX),
            height: px(PANEL_PREVIEW_PX),
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

/// Lines an action form's summary reserves, so a one-line result and a
/// two-line refusal leave the form's action button in place. A longer refusal
/// still shows in full.
pub(crate) const SUMMARY_LINES: f32 = 3.0;

/// A one-pixel rule between groups of a panel.
pub(crate) fn divider() -> impl Bundle {
    (
        Node {
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.35),
    )
}

/// A button sized for a narrow side panel or inspector.
pub(crate) fn compact_button(spec: ButtonSpec) -> impl Bundle {
    button(ButtonSpec {
        min_height: 22.0,
        font_size: 12.0,
        ..spec
    })
}

/// Height of the Map and Ship footer, in logical px. Fixed, so a legend or a
/// hint that wraps cannot push the view up.
const PANE_FOOTER_PX: f32 = 96.0;

/// The footer under the Map and Ship panes: the legend on the left, the view
/// controls and the input hints in the centre, and the selection summary on
/// the right.
#[derive(Component)]
pub(crate) struct InterfacePaneFooter;

/// One footer hint of a key a pane answers to, read from the live
/// [`InputBindings`] by [`refresh_pane_input_hints`]: a verb and the actions
/// whose keyboard binds it names. One node per hint, so a wrapping row breaks
/// between hints and never between a key and its verb.
#[derive(Component)]
pub(crate) struct PaneInputHint {
    verb: &'static str,
    actions: &'static [&'static str],
}

/// The keys the Map pane answers to. Every name must be registered.
const MAP_KEY_HINTS: &[(&str, &[&str])] = &[
    ("Turn", &["viewer_orbit_left", "viewer_orbit_right"]),
    ("Tilt", &["viewer_orbit_up", "viewer_orbit_down"]),
    (
        "Pan",
        &[
            "viewer_pan_forward",
            "viewer_pan_left",
            "viewer_pan_back",
            "viewer_pan_right",
        ],
    ),
    ("Reframe", &["viewer_reframe"]),
    ("Select", &["viewer_prev", "viewer_next"]),
    ("GOTO", &["map_goto"]),
    ("Chart", &["map_chart"]),
];

/// The keys the Ship pane answers to. Every name must be registered.
const SHIP_KEY_HINTS: &[(&str, &[&str])] = &[
    ("Turn", &["viewer_orbit_left", "viewer_orbit_right"]),
    ("Tilt", &["viewer_orbit_up", "viewer_orbit_down"]),
    ("Reset", &["viewer_reframe"]),
    ("Select", &["viewer_prev", "viewer_next"]),
    ("Mates", &["ship_mates"]),
    ("Repair", &["ship_repair"]),
    ("Rebind", &["ship_rebind"]),
];

/// The pointer gestures both viewers read raw, outside [`InputBindings`]:
/// a left click picks, the right button drags the orbit and the wheel zooms.
const POINTER_HINTS: [&str; 3] = ["Click select", "Right-drag look", "Wheel zoom"];

/// The footer under a viewer pane: legend left, live key hints center, and
/// clickable view controls right. Selection identity stays in the side panel.
fn pane_footer(
    body: &mut ChildSpawnerCommands,
    legend: impl FnOnce(&mut ChildSpawnerCommands),
    controls: impl FnOnce(&mut ChildSpawnerCommands),
    keys: &'static [(&'static str, &'static [&'static str])],
) {
    let side = || Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        height: percent(100),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        align_content: AlignContent::Center,
        column_gap: px(12),
        row_gap: px(6),
        overflow: Overflow::clip(),
        ..default()
    };
    body.spawn((
        InterfacePaneFooter,
        Node {
            height: px(PANE_FOOTER_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(16),
            padding: UiRect::axes(px(12), px(8)),
            border: UiRect::top(px(1)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.5),
    ))
    .with_children(|footer| {
        footer.spawn(side()).with_children(legend);
        footer
            .spawn(Node {
                flex_shrink: 1.0,
                min_width: px(0),
                max_width: percent(50),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            })
            .with_children(|centre| {
                centre
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        column_gap: px(12),
                        row_gap: px(2),
                        ..default()
                    })
                    .with_children(|hints| {
                        for &(verb, actions) in keys {
                            hints.spawn((
                                PaneInputHint { verb, actions },
                                themed_label("", 12.0, UiColor::Body),
                            ));
                        }
                        for pointer in POINTER_HINTS {
                            hints.spawn(themed_label(pointer, 12.0, UiColor::Label));
                        }
                    });
            });
        footer
            .spawn(Node {
                justify_content: JustifyContent::FlexEnd,
                ..side()
            })
            .with_children(|right| {
                right
                    .spawn(control_row(JustifyContent::FlexEnd))
                    .with_children(controls);
            });
    });
}

/// Write each footer hint from the live bindings when the hint is new or the
/// bindings change, so a rebind moves the hint with the key.
///
/// # Panics
///
/// When a hint names an action [`InputBindings`] does not register: the hint
/// tables and [`interface_bindings`](crate::bindings::interface_bindings) are
/// one vocabulary, and a hint for a missing action would name no key.
pub(crate) fn refresh_pane_input_hints(
    bindings: Res<InputBindings>,
    mut q_hint: Query<(Ref<PaneInputHint>, &mut Text)>,
) {
    for (hint, mut text) in &mut q_hint {
        if !hint.is_added() && !bindings.is_changed() {
            continue;
        }
        let keys = hint
            .actions
            .iter()
            .map(|name| {
                bindings
                    .get(name)
                    .unwrap_or_else(|| {
                        panic!("footer hint names `{name}`, which is not registered")
                    })
                    .keyboard_display()
            })
            .collect::<Vec<_>>()
            .join("/");
        let value = format!("{keys} {}", hint.verb);
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// The map: the scene and contact panel beside it, then legend, hints, Reframe
/// and Chart.
fn map_body(body: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    body.spawn(view_split()).with_children(|split| {
        split.spawn((MapViewportMarker, viewport_node(), ImageNode::default()));
        spawn_map_panel(split, icons);
    });
    pane_footer(
        body,
        |legend| {
            legend.spawn((
                MapLegendMarker,
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    row_gap: px(6),
                    ..default()
                },
            ));
        },
        |controls| {
            controls
                .spawn((
                    button(ButtonSpec::new("Reframe").fit()),
                    Name::new("MapReframe"),
                ))
                .observe(on_map_reframe_button);
            controls
                .spawn(map_chart_button())
                .observe(on_map_chart_button);
        },
        MAP_KEY_HINTS,
    );
}

/// The ship: the scene and section panel beside it, then legend, hints and view controls.
fn ship_body(body: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    body.spawn(view_split()).with_children(|split| {
        split
            .spawn((
                ShipViewportMarker,
                Name::new("ShipViewport"),
                viewport_node(),
                ImageNode::default(),
            ))
            .observe(on_ship_viewport_enter)
            .observe(on_ship_viewport_move)
            .observe(on_ship_viewport_leave)
            .observe(on_ship_viewport_click);
        spawn_ship_panel(split, icons);
    });
    pane_footer(
        body,
        |legend| {
            legend
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    ..default()
                })
                .with_children(|rows| {
                    for icons_in_row in SectionIconType::ALL.chunks(3) {
                        rows.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                            ..default()
                        })
                        .with_children(|row| {
                            for &icon in icons_in_row {
                                row.spawn(legend_entry()).with_children(|entry| {
                                    entry.spawn(icon_node(icons.section(icon), icon.color(), 18.0));
                                    entry.spawn(themed_label(icon.label(), 12.0, UiColor::Body));
                                });
                            }
                        });
                    }
                });
        },
        |controls| {
            controls
                .spawn((button(ButtonSpec::new("Fit").fit()), Name::new("ShipFit")))
                .observe(on_ship_fit_button);
            controls
                .spawn((
                    button(ButtonSpec::new("Reset").fit()),
                    Name::new("ShipReset"),
                ))
                .observe(on_ship_reset_button);
        },
        SHIP_KEY_HINTS,
    );
}

/// One icon and its word in a legend.
pub(crate) fn legend_entry() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(4),
        ..default()
    }
}

/// A themed text bundle for the interface chrome.
pub(crate) fn themed_label(value: &str, size: f32, color: UiColor) -> impl Bundle {
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

/// Raise the viewer contexts to match the interface: `Viewer` while the
/// interface owns the screen, and the shown pane's own `InterfacePane` context.
/// Both drop under the command modal, where the keyboard is typing.
pub(crate) fn sync_nova_os_contexts(
    pause: Option<Res<State<PauseStates>>>,
    pane: Res<InterfacePaneType>,
    mut active: ResMut<ActiveContexts>,
) {
    let open = pause.is_some_and(|state| *state.get() == PauseStates::Interface);
    ActiveContexts::sync(&mut active, ActionContext::Viewer, open);
    for (each, _) in InterfacePaneType::ALL {
        ActiveContexts::sync(
            &mut active,
            ActionContext::InterfacePane(each.context_id()),
            open && *pane == each,
        );
    }
}
