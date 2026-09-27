//! The TAB interface: one themed full-screen surface over the frozen world,
//! with a Map pane and a Ship pane.
//!
//! TAB opens it from flight (`Unpaused -> Interface`) and closes it again.
//! Keyboard M, gamepad Y and the Map and Ship buttons in the card's title row
//! switch the pane. The first open shows Map; after that TAB reopens the last
//! pane. The `:` command modal opens over the pane without tearing it down, so
//! closing the modal returns to the same pane with its scene, camera and
//! selection.
//!
//! Escape is not read here: the one back-out owner is
//! [`close_surface_from_menu_keys`](crate::terminal), so a single press never
//! closes both the modal and the pane.
//!
//! Touch this module when changing how the interface opens, which pane shows,
//! or the layout around the panes.

use bevy::prelude::*;
use nova_gameplay::{prelude::PlayerSpaceshipMarker, PauseStates};
use nova_input::prelude::{ActionContext, ActiveContexts, InputBindings, InputSources};
use nova_ui::{
    prelude::*,
    theme::UiColor,
    widget::{ThemedText, UiText},
};

use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    map::{on_map_reframe_button, MapLegendMarker, MapReadoutMarker, MapViewportMarker},
    ship::{
        on_ship_fit_button, on_ship_reset_button, on_ship_step_button, spawn_ship_panel,
        ShipRuntime, ShipViewportMarker,
    },
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
    /// The player ship: sections, repair, reload, rebind and mates.
    Ship,
}

impl InterfacePaneType {
    /// Every pane, in title-row order, with its button label.
    const ALL: [(Self, &'static str); 2] = [(Self::Map, "Map"), (Self::Ship, "Ship")];

    /// The per-pane action context id its verbs are bound under.
    pub(crate) fn context_id(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Ship => "ship",
        }
    }
}

/// The full-screen interface root. The card and its body under it are rebuilt
/// by [`rebuild_interface_body`] when the pane changes.
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

/// `interface_next_tab` steps to the other pane while the interface owns the
/// screen. An armed section rebind takes the key instead: leaving the Ship pane
/// would drop the capture.
pub(crate) fn next_interface_pane(
    sources: InputSources,
    bindings: Res<InputBindings>,
    current: Res<State<PauseStates>>,
    mut pane: ResMut<InterfacePaneType>,
    ship: Option<Res<ShipRuntime>>,
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
    *pane = match *pane {
        InterfacePaneType::Map => InterfacePaneType::Ship,
        InterfacePaneType::Ship => InterfacePaneType::Map,
    };
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

/// Widest the interface grows, in logical px.
const INTERFACE_MAX_PX: f32 = 1520.0;

/// Rebuild the card on a fresh root or a pane change.
///
/// The Map and Ship buttons sit in the card's title row and are rebuilt with
/// the body, so the selected one follows a keyboard or pad switch as well as a
/// click.
pub(crate) fn rebuild_interface_body(
    mut commands: Commands,
    pane: Res<InterfacePaneType>,
    icons: Res<InterfaceIcons>,
    q_root: Query<(Entity, Ref<InterfaceRootMarker>)>,
) {
    let Ok((root, marker)) = q_root.single() else {
        return;
    };
    if !marker.is_added() && !pane.is_changed() {
        return;
    }
    let pane = *pane;
    commands.entity(root).despawn_children();
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
            let title = match pane {
                InterfacePaneType::Map => "Map",
                InterfacePaneType::Ship => "Ship",
            };
            card.spawn(panel_head(title, None)).with_children(|head| {
                head.spawn(segmented_container()).with_children(|seg| {
                    for (value, label) in InterfacePaneType::ALL {
                        let mut option = seg.spawn((
                            segmented_option(label),
                            ButtonValue(value),
                            Name::new(format!("InterfaceTab{label}")),
                        ));
                        if value == pane {
                            option.insert(Selected);
                        }
                    }
                });
            });
            card.spawn(Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(12)),
                ..default()
            })
            .with_children(|body| match pane {
                InterfacePaneType::Map => map_body(body),
                InterfacePaneType::Ship => ship_body(body, &icons),
            });
        });
    });
}

/// A pane's 3D view: the node its scene image fills and its blips ride on.
fn viewport_node() -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        position_type: PositionType::Relative,
        overflow: Overflow::clip(),
        ..default()
    }
}

/// A row of controls under a view.
fn control_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(12),
        flex_shrink: 0.0,
        ..default()
    }
}

/// The map: the scene, and under it the readout, the legend of what it plots
/// and Reframe.
fn map_body(body: &mut ChildSpawnerCommands) {
    body.spawn((MapViewportMarker, viewport_node(), ImageNode::default()));
    body.spawn(Node {
        height: px(40),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|line| {
        line.spawn((
            MapReadoutMarker,
            themed_label(
                "Select a contact for range and bearing.",
                14.0,
                UiColor::Body,
            ),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip(),
                ..default()
            },
        ));
        line.spawn((
            MapLegendMarker,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        line.spawn((
            button(ButtonSpec::new("Reframe").fit()),
            Name::new("MapReframe"),
        ))
        .observe(on_map_reframe_button);
    });
}

/// The ship: the scene with the section legend and the step, fit and reset
/// controls centred under it, and the section panel beside it. An empty side
/// as wide as the legend's keeps the controls centred.
fn ship_body(body: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    let side = || Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    body.spawn(Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    })
    .with_children(|split| {
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
                viewer.spawn((ShipViewportMarker, viewport_node(), ImageNode::default()));
                viewer
                    .spawn(control_row(JustifyContent::Center))
                    .with_children(|foot| {
                        foot.spawn(side()).with_children(|side| {
                            side.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                align_items: AlignItems::Center,
                                column_gap: px(12),
                                row_gap: px(6),
                                ..default()
                            })
                            .with_children(|legend| {
                                for icon in SectionIconType::ALL {
                                    legend.spawn(legend_entry()).with_children(|entry| {
                                        entry.spawn(icon_node(
                                            icons.section(icon),
                                            icon.color(),
                                            18.0,
                                        ));
                                        entry.spawn(themed_label(
                                            icon.label(),
                                            12.0,
                                            UiColor::Body,
                                        ));
                                    });
                                }
                            });
                        });
                        foot.spawn(control_row(JustifyContent::Center))
                            .with_children(|controls| {
                                for (label, step) in [("Prev", -1), ("Next", 1)] {
                                    controls
                                        .spawn((
                                            button(ButtonSpec::new(label).fit()),
                                            Name::new(format!("Ship{label}")),
                                        ))
                                        .observe(on_ship_step_button(step));
                                }
                                controls
                                    .spawn((
                                        button(ButtonSpec::new("Fit").fit()),
                                        Name::new("ShipFit"),
                                    ))
                                    .observe(on_ship_fit_button);
                                controls
                                    .spawn((
                                        button(ButtonSpec::new("Reset").fit()),
                                        Name::new("ShipReset"),
                                    ))
                                    .observe(on_ship_reset_button);
                            });
                        foot.spawn(side());
                    });
            });
        spawn_ship_panel(split, icons);
    });
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
