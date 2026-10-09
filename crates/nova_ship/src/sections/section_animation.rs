//! Authored section animation: named nodes in a section's render scene,
//! driven procedurally by gameplay cues.
//!
//! A section's config declares TRACKS ([`SectionAnimation`]): which scene
//! nodes move (by glTF node-name prefix), how ([`SectionAnimationMotion`],
//! composed onto each node's authored rest pose), and how fast. The cue
//! ([`SectionAnimationCue`]) is the contract between art and mechanics:
//! content owns WHAT moves, the section kind's own systems own WHEN by
//! steering the cue's target through [`SectionAnimations::set_cue`].
//! Procedural data rather than glTF clips: nothing in the workspace drives
//! `AnimationPlayer`, and the shipped section art comes from
//! `scripts/nova_glb.py`, which writes no animation samplers.
//!
//! This module owns the generic half only - rig resolution against spawned
//! scenes and the progress driver. It knows nothing about torpedoes or
//! turrets; kind modules (the bay's muzzle door, later the railgun charge
//! and the PDC stow) write cue targets and nothing else.

use bevy::{app::SceneSpawnerSystems, prelude::*, world_serialization::WorldInstanceReady};

/// The authored track types, the runtime [`SectionAnimations`] component, and
/// `SectionAnimationPlugin` with `SectionAnimationSystems`.
pub mod prelude {
    pub use super::{
        FrozenSectionAnimations, SectionAnimation, SectionAnimationCue, SectionAnimationMotion,
        SectionAnimationPlugin, SectionAnimationRigDirty, SectionAnimationSystems,
        SectionAnimations,
    };
}

/// Which gameplay moment drives an authored animation track. One cue can
/// drive several tracks (a stow that folds a cover AND drops the mount); a
/// cue no system steers rests at progress 0. Add a variant here when a new
/// mechanic wants art.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionAnimationCue {
    /// A weapon bay's muzzle cover: driven to 1 (open) across a launch and
    /// back to 0 (closed) at rest by the bay's own fire path.
    MuzzleDoor,
    /// A retractable mount's elevator: driven to 1 (sunk into the housing)
    /// while stowed, 0 (raised, the rest pose) while deployed. Steered by
    /// the turret's stow state machine or the mining emitter's sequence, each
    /// of which sequences it against [`Self::StowDoors`].
    StowLift,
    /// A retractable mount's housing lids: driven to 1 (shut over the sunk
    /// gun or emitter tip) while stowed, 0 (parted, the rest pose) while
    /// deployed. Both sequences shut them only after [`Self::StowLift`]
    /// reaches 1, and part them before raising it.
    StowDoors,
    /// A charging weapon's capacitor bolt: driven to 1 across the charge and
    /// snapped back to 0 the instant the shot leaves. The railgun's track
    /// walks the bolt from the breech to the muzzle brake, so the length of
    /// bore it has crossed IS how much charge is left to run - a tell the
    /// firing ship reads on its own hull and an enemy reads across the gap.
    ///
    /// Steered by the railgun's charge system, which writes the gameplay
    /// charge fraction straight onto the track rather than letting it travel
    /// at its own authored speed: the authored charge time is the ONE clock,
    /// and art that ran on a second one would promise a shot that had not
    /// arrived.
    Charge,
    /// A docking port's telescoping sleeve: driven to 1 (fully out) while a
    /// connection holds the port and back to 0 as it retracts. Steered by the
    /// port's own [`DockingSectionState`](super::docking_section::DockingSectionState),
    /// which is set by the connection rather than by the travel - the joint
    /// is what holds the ships, so the sleeve is free to be art and arrive
    /// whenever the authored travel says.
    DockTube,
    /// A cargo intake's accordion door: driven to 1 (folded open) while a
    /// canister is in the intake's detection volume or a jettison waits to
    /// leave, and back to 0 (shut) otherwise. Steered by the intake's own
    /// system, which takes a canister in or drops one only at progress 1:
    /// the open door is the tell that the hold is ready.
    IntakeDoor,
}

/// How each target node moves as its track's progress runs 0 -> 1, composed
/// onto the node's authored rest transform every frame.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionAnimationMotion {
    /// Rotate each node about its LOCAL X axis, reaching `degrees` at
    /// progress 1. Local X is the authoring convention for a hinge: the
    /// model gives each moving part its own node with the origin ON the
    /// hinge line and X along it, and the node's placement transform aims
    /// the hinge (`gen-section-parts.py` `nodes` recipes). One motion then
    /// serves every copy of the part - the bay's six iris petals share this
    /// track with six different hinge orientations.
    RotateX {
        /// Signed rotation at full progress, in degrees.
        degrees: f32,
    },
    /// Slide each node along a LOCAL displacement, reaching `offset` at
    /// progress 1, composed onto the rest translation in the node's rest
    /// frame. The same one-motion-many-nodes convention as [`Self::RotateX`]:
    /// the node's placement rotation aims the slide, so two housing lids
    /// authored mirror-rotated part in opposite directions off one track.
    Translate {
        /// Node-local displacement at full progress, in world units (10 m
        /// each). Mesh-frame geometry rather than an authored distance: it is
        /// measured off the glTF node it slides, so it stays in the units that
        /// model is built in.
        offset: Vec3,
    },
    /// Fold an accordion of slats aside into two pockets, reaching `degrees`
    /// at progress 1. Each node name must end, after the track's prefix, in
    /// `<l|r><index>`: the side whose pocket the slat folds toward (`l` to
    /// the parent's +X, `r` to -X) and its place counted from that pocket's
    /// wall, from 0. A slat turns about its local X, even indices one way and
    /// odd the other, and slides along the parent's X so that each fold
    /// meets the next at an edge, as the pleats of an accordion do.
    Fold {
        /// Signed slat rotation at full progress, in degrees.
        degrees: f32,
        /// Slat extent across the door, along the parent's X at rest, in
        /// world units (10 m each), measured off the glTF slat.
        slat_width: f32,
        /// Slat thickness, in world units (10 m each), measured off the
        /// glTF slat.
        slat_thickness: f32,
    },
}

impl SectionAnimationMotion {
    /// Write the pose at `progress` onto `transform`, relative to the node's
    /// rest pose.
    fn apply(self, node: &TrackNode, progress: f32, transform: &mut Transform) {
        let rest = &node.rest;
        match self {
            Self::RotateX { degrees } => {
                *transform = Transform {
                    rotation: rest.rotation
                        * Quat::from_rotation_x(degrees.to_radians() * progress),
                    ..*rest
                };
            }
            Self::Translate { offset } => {
                *transform = Transform {
                    translation: rest.translation + rest.rotation * (offset * progress),
                    ..*rest
                };
            }
            Self::Fold {
                degrees,
                slat_width,
                slat_thickness,
            } => {
                let slat = node
                    .slat
                    .expect("a Fold track node resolves with its slat place");
                let fold = degrees.to_radians() * progress;
                let turn = if slat.index.is_multiple_of(2) {
                    fold
                } else {
                    -fold
                };
                let reach = (slat.index as f32 + 0.5) * slat_width;
                let mut translation = rest.translation;
                translation.x +=
                    slat.toward * (reach * (fold.cos() - 1.0) + slat_thickness * 0.5 * fold.sin());
                *transform = Transform {
                    translation,
                    rotation: rest.rotation * Quat::from_rotation_x(turn),
                    ..*rest
                };
            }
        }
    }

    /// The slat place a [`Self::Fold`] node carries in its name suffix after
    /// `prefix`, or `None` for the other motions.
    ///
    /// # Panics
    ///
    /// On a Fold node whose suffix is not `<l|r><index>`: the art and the
    /// track disagree, and a slat with no place would fold through its
    /// neighbours.
    fn slat(self, section: &str, name: &str, prefix: &str) -> Option<FoldSlat> {
        let Self::Fold { .. } = self else {
            return None;
        };
        let slat = name
            .strip_prefix(prefix)
            .and_then(|suffix| suffix.split_at_checked(1))
            .and_then(|(side, index)| {
                let toward = match side {
                    "l" => 1.0,
                    "r" => -1.0,
                    _ => return None,
                };
                Some(FoldSlat {
                    toward,
                    index: index.parse().ok()?,
                })
            });
        Some(slat.unwrap_or_else(|| {
            panic!("section {section}: Fold node {name:?} is not a `{prefix}<l|r><index>` slat")
        }))
    }
}

/// Where one [`SectionAnimationMotion::Fold`] slat sits in its door.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
struct FoldSlat {
    /// +1 for a slat that folds toward the parent's +X, -1 toward -X.
    toward: f32,
    /// The slat's place counted from its pocket's wall, from 0.
    index: u32,
}

/// One resolved scene node of a track.
#[derive(Clone, Copy, Debug, Reflect)]
struct TrackNode {
    entity: Entity,
    /// The authored transform, captured when the scene instance readies.
    rest: Transform,
    /// The node's place in its door, for a Fold track only.
    slat: Option<FoldSlat>,
}

/// One authored animation track on a section's render scene: the moving
/// nodes, the motion they perform, and the travel times. Authored in the
/// section RON as `animations` on the base config; a section without the
/// field has none.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionAnimation {
    /// The gameplay moment that drives this track.
    pub cue: SectionAnimationCue,
    /// The moving scene nodes, by glTF node-name prefix: `"door_petal_"`
    /// matches `door_petal_0..5`. A prefix rather than a list, so art can
    /// change the part count without touching the authored track.
    pub node_prefix: String,
    /// What each matched node does as progress runs 0 -> 1.
    pub motion: SectionAnimationMotion,
    /// Seconds for progress to travel 0 -> 1. Zero or negative snaps.
    pub open_seconds: f32,
    /// Seconds for progress to travel 1 -> 0. Zero or negative snaps.
    pub close_seconds: f32,
}

/// One track's runtime state: the authored declaration plus its progress and
/// the resolved scene nodes with their rest poses.
#[derive(Clone, Debug, Reflect)]
struct TrackState {
    config: SectionAnimation,
    /// Where the track is, 0 (rest) to 1 (deployed).
    progress: f32,
    /// Where the track is going, steered through [`SectionAnimations::set_cue`].
    target: f32,
    /// Forces one transform write even at rest - set on retarget and on
    /// snap, so a snapped pose lands without travel.
    dirty: bool,
    /// The matched scene nodes.
    nodes: Vec<TrackNode>,
}

/// Runtime state of a section's authored animation tracks. Inserted by
/// `base_section` from the config's `animations`; empty for the sections
/// that author none. Kind systems steer it with [`Self::set_cue`]; the
/// [`SectionAnimationPlugin`] systems resolve scene nodes and move them.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct SectionAnimations {
    tracks: Vec<TrackState>,
}

impl SectionAnimations {
    /// Runtime state for the authored `tracks`, all at rest.
    pub fn new(tracks: Vec<SectionAnimation>) -> Self {
        Self {
            tracks: tracks
                .into_iter()
                .map(|config| TrackState {
                    config,
                    progress: 0.0,
                    target: 0.0,
                    dirty: false,
                    nodes: Vec::new(),
                })
                .collect(),
        }
    }

    /// True when the section authors no tracks.
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Steer every track of `cue` toward `target` (clamped 0..1). Travel
    /// runs at the track's authored speed; setting the same target twice is
    /// free, so a driver may call this every tick it holds a pose.
    pub fn set_cue(&mut self, cue: SectionAnimationCue, target: f32) {
        let target = target.clamp(0.0, 1.0);
        for track in &mut self.tracks {
            if track.config.cue == cue && track.target != target {
                track.target = target;
                track.dirty = true;
            }
        }
    }

    /// Land every track of `cue` AT `target` (clamped 0..1), skipping the
    /// travel: progress and target both jump. For cold-start poses that must
    /// not play out as motion - a scene that begins stowed snaps its stow
    /// cues here, and the rig writes the landed pose the moment it resolves.
    pub fn snap_cue(&mut self, cue: SectionAnimationCue, target: f32) {
        let target = target.clamp(0.0, 1.0);
        for track in &mut self.tracks {
            if track.config.cue == cue {
                track.target = target;
                track.progress = target;
                track.dirty = true;
            }
        }
    }

    /// True when the section authors at least one track of `cue`.
    pub fn has_cue(&self, cue: SectionAnimationCue) -> bool {
        self.tracks.iter().any(|track| track.config.cue == cue)
    }

    /// The progress of the first track of `cue`, if the section authors one.
    /// 0 is rest, 1 is fully deployed. For walk asserts and tests.
    pub fn cue_progress(&self, cue: SectionAnimationCue) -> Option<f32> {
        self.tracks
            .iter()
            .find(|track| track.config.cue == cue)
            .map(|track| track.progress)
    }

    /// Where the first track of `cue` is HEADED, if the section authors one -
    /// as against [`cue_progress`](Self::cue_progress), which is where it has
    /// got to.
    ///
    /// The pair is what makes a mechanism's direction readable. Progress alone
    /// cannot say it: a door 40% open is the same reading whether it is
    /// opening or closing, and a driver that wants the EDGE (to report it, to
    /// sound it) needs the target it is about to overwrite.
    pub fn cue_target(&self, cue: SectionAnimationCue) -> Option<f32> {
        self.tracks
            .iter()
            .find(|track| track.config.cue == cue)
            .map(|track| track.target)
    }

    /// Every track's progress and target, without the resolved scene nodes.
    pub fn freeze(&self) -> FrozenSectionAnimations {
        FrozenSectionAnimations(
            self.tracks
                .iter()
                .map(|track| FrozenTrack {
                    progress: track.progress,
                    target: track.target,
                })
                .collect(),
        )
    }

    /// Land every track at the progress and target `frozen` recorded, so a
    /// thawed rig resumes mid-travel where it froze. The rig writes the pose
    /// when its nodes resolve.
    ///
    /// # Panics
    ///
    /// When `frozen` holds a different track count: it was frozen from
    /// another section design, and pairing its tracks by position would
    /// pose the wrong parts.
    pub fn thaw(&mut self, frozen: &FrozenSectionAnimations) {
        assert_eq!(
            self.tracks.len(),
            frozen.0.len(),
            "SectionAnimations::thaw: frozen track count differs from the section's"
        );
        for (track, frozen) in self.tracks.iter_mut().zip(&frozen.0) {
            track.progress = frozen.progress;
            track.target = frozen.target;
            track.dirty = true;
        }
    }
}

/// A section's animation tracks when its body froze: each track's progress
/// and target, in authored track order, with no `Entity` in it.
#[derive(Clone, Debug, PartialEq)]
pub struct FrozenSectionAnimations(Vec<FrozenTrack>);

/// One track's progress and target when its body froze.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrozenTrack {
    progress: f32,
    target: f32,
}

/// Marks a section whose animation rig must be (re)resolved against its
/// spawned scene nodes. Inserted by the [`WorldInstanceReady`] observer when
/// a scene finishes spawning under an animated section, and by a kind system
/// whose animated nodes are code-built rather than scene-spawned (the turret
/// stow armer's lift joint); tests insert it by hand on a hand-built
/// hierarchy.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct SectionAnimationRigDirty;

/// A scene finished spawning somewhere below a section: if that section
/// authors animation tracks, queue a rig resolve. The scene's nodes are at
/// their authored pose on this flush, which is what resolution captures as
/// the rest pose.
fn mark_ready_section_rigs(
    ready: On<WorldInstanceReady>,
    q_child_of: Query<&ChildOf>,
    q_sections: Query<&SectionAnimations>,
    mut commands: Commands,
) {
    for ancestor in q_child_of.iter_ancestors(ready.entity) {
        if q_sections.get(ancestor).is_ok_and(|a| !a.is_empty()) {
            commands.entity(ancestor).insert(SectionAnimationRigDirty);
            return;
        }
    }
}

/// Match every track's `node_prefix` against the names below a dirty
/// section and capture each hit's transform as the track's rest pose.
/// Whole-tree and idempotent: a section with several scenes re-walks them
/// all on each ready, and a replaced scene drops its dead entities here.
///
/// A node the track ALREADY holds keeps its captured rest. A section with
/// several scenes (a turret's part glbs) readies once per scene, and by the
/// second walk the driver may have posed the survivors of the first - their
/// current transform is a driven pose, and re-capturing it as "rest" would
/// compose the motion onto itself. Fresh entities are genuinely at their
/// authored pose (nothing drives an unresolved node), so first capture is
/// the only correct one.
///
/// Each fresh node gets the track's current pose here, not from the driver:
/// this runs in `SpawnScene` right after the scene spawner, after `Update`,
/// so a scene's first rendered frame already shows its rig's pose. A stowed
/// emitter never renders one frame at its deployed rest pose.
fn resolve_section_animation_rigs(
    mut q_dirty: Query<
        (Entity, Option<&Name>, &mut SectionAnimations),
        With<SectionAnimationRigDirty>,
    >,
    q_children: Query<&Children>,
    mut q_named: Query<(&Name, &mut Transform)>,
    mut commands: Commands,
) {
    for (section, section_name, mut animations) in &mut q_dirty {
        let section_label = match section_name {
            Some(name) => format!("{name} ({section})"),
            None => section.to_string(),
        };
        let known: Vec<Vec<TrackNode>> = animations
            .tracks
            .iter_mut()
            .map(|track| std::mem::take(&mut track.nodes))
            .collect();
        for node in q_children.iter_descendants(section) {
            let Ok((name, mut transform)) = q_named.get_mut(node) else {
                continue;
            };
            // Read once: a track that matches first poses the node below.
            let authored = *transform;
            for (track, known) in animations.tracks.iter_mut().zip(&known) {
                let prefix = track.config.node_prefix.as_str();
                if !name.as_str().starts_with(prefix) {
                    continue;
                }
                if let Some(seen) = known.iter().find(|seen| seen.entity == node) {
                    track.nodes.push(*seen);
                    continue;
                }
                let resolved = TrackNode {
                    entity: node,
                    rest: authored,
                    slat: track.config.motion.slat(&section_label, name, prefix),
                };
                track
                    .config
                    .motion
                    .apply(&resolved, track.progress, &mut transform);
                track.nodes.push(resolved);
            }
        }
        commands
            .entity(section)
            .remove::<SectionAnimationRigDirty>();
    }
}

/// Run every track toward its target at the authored speed and write the
/// resulting pose onto the resolved nodes. Render-clock: these transforms
/// are art, never physics - colliders and spawn points do not move.
fn drive_section_animations(
    time: Res<Time>,
    mut q_sections: Query<&mut SectionAnimations>,
    mut q_transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    for mut animations in &mut q_sections {
        // Settle check through `Deref` only: every section carries this
        // component and most rigs are empty or at rest, so skipping before
        // any mutable deref keeps them out of downstream change detection.
        if animations
            .tracks
            .iter()
            .all(|track| track.progress == track.target && !track.dirty)
        {
            continue;
        }
        for track in &mut animations.tracks {
            if track.progress != track.target {
                let seconds = if track.target > track.progress {
                    track.config.open_seconds
                } else {
                    track.config.close_seconds
                };
                track.progress = if seconds > 0.0 {
                    let step = dt / seconds;
                    if track.target > track.progress {
                        (track.progress + step).min(track.target)
                    } else {
                        (track.progress - step).max(track.target)
                    }
                } else {
                    track.target
                };
                track.dirty = true;
            }
            if !track.dirty {
                continue;
            }
            for node in &track.nodes {
                // A despawned scene node is skipped, not an error: the rig
                // re-resolves when its replacement scene readies.
                if let Ok(mut transform) = q_transforms.get_mut(node.entity) {
                    track
                        .config
                        .motion
                        .apply(node, track.progress, &mut transform);
                }
            }
            track.dirty = false;
        }
    }
}

/// System set for the section-animation driver, on the render clock
/// (`Update`). Rig resolution runs later, in `SpawnScene`. Cue writers on
/// the fixed clock (the bay's fire path) need no edge against this set: a
/// target written in `FixedUpdate` is picked up the same frame, because
/// `FixedUpdate` runs first.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SectionAnimationSystems;

/// The generic section-animation machinery: scene-node rig resolution and
/// the per-frame progress driver. Added by `SpaceshipSectionPlugin` for
/// every app; without rendered scenes the rigs simply stay empty and the
/// driver only moves numbers.
#[derive(Default, Clone, Debug)]
pub struct SectionAnimationPlugin;

impl Plugin for SectionAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SectionAnimations>();
        app.add_observer(mark_ready_section_rigs);
        app.add_systems(
            Update,
            drive_section_animations.in_set(SectionAnimationSystems),
        );
        app.add_systems(
            SpawnScene,
            resolve_section_animation_rigs.after(SceneSpawnerSystems::WorldInstanceSpawn),
        );
    }
}

#[cfg(test)]
mod tests;
