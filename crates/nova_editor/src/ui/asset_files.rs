//! The Assets tab's file tiles: the files in the mod's folder beside its
//! items, like a project window. An image shows itself, a mesh its rendered
//! picture and any other file a glyph. A double click views an image or a mesh
//! large and plays a sound. The tile's right-click menu holds Reveal Path,
//! Rename, Delete, Include in Package and Import File.
//!
//! The model and the verbs are [`crate::asset::files`]'s; this module draws
//! them.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui_widgets::{observe, Activate},
    window::PrimaryWindow,
};
use nova_assets::mod_cache::prelude::MODS_SOURCE;
use nova_gameplay::prelude::{AudioRoute, PlaySfx};
use nova_ui::{prelude::*, theme::UiColorType};

use super::{
    assets::{import_file_item, new_asset_items, AssetRenaming, RenameTarget},
    browser::pane_text,
    files::PathPurposeType,
    layer,
    menu_context::{verb_state, MenuFacts, MenuVerbType, VerbState},
    thumbnail::{thumbnail_key, AssetThumbnails, ThumbnailParts},
};
use crate::{
    asset::files::{
        delete_file, file_kind, include_file, loads_as_image, plays, reveal_file, ModFile,
        AUDIO_KIND, IMAGE_KIND, MESH_KIND,
    },
    bundle::{FileWindowKind, FileWindowRequest},
    config::{EditorSays, SelectedNode},
    project::{
        verbs::{note, pressed, refuse},
        ModProject,
    },
    ExampleStates,
};

/// The most room the viewer gives a picture.
const VIEW_MAX: Vec2 = Vec2::new(480.0, 360.0);

/// What a file tile's menu row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileVerbType {
    /// View an image or a mesh, or play a sound.
    View,
    /// Say where the file stands.
    Reveal,
    Rename,
    Delete,
    /// Declare the file, so the package holds it.
    Include,
    /// Copy a file from elsewhere into the folder.
    Import,
}

/// What a file verb's state reads of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileFacts {
    /// The manifest declares it.
    pub(crate) packaged: bool,
    /// It is in the folder.
    pub(crate) on_disk: bool,
}

impl From<&ModFile> for FileFacts {
    fn from(file: &ModFile) -> Self {
        Self {
            packaged: file.packaged,
            on_disk: file.on_disk,
        }
    }
}

/// A file tile, and its file.
#[derive(Component, Clone)]
pub(crate) struct AssetFileTile(pub(crate) ModFile);

/// A file tile's menu row.
#[derive(Component, Clone)]
pub(crate) struct FileVerb(pub(crate) FileVerbType, pub(crate) ModFile);

/// An image tile's picture slot, waiting for its image.
#[derive(Component)]
pub(crate) struct AssetImage(pub(crate) Handle<Image>);

/// The viewer's picture, sized once its image is in.
#[derive(Component)]
pub(crate) struct ViewedImage(Handle<Image>);

/// The asset path of `file` in mod `id`.
pub(crate) fn file_asset_path(id: &str, file: &str) -> String {
    format!("{MODS_SOURCE}://{id}/{file}")
}

/// The picture of a mesh file: a hull wearing it, as a section tile's picture
/// is taken.
pub(crate) fn mesh_parts(id: &str, file: &str) -> ThumbnailParts {
    use nova_gameplay::prelude::AssetRef;
    use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, SectionConfig, SectionKind};
    let mesh = format!("{}#Scene0", file_asset_path(id, file));
    vec![(
        SectionConfig {
            base: BaseSectionConfig::default(),
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(AssetRef::Path(mesh)),
                render_mesh_transform: None,
            }),
        },
        Transform::default(),
    )]
}

/// `size` scaled down to fit in `room`, aspect kept. Never scaled up.
pub(crate) fn fit(size: Vec2, room: Vec2) -> Vec2 {
    if size.x <= 0.0 || size.y <= 0.0 {
        return room;
    }
    size * (room.x / size.x).min(room.y / size.y).min(1.0)
}

/// The rows of a file tile's menu that `menu` does not hide.
pub(crate) fn spawn_file_rows(
    menu: &mut RelatedSpawnerCommands<ChildOf>,
    file: &ModFile,
    context: &super::menu_context::MenuContext,
) {
    let view = match file_kind(&file.path) {
        AUDIO_KIND => "Play",
        IMAGE_KIND | MESH_KIND => "View",
        _ => "",
    };
    let rows = [
        (view, "Enter", FileVerbType::View),
        ("Reveal Path", "", FileVerbType::Reveal),
        ("Rename", "", FileVerbType::Rename),
        ("Include in Package", "", FileVerbType::Include),
        ("Delete", "", FileVerbType::Delete),
        ("Import File...", "", FileVerbType::Import),
    ];
    for (word, hint, verb) in rows {
        let row = MenuVerbType::File(verb, file.into());
        if word.is_empty() || verb_state(row, context) == VerbState::Hidden {
            continue;
        }
        menu.spawn((
            Name::new(format!("Asset Menu {word}")),
            row,
            FileVerb(verb, file.clone()),
            popup_row(word, hint),
            ButtonCueType::Owned,
            observe(on_file_verb),
        ));
    }
}

/// A right press on the grid's bare floor: New Asset, and Import File where
/// the build writes a folder.
pub(crate) fn on_asset_grid_menu(
    click: On<PointerClick>,
    mut commands: Commands,
    roots: Query<Entity, With<PopupRoot>>,
    facts: MenuFacts,
    selected: Res<SelectedNode>,
) {
    if click.button != PointerButton::Secondary
        || click.event_target() != click.original_event_target()
    {
        return;
    }
    let row = MenuVerbType::File(
        FileVerbType::Import,
        FileFacts {
            packaged: true,
            on_disk: true,
        },
    );
    let context = facts.context(&selected);
    let mut items = vec![CascadeItem::submenu(
        "Asset Menu New Asset",
        "New Asset",
        new_asset_items(&context),
    )];
    // No Paste: the clipboard holds nodes and blocks, never a whole asset.
    match verb_state(row, &context) {
        VerbState::Hidden => {}
        VerbState::Greyed(reason) => items.push(import_file_item().greyed(reason)),
        VerbState::Live => items.push(import_file_item()),
    }
    close_popups(&mut commands, &roots);
    let root = spawn_cascade(&mut commands, click.pointer.position, layer::MENU_Z, items);
    commands
        .entity(root)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// Import File: the path window that picks the file.
pub(crate) fn on_import_file(_activate: On<Activate>, mut request: ResMut<FileWindowRequest>) {
    request.0 = Some(FileWindowKind::PickPath(PathPurposeType::File));
}

/// A file tile's menu row.
pub(crate) fn on_file_verb(
    activate: On<Activate>,
    mut commands: Commands,
    verbs: Query<&FileVerb>,
    mut renaming: ResMut<AssetRenaming>,
    mut request: ResMut<FileWindowRequest>,
    mut says: EditorSays,
) {
    let Ok(FileVerb(verb, file)) = verbs.get(activate.entity) else {
        return;
    };
    let file = file.clone();
    match verb {
        FileVerbType::View => {
            commands.queue(move |world: &mut World| view_file(world, &file));
        }
        FileVerbType::Reveal => {
            commands.queue(move |world: &mut World| reveal_file(world, &file.path));
        }
        FileVerbType::Rename => {
            renaming.0 = Some(RenameTarget::File(file.path));
            says.accept();
        }
        FileVerbType::Delete => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| delete_file(world, &file.path));
            });
        }
        FileVerbType::Include => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| include_file(world, &file.path));
            });
        }
        FileVerbType::Import => {
            request.0 = Some(FileWindowKind::PickPath(PathPurposeType::File));
            says.accept();
        }
    }
}

/// Where the viewer stands: the window's middle, or the corner without one.
fn viewer_at(world: &mut World) -> Vec2 {
    let window = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .map(|window| Vec2::new(window.width(), window.height()));
    window.map_or(Vec2::splat(80.0), |size| {
        ((size - VIEW_MAX) / 2.0).max(Vec2::ZERO)
    })
}

/// Stand the viewer up with `picture` and the file's path under it.
fn open_viewer(world: &mut World, file: &str, picture: impl Bundle) {
    let at = viewer_at(world);
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<PopupRoot>>()
        .iter(world)
        .collect();
    for root in roots {
        world.entity_mut(root).despawn();
    }
    let mut commands = world.commands();
    let root = spawn_popup(&mut commands, at, layer::MENU_Z, |panel| {
        panel.spawn((Name::new("File Viewer Picture"), picture));
        panel.spawn((
            Name::new("File Viewer Path"),
            pane_text(file, nova_ui::theme::density::TEXT, UiColorType::TextMuted),
        ));
    });
    commands.entity(root).insert((
        Name::new("File Viewer"),
        DespawnOnExit(ExampleStates::Editor),
    ));
    world.flush();
}

/// View `file`: an image or a mesh large, a sound played. Any other kind says
/// where the file is.
pub(crate) fn view_file(world: &mut World, file: &ModFile) {
    if !file.on_disk {
        refuse(world, format!("{} is not in the mod's folder", file.path));
        return;
    }
    let Some(id) = world
        .get_resource::<ModProject>()
        .and_then(|project| project.id.clone())
    else {
        return;
    };
    let path = file_asset_path(&id, &file.path);
    match file_kind(&file.path) {
        IMAGE_KIND if !loads_as_image(&file.path) => {
            refuse(world, format!("the game does not read {}", file.path));
        }
        IMAGE_KIND => {
            let image: Handle<Image> = world.resource::<AssetServer>().load(path);
            open_viewer(
                world,
                &file.path,
                (
                    ViewedImage(image.clone()),
                    ImageNode::new(image),
                    Node {
                        width: px(VIEW_MAX.x),
                        height: px(VIEW_MAX.y),
                        ..default()
                    },
                ),
            );
        }
        MESH_KIND => {
            let parts = mesh_parts(&id, &file.path);
            let key = thumbnail_key(&file.path, &parts);
            let Some(picture) = world.resource::<AssetThumbnails>().picture(key).cloned() else {
                refuse(world, format!("{} has no picture yet", file.path));
                return;
            };
            open_viewer(
                world,
                &file.path,
                (
                    ImageNode::new(picture),
                    Node {
                        width: px(VIEW_MAX.y),
                        height: px(VIEW_MAX.y),
                        ..default()
                    },
                ),
            );
        }
        AUDIO_KIND if !plays(&file.path) => {
            refuse(world, format!("the game does not play {}", file.path));
        }
        AUDIO_KIND => {
            let sound = world.resource::<AssetServer>().load(path);
            world.trigger(PlaySfx::new(sound, AudioRoute::Interface));
            note(world, format!("playing {}", file.path));
        }
        _ => note(
            world,
            format!(
                "{}: no viewer for this file; Reveal Path says where it is",
                file.path
            ),
        ),
    }
}

/// Size the viewer's picture to its image once the image is in.
pub(crate) fn fit_viewed_images(
    images: Res<Assets<Image>>,
    mut viewed: Query<(&ViewedImage, &mut Node)>,
) {
    for (ViewedImage(handle), mut node) in &mut viewed {
        let Some(image) = images.get(handle) else {
            continue;
        };
        let size = fit(image.size_f32(), VIEW_MAX);
        let (width, height) = (px(size.x), px(size.y));
        if node.width != width || node.height != height {
            node.width = width;
            node.height = height;
        }
    }
}
