//! Naming what `ExitStatus::code()` cannot.
//!
//! A process killed by a signal has no exit code, so `code()` is `None` and an
//! assertion that formats it says only "exited with None" - no signal, no core
//! dump, no hint that the process was killed rather than exiting badly. This
//! module names the signal instead, so a failure message diagnoses a signal
//! death on its own.

use std::process::ExitStatus;

/// Describe how a process ended, in a phrase that completes "the example ...".
///
/// On unix a signal death names the signal (`was killed by SIGSEGV`), flags a
/// core dump, and points SIGKILL at the OOM killer. Everything else reports the
/// exit code.
///
/// ```
/// # #[cfg(unix)]
/// # {
/// use std::os::unix::process::ExitStatusExt;
/// use std::process::ExitStatus;
///
/// let status = ExitStatus::from_raw(11);
/// assert_eq!(nova_autopilot::exit::describe(&status), "was killed by SIGSEGV");
/// # }
/// ```
pub fn describe(status: &ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        if let Some(signal) = status.signal() {
            let name = match signal {
                4 => "SIGILL".to_string(),
                6 => "SIGABRT".to_string(),
                7 => "SIGBUS".to_string(),
                9 => "SIGKILL".to_string(),
                11 => "SIGSEGV".to_string(),
                15 => "SIGTERM".to_string(),
                other => format!("signal {other}"),
            };
            let mut message = format!("was killed by {name}");
            if status.core_dumped() {
                message.push_str(" (core dumped)");
            }
            if signal == 9 {
                message.push_str(" - likely the OOM killer");
            }
            return message;
        }
    }

    match status.code() {
        Some(code) => format!("exited with code {code}"),
        None => "exited with no code".to_string(),
    }
}

// Every case here is built from a raw unix wait status, so the module as a
// whole is unix-only rather than each test.
#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::process::ExitStatusExt;

    use super::*;

    #[test]
    fn names_a_segfault() {
        assert_eq!(describe(&ExitStatus::from_raw(11)), "was killed by SIGSEGV");
    }

    #[test]
    fn flags_a_core_dump() {
        assert_eq!(
            describe(&ExitStatus::from_raw(139)),
            "was killed by SIGSEGV (core dumped)"
        );
    }

    #[test]
    fn points_sigkill_at_the_oom_killer() {
        assert_eq!(
            describe(&ExitStatus::from_raw(9)),
            "was killed by SIGKILL - likely the OOM killer"
        );
    }

    #[test]
    fn degrades_an_unnamed_signal_to_its_number() {
        assert_eq!(describe(&ExitStatus::from_raw(5)), "was killed by signal 5");
    }

    #[test]
    fn reports_a_plain_exit_code() {
        assert_eq!(
            describe(&ExitStatus::from_raw(0x1F00)),
            "exited with code 31"
        );
    }
}
