//! The env var names and completion keys are a contract with callers, not an
//! implementation detail: the crate docs, the wiki page and every caller's run
//! script spell them out, so renaming one is a break rather than a refactor.

use nova_autopilot::prelude::*;

#[test]
fn the_env_contract_names_are_the_documented_ones() {
    assert_eq!(AUTOPILOT_ENV, "NOVA_AUTOPILOT");
    assert_eq!(CAPTURE_ENV, "NOVA_CAPTURE");
    assert_eq!(CAPTURE_DIR_ENV, "NOVA_CAPTURE_DIR");
    assert_eq!(DEADLINE_ENV, "NOVA_AUTOPILOT_DEADLINE");
    assert_eq!((AUTOPILOT, LOOP_CAPTURE), ("autopilot", "loop_capture"));
}
