//! The mixer's routing model: which track scales a cue, and - inside the world
//! track - whether the pilot hears it through their own hull or out in space.
//!
//! Owns the per-bus volume resources. The amplitude
//! law is [`super::mixing`], the stereo placement is [`super::spatial`], and the
//! playback is [`super::voice`]; this module only decides HOW LOUD a route is
//! allowed to be.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::settings::prelude::{HarnessMute, MasterVolume};

/// A mixer track. Every voice belongs to exactly one, and a track's volume is
/// the only per-track control there is: nothing scales a cue except its bus
/// volume and the master.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AudioBus {
    /// UI chrome: menu clicks, objective chimes, the NOVA OS terminal. Never
    /// positional, never attenuated, never panned.
    Interface,
    /// Everything diegetic - the sounds the fiction says exist.
    World,
    /// RESERVED. The bus and its volume exist so the settings surface and the
    /// saved-settings format carry three sliders today; no voice routes here
    /// yet.
    Music,
}

impl AudioBus {
    /// Short display label for the settings surface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::World => "World",
            Self::Music => "Music",
        }
    }
}

/// Where a voice sits in the mix: its [`AudioBus`], and - inside
/// [`AudioBus::World`] - where the pilot hears it FROM.
///
/// This is one flat enum rather than a bus plus a separate tag because the two
/// world tags are the only place the distinction exists: an Interface cue heard
/// "through the hull" is not a thing, and a route that cannot be spelled cannot
/// be miswired.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum AudioRoute {
    /// UI chrome, on [`AudioBus::Interface`]. Non-positional.
    #[default]
    Interface,
    /// Structure-borne through the PLAYER'S OWN ship: your guns, your engines,
    /// your RCS, damage landing on your hull, your own reload and charge cues.
    /// Non-positional by definition - it is the room the pilot is sitting in,
    /// not a place out in the world - so it is never distance-attenuated and
    /// never panned. This routing fact REPLACES the old "if this is the player,
    /// skip attenuation" special case: your own engines are Hull, so there is
    /// no exemption left to write.
    Hull,
    /// Everything else in the world: another ship's guns, a distant explosion,
    /// an asteroid breaking up. Distance-attenuated AND panned by bearing - the
    /// only route that is either.
    Exterior,
    /// RESERVED, on [`AudioBus::Music`]. Nothing routes here yet.
    Music,
}

impl AudioRoute {
    /// The track this route is scaled by.
    pub fn bus(self) -> AudioBus {
        match self {
            Self::Interface => AudioBus::Interface,
            Self::Hull | Self::Exterior => AudioBus::World,
            Self::Music => AudioBus::Music,
        }
    }

    /// Whether the voice is placed in the world: attenuated by distance and
    /// panned by bearing. True for [`Self::Exterior`] alone.
    pub fn is_positional(self) -> bool {
        matches!(self, Self::Exterior)
    }
}

/// Linear volume of the [`AudioBus::Interface`] track, `0.0..=1.0`.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InterfaceVolume(pub f32);

impl Default for InterfaceVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl InterfaceVolume {
    /// The clamped linear factor, so a corrupt persisted value can never push
    /// the mixer out of range.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

/// Linear volume of the [`AudioBus::World`] track, `0.0..=1.0`. Scales both
/// world routes - [`AudioRoute::Hull`] and [`AudioRoute::Exterior`] - because
/// they are one track heard two ways, not two tracks.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldVolume(pub f32);

impl Default for WorldVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl WorldVolume {
    /// The clamped linear factor.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

/// Linear volume of the RESERVED [`AudioBus::Music`] track, `0.0..=1.0`. The
/// slider moves it and the store saves it; no voice reads it yet.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MusicVolume(pub f32);

impl Default for MusicVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl MusicVolume {
    /// The clamped linear factor.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

/// Read access to every knob that scales a voice: the three bus volumes and the
/// master.
///
/// Every resource is optional so an audio-only test rig that never adds
/// [`NovaAudioPlugin`](super::NovaAudioPlugin) or the settings plugin keeps full
/// volume instead of panicking on a missing resource - the same graceful
/// degradation the rest of the engine promises.
#[derive(SystemParam)]
pub struct Mixer<'w> {
    interface: Option<Res<'w, InterfaceVolume>>,
    world: Option<Res<'w, WorldVolume>>,
    music: Option<Res<'w, MusicVolume>>,
    master: Option<Res<'w, MasterVolume>>,
    mute: Option<Res<'w, HarnessMute>>,
}

impl Mixer<'_> {
    /// The gain `route`'s bus applies, WITHOUT the master.
    ///
    /// This is what a freshly spawned sink is given, because bevy's
    /// `audio_output` multiplies the master in for us through `GlobalVolume`.
    pub fn bus_gain(&self, route: AudioRoute) -> f32 {
        bus_gain(
            route,
            self.interface.as_deref().copied().unwrap_or_default(),
            self.world.as_deref().copied().unwrap_or_default(),
            self.music.as_deref().copied().unwrap_or_default(),
        )
    }

    /// The master output gain, which a PER-FRAME sink write must fold in by
    /// hand.
    ///
    /// Writing a sink's volume bypasses the `GlobalVolume` path bevy applies
    /// once at sink creation, so the master is re-applied here - through
    /// [`MasterVolume::output_gain`], which is what a [`HarnessMute`]d run
    /// silences.
    pub fn master_gain(&self) -> f32 {
        let mute = self.mute.as_deref().copied().unwrap_or_default();
        self.master.as_deref().map_or(1.0, |m| m.output_gain(mute))
    }

    /// The full gain a per-frame sink write applies: [`Self::bus_gain`] times
    /// [`Self::master_gain`].
    pub fn output_gain(&self, route: AudioRoute) -> f32 {
        self.bus_gain(route) * self.master_gain()
    }
}

/// The bus gain for one route, as a pure function of the knobs. Both world
/// routes read the same track: `Hull` and `Exterior` are one bus heard two
/// ways, and what separates them is the rolloff and the pan, not the volume.
pub fn bus_gain(
    route: AudioRoute,
    interface: InterfaceVolume,
    world: WorldVolume,
    music: MusicVolume,
) -> f32 {
    match route {
        AudioRoute::Interface => interface.factor(),
        AudioRoute::Hull | AudioRoute::Exterior => world.factor(),
        AudioRoute::Music => music.factor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_names_the_one_track_that_scales_it() {
        assert_eq!(AudioRoute::Interface.bus(), AudioBus::Interface);
        assert_eq!(AudioRoute::Hull.bus(), AudioBus::World);
        assert_eq!(AudioRoute::Exterior.bus(), AudioBus::World);
        assert_eq!(AudioRoute::Music.bus(), AudioBus::Music);
        // Only the exterior is placed in the world.
        assert!(AudioRoute::Exterior.is_positional());
        for route in [AudioRoute::Interface, AudioRoute::Hull, AudioRoute::Music] {
            assert!(!route.is_positional(), "{route:?} must be non-positional");
        }
    }

    #[test]
    fn a_bus_volume_scales_only_its_own_routes() {
        let quiet_world = WorldVolume(0.25);
        let gain = |route| {
            bus_gain(
                route,
                InterfaceVolume::default(),
                quiet_world,
                MusicVolume::default(),
            )
        };
        assert_eq!(gain(AudioRoute::Hull), 0.25);
        assert_eq!(
            gain(AudioRoute::Exterior),
            0.25,
            "the two world routes are one track heard two ways"
        );
        assert_eq!(
            gain(AudioRoute::Interface),
            1.0,
            "interface is its own track"
        );
        assert_eq!(gain(AudioRoute::Music), 1.0, "music is its own track");
    }

    #[test]
    fn a_corrupt_persisted_volume_cannot_push_the_mixer_out_of_range() {
        assert_eq!(InterfaceVolume(4.0).factor(), 1.0);
        assert_eq!(WorldVolume(-1.0).factor(), 0.0);
        assert_eq!(MusicVolume(2.5).factor(), 1.0);
    }
}
