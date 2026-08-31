//! Authored section animation: named nodes in a section's render scene,
//! driven procedurally by gameplay cues.
//!
//! A section's config declares TRACKS ([`SectionAnimation`]): which scene
//! nodes move (by glTF node-name prefix), how ([`SectionAnimationMotion`],
//! composed onto each node's authored rest pose), and how fast. The cue
//! ([`SectionAnimationCue`]) is the contract between art and mechanics:
//! content owns WHAT moves, the section kind's own systems own WHEN by
//! steering the cue's target through [`SectionAnimations::set_cue`]. See
//! `tasks/20260831-083625/animation-research.md` for why this is procedural
//! data rather than glTF clips.
//!
//! This module owns the generic half only - rig resolution against spawned
//! scenes and the progress driver. It knows nothing about torpedoes or
//! turrets; kind modules (the bay's muzzle door, later the railgun charge
//! and the PDC stow) write cue targets and nothing else.

use bevy::{prelude::*, world_serialization::WorldInstanceReady};

/// The authored track types, the runtime [`SectionAnimations`] component, and
/// `SectionAnimationPlugin` with `SectionAnimationSystems`.
pub mod prelude {
    pub use super::{
        SectionAnimation, SectionAnimationCue, SectionAnimationMotion, SectionAnimationPlugin,
        SectionAnimationRigDirty, SectionAnimationSystems, SectionAnimations,
    };
}

/// Which gameplay moment drives an authored animation track. One cue can
/// drive several tracks (a stow that folds a cover AND drops the mount); a
/// cue no system steers rests at progress 0. Add a variant here when a new
/// mechanic wants art: the railgun charge is the known next consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionAnimationCue {
    /// A weapon bay's muzzle cover: driven to 1 (open) across a launch and
    /// back to 0 (closed) at rest by the bay's own fire path.
    MuzzleDoor,
    /// A retractable turret's elevator: driven to 1 (sunk into the housing)
    /// while stowed, 0 (raised, the rest pose) while deployed. Steered by
    /// the turret's stow state machine, which sequences it against
    /// [`Self::StowDoors`].
    StowLift,
    /// A retractable turret's housing lids: driven to 1 (shut over the sunk
    /// gun) while stowed, 0 (parted, the rest pose) while deployed. The stow
    /// machine shuts them only after [`Self::StowLift`] reaches 1, and parts
    /// them before raising it.
    StowDoors,
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
        /// Node-local displacement at full progress, in world units.
        offset: Vec3,
    },
}

impl SectionAnimationMotion {
    /// Write the pose at `progress` onto `transform`, relative to `rest`.
    fn apply(self, rest: &Transform, progress: f32, transform: &mut Transform) {
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
        }
    }
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
    /// Forces one transform write even at rest - set on resolve and on
    /// retarget, so late-spawning scenes land on the current pose.
    dirty: bool,
    /// The matched node entities and their authored rest transforms,
    /// captured when the scene instance readies.
    nodes: Vec<(Entity, Transform)>,
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
fn resolve_section_animation_rigs(
    mut q_dirty: Query<(Entity, &mut SectionAnimations), With<SectionAnimationRigDirty>>,
    q_children: Query<&Children>,
    q_named: Query<(&Name, &Transform)>,
    mut commands: Commands,
) {
    for (section, mut animations) in &mut q_dirty {
        let known: Vec<Vec<(Entity, Transform)>> = animations
            .tracks
            .iter_mut()
            .map(|track| std::mem::take(&mut track.nodes))
            .collect();
        for node in q_children.iter_descendants(section) {
            let Ok((name, transform)) = q_named.get(node) else {
                continue;
            };
            for (track, known) in animations.tracks.iter_mut().zip(&known) {
                if name.as_str().starts_with(track.config.node_prefix.as_str()) {
                    let rest = known
                        .iter()
                        .find(|(seen, _)| *seen == node)
                        .map_or(*transform, |&(_, rest)| rest);
                    track.nodes.push((node, rest));
                    track.dirty = true;
                }
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
            for &(node, rest) in &track.nodes {
                // A despawned scene node is skipped, not an error: the rig
                // re-resolves when its replacement scene readies.
                if let Ok(mut transform) = q_transforms.get_mut(node) {
                    track
                        .config
                        .motion
                        .apply(&rest, track.progress, &mut transform);
                }
            }
            track.dirty = false;
        }
    }
}

/// System set for the section-animation rig resolution and driver, on the
/// render clock (`Update`). Cue writers on the fixed clock (the bay's fire
/// path) need no edge against this set: a target written in `FixedUpdate`
/// is picked up the same frame, because `FixedUpdate` runs first.
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
            (resolve_section_animation_rigs, drive_section_animations)
                .chain()
                .in_set(SectionAnimationSystems),
        );
    }
}

#[cfg(test)]
mod tests;
