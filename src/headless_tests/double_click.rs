//! Headless tests of a double-click on the map whose target moves between
//! the clicks (ZOOM-16, HOV-02, INS-07): Bevy's own picking runs, with the
//! rings as the targets, so the clicks land where the rings are drawn.

use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;

use super::*;
use crate::view::rings::picking::{RingPaths, RingPickingPlugin, distance_to_segments};

/// Corvane with markers, ring hit targets, the map's click handling and
/// Bevy's picking driven by the cursor.
fn click_app() -> App {
    use crate::view::rings::marks::RingMarksPlugin;
    use bevy::picking::{InteractionPlugin, PickingPlugin, PickingSystems};
    let mut app = symbols_app_with(|app| {
        app.add_plugins((SharedOrbitPlugin, RingMarksPlugin))
            .add_plugins((PickingPlugin, InteractionPlugin, RingPickingPlugin))
            .add_plugins(crate::view::hover::HoverPlugin)
            // The clicks are timed on the real clock; a long interval
            // keeps a slow test machine from splitting a double-click.
            .insert_resource(bevy::picking::PickingSettings {
                multi_click_interval: std::time::Duration::from_secs(30),
                ..default()
            })
            .add_systems(
                PreUpdate,
                super::camera::follow_cursor_with_pointer.before(PickingSystems::ProcessInput),
            );
    });
    let window = the_window(&mut app);
    app.world_mut()
        .entity_mut(window)
        .insert(bevy::window::PrimaryWindow);
    app.world_mut()
        .spawn(bevy::picking::pointer::PointerId::Mouse);
    step(&mut app, SETTLE_FRAMES);
    app
}

/// Run `frames` frames, keeping the headless camera's projection in step
/// with its viewport as the inset narrows it.
fn step(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
        compute_camera_headless(app);
    }
}

/// A press and release of the left button at `at`, through Bevy's picking
/// as well as the button input.
fn click_at(app: &mut App, at: Vec2) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
    let window = the_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(at));
    step(app, 1);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            bevy::window::WindowRef::Entity(window)
                .normalize(None)
                .unwrap(),
        ),
        position: at,
    };
    for (state, action) in [
        (
            ButtonState::Pressed,
            PointerAction::Press(PointerButton::Primary),
        ),
        (
            ButtonState::Released,
            PointerAction::Release(PointerButton::Primary),
        ),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        step(app, 1);
    }
}

/// `body`'s ring hit target and its drawn segments.
fn ring_of(app: &App, body: Entity) -> (Entity, Vec<(Vec2, Vec2)>) {
    app.world()
        .resource::<RingPaths>()
        .0
        .iter()
        .find(|(ring, _)| app.world().get::<MarkerFor>(**ring).map(|m| m.0) == Some(body))
        .map(|(&ring, segments)| (ring, segments.clone()))
        .expect("the body's ring is drawn")
}

/// The rightmost point on `body`'s ring that is clear of every other
/// ring. There the ring runs up and down the screen, so when the inset
/// narrows the map and slides it sideways the ring leaves the point.
fn side_of_ring(app: &App, body: Entity) -> Vec2 {
    let tolerance = Tunables::default().ring_hit_px;
    let (ring, segments) = ring_of(app, body);
    let paths = &app.world().resource::<RingPaths>().0;
    let mut points: Vec<Vec2> = segments.iter().map(|&(a, b)| a.lerp(b, 0.5)).collect();
    points.sort_by(|a, b| b.x.total_cmp(&a.x));
    points
        .into_iter()
        .find(|&point| {
            paths.iter().all(|(&other, segments)| {
                other == ring || distance_to_segments(point, segments) > 3.0 * tolerance
            })
        })
        .expect("a point on the ring clear of the others")
}

fn selected(app: &App) -> Option<Entity> {
    app.world().resource::<Selection>().selected
}

/// ZOOM-16 / INS-07 / HOV-02: with nothing selected, the first click of a
/// double-click on a planet's ring selects it and opens the inset, which
/// slides the map, and the ring, out from under the pointer. The second
/// click still lands on the planet: the planet becomes the frame.
#[test]
fn a_double_click_frames_the_planet_though_the_inset_moved_it() {
    let mut app = click_app();
    let halden = body(&mut app, "Halden IV");
    assert!(in_star_frame(&app));
    assert_eq!(selected(&app), None);
    let point = side_of_ring(&app, halden);

    click_at(&mut app, point);
    assert_eq!(selected(&app), Some(halden), "the first click selects");
    // The inset slides open over 200 ms; the second click comes as the
    // slide ends, well within a double-click's interval.
    step(&mut app, 12);
    let (_, moved) = ring_of(&app, halden);
    assert!(
        distance_to_segments(point, &moved) > Tunables::default().ring_hit_px,
        "the ring has slid out from under the pointer"
    );

    click_at(&mut app, point);
    step(&mut app, SETTLE_FRAMES);
    assert_eq!(selected(&app), Some(halden));
    assert_eq!(frame_body(&app), halden, "the planet is the frame");
}

/// ZOOM-16 / INS-03: a first click on empty map clears the selection and
/// starts no double-click, so a quick click on a planet's ring after it
/// only selects the planet.
#[test]
fn a_click_after_an_empty_click_only_selects() {
    let mut app = click_app();
    let halden = body(&mut app, "Halden IV");
    let point = side_of_ring(&app, halden);

    click_at(&mut app, Vec2::new(60.0, VIEWPORT.y as f32 - 60.0));
    assert_eq!(selected(&app), None);
    click_at(&mut app, point);
    step(&mut app, SETTLE_FRAMES);
    assert_eq!(selected(&app), Some(halden));
    assert!(in_star_frame(&app), "no frame jump");
}
