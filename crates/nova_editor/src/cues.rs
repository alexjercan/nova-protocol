//! The editor's four voices: seating a part, lifting one out, turning the
//! ghost a detent, and refusing a placement.
//!
//! One [`SystemParam`] rather than a bank lookup at each call site, because
//! the four are a SET: they are placed a band under the menus (the editor is a
//! workspace a builder sits in for an hour, not a screen they pass through),
//! and that only holds if their levels stay together. Every cue routes
//! [`AudioRoute::Interface`] - the editor has no world to hear them in.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_gameplay::prelude::*;

use crate::config::PlacementPose;

/// The editor's cue bank, as a `SystemParam` so a system that already takes
/// eight arguments adds one.
///
/// A missing [`SoundBank`] is a graceful silence: an editor opened before the
/// asset load finished is still an editor.
#[derive(SystemParam)]
pub(crate) struct EditorCues<'w, 's> {
    bank: Option<Res<'w, SoundBank<UiSfx>>>,
    commands: Commands<'w, 's>,
}

impl EditorCues<'_, '_> {
    fn play(&mut self, key: UiSfx, volume: f32) {
        let Some(bank) = self.bank.as_ref() else {
            return;
        };
        self.commands
            .play_sfx(bank.get(key), AudioRoute::Interface, volume);
    }

    /// A part seated in the grid.
    pub(crate) fn place(&mut self) {
        self.play(UiSfx::EditorPlace, EDITOR_PLACE_VOLUME);
    }

    /// A part deleted off the ship, by the Del key or the Ship menu row. The
    /// caller cues it past its own guards, so a delete the editor refuses
    /// stays silent.
    pub(crate) fn remove(&mut self) {
        self.play(UiSfx::EditorRemove, EDITOR_REMOVE_VOLUME);
    }

    /// The ghost turned one detent.
    pub(crate) fn rotate(&mut self) {
        self.play(UiSfx::EditorRotate, EDITOR_ROTATE_VOLUME);
    }

    /// A placement the graph refused.
    pub(crate) fn deny(&mut self) {
        self.play(UiSfx::EditorDeny, EDITOR_DENY_VOLUME);
    }
}

/// Tick the ghost's detent whenever the placement pose actually MOVES.
///
/// One watcher on the resource rather than a cue at each of the five gestures
/// that write it - the R key, the F key, the wheel, Ctrl+wheel, and the two
/// menu rows. The pose is the thing the builder is steering, so the pose is
/// where the sound belongs, and a sixth way to turn a part inherits the tick
/// for free.
///
/// Compared by VALUE, not by `is_changed`: a `ResMut` marks the resource
/// changed on deref, and `cycle_placement_pose` holds one every frame a part
/// is armed. It would tick continuously.
pub(crate) fn play_placement_pose_cue(
    pose: Res<PlacementPose>,
    mut last: ResMut<PlacementPoseHeard>,
    mut cues: EditorCues,
) {
    let current = *pose;
    let moved = last.0.is_some_and(|last| last != current);
    last.0 = Some(current);
    if moved {
        cues.rotate();
    }
}

/// The pose the ghost detent last spoke for.
///
/// A RESOURCE and not a `Local`, so the entry that resets [`PlacementPose`] can
/// reset this beside it. The watcher only runs in the editor state, and a
/// `Local` outlives the trip out to a test flight: coming back, the reset pose
/// was compared against the one the builder left with and the ghost ticked for
/// a turn nobody made.
#[derive(Resource, Default, Debug)]
pub(crate) struct PlacementPoseHeard(pub(crate) Option<PlacementPose>);

#[cfg(test)]
mod tests {
    use nova_gameplay::audio::UI_SFX_FILES;

    use super::*;

    /// A headless rig for the pose watcher: the real system, a loaded bank, and
    /// a `PlaySfx` capture. No audio device - `PlaySfx` is a report, and the
    /// engine owns every sink write.
    fn pose_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(SoundBank::load(
            app.world().resource::<AssetServer>(),
            UI_SFX_FILES,
        ));
        app.init_resource::<PlacementPose>();
        app.init_resource::<PlacementPoseHeard>();
        app.init_resource::<Ticks>();
        app.add_observer(|_: On<PlaySfx>, mut ticks: ResMut<Ticks>| ticks.0 += 1);
        app.add_systems(Update, play_placement_pose_cue);
        app
    }

    #[derive(Resource, Default)]
    struct Ticks(usize);

    fn ticks(app: &App) -> usize {
        app.world().resource::<Ticks>().0
    }

    #[test]
    fn the_ghost_ticks_when_its_pose_moves_and_not_while_it_is_merely_armed() {
        let mut app = pose_app();

        // Arming a part holds a `ResMut<PlacementPose>` every frame, which
        // marks the resource changed without moving it. Frame one is also the
        // resource's own insertion. Neither is a detent.
        app.update();
        app.update();
        assert_eq!(ticks(&app), 0, "an armed, unturned part is silent");

        app.world_mut().resource_mut::<PlacementPose>().roll = 1;
        app.update();
        assert_eq!(ticks(&app), 1);

        // A frame that touches the pose without changing it: the builder is
        // holding a part, not turning it.
        let _ = app.world_mut().resource_mut::<PlacementPose>();
        app.update();
        assert_eq!(ticks(&app), 1);

        // Leaving for a test flight and coming back. The editor's entry resets
        // both the pose and what the detent last heard; without the second
        // reset the fresh pose reads as a turn nobody made.
        *app.world_mut().resource_mut::<PlacementPose>() = PlacementPose::default();
        app.world_mut().resource_mut::<PlacementPoseHeard>().0 = None;
        app.update();
        assert_eq!(ticks(&app), 1, "coming back to the editor is silent");

        // Rolling and cycling the socket in ONE frame is one detent, not two:
        // the pose is what moved, and it moved once.
        {
            let mut pose = app.world_mut().resource_mut::<PlacementPose>();
            pose.roll = 2;
            pose.source = 3;
        }
        app.update();
        assert_eq!(ticks(&app), 2);
    }
}
