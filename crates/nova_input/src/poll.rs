//! Read a named action straight off the button state, for the toggles that are
//! not part of any rig.
//!
//! Most actions reach the game through `bevy_enhanced_input`: a rig entity
//! carries them, the rig spawns with the player ship, and conditions and
//! modifiers run. The MODE toggles cannot work that way. Opening NOVA OS or
//! dropping the HUD to cinematic must answer with no flight rig in the world at
//! all, and they carry no condition worth a rig - one press, one flip.
//!
//! So they stay plain polling systems, and this is what makes them rebindable
//! anyway: the system asks the registry which sources the action holds instead
//! of naming a [`KeyCode`] itself.
//!
//! # The gamepad is a component, not a resource
//!
//! Bevy 0.19 keeps digital button state on the [`Gamepad`] COMPONENT
//! ([`Gamepad::digital`]); it registers no `ButtonInput<GamepadButton>`
//! resource, and never has in this version. A system that asked for one as
//! `Option<Res<..>>` therefore got `None` on every real run - silently, because
//! the option made it look deliberate - and only ever saw `Some` in a test that
//! inserted the resource itself.
//!
//! [`InputSources`] reads the component, so a pad answers here whether or not
//! anything else in the tree ever did.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{registry::ActionBinding, source::InputSource};

/// Every button surface an action can be bound to, as one system parameter.
#[derive(SystemParam)]
pub struct InputSources<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
}

impl InputSources<'_, '_> {
    /// Whether any source `action` holds went down this frame.
    pub fn just_pressed(&self, action: &ActionBinding) -> bool {
        action.sources().any(|source| self.edge(source))
    }

    /// Whether any source `action` holds is down.
    pub fn pressed(&self, action: &ActionBinding) -> bool {
        action.sources().any(|source| self.level(source))
    }

    /// Whether any KEYBOARD OR MOUSE source went down this frame, ignoring the
    /// pad.
    ///
    /// The NOVA OS toggle needs the two apart: Tab opens the computer and
    /// Escape closes it, while the pad button does both, because a pad has no
    /// Escape to reach for.
    pub fn just_pressed_desk(&self, action: &ActionBinding) -> bool {
        action
            .sources()
            .filter(|source| !matches!(source, InputSource::Gamepad(_)))
            .any(|source| self.edge(source))
    }

    /// Whether any GAMEPAD source went down this frame.
    pub fn just_pressed_pad(&self, action: &ActionBinding) -> bool {
        action
            .sources()
            .filter(|source| matches!(source, InputSource::Gamepad(_)))
            .any(|source| self.edge(source))
    }

    fn edge(&self, source: InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keys.just_pressed(key),
            InputSource::Mouse(button) => self.mouse.just_pressed(button),
            InputSource::Gamepad(button) => self
                .gamepads
                .iter()
                .any(|pad| pad.digital().just_pressed(button)),
        }
    }

    fn level(&self, source: InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keys.pressed(key),
            InputSource::Mouse(button) => self.mouse.pressed(button),
            InputSource::Gamepad(button) => self
                .gamepads
                .iter()
                .any(|pad| pad.digital().pressed(button)),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::{
        poll::InputSources,
        prelude::{ActionBinding, InputBindings, InputSource},
    };

    #[derive(Resource, Default)]
    struct Seen {
        any: bool,
        desk: bool,
        pad: bool,
        held: bool,
    }

    fn watch(bindings: Res<InputBindings>, sources: InputSources, mut seen: ResMut<Seen>) {
        let action = bindings.get("novaos_toggle").expect("registered");
        seen.any |= sources.just_pressed(action);
        seen.desk |= sources.just_pressed_desk(action);
        seen.pad |= sources.just_pressed_pad(action);
        seen.held |= sources.pressed(action);
    }

    /// No `InputPlugin`: it clears the `just_*` edges in `PreUpdate`, which
    /// would wipe a press staged before `update()` before the watcher ever ran.
    /// The resources are the same ones it would have inserted.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<Seen>();
        app.insert_resource(InputBindings::from_actions([ActionBinding::new(
            "novaos_toggle",
            "SYSTEM",
            "NOVA OS",
        )
        .keyboard([InputSource::Keyboard(KeyCode::Tab)])
        .gamepad([InputSource::Gamepad(GamepadButton::RightThumb)])]));
        app.add_systems(Update, watch);
        app
    }

    /// A connected pad, the way bevy models one: a component, not a resource.
    fn connect_pad(app: &mut App, button: GamepadButton) {
        let mut pad = Gamepad::default();
        pad.digital_mut().press(button);
        app.world_mut().spawn(pad);
    }

    #[test]
    fn a_bound_key_answers_and_an_unbound_one_does_not() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyJ);
        app.update();
        assert!(!app.world().resource::<Seen>().any);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        let seen = app.world().resource::<Seen>();
        assert!(seen.any, "the bound key answers");
        assert!(seen.desk, "and it answers as a desk source");
        assert!(!seen.pad, "a key is not the pad");
        assert!(seen.held, "a press is also a hold");
    }

    /// The regression that motivated reading the component: a pad button must
    /// answer with no `ButtonInput<GamepadButton>` resource in the world,
    /// because bevy 0.19 never puts one there.
    #[test]
    fn a_pad_button_answers_off_the_gamepad_component() {
        let mut app = app();
        connect_pad(&mut app, GamepadButton::RightThumb);
        app.update();
        assert!(
            app.world()
                .get_resource::<ButtonInput<GamepadButton>>()
                .is_none(),
            "guard: there is no such resource, which is the whole point"
        );
        assert!(app.world().resource::<Seen>().any, "the pad answers anyway");
    }

    /// The pad half has to be separable: the NOVA OS toggle closes on the pad
    /// button and not on Tab, because a pad has no Escape.
    #[test]
    fn the_pad_half_is_separable_from_the_desk_half() {
        let mut app = app();
        connect_pad(&mut app, GamepadButton::RightThumb);
        app.update();
        let seen = app.world().resource::<Seen>();
        assert!(seen.pad, "the pad source answers");
        assert!(!seen.desk, "and it is not counted as a desk source");
    }

    /// An unbound pad button is not the bound one, and a disconnected pad is
    /// simply no rows to iterate.
    #[test]
    fn an_unbound_pad_button_stays_quiet() {
        let mut app = app();
        connect_pad(&mut app, GamepadButton::South);
        app.update();
        assert!(!app.world().resource::<Seen>().any);
    }

    #[test]
    fn no_pad_at_all_is_simply_no_rows_to_iterate() {
        let mut app = app();
        app.update();
        assert!(!app.world().resource::<Seen>().any);
    }
}
