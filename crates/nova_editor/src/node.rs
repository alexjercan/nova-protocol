//! The editor's document: a tree of NODES, and the context you are inside.
//!
//! Everything the editor edits is a node entity. A [`ScenarioNode`] holds
//! [`ShipNode`]s and [`ObjectNode`]s, a ship holds [`SectionNode`]s, and each
//! node carries its own config as a component - so "the ship being built" is not
//! a resource anywhere, it is a subtree. Two ships cost nothing but two
//! subtrees, and the rocks, beacons and lights standing around them are the
//! same kind of thing one level up.
//!
//! MODEL AND VIEW ARE SEPARATE ENTITIES. A node is data and persists across
//! [`ExampleStates`]; its [`NodeView`] child carries the mesh, the collider and
//! the picking, and is `DespawnOnExit(Editor)`. That split is what lets Play
//! leave the editor and come back to the same document, and it is why a node can
//! never carry a collider into the flown scenario: the thing with the collider
//! is the thing that gets despawned.
//!
//! Picking therefore lands on a VIEW. Every pointer path maps it back with
//! [`node_of_view`].

use std::collections::BTreeMap;

use bevy::{prelude::*, ui_widgets::Activate};
use bevy_enhanced_input::prelude::Binding;
use nova_gameplay::prelude::{Allegiance, AssetRef};
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::{
    bundle::{insert_lifted_ship, lift_objects},
    config::{EditorSays, SelectedNode},
    gallery::EditorCamera,
    preview::{insert_preview_object, insert_preview_section, PreviewArt, PreviewRole},
    scenario::default_world_objects,
    ExampleStates,
};

/// How far apart two ship nodes sit on the stage. Wide enough that the biggest
/// hull anyone builds by hand does not reach its neighbour.
const SHIP_NODE_SPACING: f32 = 24.0;

/// The asset paths an object node's config points at.
///
/// DIRECT paths, not `self://` or `dep://`: the editor's document is built at
/// runtime outside the mod merge, so a scheme ref placed here would never be
/// rewritten and would resolve to nothing.
pub(crate) const ASTEROID_TEXTURE: &str = "base/textures/asteroid.png";
/// The sound a hit on a placed rock plays.
pub(crate) const IMPACT_SOUND: &str = "base/sounds/impact.wav";
/// The sound a placed rock's destruction plays.
pub(crate) const DESTROY_SOUND: &str = "base/sounds/explosion.wav";
/// The ding a placed salvage crate is picked up with.
pub(crate) const SALVAGE_SOUND: &str = "base/sounds/salvage_pickup.wav";

/// Marker on every node of the document tree, at every depth.
#[derive(Component, Debug)]
pub(crate) struct EditorNode;

/// A node's stable id.
///
/// Minted once when the node is created and never re-derived, because it is what
/// a saved file and its input mapping are keyed by. Ids used to BE the live
/// entity id stringified, which made a saved file impossible: entity ids do not
/// survive a process restart, and they did not even survive leaving the editor
/// and coming back.
///
/// Unique within the PARENT, which is exactly the scope both consumers need -
/// `input_mapping` keys are per hull, and `BaseScenarioObjectConfig::id` is per
/// scenario.
#[derive(Component, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodeId(pub(crate) String);

/// The next ordinal this node hands out to a child id.
///
/// Monotonic, never reused: deleting `thruster_2` and placing another thruster
/// mints `thruster_3`. Reuse would let a stale reference to the old id silently
/// attach to the new section.
#[derive(Component, Debug, Default)]
pub(crate) struct NextChildOrdinal(pub(crate) u32);

/// The outermost node: the scenario being edited. One per document, and the
/// context the editor opens in.
#[derive(Component, Debug)]
pub(crate) struct ScenarioNode;

/// Who drives a ship once the scenario runs.
///
/// A property of the SHIP, not of the edit context: "which ship do I fly" is
/// answered by the document, so entering a ship to work on it cannot change
/// which one Play hands to the player.
///
/// The same three states [`SpaceshipController`] has, because they answer the
/// same question. A hull with nobody at the controls is what every derelict in
/// a scenario is, and the editor could not say it until the seeded ones became
/// nodes of this kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ShipDriver {
    /// The ship the player flies on Play.
    #[default]
    Player,
    /// Driven by the AI once the scenario runs.
    Ai,
    /// Nobody drives it: it station-keeps, takes damage and comes apart.
    Adrift,
}

/// The side a ship of this kind stands on when nothing has chosen one.
///
/// AI ships stand NEUTRAL. A live pilot with no HOSTILE contact holds station,
/// so a ship added beside the player's neither opens fire off the line nor
/// gets shot by a woken picket - which is what "I put a second ship on the
/// range" means. The engine's own default for an AI ship is `Enemy`, and a
/// document that said nothing used to get that by accident.
///
/// A player ship and a derelict say nothing and take the engine's default:
/// `Allegiance::Player` for one, none at all for the other.
pub(crate) fn default_allegiance(driver: ShipDriver) -> Option<Allegiance> {
    match driver {
        ShipDriver::Ai => Some(Allegiance::Neutral),
        ShipDriver::Player | ShipDriver::Adrift => None,
    }
}

/// A ship being built: everything about it that is not one of its sections.
///
/// Also what a seeded hull is held as - a picket and a target hulk are ships
/// the document did not mint, and holding them as anything else meant a double
/// click could not go inside one. The three fields below the style are what a
/// spawn says about a hull that the hull itself does not: `driver`,
/// `allegiance` and `pilot` are exactly the parts of [`SpaceshipConfig`] that
/// are not its section list.
#[derive(Component, Debug, Clone)]
pub(crate) struct ShipNode {
    /// What the ship is CALLED: the name the tree, the breadcrumb and the
    /// flown scenario all read. Empty where nothing named it, and every
    /// surface falls back to the node's minted id.
    pub(crate) name: String,
    /// Whether the ship wears its derived skin - shown live in the build
    /// view (see [`crate::skin`]) and carried through to the flown ship, so what
    /// the builder sees is what they fly.
    pub(crate) skin: bool,
    /// The style id the skin wears, or `None` for the first style the
    /// content merge loaded.
    pub(crate) style: Option<String>,
    /// Who drives it once the scenario runs.
    pub(crate) driver: ShipDriver,
    /// Which side it fights for, or `None` to take the driver's own default -
    /// Player ships read `Allegiance::Player`, AI ships `Allegiance::Enemy`.
    /// `Some(Neutral)` is what makes a picket dormant.
    pub(crate) allegiance: Option<Allegiance>,
    /// The AI pilot's standing orders - patrol, orbit, leash, grace. Carried
    /// whole rather than picked apart because it is the scenario's own type and
    /// a picket's leash has to survive a trip through the document. Ignored
    /// unless `driver` is [`ShipDriver::Ai`].
    pub(crate) pilot: AIControllerConfig,
}

impl Default for ShipNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            // Off, as the build state's `skin` was: a new ship starts bare.
            skin: false,
            style: None,
            driver: ShipDriver::Player,
            allegiance: None,
            pilot: AIControllerConfig::default(),
        }
    }
}

/// One section placed on a ship. Its POSE is the node's `Transform` rather than
/// a field, because the node is an entity and that is where an entity keeps its
/// pose.
#[derive(Component, Debug, Clone)]
pub(crate) struct SectionNode {
    /// Where the section's config comes from: inline, or a catalog prototype by
    /// id. The editor writes `Inline` today; `Prototype` resolves through
    /// [`SectionNode::resolve`] rather than being dropped, which is what the old
    /// rebuild path did to it.
    pub(crate) source: SectionSource,
    /// Data-only deltas applied at spawn. Carried so a lifted section keeps
    /// them; the editor authors none yet.
    pub(crate) modifications: Vec<SectionModification>,
    /// The inputs this section fires on. Document data rather than a component
    /// on the view: the view is render-only, and a binding that lived out there
    /// would be a second copy to keep in step across a despawn.
    pub(crate) binds: Vec<Binding>,
}

impl SectionNode {
    /// The section's config, or `None` when a mod overlay dropped the prototype
    /// it names.
    pub(crate) fn resolve<'a>(
        &'a self,
        sections: Option<&'a GameSections>,
    ) -> Option<&'a SectionConfig> {
        match &self.source {
            SectionSource::Inline(config) => Some(config),
            SectionSource::Prototype(id) => sections?.get_section(id),
        }
    }

    /// The catalog id this section was built from - what the pipette arms and
    /// what a minted id is named after.
    pub(crate) fn prototype(&self) -> &str {
        match &self.source {
            SectionSource::Inline(config) => &config.base.id,
            SectionSource::Prototype(id) => id,
        }
    }

    /// Whether this kind of section takes an input binding at all. Hull and
    /// controller sections do not.
    pub(crate) fn bindable(&self, sections: Option<&GameSections>) -> bool {
        self.resolve(sections).is_some_and(|config| {
            !matches!(
                config.kind,
                SectionKind::Hull(_) | SectionKind::Controller(_)
            )
        })
    }
}

/// One non-ship thing the world holds: a rock, a beacon, a salvage crate, an
/// anchor, a light - or a fixed hull the editor does not design.
///
/// A sibling of the ships under the scenario node, because that is what it is:
/// one more object the range spawns on start. Its ID is the node's [`NodeId`]
/// and its POSE is the node's `Transform`, so the lowering reads both from
/// where a drag and a mint already wrote them, and the kind config carries only
/// what is left.
#[derive(Component, Debug, Clone)]
pub(crate) struct ObjectNode {
    /// The display name the spawned object wears.
    pub(crate) name: String,
    /// Which kind it is, and that kind's own config.
    pub(crate) kind: ScenarioObjectKind,
}

/// The object kinds the palette can place, in the order the rail lists them.
///
/// `Spaceship` is deliberately absent: a ship is added with Add Ship and built
/// out of sections, and the fixed hulls the default world stands on are seeded
/// rather than authored here.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectChoice {
    /// An invisible gravity/framing point.
    Anchor,
    /// A destructible rock.
    Asteroid,
    /// A nav waypoint with a HUD chip.
    Beacon,
    /// A proximity pickup.
    SalvageCrate,
    /// One of the scene's own lights.
    Light,
}

impl ObjectChoice {
    /// Every kind the palette offers.
    pub(crate) const ALL: [ObjectChoice; 5] = [
        ObjectChoice::Anchor,
        ObjectChoice::Asteroid,
        ObjectChoice::Beacon,
        ObjectChoice::SalvageCrate,
        ObjectChoice::Light,
    ];

    /// The row label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ObjectChoice::Anchor => "Anchor",
            ObjectChoice::Asteroid => "Asteroid",
            ObjectChoice::Beacon => "Beacon",
            ObjectChoice::SalvageCrate => "Salvage",
            ObjectChoice::Light => "Light",
        }
    }

    /// The stem a minted id is named after, so `asteroid_3` says what it is the
    /// same way `thruster_3` does.
    pub(crate) fn stem(self) -> &'static str {
        match self {
            ObjectChoice::Anchor => "anchor",
            ObjectChoice::Asteroid => "asteroid",
            ObjectChoice::Beacon => "beacon",
            ObjectChoice::SalvageCrate => "salvage",
            ObjectChoice::Light => "light",
        }
    }

    /// A fresh object of this kind: the smallest config that reads as the thing
    /// it is. Every number here is a starting point the builder retunes, not a
    /// tuned value - a placed rock is a rock you can see, not the right rock.
    pub(crate) fn stock(self) -> ObjectNode {
        let kind = match self {
            // Inert: the well exists at zero strength, so a placed anchor frames
            // and anchors without pulling the player into something invisible.
            ObjectChoice::Anchor => ScenarioObjectKind::Anchor(AnchorConfig {
                body_radius: 5.0,
                mass: None,
            }),
            ObjectChoice::Asteroid => ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 3.0,
                texture: AssetRef::from(ASTEROID_TEXTURE),
                impact_sound: Some(AssetRef::from(IMPACT_SOUND)),
                destroy_sound: Some(AssetRef::from(DESTROY_SOUND)),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
            ObjectChoice::Beacon => ScenarioObjectKind::Beacon(BeaconConfig {
                label: "BEACON".to_string(),
                radius: 3.0,
                color: Color::srgb(0.20, 0.90, 1.0),
                area_radius: None,
                lock_signature: None,
            }),
            ObjectChoice::SalvageCrate => ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
                size: 2.0,
                area_radius: 12.0,
                pickup_sound: Some(AssetRef::from(SALVAGE_SOUND)),
            }),
            // Aimed by the NODE's rotation, not by `aim`: the pose lives on the
            // node like every other node's does, and a second aim point in the
            // config would be a copy to keep in step with it.
            ObjectChoice::Light => ScenarioObjectKind::Light(LightConfig::Directional {
                illuminance: 9_000.0,
                color: Color::WHITE,
                shadows: false,
                aim: None,
            }),
        };
        ObjectNode {
            name: self.label().to_string(),
            kind,
        }
    }
}

/// The render half of a node: mesh, collider and picking. Spawned on entering
/// the editor and despawned on leaving it, so the flown scenario never contains
/// one.
#[derive(Component, Debug)]
pub(crate) struct NodeView;

/// The node the editor is INSIDE, as a path from the scenario node down.
///
/// A path rather than one handle so exiting has somewhere to return to: enter
/// pushes, exit pops, and the root can never be popped. Empty until the document
/// exists.
#[derive(Resource, Debug, Default)]
pub(crate) struct EditContext {
    pub(crate) path: Vec<Entity>,
}

impl EditContext {
    /// The scenario node, or `None` before the document exists.
    pub(crate) fn scenario(&self) -> Option<Entity> {
        self.path.first().copied()
    }

    /// The node the editor is inside.
    pub(crate) fn current(&self) -> Option<Entity> {
        self.path.last().copied()
    }

    /// The ship being edited, or `None` out in the scenario context. Depth is
    /// the test because only a ship can be entered today.
    pub(crate) fn ship(&self) -> Option<Entity> {
        (self.path.len() >= 2).then(|| self.path[1])
    }

    /// Go inside `node`.
    ///
    /// A no-op before the document exists: there is nothing to be inside OF,
    /// and pushing onto an empty path would leave a ship node answering as the
    /// scenario node.
    pub(crate) fn enter(&mut self, node: Entity) {
        if self.path.is_empty() || self.current() == Some(node) {
            return;
        }
        // Entering a sibling is an enter, not a nest: drop back to the scenario
        // first so the path is always a real ancestry.
        self.path.truncate(1);
        self.path.push(node);
    }

    /// Come back out one level. The scenario node is the floor.
    pub(crate) fn exit(&mut self) {
        if self.path.len() > 1 {
            self.path.pop();
        }
    }

    /// Straight back to the scenario node, however deep the path is.
    pub(crate) fn exit_all(&mut self) {
        self.path.truncate(1);
    }

    /// Come back out to `node`, if the editor is inside it. Says whether it
    /// was.
    ///
    /// The gesture a click on an ancestor row makes: a row for something you
    /// are already INSIDE cannot mean "go there", so it means "come back out
    /// to there" - one click, at whatever depth, rather than a double on the
    /// root and a key for each rung between.
    pub(crate) fn leave_to(&mut self, node: Entity) -> bool {
        let Some(depth) = self.path.iter().position(|step| *step == node) else {
            return false;
        };
        if depth + 1 == self.path.len() {
            return false;
        }
        self.path.truncate(depth + 1);
        true
    }
}

/// A minted id split into WHAT it is and WHICH one it is: `thruster_section_3`
/// is `("thruster", "3")` and `asteroid_7` is `("asteroid", "7")`. An id nobody
/// minted is all stem and no ordinal.
///
/// One rule, so the order a list is drawn in and the two columns a row is drawn
/// with agree about where an id ends and its number begins.
pub(crate) fn split_ordinal(id: &str) -> (&str, &str) {
    match id.split_once("_section_") {
        Some((stem, ordinal)) => (stem, ordinal),
        None => match id.rsplit_once('_') {
            Some((stem, tail))
                if !tail.is_empty() && tail.chars().all(|digit| digit.is_ascii_digit()) =>
            {
                (stem, tail)
            }
            _ => (id, ""),
        },
    }
}

/// The order minted ids take in a list: the stem, then the ordinal as a NUMBER.
///
/// Sorted as TEXT, `thruster_section_10` landed between `_1` and `_2`, so the
/// ordinal column - the only thing telling six reinforced hulls apart - read
/// `1, 10, 2`. The whole id breaks a tie, so the order is total.
pub(crate) fn id_order(id: &str) -> (&str, u64, &str) {
    let (stem, ordinal) = split_ordinal(id);
    // An ordinal too big for a `u64` sorts last rather than first: it is a
    // number nobody minted, and `unwrap_or_default` would file it under 0.
    (stem, ordinal.parse().unwrap_or(u64::MAX), id)
}

/// Every section node on `ship`, in id order.
///
/// SORTED rather than in query order: an archetype walk hands the same ship over
/// in whatever order its entities were spawned, and both the lowering (whose
/// output is a file) and the solver (whose refusals name section indices) need
/// one answer per ship. This is the sort `sync_editor_skin` used to do by
/// position, for the same reason and against a worse key.
pub(crate) fn sections_of<'a>(
    ship: Entity,
    nodes: &'a SectionNodes,
) -> Vec<(Entity, &'a NodeId, &'a SectionNode, &'a Transform)> {
    let mut found: Vec<_> = nodes
        .iter()
        .filter(|(_, child_of, ..)| child_of.parent() == ship)
        .map(|(entity, _, id, section, transform)| (entity, id, section, transform))
        .collect();
    found.sort_unstable_by(|a, b| id_order(&a.1 .0).cmp(&id_order(&b.1 .0)));
    found
}

/// Read-only access to every section node and its pose.
pub(crate) type SectionNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static NodeId,
        &'static SectionNode,
        &'static Transform,
    ),
>;

/// Read-only access to every ship node.
pub(crate) type ShipNodes<'w, 's> =
    Query<'w, 's, (Entity, &'static ChildOf, &'static NodeId, &'static ShipNode)>;

/// Read-only access to every object node and its pose.
pub(crate) type ObjectNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static NodeId,
        &'static ObjectNode,
        &'static Transform,
    ),
>;

/// Every object node of `scenario`, in id order.
///
/// SORTED for the same reason [`sections_of`] is: the output is a scenario's
/// object list, and a file that reordered itself on every save would diff
/// against itself.
pub(crate) fn objects_of<'a>(
    scenario: Entity,
    nodes: &'a ObjectNodes,
) -> Vec<(Entity, &'a NodeId, &'a ObjectNode, &'a Transform)> {
    let mut found: Vec<_> = nodes
        .iter()
        .filter(|(_, child_of, ..)| child_of.parent() == scenario)
        .map(|(entity, _, id, object, transform)| (entity, id, object, transform))
        .collect();
    found.sort_unstable_by(|a, b| id_order(&a.1 .0).cmp(&id_order(&b.1 .0)));
    found
}

/// One node the current edit context contains.
pub(crate) struct ContextNode<'a> {
    pub(crate) entity: Entity,
    pub(crate) id: &'a NodeId,
}

/// Everything the edit context CONTAINS, in id order.
///
/// At the scenario node that is its ships; inside a ship it is that ship's
/// sections. This lives here rather than in the rail that draws it because it
/// is a question about the document, and the probe has to answer it too - a
/// driven run that read the answer off the UI tree would be testing the rail
/// rather than the tree.
pub(crate) fn context_nodes<'a>(
    context: &EditContext,
    q_ships: &'a ShipNodes,
    q_objects: &'a ObjectNodes,
    nodes: &'a SectionNodes,
) -> Vec<ContextNode<'a>> {
    let Some(scenario) = context.scenario() else {
        return Vec::new();
    };
    let Some(ship) = context.ship() else {
        // Ships first, then the rest of the world - the order the tree draws
        // them in, because the ships are what a builder came here for and the
        // world is what stands around them.
        let mut ships: Vec<_> = q_ships
            .iter()
            .filter(|(_, owner, ..)| owner.parent() == scenario)
            .map(|(entity, _, id, _)| ContextNode { entity, id })
            .collect();
        ships.sort_unstable_by(|a, b| id_order(&a.id.0).cmp(&id_order(&b.id.0)));
        ships.extend(
            objects_of(scenario, q_objects)
                .into_iter()
                .map(|(entity, id, ..)| ContextNode { entity, id }),
        );
        return ships;
    };
    sections_of(ship, nodes)
        .into_iter()
        .map(|(entity, id, ..)| ContextNode { entity, id })
        .collect()
}

/// The id of the node the editor is inside, or `None` at the scenario node.
pub(crate) fn inside_id<'a>(context: &EditContext, q_ships: &'a ShipNodes) -> Option<&'a NodeId> {
    let ship = context.ship()?;
    q_ships.get(ship).ok().map(|(_, _, id, _)| id)
}

/// FOCUS the entered ship: inside one, every other ship AND the whole world
/// around it leaves the stage; at the scenario node everything is back.
///
/// The world goes with the sibling ships because focus means one thing: what
/// you are working on is what is on screen. Leaving the rocks up would also
/// take the founding click away - "nothing under the pointer" is how an empty
/// ship gets its first part, and a range full of scenery is never nothing.
///
/// Two writes because hiding is two facts. `Visibility` takes the meshes off
/// screen, and the views' `Pickable` follows it because the picking ray does
/// not care what renders - an invisible collider would still eat clicks.
pub(crate) fn sync_ship_focus(
    mut commands: Commands,
    context: Res<EditContext>,
    mut staged: Query<(Entity, &mut Visibility), Or<(With<ShipNode>, With<ObjectNode>)>>,
    q_sections: Query<&ChildOf, With<SectionNode>>,
    views: Query<(Entity, &ChildOf, Has<Pickable>), With<NodeView>>,
) {
    let entered = context.ship();
    let hidden = |node: Entity| entered.is_some_and(|edited| edited != node);
    for (node, mut visibility) in &mut staged {
        let wanted = if hidden(node) {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    // Views carry no `Pickable` of their own, so its presence IS the "this
    // node is off the stage" mark and removal restores the default. A section's
    // view answers for its SHIP; an object's view answers for itself.
    for (view, owner, ignored) in &views {
        let owner = owner.parent();
        let staged_node = q_sections
            .get(owner)
            .map_or(owner, |section_owner| section_owner.parent());
        match (hidden(staged_node), ignored) {
            (true, false) => {
                commands.entity(view).insert(Pickable::IGNORE);
            }
            (false, true) => {
                commands.entity(view).remove::<Pickable>();
            }
            _ => {}
        }
    }
}

/// Snap the stage camera to the edit context: entering a ship frames that
/// ship, and the scenario node frames everything the document holds.
///
/// The free-fly rig rewrites the camera `Transform` every frame from private
/// state, so a bare pose write is gone by the next frame. The controller is
/// therefore removed and re-inserted around the write - the same move the
/// gallery's camera parking makes - and its setup re-reads the transform this
/// system just set. Keyed on the PATH (compared, not change-ticked: `exit()`
/// at the floor marks the resource changed without moving it) plus a fresh
/// camera, because a second editor visit spawns a new camera at the stock pose
/// while the context still points wherever it pointed.
pub(crate) fn sync_camera_focus(
    mut commands: Commands,
    context: Res<EditContext>,
    fresh: Query<(), Added<EditorCamera>>,
    ships: Query<&Transform, (With<ShipNode>, Without<EditorCamera>)>,
    camera: Option<Single<(Entity, &mut Transform), (With<EditorCamera>, Without<ShipNode>)>>,
    mut shown: Local<Option<Vec<Entity>>>,
) {
    if fresh.is_empty() && shown.as_ref() == Some(&context.path) {
        return;
    }
    // The move is recorded only once it lands: an entered ship or the camera
    // can be a frame away (both are spawned through commands), and a change
    // swallowed while they settle would leave the camera wherever it was.
    let Some(camera) = camera else {
        return;
    };
    let (entity, mut transform) = camera.into_inner();
    let pose = match context.ship() {
        Some(ship) => {
            let Ok(target) = ships.get(ship) else {
                return;
            };
            frame_stage(target.translation, 0.0)
        }
        None => {
            let positions: Vec<Vec3> = ships.iter().map(|ship| ship.translation).collect();
            let centre = positions.iter().sum::<Vec3>() / positions.len().max(1) as f32;
            let spread = positions
                .iter()
                .map(|position| position.distance(centre))
                .fold(0.0, f32::max);
            frame_stage(centre, spread)
        }
    };
    *shown = Some(context.path.clone());
    *transform = pose;
    commands
        .entity(entity)
        .remove::<WASDCameraController>()
        .insert(WASDCameraController);
}

/// How much further back the frame stands than the spread alone asks for.
///
/// The docked panels take about half the window at the 1024-wide size the
/// editor is built for, so content framed to fill the WINDOW puts its outer
/// ships under the rail - visible, and unclickable, because the panel eats the
/// pick. Framing to the band between the panels is what this buys: at a spread
/// of 12 (two ships 24 apart) the outer hull lands a comfortable 45px inside
/// the rail edge rather than 7px outside it.
const CHROME_ROOM: f32 = 1.6;

/// The stock editor view over `target`: the spawn pose slid onto it, backed
/// off by `spread` (and the room the chrome takes) so a stage of several ships
/// fits BETWEEN the panels. Zero spread IS the spawn pose, which is what keeps
/// the driven walks' fixed screen points meaning what they measured.
pub(crate) fn frame_stage(target: Vec3, spread: f32) -> Transform {
    let spread = spread * CHROME_ROOM;
    Transform::from_translation(target + Vec3::new(0.0, 5.0 + 0.75 * spread, 10.0 + 1.5 * spread))
        .looking_at(target, Vec3::Y)
}

/// Delete the document on the way out of the session (Back to Main Menu).
///
/// The nodes survive every INNER state change on purpose - Play leaves and
/// returns to the same ships - but a session ends at the main menu, and a
/// document that outlived it read as the editor never resetting (owner,
/// 2026-08-25). Despawning the scenario node takes the whole tree with it;
/// the context and selection are cleared so nothing points into the rubble.
pub(crate) fn teardown_document(
    mut commands: Commands,
    mut context: ResMut<EditContext>,
    mut selected: ResMut<SelectedNode>,
    roots: Query<Entity, With<ScenarioNode>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !context.path.is_empty() {
        context.path.clear();
    }
    if selected.0.is_some() {
        selected.0 = None;
    }
}

/// The node a picking hit belongs to: hits land on views, and the document is
/// the parent.
pub(crate) fn node_of_view(
    view: Entity,
    views: &Query<&ChildOf, With<NodeView>>,
) -> Option<Entity> {
    views.get(view).ok().map(ChildOf::parent)
}

/// Mint the next id under `parent`, named after `prototype`.
fn mint_id(ordinals: &mut Query<&mut NextChildOrdinal>, parent: Entity, prototype: &str) -> NodeId {
    let ordinal = match ordinals.get_mut(parent) {
        Ok(mut next) => {
            next.0 += 1;
            next.0
        }
        // A parent with no counter cannot mint a unique id, and a duplicate id
        // is a silently broken save. Loud and non-fatal.
        Err(_) => {
            error!("editor: node {parent} has no id counter - falling back to ordinal 0");
            0
        }
    };
    NodeId(format!("{prototype}_{ordinal}"))
}

/// Create the document if there is none: one empty scenario node, entered.
///
/// Lazily rather than at plugin build so a headless rig that never opens the
/// editor never grows a tree. The document is NOT `DespawnOnExit` of any inner
/// state: Play and F1 round-trip it. It dies with the SESSION instead - Back
/// to Main Menu runs [`teardown_document`] (owner decision, 2026-08-25), so a
/// later Sandbox entry starts on a fresh scenario.
pub(crate) fn ensure_document(
    mut commands: Commands,
    sections: Option<Res<GameSections>>,
    mut context: ResMut<EditContext>,
) {
    if context.scenario().is_some() {
        return;
    }
    found_document(&mut commands, sections.as_deref(), &mut context);
}

/// Found a document: one scenario node, seeded with the stock range, and the
/// context standing on it.
///
/// A new document opens on the stock range rather than on the void: the
/// sandbox's rocks, hulks, pickets, beacons and lights are the DEFAULT WORLD
/// now, not constants baked into the hand-off. Seeded HERE, once, when the
/// document is created - a "the world looks empty, refill it" pass would
/// resurrect everything the builder deleted on the next editor entry.
pub(crate) fn found_document(
    commands: &mut Commands,
    sections: Option<&GameSections>,
    context: &mut EditContext,
) -> Entity {
    let scenario = found_empty_document(commands, context);
    // Through the SAME lift a saved file goes through: the stock range's hulks
    // and pickets are hulls with sections, and a hull the document held as an
    // opaque object was one a double click could not go inside.
    let seed = lift_objects(default_world_objects(), &BTreeMap::new());
    for object in seed.objects {
        insert_object_node(commands, scenario, object);
    }
    for ship in seed.ships {
        insert_lifted_ship(commands, sections, scenario, ship);
    }
    scenario
}

/// Found a document with NOTHING in it, entered.
///
/// The seed is what [`found_document`] adds on top. A LOAD founds an empty one
/// and fills it from the file: seeding the stock range first would leave every
/// rock the builder deleted standing beside the ones they saved.
pub(crate) fn found_empty_document(commands: &mut Commands, context: &mut EditContext) -> Entity {
    let scenario = commands
        .spawn((
            EditorNode,
            ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
            Name::new("Scenario Node"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    context.path = vec![scenario];
    scenario
}

/// Throw the document away and found a new one - File > New Scenario.
///
/// Torn down and re-founded in ONE go rather than by clearing the context and
/// letting `ensure_document` notice: that system runs on entering the editor,
/// and a "no document, make one" pass in `Update` would also undo the teardown
/// that ends the session (the state change lands a frame later than the
/// despawn).
pub(crate) fn reset_document(
    _activate: On<Activate>,
    mut commands: Commands,
    sections: Option<Res<GameSections>>,
    mut context: ResMut<EditContext>,
    mut selected: ResMut<SelectedNode>,
    roots: Query<Entity, With<ScenarioNode>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    selected.0 = None;
    found_document(&mut commands, sections.as_deref(), &mut context);
}

/// Put one authored scenario object into the document under `scenario`, keeping
/// the id, name and pose the config carries.
///
/// The id is taken as WRITTEN rather than minted: the default world's objects
/// are named by the sandbox's own events (`picket_warden` is what the wake
/// handler flips, `beacon_veil` is what swaps the sky), so a re-minted id would
/// leave the script pointing at nothing.
pub(crate) fn insert_object_node(
    commands: &mut Commands,
    scenario: Entity,
    config: ScenarioObjectConfig,
) -> Entity {
    let id = NodeId(config.base.id);
    let transform =
        Transform::from_translation(config.base.position).with_rotation(config.base.rotation);
    insert_object(
        commands,
        scenario,
        id,
        ObjectNode {
            name: config.base.name,
            kind: config.kind,
        },
        transform,
    )
}

/// Add a fresh object of `choice` to the world at `transform`, under a minted
/// id.
pub(crate) fn spawn_object_node(
    commands: &mut Commands,
    ordinals: &mut Query<&mut NextChildOrdinal>,
    scenario: Entity,
    choice: ObjectChoice,
    transform: Transform,
) -> Entity {
    let id = mint_id(ordinals, scenario, choice.stem());
    // Named for the id it was minted under, so two rocks are two names in the
    // tree rather than two rows both reading "Asteroid".
    let object = ObjectNode {
        name: minted_name(&id.0),
        ..choice.stock()
    };
    insert_object(commands, scenario, id, object, transform)
}

/// The spawn itself, for callers that already know the id.
///
/// No view: an object's body is a mesh the editor BUILDS, which needs asset
/// stores a `Commands`-only path cannot reach, so [`sync_object_views`] gives it
/// one. That also covers the two other ways an object node appears - the
/// document seed, and a second visit to the editor after its views were
/// despawned.
fn insert_object(
    commands: &mut Commands,
    scenario: Entity,
    id: NodeId,
    object: ObjectNode,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            EditorNode,
            Name::new(format!("Object Node {}", id.0)),
            id,
            object,
            transform,
            // INHERITED for the same reason a section is: an explicit `Visible`
            // overrides the hidden ancestor `sync_ship_focus` is relying on.
            Visibility::Inherited,
            ChildOf(scenario),
        ))
        .id()
}

/// Give every object node a view, and only one.
///
/// A reconciler rather than an eager spawn beside the node, unlike a section:
/// three different things create object nodes (the document seed, the palette,
/// and a second editor visit that found the nodes without bodies), and all
/// three would otherwise have to carry the asset stores the mesh is built from.
pub(crate) fn sync_object_views(
    mut commands: Commands,
    mut art: PreviewArt,
    sections: Option<Res<GameSections>>,
    ships: Option<Res<GameShips>>,
    nodes: Query<(Entity, &ObjectNode, Option<&Children>)>,
    views: Query<(), With<NodeView>>,
) {
    for (node, object, children) in &nodes {
        let bodied =
            children.is_some_and(|children| children.iter().any(|child| views.contains(child)));
        if bodied {
            continue;
        }
        commands.entity(node).with_children(|parent| {
            let mut view = parent.spawn((
                DespawnOnExit(ExampleStates::Editor),
                NodeView,
                Name::new("Object View"),
                Transform::default(),
                Visibility::Inherited,
            ));
            insert_preview_object(
                &mut view,
                object,
                &mut art,
                sections.as_deref(),
                ships.as_deref(),
            );
        });
    }
}

/// Take the body off an object whose config just changed, so the pair above
/// builds it again from what the config now says.
///
/// A node's view is built ONCE - by [`sync_object_views`] when the node has
/// none, or by [`rebuild_node_views`] on the way into the editor - because
/// until the inspector existed nothing could change a config that already had
/// a body. Dropping the body is the whole mechanism: a rock typed from radius
/// 3 to radius 12 has to be a bigger rock on the stage, not a bigger rock the
/// next time you visit.
///
/// SECTIONS are deliberately not here. What an editable section field changes
/// (thrust, health) is not what its mesh is built from, and rebuilding a
/// section view would drop the skin derivation and the placement solve
/// that read it in the same frame.
pub(crate) fn drop_edited_views(
    mut commands: Commands,
    edited: Query<&Children, Changed<ObjectNode>>,
    views: Query<(), With<NodeView>>,
) {
    for children in &edited {
        for view in children.iter().filter(|child| views.contains(*child)) {
            commands.entity(view).despawn();
        }
    }
}

/// The stem every id the editor MINTS for a ship carries.
///
/// What tells a ship the builder added from one the range came with: a seeded
/// hull keeps the scenario's own id (`picket_warden`, `hulk_0`), because that
/// is what the wake handler flips.
pub(crate) const MINTED_SHIP_STEM: &str = "ship";

/// Add a BLANK ship to the document and go inside it.
///
/// Additive: a second "Add Ship" is one more subtree standing beside the first
/// rather than a reset. Ships are spaced along +X so two of them are two things
/// on the stage rather than one pile. Blank on purpose - which part a ship
/// starts from is the builder's first decision, and the empty ship's founding
/// click (see `crate::placement::found_empty_ship`) is where they make it.
pub(crate) fn spawn_ship_node(
    commands: &mut Commands,
    ordinals: &mut Query<&mut NextChildOrdinal>,
    context: &mut EditContext,
    ships: usize,
    driver: ShipDriver,
) -> Option<Entity> {
    let scenario = context.scenario()?;
    let id = mint_id(ordinals, scenario, MINTED_SHIP_STEM);
    let ship = commands
        .spawn((
            EditorNode,
            ShipNode {
                name: minted_name(&id.0),
                driver,
                allegiance: default_allegiance(driver),
                ..default()
            },
            Name::new(format!("Ship Node {}", id.0)),
            id,
            NextChildOrdinal::default(),
            Transform::from_xyz(ships as f32 * SHIP_NODE_SPACING, 0.0, 0.0),
            Visibility::Visible,
            ChildOf(scenario),
        ))
        .id();
    context.enter(ship);
    Some(ship)
}

/// A minted id, said the way a person would: `ship_1` reads "Ship 1".
///
/// A name a builder can then change. The id it came from does not change with
/// it - the id is what the document, the walks and the save file key on, and a
/// rename that moved it would break every one of them.
fn minted_name(id: &str) -> String {
    id.split('_')
        .map(|word| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) => first.to_uppercase().chain(letters).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Put a ship the document already named back under `scenario` - the load's
/// counterpart to [`spawn_ship_node`], which mints.
///
/// The context is NOT entered: a load lands you at the scenario node looking at
/// what you opened, not inside whichever ship the file listed last.
pub(crate) fn insert_ship_node(
    commands: &mut Commands,
    scenario: Entity,
    id: NodeId,
    ship: ShipNode,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            EditorNode,
            Name::new(format!("Ship Node {}", id.0)),
            ship,
            id,
            NextChildOrdinal::default(),
            transform,
            Visibility::Visible,
            ChildOf(scenario),
        ))
        .id()
}

/// Put a section back on `ship` with the source the file wrote, and give it a
/// view if its config can be resolved.
///
/// Unlike [`spawn_section_node`] this does not inline what it is handed: a
/// saved section may name a catalog prototype, and re-inlining it would fork
/// the design away from the catalog it was built against. A prototype the
/// catalog has lost keeps its place in the document with nothing to show, which
/// is what [`rebuild_node_views`] warns about.
pub(crate) fn insert_lifted_section(
    commands: &mut Commands,
    sections: Option<&GameSections>,
    ship: Entity,
    id: NodeId,
    section: SectionNode,
    transform: Transform,
) -> Entity {
    let config = section.resolve(sections).cloned();
    let node = commands
        .spawn((
            EditorNode,
            Name::new(format!("Section Node {}", id.0)),
            section,
            id,
            transform,
            Visibility::Inherited,
            ChildOf(ship),
        ))
        .id();
    if let Some(config) = config {
        spawn_node_view(commands, node, &config);
    }
    node
}

/// Set a node's id counter to `ordinal`, so the next mint under it is fresh.
///
/// The load's last step on every node it filled: the counter is what stops a
/// newly placed part from taking an id the file already used.
pub(crate) fn resume_ordinals(
    ordinals: &mut Query<&mut NextChildOrdinal>,
    node: Entity,
    ordinal: u32,
) {
    if let Ok(mut next) = ordinals.get_mut(node) {
        next.0 = ordinal;
    }
}

/// Add a section to `ship` at `transform`, and give it a view to be seen by.
pub(crate) fn spawn_section_node(
    commands: &mut Commands,
    ordinals: &mut Query<&mut NextChildOrdinal>,
    ship: Entity,
    config: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) -> Entity {
    let id = mint_id(ordinals, ship, &config.base.id);
    insert_section_node(commands, ship, id, config, transform, binds)
}

/// The spawn itself, for callers that already know the id.
fn insert_section_node(
    commands: &mut Commands,
    ship: Entity,
    id: NodeId,
    config: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) -> Entity {
    let node = commands
        .spawn((
            EditorNode,
            SectionNode {
                source: SectionSource::Inline(config.clone()),
                modifications: vec![],
                binds,
            },
            Name::new(format!("Section Node {}", id.0)),
            id,
            transform,
            // INHERITED, never `Visible`: an explicit `Visible` overrides a
            // hidden ancestor, so every section of a ship `sync_ship_focus`
            // hid would stay on screen while its picking was off.
            Visibility::Inherited,
            ChildOf(ship),
        ))
        .id();
    spawn_node_view(commands, node, config);
    node
}

/// Give `node` the mesh, collider and picking that make it visible and clickable.
pub(crate) fn spawn_node_view(commands: &mut Commands, node: Entity, config: &SectionConfig) {
    commands.entity(node).with_children(|parent| {
        let mut view = parent.spawn((
            DespawnOnExit(ExampleStates::Editor),
            NodeView,
            Transform::default(),
            Visibility::Inherited,
        ));
        insert_preview_section(&mut view, config, PreviewRole::Section);
    });
}

/// Rebuild every node's view on entering the editor.
///
/// The views are `DespawnOnExit(Editor)` and the nodes are not, so a second
/// visit finds a document with no bodies and gives it new ones. This replaces
/// the old re-keying rebuild, which had to re-derive both maps onto whatever
/// entities it happened to spawn - and which DROPPED any section it could not
/// inline.
pub(crate) fn rebuild_node_views(
    mut commands: Commands,
    sections: Option<Res<GameSections>>,
    nodes: Query<(Entity, &NodeId, &SectionNode, Option<&Children>)>,
    views: Query<(), With<NodeView>>,
) {
    for (node, id, section, children) in &nodes {
        // A section founded THIS frame already has its body: `ensure_document`
        // runs chained before this, and a second view would be a second mesh
        // on the same node.
        let bodied =
            children.is_some_and(|children| children.iter().any(|child| views.contains(child)));
        if bodied {
            continue;
        }
        let Some(config) = section.resolve(sections.as_deref()) else {
            warn!(
                "editor: section '{}' names prototype '{}', which is not in the catalog - \
                 it stays in the document but has nothing to show",
                id.0,
                section.prototype()
            );
            continue;
        };
        spawn_node_view(&mut commands, node, config);
    }
}

/// Say when two children of one node ended up wearing the same id.
///
/// An id is the document's own key: it is what a save writes, what a load reads
/// back, and what the tree shows. Two rows with identical text are two rows
/// nothing can tell apart - and the only sign used to be one `error!` line at
/// the moment a counter was missing, which said nothing about the row that came
/// out of it. Said once per clash, and cleared when the clash goes.
pub(crate) fn report_duplicate_ids(
    parents: Query<&Children>,
    ids: Query<&NodeId>,
    mut said: Local<Option<String>>,
    mut says: EditorSays,
) {
    let clash = parents.iter().find_map(|children| {
        let mut seen: Vec<&str> = Vec::new();
        children.iter().find_map(|child| {
            let id = ids.get(child).ok()?;
            if seen.contains(&id.0.as_str()) {
                return Some(id.0.clone());
            }
            seen.push(&id.0);
            None
        })
    });
    if clash == *said {
        return;
    }
    if let Some(id) = &clash {
        says.refuse(format!("two nodes are both called '{id}' - rename one"));
    }
    *said = clash;
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn hull(id: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A ship node with one section node on it, at the given source.
    fn document(world: &mut World, source: SectionSource) -> (Entity, Entity) {
        let ship = world
            .spawn((
                EditorNode,
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                NextChildOrdinal(1),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        let section = world
            .spawn((
                EditorNode,
                SectionNode {
                    source,
                    modifications: vec![],
                    binds: vec![],
                },
                NodeId("hull_1".to_string()),
                Transform::from_xyz(1.0, 2.0, 3.0),
                Visibility::Inherited,
                ChildOf(ship),
            ))
            .id();
        (ship, section)
    }

    fn views(world: &mut World) -> usize {
        world
            .query_filtered::<(), With<NodeView>>()
            .iter(world)
            .count()
    }

    /// The point of the whole split: leaving the editor takes the BODIES and
    /// leaves the document, so a second visit rebuilds the same ship under the
    /// same ids. The old path re-keyed two maps onto whatever entities it
    /// happened to spawn, which is why a saved file was impossible.
    #[test]
    fn a_document_outlives_its_views_and_keeps_its_ids() {
        let mut world = World::new();
        let (_, section) = document(&mut world, SectionSource::Inline(hull("hull")));

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");
        assert_eq!(views(&mut world), 1, "the section got a body");

        // What DespawnOnExit(Editor) does on the way out to the scenario.
        let view: Vec<Entity> = world
            .query_filtered::<Entity, With<NodeView>>()
            .iter(&world)
            .collect();
        for entity in view {
            world.despawn(entity);
        }
        assert_eq!(views(&mut world), 0);

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");

        assert_eq!(views(&mut world), 1, "and a new one on the way back in");
        assert_eq!(
            world.get::<NodeId>(section),
            Some(&NodeId("hull_1".to_string())),
            "the id is the same id, not a re-derived one"
        );
        assert_eq!(
            world.get::<Transform>(section).map(|t| t.translation),
            Some(Vec3::new(1.0, 2.0, 3.0)),
            "and so is the pose"
        );
    }

    /// A section that names a catalog prototype is REBUILT rather than dropped.
    /// The old rebuild inlined-or-discarded, which is the direct blocker on ever
    /// loading a file that references the catalog.
    #[test]
    fn a_prototype_sourced_section_is_rebuilt_from_the_catalog() {
        let mut world = World::new();
        world.insert_resource(GameSections(vec![hull("hull")]));
        document(&mut world, SectionSource::Prototype("hull".to_string()));

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");

        assert_eq!(views(&mut world), 1, "the prototype resolved to a body");
    }

    /// A prototype a mod overlay dropped leaves the section in the document with
    /// nothing to show, rather than deleting the player's work.
    #[test]
    fn a_missing_prototype_keeps_the_section_and_shows_nothing() {
        let mut world = World::new();
        world.insert_resource(GameSections(vec![]));
        let (_, section) = document(&mut world, SectionSource::Prototype("gone".to_string()));

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");

        assert_eq!(views(&mut world), 0);
        assert!(
            world.get::<SectionNode>(section).is_some(),
            "the section is still in the document"
        );
    }

    /// Entering a ship takes every other ship off the stage - the meshes AND
    /// the picking, because an invisible collider would still eat clicks.
    #[test]
    fn entering_a_ship_hides_and_unpicks_its_siblings() {
        let mut world = World::new();
        let (first, _) = document(&mut world, SectionSource::Inline(hull("hull")));
        let (second, second_section) = document(&mut world, SectionSource::Inline(hull("hull")));
        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, first],
        });

        let view_of = |world: &mut World, section: Entity| {
            world
                .query_filtered::<(Entity, &ChildOf), With<NodeView>>()
                .iter(world)
                .find(|(_, owner)| owner.parent() == section)
                .map(|(view, _)| view)
                .expect("the section grew a view")
        };

        world
            .run_system_once(sync_ship_focus)
            .expect("the focus sync runs");
        assert_eq!(world.get::<Visibility>(first), Some(&Visibility::Visible));
        assert_eq!(
            world.get::<Visibility>(second),
            Some(&Visibility::Hidden),
            "the sibling leaves the stage"
        );
        let hidden_view = view_of(&mut world, second_section);
        assert_eq!(
            world.get::<Pickable>(hidden_view),
            Some(&Pickable::IGNORE),
            "and its colliders leave the pointer's way"
        );

        world.resource_mut::<EditContext>().exit();
        world
            .run_system_once(sync_ship_focus)
            .expect("the focus sync runs");
        assert_eq!(world.get::<Visibility>(second), Some(&Visibility::Visible));
        assert_eq!(
            world.get::<Pickable>(hidden_view),
            None,
            "back at the scenario node the default picking is restored"
        );
    }

    /// A placed section INHERITS its ship's visibility. `Visibility::Visible`
    /// overrides a hidden ancestor in bevy, so sections spawned with it stayed
    /// on screen after `sync_ship_focus` hid their ship - the focus tests all
    /// passed by reading the ship's own component while the live stage showed
    /// every ship at once.
    #[test]
    fn a_placed_section_inherits_its_ships_visibility() {
        let mut world = World::new();
        let ship = world
            .spawn((ShipNode::default(), NextChildOrdinal::default()))
            .id();
        let section = world
            .run_system_once(move |mut commands: Commands| {
                insert_section_node(
                    &mut commands,
                    ship,
                    NodeId("hull_1".to_string()),
                    &hull("hull"),
                    Transform::default(),
                    vec![],
                )
            })
            .expect("the section spawner runs");

        assert_eq!(
            world.get::<Visibility>(section),
            Some(&Visibility::Inherited),
            "an explicit Visible would override the hidden ship above it"
        );
    }

    /// Back to Main Menu ends the session, and the session owns the document:
    /// the whole tree goes, and nothing keeps pointing into it.
    #[test]
    fn leaving_the_session_deletes_the_document() {
        let mut world = World::new();
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        world
            .run_system_once(ensure_document)
            .expect("the document is created");
        let scenario = world
            .resource::<EditContext>()
            .scenario()
            .expect("the document exists");
        let ship = world
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                NextChildOrdinal::default(),
                ChildOf(scenario),
            ))
            .id();
        world.resource_mut::<EditContext>().enter(ship);
        world.resource_mut::<SelectedNode>().0 = Some(ship);

        world
            .run_system_once(teardown_document)
            .expect("the teardown runs");

        assert_eq!(
            world.query::<&EditorNode>().iter(&world).count(),
            0,
            "the scenario node takes its whole subtree with it"
        );
        assert!(world.resource::<EditContext>().path.is_empty());
        assert_eq!(world.resource::<SelectedNode>().0, None);

        // And the next entry starts a fresh document rather than resuming.
        world
            .run_system_once(ensure_document)
            .expect("the document check runs");
        assert_ne!(
            world.resource::<EditContext>().scenario(),
            Some(scenario),
            "a later Sandbox entry founds a new scenario"
        );
    }

    /// The sandbox range is a DOCUMENT now, not a table the hand-off reads:
    /// founding one stands the whole world up as nodes.
    ///
    /// Under the AUTHORED ids, not minted ones. The scenario's own events name
    /// `picket_warden` and `beacon_veil`; re-keying the world on placement
    /// order would leave every one of them pointing at nothing.
    #[test]
    fn a_new_document_stands_the_default_world_up_under_its_authored_ids() {
        let mut world = World::new();
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();

        world
            .run_system_once(ensure_document)
            .expect("the document is created");

        let mut standing: Vec<String> = world
            .query_filtered::<&NodeId, Or<(With<ObjectNode>, With<ShipNode>)>>()
            .iter(&world)
            .map(|id| id.0.clone())
            .collect();
        standing.sort();
        let mut authored: Vec<String> = default_world_objects()
            .iter()
            .map(|object| object.base.id.clone())
            .collect();
        authored.sort();
        assert_eq!(standing, authored, "the whole range came up as nodes");

        // The HULLS came up as ships, sections and all: a hulk the document
        // held as an opaque object was one a double click could not open.
        let hulks: Vec<&NodeId> = world
            .query_filtered::<&NodeId, With<ShipNode>>()
            .iter(&world)
            .filter(|id| id.0.starts_with("hulk_"))
            .collect();
        assert_eq!(hulks.len(), 5, "every target hulk is a ship node");
        assert!(
            world
                .query_filtered::<&NodeId, With<ShipNode>>()
                .iter(&world)
                .any(|id| id.0 == "picket_warden"),
            "and so is every picket, under the id the wake handler flips"
        );
        assert!(
            world.query::<&SectionNode>().iter(&world).count() > 5,
            "a seeded hull's sections are nodes of their own"
        );

        // And once only: a second entry into the editor finds the world it
        // left rather than standing a second copy of it up beside the first.
        world
            .run_system_once(ensure_document)
            .expect("the document check runs");
        assert_eq!(
            world
                .query_filtered::<(), Or<(With<ObjectNode>, With<ShipNode>)>>()
                .iter(&world)
                .count(),
            authored.len()
        );
    }

    /// Entering a ship takes the WORLD off the stage, not just the sibling
    /// ships. A ship is edited in clear space: the builder's founding click
    /// needs empty space under the pointer, and a rock parked between the
    /// camera and the build plane would eat it.
    #[test]
    fn entering_a_ship_takes_the_world_off_the_stage() {
        let mut world = World::new();
        let (ship, _) = document(&mut world, SectionSource::Inline(hull("hull")));
        let rock = world
            .spawn((
                EditorNode,
                ObjectChoice::Asteroid.stock(),
                NodeId("asteroid_1".to_string()),
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });

        world
            .run_system_once(sync_ship_focus)
            .expect("the focus sync runs");
        assert_eq!(
            world.get::<Visibility>(rock),
            Some(&Visibility::Hidden),
            "the world leaves the stage with the sibling ships"
        );

        world.resource_mut::<EditContext>().exit();
        world
            .run_system_once(sync_ship_focus)
            .expect("the focus sync runs");
        assert_eq!(
            world.get::<Visibility>(rock),
            Some(&Visibility::Visible),
            "and comes back when the editor does"
        );
    }

    /// An object node is model-only; the body under it is a view like any
    /// other, spawned once and reconciled rather than respawned - a view per
    /// frame would flicker the whole range.
    #[test]
    fn an_object_node_grows_one_view_and_keeps_it() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        let node = app
            .world_mut()
            .spawn((
                EditorNode,
                // The anchor: the one stock kind that reaches for no texture,
                // so the test needs no asset directory behind it.
                ObjectChoice::Anchor.stock(),
                NodeId("anchor_1".to_string()),
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();

        let views = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&ChildOf, With<NodeView>>()
                .iter(app.world())
                .filter(|owner| owner.parent() == node)
                .count()
        };

        app.world_mut()
            .run_system_once(sync_object_views)
            .expect("the object view sync runs");
        assert_eq!(views(&mut app), 1, "the node got a body");

        app.world_mut()
            .run_system_once(sync_object_views)
            .expect("the object view sync runs");
        assert_eq!(views(&mut app), 1, "and not a second one on the next pass");
    }

    /// Enter/exit is a path, so exiting has somewhere to return to - and the
    /// scenario node is the floor, because there is nothing outside the
    /// document to back out into.
    #[test]
    fn the_context_enters_exits_and_never_pops_the_scenario_node() {
        let scenario = Entity::from_raw_u32(1).expect("a test entity id");
        let first = Entity::from_raw_u32(2).expect("a test entity id");
        let second = Entity::from_raw_u32(3).expect("a test entity id");

        let mut context = EditContext {
            path: vec![scenario],
        };
        assert_eq!(context.scenario(), Some(scenario));
        assert_eq!(context.ship(), None, "the document opens outside any ship");

        context.enter(first);
        assert_eq!(context.ship(), Some(first));
        assert_eq!(context.current(), Some(first));

        // Entering a SIBLING is an enter, not a nest: a ship is never inside
        // another ship.
        context.enter(second);
        assert_eq!(context.path, vec![scenario, second]);
        assert_eq!(context.ship(), Some(second));

        context.exit();
        assert_eq!(context.current(), Some(scenario));
        assert_eq!(context.ship(), None);

        context.exit();
        assert_eq!(
            context.path,
            vec![scenario],
            "the scenario node is the floor"
        );
    }

    /// File > New throws the whole document away and stands the context on a
    /// fresh one. Everything the builder added has to go with it - a "New" that
    /// left the old ships parented to a dead root would keep them in the world
    /// with no row in the tree pointing at them.
    #[test]
    fn file_new_replaces_the_whole_document() {
        let mut world = World::new();
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        world.add_observer(reset_document);

        world
            .run_system_once(ensure_document)
            .expect("the founding system runs");
        let first = world
            .resource::<EditContext>()
            .scenario()
            .expect("a document was founded");
        let ship = world
            .spawn((EditorNode, ShipNode::default(), ChildOf(first)))
            .id();
        world.resource_mut::<EditContext>().enter(ship);
        world.resource_mut::<SelectedNode>().0 = Some(ship);

        world.trigger(Activate {
            entity: Entity::PLACEHOLDER,
        });
        world.flush();

        let second = world
            .resource::<EditContext>()
            .scenario()
            .expect("a new document was founded");
        assert_ne!(second, first, "the old root is gone, not reused");
        assert!(
            world.get_entity(first).is_err(),
            "the old root was despawned"
        );
        assert!(
            world.get_entity(ship).is_err(),
            "and took its ships with it"
        );
        assert_eq!(
            world.resource::<EditContext>().ship(),
            None,
            "the context stands on the new scenario node, not inside a dead ship"
        );
        assert_eq!(world.resource::<SelectedNode>().0, None);
    }

    /// Two rows with the same text are two rows nothing can tell apart, so the
    /// editor names the id rather than leaving the builder to find the pair.
    #[test]
    fn two_children_wearing_one_id_are_reported_by_name() {
        let mut world = World::new();
        world.init_resource::<crate::config::EditorStatus>();
        world.init_resource::<Time>();
        let scenario = world.spawn(ScenarioNode).id();
        world.spawn((NodeId("rock_1".to_string()), ChildOf(scenario)));
        world.spawn((NodeId("rock_2".to_string()), ChildOf(scenario)));

        world
            .run_system_once(report_duplicate_ids)
            .expect("the check runs");
        assert_eq!(
            world.resource::<crate::config::EditorStatus>().line(),
            None,
            "distinct ids are not worth saying anything about"
        );

        world.spawn((NodeId("rock_1".to_string()), ChildOf(scenario)));
        world
            .run_system_once(report_duplicate_ids)
            .expect("the check runs");
        let (line, _) = world
            .resource::<crate::config::EditorStatus>()
            .line()
            .expect("a clash is said");
        assert!(
            line.contains("rock_1"),
            "the line must NAME the id, not just report a clash; it read {line:?}"
        );
    }
}
