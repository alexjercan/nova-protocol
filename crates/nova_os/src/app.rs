//! App runtime: the [`NovaOsAppRuntime`] trait apps implement and the terminal
//! footer hints. Apps are held by the unified command registry
//! ([`crate::command::NovaOsCommandRegistry`]) as the body of a
//! [`crate::command::TerminalCommand`]; this module owns only the runtime seam
//! itself.

use bevy::{input::keyboard::Key, prelude::*};
use nova_input::prelude::{InputBindings, InputSource};

/// The terminal-surface footer hints that name a FIXED key: the shell's own
/// editing keys, which are part of the terminal's grammar rather than player
/// bindings. Kept ASCII (no arrow glyphs) and terse so the row fits.
///
/// The rebindable half is prepended by [`terminal_hints`]. Apps override
/// [`NovaOsAppRuntime::hints`] to swap the whole set for their own while active.
pub const NOVA_OS_TERMINAL_HINTS: &[&str] = &[
    "ENTER: RUN",
    "UP/DN: HISTORY",
    "PGUP/PGDN: SCROLL",
    // Only Escape closes the computer AT THE PROMPT; Ctrl+C / Ctrl+[ is
    // an app-exit chord (a no-op here), so it belongs on app hint sets, not this
    // one - do not advertise an unwired key on this surface.
    "ESC: CLOSE",
    "TYPE HELP",
];

/// The Command-shell footer hints. Same editing grammar as the NOVA OS prompt,
/// but Escape means BACK: it climbs to the shell this one was entered from, and
/// only closes the computer when there is nothing underneath.
pub const COMMAND_SHELL_HINTS: &[&str] = &[
    "ENTER: RUN",
    "UP/DN: HISTORY",
    "PGUP/PGDN: SCROLL",
    "ESC: BACK",
    "TYPE HELP",
];

/// The Command-shell footer, with the rebindable completion key resolved
/// against the live table. The completion key is the emulator's, not a shell's,
/// so both prompts advertise the same one.
pub fn command_shell_hints(bindings: &InputBindings) -> Vec<String> {
    let mut hints = vec![format!("{}: COMPLETE", completion_key_label(bindings))];
    hints.extend(COMMAND_SHELL_HINTS.iter().map(|hint| (*hint).to_string()));
    hints
}

/// The label of whatever key currently completes at the prompt.
fn completion_key_label(bindings: &InputBindings) -> String {
    bindings
        .get("novaos_toggle")
        .and_then(|action| action.keyboard.first())
        .map(InputSource::readout_label)
        .unwrap_or_else(|| "Tab".to_string())
        .to_uppercase()
}

/// The prompt-surface footer, with the rebindable completion key resolved
/// against the live table.
///
/// The monitor's own key doubles as autocomplete while the shell has the
/// screen, so a player who moved `novaos_toggle` moved this too.
pub fn terminal_hints(bindings: &InputBindings) -> Vec<String> {
    let mut hints = vec![format!("{}: COMPLETE", completion_key_label(bindings))];
    hints.extend(
        NOVA_OS_TERMINAL_HINTS
            .iter()
            .map(|hint| (*hint).to_string()),
    );
    hints
}

/// A NOVA OS app: a full-screen tool launched from the terminal that swallows the
/// terminal surface and owns input until the user exits back to the prompt.
///
/// This is the app-as-plugin seam: each app is its own runtime object, held as
/// the [`crate::command::CommandBody::App`]
/// body of a [`crate::command::TerminalCommand`] registered into the
/// [`crate::command::NovaOsCommandRegistry`]. The NOVA OS owns the generic parts -
/// the [`crate::terminal::TerminalMode::App`] transition, input ownership, the
/// persistent header (its breadcrumb + close control) and footer, and the uniform
/// exit (Escape / close control). An app only supplies its identity, its body UI,
/// and its own key handling; the `map`/`ship viewer` apps register their own
/// runtime and spawn arbitrary UI into the body slot without editing this module.
pub trait NovaOsAppRuntime: Send + Sync + 'static {
    /// Stable id; also the launch word typed at the prompt (e.g. `map`). Matches
    /// the name of the [`crate::command::TerminalCommand`] whose body owns it. The
    /// header breadcrumb shows this id upper-cased (`APPS / MAP`).
    fn id(&self) -> &'static str;
    /// Human-readable title for the app. Informational only: the shared header
    /// shows the launch word (`id`) in its breadcrumb, not this string (which may
    /// carry a `/`, e.g. the map's "MAP / LOCAL SPACE"). Defaults to the `id` so
    /// apps need not supply one; an app may still override it for a friendlier
    /// label used by future/debug surfaces.
    fn title(&self) -> &'static str {
        self.id()
    }
    /// Spawn the app's body under `body`; it absolute-fills the shared `<main>`
    /// region. `font` is the shared NOVA OS terminal font.
    fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>);
    /// React to a key press while the app owns input. The runtime handles the
    /// universal exit (Escape / close control) itself, so this is for the app's
    /// own keys. Default: swallow the key and stay open (input is owned even when
    /// the app does nothing with it).
    fn handle_key(&self, key: &Key) -> NovaOsAppInputOutcome {
        let _ = key;
        NovaOsAppInputOutcome::Continue
    }
    /// The footer hints shown while this app owns the screen (PoC `HINTS` map).
    /// Default: the terminal hint set, so an app that does not care still shows a
    /// sensible footer.
    ///
    /// Built from the live bindings rather than returned as fixed text: the
    /// app keys are rebindable, and a footer that printed the shipped default
    /// would tell a player who moved one the wrong key.
    fn hints(&self, bindings: &InputBindings) -> Vec<String> {
        terminal_hints(bindings)
    }
}

/// What an app wants after handling one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NovaOsAppInputOutcome {
    /// Stay open (the key was consumed by the app or ignored).
    Continue,
    /// Exit back to the terminal (the app requested its own close).
    Exit,
}

/// The `NovaOsAppRuntime` seam, its `NovaOsAppInputOutcome`, and
/// `NOVA_OS_TERMINAL_HINTS`.
pub mod prelude {
    pub use super::{
        command_shell_hints, terminal_hints, NovaOsAppInputOutcome, NovaOsAppRuntime,
        COMMAND_SHELL_HINTS, NOVA_OS_TERMINAL_HINTS,
    };
}
