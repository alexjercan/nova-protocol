//! The CRT pipeline: the sampled image, its uniforms and the forwarded pointer.

use bevy::camera::{visibility::RenderLayers, RenderTarget};

use super::*;

#[test]
fn nova_os_screen_samples_offscreen_image() {
    // Render-capable: the screen node hosts ONE sampling surface bound to the
    // offscreen image, the terminal content lives in the image-camera content
    // root, and the old overlay path is gone.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    spawn_nova_os_shell_with_crt(&mut app);

    let surfaces = app
        .world_mut()
        .query_filtered::<&MaterialNode<NovaOsCrtMaterial>, With<NovaOsSamplingSurfaceMarker>>()
        .iter(app.world())
        .count();
    assert_eq!(
        surfaces, 1,
        "the screen has one shader-backed sampling surface in render-capable apps"
    );

    let (rtt_content_root, rtt_image) = {
        let rtt = app
            .world()
            .get_resource::<NovaOsRtt>()
            .expect("render-capable build inserts the NovaOsRtt pipeline");
        (rtt.content_root, rtt.image.clone())
    };

    // The chrome (header + main + footer) renders through the image camera,
    // i.e. under the content root, not directly under the screen node. The
    // main region is a direct child of the content root; the terminal content
    // now lives inside main.
    let (main_entity, main_parent) = app
        .world_mut()
        .query_filtered::<(Entity, &ChildOf), With<NovaOsMainMarker>>()
        .single(app.world())
        .map(|(entity, parent)| (entity, parent.parent()))
        .expect("main region exists");
    assert_eq!(
        main_parent, rtt_content_root,
        "the main region is routed to the offscreen content root"
    );
    let content_parent = app
        .world_mut()
        .query_filtered::<&ChildOf, With<NovaOsTerminalContentMarker>>()
        .single(app.world())
        .expect("terminal content exists")
        .parent();
    assert_eq!(
        content_parent, main_entity,
        "the terminal content is nested inside the main region"
    );

    // The sampling material binds the offscreen image (not the default handle).
    let material = app
        .world()
        .resource::<Assets<NovaOsCrtMaterial>>()
        .iter()
        .next()
        .expect("one CRT material")
        .1;
    assert_eq!(
        material.source, rtt_image,
        "the sampling material binds the offscreen image target"
    );
    assert_eq!(
        material.data.vignette_strength, NOVA_OS_CRT_VIGNETTE_STRENGTH,
        "CRT material carries the near-black corner pass"
    );
}

#[test]
fn nova_os_crt_material_receives_resolution_time_and_power() {
    // `animate_nova_os_crt` feeds the sampling surface's ComputedNode size into
    // the material's `resolution` uniform (resolution-aware scanlines + bloom
    // taps), stamps `time`, and pipes NovaOsOpenness in as the `power` level
    // (the raster power-on/off collapse).
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<NovaOsCrtMaterial>();
    app.init_resource::<Time<Real>>();
    app.init_resource::<NovaOsMonitorSettings>();
    app.init_resource::<NovaOsDegauss>();
    app.add_systems(Update, animate_nova_os_crt);

    // The eased openness the shader reads as its power level.
    app.world_mut()
        .spawn((NovaOsRootMarker, NovaOsOpenness(0.5)));
    let handle = app
        .world_mut()
        .resource_mut::<Assets<NovaOsCrtMaterial>>()
        .add(NovaOsCrtMaterial::default());
    app.world_mut().spawn((
        NovaOsSamplingSurfaceMarker,
        MaterialNode(handle.clone()),
        ComputedNode {
            size: Vec2::new(800.0, 600.0),
            ..default()
        },
    ));

    app.update();

    let material = app
        .world()
        .resource::<Assets<NovaOsCrtMaterial>>()
        .get(&handle)
        .expect("CRT material still present");
    assert_eq!(
        material.data.resolution,
        Vec2::new(800.0, 600.0),
        "the screen's panel pixel size is fed into the resolution uniform"
    );
    assert!(
        material.data.time.is_finite(),
        "the shimmer time uniform is stamped each frame"
    );
    assert_eq!(
        material.data.power, 0.5,
        "NovaOsOpenness drives the CRT power collapse uniform"
    );
}

#[test]
fn nova_os_app_mode_change_pulses_and_decays_the_degauss_uniform() {
    // A real app launch/exit/switch (a mode change that
    // gets past `sync_nova_os_app_ui`'s diff-guard) kicks the degauss coil, and
    // `animate_nova_os_crt` bleeds the 0..1 envelope back down over real time.
    // Driven with `run_system_once` + a hand-advanced `Time<Real>` so the decay
    // is deterministic rather than tied to the wall clock.
    use std::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<NovaOsCrtMaterial>();
    // The app-launch spawn loads a font through the AssetServer this rig has
    // (AssetPlugin), so the Font/Image asset types must be registered or the
    // load panics (mirrors `chin_controls_app`).
    app.init_asset::<Font>();
    app.init_asset::<Image>();
    app.init_resource::<Time<Real>>();
    app.init_resource::<NovaOsMonitorSettings>();
    app.init_resource::<NovaOsDegauss>();
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<NovaOsTerminal>();
    let mut registry = NovaOsCommandRegistry::default();
    registry.register(TerminalCommand::app(
        "sample",
        "Test-only lifecycle app",
        SampleApp,
    ));
    app.insert_resource(registry);

    app.world_mut()
        .spawn((NovaOsRootMarker, NovaOsOpenness(1.0)));
    // A screen entity so the launch actually spawns an app root - that is what
    // makes the NEXT frame a no-op (current == desired) so the pulse fires once,
    // not every frame.
    app.world_mut().spawn(NovaOsScreenMarker);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<NovaOsCrtMaterial>>()
        .add(NovaOsCrtMaterial::default());
    app.world_mut().spawn((
        NovaOsSamplingSurfaceMarker,
        MaterialNode(handle.clone()),
        ComputedNode {
            size: Vec2::new(800.0, 600.0),
            ..default()
        },
    ));

    let degauss_uniform = |app: &App| {
        app.world()
            .resource::<Assets<NovaOsCrtMaterial>>()
            .get(&handle)
            .expect("CRT material still present")
            .data
            .degauss
    };

    // Idle: no pulse yet.
    app.world_mut()
        .run_system_once(animate_nova_os_crt)
        .unwrap();
    assert_eq!(
        degauss_uniform(&app),
        0.0,
        "the CRT sits idle before any launch"
    );

    // Launch an app: the mode change pulses the coil, and the same frame's
    // animate stamps the (near-full) envelope into the uniform.
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app("sample");
    app.world_mut()
        .run_system_once(sync_nova_os_app_ui)
        .unwrap();
    app.world_mut()
        .run_system_once(animate_nova_os_crt)
        .unwrap();
    let peak = degauss_uniform(&app);
    assert!(
        peak > 0.9,
        "a launch kicks the degauss envelope to (near) full, got {peak}"
    );

    // Half the pulse duration later, with no new mode change, the envelope has
    // bled partway down but is still lit.
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs_f32(NOVA_OS_DEGAUSS_DURATION * 0.5));
    app.world_mut()
        .run_system_once(sync_nova_os_app_ui)
        .unwrap();
    app.world_mut()
        .run_system_once(animate_nova_os_crt)
        .unwrap();
    let mid = degauss_uniform(&app);
    assert!(
        mid < peak && mid > 0.0,
        "the envelope decays but is still lit mid-pulse, got {mid} (peak {peak})"
    );

    // Past the full duration it has settled back to an exact no-op.
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs_f32(NOVA_OS_DEGAUSS_DURATION));
    app.world_mut()
        .run_system_once(animate_nova_os_crt)
        .unwrap();
    assert_eq!(
        degauss_uniform(&app),
        0.0,
        "the degauss envelope settles back to zero (readability preserved at rest)"
    );
}

#[test]
fn mirror_hover_serves_content_but_never_clobbers_window_ui() {
    // `mirror_nova_os_hover` must feed `Hovered` for the forwarded pointer
    // ONLY on entities rendered through the image (descendants of the content
    // root). It must NOT touch window-space UI - otherwise it force-writes
    // `Hovered(false)` on the real cursor's targets every frame (regressing the
    // chin knobs, menus, any Button). Regression pin for review finding M1.
    use bevy::{
        ecs::entity::EntityHashMap, picking::backend::HitData, platform::collections::HashMap,
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, mirror_nova_os_hover);

    let content_root = app.world_mut().spawn(Hovered::default()).id();
    // A terminal node under the content root (served by the forwarded pointer).
    let terminal_node = app
        .world_mut()
        .spawn((Hovered::default(), ChildOf(content_root)))
        .id();
    // A window-space node hovered by the REAL mouse pointer, NOT under the
    // content root.
    let window_node = app.world_mut().spawn(Hovered(true)).id();

    // The NovaOsRtt pipeline (only content_root matters here).
    app.insert_resource(NovaOsRtt {
        image: Handle::default(),
        camera: Entity::PLACEHOLDER,
        content_root,
        pointer: Entity::PLACEHOLDER,
    });

    // The forwarded pointer's HoverMap hits the terminal node.
    let mut inner = EntityHashMap::default();
    inner.insert(
        terminal_node,
        HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    );
    let mut map: HashMap<PointerId, EntityHashMap<HitData>> = HashMap::default();
    map.insert(nova_os_pointer_id(), inner);
    app.insert_resource(HoverMap(map));

    app.update();

    assert!(
        app.world()
            .entity(terminal_node)
            .get::<Hovered>()
            .unwrap()
            .get(),
        "content-root node hit by the forwarded pointer is mirrored to Hovered(true)"
    );
    assert!(
        app.world()
            .entity(window_node)
            .get::<Hovered>()
            .unwrap()
            .get(),
        "window-space Hovered(true) is NOT clobbered by the forwarded-pointer mirror"
    );
}
/// The pointer's screen->image mapping IS the shader's, everywhere on the
/// screen - not an approximation of its inverse.
///
/// This is the failing test the bug was found with: before the fix the
/// pointer applied `c / (1 + a*r^2)` (the barrel INVERSE, i.e. the wrong
/// direction) and ignored the shader's 0.93 overscan entirely, so on a
/// 1280x720 screen the worst-case miss was 27.1 px in x / 15.3 px in y at the
/// corners and still ~8 px only a tenth of the way out from centre - both far
/// wider than the 12 px blips, which is why map contacts spread across the
/// viewport were unclickable while the ship app's centre-clustered ones were
/// not.
///
/// Swept at several power levels, not just a full raster (review R1.1): at a
/// settled raster the collapse remap is exactly the identity, so a grid run
/// only at full power leaves that whole half of the mapping unexercised - a
/// divide flipped to a multiply there passed the entire suite.
#[test]
fn nova_os_pointer_mapping_matches_the_crt_shader_across_the_screen() {
    use crate::hud::nova_os_pointer_rig::{crt_uv_grid, shader_draws_at, CRT_MAP_BUDGET_PX};

    let image = Vec2::new(1280.0, 720.0);
    let mut powers_that_collapsed = 0;
    // 1.0 and 0.65 are settled rasters (both smoothsteps have reached 1 by
    // the taller edge); 0.35 is squeezed vertically only (`open_w` is already
    // 1 past its 0.28 edge), 0.15 in both axes - so the sweep covers each
    // branch of the collapse.
    for power in [0.15, 0.35, 0.65, 1.0] {
        let mut worst = Vec2::ZERO;
        let mut worst_at = Vec2::ZERO;
        let mut on_picture = 0;
        let mut off_picture = 0;
        for uv in crt_uv_grid() {
            let ours =
                nova_os_crt_screen_to_image_uv(uv, NOVA_OS_CRT_WARP, NOVA_OS_CRT_OVERSCAN, power);
            // `shader_draws_at` is the shader's WHOLE answer: the sample UV,
            // gated by both the raster-collapse test and the in-bounds test
            // its fragment multiplies the output by. The pointer must call a
            // point clickable exactly when the shader draws something there.
            let reference = shader_draws_at(uv, NOVA_OS_CRT_WARP, NOVA_OS_CRT_OVERSCAN, power);
            assert_eq!(
                ours.is_some(),
                reference.is_some(),
                "at power {power}, screen uv {uv:?}: the pointer says {} but the \
                 shader draws {}",
                if ours.is_some() {
                    "on-picture"
                } else {
                    "off-picture"
                },
                match reference {
                    Some(at) => format!("image uv {at:?} there"),
                    None => "nothing there".to_string(),
                },
            );
            match (ours, reference) {
                (Some(ours), Some(reference)) => {
                    on_picture += 1;
                    let error = ((ours - reference) * image).abs();
                    if error.max_element() > worst.max_element() {
                        worst = error;
                        worst_at = uv;
                    }
                }
                _ => off_picture += 1,
            }
        }
        assert!(
            worst.max_element() <= CRT_MAP_BUDGET_PX,
            "at power {power} the forwarded pointer lands {worst:?} px from what \
             the CRT shader displays (worst at screen uv {worst_at:?}), budget \
             {CRT_MAP_BUDGET_PX} px",
        );
        // Guard the guard: a power that mapped NOTHING on-picture would
        // satisfy the budget vacuously.
        assert!(
            on_picture > 0,
            "power {power} put the whole grid off-picture - the budget above \
             asserted nothing"
        );
        // The raster is collapsed exactly while a smoothstep is still below
        // 1, i.e. below the TALLER of the two edges. Derived from the
        // constants rather than assumed from "power < 1", which is wrong at
        // 0.65 (open_h has already reached 1 there).
        let collapsed = power < NOVA_OS_CRT_POWER_OPEN_H.max(NOVA_OS_CRT_POWER_OPEN_W);
        assert_eq!(
            off_picture > 0,
            collapsed,
            "at power {power} the raster is {} yet {off_picture} of the grid \
             mapped off-picture",
            if collapsed { "COLLAPSED" } else { "fully open" },
        );
        powers_that_collapsed += usize::from(collapsed);
    }
    // ...and the sweep as a whole must actually reach the collapsed regime,
    // or the divide in the remap is still never exercised (review R1.1).
    assert!(
        powers_that_collapsed >= 2,
        "only {powers_that_collapsed} of the swept powers collapsed the raster \
         - the sweep is not covering the remap"
    );
}

/// The shader and the pointer must not be able to drift apart: the WGSL reads
/// its warp AND its overscan from the uniform this crate fills, and the only
/// place the barrel algebra lives in WGSL is the `barrel()` helper the
/// reference above transcribes.
#[test]
fn nova_os_crt_shader_takes_its_warp_and_overscan_from_the_uniform() {
    let source = std::fs::read_to_string("../../assets/shaders/nova_os_crt.wgsl")
        .expect("the CRT shader source is readable from the crate dir");

    assert!(
        source.contains("return vec2<f32>(0.5, 0.5) + centered * (1.0 + amount * r2);"),
        "the shader's barrel() is no longer the algebra the pointer mirrors - \
         re-derive `nova_os_crt_screen_to_image_uv` from the new shader",
    );
    assert!(
        source.contains("barrel(shaken_uv, material.warp)"),
        "the shader must take its warp amount from the uniform this crate fills",
    );
    assert!(
        source.contains("* material.overscan +"),
        "the shader must take its overscan from the uniform this crate fills, \
         not from a WGSL-local constant the Rust side cannot see",
    );
    assert!(
        !source.contains("const NOVA_OS_OVERSCAN"),
        "a WGSL-local overscan constant is a second definition of the mapping \
         - the pointer cannot see it, which is exactly how this bug happened",
    );

    // The power-collapse remap stays a pair of literals on each side (it is
    // shape, not a tunable knob), so pin the shader's against the Rust ones
    // rather than leaving them free to drift.
    for line in [
        format!("smoothstep(0.0, {NOVA_OS_CRT_POWER_OPEN_H}, material.power)"),
        format!("smoothstep(0.0, {NOVA_OS_CRT_POWER_OPEN_W}, material.power)"),
        format!("max(open_h, {NOVA_OS_CRT_POWER_EPSILON})"),
        format!("max(open_w, {NOVA_OS_CRT_POWER_EPSILON})"),
    ] {
        assert!(
            source.contains(&line),
            "the shader no longer contains `{line}` - the pointer's raster-collapse \
             remap has drifted from the picture's",
        );
    }
}

/// The uniform the shader reads carries the same constants the pointer maps
/// with, so `nova_os_crt_screen_to_image_uv` describes the live composite.
#[test]
fn nova_os_crt_material_publishes_the_mapping_constants() {
    let material = NovaOsCrtMaterial::default();
    assert_eq!(material.data.warp, NOVA_OS_CRT_WARP);
    assert_eq!(material.data.overscan, NOVA_OS_CRT_OVERSCAN);
}

/// The RTT ELEMENT claim, the one the retired `nova_os_rtt_poc` example was the
/// only evidence for: the screen displays a subtree that is actually rendered
/// offscreen, not an empty target.
///
/// The sampling half - the surface, the material, the uniforms, the forwarded
/// pointer - is covered above. What no other test names is the CAMERA: that an
/// image camera exists, that it draws into the image the shader samples, and
/// that it and the content subtree share the private render layer that keeps
/// stray world 2D sprites out of the terminal picture.
#[test]
fn rtt_element_renders_its_subtree() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    spawn_nova_os_shell_with_crt(&mut app);

    let (camera, content_root, image) = {
        let rtt = app
            .world()
            .get_resource::<NovaOsRtt>()
            .expect("render-capable build inserts the NovaOsRtt pipeline");
        (rtt.camera, rtt.content_root, rtt.image.clone())
    };

    let cameras = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsImageCameraMarker>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(
        cameras,
        vec![camera],
        "exactly one image camera exists, and it is the one the pipeline resource names"
    );

    // It draws INTO the image the CRT surface samples - the two halves of the
    // element meeting at the same handle.
    let target = app
        .world()
        .entity(camera)
        .get::<RenderTarget>()
        .expect("the image camera renders to a target");
    assert!(
        matches!(target, RenderTarget::Image(target) if target.handle == image),
        "the image camera renders into the offscreen image the screen samples"
    );

    let camera_component = app
        .world()
        .entity(camera)
        .get::<Camera>()
        .expect("the image camera is a camera");
    assert_eq!(
        camera_component.order, NOVA_OS_RTT_CAMERA_ORDER,
        "the offscreen pass runs before the window/UI cameras, so the sampled \
         image is ready when the screen surface reads it"
    );

    let rtt_layer = RenderLayers::layer(NOVA_OS_RTT_LAYER);
    assert_eq!(
        app.world().entity(camera).get::<RenderLayers>(),
        Some(&rtt_layer),
        "the image camera draws ONLY the private terminal layer"
    );
    assert_eq!(
        app.world().entity(content_root).get::<RenderLayers>(),
        Some(&rtt_layer),
        "the content root sits on the layer the image camera draws"
    );

    // NON-EMPTY: the element would sample a blank target if the chrome were
    // routed anywhere else. Walk the subtree rather than counting children, so
    // a nesting change cannot quietly empty the picture.
    let mut subtree = Vec::new();
    let mut stack = vec![content_root];
    while let Some(entity) = stack.pop() {
        if entity != content_root {
            subtree.push(entity);
        }
        if let Some(children) = app.world().entity(entity).get::<Children>() {
            stack.extend(children.iter());
        }
    }
    assert!(
        !subtree.is_empty(),
        "the content root carries the terminal subtree; an empty one means the \
         screen samples a blank image"
    );

    // Nothing under it opts back out onto another layer - a descendant on a
    // layer the image camera does not draw is invisible in the picture while
    // still passing every structural test above.
    let strays: Vec<Entity> = subtree
        .into_iter()
        .filter(|entity| {
            app.world()
                .entity(*entity)
                .get::<RenderLayers>()
                .is_some_and(|layers| *layers != rtt_layer)
        })
        .collect();
    assert!(
        strays.is_empty(),
        "every node under the content root stays on the image camera's layer; \
         {strays:?} carry another"
    );
}
