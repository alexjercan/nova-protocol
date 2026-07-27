//! App runtime: the [`NovaOsAppRuntime`] trait apps implement, the registry
//! that holds them, the launch-word command mirror, and the per-surface
//! footer hints.

use bevy::{input::keyboard::Key, prelude::*};

use crate::{shell::CommandArity, terminal::TerminalMode};

/// The terminal-surface footer hints (PoC `HINTS.terminal`). Apps override
/// [`NovaOsAppRuntime::hints`] to swap these for their own set while active.
pub const NOVA_OS_TERMINAL_HINTS: [&str; 3] = [
    "TAB: AUTOCOMPLETE",
    "ESC: CLOSE COMPUTER",
    "HINT: TYPE HELP",
];

/// A launchable app as the terminal sees it: the launch word plus a one-line
/// summary for `help`/autocomplete. Mirrored from the [`NovaOsAppRegistry`] into
/// [`crate::terminal::NovaOsTerminal::app_commands`] so command parsing, completion and `help`
/// treat app launch words as first-class commands without reaching into the
/// registry from every terminal method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NovaOsAppCommand {
    /// The launch word typed at the prompt (also the app's id).
    pub id: &'static str,
    /// One-line summary for `help` and autocomplete.
    pub summary: &'static str,
    /// How many argument words the launch word accepts. All current apps are
    /// [`CommandArity::None`]; the parser supports arguments so an app task can
    /// register an argument-taking launch word without reworking parsing.
    pub arity: CommandArity,
}

/// A NOVA OS app: a full-screen tool launched from the terminal that swallows the
/// terminal surface and owns input until the user exits back to the prompt.
///
/// This is the app-as-plugin seam (see `tasks/20260726-115334/DECISION.md`): each
/// app is its own runtime object registered into [`NovaOsAppRegistry`]. The NOVA OS
/// owns the generic parts - the [`TerminalMode::App`] transition, input ownership,
/// the chrome (title bar + close control) and the uniform exit (Escape / close
/// control). An app only supplies its identity, its body UI, and its own key
/// handling; the real `map`/`ship viewer` apps register their own runtime and
/// spawn arbitrary UI into the body slot without editing this module.
pub trait NovaOsAppRuntime: Send + Sync + 'static {
    /// Stable id; also the launch word typed at the prompt (e.g. `map`).
    fn id(&self) -> &'static str;
    /// Title shown in the app's chrome bar.
    fn title(&self) -> &'static str;
    /// One-line summary for `help` and the completion hint.
    fn summary(&self) -> &'static str;
    /// Spawn the app's body under `body` (the chrome is spawned by the runtime).
    /// `font` is the shared NOVA OS terminal font.
    fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>);
    /// React to a key press while the app owns input. The runtime handles the
    /// universal exit (Escape / close control) itself, so this is for the app's
    /// own keys. Default: swallow the key and stay open (input is owned even when
    /// the app does nothing with it).
    fn handle_key(&self, key: &Key) -> NovaOsAppInputOutcome {
        let _ = key;
        NovaOsAppInputOutcome::Continue
    }
    /// How many argument words this app's launch word accepts. Default: none, so
    /// `map foo` is rejected the same way a built-in with arguments is.
    fn arity(&self) -> CommandArity {
        CommandArity::None
    }
    /// The footer hints shown while this app owns the screen (PoC `HINTS` map).
    /// Default: the terminal hint set, so an app that does not care still shows a
    /// sensible footer.
    fn hints(&self) -> [&'static str; 3] {
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

/// The set of registered NOVA OS apps. Apps register at plugin build; the
/// terminal mirrors their launch words into [`crate::terminal::NovaOsTerminal::app_commands`] and
/// looks a runtime up by id when spawning/handling the active app.
#[derive(Resource, Default)]
pub struct NovaOsAppRegistry {
    apps: Vec<Box<dyn NovaOsAppRuntime>>,
}

impl NovaOsAppRegistry {
    /// The registration seam future apps plug into (the `map`/`ship viewer` tasks
    /// and the lifecycle tests). No production app registers yet - this task ships
    /// the runtime, not an app - so it is unused outside `#[cfg(test)]`.
    pub fn register(&mut self, app: impl NovaOsAppRuntime) {
        self.apps.push(Box::new(app));
    }

    /// The registered runtime with launch word `id`, if any.
    pub fn get(&self, id: &str) -> Option<&dyn NovaOsAppRuntime> {
        self.apps.iter().map(Box::as_ref).find(|app| app.id() == id)
    }

    /// The registered apps as terminal launch-word commands, for mirroring into
    /// [`crate::terminal::NovaOsTerminal`].
    pub fn commands(&self) -> Vec<NovaOsAppCommand> {
        self.apps
            .iter()
            .map(|app| NovaOsAppCommand {
                id: app.id(),
                summary: app.summary(),
                arity: app.arity(),
            })
            .collect()
    }

    /// The number of registered apps.
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    /// Whether no apps are registered.
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    /// The registered apps' ids (launch words), in registration order.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.apps.iter().map(|app| app.id())
    }
}

/// The footer hint set for the active surface (PoC `HINTS` map): the terminal set
/// at the prompt, or the running app's own [`NovaOsAppRuntime::hints`] while an
/// app owns the screen.
pub fn nova_os_footer_hints(mode: TerminalMode, registry: &NovaOsAppRegistry) -> [&'static str; 3] {
    match mode {
        TerminalMode::Prompt => NOVA_OS_TERMINAL_HINTS,
        TerminalMode::App { id } => registry
            .get(id)
            .map(|app| app.hints())
            .unwrap_or(NOVA_OS_TERMINAL_HINTS),
    }
}
