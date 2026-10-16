//! Headless tests of satellite grouping and the motion rule (GRP, MOT):
//! fold state as the scale changes, count pills, motion modes as the rate
//! changes, and how a ring-mode body hands the frame to its parent.

use super::*;
use crate::view::markers::pills::parent_clear_px;

/// ARCH-31 / GRP-01 / GRP-02 / GRP-04: fold state follows the scale. At the
/// system frame every satellite is folded; at Halden IV's satellite frame
/// its outer moons unfold while the close-in station stays folded; and back
/// out, everything folds again.
#[test]
fn fold_state_after_a_scale_change() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin)).add_plugins((
        TunablesPlugin,
        ProjectionPlugin,
        StopsPlugin,
        GroupingPlugin,
    ));
    run(&mut app, 2);
    // Hold everything still so the camera stays on its target.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 2);

    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let system_radius = app.world().resource::<ZoomStops>().system_radius;
    let system_distance = fit_distance(system_radius, fov(), TILT, margin);
    let halden = body(&mut app, "Halden IV");
    let satellite_radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let satellite_distance = fit_distance(satellite_radius, fov(), TILT, margin);
    let halden_pos = app
        .world()
        .get::<GlobalTransform>(halden)
        .unwrap()
        .translation();

    let camera = app
        .world_mut()
        .spawn(test_camera(Vec3::ZERO, system_distance))
        .id();
    run(&mut app, SETTLE_FRAMES);
    let folded = |app: &mut App, name: &str| {
        let entity = body(app, name);
        app.world()
            .get::<SatelliteFoldState>(entity)
            .unwrap()
            .folded
    };
    let mut satellites = app.world_mut().query::<&SatelliteFoldState>();
    assert!(
        satellites.iter(app.world()).all(|fold| fold.folded),
        "GRP-04: everything folded at the system fit"
    );

    // Zoom in to Halden IV's satellite frame.
    let (_, _, _, close) = test_camera(halden_pos, satellite_distance);
    *app.world_mut().get_mut::<Transform>(camera).unwrap() = close;
    run(&mut app, SETTLE_FRAMES);
    assert!(
        !folded(&mut app, "Halden IVd"),
        "the outermost moon unfolds"
    );
    assert!(!folded(&mut app, "Halden IVc"));
    assert!(
        folded(&mut app, "Arden Highport"),
        "GRP-02: inner ones later"
    );
    assert!(folded(&mut app, "Iver Yard"), "other planets stay folded");

    // And back out.
    let (_, _, _, far) = test_camera(Vec3::ZERO, system_distance);
    *app.world_mut().get_mut::<Transform>(camera).unwrap() = far;
    run(&mut app, SETTLE_FRAMES);
    let mut satellites = app.world_mut().query::<&SatelliteFoldState>();
    assert!(
        satellites
            .iter(app.world())
            .all(|fold| fold.folded && fold.effective_hidden)
    );
}

/// ARCH-31 / MOT-01, MOT-02, MOT-04, MOT-12, MOT-13: the motion mode
/// follows the time rate, easing into each change.
#[test]
fn motion_mode_after_a_rate_change() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, MotionPlugin))
        // Shared view state the ring labels read; `ViewPlugin` owns it.
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .insert_resource(placeholder_fonts());
    let ring = |app: &mut App, name: &str| {
        let entity = body(app, name);
        app.world()
            .get::<MotionModeState>(entity)
            .unwrap()
            .ring_mode
    };

    run(&mut app, SETTLE_FRAMES);
    // TEST-02 at 1 d/s: IVa (1.5 s) is a ring, IVc (31 s) a dot.
    assert!(ring(&mut app, "Halden IVa"));
    assert!(!ring(&mut app, "Halden IVc"));

    // At 5 d/s IVc's apparent period is 6.2 s: a ring.
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, SETTLE_FRAMES);
    assert!(ring(&mut app, "Halden IVc"));

    // MOT-12: paused, everything is a full-brightness dot with its tail,
    // capped at `dot_fade_from_degrees`.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    // MOT-13: the tail eases down rather than snapping, so a frame after
    // pausing IVc's dot is still fading back in.
    run(&mut app, 1);
    let ivc = body(&mut app, "Halden IVc");
    let easing = *app.world().get::<MotionModeState>(ivc).unwrap();
    assert!(easing.tail_degrees > 300.0 && easing.dot_alpha() < 1.0);
    let settled = eased_frames();
    run(&mut app, settled);
    let mut modes = app.world_mut().query::<&MotionModeState>();
    assert!(
        modes
            .iter(app.world())
            .all(|mode| !mode.ring_mode && mode.dot_alpha() == 1.0 && mode.tail_degrees <= 300.0)
    );
    // Unpausing restores the running state.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, settled);
    assert!(ring(&mut app, "Halden IVc"));
}

/// TEST-08 / MOT-02: at 1 d/s six of Morrow's moons (periods under 8 d)
/// are exposure rings and five (8 d or more) are dots.
#[test]
fn morrows_moons_straddle_both_motion_modes() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, MotionPlugin))
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .insert_resource(placeholder_fonts());
    run(&mut app, 2);
    let morrow = body(&mut app, "Morrow");
    let mut moons = app
        .world_mut()
        .query::<(&ChildOf, &BodyClass, &MotionModeState)>();
    let (rings, dots): (Vec<bool>, Vec<bool>) = moons
        .iter(app.world())
        .filter(|(child_of, class, _)| child_of.parent() == morrow && **class == BodyClass::Moon)
        .map(|(_, _, mode)| mode.ring_mode)
        .partition(|ring| *ring);
    assert_eq!((rings.len(), dots.len()), (6, 5));
}

/// MOT-08: the motion rule covers every orbiting object - planets, moons,
/// stations and parked ships - and not the gates. At 5 d/s Pell (an 86-day
/// orbit, 17 s apparent) becomes an exposure ring like any satellite.
#[test]
fn the_rule_applies_to_every_orbiting_object() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, MotionPlugin))
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .insert_resource(placeholder_fonts());
    run(&mut app, 2);
    for name in [
        "Pell",
        "Iver Yard",
        "Orison",
        "K-3",
        "Halden IVc",
        "Lantern",
    ] {
        let entity = body(&mut app, name);
        assert!(
            app.world().get::<MotionModeState>(entity).is_some(),
            "{name}"
        );
    }
    let gate = body(&mut app, "Gate Ostra");
    assert!(
        app.world().get::<MotionModeState>(gate).is_none(),
        "gates keep edge arrows"
    );

    app.world_mut().resource_mut::<GameClock>().run_at(16.0);
    run(&mut app, SETTLE_FRAMES);
    let pell = body(&mut app, "Pell");
    assert!(app.world().get::<MotionModeState>(pell).unwrap().ring_mode);
}

/// MRK-06: zoomed all the way out, where the inner planets crowd
/// together, every planet still has its own marker position on screen and
/// none is ever folded.
#[test]
fn planets_are_never_merged() {
    let mut app = camera_app_with(|app| {
        app.add_plugins(GroupingPlugin);
    });
    for _ in 0..10 {
        scroll(&mut app, -1.0);
    }
    run(&mut app, SETTLE_FRAMES);
    let moon = body(&mut app, "Halden IVc");
    assert!(app.world().get::<SatelliteFoldState>(moon).unwrap().folded);
    let names = ["Pell", "Iver", "Halden IV", "Morrow"];
    let planets: Vec<Entity> = names.iter().map(|name| body(&mut app, name)).collect();
    let screen = app.world().resource::<ScreenPositions>();
    let positions: Vec<Vec2> = planets
        .iter()
        .map(|&planet| screen.get(planet).unwrap())
        .collect();
    for (i, &planet) in planets.iter().enumerate() {
        assert!(app.world().get::<SatelliteFoldState>(planet).is_none());
        assert!(screen.in_viewport(positions[i]), "{} off screen", names[i]);
        for j in 0..i {
            assert_ne!(positions[i], positions[j]);
        }
    }
}

/// GRP-05: at the system fit Halden IV shows a "+N" pill counting all
/// its folded satellites, beside its marker and clear of its bracket box.
#[test]
fn a_parent_with_folded_satellites_shows_a_count_pill() {
    let mut app = camera_app_with(|app| {
        app.add_plugins((GroupingPlugin, PillsPlugin))
            .insert_resource(placeholder_fonts());
    });
    run(&mut app, SETTLE_FRAMES);

    let halden = body(&mut app, "Halden IV");
    let satellites = app.world().get::<Children>(halden).unwrap().len();
    let marker = app
        .world()
        .resource::<ScreenPositions>()
        .get(halden)
        .unwrap();
    let mut pills = app.world_mut().query::<(&GroupPillFor, &Node, &Children)>();
    let (_, node, children) = pills
        .iter(app.world())
        .find(|(pill_for, ..)| pill_for.0 == halden)
        .unwrap();
    assert_eq!(node.display, Display::Flex);
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("pill not placed in pixels");
    };
    let clear = parent_clear_px(BodyClass::Planet, false, 0.0, &Tunables::default());
    assert!(left - marker.x >= clear, "the pill overlaps the marker");
    assert!(
        top > marker.y,
        "the pill sits beside the marker, below its centre"
    );
    let count = children
        .iter()
        .find_map(|child| app.world().get::<Text>(child))
        .unwrap();
    assert_eq!(count.0, format!("+{satellites}"));
}

/// TEST-09: Morrow IX's own satellites at 1 d/s: Tern Relay is an exposure
/// ring and the trader a dot. Both fold away at the system fit; framing
/// Morrow IX, three levels below the star, moves to its satellite fit,
/// where the trader unfolds.
#[test]
fn a_moon_with_satellites_folds_and_frames_three_levels_deep() {
    let mut app = camera_app_with(|app| {
        app.add_plugins((MotionPlugin, GroupingPlugin))
            .insert_resource(placeholder_fonts());
    });
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(1.0);
    run(&mut app, SETTLE_FRAMES);
    let ix = body(&mut app, "Morrow IX");
    let relay = body(&mut app, "Tern Relay");
    let trader = body(&mut app, "Kessel trader");
    let ring_mode = |app: &App, e: Entity| app.world().get::<MotionModeState>(e).unwrap().ring_mode;
    let folded = |app: &App, e: Entity| app.world().get::<SatelliteFoldState>(e).unwrap().folded;
    assert!(ring_mode(&app, relay));
    assert!(!ring_mode(&app, trader));
    assert!(folded(&app, relay) && folded(&app, trader));

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(ix);
    run(&mut app, 60);
    assert_eq!(frame_body(&app), ix);
    let morrow = body(&mut app, "Morrow");
    assert_eq!(app.world().get::<ChildOf>(ix).unwrap().parent(), morrow);
    assert!(
        app.world()
            .resource::<ZoomStops>()
            .satellite_radius(ix)
            .is_some()
    );
    assert!(
        !folded(&app, trader),
        "the trader unfolds at IX's satellite fit"
    );
}

/// CAM-24 / ZOOM-16: an object in exposure-ring mode can't be a frame; a
/// request for it moves the frame to its parent instead, and the view to
/// the parent's satellite fit.
#[test]
fn a_ring_mode_object_cannot_be_a_frame() {
    let mut app = motion_camera_app(1.0);
    let iva = body(&mut app, "Halden IVa");
    let halden = body(&mut app, "Halden IV");
    assert!(app.world().get::<MotionModeState>(iva).unwrap().ring_mode);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iva);
    run(&mut app, 1);
    let target = app.world().resource::<ActiveMove>().0.unwrap().target;
    run(&mut app, 40);
    assert_eq!(frame_body(&app), halden);
    let fit = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let tilt = app.world().resource::<CameraRig>().tilt;
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let expected = distance_for_radius(fit, fov(), tilt, margin, VIEWPORT.as_vec2());
    assert!((target.distance - expected).abs() < expected * 1e-3);
}

/// CAM-24: when the frame body enters ring mode, the frame moves to its
/// parent and the view moves to the parent's satellite fit.
#[test]
fn the_frame_moves_to_the_parent_when_the_frame_body_becomes_a_ring() {
    let mut app = motion_camera_app(0.1);
    let ivc = body(&mut app, "Halden IVc");
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(ivc);
    run(&mut app, 40);
    assert_eq!(frame_body(&app), ivc);

    // IVc's apparent period drops below tail_secs.
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, eased_frames());
    let new_frame = frame(&app);
    assert_eq!(new_frame.body, halden, "the frame moved to the parent");
    // The view moved to bring Halden IV back in, as a frame jump does.
    let rig = *app.world().resource::<CameraRig>();
    let stops = app.world().resource::<ZoomStops>();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let framed = fit_distance(
        stops.satellite_radius(halden).unwrap(),
        fov(),
        rig.tilt,
        margin,
    );
    assert!((rig.distance - framed).abs() < framed * 1e-3, "framed");
    assert!(new_frame.offset.length() < 1e-3, "centred on the parent");

    // Slowing down again does not move the frame back.
    app.world_mut().resource_mut::<GameClock>().run_at(0.1);
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(frame_body(&app), halden);
}

/// CAM-24 / CAM-08: a planet's frame with no ancestor left: when the planet
/// turns into an exposure ring, the frame returns to the star at the system
/// fit.
#[test]
fn a_ringed_planet_hands_the_frame_back_to_the_star() {
    let mut app = motion_camera_app(0.1);
    let system_distance = app.world().resource::<CameraRig>().distance;
    let pell = body(&mut app, "Pell");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(pell);
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), pell);

    app.world_mut().resource_mut::<GameClock>().run_at(16.0);
    run(&mut app, SETTLE_FRAMES + move_frames());
    assert!(in_star_frame(&app));
    let rig = *app.world().resource::<CameraRig>();
    let tilt_ratio = system_distance / rig.distance;
    assert!(
        rig.pivot.length() < 1e-3,
        "centred on the star: {}",
        rig.pivot
    );
    // The system fit at the current tilt (the start-up one is at the
    // standard tilt, which this view still has).
    assert!((tilt_ratio - 1.0).abs() < 1e-3, "system fit");
}
