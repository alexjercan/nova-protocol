//! The screenshot driver's stand-down path, in its own test BINARY.
//!
//! The decision is read from process-global env vars in `Plugin::build`, and
//! arming `NOVA_AUTOPILOT` inside the lib-test binary would make every other
//! screenshot test stand down too. One test per process means no race.

use bevy::{prelude::*, render::view::screenshot::Screenshot, state::app::StatesPlugin};
use nova_autopilot::{
    autopilot::AUTOPILOT_ENV,
    completion::HarnessCompletion,
    screenshot::{ScreenshotPlugin, SCREENSHOT_ENV},
};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum TestState {
    #[default]
    Boot,
    Playing,
}

#[test]
fn screenshot_stands_down_when_the_autopilot_is_armed() {
    std::env::set_var(SCREENSHOT_ENV, "390x844");
    std::env::set_var(AUTOPILOT_ENV, "1");

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.init_state::<TestState>();
    app.add_plugins(ScreenshotPlugin::new(TestState::Playing));

    for _ in 0..8 {
        app.update();
    }

    assert!(
        app.world().get_resource::<HarnessCompletion>().is_none(),
        "standing down means registering NOTHING: a registered collector that \
         never reports done would hold the autopilot's exit open forever"
    );
    let mut captures = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
    assert_eq!(
        captures.iter(app.world()).count(),
        0,
        "and capturing nothing"
    );
    assert_eq!(
        app.world().resource::<State<TestState>>().get(),
        &TestState::Boot,
        "and never touching NextState, which is what the autopilot drives"
    );
}
