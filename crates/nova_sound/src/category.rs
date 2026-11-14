//! The balance table: every game sound belongs to one [`SoundCategoryType`],
//! and the category alone decides how loud it is, how far it carries, how
//! often it may repeat and how many copies may sound at once.
//!
//! This table is the ONLY balance knob. Game code names a category and never a
//! gain. Every clip is first brought to [`LOUDNESS_TARGET_LUFS`] by its
//! measured [`SoundLoudness`](super::SoundLoudness) gain, so a level here is a
//! loudness relative to that target, not a correction for how hot the file was
//! recorded.
//!
//! [`LOUDNESS_TARGET_LUFS`]: super::LOUDNESS_TARGET_LUFS

use bevy::prelude::*;
use nova_events::prelude::Meters;

/// The two mixer tracks. A track's slider is the only per-track control.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
pub enum SoundBus {
    /// Sounds that happen in the world: guns, engines, doors, explosions.
    /// Always positional: faded by distance from the listener and panned by
    /// bearing. Scaled by the Effects slider (`WorldVolume`).
    Effect,
    /// Interface and cockpit feedback. Never positional. Scaled by the
    /// Interface slider (`InterfaceVolume`).
    Ui,
}

/// What kind of sound a cue is. Closed: a new kind is a new row in
/// [`SoundCategoryType::spec`], never a number at a call site. Content names
/// one by its variant in a scenario's `PlaySound` action.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SoundCategoryType {
    /// A whole ship coming apart: the loudest sound in the game.
    Collapse,
    /// A section, a rock or a warhead blowing up.
    Explosion,
    /// The railgun report.
    HeavyWeapon,
    /// A torpedo leaving its tube.
    Launch,
    /// A torpedo drive lighting.
    Ignition,
    /// A point-defense gun firing.
    Weapon,
    /// A round hitting plate or stone.
    Impact,
    /// A railgun slug hitting.
    HeavyImpact,
    /// A torpedo dying without its warhead: a dud or an intercept.
    TorpedoDeath,
    /// A weapon reloading.
    Reload,
    /// Doors, stows, clamps and ejectors: quiet ship machinery.
    Machinery,
    /// The intake taking a canister.
    Pickup,
    /// A mining beam pulse, landed or refused.
    Mining,
    /// A gun mount clicking on an empty magazine.
    DryFire,
    /// A main drive loop. Loop: intensity is the throttle.
    Drive,
    /// An RCS hiss loop. Loop: intensity is the thruster effort.
    Rcs,
    /// A railgun capacitor whine loop. Loop: intensity is the charge.
    Charge,
    /// A cockpit alarm: incoming lock, hull warning.
    Alarm,
    /// A cockpit tone: lock gained or lost, safety, kill confirm, radar deny.
    Cockpit,
    /// A small cockpit tick: retarget, torpedo closing, ammo dry.
    CockpitTick,
    /// A button press, a panel opening or closing, an accepted action.
    UiClick,
    /// The smallest interface cue: the pointer arriving on a button, a slider
    /// detent, a keystroke, a rotation step. Fired in streams, so it is the
    /// quietest row and repeats at most every 30 ms.
    UiTick,
    /// A command or an operation that succeeded, and the terminal's power
    /// sweeps.
    UiConfirm,
    /// A refused press, a failed command: the deny buzz.
    UiError,
    /// A line posted to the HUD notice slot.
    Notice,
    /// An objective posted, completed or failed.
    Objective,
    /// A comms line showing on the panel.
    Comms,
    /// A part seated in or lifted out of the editor grid.
    Editor,
    /// A creator listening to a sound file in the editor.
    Preview,
    /// The terminal's ambient bed. Loop: intensity is on or off.
    TerminalBed,
}

/// One row of the balance table.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SoundCategory {
    /// The track that scales it, and whether it is positional.
    pub bus: SoundBus,
    /// The level relative to the loudness target, in dB.
    pub level_db: f32,
    /// Inside this distance an [`SoundBus::Effect`] sound plays at its full
    /// level; beyond it the level falls as `reference / distance`.
    pub reference: Meters,
    /// The distance at which an [`SoundBus::Effect`] sound has faded to
    /// silence.
    pub max: Meters,
    /// How many copies of one sound may play at once from one root entity.
    /// The oldest one-shot is stopped for a new one; a loop over the count is
    /// held silent.
    pub max_voices: u8,
    /// The least time between two one-shots of one sound from one root
    /// entity, in seconds. A one-shot inside it is dropped.
    pub repeat_interval: f32,
    /// Which loops keep sounding when the loop cap is full: a higher priority
    /// wins before level is compared.
    pub priority: u8,
    /// For a loop category, the `(at zero, at full)` dB offsets that intensity
    /// maps between. `None` for a one-shot category.
    pub loop_db: Option<(f32, f32)>,
}

/// The share of [`SoundCategory::max`] over which the fade to silence runs.
const FADE_OUT_SHARE: f32 = 0.25;

impl SoundCategoryType {
    /// Every category, for the table checks.
    pub const ALL: [Self; 30] = [
        Self::Collapse,
        Self::Explosion,
        Self::HeavyWeapon,
        Self::Launch,
        Self::Ignition,
        Self::Weapon,
        Self::Impact,
        Self::HeavyImpact,
        Self::TorpedoDeath,
        Self::Reload,
        Self::Machinery,
        Self::Pickup,
        Self::Mining,
        Self::DryFire,
        Self::Drive,
        Self::Rcs,
        Self::Charge,
        Self::Alarm,
        Self::Cockpit,
        Self::CockpitTick,
        Self::UiClick,
        Self::UiTick,
        Self::UiConfirm,
        Self::UiError,
        Self::Notice,
        Self::Objective,
        Self::Comms,
        Self::Editor,
        Self::Preview,
        Self::TerminalBed,
    ];

    /// The balance table.
    ///
    /// Realism sets the order: quiet machinery sits well under the guns, the
    /// guns sit under the explosions and the railgun, and a ship collapsing is
    /// the loudest thing there is. The levels started from the mix the game
    /// had before equalization, then moved the machinery down past the guns.
    ///
    /// The interface sits under the loud half of the world: a click is heard
    /// at about the level of a nearby gun, never over a launch or the drive.
    /// Objective cues are the loudest interface rows because they must read
    /// in a fight; the tick is the quietest because it fires in streams.
    pub const fn spec(self) -> SoundCategory {
        use SoundBus::{Effect, Ui};
        //                      bus     dB     ref m   max m   voices repeat prio loop dB
        match self {
            Self::Collapse => row(Effect, -1.0, 600.0, 10_000.0, 2, 0.10, 0, None),
            Self::Explosion => row(Effect, -3.0, 400.0, 8_000.0, 4, 0.06, 0, None),
            Self::HeavyWeapon => row(Effect, -1.0, 400.0, 8_000.0, 2, 0.0, 0, None),
            Self::Launch => row(Effect, -5.0, 250.0, 5_000.0, 2, 0.10, 0, None),
            Self::Ignition => row(Effect, -8.0, 250.0, 5_000.0, 2, 0.10, 0, None),
            Self::Weapon => row(Effect, -14.0, 200.0, 3_000.0, 6, 0.05, 0, None),
            Self::Impact => row(Effect, -13.0, 150.0, 2_500.0, 4, 0.04, 0, None),
            Self::HeavyImpact => row(Effect, -8.0, 250.0, 5_000.0, 2, 0.0, 0, None),
            Self::TorpedoDeath => row(Effect, -10.0, 200.0, 3_000.0, 2, 0.08, 0, None),
            Self::Reload => row(Effect, -16.0, 40.0, 800.0, 1, 0.0, 0, None),
            Self::Machinery => row(Effect, -22.0, 30.0, 600.0, 1, 0.15, 0, None),
            Self::Pickup => row(Effect, -16.0, 30.0, 600.0, 1, 0.0, 0, None),
            Self::Mining => row(Effect, -14.0, 60.0, 1_500.0, 1, 0.0, 0, None),
            Self::DryFire => row(Effect, -20.0, 20.0, 300.0, 1, 0.0, 0, None),
            Self::Drive => row(Effect, -2.0, 250.0, 6_000.0, 1, 0.0, 2, Some((-18.0, 0.0))),
            Self::Rcs => row(Effect, -12.0, 60.0, 1_500.0, 1, 0.0, 1, Some((-12.0, 0.0))),
            Self::Charge => row(Effect, -6.0, 100.0, 3_000.0, 1, 0.0, 3, Some((-9.0, 0.0))),
            Self::Alarm => row(Ui, -5.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::Cockpit => row(Ui, -6.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::CockpitTick => row(Ui, -12.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::UiClick => row(Ui, -8.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::UiTick => row(Ui, -15.0, 0.0, 0.0, 1, 0.03, 0, None),
            Self::UiConfirm => row(Ui, -9.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::UiError => row(Ui, -9.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::Notice => row(Ui, -8.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::Objective => row(Ui, -5.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::Comms => row(Ui, -11.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::Editor => row(Ui, -9.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::Preview => row(Ui, -6.0, 0.0, 0.0, 1, 0.0, 0, None),
            Self::TerminalBed => row(Ui, -24.0, 0.0, 0.0, 1, 0.0, 0, Some((0.0, 0.0))),
        }
    }

    /// The track this category plays on.
    pub const fn bus(self) -> SoundBus {
        self.spec().bus
    }

    /// Whether this is a loop category.
    pub const fn is_loop(self) -> bool {
        self.spec().loop_db.is_some()
    }
}

/// One table row. A function and not a struct literal so the table reads as
/// one aligned line per category.
#[expect(
    clippy::too_many_arguments,
    reason = "one argument per table column, named by the column header"
)]
const fn row(
    bus: SoundBus,
    level_db: f32,
    reference: f32,
    max: f32,
    max_voices: u8,
    repeat_interval: f32,
    priority: u8,
    loop_db: Option<(f32, f32)>,
) -> SoundCategory {
    SoundCategory {
        bus,
        level_db,
        reference: Meters(reference),
        max: Meters(max),
        max_voices,
        repeat_interval,
        priority,
        loop_db,
    }
}

impl SoundCategory {
    /// [`Self::level_db`] as a linear amplitude factor.
    pub fn level(self) -> f32 {
        db_to_gain(self.level_db)
    }

    /// The linear factor a loop at `intensity` plays at, on top of
    /// [`Self::level`]. Zero intensity is silent; above it the dB offset runs
    /// linearly across [`Self::loop_db`]. A one-shot category returns `1.0`.
    pub fn intensity_gain(self, intensity: f32) -> f32 {
        let Some((low, high)) = self.loop_db else {
            return 1.0;
        };
        let intensity = intensity.clamp(0.0, 1.0);
        if intensity <= 0.0 {
            return 0.0;
        }
        db_to_gain(low + (high - low) * intensity)
    }

    /// The distance fade for a sound `distance` from the listener.
    ///
    /// Full level inside [`Self::reference`]; beyond it inverse distance,
    /// `reference / distance`; then a smoothstep to zero over the last
    /// quarter before [`Self::max`]. Silent at and past `max`.
    pub fn distance_fade(self, distance: Meters) -> f32 {
        let distance = distance.get().max(0.0);
        let max = self.max.get();
        if distance >= max {
            return 0.0;
        }
        let reference = self.reference.get().max(f32::MIN_POSITIVE);
        let inverse = if distance <= reference {
            1.0
        } else {
            reference / distance
        };
        let fade_start = max * (1.0 - FADE_OUT_SHARE);
        if distance <= fade_start {
            return inverse;
        }
        let t = (distance - fade_start) / (max - fade_start);
        inverse * (1.0 - t * t * (3.0 - 2.0 * t))
    }
}

/// A dB value as a linear amplitude factor.
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn an_effect_sound_plays_full_inside_its_reference_distance() {
        let weapon = SoundCategoryType::Weapon.spec();
        assert_eq!(weapon.distance_fade(Meters(0.0)), 1.0);
        assert_eq!(weapon.distance_fade(weapon.reference), 1.0);
    }

    #[test]
    fn an_effect_sound_falls_as_inverse_distance_past_its_reference() {
        let weapon = SoundCategoryType::Weapon.spec();
        let reference = weapon.reference.get();
        assert!(close(weapon.distance_fade(Meters(reference * 2.0)), 0.5));
        assert!(close(weapon.distance_fade(Meters(reference * 4.0)), 0.25));
    }

    #[test]
    fn an_effect_sound_fades_smoothly_to_silence_at_its_max_distance() {
        let weapon = SoundCategoryType::Weapon.spec();
        let max = weapon.max.get();
        assert_eq!(weapon.distance_fade(weapon.max), 0.0);
        assert_eq!(weapon.distance_fade(Meters(max * 2.0)), 0.0);
        let mut last = f32::MAX;
        for step in 0..=100 {
            let distance = Meters(max * step as f32 / 100.0);
            let fade = weapon.distance_fade(distance);
            assert!(fade <= last, "the fade never rises with distance");
            last = fade;
        }
        let near_max = weapon.distance_fade(Meters(max * 0.99));
        assert!(
            near_max > 0.0 && near_max < 0.01 * weapon.distance_fade(Meters(max * 0.75)),
            "the last quarter eases down to zero, not a cliff: {near_max}"
        );
    }

    #[test]
    fn loop_intensity_maps_across_the_category_db_range() {
        let drive = SoundCategoryType::Drive.spec();
        let (low, high) = drive.loop_db.expect("Drive is a loop");
        assert_eq!(drive.intensity_gain(0.0), 0.0, "zero intensity is silent");
        assert!(close(drive.intensity_gain(1.0), db_to_gain(high)));
        assert!(close(
            drive.intensity_gain(0.5),
            db_to_gain((low + high) / 2.0)
        ));
        assert!(close(
            drive.intensity_gain(f32::MIN_POSITIVE),
            db_to_gain(low)
        ));
        assert!(
            close(drive.intensity_gain(4.0), db_to_gain(high)),
            "intensity is clamped to 1"
        );
    }

    #[test]
    fn every_effect_category_carries_a_reach_and_every_ui_category_none() {
        for category in SoundCategoryType::ALL {
            let spec = category.spec();
            match spec.bus {
                SoundBus::Effect => assert!(
                    spec.reference.get() > 0.0 && spec.max.get() > spec.reference.get(),
                    "{category:?} needs a reference inside its max"
                ),
                SoundBus::Ui => assert!(spec.max.get() == 0.0, "{category:?} is flat feedback"),
            }
            assert!(spec.max_voices > 0, "{category:?} must be able to sound");
        }
    }
}
