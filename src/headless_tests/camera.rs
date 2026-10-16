//! Headless tests of the camera (CAM, ZOOM): the reference frame and the
//! body kept in place on screen, zoom stops and the wheel, drags, keys,
//! bookmarks and animated moves, the breadcrumb that reframes, and the
//! debug screenshot's view, on Corvane with a camera that projects.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion};

use super::*;
use crate::view::camera::keep::inside_margin;
use crate::view::camera::requests::RecentreRequest;
use crate::view::world::RenderOrigin;

/// ARCH-31 / ZOOM-15: the stops come from Corvane's real orbits.
#[test]
fn zoom_stops_come_from_corvanes_orbits() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, StopsPlugin));
    run(&mut app, 1);

    let close = |a: f32, b: f32| (a - b).abs() < b * 1e-5;
    let stops = app.world().resource::<ZoomStops>();
    // System frame: Morrow at 4.3 AU.
    assert!(close(stops.system_radius, km_to_render(4.3 * KM_PER_AU)));
    let satellite_frames = stops.per_body_satellite_radius.clone();

    for (planet, outermost_km) in [
        ("Halden IV", 726_432.0),
        // Iver Yard, in Iver's High band of 34 radii (BAND-01).
        ("Iver", 197_200.0),
        ("Morrow", 6_619_046.0),
    ] {
        let entity = body(&mut app, planet);
        let radius = satellite_frames[&entity];
        assert!(
            close(radius, km_to_render(outermost_km)),
            "{planet}'s satellite frame is its outermost orbit"
        );
    }
    let pell = body(&mut app, "Pell");
    assert!(
        !satellite_frames.contains_key(&pell),
        "Pell has no satellites"
    );
}

/// CAM-05: a planet that is the frame body keeps its place on screen at
/// any time rate. At 5 d/s, Halden IV moves about 65 px a frame at its
/// satellite fit; following it a frame late made it shake. With zero
/// offset it must sit at the screen centre on every frame.
#[test]
fn the_frame_body_keeps_its_place_on_screen() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, InputPlugin))
        .add_plugins((
            TunablesPlugin,
            BindingsPlugin,
            ProjectionPlugin,
            CameraPlugin,
        ))
        .add_plugins((ScreenPlugin, TrueSizePlugin))
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .add_plugins((FramePlugin, crate::view::sidebar::SidebarPlugin))
        .init_resource::<PressStart>();
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(VIEWPORT.x, VIEWPORT.y),
        ..default()
    });
    run(&mut app, 1);
    compute_camera_headless(&mut app);

    let halden = body(&mut app, "Halden IV");
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    app.world_mut().resource_mut::<CameraRig>().distance = fit_distance(radius, fov(), 0.0, margin);
    set_frame(&mut app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);

    let centre = VIEWPORT.as_vec2() / 2.0;
    for frame in 0..SETTLE_FRAMES {
        run(&mut app, 1);
        let on_screen = app
            .world()
            .resource::<ScreenPositions>()
            .get(halden)
            .unwrap();
        assert!(
            on_screen.distance(centre) < 0.5,
            "frame {frame}: Halden IV at {on_screen}, {} px off centre",
            on_screen.distance(centre)
        );
    }
}

/// SCL-06: render space is centred on the frame body. Framed 4.3 AU out,
/// Morrow sits exactly at the render origin and on the screen centre on
/// every frame at a close zoom, the star sits at minus its position, and a
/// moon's render position is its own offset from Morrow at full precision.
#[test]
fn render_space_is_centred_on_a_distant_frame_body() {
    let mut app = camera_app();
    let morrow = body(&mut app, "Morrow");
    let moon = body(&mut app, "Morrow I");
    let star = frame(&app).star;
    app.world_mut().resource_mut::<CameraRig>().distance = 0.01;
    set_frame(&mut app, morrow, Vec3::ZERO);
    app.world_mut().resource_mut::<GameClock>().run_at(1.0);

    let centre = VIEWPORT.as_vec2() / 2.0;
    for frame in 0..SETTLE_FRAMES {
        run(&mut app, 1);
        let positions = app.world().resource::<WorldPositions>();
        assert_eq!(positions.0[&morrow], Vec3::ZERO, "frame {frame}");
        let local = app.world().get::<OrbitPositionKm>(moon).unwrap().0;
        let expected = km_to_render_vec3(local);
        let error = (positions.0[&moon] - expected).length() / expected.length();
        assert!(error < 1e-6, "frame {frame}: moon off by {error}");
        let morrow_km = app.world().resource::<RenderOrigin>().km;
        let star_at = positions.0[&star];
        assert!((star_at - km_to_render_vec3(-morrow_km)).length() < 1e-3);
        let on_screen = app
            .world()
            .resource::<ScreenPositions>()
            .get(morrow)
            .unwrap();
        assert!(
            on_screen.distance(centre) < 0.01,
            "frame {frame}: Morrow {} px off centre",
            on_screen.distance(centre)
        );
    }
}

/// CAM-06 / ZOOM-14 / ZOOM-13 with the real zoom: the view starts in the
/// star's frame; zooming in on the cursor keeps the point under the cursor
/// in place, never changes the frame, and stops at the minimum view
/// radius.
#[test]
fn zooming_in_the_star_frame_keeps_the_frame_and_the_cursor_point() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let halden_pos = app.world().resource::<WorldPositions>().0[&halden];
    assert!(in_star_frame(&app), "CAM-05: the star's frame by default");
    app.world_mut().resource_mut::<Frame>().offset = halden_pos;
    run(&mut app, 2);
    // ZOOM-13: the cursor's point stays under the cursor.
    let cursor = Vec2::new(1000.0, 600.0);
    move_cursor(&mut app, cursor);
    run(&mut app, 1);
    let before = plane_point_under(&mut app, cursor);
    scroll(&mut app, 1.0);
    let after = plane_point_under(&mut app, cursor);
    assert!(
        (after - before).length() < before.length() * 1e-3,
        "ZOOM-13: {before} drifted to {after}"
    );
    for _ in 0..30 {
        scroll(&mut app, 1.0);
    }
    assert!(in_star_frame(&app), "CAM-06");
    let tilt = app.world().resource::<CameraRig>().tilt;
    let radius = km_to_render(0.05 * KM_PER_AU);
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let limit = fit_distance(radius, fov(), tilt, margin);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!(
        (distance - limit).abs() < limit * 1e-3,
        "ZOOM-14 stopped at {distance}, limit {limit}"
    );
}

/// ZOOM-14 / ZOOM-15 / CAM-08 / CAM-23 with the real zoom: in a planet's
/// frame, zooming in settles on the satellite fit, then goes on to the
/// body's inner limit (its diameter a quarter of the map height) and no
/// further; zooming out to the system fit returns the frame to the star and
/// eases the view to the standard tilt.
#[test]
fn zoom_in_a_frame_has_a_satellite_stop_an_inner_limit_and_ends_at_the_system_fit() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let system_distance = app.world().resource::<CameraRig>().distance;
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let tilt = app.world().resource::<CameraRig>().tilt;
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let satellites = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let satellite_distance = fit_distance(satellites, fov(), tilt, margin);

    // Out a notch, then back in: the step lands exactly on the stop.
    scroll(&mut app, -1.0);
    scroll(&mut app, 1.0);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!(
        (distance - satellite_distance).abs() < satellite_distance * 1e-3,
        "{distance} vs {satellite_distance}"
    );

    for _ in 0..30 {
        scroll(&mut app, 1.0);
    }
    let radius_km = app.world().get::<PhysicalRadius>(halden).unwrap().0;
    let diameter_px = 2.0
        * projected_radius_px(
            km_to_render(radius_km),
            app.world().resource::<CameraRig>().distance,
            fov(),
            VIEWPORT.y as f32,
        );
    let map_height = VIEWPORT.y as f32 - TOP_BAR_HEIGHT_PX;
    assert!(
        (diameter_px - 0.25 * map_height).abs() < 1.0,
        "inner limit: {diameter_px} px of {map_height}"
    );
    assert_eq!(frame_body(&app), halden);

    // Tilt away from the standard, then zoom all the way out.
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.7;
    run(&mut app, 2);
    for _ in 0..40 {
        scroll(&mut app, -1.0);
    }
    let rig = *app.world().resource::<CameraRig>();
    let standard = app
        .world()
        .resource::<Tunables>()
        .standard_tilt_deg
        .to_radians();
    assert!((rig.distance - system_distance).abs() < system_distance * 1e-3);
    assert!((rig.tilt - standard).abs() < 1e-4, "CAM-23 standard tilt");
    assert!(rig.pivot.length() < 1e-3, "CAM-23 pivot on the star");
    assert!(in_star_frame(&app), "CAM-08");
}

/// ZOOM-13 / CAM-08: zooming in a body's frame keeps the frame body where
/// it is on screen, and keeps the frame.
#[test]
fn zoom_in_a_frame_keeps_the_body_in_place() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 3);
    for _ in 0..6 {
        scroll(&mut app, 1.0);
        let screen = app
            .world()
            .resource::<ScreenPositions>()
            .get(halden)
            .unwrap();
        let centre = VIEWPORT.as_vec2() / 2.0;
        assert!(screen.distance(centre) < 0.5, "Halden IV at {screen}");
        assert_eq!(frame_body(&app), halden);
    }
}

fn the_window(app: &mut App) -> Entity {
    let mut windows = app.world_mut().query_filtered::<Entity, With<Window>>();
    windows.single(app.world()).unwrap()
}

/// The point on the orbital plane under a screen position, through the
/// camera as placed this frame.
fn plane_point_under(app: &mut App, screen: Vec2) -> Vec3 {
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&Camera, &Transform), With<OrbitCamera>>();
    let (camera, transform) = cameras.single(app.world()).unwrap();
    let ray = camera
        .viewport_to_world(&GlobalTransform::from(*transform), screen)
        .unwrap();
    crate::view::camera::rules::plane_point(ray).unwrap()
}

fn move_cursor(app: &mut App, to: Vec2) {
    let window = the_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(to));
}

fn mouse(app: &mut App, button: MouseButton, state: ButtonState) {
    let window = the_window(app);
    app.world_mut().write_message(MouseButtonInput {
        button,
        state,
        window,
    });
}

/// CAM-10: drag the star from where it is to `to`, a few frames at a time.
fn drag_star(app: &mut App, star: Entity, to: Vec2) {
    drag_star_with(app, MouseButton::Left, star, to);
}

/// Drag the star to `to` with `button` held.
fn drag_star_with(app: &mut App, button: MouseButton, star: Entity, to: Vec2) {
    let from = app.world().resource::<ScreenPositions>().get(star).unwrap();
    move_cursor(app, from);
    run(app, 1);
    mouse(app, button, ButtonState::Pressed);
    run(app, 1);
    for step in 1..=10 {
        move_cursor(app, from.lerp(to, step as f32 / 10.0));
        run(app, 1);
    }
    mouse(app, button, ButtonState::Released);
    run(app, 2);
}

/// CAM-10: the plane point grabbed at the start of a left drag stays under
/// the cursor - also when tilted - and a press that barely moves is a
/// click, which doesn't pan.
#[test]
fn left_drag_keeps_the_grabbed_point_under_the_cursor() {
    let mut app = camera_app();
    let star = body(&mut app, "Corvane");

    for tilt in [0.0, 0.8] {
        app.world_mut().resource_mut::<CameraRig>().tilt = tilt;
        run(&mut app, 2);
        let to = Vec2::new(1100.0, 300.0);
        drag_star(&mut app, star, to);
        let at = app.world().resource::<ScreenPositions>().get(star).unwrap();
        assert!(
            at.distance(to) < 1.0,
            "tilt {tilt}: star at {at}, cursor at {to}"
        );
    }

    // A 2 px jitter is a click: the view doesn't move.
    let before = app.world().resource::<CameraRig>().pivot;
    let start = Vec2::new(500.0, 500.0);
    move_cursor(&mut app, start);
    run(&mut app, 1);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    run(&mut app, 1);
    move_cursor(&mut app, start + Vec2::new(2.0, 0.0));
    run(&mut app, 1);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    run(&mut app, 2);
    assert_eq!(app.world().resource::<CameraRig>().pivot, before);
}

/// CAM-07: panning moves the offset and keeps the frame.
#[test]
fn panning_keeps_the_frame() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 3);
    let star = body(&mut app, "Corvane");
    drag_star(&mut app, star, Vec2::new(700.0, 400.0));
    let framed = frame(&app);
    assert_eq!(framed.body, halden);
    assert!(
        framed.offset.length() > 1.0,
        "the offset moved: {}",
        framed.offset
    );
}

/// ZOOM-16 / CAM-03 / CAM-04 with the real camera: a frame request moves
/// to the body's satellite fit in its frame; N levels and turns north up,
/// keeping pivot and scale; Home returns to the system fit, level and
/// north up, and returns the frame to the star.
#[test]
fn frame_north_and_home_move_the_real_camera() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let system_distance = app.world().resource::<CameraRig>().distance;

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let standard_tilt = app
        .world()
        .resource::<Tunables>()
        .standard_tilt_deg
        .to_radians();
    let framed = fit_distance(radius, fov(), standard_tilt, margin);
    let rig = *app.world().resource::<CameraRig>();
    assert!(
        (rig.distance - framed).abs() < framed * 1e-3,
        "ZOOM-16 frames the satellites"
    );
    assert_eq!(frame_body(&app), halden);

    // Tilt and turn, then N.
    {
        let mut rig = app.world_mut().resource_mut::<CameraRig>();
        rig.tilt = 0.7;
        rig.heading = 2.0 * std::f32::consts::TAU + 1.0;
    }
    run(&mut app, 2);
    let before = *app.world().resource::<CameraRig>();
    press_key(&mut app, KeyCode::KeyN);
    run(&mut app, 40);
    let after = *app.world().resource::<CameraRig>();
    assert!(
        (after.tilt - standard_tilt).abs() < 1e-5,
        "CAM-03 standard tilt"
    );
    assert!(
        (after.heading - 2.0 * std::f32::consts::TAU).abs() < 1e-4,
        "north up, the short way"
    );
    assert!(
        (after.distance - before.distance).abs() < 1e-5,
        "scale kept"
    );
    assert_eq!(frame_body(&app), halden, "frame kept");

    // Home.
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.5;
    run(&mut app, 2);
    press_key(&mut app, KeyCode::Home);
    run(&mut app, 40);
    let home = *app.world().resource::<CameraRig>();
    assert!((home.distance - system_distance).abs() < system_distance * 1e-3);
    assert!((home.tilt - standard_tilt).abs() < 1e-5 && home.pivot.length() < 1e-3);
    assert!(in_star_frame(&app), "ZOOM-10: Home sets the star's frame");
}

fn press_key(app: &mut App, key: KeyCode) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        key_event(app, key, state);
        run(app, 1);
    }
}

fn key_event(app: &mut App, key: KeyCode, state: ButtonState) {
    let window = the_window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code: key,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window,
    });
}

/// Hold `key` down for `frames` frames, then let it go.
fn hold_key(app: &mut App, key: KeyCode, frames: usize) {
    key_event(app, key, ButtonState::Pressed);
    run(app, frames);
    key_event(app, key, ButtonState::Released);
    run(app, 1);
}

/// CAM-11: middle drag pans exactly as left drag: the grabbed point stays
/// under the cursor.
#[test]
fn middle_drag_pans_like_left_drag() {
    let mut app = camera_app();
    let star = body(&mut app, "Corvane");
    let to = Vec2::new(1000.0, 600.0);
    drag_star_with(&mut app, MouseButton::Middle, star, to);
    let at = app.world().resource::<ScreenPositions>().get(star).unwrap();
    assert!(at.distance(to) < 1.0, "star at {at}, cursor at {to}");
}

/// Right-drag the cursor `dx` pixels sideways from the screen centre.
fn right_drag(app: &mut App, dx: f32) {
    let start = VIEWPORT.as_vec2() / 2.0;
    move_cursor(app, start);
    run(app, 1);
    mouse(app, MouseButton::Right, ButtonState::Pressed);
    run(app, 1);
    for step in 1..=10 {
        move_cursor(app, start + Vec2::new(dx * step as f32 / 10.0, 0.0));
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(dx / 10.0, 0.0),
        });
        run(app, 1);
    }
    mouse(app, MouseButton::Right, ButtonState::Released);
    run(app, 2);
}

/// CAM-15: Q turns as dragging left does and E as dragging right, at
/// `key_rotate_deg_per_s`; Page Up tilts towards the horizon and Page
/// Down back towards straight down, clamped at `max_pitch_deg`.
#[test]
fn q_e_and_page_keys_turn_and_tilt_the_camera() {
    let mut app = camera_app();
    let tunables = app.world().resource::<Tunables>().clone();
    let rate = tunables.key_rotate_deg_per_s.to_radians();
    let heading = |app: &App| app.world().resource::<CameraRig>().heading;
    let tilt = |app: &App| app.world().resource::<CameraRig>().tilt;

    let before = heading(&app);
    hold_key(&mut app, KeyCode::KeyQ, 30);
    let turned = heading(&app) - before;
    let expected = rate * 30.0 * FRAME.as_secs_f32();
    assert!(
        (turned - expected).abs() < expected * 0.1,
        "Q turned {turned}, expected {expected}"
    );

    let before = heading(&app);
    hold_key(&mut app, KeyCode::KeyE, 30);
    assert!(heading(&app) < before, "E turns the other way");

    let mut dragged = camera_app();
    let before = heading(&dragged);
    right_drag(&mut dragged, -100.0);
    assert!(
        heading(&dragged) > before,
        "dragging left turns the way Q does"
    );

    hold_key(&mut app, KeyCode::PageUp, 30);
    assert!(tilt(&app) > 0.0, "Page Up leans towards the horizon");
    hold_key(&mut app, KeyCode::PageUp, 300);
    let max = tunables.max_pitch_deg.to_radians();
    assert!((tilt(&app) - max).abs() < 1e-4, "tilt clamped at {max}");
    hold_key(&mut app, KeyCode::PageDown, 300);
    assert!(tilt(&app).abs() < 1e-4, "Page Down back to straight down");
}

/// CAM-16: = and numpad + zoom in one step, and - and numpad - out one
/// step, landing exactly where a wheel notch does.
#[test]
fn plus_and_minus_keys_zoom_like_a_wheel_notch() {
    let distance = |app: &App| app.world().resource::<CameraRig>().distance;
    let mut wheel = camera_app();
    scroll(&mut wheel, 1.0);
    let wheel_in = distance(&wheel);
    scroll(&mut wheel, 1.0);
    scroll(&mut wheel, -1.0);
    let wheel_out = distance(&wheel);

    for (zoom_in, zoom_out) in [
        (KeyCode::Equal, KeyCode::Minus),
        (KeyCode::NumpadAdd, KeyCode::NumpadSubtract),
    ] {
        let mut keys = camera_app();
        press_key(&mut keys, zoom_in);
        run(&mut keys, 20);
        assert!((distance(&keys) - wheel_in).abs() < wheel_in * 1e-4);
        press_key(&mut keys, zoom_in);
        run(&mut keys, 20);
        press_key(&mut keys, zoom_out);
        run(&mut keys, 20);
        assert!((distance(&keys) - wheel_out).abs() < wheel_out * 1e-4);
    }
}

/// ZOOM-10: Home goes back to the system fit in an animated move, not a
/// cut: partway through, the camera is between where it was and the frame.
#[test]
fn home_is_an_animated_move_to_the_system_frame() {
    let mut app = camera_app();
    let system_distance = app.world().resource::<CameraRig>().distance;
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    let close = app.world().resource::<CameraRig>().distance;

    press_key(&mut app, KeyCode::Home);
    run(&mut app, move_frames() / 4);
    let partway = app.world().resource::<CameraRig>().distance;
    assert!(
        close < partway && partway < system_distance * 0.999,
        "partway at {partway}, from {close} to {system_distance}"
    );
    run(&mut app, move_frames());
    let home = app.world().resource::<CameraRig>().distance;
    assert!((home - system_distance).abs() < system_distance * 1e-3);
}

/// Press a bookmark slot with Ctrl held, which saves it.
fn save_bookmark(app: &mut App, slot: KeyCode) {
    key_event(app, KeyCode::ControlLeft, ButtonState::Pressed);
    press_key(app, slot);
    key_event(app, KeyCode::ControlLeft, ButtonState::Released);
    run(app, 1);
}

/// ZOOM-11: Ctrl+1 saves the view and 1 recalls it with an animated move;
/// a bookmark saved in a body's frame brings the frame back, and one saved
/// in the star's frame returns the frame to the star (CAM-06, CAM-08).
#[test]
fn bookmarks_save_and_recall_the_view() {
    let mut app = camera_app();
    let saved = CameraRig {
        pivot: Vec3::new(3.0, 0.0, -2.0),
        heading: 0.8,
        tilt: 0.4,
        distance: app.world().resource::<CameraRig>().distance * 0.5,
    };
    *app.world_mut().resource_mut::<CameraRig>() = saved;
    // In the star's frame the offset is the pivot (CAM-05).
    app.world_mut().resource_mut::<Frame>().offset = saved.pivot;
    run(&mut app, 2);
    save_bookmark(&mut app, KeyCode::Digit1);

    // Move into Halden IV's frame and save that in slot 2.
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    save_bookmark(&mut app, KeyCode::Digit2);
    let framed_distance = app.world().resource::<CameraRig>().distance;

    press_key(&mut app, KeyCode::Digit1);
    run(&mut app, 3);
    let early = *app.world().resource::<CameraRig>();
    assert!(
        (early.distance - saved.distance).abs() > saved.distance * 1e-3,
        "recalling is a move, not a cut"
    );
    run(&mut app, move_frames());
    let rig = *app.world().resource::<CameraRig>();
    assert!((rig.pivot - saved.pivot).length() < 1e-3);
    assert!((rig.heading - saved.heading).abs() < 1e-4);
    assert!((rig.tilt - saved.tilt).abs() < 1e-4);
    assert!((rig.distance - saved.distance).abs() < saved.distance * 1e-4);
    assert!(in_star_frame(&app));

    press_key(&mut app, KeyCode::Digit2);
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), halden);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!((distance - framed_distance).abs() < framed_distance * 1e-4);
}

/// CAM-20: pan or rotate input during an animated move ends the move and
/// carries on from where the camera is, with no jump.
#[test]
fn input_during_a_move_takes_over_without_a_jump() {
    for interrupt in [KeyCode::KeyQ, KeyCode::KeyD] {
        let mut app = camera_app();
        let halden = body(&mut app, "Halden IV");
        app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
        run(&mut app, move_frames() / 3);
        assert!(app.world().resource::<ActiveMove>().0.is_some());
        let before = *app.world().resource::<CameraRig>();

        key_event(&mut app, interrupt, ButtonState::Pressed);
        run(&mut app, 1);
        let after = *app.world().resource::<CameraRig>();
        assert!(
            app.world().resource::<ActiveMove>().0.is_none(),
            "{interrupt:?} ends the move"
        );
        // One frame of input moves the camera a little, never back to the
        // start or on to the target.
        assert!((after.distance - before.distance).abs() < before.distance * 0.01);
        assert!((after.pivot - before.pivot).length() < before.distance * 0.01);
        assert!((after.heading - before.heading).abs() < 0.05);
        key_event(&mut app, interrupt, ButtonState::Released);
        run(&mut app, 20);
        let later = app.world().resource::<CameraRig>().distance;
        assert!(
            (later - after.distance).abs() < after.distance * 1e-4,
            "the move stays ended"
        );
    }
}

/// CAM-07: panning a long way in a body's frame stops with the body at
/// least `focus_keep_margin_px` inside the map area, and keeps the frame.
#[test]
fn panning_in_a_frame_keeps_the_body_in_the_map_area() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 3);
    let star = body(&mut app, "Corvane");
    drag_star(&mut app, star, Vec2::new(1590.0, 890.0));
    run(&mut app, 3);
    let screen = app
        .world()
        .resource::<ScreenPositions>()
        .get(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().focus_keep_margin_px;
    assert!(
        inside_margin(screen, VIEWPORT.as_vec2(), margin - 1.0),
        "Halden IV at {screen}"
    );
    assert_eq!(frame_body(&app), halden);
}

/// CAM-02: the view starts in the standard orientation: north up, tilted by
/// `standard_tilt_deg`, over the star at the system fit.
#[test]
fn the_view_starts_in_the_standard_orientation() {
    let app = camera_app();
    let rig = *app.world().resource::<CameraRig>();
    let standard = app
        .world()
        .resource::<Tunables>()
        .standard_tilt_deg
        .to_radians();
    assert_eq!((rig.heading, rig.pivot), (0.0, Vec3::ZERO));
    assert!((rig.tilt - standard).abs() < 1e-6);
}

/// ZOOM-13 / CAM-05: zooming in the frame of a fast-moving body keeps it
/// at the same place on screen on every frame of the transition, not just
/// at its end.
#[test]
fn a_zoom_transition_follows_a_fast_frame_body() {
    let mut app = motion_camera_app(5.0);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let window = the_window(&mut app);
    for notches in [1.0, -1.0] {
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: notches,
            window,
            phase: TouchPhase::Moved,
        });
        for frame in 0..20 {
            run(&mut app, 1);
            let on_screen = app
                .world()
                .resource::<ScreenPositions>()
                .get(halden)
                .unwrap();
            assert!(
                on_screen.distance(VIEWPORT.as_vec2() / 2.0) < 0.5,
                "frame {frame}: Halden IV at {on_screen}"
            );
        }
    }
}

/// ZOOM-02 / ZOOM-13: big wheel inputs that retarget a zoom in flight, on a
/// fast frame body, never move it off its place on screen.
#[test]
fn rapid_large_wheel_inputs_keep_a_fast_frame_body_in_place() {
    let mut app = motion_camera_app(5.0);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let window = the_window(&mut app);
    let notches = [3.0, -2.0, 4.0, -5.0, 2.0, 1.0, -3.0];
    for (index, y) in notches.into_iter().enumerate() {
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y,
            window,
            phase: TouchPhase::Moved,
        });
        for frame in 0..4 {
            run(&mut app, 1);
            let on_screen = app
                .world()
                .resource::<ScreenPositions>()
                .get(halden)
                .unwrap();
            assert!(
                on_screen.distance(VIEWPORT.as_vec2() / 2.0) < 0.5,
                "input {index}, frame {frame}: Halden IV at {on_screen}"
            );
        }
    }
}

/// ZOOM-16: a frame request for the star, from inside a planet's frame,
/// sets the star's frame and moves to the system fit, keeping heading and
/// tilt.
#[test]
fn a_jump_to_the_star_goes_to_the_system_fit() {
    let mut app = camera_app();
    let system_distance = app.world().resource::<CameraRig>().distance;
    let halden = body(&mut app, "Halden IV");
    let star = body(&mut app, "Corvane");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    app.world_mut().resource_mut::<CameraRig>().heading = 0.9;
    run(&mut app, 2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(star);
    run(&mut app, move_frames());
    assert!(in_star_frame(&app));
    let rig = *app.world().resource::<CameraRig>();
    assert!((rig.distance - system_distance).abs() < system_distance * 1e-3);
    assert!(rig.pivot.length() < 1e-3);
    assert!((rig.heading - 0.9).abs() < 1e-5, "heading kept");
}

/// CAM-25: the frame body's own breadcrumb entry re-centres it with zero
/// offset at the same scale.
#[test]
fn re_centring_keeps_the_scale_and_clears_the_offset() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    drag_star(&mut app, halden, Vec2::new(700.0, 400.0));
    assert!(frame(&app).offset.length() > 1e-5);
    let distance = app.world().resource::<CameraRig>().distance;

    app.world_mut().resource_mut::<RecentreRequest>().0 = true;
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), halden);
    assert!(frame(&app).offset.length() < 1e-5);
    let after = app.world().resource::<CameraRig>().distance;
    assert!((after - distance).abs() < distance * 1e-4, "scale kept");
}

/// CAM-25 / CLK-02: in Arden Highport's frame the breadcrumb reads
/// "Cluster / Corvane / Halden IV / Arden Highport", in the label font
/// (MRK-02); clicking the Halden IV entry makes Halden IV the frame.
#[test]
fn the_breadcrumb_shows_the_frame_path_and_reframes_on_click() {
    let fonts = placeholder_fonts();
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins(BreadcrumbPlugin);
    });
    app.world_mut().spawn((BreadcrumbRoot, Node::default()));
    run(&mut app, 2);
    let names: Vec<String> = breadcrumb_entries(&mut app)
        .into_iter()
        .map(|(_, t)| t)
        .collect();
    assert_eq!(names, ["Cluster", "Corvane"]);

    let highport = body(&mut app, "Arden Highport");
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, highport, Vec3::ZERO);
    run(&mut app, 2);
    let entries = breadcrumb_entries(&mut app);
    let names: Vec<&str> = entries.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(names, ["Cluster", "Corvane", "Halden IV", "Arden Highport"]);
    let font = app.world().get::<TextFont>(entries[2].0).unwrap();
    assert_eq!(font.font, fonts.label(1.0).font, "MRK-02");

    let entry = entries[2].0;
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(PointerClick {
        entity: entry,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: VIEWPORT.x,
                    height: VIEWPORT.y,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: Duration::from_millis(80),
        count: 1,
    });
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), halden);
}

/// A press and release of the left button at `at`, without moving.
fn click_at(app: &mut App, at: Vec2) {
    move_cursor(app, at);
    run(app, 1);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    run(app, 1);
    mouse(app, MouseButton::Left, ButtonState::Released);
    run(app, 2);
}

/// The mouse pointer's location for Bevy's picking, kept with the window's
/// cursor, as the real input plugin would.
fn follow_cursor_with_pointer(
    windows: Query<(Entity, &Window)>,
    mut pointers: Query<(
        &bevy::picking::pointer::PointerId,
        &mut bevy::picking::pointer::PointerLocation,
    )>,
) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerId};
    let Ok((window, state)) = windows.single() else {
        return;
    };
    for (id, mut location) in &mut pointers {
        if *id == PointerId::Mouse {
            location.location = state.cursor_position().map(|position| Location {
                target: NormalizedRenderTarget::Window(
                    bevy::window::WindowRef::Entity(window)
                        .normalize(None)
                        .unwrap(),
                ),
                position,
            });
        }
    }
}

/// INS-03 / PLNR-03: a click on empty map space clears the selection,
/// unless the transfer planner is open; the release at the end of a pan
/// doesn't. Bevy's own picking runs, so the window's
/// catch-all hit behind everything is in play, as in the game.
#[test]
fn an_empty_click_clears_the_selection_and_a_pan_does_not() {
    use bevy::picking::{InteractionPlugin, PickingPlugin, PickingSystems};
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins((PickingPlugin, InteractionPlugin))
            .add_plugins(crate::view::hover::HoverPlugin)
            .add_systems(
                PreUpdate,
                follow_cursor_with_pointer.before(PickingSystems::ProcessInput),
            );
    });
    let window = the_window(&mut app);
    app.world_mut()
        .entity_mut(window)
        .insert(bevy::window::PrimaryWindow);
    app.world_mut()
        .spawn(bevy::picking::pointer::PointerId::Mouse);
    let halden = body(&mut app, "Halden IV");
    let star = body(&mut app, "Corvane");

    app.world_mut().resource_mut::<Selection>().select(halden);
    drag_star(&mut app, star, Vec2::new(900.0, 500.0));
    assert_eq!(app.world().resource::<Selection>().selected, Some(halden));

    let empty = Vec2::new(300.0, 700.0);
    click_at(&mut app, empty);
    let hits = app
        .world()
        .resource::<bevy::picking::hover::HoverMap>()
        .0
        .get(&bevy::picking::pointer::PointerId::Mouse)
        .map_or(0, |hits| hits.len());
    assert_eq!(hits, 1, "only the window is under the pointer");
    assert_eq!(app.world().resource::<Selection>().selected, None);

    // PLNR-03: while the transfer planner is open, the same click does
    // nothing.
    app.world_mut().resource_mut::<Selection>().select(halden);
    app.world_mut().resource_mut::<PlannerState>().open(halden);
    click_at(&mut app, empty);
    assert_eq!(app.world().resource::<Selection>().selected, Some(halden));
}

/// The screenshot debug option sets up the view it is given: the frame,
/// the selection, the clock rate, then distance, tilt and heading, and
/// asks for a capture once the view has settled.
#[test]
fn the_debug_shot_sets_up_its_view_then_asks_for_a_capture() {
    use crate::view::debug_shot::{DebugShotPlugin, Shot, ShotPhase, ShotPlan};
    use bevy::render::view::screenshot::Screenshot;

    let plan = ShotPlan::parse(
        "out=unused.png;frame=Halden IV;select=Morrow;distance=1e6;tilt=30;heading=45;rate=4;wait=0.5",
    )
    .unwrap();
    let mut app = camera_app_with(|app| {
        app.insert_resource(Shot::new(plan))
            .add_plugins(DebugShotPlugin);
    });
    let halden = body(&mut app, "Halden IV");
    let morrow = body(&mut app, "Morrow");
    let mut frames = 0;
    while app.world().resource::<Shot>().phase != ShotPhase::Capturing {
        run(&mut app, 1);
        frames += 1;
        assert!(frames < 600, "no capture after {frames} frames");
    }
    assert_eq!(frame_body(&app), halden);
    assert_eq!(app.world().resource::<Selection>().selected, Some(morrow));
    let clock = app.world().resource::<GameClock>();
    assert!(!clock.is_paused());
    assert_eq!(clock.rate_days_per_sec(), 4.0);
    let rig = *app.world().resource::<CameraRig>();
    assert!((rig.distance - km_to_render(1e6)).abs() < 1e-6);
    assert!((rig.tilt - 30f32.to_radians()).abs() < 1e-6);
    assert!((rig.heading - 45f32.to_radians()).abs() < 1e-6);
    let mut shots = app.world_mut().query::<&Screenshot>();
    assert_eq!(shots.iter(app.world()).count(), 1);
}

/// Runs a debug shot until it asks for a capture or fails, and says
/// which, with whether the app was asked to exit with an error.
fn run_shot_to_end(app: &mut App) -> (crate::view::debug_shot::ShotPhase, bool) {
    use crate::view::debug_shot::{Shot, ShotPhase};
    let mut frames = 0;
    loop {
        run(app, 1);
        frames += 1;
        let phase = app.world().resource::<Shot>().phase;
        if matches!(phase, ShotPhase::Capturing | ShotPhase::Failed) {
            let failed = app.should_exit().is_some_and(|exit| exit.is_error());
            return (phase, failed);
        }
        assert!(frames < 600, "the shot is stuck in {phase:?}");
    }
}

/// A debug shot of Corvane with the transfer planner, for `spec`.
fn planner_shot_app(spec: &str) -> App {
    use crate::view::debug_shot::{DebugShotPlugin, Shot, ShotPlan};
    use crate::view::planner::PlannerPlugin;
    let plan = ShotPlan::parse(spec).unwrap();
    camera_app_with(|app| {
        app.insert_resource(Shot::new(plan))
            .insert_resource(placeholder_fonts())
            .add_plugins((DebugShotPlugin, HoveredPlugin, PlannerPlugin));
    })
}

/// A shot that flies to a place the ship can't reach fails as soon as
/// the planner has searched and solved it, long before its timeout: a
/// Rendezvous at Iver needs more delta-v than Orison has.
#[test]
fn a_shot_that_cant_plan_its_trip_fails_once_the_planner_answers() {
    use crate::view::debug_shot::ShotPhase;
    let mut app = planner_shot_app("out=unused.png;select=Orison;planner=Iver;fly=5;timeout=30");
    let (phase, failed) = run_shot_to_end(&mut app);
    assert_eq!(phase, ShotPhase::Failed);
    assert!(failed, "the app is asked to exit with an error");
    let elapsed = app.world().resource::<Time>().elapsed_secs();
    assert!(elapsed < 5.0, "failed after {elapsed}s");
    let planner = app.world().resource::<PlannerState>();
    let session = planner.session.as_ref().expect("the planner stays open");
    assert!(session.found.is_some(), "the search ran before the failure");
    assert!(
        session.setting.is_some() && session.solved_for == session.setting,
        "the leg was solved before the failure"
    );
}

/// The same check lets a trip that can be planned go on: Halden IVa is
/// within Orison's budget, so the order is scheduled and the shot goes on
/// to its capture.
#[test]
fn a_shot_that_can_plan_its_trip_goes_on_to_the_capture() {
    use crate::view::debug_shot::ShotPhase;
    let mut app =
        planner_shot_app("out=unused.png;select=Orison;planner=Halden IVa;fly=1;wait=0.1");
    let (phase, failed) = run_shot_to_end(&mut app);
    assert_eq!(phase, ShotPhase::Capturing);
    assert!(!failed);
}

/// `planner` needs a selected ship that can open the planner (PLNR-01):
/// with nothing selected, or a planet selected, the shot fails at its
/// start instead of waiting for the timeout.
#[test]
fn a_shot_whose_selection_cant_open_the_planner_fails_at_its_start() {
    use crate::view::debug_shot::ShotPhase;
    for spec in [
        "out=unused.png;planner=open",
        "out=unused.png;select=Halden IV;planner=Iver;fly=1",
    ] {
        let mut app = planner_shot_app(spec);
        let (phase, failed) = run_shot_to_end(&mut app);
        assert_eq!(phase, ShotPhase::Failed, "{spec}");
        assert!(failed, "{spec}");
        assert!(
            !app.world().resource::<PlannerState>().is_open(),
            "{spec}: no planner opens"
        );
    }
}
