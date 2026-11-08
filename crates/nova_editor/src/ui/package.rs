//! File > Package: a centred window with the packager's settings first, and
//! what it wrote after.
//!
//! The settings are the packager's own: the manifest's version (typed, or one
//! bump on), the exports folder it always writes to, the file name that version
//! makes, the check it runs, and the files the archive holds beside the mod's
//! files it leaves out. A bump or a typed version is a manifest edit, one undo
//! step, and the window checks again whenever the history moves.
//!
//! After a package: the path, its size, Copy Path and Open Folder. A refusal or
//! a failed write stays in the window with its reason. Esc, or a press beside
//! the panel, closes it: its scrim is a [`PopupRoot`]. It holds
//! the keyboard ([`InputMode::Browse`]) so the stage's verbs do not run under
//! it, and Ctrl+Z still reaches the history.
//!
//! Native only: the web build has no store to package from, and File > Package
//! says so there.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use nova_ui::{
    prelude::*,
    theme::{self, density, UiColor},
};

use super::{assets::UNPACKAGED_MARK, layer, settings::MetaFieldType};
use crate::{
    clipboard::copy_out,
    history::EditHistory,
    project::{
        package::{
            bumped, open_folder, package_project, plan_package, shown_size, PackageDone,
            PackagePlan, VersionBumpType,
        },
        verbs::{edit_meta, note, pressed, refuse},
        ModProject,
    },
    ExampleStates,
};

/// The window's width.
const MODAL_W: f32 = 600.0;

/// The label column's width.
const LABEL_W: f32 = 70.0;

/// How many of the check's errors the window lists. The rest are in Problems.
const SHOWN_ERRORS: usize = 3;

/// How tall the file list may grow before it scrolls.
const FILES_H: f32 = 150.0;

/// How long a typed version may be.
const VERSION_CHARS: usize = 32;

/// The window: its last check, what the last press did, and a line to say.
#[derive(Component, Debug, Default)]
pub(crate) struct PackageModal {
    /// The history revision the check ran at.
    checked_at: Option<Option<u64>>,
    /// The check, or why there is none.
    plan: Option<Result<PackagePlan, String>>,
    /// What the last Package press did.
    outcome: Option<Result<PackageDone, String>>,
    /// What Copy Path or Open Folder said.
    said: Option<(String, UiColor)>,
}

/// The panel the window draws into.
#[derive(Component)]
pub(crate) struct PackagePanel;

/// A bump button.
#[derive(Component, Clone, Copy)]
struct PackageBump(VersionBumpType);

/// The Package window's systems.
pub(crate) struct PackageWindowPlugin;

impl Plugin for PackageWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, claim_package_keyboard.before(InputModeSystems));
        app.add_systems(
            Update,
            (
                package_keys.run_if(in_input_mode(InputMode::Browse)),
                check_package_again,
                draw_package_modal,
            )
                .chain()
                .run_if(in_state(ExampleStates::Editor)),
        );
    }
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

/// The history revision now.
fn revision(world: &World) -> Option<u64> {
    world
        .get_resource::<EditHistory>()
        .and_then(EditHistory::revision)
}

/// Put the window up over everything, checked. One at a time.
pub(crate) fn open_package_modal(world: &mut World) {
    if world
        .query_filtered::<(), With<PackageModal>>()
        .iter(world)
        .next()
        .is_some()
    {
        return;
    }
    let checked_at = Some(revision(world));
    let plan = Some(plan_package(world));
    world
        .spawn((
            Name::new("Package Modal"),
            PackageModal {
                checked_at,
                plan,
                ..default()
            },
            // A popup to the editor's Esc owner, so Esc closes the window and
            // does not fall through to the pause menu.
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
        .with_child((
            Name::new("Package Panel"),
            PackagePanel,
            Node {
                width: px(MODAL_W),
                max_width: percent(94),
                max_height: percent(90),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(theme::BORDER_W)),
                overflow: Overflow::clip(),
                ..default()
            },
            ThemedRadius::control(),
            panel(),
        ));
}

/// Check again when the history moves under the window: a bump, a typed
/// version, an undo.
fn check_package_again(world: &mut World) {
    let Some(modal) = world
        .query_filtered::<Entity, With<PackageModal>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let now = Some(revision(world));
    if world.get::<PackageModal>(modal).map(|held| held.checked_at) == Some(now) {
        return;
    }
    let plan = plan_package(world);
    if let Some(mut held) = world.get_mut::<PackageModal>(modal) {
        held.checked_at = now;
        held.plan = Some(plan);
        held.outcome = None;
        held.said = None;
    }
}

/// Hold the keyboard while the window is up.
fn claim_package_keyboard(
    modals: Query<(), With<PackageModal>>,
    mut claims: MessageWriter<ClaimKeyboard>,
) {
    if !modals.is_empty() {
        claims.write(ClaimKeyboard(InputMode::Browse));
    }
}

/// Esc closes the window.
fn package_keys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    modals: Query<Entity, With<PackageModal>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for modal in &modals {
            commands.entity(modal).try_despawn();
        }
    }
}

/// Close.
fn on_close(
    _activate: On<Activate>,
    mut commands: Commands,
    modals: Query<Entity, With<PackageModal>>,
) {
    for modal in &modals {
        commands.entity(modal).try_despawn();
    }
}

/// Draw the window from what it holds.
fn draw_package_modal(
    mut commands: Commands,
    modals: Query<(&PackageModal, &Children), Changed<PackageModal>>,
    panels: Query<(), With<PackagePanel>>,
) {
    for (modal, children) in &modals {
        for panel in children.iter().filter(|child| panels.contains(*child)) {
            commands.entity(panel).despawn_related::<Children>();
            commands
                .entity(panel)
                .with_children(|body| match &modal.outcome {
                    Some(Ok(done)) => spawn_done(body, done, modal.said.as_ref()),
                    _ => spawn_settings(body, modal),
                });
        }
    }
}

/// A label and what follows it on one line.
fn spawn_field(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    name: &str,
    label: &str,
    value: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
) {
    body.spawn((
        Name::new(format!("{name} Row")),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(density::GAP),
            min_height: px(22),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Node {
                width: px(LABEL_W),
                flex_shrink: 0.0,
                ..default()
            },
            children![words(label, density::TEXT_SMALL, UiColor::Label)],
        ));
        row.spawn(Node {
            flex_grow: 1.0,
            min_width: px(0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: px(density::GAP),
            ..default()
        })
        .with_children(value);
    });
}

/// The settings, the check and the files, with Cancel and Package.
fn spawn_settings(body: &mut RelatedSpawnerCommands<ChildOf>, modal: &PackageModal) {
    body.spawn((
        Name::new("Package Title"),
        words("PACKAGE MOD", density::TEXT, UiColor::Accent),
    ));
    match &modal.plan {
        Some(Ok(plan)) => spawn_plan(body, plan),
        Some(Err(reason)) => {
            body.spawn((
                Name::new("Package Plan Fault"),
                words(reason.clone(), density::TEXT_SMALL, UiColor::Danger),
            ));
        }
        None => {}
    }
    if let Some(Err(reason)) = &modal.outcome {
        body.spawn((
            Name::new("Package Refusal"),
            words(reason.clone(), density::TEXT_SMALL, UiColor::Danger),
        ));
    }
    body.spawn((
        Name::new("Package Foot"),
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(density::GAP),
            margin: UiRect::top(px(4)),
            ..default()
        },
    ))
    .with_children(|foot| {
        foot.spawn((
            Name::new("Package Cancel"),
            button(ButtonSpec::new("Cancel").dense().ghost()),
            ButtonCueType::Back,
            observe(on_close),
        ));
        let mut go = foot.spawn((
            Name::new("Package Button"),
            button(ButtonSpec::new("Package").dense().primary()),
            ButtonCueType::Owned,
            observe(on_package),
        ));
        if !matches!(modal.plan, Some(Ok(_))) {
            go.insert(InteractionDisabled);
        }
    });
}

/// What the check found, as the window's Check line.
pub(crate) fn check_line(errors: usize, warnings: usize) -> (String, UiColor) {
    let count = |count: usize, word: &str| match count {
        1 => format!("1 {word}"),
        count => format!("{count} {word}s"),
    };
    let line = format!("{}, {}", count(errors, "error"), count(warnings, "warning"));
    match errors {
        0 => (format!("passes: {line}"), UiColor::Nominal),
        _ => (format!("blocked: {line}"), UiColor::Danger),
    }
}

/// The settings of one check.
fn spawn_plan(body: &mut RelatedSpawnerCommands<ChildOf>, plan: &PackagePlan) {
    spawn_field(body, "Package Version", "Version", |row| {
        row.spawn(Node {
            width: px(110),
            ..default()
        })
        .with_child((
            Name::new("Package Version Field"),
            MetaFieldType::Version,
            text_field(
                TextFieldSpec::new(plan.version.clone())
                    .max_chars(VERSION_CHARS)
                    .tool(),
            ),
        ));
        for bump in VersionBumpType::ALL {
            let next = bumped(&plan.version, bump);
            let label = next.as_ref().map_or_else(
                || bump.label().to_string(),
                |next| format!("{} {next}", bump.label()),
            );
            let mut button_row = row.spawn((
                Name::new(format!("Package Bump {}", bump.label())),
                PackageBump(bump),
                button(ButtonSpec::new(label).dense().ghost().fit()),
                ButtonCueType::Owned,
                observe(on_bump),
            ));
            if next.is_none() {
                button_row.insert(InteractionDisabled);
            }
        }
    });
    if VersionBumpType::ALL
        .iter()
        .all(|bump| bumped(&plan.version, *bump).is_none())
    {
        body.spawn((
            Name::new("Package Bump Note"),
            words(
                "a bump steps a version of up to three numbers, like 1.0.0",
                density::TEXT_SMALL,
                UiColor::Label,
            ),
        ));
    }
    let (folder, tint) = match &plan.folder {
        Ok(folder) => (folder.display().to_string(), UiColor::Primary),
        Err(reason) => (reason.clone(), UiColor::Danger),
    };
    spawn_field(body, "Package Output", "Output", |row| {
        row.spawn((
            Name::new("Package Output Folder"),
            words(folder, density::TEXT_SMALL, tint),
        ));
    });
    spawn_field(body, "Package File", "File", |row| {
        row.spawn((
            Name::new("Package File Name"),
            words(
                plan.file_name.clone(),
                density::TEXT_SMALL,
                UiColor::Primary,
            ),
        ));
    });
    let (line, tint) = check_line(plan.errors.len(), plan.warnings);
    spawn_field(body, "Package Check", "Check", |row| {
        row.spawn((
            Name::new("Package Check Line"),
            words(line, density::TEXT_SMALL, tint),
        ));
    });
    for (index, error) in plan.errors.iter().take(SHOWN_ERRORS).enumerate() {
        body.spawn((
            Name::new(format!("Package Error {index}")),
            Node {
                margin: UiRect::left(px(LABEL_W + density::GAP)),
                ..default()
            },
            children![words(error.clone(), density::TEXT_SMALL, UiColor::Danger)],
        ));
    }
    if plan.errors.len() > SHOWN_ERRORS {
        body.spawn((
            Name::new("Package Errors More"),
            Node {
                margin: UiRect::left(px(LABEL_W + density::GAP)),
                ..default()
            },
            children![words(
                format!("+{} more in Problems", plan.errors.len() - SHOWN_ERRORS),
                density::TEXT_SMALL,
                UiColor::Label,
            )],
        ));
    }
    let held = plan.files.iter().filter(|file| file.size.is_some()).count();
    let left_out = plan.files.len() - held;
    let title = match left_out {
        0 => format!("FILES {held}"),
        out => format!("FILES {held}, {out} {UNPACKAGED_MARK} not packaged"),
    };
    body.spawn((
        Name::new("Package Files Title"),
        words(title, density::TEXT_SMALL, UiColor::Label),
    ));
    body.spawn((Name::new("Package Files Scroll"), scroll_row()))
        .insert(Node {
            max_height: px(FILES_H),
            ..default()
        })
        .with_children(|scroll| {
            scroll
                .spawn((
                    Name::new("Package Files"),
                    Node {
                        align_items: AlignItems::Stretch,
                        ..scroll_column()
                    },
                    scroll_viewport(),
                ))
                .with_children(|list| {
                    for file in &plan.files {
                        spawn_file_row(list, &file.path, file.size);
                    }
                });
            scroll.spawn((Name::new("Package Files Bar"), scroll_bar()));
        });
}

/// One file: the mark of a file the package leaves out, its path and size.
fn spawn_file_row(list: &mut RelatedSpawnerCommands<ChildOf>, path: &str, size: Option<u64>) {
    let (mark, detail, tint) = match size {
        Some(size) => ("", shown_size(size), UiColor::Primary),
        None => (UNPACKAGED_MARK, "not packaged".to_string(), UiColor::Label),
    };
    list.spawn((
        Name::new(format!("Package File Row {path}")),
        Node {
            width: percent(100),
            min_height: px(18),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::horizontal(px(4)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Node {
                width: px(12),
                flex_shrink: 0.0,
                ..default()
            },
            children![words(mark, density::TEXT_SMALL, UiColor::Label)],
        ));
        row.spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                overflow: Overflow::clip(),
                ..default()
            },
            children![words(path, density::TEXT_SMALL, tint)],
        ));
        row.spawn(words(detail, density::TEXT_SMALL, UiColor::Label));
    });
}

/// The written package: its path and size, Copy Path, Open Folder and Close.
fn spawn_done(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    done: &PackageDone,
    said: Option<&(String, UiColor)>,
) {
    body.spawn((
        Name::new("Package Title"),
        words("PACKAGED", density::TEXT, UiColor::Accent),
    ));
    let path = done.path.display().to_string();
    spawn_field(body, "Package Done Path", "Path", |row| {
        row.spawn((
            Name::new("Package Done Path"),
            words(path, density::TEXT_SMALL, UiColor::Primary),
        ));
    });
    spawn_field(body, "Package Done Size", "Size", |row| {
        row.spawn((
            Name::new("Package Done Size"),
            words(shown_size(done.size), density::TEXT_SMALL, UiColor::Primary),
        ));
    });
    if let Some((line, tint)) = said {
        body.spawn((
            Name::new("Package Said"),
            words(line.clone(), density::TEXT_SMALL, *tint),
        ));
    }
    body.spawn((
        Name::new("Package Foot"),
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(density::GAP),
            margin: UiRect::top(px(4)),
            ..default()
        },
    ))
    .with_children(|foot| {
        foot.spawn((
            Name::new("Package Copy Path"),
            button(ButtonSpec::new("Copy Path").dense().ghost()),
            ButtonCueType::Owned,
            observe(on_copy_path),
        ));
        foot.spawn((
            Name::new("Package Open Folder"),
            button(ButtonSpec::new("Open Folder").dense().ghost()),
            ButtonCueType::Owned,
            observe(on_open_folder),
        ));
        foot.spawn((
            Name::new("Package Close"),
            button(ButtonSpec::new("Close").dense().primary()),
            ButtonCueType::Back,
            observe(on_close),
        ));
    });
}

/// The window, for a verb that writes to it.
fn modal_mut(world: &mut World) -> Option<Mut<'_, PackageModal>> {
    let modal = world
        .query_filtered::<Entity, With<PackageModal>>()
        .iter(world)
        .next()?;
    world.get_mut::<PackageModal>(modal)
}

/// A bump: the version one step on, as one undo step. The window checks again
/// when the history moves.
fn on_bump(activate: On<Activate>, mut commands: Commands, bumps: Query<&PackageBump>) {
    let Ok(PackageBump(bump)) = bumps.get(activate.entity).copied() else {
        return;
    };
    commands.queue(move |world: &mut World| {
        pressed(world, |world| {
            let Some(version) = world
                .get_resource::<ModProject>()
                .map(|project| project.data.meta.version.clone())
            else {
                return;
            };
            match bumped(&version, bump) {
                Some(next) => edit_meta(world, "Bump Mod Version", move |meta| {
                    meta.version = next;
                }),
                None => refuse(
                    world,
                    format!("{version} has a part that is not a number: type the version"),
                ),
            }
        });
    });
}

/// Package: write the archive, and keep what happened in the window.
fn on_package(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        pressed(world, |world| {
            let outcome = package_project(world);
            if let Some(mut modal) = modal_mut(world) {
                modal.outcome = Some(outcome);
                modal.said = None;
            }
        });
    });
}

/// The written package's path.
fn done_path(world: &mut World) -> Option<std::path::PathBuf> {
    match modal_mut(world)?.outcome.as_ref()? {
        Ok(done) => Some(done.path.clone()),
        Err(_) => None,
    }
}

/// Say `line` in the window and on the status line.
fn say_in_window(world: &mut World, line: String, refused: bool) {
    if refused {
        refuse(world, line.clone());
    } else {
        note(world, line.clone());
    }
    let tint = if refused {
        UiColor::Danger
    } else {
        UiColor::Nominal
    };
    if let Some(mut modal) = modal_mut(world) {
        modal.said = Some((line, tint));
    }
}

/// Copy Path.
fn on_copy_path(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        pressed(world, |world| {
            let Some(path) = done_path(world) else {
                return;
            };
            let shown = path.display().to_string();
            match copy_out(world, shown.clone()) {
                Ok(()) => say_in_window(world, format!("Copied {shown}"), false),
                Err(reason) => say_in_window(world, format!("copy refused: {reason}"), true),
            }
        });
    });
}

/// Open Folder: the exports folder in the desktop's file manager.
fn on_open_folder(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        pressed(world, |world| {
            let Some(folder) =
                done_path(world).and_then(|path| path.parent().map(std::path::Path::to_path_buf))
            else {
                return;
            };
            match open_folder(&folder) {
                Ok(()) => say_in_window(world, format!("opened {}", folder.display()), false),
                Err(reason) => {
                    say_in_window(world, format!("the folder did not open: {reason}"), true);
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_check_line_counts_and_says_whether_it_blocks() {
        assert_eq!(
            check_line(0, 1),
            ("passes: 0 errors, 1 warning".to_string(), UiColor::Nominal)
        );
        assert_eq!(
            check_line(2, 0),
            ("blocked: 2 errors, 0 warnings".to_string(), UiColor::Danger)
        );
    }
}
