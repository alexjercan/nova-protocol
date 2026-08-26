//! App runtime: the [`NovaOsAppRuntime`] trait apps implement and the terminal
//! footer hints. Apps are held by the unified command registry
//! ([`crate::command::NovaOsCommandRegistry`]) as the body of a
//! [`crate::command::TerminalCommand`]; this module owns only the runtime seam
//! itself.

use bevy::{input::keyboard::Key, prelude::*};

/// The terminal-surface footer hints: the full set of keys that work at the
/// prompt (owner playtest - the footer should list every current binding, not
/// just three). Kept ASCII (no arrow glyphs) and terse so the row fits. Apps
/// override [`NovaOsAppRuntime::hints`] to swap these for their own set while
/// active. A slice, not a fixed array, so the terminal and each app can list a
/// different number of keys.
pub const NOVA_OS_TERMINAL_HINTS: &[&str] = &[
    "TAB: COMPLETE",
    "ENTER: RUN",
    "UP/DN: HISTORY",
    "PGUP/PGDN: SCROLL",
    // Only Escape closes the computer AT THE PROMPT; Ctrl+C / Ctrl+[ is
    // an app-exit chord (a no-op here), so it belongs on app hint sets, not this
    // one - do not advertise an unwired key on this surface.
    "ESC: CLOSE",
    "TYPE HELP",
];

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
    fn hints(&self) -> &'static [&'static str] {
        NOVA_OS_TERMINAL_HINTS
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
    pub use super::{NovaOsAppInputOutcome, NovaOsAppRuntime, NOVA_OS_TERMINAL_HINTS};
}
