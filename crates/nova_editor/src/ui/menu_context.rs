//! What every menu offers where the editor stands.
//!
//! One rule decides each menu verb, from the open document and its view
//! ([`MenuContext`]): a verb that cannot apply there is HIDDEN, a verb that
//! fits but cannot run now is GREYED with the reason on hover, and the rest are
//! live ([`verb_state`]). The bar menus, their rows and every right-click menu
//! (Hierarchy, stage, canvas, Assets tiles) ask it, so no menu holds a rule of
//! its own.

use bevy::{ecs::system::SystemParam, prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_modding::prelude::Content;
use nova_ship::prelude::{GameSections, HeldPart};
use nova_ui::prelude::Tooltip;

use super::{
    asset_files::{FileFacts, FileVerbType},
    assets::AssetVerbType,
    menu::{MenuDropdown, MenuId, MenuItemLabel, OpenMenu},
};
use crate::{
    asset::{
        has_mesh, mesh_of,
        verbs::{add_socket, set_collider},
        AssetNode,
    },
    bundle::FileWindowRequest,
    clipboard::EditorClipboard,
    config::{EditorViewType, SelectedNode},
    inspect::{item_text, AssetTextType},
    node::{EditContext, SectionNode},
    placement::{deletable, DeletableNode},
    project::{store::CAN_SAVE, verbs::pressed, DocumentType, ModProject, KINDS},
};

/// The collider shapes a section can author, in the order Add lists them.
pub(crate) const COLLIDER_SHAPES: [&str; 4] = ["Cuboid", "Sphere", "Capsule", "Cylinder"];

/// One verb a menu offers. A bar menu's row carries it, and
/// [`sync_menu_verbs`] shows, greys or hides it by [`verb_state`]. The Add
/// menu and the right-click menus read [`verb_state`] when they open.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Tooltip)]
pub(crate) enum MenuVerbType {
    /// A whole bar menu, button and dropdown.
    Menu(MenuId),
    /// Edit > Copy.
    Copy,
    /// Edit > Paste.
    Paste,
    /// Edit > Duplicate, and the Hierarchy row's.
    Duplicate,
    /// Edit > Delete.
    Delete,
    /// Edit > Select All.
    SelectAll,
    /// View > Local Axes: the move handles' space.
    HandleAxes,
    /// View > Key Legend.
    KeyLegend,
    /// View > Link Points.
    LinkPoints,
    /// View > World Grid.
    WorldGrid,
    /// View > Object Volumes.
    ObjectVolumes,
    /// View > Ids: the tree names nodes by id.
    Ids,
    /// View > Frame Selection and the axis views.
    Frame,
    /// Ship > Parts.
    Parts,
    /// Ship > Roll, Cycle and Put Down: the verbs of a part in hand.
    PosePart,
    /// Ship > Rebind Key.
    Rebind,
    /// Add in a scenario's stage: its ships, objects, lights and markers.
    AddWorld,
    /// Add in a ship: the part categories.
    AddPart,
    /// The Hierarchy's Add Part on a ship row: it enters the ship and opens
    /// the parts, so the scenario offers it too.
    EnterShipParts,
    /// Add in a section: its mesh, its collider and its sockets.
    AddSectionPart,
    /// Add > Mesh: the import window for the section's one mesh.
    AddMesh,
    /// Add > a collider shape, one of [`COLLIDER_SHAPES`].
    AddCollider(&'static str),
    /// Add > Socket.
    AddSocket,
    /// The canvas's Clean Up: the scripts in one column.
    CleanUp,
    /// A row of a New Asset tree: a blank item of this kind, open.
    NewAsset(&'static str),
    /// A verb of an Assets tile, and the kind of what the tile holds.
    Tile(AssetVerbType, &'static str, bool),
    /// A verb of an Assets file tile, and what is known of the file.
    File(FileVerbType, FileFacts),
}

/// What the hover of a verb says while it is live. A greyed verb says why
/// instead; a verb with no hint says nothing while live.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct VerbHint(pub(crate) &'static str);

/// What a verb shows as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VerbState {
    /// Not offered here: the verb has nothing to act on in this document.
    Hidden,
    /// Offered, but it cannot run now, for this reason.
    Greyed(String),
    /// Offered and pressable.
    Live,
}

impl VerbState {
    /// Whether a press would run the verb.
    pub(crate) fn is_live(&self) -> bool {
        *self == Self::Live
    }
}

/// A section's mesh, as the Add menu reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MeshSlotType {
    /// A turret: a mesh on each joint, none on the section.
    PerJoint,
    /// No mesh: the kind's built-in one draws.
    Empty,
    /// A mesh is set.
    Set,
}

/// The open section's parts, as the Add menu reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SectionFacts {
    /// Its mesh.
    pub(crate) mesh: MeshSlotType,
    /// Its collider's shape word, or `None` while none is authored.
    pub(crate) collider: Option<&'static str>,
}

/// Where the editor stands, as every menu reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuContext {
    /// The open document.
    pub(crate) document: DocumentType,
    /// Its view.
    pub(crate) view: EditorViewType,
    /// Something is selected.
    pub(crate) selected: bool,
    /// The whole selection can be deleted.
    pub(crate) deletable: bool,
    /// There is something to put the camera on.
    pub(crate) framable: bool,
    /// The selection is a part of the entered ship that takes a key.
    pub(crate) bindable: bool,
    /// A part is in hand.
    pub(crate) holding: bool,
    /// The clipboard holds something.
    pub(crate) clipboard: bool,
    /// The mod has a folder, so a file can be copied into it.
    pub(crate) saved: bool,
    /// The open section's parts, when a section is open.
    pub(crate) section: Option<SectionFacts>,
    /// The mod states its one price table.
    pub(crate) price_table: bool,
}

/// What hiding, greying and showing reads from the world, beside the
/// selection: a menu that opens on a right-click marks its row first, so the
/// caller hands the selection in.
#[derive(SystemParam)]
pub(crate) struct MenuFacts<'w, 's> {
    project: Option<Res<'w, ModProject>>,
    context: Res<'w, EditContext>,
    view: Res<'w, EditorViewType>,
    held: Res<'w, HeldPart>,
    clipboard: Res<'w, EditorClipboard>,
    catalog: Option<Res<'w, GameSections>>,
    deletes: Query<'w, 's, (), DeletableNode>,
    sections: Query<'w, 's, (&'static SectionNode, &'static ChildOf)>,
    benches: Query<'w, 's, &'static AssetNode>,
}

impl MenuFacts<'_, '_> {
    /// The context now, with `selected` the selection.
    pub(crate) fn context(&self, selected: &SelectedNode) -> MenuContext {
        let open = self
            .project
            .as_ref()
            .and_then(|project| project.data.open.as_ref());
        let section = self
            .context
            .scenario()
            .and_then(|root| self.benches.get(root).ok())
            .and_then(|bench| section_facts(&bench.0));
        MenuContext {
            document: DocumentType::of(open),
            view: *self.view,
            selected: !selected.is_empty(),
            deletable: !selected.is_empty()
                && selected
                    .iter()
                    .all(|node| deletable(node, &self.context, &self.deletes)),
            framable: selected
                .primary()
                .or_else(|| self.context.current())
                .is_some(),
            bindable: selected.primary().is_some_and(|node| {
                self.sections.get(node).is_ok_and(|(section, owner)| {
                    self.context.ship() == Some(owner.parent())
                        && section.bindable(self.catalog.as_deref())
                })
            }),
            holding: self.held.0.is_some(),
            clipboard: !self.clipboard.is_empty(),
            saved: self
                .project
                .as_ref()
                .is_some_and(|project| project.id.is_some()),
            section,
            price_table: self
                .project
                .as_ref()
                .is_some_and(|project| !project.data.ids_of("price table").is_empty()),
        }
    }
}

/// The parts of `item`, when it is a section.
pub(crate) fn section_facts(item: &Content) -> Option<SectionFacts> {
    let Content::Section(config) = item else {
        return None;
    };
    let mesh = if !has_mesh(&config.kind) {
        MeshSlotType::PerJoint
    } else if mesh_of(&config.kind).is_empty() {
        MeshSlotType::Empty
    } else {
        MeshSlotType::Set
    };
    // "Unit Cube", the word for no authored collider, is none of the shapes.
    let shape = item_text(item, AssetTextType::ColliderShape);
    let collider = COLLIDER_SHAPES.into_iter().find(|word| *word == shape);
    Some(SectionFacts { mesh, collider })
}

/// How `verb` shows in `context`. The one place a menu rule lives.
pub(crate) fn verb_state(verb: MenuVerbType, context: &MenuContext) -> VerbState {
    use DocumentType as Doc;
    use EditorViewType as View;
    use MenuVerbType as Verb;
    use VerbState::{Greyed, Hidden, Live};

    let greyed = |reason: &str| Greyed(reason.to_string());
    let live_if = |live: bool, reason: &str| if live { Live } else { greyed(reason) };
    let stage = matches!(context.view, View::Scene | View::Ship | View::Section);
    let document = context.document;
    match verb {
        Verb::Menu(MenuId::Ship) if document != Doc::Ship => Hidden,
        Verb::Menu(_) => Live,
        Verb::Copy | Verb::Duplicate => match document {
            Doc::Scenario => live_if(context.selected, "select a ship, an object or a block"),
            Doc::Ship => greyed("a part mates to a socket - press P over it to arm another"),
            Doc::Section | Doc::Form => Hidden,
        },
        Verb::Paste => match document {
            Doc::Scenario => live_if(context.clipboard, "copy something first"),
            Doc::Ship => greyed("paste works on a scenario - go Back to one first"),
            Doc::Section | Doc::Form => Hidden,
        },
        Verb::Delete => match document {
            Doc::Form => Hidden,
            Doc::Section => live_if(context.deletable, "select a socket to delete"),
            Doc::Scenario | Doc::Ship => {
                live_if(context.deletable, "select something that can be deleted")
            }
        },
        Verb::SelectAll => match document {
            Doc::Scenario | Doc::Ship => Live,
            Doc::Section | Doc::Form => Hidden,
        },
        Verb::HandleAxes | Verb::KeyLegend | Verb::WorldGrid if stage => Live,
        Verb::LinkPoints if matches!(context.view, View::Ship | View::Section) => Live,
        Verb::ObjectVolumes if context.view == View::Scene => Live,
        Verb::Ids if matches!(context.view, View::Scene | View::Events | View::Ship) => Live,
        Verb::Frame if stage => live_if(context.framable, "nothing to frame yet"),
        Verb::HandleAxes
        | Verb::KeyLegend
        | Verb::WorldGrid
        | Verb::LinkPoints
        | Verb::ObjectVolumes
        | Verb::Ids
        | Verb::Frame => Hidden,
        Verb::Parts if document == Doc::Ship => Live,
        Verb::PosePart if document == Doc::Ship => {
            live_if(context.holding, "pick a part up first: Parts... or Q")
        }
        Verb::Rebind if document == Doc::Ship => live_if(
            context.bindable,
            "select a part that takes a key, such as a turret",
        ),
        Verb::Parts | Verb::PosePart | Verb::Rebind => Hidden,
        Verb::AddWorld if context.view == View::Scene => Live,
        Verb::AddPart if context.view == View::Ship => Live,
        Verb::EnterShipParts if matches!(context.view, View::Scene | View::Ship) => Live,
        Verb::AddWorld | Verb::AddPart | Verb::EnterShipParts => Hidden,
        Verb::AddSectionPart | Verb::AddSocket => match context.section {
            Some(_) => Live,
            None => Hidden,
        },
        // An Add row that would refuse is not offered: a turret's meshes sit
        // on its joints, a mesh is copied into the mod's folder, which an
        // unsaved mod does not have, and the held shape is set already.
        Verb::AddMesh => match context.section.map(|section| section.mesh) {
            Some(MeshSlotType::Empty | MeshSlotType::Set) if context.saved => Live,
            _ => Hidden,
        },
        Verb::AddCollider(shape) => match context.section.map(|section| section.collider) {
            None => Hidden,
            Some(Some(held)) if held == shape => Hidden,
            Some(_) => Live,
        },
        Verb::NewAsset(kind) => new_asset_state(kind, context),
        Verb::CleanUp if context.view == View::Events => Live,
        Verb::CleanUp => Hidden,
        Verb::Tile(verb, kind, in_scenario) => {
            tile_state(verb, kind, in_scenario, context.document)
        }
        Verb::File(verb, file) => file_state(verb, file),
    }
}

/// How New Asset > `kind` shows. A row is offered only where choosing it makes
/// an item that opens, saves and lints; anything else is hidden, never greyed:
/// a kind the editor does not make ([`KINDS`]), and a second price table.
fn new_asset_state(kind: &str, context: &MenuContext) -> VerbState {
    let made = KINDS.iter().any(|row| row.kind == kind && row.add);
    let second_table = kind == "price table" && context.price_table;
    if made && !second_table {
        VerbState::Live
    } else {
        VerbState::Hidden
    }
}

/// How a verb of a file tile shows for `file`. A build that writes no folder
/// offers no verb that writes one.
fn file_state(verb: FileVerbType, file: FileFacts) -> VerbState {
    use FileVerbType as File;
    let gone = || VerbState::Greyed("the file is not in the mod's folder".to_string());
    match verb {
        File::Reveal => VerbState::Live,
        File::Rename | File::Import if !CAN_SAVE => VerbState::Hidden,
        File::Include if file.packaged => VerbState::Hidden,
        File::View | File::Rename | File::Include if !file.on_disk => gone(),
        File::View | File::Rename | File::Include | File::Delete | File::Import => VerbState::Live,
    }
}

/// How a tile's `verb` shows on a tile of `kind`. `in_scenario` is a design a
/// scenario owns: the ship of one of its nodes.
fn tile_state(
    verb: AssetVerbType,
    kind: &str,
    in_scenario: bool,
    document: DocumentType,
) -> VerbState {
    use AssetVerbType as Tile;
    match (verb, kind) {
        (Tile::Open, _) => VerbState::Live,
        // A placement names a design. A scenario's own ship is not one: its
        // design is written for that one spawn.
        (Tile::Place, "ship") if !in_scenario => match document {
            DocumentType::Scenario => VerbState::Live,
            _ => VerbState::Greyed("open a scenario to place the ship in".to_string()),
        },
        (Tile::Place, _) => VerbState::Hidden,
        // A mod states one price table: it has no id to change, and no second
        // to copy into.
        (Tile::Rename | Tile::Duplicate, "price table") => VerbState::Hidden,
        // A scenario's design is renamed, copied and deleted as the ship it
        // is, in its scenario: the id is the scenario's and the node's.
        _ if in_scenario => VerbState::Hidden,
        _ => VerbState::Live,
    }
}

/// The label of `verb` in `context`, where it changes with what is set: a
/// section holds one mesh and one collider, so Add adds the first and the
/// rows then say Replace.
pub(crate) fn verb_label(verb: MenuVerbType, context: &MenuContext) -> Option<String> {
    let section = context.section?;
    match verb {
        MenuVerbType::AddMesh => Some(match section.mesh {
            MeshSlotType::Set => "Replace Mesh...".to_string(),
            MeshSlotType::Empty | MeshSlotType::PerJoint => "Mesh...".to_string(),
        }),
        MenuVerbType::AddCollider(shape) => Some(match section.collider {
            Some(_) => format!("Replace Collider with {shape}"),
            None => format!("{shape} Collider"),
        }),
        _ => None,
    }
}

/// Show, grey or hide every menu verb, write the reason it is greyed onto its
/// hover, and name the rows whose label follows what is set. A menu that hides
/// while open closes.
pub(crate) fn sync_menu_verbs(
    mut commands: Commands,
    facts: MenuFacts,
    selected: Res<SelectedNode>,
    mut open: ResMut<OpenMenu>,
    mut verbs: Query<(
        Entity,
        &MenuVerbType,
        &mut Node,
        &mut Tooltip,
        Option<&VerbHint>,
        Has<InteractionDisabled>,
        Option<&Children>,
    )>,
    mut labels: Query<&mut Text, With<MenuItemLabel>>,
) {
    let context = facts.context(&selected);
    for (entity, verb, mut node, mut tooltip, hint, disabled, children) in &mut verbs {
        let state = verb_state(*verb, &context);
        let display = if state == VerbState::Hidden {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        let reason = match &state {
            VerbState::Greyed(reason) => reason.as_str(),
            VerbState::Hidden | VerbState::Live => hint.map_or("", |hint| hint.0),
        };
        if tooltip.0 != reason {
            tooltip.0 = reason.to_string();
        }
        // A bar menu is never pressed itself; its rows are.
        if matches!(verb, MenuVerbType::Menu(_)) {
            if let MenuVerbType::Menu(menu) = verb {
                if state == VerbState::Hidden && open.0 == Some(*menu) {
                    open.0 = None;
                }
            }
            continue;
        }
        match (state.is_live(), disabled) {
            (true, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            (false, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            _ => {}
        }
        let Some(wanted) = verb_label(*verb, &context) else {
            continue;
        };
        for child in children.into_iter().flatten() {
            if let Ok(mut label) = labels.get_mut(*child) {
                if label.0 != wanted {
                    label.0.clone_from(&wanted);
                }
            }
        }
    }
}

/// A separator in a menu, which [`tidy_menu_separators`] hides when no shown
/// row stands on one side of it.
#[derive(Component)]
pub(crate) struct MenuSeparator;

/// Hide each separator that would rule off nothing: one at either end of the
/// shown rows, or a second in a row. Run after [`sync_menu_verbs`].
pub(crate) fn tidy_menu_separators(
    parents: Query<&Children, Or<(With<MenuDropdown>, With<MenuVerbType>)>>,
    separators: Query<(), With<MenuSeparator>>,
    mut nodes: Query<&mut Node>,
) {
    for children in &parents {
        let mut wanted = Vec::new();
        let mut row_above = false;
        let mut pending = None;
        for child in children.iter() {
            if separators.contains(child) {
                if row_above && pending.is_none() {
                    pending = Some(child);
                } else {
                    wanted.push((child, Display::None));
                }
                continue;
            }
            let shown = nodes
                .get(child)
                .is_ok_and(|node| node.display != Display::None);
            if shown {
                if let Some(separator) = pending.take() {
                    wanted.push((separator, Display::Flex));
                }
                row_above = true;
            }
        }
        if let Some(separator) = pending {
            wanted.push((separator, Display::None));
        }
        for (separator, display) in wanted {
            if let Ok(mut node) = nodes.get_mut(separator) {
                if node.display != display {
                    node.display = display;
                }
            }
        }
    }
}

/// Add > Mesh: the window that picks the `.glb` to import.
pub(crate) fn on_add_mesh(_activate: On<Activate>, mut request: ResMut<FileWindowRequest>) {
    request.0 = Some(crate::bundle::FileWindowKind::PickPath(
        super::files::PathPurposeType::Mesh,
    ));
}

/// The collider shape an Add row sets, one of [`COLLIDER_SHAPES`].
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ColliderShape(pub(crate) &'static str);

/// Add > Collider > a shape: the section's collider becomes it, one step.
pub(crate) fn on_add_collider(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&ColliderShape>,
) {
    let Ok(ColliderShape(shape)) = rows.get(activate.entity).copied() else {
        return;
    };
    commands.queue(move |world: &mut World| pressed(world, |world| set_collider(world, shape)));
}

/// Add > Socket: one more socket on a free face, one step.
pub(crate) fn on_add_socket(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, add_socket));
}

#[cfg(test)]
pub(crate) mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::{BaseSectionConfig, SectionConfig, SectionKind, TurretSectionConfig};

    use super::*;
    use crate::node::ShipNode;

    /// A context of `document` in `view`, with nothing selected.
    pub(crate) fn context(document: DocumentType, view: EditorViewType) -> MenuContext {
        MenuContext {
            document,
            view,
            selected: false,
            deletable: false,
            framable: true,
            bindable: false,
            holding: false,
            clipboard: false,
            saved: true,
            section: (document == DocumentType::Section).then_some(SectionFacts {
                mesh: MeshSlotType::Empty,
                collider: None,
            }),
            price_table: false,
        }
    }

    /// Add holds only what the open document takes: a scenario's world, a
    /// ship's parts, a section's own parts. Events and a form add only a new
    /// asset, which every document offers. The Ship menu shows only in a ship.
    #[test]
    fn each_document_offers_only_its_own_add_verbs() {
        use DocumentType as Doc;
        use EditorViewType as View;
        use MenuVerbType as Verb;

        let verbs = [
            Verb::AddWorld,
            Verb::AddPart,
            Verb::AddSectionPart,
            Verb::Menu(MenuId::Add),
            Verb::Menu(MenuId::Ship),
        ];
        for (document, view, shown) in [
            (
                Doc::Scenario,
                View::Scene,
                vec![Verb::AddWorld, Verb::Menu(MenuId::Add)],
            ),
            (Doc::Scenario, View::Events, vec![Verb::Menu(MenuId::Add)]),
            (
                Doc::Ship,
                View::Ship,
                vec![
                    Verb::AddPart,
                    Verb::Menu(MenuId::Add),
                    Verb::Menu(MenuId::Ship),
                ],
            ),
            (
                Doc::Section,
                View::Section,
                vec![Verb::AddSectionPart, Verb::Menu(MenuId::Add)],
            ),
            (Doc::Form, View::Form, vec![Verb::Menu(MenuId::Add)]),
        ] {
            let at = context(document, view);
            for verb in verbs {
                let hidden = verb_state(verb, &at) == VerbState::Hidden;
                assert_eq!(
                    !hidden,
                    shown.contains(&verb),
                    "{verb:?} in {document:?} {view:?}"
                );
            }
        }
    }

    /// A section holds one mesh and at most one collider: Add sets the first,
    /// then the rows say Replace. A row that would refuse is hidden: the shape
    /// it has already, a turret's mesh, which sits on the joints, and a mesh in
    /// a mod with no folder to copy it into.
    #[test]
    fn a_section_part_reads_add_until_it_is_set_then_replace() {
        let mut at = context(DocumentType::Section, EditorViewType::Section);
        let sphere = MenuVerbType::AddCollider("Sphere");
        assert_eq!(verb_label(sphere, &at).as_deref(), Some("Sphere Collider"));
        assert_eq!(
            verb_label(MenuVerbType::AddMesh, &at).as_deref(),
            Some("Mesh...")
        );

        at.section = Some(SectionFacts {
            mesh: MeshSlotType::Set,
            collider: Some("Sphere"),
        });
        assert_eq!(
            verb_label(MenuVerbType::AddCollider("Cuboid"), &at).as_deref(),
            Some("Replace Collider with Cuboid")
        );
        assert_eq!(
            verb_label(MenuVerbType::AddMesh, &at).as_deref(),
            Some("Replace Mesh...")
        );
        assert_eq!(verb_state(sphere, &at), VerbState::Hidden);

        at.section = Some(SectionFacts {
            mesh: MeshSlotType::PerJoint,
            collider: None,
        });
        assert_eq!(verb_state(MenuVerbType::AddMesh, &at), VerbState::Hidden);
        at.saved = false;
        at.section = Some(SectionFacts {
            mesh: MeshSlotType::Empty,
            collider: None,
        });
        assert_eq!(verb_state(MenuVerbType::AddMesh, &at), VerbState::Hidden);
    }

    /// The sync greys a verb that fits but cannot run, with the reason on its
    /// hover, and frees it once it can: Rebind waits for a part that takes a
    /// key. A separator with no shown row below it hides.
    #[test]
    fn a_greyed_verb_says_why_until_it_can_run() {
        let section = |kind: SectionKind| SectionNode {
            source: SectionSource::Inline(SectionConfig {
                base: BaseSectionConfig {
                    id: "part".to_string(),
                    name: "part".to_string(),
                    ..default()
                },
                kind,
            }),
            binds: vec![],
        };
        let mut world = World::new();
        world.init_resource::<SelectedNode>();
        world.init_resource::<HeldPart>();
        world.init_resource::<EditorClipboard>();
        world.init_resource::<OpenMenu>();
        world.insert_resource(EditorViewType::Ship);
        let mut project = crate::project::ModProject::new(
            None,
            "Scratch",
            "",
            crate::template::ScenarioTemplate::default(),
        );
        std::sync::Arc::make_mut(&mut project.data).open = Some(crate::project::OpenAsset {
            kind: "ship",
            id: "range.ship_1".to_string(),
        });
        world.insert_resource(project);
        let ship = world.spawn(ShipNode::default()).id();
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });
        let hull = world
            .spawn((section(SectionKind::Hull(default())), ChildOf(ship)))
            .id();
        let turret = world
            .spawn((
                section(SectionKind::Turret(TurretSectionConfig::default())),
                ChildOf(ship),
            ))
            .id();
        let menu = world.spawn((MenuDropdown, Node::default())).id();
        let rebind = world
            .spawn((MenuVerbType::Rebind, Node::default(), ChildOf(menu)))
            .id();
        let rule = world
            .spawn((MenuSeparator, Node::default(), ChildOf(menu)))
            .id();
        world.spawn((MenuVerbType::Copy, Node::default(), ChildOf(menu)));
        let run = |world: &mut World| {
            world
                .run_system_once(sync_menu_verbs)
                .expect("the sync runs");
            world
                .run_system_once(tidy_menu_separators)
                .expect("the tidy runs");
        };

        world.resource_mut::<SelectedNode>().set(Some(hull));
        run(&mut world);
        assert!(world.entity(rebind).contains::<InteractionDisabled>());
        assert!(
            world
                .get::<Tooltip>(rebind)
                .is_some_and(|hover| hover.0.contains("takes a key")),
            "a hull takes no key, and the hover says what would"
        );
        assert_eq!(
            world.get::<Node>(rule).map(|node| node.display),
            Some(Display::Flex),
            "Copy is greyed in a ship, not hidden, so the rule has a row below"
        );

        world.resource_mut::<SelectedNode>().set(Some(turret));
        run(&mut world);
        assert!(!world.entity(rebind).contains::<InteractionDisabled>());
        assert_eq!(
            world.get::<Tooltip>(rebind).map(|hover| hover.0.as_str()),
            Some("")
        );

        world.insert_resource(EditorViewType::Scene);
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER],
        });
        world
            .resource_mut::<crate::project::ModProject>()
            .edit()
            .open = Some(crate::project::OpenAsset {
            kind: crate::project::SCENARIO_KIND,
            id: "range".to_string(),
        });
        run(&mut world);
        assert_eq!(
            world.get::<Node>(rebind).map(|node| node.display),
            Some(Display::None),
            "in a scenario, the ship verbs are not offered"
        );
    }
}
