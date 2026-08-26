//! The scroll unit contract and the every-frame clamp.

use bevy::{
    ecs::system::RunSystemOnce,
    input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    },
    prelude::*,
};

use super::*;
use crate::prelude::UiSkin;

/// A viewport 300 physical px tall holding 500 physical px of content, at
/// `scale` device pixels per logical pixel.
fn viewport(scale: f32) -> ComputedNode {
    ComputedNode {
        size: Vec2::new(200.0, 300.0),
        content_size: Vec2::new(200.0, 500.0),
        inverse_scale_factor: 1.0 / scale,
        ..ComputedNode::DEFAULT
    }
}

#[test]
fn max_scroll_y_is_logical_pixels_at_every_scale() {
    // 500 - 300 = 200 PHYSICAL px of overflow. `ScrollPosition` is logical, so
    // the answer must shrink with the scale factor, not stay at 200.
    assert_eq!(max_scroll_y(Some(&viewport(1.0))), 200.0);
    assert_eq!(
        max_scroll_y(Some(&viewport(2.0))),
        100.0,
        "on a 2x display the maximum is half the physical overflow"
    );
    assert_eq!(
        max_scroll_y(None),
        f32::MAX,
        "before layout there is nothing measured to clamp against"
    );
}

#[test]
fn page_step_is_logical_pixels_at_every_scale() {
    assert_eq!(page_step(Some(&viewport(1.0))), 240.0);
    assert_eq!(
        page_step(Some(&viewport(2.0))),
        120.0,
        "one page is 0.8 of the LOGICAL viewport, not 1.6 of it"
    );
    assert_eq!(page_step(None), 0.0);
}

/// The wheel delta is logical too, so the same gesture must land at the same
/// clamped offset whatever the display scale.
#[test]
fn wheel_clamps_at_both_ends() {
    let scroll_after = |scale: f32, start_y: f32, wheel_y: f32| -> f32 {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        app.world_mut().spawn((
            ScrollViewport,
            viewport(scale),
            ScrollPosition(Vec2::new(0.0, start_y)),
        ));
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Pixel,
            x: 0.0,
            y: wheel_y,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.world_mut()
            .run_system_once(scroll_viewports)
            .expect("scroll driver runs");
        app.world_mut()
            .query::<&ScrollPosition>()
            .single(app.world())
            .expect("one scroll position")
            .0
            .y
    };

    assert_eq!(
        scroll_after(1.0, 0.0, 5.0),
        0.0,
        "wheel up clamps at the top"
    );
    assert_eq!(scroll_after(1.0, 0.0, -50.0), 50.0);
    assert_eq!(
        scroll_after(2.0, 0.0, -500.0),
        100.0,
        "the bottom clamp is the logical maximum, not the physical one"
    );
}

/// F28: nothing re-clamped when the CONTENT shrank, so collapsing a section left
/// the pane scrolled past its end and rendering blank.
#[test]
fn clamp_pulls_a_stale_offset_back_when_content_shrinks() {
    let mut app = App::new();
    let panel = app
        .world_mut()
        .spawn((
            ScrollViewport,
            viewport(1.0),
            ScrollPosition(Vec2::new(0.0, 200.0)),
        ))
        .id();

    // The content collapses to less than the viewport: no overflow left at all.
    app.world_mut().entity_mut(panel).insert(ComputedNode {
        content_size: Vec2::new(200.0, 100.0),
        ..viewport(1.0)
    });
    app.world_mut()
        .run_system_once(clamp_viewports)
        .expect("clamp runs");

    assert_eq!(
        app.world()
            .entity(panel)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y,
        0.0,
        "a shrinking content size pulls the stored offset back to the top"
    );
}

#[test]
fn hovered_viewport_takes_the_whole_wheel() {
    use bevy::picking::hover::Hovered;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.world_mut().init_resource::<Messages<MouseWheel>>();
    let hovered = app
        .world_mut()
        .spawn((
            ScrollViewport,
            viewport(1.0),
            ScrollPosition::default(),
            Hovered(true),
        ))
        .id();
    let idle = app
        .world_mut()
        .spawn((
            ScrollViewport,
            viewport(1.0),
            ScrollPosition::default(),
            Hovered(false),
        ))
        .id();
    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Pixel,
        x: 0.0,
        y: -30.0,
        window: Entity::PLACEHOLDER,
        phase: TouchPhase::Moved,
    });
    app.world_mut()
        .run_system_once(scroll_viewports)
        .expect("scroll driver runs");

    assert_eq!(
        app.world()
            .entity(hovered)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y,
        30.0
    );
    assert_eq!(
        app.world()
            .entity(idle)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y,
        0.0,
        "the unhovered viewport stays put while a sibling is hovered"
    );
}

/// The bar is spawned beside the pane it drives, in one `children!` that
/// cannot name either entity. The wiring pass is what makes that legal.
#[test]
fn a_scroll_bar_takes_the_pane_beside_it() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let row = app
        .world_mut()
        .spawn(scroll_row())
        .with_children(|row| {
            row.spawn((scroll_column(), scroll_viewport()));
            row.spawn(scroll_bar(UiSkin::default()));
        })
        .id();
    app.world_mut().run_system_once(wire_scroll_bars).unwrap();

    let children: Vec<Entity> = app.world().entity(row).get::<Children>().unwrap().to_vec();
    let (pane, bar) = (children[0], children[1]);
    assert_eq!(
        app.world()
            .entity(bar)
            .get::<bevy::ui_widgets::Scrollbar>()
            .map(|bar| bar.target),
        Some(pane),
        "the bar drives the viewport it stands next to"
    );
}

/// A bar left pointing at a despawned pane drives nothing, so the pass has to
/// look again rather than trusting the component it already wrote.
#[test]
fn a_rebuilt_pane_is_picked_up_again() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let row = app
        .world_mut()
        .spawn(scroll_row())
        .with_children(|row| {
            row.spawn((scroll_column(), scroll_viewport()));
            row.spawn(scroll_bar(UiSkin::default()));
        })
        .id();
    app.world_mut().run_system_once(wire_scroll_bars).unwrap();

    let children: Vec<Entity> = app.world().entity(row).get::<Children>().unwrap().to_vec();
    app.world_mut().entity_mut(children[0]).despawn();
    let fresh = app
        .world_mut()
        .spawn((scroll_column(), scroll_viewport(), ChildOf(row)))
        .id();
    app.world_mut().run_system_once(wire_scroll_bars).unwrap();

    assert_eq!(
        app.world()
            .entity(children[1])
            .get::<bevy::ui_widgets::Scrollbar>()
            .map(|bar| bar.target),
        Some(fresh),
        "the bar follows the pane that replaced the one it had"
    );
}

/// A bar over a pane with nothing to scroll is a bright line saying nothing.
/// It keeps its slot in the row - the pane must not change width when the bar
/// comes and goes - and stops being painted.
#[test]
fn a_bar_is_painted_only_while_its_pane_can_move() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let short = app
        .world_mut()
        .spawn((
            scroll_column(),
            scroll_viewport(),
            ComputedNode {
                size: Vec2::new(200.0, 300.0),
                content_size: Vec2::new(200.0, 120.0),
                inverse_scale_factor: 1.0,
                ..ComputedNode::DEFAULT
            },
        ))
        .id();
    let bar = app
        .world_mut()
        .spawn((
            scroll_bar(UiSkin::default()),
            bevy::ui_widgets::Scrollbar::new(
                short,
                bevy::ui_widgets::ControlOrientation::Vertical,
                24.0,
            ),
        ))
        .id();
    app.world_mut()
        .run_system_once(hide_idle_scroll_bars)
        .unwrap();
    assert_eq!(
        app.world().entity(bar).get::<Visibility>(),
        Some(&Visibility::Hidden),
        "nothing overflows, so nothing is painted"
    );

    app.world_mut().entity_mut(short).insert(ComputedNode {
        size: Vec2::new(200.0, 300.0),
        content_size: Vec2::new(200.0, 900.0),
        inverse_scale_factor: 1.0,
        ..ComputedNode::DEFAULT
    });
    app.world_mut()
        .run_system_once(hide_idle_scroll_bars)
        .unwrap();
    assert_eq!(
        app.world().entity(bar).get::<Visibility>(),
        Some(&Visibility::Inherited),
        "the content outgrew the pane, so the bar says how far"
    );
}

/// A bar outlives the pane it drives for as long as the frame that despawns
/// one takes to despawn the other. Unknown is not "scrolls forever": an
/// unmeasured or missing pane leaves a full-track bar standing over nothing.
#[test]
fn a_bar_over_a_pane_that_is_gone_is_not_painted() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let pane = app
        .world_mut()
        .spawn((
            scroll_column(),
            scroll_viewport(),
            ComputedNode {
                size: Vec2::new(200.0, 300.0),
                content_size: Vec2::new(200.0, 900.0),
                inverse_scale_factor: 1.0,
                ..ComputedNode::DEFAULT
            },
        ))
        .id();
    let bar = app
        .world_mut()
        .spawn((
            scroll_bar(UiSkin::default()),
            bevy::ui_widgets::Scrollbar::new(
                pane,
                bevy::ui_widgets::ControlOrientation::Vertical,
                24.0,
            ),
        ))
        .id();
    app.world_mut()
        .run_system_once(hide_idle_scroll_bars)
        .unwrap();
    assert_eq!(
        app.world().entity(bar).get::<Visibility>(),
        Some(&Visibility::Inherited),
        "the pane overflows while it is there"
    );

    app.world_mut().entity_mut(pane).despawn();
    app.world_mut()
        .run_system_once(hide_idle_scroll_bars)
        .unwrap();

    assert_eq!(
        app.world().entity(bar).get::<Visibility>(),
        Some(&Visibility::Hidden),
        "and the bar goes with it"
    );
}
