//! Test fixtures shared by the terminal test modules.

use crate::terminal::CommandTerminal;

pub(super) fn type_text(terminal: &mut CommandTerminal, text: &str) {
    terminal.insert_text(text);
}
