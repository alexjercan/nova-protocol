//! The front door for game sounds: a one-shot is a [`PlaySound`] event, a loop
//! is a [`SoundLoop`] component. The caller names the sound, its
//! [`SoundCategoryType`] and where it happens, and nothing else: the category
//! decides the level, the reach, the repeat interval and the voice count.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_sound::prelude::*;
//! # fn demo(mut commands: Commands, door: Handle<AudioSource>, ship_section: Entity) {
//! commands.trigger(PlaySound {
//!     sound: door,
//!     category: SoundCategoryType::Machinery,
//!     at: SoundAt::Entity(ship_section),
//! });
//! # }
//! ```

use std::time::Duration;

use bevy::prelude::*;

use super::{
    category::{SoundBus, SoundCategoryType},
    mixing::{area_cell, CueThrottle},
    voice::{AwaitingSink, LoopRelease, VoiceMarker},
};

/// Marks the voice entity behind one [`PlaySound`] one-shot.
///
/// Exists for owners OUTSIDE this crate: the entity's own despawn rides its
/// audio sink (`PlaybackMode::Despawn`), which plays on the wall clock and is
/// never created without an output device, so nothing else can recognize a clip
/// that outlived whoever asked for it. nova_scenario scopes on this marker so a
/// one-shot spawned by a live scenario dies with the teardown instead of
/// playing into the next scenario.
#[derive(Component, Debug, Clone, Reflect)]
pub struct SfxAudioMarker;

/// Where a one-shot happens.
#[derive(Clone, Copy, PartialEq, Debug, Reflect)]
pub enum SoundAt {
    /// Interface feedback, heard flat. Only a [`SoundBus::Ui`] category may
    /// use it.
    Ui,
    /// On this entity, at its world position when the sound starts. The
    /// sound groups with the entity's `ChildOf` root, so one ship's bay doors
    /// share one voice. Only a [`SoundBus::Effect`] category may use it.
    Entity(Entity),
    /// At a world point, for a sound with no body over it. It groups with the
    /// other sounds in its world cell. Only a [`SoundBus::Effect`] category
    /// may use it.
    Point(Vec3),
}

/// Play a one-shot sound. Trigger it; the engine spawns the voice and retires
/// it when the clip ends.
///
/// A category whose bus does not match `at` (a [`SoundBus::Ui`] category in
/// the world, or an [`SoundBus::Effect`] category at [`SoundAt::Ui`]) is a
/// programming error: it is logged and dropped. So is a loop category, and a
/// [`SoundBus::Music`] one.
#[derive(Event, Clone, Debug)]
pub struct PlaySound {
    /// The sound to play.
    pub sound: Handle<AudioSource>,
    /// What kind of sound it is.
    pub category: SoundCategoryType,
    /// Where it happens.
    pub at: SoundAt,
}

impl PlaySound {
    /// Interface feedback: `sound` heard flat as a `category` cue. The
    /// category must be a [`SoundBus::Ui`] one.
    pub fn ui(sound: Handle<AudioSource>, category: SoundCategoryType) -> Self {
        Self {
            sound,
            category,
            at: SoundAt::Ui,
        }
    }
}

/// A looping sound on this entity: heard from its world position on
/// [`SoundBus::Effect`], flat on [`SoundBus::Ui`].
///
/// The owner writes [`Self::intensity`] and [`Self::rate`] each frame; the
/// category maps the intensity onto its loop dB range.
///
/// The release is the engine's. Write zero on the frame the source idles and
/// the voice ramps to silence inside
/// [`LOOP_RELEASE_SECONDS`](super::LOOP_RELEASE_SECONDS). Do not ease the
/// intensity toward zero: every value above zero plays at least the bottom of
/// the dB range, so an eased tail holds the loop there until the ease ends.
/// Remove the component, or despawn the entity, to cut the loop with no
/// release.
#[derive(Component, Clone, Debug)]
pub struct SoundLoop {
    /// The sound to loop.
    pub sound: Handle<AudioSource>,
    /// What kind of loop it is. Must be a loop category.
    pub category: SoundCategoryType,
    /// How hard the source is working, `0.0..=1.0`. Zero is an idle source:
    /// the voice releases to silence.
    pub intensity: f32,
    /// Playback rate, which also shifts pitch. `1.0` is normal.
    pub rate: f32,
}

impl SoundLoop {
    /// A silent loop at normal rate. Raise [`Self::intensity`] to bring it in.
    pub fn new(sound: Handle<AudioSource>, category: SoundCategoryType) -> Self {
        Self {
            sound,
            category,
            intensity: 0.0,
            rate: 1.0,
        }
    }
}

/// Where a [`SoundVoice`] is heard from.
#[derive(Clone, Copy, PartialEq, Debug, Reflect)]
pub enum VoicePlace {
    /// Flat interface feedback.
    Ui,
    /// A fixed world point.
    At(Vec3),
    /// A loop's owner, re-read every frame. Silent once the owner is gone.
    Follow(Entity),
}

/// What a voice is counted with for the repeat interval and the voice cap.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
pub enum VoiceGroup {
    /// Interface feedback: one group per sound.
    Ui,
    /// Everything on one `ChildOf` root entity: one ship, one rock.
    Root(Entity),
    /// A world cell, for a sound with no body.
    Cell(IVec3),
}

/// One category sound the engine is playing. Spawned by the engine from a
/// [`PlaySound`] or a [`SoundLoop`]; read it, never spawn it.
#[derive(Component, Clone, Debug)]
#[require(VoiceMarker)]
pub struct SoundVoice {
    /// The sound.
    pub sound: Handle<AudioSource>,
    /// Its category.
    pub category: SoundCategoryType,
    /// Where it is heard from.
    pub place: VoicePlace,
    /// What it is counted with.
    pub group: VoiceGroup,
    /// Whether it loops.
    pub looping: bool,
    /// A loop's intensity, copied from its [`SoundLoop`] each frame. `1.0`
    /// for a one-shot.
    pub intensity: f32,
    /// Playback rate. `1.0` for a one-shot.
    pub rate: f32,
    /// When it was spawned, in real time: the cap stops the oldest one-shot.
    pub(crate) started: Duration,
    /// A loop's release ramp. Unused by a one-shot.
    pub(crate) release: LoopRelease,
}

/// The repeat-interval budget, keyed by sound and group.
#[derive(Resource, Default)]
pub struct SoundRepeat(CueThrottle<(AssetId<AudioSource>, VoiceGroup)>);

/// Seconds a repeat key is kept after it last fired. Longer than every
/// category's repeat interval.
const SOUND_REPEAT_PRUNE_WINDOW: f32 = 2.0;

/// A loop owner's engine voice.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct LoopVoice(Entity);

/// Why a [`PlaySound`] was dropped.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SoundRefusal {
    /// A [`SoundBus::Ui`] category placed in the world, or an
    /// [`SoundBus::Effect`] category at [`SoundAt::Ui`].
    BusMismatch,
    /// A loop category fired as a one-shot.
    LoopAsOneShot,
    /// A [`SoundBus::Music`] category fired as a cue. Only the
    /// [`MusicDeck`](super::MusicDeck) plays music.
    MusicAsCue,
    /// [`SoundAt::Entity`] named an entity with no world pose.
    NoPose,
}

/// Marks a container whose children are separate bodies: a world sector that
/// owns the ships and rocks standing in it. [`sound_root`] stops below it, so
/// two ships in one sector are two groups.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SoundGroupBoundary;

/// The ROOT of `entity`: the top of its `ChildOf` chain, or the last entity
/// below a [`SoundGroupBoundary`].
pub fn sound_root(
    entity: Entity,
    q_parents: &Query<&ChildOf>,
    q_boundaries: &Query<(), With<SoundGroupBoundary>>,
) -> Entity {
    let mut current = entity;
    while let Ok(&ChildOf(parent)) = q_parents.get(current) {
        if q_boundaries.contains(parent) {
            break;
        }
        current = parent;
    }
    current
}

/// Resolve where a one-shot plays and what it groups with. Pure, so the
/// refusals are testable without an app.
///
/// `pose` and `root` describe [`SoundAt::Entity`]'s entity and are not read
/// for the other places.
pub fn place_one_shot(
    category: SoundCategoryType,
    at: SoundAt,
    pose: Option<Vec3>,
    root: Entity,
) -> Result<(VoicePlace, VoiceGroup), SoundRefusal> {
    if category.is_loop() {
        return Err(SoundRefusal::LoopAsOneShot);
    }
    match (category.bus(), at) {
        (SoundBus::Music, _) => Err(SoundRefusal::MusicAsCue),
        (SoundBus::Ui, SoundAt::Ui) => Ok((VoicePlace::Ui, VoiceGroup::Ui)),
        (SoundBus::Effect, SoundAt::Point(point)) => {
            Ok((VoicePlace::At(point), VoiceGroup::Cell(area_cell(point))))
        }
        (SoundBus::Effect, SoundAt::Entity(_)) => pose
            .map(|pose| (VoicePlace::At(pose), VoiceGroup::Root(root)))
            .ok_or(SoundRefusal::NoPose),
        (SoundBus::Ui, SoundAt::Entity(_) | SoundAt::Point(_))
        | (SoundBus::Effect, SoundAt::Ui) => Err(SoundRefusal::BusMismatch),
    }
}

/// Turn a [`PlaySound`] into a voice, inside the category's repeat interval
/// and voice count.
pub(crate) fn on_play_sound(
    event: On<PlaySound>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut repeat: ResMut<SoundRepeat>,
    q_pose: Query<&GlobalTransform>,
    q_parents: Query<&ChildOf>,
    q_boundaries: Query<(), With<SoundGroupBoundary>>,
    q_voices: Query<(Entity, &SoundVoice)>,
) {
    let PlaySound {
        ref sound,
        category,
        at,
    } = *event;
    let (pose, root) = match at {
        SoundAt::Entity(entity) => (
            q_pose.get(entity).ok().map(GlobalTransform::translation),
            sound_root(entity, &q_parents, &q_boundaries),
        ),
        SoundAt::Ui | SoundAt::Point(_) => (None, Entity::PLACEHOLDER),
    };
    let (place, group) = match place_one_shot(category, at, pose, root) {
        Ok(placed) => placed,
        Err(SoundRefusal::NoPose) => {
            debug!("nova audio: dropped {category:?} at {at:?}: the entity has no pose");
            return;
        }
        Err(refusal) => {
            error!("nova audio: dropped {category:?} at {at:?}: {refusal:?}");
            return;
        }
    };
    let spec = category.spec();
    let now = time.elapsed();
    if !repeat
        .0
        .allow((sound.id(), group), now.as_secs_f32(), spec.repeat_interval)
    {
        return;
    }

    // The voice count: stop the oldest one-shots of this sound in this group
    // until the new one fits.
    let mut playing: Vec<(Duration, Entity)> = q_voices
        .iter()
        .filter(|(_, voice)| {
            !voice.looping && voice.group == group && voice.sound.id() == sound.id()
        })
        .map(|(entity, voice)| (voice.started, entity))
        .collect();
    let keep = usize::from(spec.max_voices).saturating_sub(1);
    if playing.len() > keep {
        playing.sort_unstable();
        for (_, entity) in &playing[..playing.len() - keep] {
            commands.entity(*entity).try_despawn();
        }
    }

    commands.spawn((
        Name::new("Sound"),
        SfxAudioMarker,
        SoundVoice {
            sound: sound.clone(),
            category,
            place,
            group,
            looping: false,
            intensity: 1.0,
            rate: 1.0,
            started: now,
            release: LoopRelease::default(),
        },
        AwaitingSink(now),
    ));
}

/// Give every [`SoundLoop`] owner its voice, keep the voice's intensity and
/// rate in step with the owner, and restart it when the owner changes sound.
///
/// A voice that is gone (stopped with the world voices at the end of play) is
/// spawned again, silent until its owner raises the intensity.
pub(crate) fn sync_sound_loops(
    mut commands: Commands,
    time: Res<Time<Real>>,
    q_parents: Query<&ChildOf>,
    q_boundaries: Query<(), With<SoundGroupBoundary>>,
    q_owners: Query<(Entity, &SoundLoop, Option<&LoopVoice>)>,
    mut q_voices: Query<&mut SoundVoice>,
) {
    for (owner, sound_loop, link) in &q_owners {
        if let Some(&LoopVoice(voice)) = link {
            if let Ok(mut voice) = q_voices.get_mut(voice) {
                if voice.sound == sound_loop.sound && voice.category == sound_loop.category {
                    voice.intensity = sound_loop.intensity;
                    voice.rate = sound_loop.rate;
                    continue;
                }
            }
            commands.entity(voice).try_despawn();
        }
        if !sound_loop.category.is_loop() {
            error!(
                "nova audio: {owner} loops {:?}, which is not a loop category",
                sound_loop.category
            );
            commands.entity(owner).remove::<SoundLoop>();
            continue;
        }
        let voice = commands
            .spawn((
                Name::new("Sound Loop"),
                SoundVoice {
                    sound: sound_loop.sound.clone(),
                    category: sound_loop.category,
                    place: VoicePlace::Follow(owner),
                    group: VoiceGroup::Root(sound_root(owner, &q_parents, &q_boundaries)),
                    looping: true,
                    intensity: sound_loop.intensity,
                    rate: sound_loop.rate,
                    started: time.elapsed(),
                    release: LoopRelease::default(),
                },
            ))
            .id();
        commands.entity(owner).insert(LoopVoice(voice));
    }
}

/// Stop a loop's voice with its [`SoundLoop`].
pub(crate) fn on_remove_sound_loop(
    remove: On<Remove<SoundLoop>>,
    mut commands: Commands,
    q_links: Query<&LoopVoice>,
) {
    if let Ok(&LoopVoice(voice)) = q_links.get(remove.entity) {
        commands.entity(voice).try_despawn();
        commands.entity(remove.entity).try_remove::<LoopVoice>();
    }
}

/// Keep the repeat budget bounded.
pub(crate) fn prune_sound_repeat(time: Res<Time<Real>>, mut repeat: ResMut<SoundRepeat>) {
    repeat
        .0
        .prune(time.elapsed_secs(), SOUND_REPEAT_PRUNE_WINDOW);
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOBODY: Entity = Entity::PLACEHOLDER;

    #[test]
    fn a_bus_mismatch_is_refused() {
        assert_eq!(
            place_one_shot(
                SoundCategoryType::Cockpit,
                SoundAt::Point(Vec3::ZERO),
                None,
                NOBODY
            ),
            Err(SoundRefusal::BusMismatch),
            "a cockpit tone is never placed in the world"
        );
        assert_eq!(
            place_one_shot(
                SoundCategoryType::Cockpit,
                SoundAt::Entity(NOBODY),
                Some(Vec3::ZERO),
                NOBODY
            ),
            Err(SoundRefusal::BusMismatch)
        );
        assert_eq!(
            place_one_shot(SoundCategoryType::Weapon, SoundAt::Ui, None, NOBODY),
            Err(SoundRefusal::BusMismatch),
            "a gun is never heard flat"
        );
        assert!(place_one_shot(SoundCategoryType::Cockpit, SoundAt::Ui, None, NOBODY).is_ok());
    }

    #[test]
    fn a_loop_category_is_refused_as_a_one_shot() {
        assert_eq!(
            place_one_shot(
                SoundCategoryType::Drive,
                SoundAt::Point(Vec3::ZERO),
                None,
                NOBODY
            ),
            Err(SoundRefusal::LoopAsOneShot)
        );
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SoundRepeat>();
        app.add_observer(on_play_sound);
        app
    }

    fn voices_in(app: &mut App, group: VoiceGroup) -> usize {
        let mut voices = app.world_mut().query::<&SoundVoice>();
        voices
            .iter(app.world())
            .filter(|voice| voice.group == group)
            .count()
    }

    /// Two bays on one ship opening on one frame are one door, and a second
    /// ship's door in the same frame is still heard.
    #[test]
    fn the_repeat_interval_holds_per_sound_and_root() {
        let mut app = app();
        // Both ships stand in one world sector, which is not their root.
        let sector = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, SoundGroupBoundary))
            .id();
        let ship = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, ChildOf(sector)))
            .id();
        let bays: Vec<Entity> = (0..3)
            .map(|_| {
                app.world_mut()
                    .spawn((GlobalTransform::IDENTITY, ChildOf(ship)))
                    .id()
            })
            .collect();
        let other_ship = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, ChildOf(sector)))
            .id();
        let door: Handle<AudioSource> = Handle::default();
        let open = |entity| PlaySound {
            sound: door.clone(),
            category: SoundCategoryType::Machinery,
            at: SoundAt::Entity(entity),
        };
        for &bay in &bays {
            app.world_mut().trigger(open(bay));
        }
        app.world_mut().trigger(open(other_ship));
        app.update();

        assert_eq!(voices_in(&mut app, VoiceGroup::Root(ship)), 1);
        assert_eq!(voices_in(&mut app, VoiceGroup::Root(other_ship)), 1);
    }

    /// Past the category's voice count the oldest one-shot of that sound on
    /// that root is stopped for the new one.
    #[test]
    fn the_voice_count_holds_per_sound_and_root() {
        let mut app = app();
        let ship = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let gun: Handle<AudioSource> = Handle::default();
        let max = usize::from(SoundCategoryType::Weapon.spec().max_voices);
        for _ in 0..max + 3 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_secs(1));
            app.world_mut().trigger(PlaySound {
                sound: gun.clone(),
                category: SoundCategoryType::Weapon,
                at: SoundAt::Entity(ship),
            });
            app.world_mut().flush();
        }
        assert_eq!(voices_in(&mut app, VoiceGroup::Root(ship)), max);
    }
}
