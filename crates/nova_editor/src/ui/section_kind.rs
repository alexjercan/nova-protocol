//! New Section: which kind. A centred modal like New Mod - a tile per kind
//! with what it does, Create and Cancel - because a section's kind is code,
//! so it is the one question a new section asks. Every field after it starts
//! blank.
//!
//! Keys: the arrows move the pick over the grid, Enter creates, Esc cancels.
//! The modal holds the keyboard ([`InputMode::Browse`]) while it is up, so the
//! stage's verbs do not run under it. Its scrim is a [`PopupRoot`]: a press
//! beside the panel, or any other popup opening, puts it away.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_ui::{
    prelude::*,
    theme::{self, density, UiColor},
};

use super::layer;
use crate::{
    asset::{
        blank::{section_kinds, spaced},
        verbs::add_asset,
    },
    project::verbs::pressed,
    ExampleStates,
};

/// The modal, and the index of the picked kind in [`section_kinds`].
#[derive(Component, Debug)]
pub(crate) struct SectionKindModal {
    picked: usize,
}

/// One kind's tile.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SectionKindTile(usize);

/// The modal's width, as New Mod's.
const MODAL_W: f32 = 500.0;

/// Tiles in a row of the grid. Up and Down move a row.
const COLUMNS: usize = 3;

/// A tile's height: its name and two lines of what the kind does.
const TILE_H: f32 = 58.0;

/// What a section of `kind` does, as its tile says it. Every kind of
/// [`section_kinds`] has one.
pub(crate) fn section_kind_hint(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "Hull" => "Structure: bulk and armour the other parts mate to.",
        "Thruster" => "Pushes the ship along its nozzle.",
        "Controller" => "Turns the ship: attitude control.",
        "Turret" => "An aimed gun on a turning mount.",
        "Torpedo" => "Launches guided torpedoes.",
        "Railgun" => "A spinal lance the whole hull aims.",
        "Docking" => "A telescoping port that docks at stations.",
        "CargoIntake" => "A hold mouth that takes in cargo canisters.",
        "Mining" => "A beam that cuts ore out of rock.",
        _ => return None,
    })
}

/// Words of `size` in `colour`.
fn words(text: impl Into<String>, size: f32, colour: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
        Pickable::IGNORE,
    )
}

/// Open the modal over everything, the first kind picked.
///
/// It opens from a menu row, and a menu row closes its menu itself; a second
/// close here would despawn the menu twice.
pub(crate) fn open_section_kind_modal(commands: &mut Commands) {
    commands
        .spawn((
            Name::new("Section Kind Modal"),
            SectionKindModal { picked: 0 },
            PopupRoot,
            DespawnOnExit(ExampleStates::Editor),
            GlobalZIndex(layer::WINDOW_Z),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ThemedFill::alpha(UiColor::Void, 0.7),
        ))
        .with_children(|scrim| {
            scrim
                .spawn((
                    Name::new("Section Kind Panel"),
                    Node {
                        width: px(MODAL_W),
                        max_width: percent(94),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|modal| {
                    modal.spawn((
                        Name::new("Section Kind Title"),
                        words("NEW SECTION", density::TEXT, UiColor::Accent),
                    ));
                    modal.spawn((
                        Name::new("Section Kind Prompt"),
                        words(
                            "Pick what the section is. Arrows move, Enter creates.",
                            density::TEXT_SMALL,
                            UiColor::Label,
                        ),
                    ));
                    modal
                        .spawn((
                            Name::new("Section Kind Tiles"),
                            Node {
                                display: Display::Grid,
                                grid_template_columns: RepeatedGridTrack::flex(COLUMNS as u16, 1.0),
                                column_gap: px(density::GAP),
                                row_gap: px(density::GAP),
                                ..default()
                            },
                        ))
                        .with_children(|tiles| {
                            for (index, kind) in section_kinds().into_iter().enumerate() {
                                spawn_tile(tiles, index, kind);
                            }
                        });
                    modal
                        .spawn((
                            Name::new("Section Kind Foot"),
                            Node {
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: px(density::GAP),
                                ..default()
                            },
                        ))
                        .with_children(|foot| {
                            foot.spawn((
                                Name::new("Section Kind Cancel"),
                                button(ButtonSpec::new("Cancel").dense().ghost()),
                                ButtonCueType::Back,
                                observe(on_cancel),
                            ));
                            foot.spawn((
                                Name::new("Section Kind Create"),
                                button(ButtonSpec::new("Create").dense().primary()),
                                ButtonCueType::Owned,
                                observe(on_create),
                            ));
                        });
                });
        });
}

/// One kind: its name and what it does.
fn spawn_tile(tiles: &mut RelatedSpawnerCommands<ChildOf>, index: usize, kind: &'static str) {
    tiles
        .spawn((
            Name::new(format!("Section Kind {kind}")),
            SectionKindTile(index),
            ListRow,
            Button,
            Hovered::default(),
            Node {
                min_width: px(0),
                height: px(TILE_H),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::all(px(6)),
                border: UiRect::all(px(theme::BORDER_W)),
                overflow: Overflow::clip(),
                ..default()
            },
            ThemedRadius::control(),
            BorderColor::all(Color::NONE),
            BackgroundColor(Color::NONE),
            observe(on_tile),
        ))
        .with_children(|tile| {
            tile.spawn(words(spaced(kind), density::TEXT, UiColor::Primary));
            // Body, not Label: a picked tile's fill drowns a muted line.
            tile.spawn(words(
                section_kind_hint(kind).unwrap_or_default(),
                density::TEXT_SMALL,
                UiColor::Body,
            ));
        });
}

/// A press on a tile picks its kind; a second press in time creates it.
fn on_tile(
    activate: On<Activate>,
    mut commands: Commands,
    tiles: Query<&SectionKindTile>,
    mut modals: Query<(Entity, &mut SectionKindModal)>,
    time: Res<Time<Real>>,
    mut last: ResMut<DoubleClick<Entity>>,
) {
    let Ok(SectionKindTile(index)) = tiles.get(activate.entity).copied() else {
        return;
    };
    let Ok((modal, mut picked)) = modals.single_mut() else {
        return;
    };
    picked.picked = index;
    if last.press(activate.entity, time.elapsed_secs()) {
        create(&mut commands, modal, index);
    }
}

/// Make a blank section of kind `index`, open it, and put the modal away.
fn create(commands: &mut Commands, modal: Entity, index: usize) {
    commands.entity(modal).try_despawn();
    let Some(kind) = section_kinds().get(index).copied() else {
        return;
    };
    commands.queue(move |world: &mut World| {
        pressed(world, |world| add_asset(world, "section", Some(kind)));
    });
}

/// Create.
fn on_create(
    _activate: On<Activate>,
    mut commands: Commands,
    modals: Query<(Entity, &SectionKindModal)>,
) {
    if let Ok((modal, picked)) = modals.single() {
        create(&mut commands, modal, picked.picked);
    }
}

/// Cancel.
fn on_cancel(
    _activate: On<Activate>,
    mut commands: Commands,
    modals: Query<Entity, With<SectionKindModal>>,
) {
    for modal in &modals {
        commands.entity(modal).try_despawn();
    }
}

/// Where the arrow `key` moves a pick of `picked` over a grid of `count`
/// tiles, [`COLUMNS`] to a row. The pick stops at the edges.
pub(crate) fn moved(picked: usize, count: usize, key: KeyCode) -> usize {
    let last = count.saturating_sub(1);
    match key {
        KeyCode::ArrowLeft => picked.saturating_sub(1),
        KeyCode::ArrowRight => (picked + 1).min(last),
        KeyCode::ArrowUp if picked >= COLUMNS => picked - COLUMNS,
        KeyCode::ArrowDown if picked + COLUMNS <= last => picked + COLUMNS,
        _ => picked,
    }
}

/// The modal's keys: the arrows move the pick, Enter creates, Esc cancels.
///
/// Not on the frame the modal opens: the Enter that ran New Asset > Section
/// in the menu is still `just_pressed`, and would create a Hull unasked.
pub(crate) fn section_kind_keys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut modals: Query<(Entity, Mut<SectionKindModal>)>,
) {
    let Ok((modal, mut picked)) = modals.single_mut() else {
        return;
    };
    if picked.is_added() {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        commands.entity(modal).try_despawn();
        return;
    }
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        create(&mut commands, modal, picked.picked);
        return;
    }
    let count = section_kinds().len();
    for key in [
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
    ] {
        let to = moved(picked.picked, count, key);
        if keys.just_pressed(key) && to != picked.picked {
            picked.picked = to;
        }
    }
}

/// Mark the picked tile.
pub(crate) fn sync_section_kind_tiles(
    mut commands: Commands,
    modals: Query<&SectionKindModal, Changed<SectionKindModal>>,
    tiles: Query<(Entity, &SectionKindTile, Has<Selected>)>,
) {
    let Ok(modal) = modals.single() else {
        return;
    };
    for (tile, SectionKindTile(index), marked) in &tiles {
        match (*index == modal.picked, marked) {
            (true, false) => {
                commands.entity(tile).try_insert(Selected);
            }
            (false, true) => {
                commands.entity(tile).try_remove::<Selected>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind the format declares has a line on its tile: a new kind
    /// without one fails here, not as a blank tile.
    #[test]
    fn every_section_kind_says_what_it_does() {
        for kind in section_kinds() {
            assert!(section_kind_hint(kind).is_some(), "{kind} has no hint");
        }
    }

    /// The arrows walk the grid a tile or a row at a time and stop at its
    /// edges.
    #[test]
    fn the_arrows_move_the_pick_over_the_grid_and_stop_at_the_edges() {
        assert_eq!(moved(0, 9, KeyCode::ArrowRight), 1);
        assert_eq!(moved(0, 9, KeyCode::ArrowLeft), 0);
        assert_eq!(moved(1, 9, KeyCode::ArrowDown), 4);
        assert_eq!(moved(7, 9, KeyCode::ArrowDown), 7, "no row below");
        assert_eq!(moved(4, 9, KeyCode::ArrowUp), 1);
        assert_eq!(moved(2, 9, KeyCode::ArrowUp), 2, "no row above");
        assert_eq!(moved(8, 9, KeyCode::ArrowRight), 8);
    }
}
